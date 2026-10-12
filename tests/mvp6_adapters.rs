use sparse_structural_rule_learner::framework::{
    ActMode, ActionId, CandidateStage, Decision, EpisodeBudget, GrammarV2, GroundingLearner,
    HypothesisId, InteractionLedger, LedgerEvent, ObservationPacket, StepInput, TerminalStatus,
};
use sparse_structural_rule_learner::mvp6::{StructuralToyEnvironment, V2EnvironmentAdapter};
use sparse_structural_rule_learner::runtime::{
    apply_environment, Environment, GenericRunController, RunPhase,
};
use sparse_structural_rule_learner::AuditReferences;

fn fallback_decision(_input: StepInput, action: ActionId) -> Decision {
    Decision::act(action, None, ActMode::Fallback, AuditReferences::empty())
        .expect("the available action is valid")
}

fn next_input(input: StepInput, result: sparse_structural_rule_learner::StepResult) -> StepInput {
    StepInput::new(
        input.run_scope(),
        input.episode_scope(),
        1,
        result.observation(),
        input.available_actions(),
        1,
        sparse_structural_rule_learner::PreviousTerminal::Continue,
    )
    .unwrap()
}

#[test]
fn adapter_a_preserves_the_existing_v2_environment_contract() {
    let mut environment = V2EnvironmentAdapter::primary();
    let mut ledger = InteractionLedger::new(environment.run_scope()).unwrap();
    let input = environment.reset(&ledger).unwrap();
    let action = input.available_actions()[0];
    let receipt = ledger
        .record_decision(input, fallback_decision(input, action))
        .unwrap();

    let result = apply_environment(&mut environment, &mut ledger, &receipt)
        .unwrap()
        .unwrap();

    assert_eq!(result.terminal(), TerminalStatus::Continue);
    assert_eq!(ledger.interaction_count(), 1);
    assert!(matches!(ledger.events()[0], LedgerEvent::Decision(_)));
    assert!(matches!(ledger.events()[1], LedgerEvent::Interaction(_)));
}

#[test]
fn adapter_b_has_different_two_step_dynamics_and_an_opaque_boundary() {
    let mut environment = StructuralToyEnvironment::new();
    let mut ledger = InteractionLedger::new(environment.run_scope()).unwrap();
    let first = environment.reset(&ledger).unwrap();
    let [prime, commit] = first.available_actions();

    let first_receipt = ledger
        .record_decision(first, fallback_decision(first, prime))
        .unwrap();
    let first_result = apply_environment(&mut environment, &mut ledger, &first_receipt)
        .unwrap()
        .unwrap();
    assert_eq!(first_result.terminal(), TerminalStatus::Continue);
    assert_eq!(first.observation().atom(0), first.observation().atom(2));
    assert_eq!(
        first_result.observation().atom(0),
        first.observation().atom(0)
    );
    assert_ne!(
        first_result.observation().atom(1),
        first.observation().atom(1)
    );

    let second = next_input(first, first_result);
    let second_receipt = ledger
        .record_decision(second, fallback_decision(second, commit))
        .unwrap();
    let second_result = apply_environment(&mut environment, &mut ledger, &second_receipt)
        .unwrap()
        .unwrap();
    assert_eq!(second_result.terminal(), TerminalStatus::Success);

    let trace = ledger.decision_trace_jsonl();
    for evaluator_label in [
        "HiddenAtom",
        "HiddenAction",
        "North",
        "Dormant",
        "Armed",
        "Lit",
        "score",
    ] {
        assert!(!trace.contains(evaluator_label), "leaked {evaluator_label}");
    }
}

fn run_controller_a() -> (String, sparse_structural_rule_learner::RunSummary) {
    let environment = V2EnvironmentAdapter::primary();
    let run_scope = environment.run_scope();
    let learner = GroundingLearner::new(run_scope).unwrap();
    let mut controller = GenericRunController::with_budget(
        learner,
        environment,
        run_scope,
        EpisodeBudget::new(1, 1, 1),
    )
    .unwrap();
    controller.discovery().unwrap();
    controller.development().unwrap();
    controller.freeze().unwrap();
    controller.final_evaluation().unwrap();
    (
        controller.ledger().decision_trace_jsonl().to_owned(),
        controller.summary().clone(),
    )
}

fn run_controller_b() -> (String, sparse_structural_rule_learner::RunSummary, String) {
    let environment = StructuralToyEnvironment::new();
    let run_scope = environment.run_scope();
    let learner = GroundingLearner::new(run_scope).unwrap();
    let mut controller = GenericRunController::with_budget(
        learner,
        environment,
        run_scope,
        EpisodeBudget::new(1, 1, 1),
    )
    .unwrap();
    controller.discovery().unwrap();
    controller.development().unwrap();
    controller.freeze().unwrap();
    controller.final_evaluation().unwrap();
    (
        controller.ledger().decision_trace_jsonl().to_owned(),
        controller.summary().clone(),
        format!("{:?}", controller.frozen_learner().unwrap()),
    )
}

#[test]
fn the_same_generic_controller_runs_both_adapters_with_the_real_learner() {
    let (a_trace, a_summary) = run_controller_a();
    let (b_trace, b_summary, _) = run_controller_b();

    assert_eq!(a_summary.episodes.len(), 3);
    assert_eq!(b_summary.episodes.len(), 3);
    assert!(a_summary.decision_count() > 0);
    assert!(b_summary.decision_count() > 0);
    assert_eq!(a_summary.episodes[0].phase, RunPhase::Discovery);
    assert_eq!(b_summary.episodes[2].phase, RunPhase::FinalEvaluation);
    assert!(!a_trace.is_empty());
    assert!(!b_trace.is_empty());
}

#[test]
fn both_adapters_are_deterministic_through_the_controller() {
    assert_eq!(run_controller_a(), run_controller_a());
    assert_eq!(run_controller_b(), run_controller_b());
}

#[test]
fn structural_learner_transfers_a_retained_relation_to_a_held_out_context() {
    let mut environment = StructuralToyEnvironment::new();
    let run_scope = environment.run_scope();
    let mut ledger = InteractionLedger::new(run_scope).unwrap();
    let mut learner = GroundingLearner::new(run_scope).unwrap();

    for _ in 0..3 {
        let mut input = environment.reset(&ledger).unwrap();
        loop {
            let recorded = learner.step(&mut ledger, input).unwrap();
            let result =
                apply_environment(&mut environment, &mut ledger, &recorded.receipt).unwrap();
            learner.observe(&mut ledger, result).unwrap();
            match result {
                Some(result) if result.terminal() == TerminalStatus::Continue => {
                    input = next_input(input, result);
                }
                _ => break,
            }
        }
    }

    let mut grammar = GrammarV2::new(run_scope).unwrap();
    let report = grammar.generate(&ledger);
    let candidate = report
        .candidates_at(CandidateStage::E3)
        .into_iter()
        .find(|candidate| candidate.action() == ActionId::new(17))
        .cloned()
        .expect("the prime relation should have an E3 candidate");
    learner.retain_generation(&mut ledger, &report).unwrap();
    learner
        .declare_competing_prediction(
            &mut ledger,
            HypothesisId::new(900),
            candidate.action(),
            ObservationPacket::new([
                sparse_structural_rule_learner::AtomId::new(997),
                sparse_structural_rule_learner::AtomId::new(998),
                sparse_structural_rule_learner::AtomId::new(997),
            ]),
            candidate.support_evidence_ids().to_vec(),
        )
        .unwrap();
    learner
        .declare_hypothesis_separating_action(candidate.action())
        .unwrap();

    let mut grounding_input = environment.reset(&ledger).unwrap();
    let grounding = learner.step(&mut ledger, grounding_input).unwrap();
    assert_eq!(grounding.trace.mode(), Some(ActMode::Exploration));
    let grounding_result =
        apply_environment(&mut environment, &mut ledger, &grounding.receipt).unwrap();
    learner.observe(&mut ledger, grounding_result).unwrap();
    if let Some(result) = grounding_result {
        if result.terminal() == TerminalStatus::Continue {
            grounding_input = next_input(grounding_input, result);
            let second = learner.step(&mut ledger, grounding_input).unwrap();
            let second_result =
                apply_environment(&mut environment, &mut ledger, &second.receipt).unwrap();
            learner.observe(&mut ledger, second_result).unwrap();
        }
    }

    let admitted = match grammar.admit(candidate, &ledger) {
        sparse_structural_rule_learner::framework::AdmissionOutcome::Admitted(value) => value,
        other => panic!("expected the grounded structural candidate, got {other:?}"),
    };
    let procedure = grammar.promote(&mut ledger, admitted).unwrap();
    learner.install_procedure(&mut ledger, procedure).unwrap();
    let frozen = learner.freeze(&mut ledger).unwrap();

    let held_out = environment.reset(&ledger).unwrap();
    let selected = frozen.step(&mut ledger, held_out).unwrap();
    assert_eq!(selected.trace.mode(), Some(ActMode::Procedure));
    let result = apply_environment(&mut environment, &mut ledger, &selected.receipt)
        .unwrap()
        .unwrap();
    assert_eq!(result.terminal(), TerminalStatus::Continue);
    assert_eq!(
        selected.trace.decision.prediction(),
        Some(result.observation())
    );
}

#[test]
fn learner_trace_contains_opaque_protocol_values_only() {
    let (trace, _, learner_state) = run_controller_b();
    for serialized in [trace, learner_state] {
        assert!(!serialized.contains("StructuralToyEnvironment"));
        assert!(!serialized.contains("TOY_SCOPE_SEED"));
        assert!(!serialized.contains("North"));
        assert!(!serialized.contains("Success"));
    }
}
