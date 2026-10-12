//! Generic, benchmark-independent run orchestration.
//!
//! [`GenericRunController`] drives the four reusable lifecycle phases without
//! knowing an environment's schedule, hidden state, labels, or scoring rules.
//! It records a learner decision in the receipt-backed ledger before calling
//! [`crate::runtime::apply_environment`], and only gives an observed result to
//! a mutable learner after the ledger has retained that result. Frozen
//! evaluation never calls `observe` and holds the frozen learner behind `&self`.
//!
//! MVP-4 [`FrameworkState`](super::FrameworkState) remains the persistence
//! boundary for learner-owned stores, ledger events, and resource counters.
//! An arbitrary learner type `L` is not serialized by this controller. The
//! [`ControllerState`] view exposes the controller-owned state for callers that
//! persist their learner through an application-specific snapshot hook.

use std::fmt;

use crate::runtime::{
    apply_environment, Environment, EnvironmentActionError, Lifecycle, LifecycleError, RunPhase,
};

use super::interaction_ledger::{InteractionLedger, LedgerError};
use super::opaque_protocol::{
    AbstainReason, Decision, ProtocolError, ScopeToken, StepInput, TerminalStatus,
};
use super::resource_scale::{LogicalResource, MeasurementError, ResourceLimitError, ResourceMeter};
use super::{LedgerDecision, LedgerFrozenLearner, LedgerLearner};

/// Counts of episodes the controller may execute in each phase.
///
/// The counts are controller policy, not a benchmark schedule. A zero count
/// is valid and still permits the lifecycle transition through that phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EpisodeBudget {
    discovery: usize,
    development: usize,
    final_evaluation: usize,
}

impl EpisodeBudget {
    pub const fn new(discovery: usize, development: usize, final_evaluation: usize) -> Self {
        Self {
            discovery,
            development,
            final_evaluation,
        }
    }

    pub const fn discovery(self) -> usize {
        self.discovery
    }

    pub const fn development(self) -> usize {
        self.development
    }

    pub const fn final_evaluation(self) -> usize {
        self.final_evaluation
    }
}

impl Default for EpisodeBudget {
    fn default() -> Self {
        Self::new(1, 1, 1)
    }
}

/// Failure classifications that can be established from the generic protocol.
///
/// The controller does not infer evaluator-only causes. For example, an
/// environment error is classified as [`Self::EnvironmentFailure`], while a
/// public terminal `Failure` result is classified as an incorrect hypothesis
/// outcome. Abstention reasons remain explicit and are never replaced by a
/// fabricated packet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureClassification {
    InsufficientEvidence,
    SearchExhaustion,
    UnsupportedGrammar,
    IncorrectHypothesis,
    EnvironmentFailure,
    BudgetExhausted,
    UnresolvedDecision,
    ProtocolViolation,
    LedgerFailure,
}

impl FailureClassification {
    pub const fn name(self) -> &'static str {
        match self {
            Self::InsufficientEvidence => "insufficient-evidence",
            Self::SearchExhaustion => "search-exhaustion",
            Self::UnsupportedGrammar => "unsupported-grammar",
            Self::IncorrectHypothesis => "incorrect-hypothesis",
            Self::EnvironmentFailure => "environment-failure",
            Self::BudgetExhausted => "budget-exhausted",
            Self::UnresolvedDecision => "unresolved-decision",
            Self::ProtocolViolation => "protocol-violation",
            Self::LedgerFailure => "ledger-failure",
        }
    }
}

/// A resource exhaustion preserved by a controller error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BudgetFailure {
    LogicalLimit(ResourceLimitError),
    WallClockLimitExceeded {
        elapsed_ns: u64,
        limit_ns: u64,
    },
    MemoryLimitExceeded {
        observed_bytes: u64,
        limit_bytes: u64,
    },
}

impl BudgetFailure {
    pub const fn classification(self) -> FailureClassification {
        match self {
            Self::LogicalLimit(error)
                if matches!(
                    error.resource(),
                    LogicalResource::CandidateGenerationAttempts
                        | LogicalResource::StructuralComparisons
                ) =>
            {
                FailureClassification::SearchExhaustion
            }
            Self::LogicalLimit(_)
            | Self::WallClockLimitExceeded { .. }
            | Self::MemoryLimitExceeded { .. } => FailureClassification::BudgetExhausted,
        }
    }

    pub const fn from_measurement(error: MeasurementError) -> Option<Self> {
        match error {
            MeasurementError::LogicalLimit(limit) => Some(Self::LogicalLimit(limit)),
            MeasurementError::WallClockLimitExceeded {
                elapsed_ns,
                limit_ns,
            } => Some(Self::WallClockLimitExceeded {
                elapsed_ns,
                limit_ns,
            }),
            MeasurementError::MemoryLimitExceeded {
                observed_bytes,
                limit_bytes,
            } => Some(Self::MemoryLimitExceeded {
                observed_bytes,
                limit_bytes,
            }),
            MeasurementError::CounterOverflow(_)
            | MeasurementError::StageCounterOverflow(_)
            | MeasurementError::StageAlreadyStarted(_)
            | MeasurementError::StageNotStarted(_)
            | MeasurementError::MissingMemoryMeasurement
            | MeasurementError::DuplicateScalePoint(_) => None,
        }
    }
}

impl fmt::Display for BudgetFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LogicalLimit(error) => error.fmt(formatter),
            Self::WallClockLimitExceeded {
                elapsed_ns,
                limit_ns,
            } => write!(formatter, "wall clock {elapsed_ns} exceeds {limit_ns}"),
            Self::MemoryLimitExceeded {
                observed_bytes,
                limit_bytes,
            } => write!(
                formatter,
                "resident memory {observed_bytes} exceeds {limit_bytes}"
            ),
        }
    }
}

impl std::error::Error for BudgetFailure {}

/// Errors returned while driving a generic run.
#[derive(Debug)]
pub enum ControllerError<E, L = ProtocolError> {
    Learner(L),
    Environment(E),
    Ledger(LedgerError),
    Protocol(ProtocolError),
    InvalidPhase {
        expected: RunPhase,
        actual: RunPhase,
    },
    UnresolvedDecision {
        sequence: u64,
    },
    Budget(BudgetFailure),
}

impl<E: fmt::Display, L: fmt::Display> fmt::Display for ControllerError<E, L> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Learner(error) => write!(formatter, "learner: {error}"),
            Self::Environment(error) => write!(formatter, "environment: {error}"),
            Self::Ledger(error) => write!(formatter, "ledger: {error}"),
            Self::Protocol(error) => write!(formatter, "protocol: {error}"),
            Self::InvalidPhase { expected, actual } => {
                write!(formatter, "expected run phase {expected:?}, got {actual:?}")
            }
            Self::UnresolvedDecision { sequence } => {
                write!(formatter, "decision {sequence} was not resolved")
            }
            Self::Budget(error) => write!(formatter, "budget: {error}"),
        }
    }
}

impl<E: std::error::Error + 'static, L: std::error::Error + 'static> std::error::Error
    for ControllerError<E, L>
{
}

impl<E, L> ControllerError<E, L> {
    /// Return the report classification established by this controller error.
    pub const fn failure_classification(&self) -> Option<FailureClassification> {
        match self {
            Self::Environment(_) => Some(FailureClassification::EnvironmentFailure),
            Self::UnresolvedDecision { .. } => Some(FailureClassification::UnresolvedDecision),
            Self::Budget(error) => Some(error.classification()),
            Self::Learner(_) => Some(FailureClassification::ProtocolViolation),
            Self::Protocol(_) => Some(FailureClassification::ProtocolViolation),
            Self::Ledger(_) => Some(FailureClassification::LedgerFailure),
            Self::InvalidPhase { .. } => None,
        }
    }
}

/// The inspectable result of one completed episode.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpisodeSummary {
    pub phase: RunPhase,
    pub episode_scope: ScopeToken,
    pub decision_sequences: Vec<u64>,
    pub decision_count: u64,
    pub action_count: u64,
    pub abstention_count: u64,
    pub terminal: TerminalStatus,
    pub failure_classifications: Vec<FailureClassification>,
}

/// The inspectable controller-owned report accumulated by a run.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RunSummary {
    pub episodes: Vec<EpisodeSummary>,
    pub failure_classifications: Vec<FailureClassification>,
}

impl RunSummary {
    pub fn decision_count(&self) -> u64 {
        self.episodes
            .iter()
            .map(|episode| episode.decision_count)
            .sum()
    }

    pub fn action_count(&self) -> u64 {
        self.episodes
            .iter()
            .map(|episode| episode.action_count)
            .sum()
    }

    pub fn abstention_count(&self) -> u64 {
        self.episodes
            .iter()
            .map(|episode| episode.abstention_count)
            .sum()
    }
}

/// A read-only view of controller state for an application-owned persistence
/// boundary. It intentionally contains no arbitrary learner serialization.
pub struct ControllerState<'a> {
    phase: RunPhase,
    ledger: &'a InteractionLedger,
    summary: &'a RunSummary,
}

impl<'a> ControllerState<'a> {
    pub const fn phase(&self) -> RunPhase {
        self.phase
    }

    pub const fn ledger(&self) -> &'a InteractionLedger {
        self.ledger
    }

    pub const fn summary(&self) -> &'a RunSummary {
        self.summary
    }
}

/// Drives a generic learner/environment pair through discovery, development,
/// freeze, and final evaluation.
pub struct GenericRunController<L, E>
where
    L: LedgerLearner,
    L::Frozen: LedgerFrozenLearner<Error = L::Error>,
    E: Environment,
{
    learner: Option<L>,
    frozen_learner: Option<L::Frozen>,
    environment: E,
    ledger: InteractionLedger,
    lifecycle: Lifecycle,
    episode_budget: EpisodeBudget,
    discovery_completed: bool,
    summary: RunSummary,
}

impl<L, E> GenericRunController<L, E>
where
    L: LedgerLearner,
    L::Frozen: LedgerFrozenLearner<Error = L::Error>,
    E: Environment,
{
    pub fn new(
        learner: L,
        environment: E,
        run_scope: ScopeToken,
    ) -> Result<Self, ControllerError<E::Error, L::Error>> {
        Self::with_budget(learner, environment, run_scope, EpisodeBudget::default())
    }

    pub fn with_budget(
        learner: L,
        environment: E,
        run_scope: ScopeToken,
        episode_budget: EpisodeBudget,
    ) -> Result<Self, ControllerError<E::Error, L::Error>> {
        let ledger = InteractionLedger::new(run_scope).map_err(map_ledger_error)?;
        Ok(Self {
            learner: Some(learner),
            frozen_learner: None,
            environment,
            ledger,
            lifecycle: Lifecycle::new(),
            episode_budget,
            discovery_completed: false,
            summary: RunSummary::default(),
        })
    }

    pub fn with_budget_and_resource_meter(
        learner: L,
        environment: E,
        run_scope: ScopeToken,
        episode_budget: EpisodeBudget,
        meter: ResourceMeter,
    ) -> Result<Self, ControllerError<E::Error, L::Error>> {
        let ledger =
            InteractionLedger::with_resource_meter(run_scope, meter).map_err(map_ledger_error)?;
        Ok(Self {
            learner: Some(learner),
            frozen_learner: None,
            environment,
            ledger,
            lifecycle: Lifecycle::new(),
            episode_budget,
            discovery_completed: false,
            summary: RunSummary::default(),
        })
    }

    pub const fn phase(&self) -> RunPhase {
        self.lifecycle.phase()
    }

    pub const fn episode_budget(&self) -> EpisodeBudget {
        self.episode_budget
    }

    pub const fn ledger(&self) -> &InteractionLedger {
        &self.ledger
    }

    pub const fn environment(&self) -> &E {
        &self.environment
    }

    pub fn environment_mut(&mut self) -> &mut E {
        &mut self.environment
    }

    pub const fn learner(&self) -> Option<&L> {
        self.learner.as_ref()
    }

    /// Borrow the mutable learner and controller ledger together for an
    /// application-owned induction/admission hook. The hook is available only
    /// before freeze; the controller still owns the lifecycle and the receipt
    /// ordering around every environment action.
    pub fn learner_and_ledger_mut(&mut self) -> Option<(&mut L, &mut InteractionLedger)> {
        self.learner
            .as_mut()
            .map(|learner| (learner, &mut self.ledger))
    }

    pub const fn frozen_learner(&self) -> Option<&L::Frozen> {
        self.frozen_learner.as_ref()
    }

    pub const fn summary(&self) -> &RunSummary {
        &self.summary
    }

    pub const fn state(&self) -> ControllerState<'_> {
        ControllerState {
            phase: self.phase(),
            ledger: &self.ledger,
            summary: &self.summary,
        }
    }

    /// Run the configured discovery episodes. This leaves the lifecycle in
    /// `Discovery`; [`Self::development`] performs the ordered transition.
    pub fn discovery(&mut self) -> Result<(), ControllerError<E::Error, L::Error>> {
        self.require_phase(RunPhase::Discovery)?;
        if self.discovery_completed {
            return Err(ControllerError::InvalidPhase {
                expected: RunPhase::Discovery,
                actual: self.phase(),
            });
        }
        self.run_episodes(RunPhase::Discovery, self.episode_budget.discovery())?;
        self.discovery_completed = true;
        Ok(())
    }

    /// Transition to and run the configured development episodes.
    pub fn development(&mut self) -> Result<(), ControllerError<E::Error, L::Error>> {
        self.require_phase(RunPhase::Discovery)?;
        if !self.discovery_completed {
            return Err(ControllerError::InvalidPhase {
                expected: RunPhase::Development,
                actual: self.phase(),
            });
        }
        let actual_phase = self.phase();
        self.learner
            .as_mut()
            .ok_or(ControllerError::InvalidPhase {
                expected: RunPhase::Development,
                actual: actual_phase,
            })?
            .enter_development()
            .map_err(ControllerError::Learner)?;
        self.lifecycle
            .begin_development()
            .map_err(map_lifecycle_error)?;
        self.run_episodes(RunPhase::Development, self.episode_budget.development())
    }

    /// Freeze both the receipt ledger and learner policy. No active decision
    /// can cross this boundary.
    pub fn freeze(&mut self) -> Result<(), ControllerError<E::Error, L::Error>> {
        self.require_phase(RunPhase::Development)?;
        self.ledger.validate_freeze().map_err(map_ledger_error)?;
        let learner = self.learner.take().ok_or(ControllerError::InvalidPhase {
            expected: RunPhase::Development,
            actual: self.phase(),
        })?;
        let frozen = match learner.freeze_with_ledger(&self.ledger) {
            Ok(frozen) => frozen,
            Err(failure) => {
                let (learner, error) = failure.into_parts();
                self.learner = Some(learner);
                return Err(ControllerError::Learner(error));
            }
        };
        // `validate_freeze` ran before the consuming learner hook, and the
        // ledger-aware hook receives only `&InteractionLedger`, so this commit
        // cannot observe a new pending operation or a learner-side freeze.
        self.ledger.commit_freeze();
        self.frozen_learner = Some(frozen);
        self.lifecycle.freeze().map_err(map_lifecycle_error)
    }

    /// Transition to final evaluation and run the configured frozen episodes.
    /// Final results are retained in the ledger for audit but are never passed
    /// to the frozen learner.
    pub fn final_evaluation(&mut self) -> Result<(), ControllerError<E::Error, L::Error>> {
        self.require_phase(RunPhase::Frozen)?;
        self.lifecycle
            .begin_final_evaluation()
            .map_err(map_lifecycle_error)?;
        self.run_episodes(
            RunPhase::FinalEvaluation,
            self.episode_budget.final_evaluation(),
        )
    }

    fn require_phase(&self, expected: RunPhase) -> Result<(), ControllerError<E::Error, L::Error>> {
        let actual = self.phase();
        if actual == expected {
            Ok(())
        } else {
            Err(ControllerError::InvalidPhase { expected, actual })
        }
    }

    fn run_episodes(
        &mut self,
        phase: RunPhase,
        count: usize,
    ) -> Result<(), ControllerError<E::Error, L::Error>> {
        for _ in 0..count {
            let input = self
                .environment
                .reset(&self.ledger)
                .map_err(|error| self.record_error(ControllerError::Environment(error)))?;
            let episode = match self.run_episode(phase, input) {
                Ok(episode) => episode,
                Err(error) => return Err(self.record_error(error)),
            };
            self.summary
                .failure_classifications
                .extend(episode.failure_classifications.iter().copied());
            self.summary.episodes.push(episode);
        }
        Ok(())
    }

    fn run_episode(
        &mut self,
        phase: RunPhase,
        mut input: StepInput,
    ) -> Result<EpisodeSummary, ControllerError<E::Error, L::Error>> {
        let episode_scope = input.episode_scope();
        let mut decision_sequences = Vec::new();
        let mut decision_count = 0_u64;
        let mut action_count = 0_u64;
        let mut abstention_count = 0_u64;
        let mut failures = Vec::new();
        let terminal = loop {
            let actual_phase = self.phase();

            let learner_decision = match phase {
                RunPhase::FinalEvaluation => self
                    .frozen_learner
                    .as_ref()
                    .ok_or(ControllerError::InvalidPhase {
                        expected: RunPhase::Frozen,
                        actual: actual_phase,
                    })?
                    .step_with_ledger(&mut self.ledger, input)
                    .map_err(ControllerError::Learner)?,
                RunPhase::Discovery | RunPhase::Development => self
                    .learner
                    .as_mut()
                    .ok_or(ControllerError::InvalidPhase {
                        expected: RunPhase::Development,
                        actual: actual_phase,
                    })?
                    .step_with_ledger(&mut self.ledger, input)
                    .map_err(ControllerError::Learner)?,
                RunPhase::Frozen => {
                    return Err(ControllerError::InvalidPhase {
                        expected: RunPhase::FinalEvaluation,
                        actual: actual_phase,
                    });
                }
            };

            let receipt = match learner_decision {
                LedgerDecision::Unrecorded(decision) => self
                    .ledger
                    .record_decision(input, decision)
                    .map_err(map_ledger_error)?,
                LedgerDecision::Recorded(receipt)
                    if receipt.record().input() == input
                        && self.ledger.pending_decision() == Some(&receipt) =>
                {
                    receipt
                }
                LedgerDecision::Recorded(receipt) => {
                    return Err(ControllerError::Ledger(LedgerError::ReceiptMismatch(
                        receipt.sequence(),
                    )))
                }
            };
            let decision = receipt.decision().clone();
            decision_sequences.push(receipt.sequence());
            decision_count += 1;
            if receipt.expects_result() {
                action_count += 1;
            } else {
                abstention_count += 1;
                if let Decision::Abstain(abstention) = &decision {
                    failures.push(classification_for_abstention(abstention.reason()));
                }
            }

            let result = match apply_environment(&mut self.environment, &mut self.ledger, &receipt)
            {
                Ok(result) => result,
                Err(error) => return Err(map_environment_error(error, receipt.sequence())),
            };

            match result {
                Some(result) => {
                    let outcome = result.terminal();
                    if outcome == TerminalStatus::Failure {
                        failures.push(FailureClassification::IncorrectHypothesis);
                    }

                    // This branch is deliberately phase-based rather than
                    // learner-type-based: frozen results never reach observe.
                    if phase != RunPhase::FinalEvaluation {
                        self.learner
                            .as_mut()
                            .ok_or(ControllerError::InvalidPhase {
                                expected: RunPhase::Development,
                                actual: actual_phase,
                            })?
                            .observe_with_ledger(&mut self.ledger, Some(result))
                            .map_err(ControllerError::Learner)?;
                    }

                    if outcome == TerminalStatus::Continue {
                        input = StepInput::new(
                            input.run_scope(),
                            input.episode_scope(),
                            1,
                            result.observation(),
                            input.available_actions(),
                            1,
                            crate::runtime::PreviousTerminal::Continue,
                        )
                        .map_err(ControllerError::Protocol)?;
                        continue;
                    }
                    break outcome;
                }
                None => {
                    if phase != RunPhase::FinalEvaluation {
                        self.learner
                            .as_mut()
                            .ok_or(ControllerError::InvalidPhase {
                                expected: RunPhase::Development,
                                actual: actual_phase,
                            })?
                            .observe_with_ledger(&mut self.ledger, None)
                            .map_err(ControllerError::Learner)?;
                    }
                    break TerminalStatus::Incomplete;
                }
            }
        };

        Ok(EpisodeSummary {
            phase,
            episode_scope,
            decision_sequences,
            decision_count,
            action_count,
            abstention_count,
            terminal,
            failure_classifications: failures,
        })
    }

    fn record_error(
        &mut self,
        error: ControllerError<E::Error, L::Error>,
    ) -> ControllerError<E::Error, L::Error> {
        if let Some(classification) = error.failure_classification() {
            self.summary.failure_classifications.push(classification);
        }
        error
    }
}

impl<L, E> crate::runtime::RunController for GenericRunController<L, E>
where
    L: LedgerLearner,
    L::Frozen: LedgerFrozenLearner<Error = L::Error>,
    E: Environment,
    E::Error: std::error::Error + 'static,
{
    type Error = ControllerError<E::Error, L::Error>;

    fn phase(&self) -> RunPhase {
        self.phase()
    }

    fn discovery(&mut self) -> Result<(), Self::Error> {
        Self::discovery(self)
    }

    fn development(&mut self) -> Result<(), Self::Error> {
        Self::development(self)
    }

    fn freeze(&mut self) -> Result<(), Self::Error> {
        Self::freeze(self)
    }

    fn final_evaluation(&mut self) -> Result<(), Self::Error> {
        Self::final_evaluation(self)
    }
}

fn classification_for_abstention(reason: AbstainReason) -> FailureClassification {
    match reason {
        AbstainReason::NoApplicableProcedure | AbstainReason::InsufficientEvidence => {
            FailureClassification::InsufficientEvidence
        }
        AbstainReason::Conflict => FailureClassification::IncorrectHypothesis,
        AbstainReason::UnsupportedByGrammar => FailureClassification::UnsupportedGrammar,
        AbstainReason::IncompleteBudget => FailureClassification::BudgetExhausted,
    }
}

fn map_lifecycle_error<E, L>(error: LifecycleError) -> ControllerError<E, L> {
    match error {
        LifecycleError::InvalidTransition { from, to } => ControllerError::InvalidPhase {
            expected: to,
            actual: from,
        },
    }
}

fn map_ledger_error<E, L>(error: LedgerError) -> ControllerError<E, L> {
    match error {
        LedgerError::Protocol(error) => ControllerError::Protocol(error),
        LedgerError::Measurement(error) => match BudgetFailure::from_measurement(error) {
            Some(failure) => ControllerError::Budget(failure),
            None => ControllerError::Ledger(LedgerError::Measurement(error)),
        },
        other => ControllerError::Ledger(other),
    }
}

fn map_environment_error<E, L>(
    error: EnvironmentActionError<E>,
    sequence: u64,
) -> ControllerError<E, L> {
    match error {
        EnvironmentActionError::Environment(error) => ControllerError::Environment(error),
        EnvironmentActionError::Ledger(LedgerError::EnvironmentResultMissing) => {
            ControllerError::UnresolvedDecision { sequence }
        }
        EnvironmentActionError::Ledger(error) => map_ledger_error(error),
    }
}
