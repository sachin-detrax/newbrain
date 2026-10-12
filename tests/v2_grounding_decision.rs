use sparse_structural_rule_learner::v2::changed_rule::ChangedRuleRevision;
use sparse_structural_rule_learner::v2::grammar_admission::{
    AdmissionOutcome, CandidateStage, GrammarV2,
};
use sparse_structural_rule_learner::v2::grounding_decision::{
    assess_partial, ChangeReason, DecisionReason, DerivedStatus, GroundingLearner, LogicalCost,
};
use sparse_structural_rule_learner::v2::interaction_ledger::{AuditScope, InteractionLedger};
use sparse_structural_rule_learner::v2::resource_scale::ResourceMeter;
use sparse_structural_rule_learner::v2::{
    ActMode, ActionId, AtomId, AuditReferences, Decision, EvidenceId, HypothesisId,
    ObservationPacket, PreviousTerminal, ScopeToken, StepInput, StepResult, TerminalStatus,
};

fn packet(a: u64, b: u64, c: u64) -> ObservationPacket {
    ObservationPacket::new([AtomId::new(a), AtomId::new(b), AtomId::new(c)])
}

fn input(
    run: u64,
    episode: u64,
    step: u8,
    observation: ObservationPacket,
    actions: [ActionId; 2],
    remaining: u8,
    previous: PreviousTerminal,
) -> StepInput {
    StepInput::new(
        ScopeToken::new(run),
        ScopeToken::new(episode),
        step,
        observation,
        actions,
        remaining,
        previous,
    )
    .unwrap()
}

fn add_observation(
    ledger: &mut InteractionLedger,
    episode: u64,
    before: ObservationPacket,
    action: u64,
    after: ObservationPacket,
    audit: AuditReferences,
    mode: ActMode,
) {
    let decision = Decision::act(
        ActionId::new(action),
        matches!(mode, ActMode::Procedure).then_some(after),
        mode,
        audit,
    )
    .unwrap();
    let step_input = input(
        ledger.run_scope().value(),
        episode,
        0,
        before,
        [ActionId::new(7), ActionId::new(9)],
        2,
        PreviousTerminal::None,
    );
    let receipt = ledger.record_decision(step_input, decision).unwrap();
    ledger
        .record_observed_result(
            &receipt,
            StepResult::new(after, TerminalStatus::Success).unwrap(),
        )
        .unwrap();
}

fn challenged_ledger() -> InteractionLedger {
    let mut ledger = InteractionLedger::new(ScopeToken::new(900)).unwrap();
    add_observation(
        &mut ledger,
        901,
        packet(10, 20, 10),
        7,
        packet(10, 21, 10),
        AuditReferences::empty(),
        ActMode::Exploration,
    );
    add_observation(
        &mut ledger,
        902,
        packet(11, 20, 11),
        7,
        packet(11, 21, 11),
        AuditReferences::empty(),
        ActMode::Exploration,
    );
    add_observation(
        &mut ledger,
        903,
        packet(10, 21, 10),
        7,
        packet(10, 21, 10),
        AuditReferences::empty(),
        ActMode::Exploration,
    );
    ledger
}

fn admitted_procedure(
    ledger: &mut InteractionLedger,
) -> sparse_structural_rule_learner::v2::grammar_admission::ProcedureRecord {
    let mut grammar = GrammarV2::new(ledger.run_scope()).unwrap();
    let report = grammar.generate(ledger);
    let candidate = report
        .candidates()
        .iter()
        .find(|candidate| candidate.stage() == CandidateStage::E3)
        .unwrap()
        .clone();
    let hypothesis = candidate.hypothesis_id();
    ledger
        .register_hypothesis(hypothesis, AuditScope::Run)
        .unwrap();
    add_observation(
        ledger,
        904,
        packet(99, 20, 99),
        7,
        packet(99, 21, 99),
        AuditReferences::new(Vec::new(), Vec::new(), vec![hypothesis]).unwrap(),
        ActMode::Exploration,
    );
    let admitted = match grammar.admit(candidate, ledger) {
        AdmissionOutcome::Admitted(admitted) => admitted,
        other => panic!("expected admitted E3 candidate, got {other:?}"),
    };
    grammar.promote(ledger, admitted).unwrap()
}

fn complete_action(
    learner: &mut GroundingLearner,
    ledger: &mut InteractionLedger,
    recorded: &sparse_structural_rule_learner::v2::grounding_decision::RecordedDecision,
    result: StepResult,
) {
    ledger
        .record_observed_result(&recorded.receipt, result)
        .unwrap();
    learner.observe(ledger, Some(result)).unwrap();
}

#[test]
fn fallback_uses_first_untried_canonical_action_order() {
    let mut ledger = InteractionLedger::new(ScopeToken::new(100)).unwrap();
    let mut learner = GroundingLearner::new(ledger.run_scope()).unwrap();
    assert_eq!(
        learner.phase(),
        sparse_structural_rule_learner::v2::MutablePhase::Discovery
    );
    learner.enter_development().unwrap();
    let first = learner
        .step(
            &mut ledger,
            input(
                100,
                101,
                0,
                packet(1, 2, 1),
                [ActionId::new(10), ActionId::new(20)],
                2,
                PreviousTerminal::None,
            ),
        )
        .unwrap();
    assert_eq!(first.trace.reason, DecisionReason::Fallback);
    assert_eq!(first.trace.chosen_action(), Some(ActionId::new(10)));
    complete_action(
        &mut learner,
        &mut ledger,
        &first,
        StepResult::new(packet(1, 2, 1), TerminalStatus::Continue).unwrap(),
    );

    let second = learner
        .step(
            &mut ledger,
            input(
                100,
                101,
                1,
                packet(1, 2, 1),
                [ActionId::new(10), ActionId::new(20)],
                1,
                PreviousTerminal::Continue,
            ),
        )
        .unwrap();
    assert_eq!(second.trace.reason, DecisionReason::Fallback);
    assert_eq!(second.trace.chosen_action(), Some(ActionId::new(20)));
}

#[test]
fn admitted_procedure_selects_action_and_prediction_before_reveal() {
    let mut ledger = challenged_ledger();
    let procedure = admitted_procedure(&mut ledger);
    let procedure_id = procedure.procedure_id();
    let mut learner = GroundingLearner::new(ledger.run_scope()).unwrap();
    learner.install_procedure(&mut ledger, procedure).unwrap();

    let recorded = learner
        .step(
            &mut ledger,
            input(
                900,
                905,
                0,
                packet(99, 20, 99),
                [ActionId::new(7), ActionId::new(9)],
                2,
                PreviousTerminal::None,
            ),
        )
        .unwrap();
    assert_eq!(recorded.trace.reason, DecisionReason::Procedure);
    assert_eq!(recorded.trace.mode(), Some(ActMode::Procedure));
    assert_eq!(recorded.trace.chosen_action(), Some(ActionId::new(7)));
    assert_eq!(
        recorded.trace.decision.prediction(),
        Some(packet(99, 21, 99))
    );
    assert_eq!(recorded.trace.procedure_refs(), &[procedure_id]);
    assert_eq!(ledger.interaction_count(), 4);
    assert_eq!(ledger.decision_count(), 5);
}

#[test]
fn conflicting_opaque_hypotheses_abstain_without_a_separator() {
    let mut ledger = InteractionLedger::new(ScopeToken::new(200)).unwrap();
    add_observation(
        &mut ledger,
        201,
        packet(1, 2, 1),
        7,
        packet(1, 2, 1),
        AuditReferences::empty(),
        ActMode::Exploration,
    );
    add_observation(
        &mut ledger,
        202,
        packet(2, 3, 2),
        7,
        packet(2, 3, 2),
        AuditReferences::empty(),
        ActMode::Exploration,
    );
    let mut learner = GroundingLearner::new(ledger.run_scope()).unwrap();
    learner
        .declare_competing_prediction(
            &mut ledger,
            HypothesisId::new(1),
            ActionId::new(20),
            packet(8, 9, 8),
            vec![EvidenceId::new(1)],
        )
        .unwrap();
    learner
        .declare_competing_prediction(
            &mut ledger,
            HypothesisId::new(2),
            ActionId::new(20),
            packet(8, 10, 8),
            vec![EvidenceId::new(2)],
        )
        .unwrap();

    let recorded = learner
        .step(
            &mut ledger,
            input(
                200,
                203,
                0,
                packet(5, 6, 5),
                [ActionId::new(10), ActionId::new(20)],
                2,
                PreviousTerminal::None,
            ),
        )
        .unwrap();
    assert_eq!(recorded.trace.reason, DecisionReason::Conflict);
    assert!(recorded.trace.decision.is_abstain());
    assert_eq!(
        recorded.trace.decision.audit().hypothesis_refs(),
        &[HypothesisId::new(1), HypothesisId::new(2)]
    );
}

#[test]
fn lowest_cost_declared_disagreement_action_is_exploration() {
    let mut ledger = InteractionLedger::new(ScopeToken::new(300)).unwrap();
    for episode in 301..=304 {
        add_observation(
            &mut ledger,
            episode,
            packet(episode, 1, episode),
            7,
            packet(episode, 1, episode),
            AuditReferences::empty(),
            ActMode::Exploration,
        );
    }
    let mut learner = GroundingLearner::new(ledger.run_scope()).unwrap();
    learner
        .declare_competing_prediction_with_cost(
            &mut ledger,
            HypothesisId::new(1),
            ActionId::new(20),
            packet(1, 2, 1),
            vec![EvidenceId::new(1)],
            LogicalCost {
                query_calls: 0,
                environment_actions: 0,
                structural_comparisons: 9,
                operation_steps: 0,
            },
        )
        .unwrap();
    learner
        .declare_competing_prediction_with_cost(
            &mut ledger,
            HypothesisId::new(2),
            ActionId::new(20),
            packet(1, 3, 1),
            vec![EvidenceId::new(2)],
            LogicalCost {
                query_calls: 0,
                environment_actions: 0,
                structural_comparisons: 9,
                operation_steps: 0,
            },
        )
        .unwrap();
    learner
        .declare_competing_prediction_with_cost(
            &mut ledger,
            HypothesisId::new(3),
            ActionId::new(10),
            packet(1, 4, 1),
            vec![EvidenceId::new(3)],
            LogicalCost {
                query_calls: 0,
                environment_actions: 0,
                structural_comparisons: 1,
                operation_steps: 0,
            },
        )
        .unwrap();
    learner
        .declare_competing_prediction_with_cost(
            &mut ledger,
            HypothesisId::new(4),
            ActionId::new(10),
            packet(1, 5, 1),
            vec![EvidenceId::new(4)],
            LogicalCost {
                query_calls: 0,
                environment_actions: 0,
                structural_comparisons: 1,
                operation_steps: 0,
            },
        )
        .unwrap();
    learner
        .declare_hypothesis_separating_action(ActionId::new(20))
        .unwrap();
    learner
        .declare_hypothesis_separating_action(ActionId::new(10))
        .unwrap();

    let recorded = learner
        .step(
            &mut ledger,
            input(
                300,
                305,
                0,
                packet(5, 6, 5),
                [ActionId::new(10), ActionId::new(20)],
                2,
                PreviousTerminal::None,
            ),
        )
        .unwrap();
    assert_eq!(
        recorded.trace.reason,
        DecisionReason::HypothesisDisagreement
    );
    assert_eq!(recorded.trace.mode(), Some(ActMode::Exploration));
    assert_eq!(recorded.trace.chosen_action(), Some(ActionId::new(10)));
    assert_eq!(
        recorded.trace.disagreement_actions[0].action,
        ActionId::new(10)
    );
}

#[test]
fn exhausted_action_budget_abstains_incomplete() {
    let mut meter = ResourceMeter::new();
    for _ in 0..36 {
        meter.begin_episode();
        meter.begin_decision();
        meter.record_environment_action().unwrap();
        meter.begin_decision();
        meter.record_environment_action().unwrap();
    }
    let mut world = sparse_structural_rule_learner::v2::evaluator_world::EvaluatorWorld::primary();
    let first = world.reset().unwrap();
    let mut ledger = InteractionLedger::with_resource_meter(first.run_scope(), meter).unwrap();
    let mut learner = GroundingLearner::new(first.run_scope()).unwrap();
    let recorded = learner.step(&mut ledger, first).unwrap();
    assert_eq!(recorded.trace.reason, DecisionReason::IncompleteBudget);
    assert!(recorded.trace.decision.is_abstain());
    assert_eq!(
        world
            .apply_recorded(&mut ledger, &recorded.receipt)
            .unwrap(),
        None
    );
    let derived = learner.observe(&mut ledger, None).unwrap();
    assert_eq!(derived.status, DerivedStatus::Incomplete);
}

#[test]
fn frozen_step_does_not_mutate_frozen_learner_state() {
    let mut ledger = InteractionLedger::new(ScopeToken::new(400)).unwrap();
    let mut learner = GroundingLearner::new(ledger.run_scope()).unwrap();
    let recorded = learner
        .step(
            &mut ledger,
            input(
                400,
                401,
                0,
                packet(1, 2, 1),
                [ActionId::new(10), ActionId::new(20)],
                2,
                PreviousTerminal::None,
            ),
        )
        .unwrap();
    complete_action(
        &mut learner,
        &mut ledger,
        &recorded,
        StepResult::new(packet(1, 2, 1), TerminalStatus::Success).unwrap(),
    );
    let frozen = learner.freeze(&mut ledger).unwrap();
    let before = frozen.clone();
    let final_decision = frozen
        .step(
            &mut ledger,
            input(
                400,
                402,
                0,
                packet(4, 5, 4),
                [ActionId::new(10), ActionId::new(20)],
                2,
                PreviousTerminal::None,
            ),
        )
        .unwrap();
    assert_eq!(frozen, before);
    assert_eq!(final_decision.trace.reason, DecisionReason::Fallback);
    assert!(ledger.is_frozen());
}

#[test]
fn partial_evidence_stays_unknown() {
    let before =
        sparse_structural_rule_learner::v2::partial_observation::PartialObservation::new([
            sparse_structural_rule_learner::v2::partial_observation::PartialAtom::Available(
                AtomId::new(7),
            ),
            sparse_structural_rule_learner::v2::partial_observation::PartialAtom::Unavailable,
            sparse_structural_rule_learner::v2::partial_observation::PartialAtom::Available(
                AtomId::new(7),
            ),
        ]);
    let after = sparse_structural_rule_learner::v2::partial_observation::PartialObservation::new([
        sparse_structural_rule_learner::v2::partial_observation::PartialAtom::Available(
            AtomId::new(7),
        ),
        sparse_structural_rule_learner::v2::partial_observation::PartialAtom::Unavailable,
        sparse_structural_rule_learner::v2::partial_observation::PartialAtom::Unavailable,
    ]);
    let assessment = assess_partial(before, after, Some(packet(7, 8, 7)));
    assert_eq!(assessment.unchanged_positions[1], None);
    assert_eq!(assessment.unchanged_positions[2], None);
    assert_eq!(assessment.prediction_matches, None);
    assert_eq!(assessment.status, DerivedStatus::Incomplete);
}

#[test]
fn conditional_result_becomes_stale_after_transition_change() {
    let mut ledger = challenged_ledger();
    let procedure = admitted_procedure(&mut ledger);
    let procedure_id = procedure.procedure_id();
    let mut learner = GroundingLearner::new(ledger.run_scope()).unwrap();
    learner.install_procedure(&mut ledger, procedure).unwrap();
    let recorded = learner
        .step(
            &mut ledger,
            input(
                900,
                905,
                0,
                packet(99, 20, 99),
                [ActionId::new(7), ActionId::new(9)],
                2,
                PreviousTerminal::None,
            ),
        )
        .unwrap();
    assert_eq!(
        learner.derived_results()[0].status,
        DerivedStatus::Conditional
    );
    learner
        .invalidate_dependency(procedure_id, ChangeReason::TransitionChanged)
        .unwrap();
    assert_eq!(
        learner.derived_results().last().unwrap().status,
        DerivedStatus::Stale
    );
    assert_eq!(recorded.receipt.sequence(), 5);
}

#[test]
fn procedure_scope_is_confined_to_its_source_run() {
    let mut source_ledger = challenged_ledger();
    let procedure = admitted_procedure(&mut source_ledger);
    let mut foreign_ledger = InteractionLedger::new(ScopeToken::new(9010)).unwrap();
    let mut foreign_learner = GroundingLearner::new(foreign_ledger.run_scope()).unwrap();
    assert!(matches!(
        foreign_learner.install_procedure(&mut foreign_ledger, procedure),
        Err(sparse_structural_rule_learner::v2::GroundingError::ScopeConfinement)
    ));
}

#[test]
fn changed_rule_revision_uses_fresh_scope_and_preserves_old_version() {
    let mut ledger = challenged_ledger();
    let procedure = admitted_procedure(&mut ledger);
    let old_id = procedure.procedure_id();
    let mut revision = ChangedRuleRevision::new(&procedure, ScopeToken::new(999)).unwrap();
    revision
        .record_contradiction(EvidenceId::new(99), Some(packet(1, 2, 1)), packet(1, 3, 1))
        .unwrap();
    let new = revision.revise(2).unwrap();
    assert_eq!(revision.old_procedure().procedure_id(), old_id);
    assert_eq!(revision.old_procedure().version(), 1);
    assert_ne!(
        revision.fresh_run_scope(),
        procedure.expansion().run_scope()
    );
    assert_eq!(procedure.version(), 1);
    assert!(!procedure.support_evidence_ids().is_empty());
    assert_eq!(new.version(), 2);
    assert_eq!(revision.fresh_run_scope(), ScopeToken::new(999));
    assert_eq!(
        revision.trace().contradiction_evidence_ids(),
        &[EvidenceId::new(99)]
    );
    assert_eq!(revision.trace().events().len(), 2);
}
