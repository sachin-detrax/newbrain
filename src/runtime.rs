//! Reusable MVP-1 runtime boundaries.
//!
//! The runtime boundary is intentionally narrower than a benchmark runner.
//! A learner sees only the opaque v2 protocol (`StepInput`, `Decision`, and
//! `StepResult`) plus its own evidence references. An [`Environment`] owns
//! hidden state and transition behavior, but can expose a result only through
//! the ledger's receipt-backed [`EnvironmentTurn`]. In particular, an
//! environment cannot apply an action through this API until the learner's
//! decision has been serialized by [`InteractionLedger::record_decision`].
//!
//! The legacy v2 evaluator bridge implementation is deliberately not part of
//! this module. It lives in [`crate::v2::adapter`]; only a compatibility alias
//! is re-exported below so this reusable API can be used without importing
//! evaluator worlds, schedules, or scoring code.

use std::fmt;

pub use crate::framework::interaction_ledger::{
    DecisionReceipt, EnvironmentActionError, EnvironmentTurn, InteractionLedger, LedgerError,
};
pub type EnvironmentError<E> = EnvironmentActionError<E>;
pub use crate::framework::interaction_ledger::{EvidenceProvenance, RunPhase as LedgerRunPhase};
pub use crate::framework::opaque_protocol::{
    AbstainDecision, AbstainReason, ActDecision, ActMode, ActionId, AtomId, AuditReferenceKind,
    AuditReferences, Decision, EvidenceId, HypothesisId, ObservationPacket, PreviousTerminal,
    ProcedureId, ProtocolError, ScopeToken, StepInput, StepResult, TerminalStatus,
};

pub use crate::framework::interaction_ledger::{
    DecisionRecord, EvidenceRecord, InteractionRecord, LedgerEvent,
};
pub use crate::framework::{
    controller, grammar_admission, grounding_decision, interaction_ledger, opaque_protocol,
    partial_observation, persistence, resource_scale, stores,
};
pub use crate::framework::{
    BudgetFailure, ControllerError, ControllerState, EpisodeBudget, EpisodeSummary,
    FailureClassification, GenericRunController, RunSummary,
};
pub use crate::framework::{
    DependencyIndex, DependencySource, DependentRecord, DerivedId, EvidenceStore, HypothesisRecord,
    HypothesisStore, HypothesisTransition, LearnerStores, ProcedureStore, ProcedureVersion,
    StoreEntity, StoreError, StoredHypothesisStatus, StoredProcedure,
};
pub use crate::framework::{
    FrameworkState, PersistenceError, StateFingerprint, PERSISTENCE_FORMAT,
    PERSISTENCE_SCHEMA_VERSION,
};
/// Compatibility-only alias for the legacy bridge. The implementation remains
/// in [`crate::v2::adapter`]; reusable environments should not use this alias.
pub use crate::v2::adapter::Evaluator as V2Evaluator;

/// A benchmark-independent environment boundary.
///
/// `reset` delivers the next learner-visible input. `apply` receives a
/// receipt-backed turn. Implementations should resolve the turn with
/// [`EnvironmentTurn::resolve`], which authorizes the action before invoking
/// the environment transition and records the opaque terminal result in the
/// ledger. For an abstention, `resolve` records `Incomplete` and does not call
/// the transition closure.
pub trait Environment {
    type Error: std::error::Error;

    fn reset(&mut self, ledger: &InteractionLedger) -> Result<StepInput, Self::Error>;

    fn apply(
        &mut self,
        turn: &mut EnvironmentTurn<'_>,
    ) -> Result<(), EnvironmentActionError<Self::Error>>;
}

/// Run one receipt-backed environment operation and return its opaque result.
///
/// This helper is the common caller path for custom environments. It opens the
/// turn only for the exact pending receipt, invokes the environment, and then
/// rejects an implementation that returned without recording either a result
/// or an abstention. It contains no schedule, labels, scoring, or benchmark
/// policy.
pub fn apply_environment<E>(
    environment: &mut E,
    ledger: &mut InteractionLedger,
    receipt: &DecisionReceipt,
) -> Result<Option<StepResult>, EnvironmentActionError<E::Error>>
where
    E: Environment,
{
    let mut turn = ledger
        .begin_environment_turn(receipt)
        .map_err(EnvironmentActionError::Ledger)?;
    environment.apply(&mut turn)?;
    turn.finish().map_err(EnvironmentActionError::Ledger)
}

/// Lifecycle phases exposed to a generic run controller.
///
/// The enum describes orchestration only. It does not name a partition,
/// schedule, seed, label, or evaluator-only concept.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunPhase {
    Discovery,
    Development,
    Frozen,
    FinalEvaluation,
}

/// Errors from the small reusable lifecycle state machine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleError {
    InvalidTransition { from: RunPhase, to: RunPhase },
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(formatter, "cannot transition run from {from:?} to {to:?}")
            }
        }
    }
}

impl std::error::Error for LifecycleError {}

/// Minimal phase state that a controller can embed without inheriting any
/// benchmark-specific schedule or scoring behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Lifecycle {
    phase: RunPhase,
}

impl Lifecycle {
    pub const fn new() -> Self {
        Self {
            phase: RunPhase::Discovery,
        }
    }

    pub const fn phase(self) -> RunPhase {
        self.phase
    }

    pub fn begin_development(&mut self) -> Result<(), LifecycleError> {
        self.transition(RunPhase::Development)
    }

    pub fn freeze(&mut self) -> Result<(), LifecycleError> {
        self.transition(RunPhase::Frozen)
    }

    pub fn begin_final_evaluation(&mut self) -> Result<(), LifecycleError> {
        self.transition(RunPhase::FinalEvaluation)
    }

    fn transition(&mut self, to: RunPhase) -> Result<(), LifecycleError> {
        let valid = matches!(
            (self.phase, to),
            (RunPhase::Discovery, RunPhase::Development)
                | (RunPhase::Development, RunPhase::Frozen)
                | (RunPhase::Frozen, RunPhase::FinalEvaluation)
        );
        if !valid {
            return Err(LifecycleError::InvalidTransition {
                from: self.phase,
                to,
            });
        }
        self.phase = to;
        Ok(())
    }
}

impl Default for Lifecycle {
    fn default() -> Self {
        Self::new()
    }
}

/// Public lifecycle seam for a caller-owned controller.
///
/// Implementations decide how a learner and an [`Environment`] are driven in
/// each phase. The framework supplies no benchmark schedule and does not
/// expose final labels to the learner through this trait.
pub trait RunController {
    type Error: std::error::Error;

    fn phase(&self) -> RunPhase;
    fn discovery(&mut self) -> Result<(), Self::Error>;
    fn development(&mut self) -> Result<(), Self::Error>;
    fn freeze(&mut self) -> Result<(), Self::Error>;
    fn final_evaluation(&mut self) -> Result<(), Self::Error>;
}

/// The learner traits are re-exported from the runtime module rather than
/// copied. Frozen policy selection remains non-mutating (`FrozenLearner::step`
/// takes `&self`).
pub use crate::framework::{
    FrozenLearner, Learner, LearnerFreezeFailure, LedgerDecision, LedgerFrozenLearner,
    LedgerLearner,
};
