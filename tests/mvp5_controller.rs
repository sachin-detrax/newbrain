use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use sparse_structural_rule_learner::framework::{
    AbstainReason, ActMode, ActionId, AuditReferences, BudgetFailure, ControllerError, Decision,
    EpisodeBudget, FailureClassification, FrozenLearner, GenericRunController, InteractionLedger,
    Learner, LearnerFreezeFailure, LedgerDecision, LedgerEvent, LedgerFrozenLearner, LedgerLearner,
    ObservationPacket, PreviousTerminal, ProtocolError, ScopeToken, StepInput, StepResult,
    TerminalStatus,
};
use sparse_structural_rule_learner::runtime::{
    Environment, EnvironmentActionError, EnvironmentTurn, RunPhase,
};

fn packet(a: u64, b: u64, c: u64) -> ObservationPacket {
    ObservationPacket::new([
        sparse_structural_rule_learner::AtomId::new(a),
        sparse_structural_rule_learner::AtomId::new(b),
        sparse_structural_rule_learner::AtomId::new(c),
    ])
}

#[derive(Debug)]
enum FakeError {
    Failed,
    Protocol(ProtocolError),
}

impl fmt::Display for FakeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed => formatter.write_str("fake environment failed"),
            Self::Protocol(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for FakeError {}

#[derive(Clone, Copy)]
enum EnvironmentMode {
    Success,
    ContinueThenSuccess,
    TerminalFailure,
    Error,
    Unresolved,
}

struct FakeEnvironment {
    next_episode: u64,
    step_in_episode: u8,
    mode: EnvironmentMode,
    closure_calls: u64,
    events: Rc<RefCell<Vec<&'static str>>>,
}

impl FakeEnvironment {
    fn new(mode: EnvironmentMode, events: Rc<RefCell<Vec<&'static str>>>) -> Self {
        Self {
            next_episode: 0,
            step_in_episode: 0,
            mode,
            closure_calls: 0,
            events,
        }
    }
}

impl Environment for FakeEnvironment {
    type Error = FakeError;

    fn reset(&mut self, ledger: &InteractionLedger) -> Result<StepInput, Self::Error> {
        self.next_episode += 1;
        self.step_in_episode = 0;
        StepInput::new(
            ledger.run_scope(),
            ScopeToken::new(100 + self.next_episode),
            0,
            packet(7, 8, 7),
            [ActionId::new(0), ActionId::new(1)],
            2,
            PreviousTerminal::None,
        )
        .map_err(FakeError::Protocol)
    }

    fn apply(
        &mut self,
        turn: &mut EnvironmentTurn<'_>,
    ) -> Result<(), EnvironmentActionError<Self::Error>> {
        self.events.borrow_mut().push("environment-apply");
        match self.mode {
            EnvironmentMode::Error => Err(EnvironmentActionError::Environment(FakeError::Failed)),
            EnvironmentMode::Unresolved => Ok(()),
            mode => {
                turn.resolve(|_| {
                    self.events.borrow_mut().push("environment-resolve");
                    self.closure_calls += 1;
                    let terminal = match mode {
                        EnvironmentMode::ContinueThenSuccess if self.step_in_episode == 0 => {
                            TerminalStatus::Continue
                        }
                        EnvironmentMode::TerminalFailure if self.step_in_episode == 0 => {
                            TerminalStatus::Continue
                        }
                        EnvironmentMode::TerminalFailure => TerminalStatus::Failure,
                        _ => TerminalStatus::Success,
                    };
                    self.step_in_episode += 1;
                    StepResult::new(packet(7, 9, 7), terminal).map_err(FakeError::Protocol)
                })?;
                Ok(())
            }
        }
    }
}

struct FakeLearner {
    abstain: Option<AbstainReason>,
    policy_updates: u64,
    observed: Vec<StepResult>,
    events: Rc<RefCell<Vec<&'static str>>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FakeFrozenLearner {
    abstain: Option<AbstainReason>,
    policy_updates: u64,
    observed_count: usize,
}

impl FakeLearner {
    fn new(abstain: Option<AbstainReason>, events: Rc<RefCell<Vec<&'static str>>>) -> Self {
        Self {
            abstain,
            policy_updates: 0,
            observed: Vec::new(),
            events,
        }
    }

    fn decision(&self, input: StepInput) -> Result<Decision, ProtocolError> {
        match self.abstain {
            Some(reason) => Ok(Decision::abstain(reason, AuditReferences::empty())),
            None => Decision::act(
                input.available_actions()[0],
                None,
                ActMode::Fallback,
                AuditReferences::empty(),
            ),
        }
    }
}

impl Learner for FakeLearner {
    type Frozen = FakeFrozenLearner;

    fn step(&mut self, input: StepInput) -> Result<Decision, ProtocolError> {
        self.events.borrow_mut().push("learner-step");
        self.policy_updates += 1;
        self.decision(input)
    }

    fn observe(&mut self, result: StepResult) -> Result<(), ProtocolError> {
        self.events.borrow_mut().push("learner-observe");
        self.observed.push(result);
        Ok(())
    }

    fn freeze(self) -> Self::Frozen {
        FakeFrozenLearner {
            abstain: self.abstain,
            policy_updates: self.policy_updates,
            observed_count: self.observed.len(),
        }
    }
}

impl FrozenLearner for FakeFrozenLearner {
    fn step(&self, input: StepInput) -> Result<Decision, ProtocolError> {
        match self.abstain {
            Some(reason) => Ok(Decision::abstain(reason, AuditReferences::empty())),
            None => Decision::act(
                input.available_actions()[0],
                None,
                ActMode::Fallback,
                AuditReferences::empty(),
            ),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum LifecycleLearnerError {
    EnterDevelopment,
    Freeze,
}

impl fmt::Display for LifecycleLearnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EnterDevelopment => formatter.write_str("enter development failed"),
            Self::Freeze => formatter.write_str("freeze failed"),
        }
    }
}

impl std::error::Error for LifecycleLearnerError {}

struct FailingLedgerLearner {
    fail_enter: bool,
    fail_freeze: bool,
    enter_calls: u8,
    freeze_calls: u8,
}

struct FailingFrozenLearner;

impl LedgerLearner for FailingLedgerLearner {
    type Error = LifecycleLearnerError;
    type Frozen = FailingFrozenLearner;

    fn step_with_ledger(
        &mut self,
        _ledger: &mut InteractionLedger,
        input: StepInput,
    ) -> Result<LedgerDecision, Self::Error> {
        Ok(LedgerDecision::Unrecorded(
            Decision::act(
                input.available_actions()[0],
                None,
                ActMode::Fallback,
                AuditReferences::empty(),
            )
            .expect("available action is valid"),
        ))
    }

    fn observe_with_ledger(
        &mut self,
        _ledger: &mut InteractionLedger,
        _result: Option<StepResult>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn enter_development(&mut self) -> Result<(), Self::Error> {
        self.enter_calls += 1;
        if self.fail_enter {
            Err(LifecycleLearnerError::EnterDevelopment)
        } else {
            Ok(())
        }
    }

    fn freeze_with_ledger(
        self,
        _ledger: &InteractionLedger,
    ) -> Result<Self::Frozen, LearnerFreezeFailure<Self, Self::Error>> {
        let mut learner = self;
        learner.freeze_calls += 1;
        if learner.fail_freeze {
            Err(LearnerFreezeFailure::new(
                learner,
                LifecycleLearnerError::Freeze,
            ))
        } else {
            Ok(FailingFrozenLearner)
        }
    }
}

impl LedgerFrozenLearner for FailingFrozenLearner {
    type Error = LifecycleLearnerError;

    fn step_with_ledger(
        &self,
        _ledger: &mut InteractionLedger,
        input: StepInput,
    ) -> Result<LedgerDecision, Self::Error> {
        Ok(LedgerDecision::Unrecorded(
            Decision::act(
                input.available_actions()[0],
                None,
                ActMode::Fallback,
                AuditReferences::empty(),
            )
            .expect("available action is valid"),
        ))
    }
}

fn controller(
    mode: EnvironmentMode,
    budget: EpisodeBudget,
    abstain: Option<AbstainReason>,
) -> (
    GenericRunController<FakeLearner, FakeEnvironment>,
    Rc<RefCell<Vec<&'static str>>>,
) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let learner = FakeLearner::new(abstain, Rc::clone(&events));
    let environment = FakeEnvironment::new(mode, Rc::clone(&events));
    let controller =
        GenericRunController::with_budget(learner, environment, ScopeToken::new(1), budget)
            .unwrap();
    (controller, events)
}

fn failing_ledger_controller(
    fail_enter: bool,
    fail_freeze: bool,
) -> GenericRunController<FailingLedgerLearner, FakeEnvironment> {
    GenericRunController::with_budget(
        FailingLedgerLearner {
            fail_enter,
            fail_freeze,
            enter_calls: 0,
            freeze_calls: 0,
        },
        FakeEnvironment::new(EnvironmentMode::Success, Rc::new(RefCell::new(Vec::new()))),
        ScopeToken::new(1),
        EpisodeBudget::new(0, 0, 0),
    )
    .unwrap()
}

#[test]
fn happy_lifecycle_runs_a_custom_environment_without_v2_imports() {
    let (mut controller, _events) = controller(
        EnvironmentMode::ContinueThenSuccess,
        EpisodeBudget::new(1, 1, 1),
        None,
    );

    controller.discovery().unwrap();
    assert_eq!(controller.phase(), RunPhase::Discovery);
    controller.development().unwrap();
    assert_eq!(controller.phase(), RunPhase::Development);
    controller.freeze().unwrap();
    assert_eq!(controller.phase(), RunPhase::Frozen);
    controller.final_evaluation().unwrap();
    assert_eq!(controller.phase(), RunPhase::FinalEvaluation);

    assert_eq!(controller.summary().episodes.len(), 3);
    assert_eq!(
        controller
            .summary()
            .episodes
            .iter()
            .map(|episode| episode.phase)
            .collect::<Vec<_>>(),
        vec![
            RunPhase::Discovery,
            RunPhase::Development,
            RunPhase::FinalEvaluation
        ]
    );
    assert_eq!(controller.summary().decision_count(), 6);
    assert_eq!(controller.summary().action_count(), 6);
    assert_eq!(controller.ledger().interaction_count(), 6);
}

#[test]
fn decision_is_recorded_before_environment_result_and_mutable_observe_follows_it() {
    let (mut controller, events) =
        controller(EnvironmentMode::Success, EpisodeBudget::new(1, 0, 0), None);
    controller.discovery().unwrap();

    assert_eq!(
        &*events.borrow(),
        &[
            "learner-step",
            "environment-apply",
            "environment-resolve",
            "learner-observe"
        ]
    );
    assert!(matches!(
        controller.ledger().events()[0],
        LedgerEvent::Decision(_)
    ));
    assert!(matches!(
        controller.ledger().events()[1],
        LedgerEvent::Interaction(_)
    ));
    assert_eq!(controller.ledger().interaction_count(), 1);
}

#[test]
fn abstention_closes_as_incomplete_without_a_fabricated_packet() {
    let (mut controller, _events) = controller(
        EnvironmentMode::Success,
        EpisodeBudget::new(1, 0, 0),
        Some(AbstainReason::InsufficientEvidence),
    );
    controller.discovery().unwrap();

    let episode = &controller.summary().episodes[0];
    assert_eq!(episode.decision_count, 1);
    assert_eq!(episode.action_count, 0);
    assert_eq!(episode.abstention_count, 1);
    assert_eq!(episode.terminal, TerminalStatus::Incomplete);
    assert_eq!(
        episode.failure_classifications,
        vec![FailureClassification::InsufficientEvidence]
    );
    assert_eq!(controller.environment().closure_calls, 0);
    let evidence = controller.ledger().interactions()[0].evidence();
    assert_eq!(evidence.terminal(), TerminalStatus::Incomplete);
    assert_eq!(evidence.after(), None);
}

#[test]
fn final_evaluation_does_not_observe_or_mutate_the_frozen_learner() {
    let (mut controller, _events) =
        controller(EnvironmentMode::Success, EpisodeBudget::new(1, 0, 1), None);
    controller.discovery().unwrap();
    controller.development().unwrap();
    controller.freeze().unwrap();
    let before = controller.frozen_learner().unwrap().clone();

    controller.final_evaluation().unwrap();

    assert_eq!(controller.frozen_learner().unwrap(), &before);
    assert_eq!(before.observed_count, 1);
    assert_eq!(
        controller.summary().episodes[1].phase,
        RunPhase::FinalEvaluation
    );
    assert_eq!(
        controller.summary().episodes[1].terminal,
        TerminalStatus::Success
    );
}

#[test]
fn out_of_order_phase_calls_are_rejected() {
    let (mut controller, _events) =
        controller(EnvironmentMode::Success, EpisodeBudget::new(1, 1, 1), None);

    assert!(matches!(
        controller.development(),
        Err(ControllerError::InvalidPhase {
            expected: RunPhase::Development,
            actual: RunPhase::Discovery
        })
    ));
    assert!(matches!(
        controller.freeze(),
        Err(ControllerError::InvalidPhase {
            expected: RunPhase::Development,
            actual: RunPhase::Discovery
        })
    ));
    assert!(matches!(
        controller.final_evaluation(),
        Err(ControllerError::InvalidPhase {
            expected: RunPhase::Frozen,
            actual: RunPhase::Discovery
        })
    ));

    controller.discovery().unwrap();
    assert!(matches!(
        controller.discovery(),
        Err(ControllerError::InvalidPhase {
            expected: RunPhase::Discovery,
            actual: RunPhase::Discovery
        })
    ));
}

#[test]
fn failed_development_hook_does_not_commit_the_lifecycle_phase() {
    let mut controller = failing_ledger_controller(true, false);
    controller.discovery().unwrap();

    assert!(matches!(
        controller.development(),
        Err(ControllerError::Learner(
            LifecycleLearnerError::EnterDevelopment
        ))
    ));
    assert_eq!(controller.phase(), RunPhase::Discovery);
    assert!(controller.learner().is_some());
    assert!(!controller.ledger().is_frozen());
    assert_eq!(controller.learner().unwrap().enter_calls, 1);

    controller
        .learner_and_ledger_mut()
        .expect("the mutable learner is preserved")
        .0
        .fail_enter = false;
    controller.development().unwrap();
    assert_eq!(controller.phase(), RunPhase::Development);
}

#[test]
fn failed_freeze_hook_preserves_mutable_learner_phase_and_ledger() {
    let mut controller = failing_ledger_controller(false, true);
    controller.discovery().unwrap();
    controller.development().unwrap();

    assert!(matches!(
        controller.freeze(),
        Err(ControllerError::Learner(LifecycleLearnerError::Freeze))
    ));
    assert_eq!(controller.phase(), RunPhase::Development);
    assert!(controller.learner().is_some());
    assert!(controller.frozen_learner().is_none());
    assert!(!controller.ledger().is_frozen());
    assert_eq!(controller.learner().unwrap().freeze_calls, 1);

    controller
        .learner_and_ledger_mut()
        .expect("the mutable learner is preserved")
        .0
        .fail_freeze = false;
    controller.freeze().unwrap();
    assert_eq!(controller.phase(), RunPhase::Frozen);
    assert!(controller.learner().is_none());
    assert!(controller.frozen_learner().is_some());
    assert!(controller.ledger().is_frozen());
}

#[test]
fn episode_counts_are_configurable_without_a_benchmark_schedule() {
    let (mut controller, _events) =
        controller(EnvironmentMode::Success, EpisodeBudget::new(2, 3, 1), None);
    controller.discovery().unwrap();
    controller.development().unwrap();
    controller.freeze().unwrap();
    controller.final_evaluation().unwrap();

    assert_eq!(controller.summary().episodes.len(), 6);
    assert_eq!(
        controller
            .summary()
            .episodes
            .iter()
            .filter(|episode| episode.phase == RunPhase::Discovery)
            .count(),
        2
    );
    assert_eq!(
        controller
            .summary()
            .episodes
            .iter()
            .filter(|episode| episode.phase == RunPhase::Development)
            .count(),
        3
    );
    assert_eq!(
        controller
            .summary()
            .episodes
            .iter()
            .filter(|episode| episode.phase == RunPhase::FinalEvaluation)
            .count(),
        1
    );
}

#[test]
fn typed_failures_preserve_environment_unresolved_and_budget_classifications() {
    let (mut environment_failure, _events) =
        controller(EnvironmentMode::Error, EpisodeBudget::new(1, 0, 0), None);
    let error = environment_failure.discovery().unwrap_err();
    assert!(matches!(
        error,
        ControllerError::Environment(FakeError::Failed)
    ));
    assert_eq!(
        error.failure_classification(),
        Some(FailureClassification::EnvironmentFailure)
    );

    let (mut unresolved, _events) = controller(
        EnvironmentMode::Unresolved,
        EpisodeBudget::new(1, 0, 0),
        None,
    );
    let error = unresolved.discovery().unwrap_err();
    assert!(matches!(
        error,
        ControllerError::UnresolvedDecision { sequence: 1 }
    ));
    assert_eq!(
        error.failure_classification(),
        Some(FailureClassification::UnresolvedDecision)
    );

    let (mut budget, _events) =
        controller(EnvironmentMode::Success, EpisodeBudget::new(73, 0, 0), None);
    let error = budget.discovery().unwrap_err();
    assert!(matches!(
        error,
        ControllerError::Budget(BudgetFailure::LogicalLimit(limit))
            if limit.resource() == sparse_structural_rule_learner::framework::LogicalResource::QueryCalls
    ));
    assert_eq!(
        error.failure_classification(),
        Some(FailureClassification::BudgetExhausted)
    );
    assert_eq!(
        budget.summary().failure_classifications,
        vec![FailureClassification::BudgetExhausted]
    );
}

#[test]
fn public_terminal_failure_is_reported_as_an_incorrect_hypothesis_outcome() {
    let (mut controller, _events) = controller(
        EnvironmentMode::TerminalFailure,
        EpisodeBudget::new(1, 0, 0),
        None,
    );
    controller.discovery().unwrap();

    assert_eq!(
        controller.summary().episodes[0].failure_classifications,
        vec![FailureClassification::IncorrectHypothesis]
    );
}
