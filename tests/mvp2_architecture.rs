use std::fmt;

use sparse_structural_rule_learner::framework::{
    ActMode, AuditReferences, Decision, FrozenLearner, Learner, ObservationPacket, ProtocolError,
    ScopeToken, StepInput, StepResult, TerminalStatus,
};
use sparse_structural_rule_learner::runtime::{
    apply_environment, Environment, EnvironmentActionError, EnvironmentTurn, InteractionLedger,
};

#[derive(Debug)]
struct GenericError;

impl fmt::Display for GenericError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("generic environment error")
    }
}

impl std::error::Error for GenericError {}

#[derive(Debug, Default)]
struct GenericEnvironment {
    applied: usize,
}

impl Environment for GenericEnvironment {
    type Error = GenericError;

    fn reset(&mut self, ledger: &InteractionLedger) -> Result<StepInput, Self::Error> {
        StepInput::new(
            ledger.run_scope(),
            ScopeToken::new(2),
            0,
            ObservationPacket::new([
                sparse_structural_rule_learner::framework::AtomId::new(7),
                sparse_structural_rule_learner::framework::AtomId::new(8),
                sparse_structural_rule_learner::framework::AtomId::new(7),
            ]),
            [
                sparse_structural_rule_learner::framework::ActionId::new(0),
                sparse_structural_rule_learner::framework::ActionId::new(1),
            ],
            2,
            sparse_structural_rule_learner::framework::PreviousTerminal::None,
        )
        .map_err(|_| GenericError)
    }

    fn apply(
        &mut self,
        turn: &mut EnvironmentTurn<'_>,
    ) -> Result<(), EnvironmentActionError<Self::Error>> {
        turn.resolve(|_| {
            self.applied += 1;
            StepResult::new(
                ObservationPacket::new([
                    sparse_structural_rule_learner::framework::AtomId::new(7),
                    sparse_structural_rule_learner::framework::AtomId::new(9),
                    sparse_structural_rule_learner::framework::AtomId::new(7),
                ]),
                TerminalStatus::Success,
            )
            .map_err(|_| GenericError)
        })?;
        Ok(())
    }
}

#[derive(Debug, Default)]
struct GenericLearner {
    observed: Vec<StepResult>,
}

#[derive(Debug, Eq, PartialEq)]
struct GenericFrozen;

impl Learner for GenericLearner {
    type Frozen = GenericFrozen;

    fn step(&mut self, input: StepInput) -> Result<Decision, ProtocolError> {
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
        GenericFrozen
    }
}

impl FrozenLearner for GenericFrozen {
    fn step(&self, input: StepInput) -> Result<Decision, ProtocolError> {
        Decision::act(
            input.available_actions()[0],
            None,
            ActMode::Fallback,
            AuditReferences::empty(),
        )
    }
}

#[test]
fn generic_environment_and_learner_use_only_framework_runtime_apis() {
    let mut environment = GenericEnvironment::default();
    let mut learner = GenericLearner::default();
    let mut ledger = InteractionLedger::new(ScopeToken::new(1)).unwrap();

    let input = environment.reset(&ledger).unwrap();
    let decision = learner.step(input).unwrap();
    let receipt = ledger.record_decision(input, decision).unwrap();
    let result = apply_environment(&mut environment, &mut ledger, &receipt)
        .unwrap()
        .unwrap();
    learner.observe(result).unwrap();

    assert_eq!(environment.applied, 1);
    assert_eq!(learner.observed, vec![result]);
    assert_eq!(result.terminal(), TerminalStatus::Success);
    assert!(matches!(learner.freeze(), GenericFrozen));
}

#[test]
fn v2_evaluator_bridge_and_experiment_runner_remain_compatible() {
    let report = sparse_structural_rule_learner::v2::experiment::run_one(
        sparse_structural_rule_learner::v2::evaluator_world::RunVariant::Primary,
        sparse_structural_rule_learner::v2::experiment::Predictor::B2EvaluatorController,
    )
    .unwrap();

    assert!(report.freeze_called);
    assert_eq!(
        report.metrics.held_out_applicable_task_success.numerator(),
        6
    );
}

#[test]
fn reusable_framework_sources_do_not_define_benchmark_world_concepts() {
    let reusable_sources = [
        include_str!("../src/framework/mod.rs"),
        include_str!("../src/framework/opaque_protocol.rs"),
        include_str!("../src/framework/interaction_ledger.rs"),
        include_str!("../src/framework/grammar_admission.rs"),
        include_str!("../src/framework/grounding_decision.rs"),
        include_str!("../src/framework/partial_observation.rs"),
        include_str!("../src/framework/resource_scale.rs"),
    ];
    let forbidden_public_concepts = [
        "EvaluatorWorld",
        "HiddenAtom",
        "HiddenAction",
        "RunVariant",
        "fixed_schedule",
        "B2EvaluatorController",
        "schedule_fingerprint",
        "score_episode",
    ];

    for source in reusable_sources {
        for concept in forbidden_public_concepts {
            assert!(
                !source.contains(concept),
                "reusable framework source unexpectedly names {concept}"
            );
        }
    }
}
