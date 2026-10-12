use std::fmt;

use sparse_structural_rule_learner::runtime::{
    apply_environment, Environment, EnvironmentActionError, EnvironmentTurn, FrozenLearner,
    InteractionLedger, Learner, LedgerEvent, Lifecycle, LifecycleError, RunController, RunPhase,
};
use sparse_structural_rule_learner::{
    ActMode, ActionId, AuditReferences, Decision, ObservationPacket, PreviousTerminal,
    ProtocolError, ScopeToken, StepInput, StepResult, TerminalStatus,
};

fn packet(first: u64, second: u64, third: u64) -> ObservationPacket {
    ObservationPacket::new([
        sparse_structural_rule_learner::AtomId::new(first),
        sparse_structural_rule_learner::AtomId::new(second),
        sparse_structural_rule_learner::AtomId::new(third),
    ])
}

#[derive(Debug)]
enum FakeError {
    Protocol(ProtocolError),
}

impl fmt::Display for FakeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for FakeError {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FakeLearner {
    observed: Vec<StepResult>,
    step_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FakeFrozenLearner {
    frozen_step_count: u64,
    observed_count: usize,
}

impl Learner for FakeLearner {
    type Frozen = FakeFrozenLearner;

    fn step(&mut self, input: StepInput) -> Result<Decision, ProtocolError> {
        self.step_count += 1;
        Decision::act(
            input.available_actions()[0],
            None,
            ActMode::Fallback,
            AuditReferences::empty(),
        )
    }

    fn observe(&mut self, result: StepResult) -> Result<(), ProtocolError> {
        self.observed.push(result);
        Ok(())
    }

    fn freeze(self) -> Self::Frozen {
        FakeFrozenLearner {
            frozen_step_count: self.step_count,
            observed_count: self.observed.len(),
        }
    }
}

impl FrozenLearner for FakeFrozenLearner {
    fn step(&self, input: StepInput) -> Result<Decision, ProtocolError> {
        Decision::act(
            input.available_actions()[0],
            None,
            ActMode::Fallback,
            AuditReferences::empty(),
        )
    }
}

#[derive(Debug, Default)]
struct FakeEnvironment {
    applied_actions: Vec<ActionId>,
}

impl Environment for FakeEnvironment {
    type Error = FakeError;

    fn reset(&mut self, ledger: &InteractionLedger) -> Result<StepInput, Self::Error> {
        StepInput::new(
            ledger.run_scope(),
            ScopeToken::new(2),
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
        turn.resolve(|action| {
            self.applied_actions.push(action);
            StepResult::new(packet(7, 9, 7), TerminalStatus::Success).map_err(FakeError::Protocol)
        })?;
        Ok(())
    }
}

#[derive(Debug, Default)]
struct NonResolvingEnvironment;

impl Environment for NonResolvingEnvironment {
    type Error = FakeError;

    fn reset(&mut self, ledger: &InteractionLedger) -> Result<StepInput, Self::Error> {
        FakeEnvironment::default().reset(ledger)
    }

    fn apply(
        &mut self,
        _turn: &mut EnvironmentTurn<'_>,
    ) -> Result<(), EnvironmentActionError<Self::Error>> {
        Ok(())
    }
}

#[test]
fn environment_turn_records_decision_before_action_and_result() {
    let mut learner = FakeLearner {
        observed: Vec::new(),
        step_count: 0,
    };
    let mut environment = FakeEnvironment::default();
    let mut ledger = InteractionLedger::new(ScopeToken::new(1)).unwrap();
    let input = environment.reset(&ledger).unwrap();
    let decision = learner.step(input).unwrap();
    let receipt = ledger.record_decision(input, decision).unwrap();

    assert_eq!(ledger.decision_count(), 1);
    assert_eq!(ledger.interaction_count(), 0);
    assert!(ledger.pending_decision().is_some());

    let result = apply_environment(&mut environment, &mut ledger, &receipt)
        .unwrap()
        .unwrap();
    learner.observe(result).unwrap();

    assert_eq!(environment.applied_actions, vec![ActionId::new(0)]);
    assert_eq!(learner.observed, vec![result]);
    assert_eq!(ledger.interaction_count(), 1);
    assert!(ledger.pending_decision().is_none());
    assert_eq!(ledger.events().len(), 2);
    assert!(matches!(ledger.events()[0], LedgerEvent::Decision(_)));
}

#[test]
fn environment_cannot_apply_without_a_pending_receipt_or_complete_a_turn() {
    let mut environment = FakeEnvironment::default();
    let mut ledger = InteractionLedger::new(ScopeToken::new(10)).unwrap();
    let input = environment.reset(&ledger).unwrap();
    let receipt = ledger
        .record_decision(
            input,
            Decision::act(
                ActionId::new(0),
                None,
                ActMode::Fallback,
                AuditReferences::empty(),
            )
            .unwrap(),
        )
        .unwrap();
    apply_environment(&mut environment, &mut ledger, &receipt).unwrap();

    let before = environment.applied_actions.len();
    assert!(matches!(
        apply_environment(&mut environment, &mut ledger, &receipt),
        Err(EnvironmentActionError::Ledger(
            sparse_structural_rule_learner::LedgerError::NoPendingDecision
        ))
    ));
    assert_eq!(environment.applied_actions.len(), before);

    let mut second_ledger = InteractionLedger::new(ScopeToken::new(20)).unwrap();
    let second_input = NonResolvingEnvironment.reset(&second_ledger).unwrap();
    let second_receipt = second_ledger
        .record_decision(
            second_input,
            Decision::act(
                ActionId::new(0),
                None,
                ActMode::Fallback,
                AuditReferences::empty(),
            )
            .unwrap(),
        )
        .unwrap();
    let mut non_resolving = NonResolvingEnvironment;
    assert!(matches!(
        apply_environment(&mut non_resolving, &mut second_ledger, &second_receipt),
        Err(EnvironmentActionError::Ledger(
            sparse_structural_rule_learner::LedgerError::EnvironmentResultMissing
        ))
    ));
    assert_eq!(second_ledger.interaction_count(), 0);
}

#[test]
fn abstention_is_closed_without_invoking_environment_action() {
    let mut environment = FakeEnvironment::default();
    let mut ledger = InteractionLedger::new(ScopeToken::new(40)).unwrap();
    let input = environment.reset(&ledger).unwrap();
    let receipt = ledger
        .record_decision(
            input,
            Decision::abstain(
                sparse_structural_rule_learner::AbstainReason::InsufficientEvidence,
                AuditReferences::empty(),
            ),
        )
        .unwrap();

    let result = apply_environment(&mut environment, &mut ledger, &receipt).unwrap();

    assert_eq!(result, None);
    assert!(environment.applied_actions.is_empty());
    assert_eq!(
        ledger.interactions()[0].evidence().terminal(),
        TerminalStatus::Incomplete
    );
    assert!(ledger.interactions()[0].evidence().after().is_none());
}

#[test]
fn frozen_learner_step_does_not_mutate_frozen_policy() {
    let learner = FakeLearner {
        observed: vec![StepResult::new(packet(1, 2, 1), TerminalStatus::Success).unwrap()],
        step_count: 3,
    };
    let frozen = learner.freeze();
    let before = frozen.clone();
    let input = StepInput::new(
        ScopeToken::new(30),
        ScopeToken::new(31),
        0,
        packet(4, 5, 4),
        [ActionId::new(0), ActionId::new(1)],
        2,
        PreviousTerminal::None,
    )
    .unwrap();

    let decision = frozen.step(input).unwrap();

    assert_eq!(frozen, before);
    assert_eq!(decision.action(), Some(ActionId::new(0)));
}

#[test]
fn existing_v2_evaluator_remains_usable_through_environment_seam() {
    let mut world = sparse_structural_rule_learner::v2::evaluator_world::EvaluatorWorld::primary();
    let mut ledger = InteractionLedger::new(world.run_scope()).unwrap();
    let input = Environment::reset(&mut world, &ledger).unwrap();
    let decision = Decision::act(
        ActionId::new(0),
        None,
        ActMode::Fallback,
        AuditReferences::empty(),
    )
    .unwrap();
    let receipt = ledger.record_decision(input, decision).unwrap();

    let result = apply_environment(&mut world, &mut ledger, &receipt)
        .unwrap()
        .unwrap();

    assert_eq!(result.terminal(), TerminalStatus::Continue);
    assert_eq!(ledger.interaction_count(), 1);
}

#[derive(Debug)]
struct FakeController {
    lifecycle: Lifecycle,
    calls: Vec<&'static str>,
}

impl RunController for FakeController {
    type Error = LifecycleError;

    fn phase(&self) -> RunPhase {
        self.lifecycle.phase()
    }

    fn discovery(&mut self) -> Result<(), Self::Error> {
        self.calls.push("discovery");
        Ok(())
    }

    fn development(&mut self) -> Result<(), Self::Error> {
        self.calls.push("development");
        self.lifecycle.begin_development()
    }

    fn freeze(&mut self) -> Result<(), Self::Error> {
        self.calls.push("freeze");
        self.lifecycle.freeze()
    }

    fn final_evaluation(&mut self) -> Result<(), Self::Error> {
        self.calls.push("final_evaluation");
        self.lifecycle.begin_final_evaluation()
    }
}

#[test]
fn controller_lifecycle_is_generic_and_ordered() {
    let mut controller = FakeController {
        lifecycle: Lifecycle::new(),
        calls: Vec::new(),
    };
    controller.discovery().unwrap();
    controller.development().unwrap();
    controller.freeze().unwrap();
    controller.final_evaluation().unwrap();

    assert_eq!(controller.phase(), RunPhase::FinalEvaluation);
    assert_eq!(
        controller.calls,
        vec!["discovery", "development", "freeze", "final_evaluation"]
    );
    assert!(matches!(
        controller.lifecycle.freeze(),
        Err(LifecycleError::InvalidTransition {
            from: RunPhase::FinalEvaluation,
            to: RunPhase::Frozen
        })
    ));
}

#[test]
fn protocol_errors_are_public_and_descriptive() {
    let error = ProtocolError::InvalidPacketLength {
        expected: 3,
        actual: 2,
    };
    assert_eq!(error.to_string(), "expected packet length 3, got 2");
    assert!(matches!(
        ObservationPacket::try_from_slice(&[
            sparse_structural_rule_learner::AtomId::new(1),
            sparse_structural_rule_learner::AtomId::new(2),
        ]),
        Err(ProtocolError::InvalidPacketLength {
            expected: 3,
            actual: 2
        })
    ));
}
