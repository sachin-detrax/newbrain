use sparse_structural_rule_learner::v2::grammar_admission::{
    AdmissionOutcome, CandidateScope, CandidateStage, GenerationStatus, GrammarV2, OutputOp,
    ProcedureStatus, RejectionReason,
};
use sparse_structural_rule_learner::v2::interaction_ledger::{AuditScope, InteractionLedger};
use sparse_structural_rule_learner::v2::resource_scale::{LogicalResource, ResourceMeter};
use sparse_structural_rule_learner::v2::{
    ActMode, ActionId, AtomId, AuditReferences, Decision, ObservationPacket, PreviousTerminal,
    ScopeToken, StepInput, StepResult, TerminalStatus,
};

fn packet(a: u64, b: u64, c: u64) -> ObservationPacket {
    ObservationPacket::new([AtomId::new(a), AtomId::new(b), AtomId::new(c)])
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
    let action = ActionId::new(action);
    let input = StepInput::new(
        ledger.run_scope(),
        ScopeToken::new(episode),
        0,
        before,
        [ActionId::new(7), ActionId::new(9)],
        2,
        PreviousTerminal::None,
    )
    .unwrap();
    let prediction = matches!(mode, ActMode::Procedure).then_some(after);
    let decision = Decision::act(action, prediction, mode, audit).unwrap();
    let receipt = ledger.record_decision(input, decision).unwrap();
    let result = StepResult::new(after, TerminalStatus::Success).unwrap();
    ledger.record_observed_result(&receipt, result).unwrap();
}

fn support_ledger() -> InteractionLedger {
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
    ledger
}

fn challenged_ledger() -> InteractionLedger {
    let mut ledger = support_ledger();
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

fn e3_candidate(
    report: &sparse_structural_rule_learner::v2::grammar_admission::GenerationReport,
) -> sparse_structural_rule_learner::v2::grammar_admission::Candidate {
    report
        .candidates()
        .iter()
        .find(|candidate| candidate.stage() == CandidateStage::E3)
        .expect("the contrasting opaque interaction should create E3")
        .clone()
}

fn add_grounding(
    ledger: &mut InteractionLedger,
    hypothesis: sparse_structural_rule_learner::v2::HypothesisId,
) {
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
}

#[test]
fn e0_e1_e2_e3_are_opaque_and_e2_transfers_to_a_held_out_id() {
    let ledger = challenged_ledger();
    let mut grammar = GrammarV2::new(ledger.run_scope()).unwrap();
    let report = grammar.generate(&ledger);

    assert!(matches!(report.status(), GenerationStatus::Complete));
    assert_eq!(report.candidates_at(CandidateStage::E0).len(), 3);
    assert_eq!(report.candidates_at(CandidateStage::E1).len(), 1);
    assert_eq!(report.candidates_at(CandidateStage::E2).len(), 1);
    assert_eq!(report.candidates_at(CandidateStage::E3).len(), 1);

    let e2 = report.candidates_at(CandidateStage::E2)[0];
    assert_eq!(e2.input_pattern().variable_positions(), vec![0, 2]);
    assert_eq!(e2.unchanged_positions(), &[0, 2]);
    assert_eq!(e2.changed_positions(), &[1]);
    assert_eq!(e2.output_operations()[0], OutputOp::Copy(0));
    assert_eq!(
        e2.output_operations()[1],
        OutputOp::Observed(AtomId::new(21))
    );
    assert_eq!(e2.output_operations()[2], OutputOp::Copy(2));
    assert_eq!(
        e2.predict(packet(77, 20, 77), ActionId::new(7)),
        Some(packet(77, 21, 77))
    );

    let e3 = e3_candidate(&report);
    assert_eq!(e3.challenge_evidence_ids().len(), 1);
    assert_eq!(e3.predict(packet(10, 21, 10), ActionId::new(7)), None);
    assert!(!e3.canonical_signature().contains("context"));
    assert!(!e3.canonical_signature().contains("marker"));
}

#[test]
fn e0_is_exact_replay_only_and_single_support_is_rejected_for_structure() {
    let mut ledger = InteractionLedger::new(ScopeToken::new(910)).unwrap();
    add_observation(
        &mut ledger,
        911,
        packet(1, 2, 1),
        7,
        packet(1, 3, 1),
        AuditReferences::empty(),
        ActMode::Exploration,
    );
    let mut grammar = GrammarV2::new(ledger.run_scope()).unwrap();
    let report = grammar.generate(&ledger);
    assert_eq!(report.candidates_at(CandidateStage::E0).len(), 1);
    assert!(report.candidates_at(CandidateStage::E2).is_empty());
    assert!(report.rejections().iter().any(|rejection| rejection
        .reasons()
        .contains(&RejectionReason::InsufficientSupport)));
}

#[test]
fn unstable_effects_do_not_become_aligned_candidates() {
    let mut ledger = InteractionLedger::new(ScopeToken::new(920)).unwrap();
    add_observation(
        &mut ledger,
        921,
        packet(1, 2, 1),
        7,
        packet(1, 3, 1),
        AuditReferences::empty(),
        ActMode::Exploration,
    );
    add_observation(
        &mut ledger,
        922,
        packet(4, 2, 4),
        7,
        packet(5, 2, 4),
        AuditReferences::empty(),
        ActMode::Exploration,
    );
    let mut grammar = GrammarV2::new(ledger.run_scope()).unwrap();
    let report = grammar.generate(&ledger);
    assert!(report.candidates_at(CandidateStage::E1).is_empty());
    assert!(report.rejections().iter().any(|rejection| rejection
        .reasons()
        .contains(&RejectionReason::UnstableEffect)));
}

#[test]
fn signatures_and_candidate_order_are_deterministic_under_record_order() {
    let first = challenged_ledger();
    let second = challenged_ledger();
    let mut first_grammar = GrammarV2::new(first.run_scope()).unwrap();
    let mut second_grammar = GrammarV2::new(second.run_scope()).unwrap();
    let first_report = first_grammar.generate(&first);
    let second_report = second_grammar.generate(&second);
    let first_signatures: Vec<_> = first_report
        .candidates()
        .iter()
        .map(|candidate| {
            (
                candidate.stage(),
                candidate.canonical_signature().to_owned(),
            )
        })
        .collect();
    let second_signatures: Vec<_> = second_report
        .candidates()
        .iter()
        .map(|candidate| {
            (
                candidate.stage(),
                candidate.canonical_signature().to_owned(),
            )
        })
        .collect();
    assert_eq!(first_signatures, second_signatures);
    assert!(first_report
        .candidates()
        .windows(2)
        .all(|pair| pair[0].canonical_order() <= pair[1].canonical_order()));
}

#[test]
fn missing_challenge_and_missing_grounding_remain_distinct_rejections() {
    let ledger = support_ledger();
    let mut grammar = GrammarV2::new(ledger.run_scope()).unwrap();
    let report = grammar.generate(&ledger);
    assert!(report.candidates_at(CandidateStage::E3).is_empty());
    assert!(report.rejections().iter().any(|rejection| rejection
        .reasons()
        .contains(&RejectionReason::MissingChallenge)));

    let e2 = (*report
        .candidates_at(CandidateStage::E2)
        .first()
        .expect("support still creates E2"))
    .clone();
    match grammar.admit(e2, &ledger) {
        AdmissionOutcome::Rejected(rejected) => {
            assert!(rejected
                .reasons()
                .contains(&RejectionReason::MissingChallenge));
            assert!(rejected
                .reasons()
                .contains(&RejectionReason::MissingGroundingEvidence));
        }
        other => panic!("expected explicit rejection, got {other:?}"),
    }
}

#[test]
fn admission_rejects_invented_output_and_scope_leakage() {
    let ledger = challenged_ledger();
    let mut grammar = GrammarV2::new(ledger.run_scope()).unwrap();
    let report = grammar.generate(&ledger);
    let e3 = e3_candidate(&report);

    let invented = e3.with_output_operations([
        OutputOp::Copy(0),
        OutputOp::Observed(AtomId::new(999)),
        OutputOp::Copy(2),
    ]);
    match grammar.admit(invented, &ledger) {
        AdmissionOutcome::Rejected(rejected) => {
            assert!(rejected
                .reasons()
                .contains(&RejectionReason::OutputInconsistency));
        }
        other => panic!("invented output must be rejected, got {other:?}"),
    }

    let leaked = e3.with_scope(
        ScopeToken::new(9999),
        CandidateScope::Run(ScopeToken::new(9999)),
    );
    match grammar.admit(leaked, &ledger) {
        AdmissionOutcome::Rejected(rejected) => {
            assert!(rejected
                .reasons()
                .contains(&RejectionReason::ScopeConfinement));
        }
        other => panic!("scope leakage must be rejected, got {other:?}"),
    }
}

#[test]
fn mixed_run_records_are_rejected_without_cross_run_generalization() {
    let ledger = challenged_ledger();
    let mut foreign = InteractionLedger::new(ScopeToken::new(990)).unwrap();
    add_observation(
        &mut foreign,
        991,
        packet(10, 20, 10),
        7,
        packet(10, 21, 10),
        AuditReferences::empty(),
        ActMode::Exploration,
    );
    let mut records = ledger.interactions().to_vec();
    records.extend(foreign.interactions().iter().cloned());
    let mut grammar = GrammarV2::new(ledger.run_scope()).unwrap();
    let report = grammar.generate_interactions(&records);
    assert!(report.rejections().iter().any(|rejection| rejection
        .reasons()
        .contains(&RejectionReason::ScopeConfinement)));
    assert!(report
        .candidates()
        .iter()
        .all(|candidate| candidate.run_scope() == ledger.run_scope()));
}

#[test]
fn candidate_budget_exhaustion_is_explicit_and_does_not_guess() {
    let ledger = challenged_ledger();
    let mut meter = ResourceMeter::new();
    for _ in 0..8 {
        meter.begin_episode();
        meter.begin_decision();
        for _ in 0..64 {
            meter.record_candidate_generation().unwrap();
        }
    }
    assert_eq!(
        meter.counters().candidate_generation_attempts(),
        meter
            .limits()
            .logical(LogicalResource::CandidateGenerationAttempts)
            .per_run()
    );
    let mut grammar = GrammarV2::with_resource_meter(ledger.run_scope(), meter).unwrap();
    let report = grammar.generate(&ledger);
    assert!(matches!(
        report.status(),
        GenerationStatus::Incomplete {
            resource: LogicalResource::CandidateGenerationAttempts,
            ..
        }
    ));
    assert!(report.candidates().is_empty());
    assert!(report.rejections().iter().any(|rejection| rejection
        .reasons()
        .contains(&RejectionReason::BudgetExhausted)));
}

#[test]
fn admitted_e3_promotes_with_full_provenance_and_later_contradiction_is_a_new_record() {
    let mut ledger = challenged_ledger();
    let mut grammar = GrammarV2::new(ledger.run_scope()).unwrap();
    let report = grammar.generate(&ledger);
    let candidate = e3_candidate(&report);
    let original_signature = candidate.canonical_signature().to_owned();
    let original_support = candidate.support_evidence_ids().to_vec();
    let original_challenge = candidate.challenge_evidence_ids().to_vec();
    add_grounding(&mut ledger, candidate.hypothesis_id());

    let admitted = match grammar.admit(candidate, &ledger) {
        AdmissionOutcome::Admitted(admitted) => admitted,
        other => panic!("expected all frozen gates to pass, got {other:?}"),
    };
    let procedure = grammar.promote(&mut ledger, admitted).unwrap();
    assert_eq!(procedure.status(), ProcedureStatus::Active);
    assert_eq!(procedure.expansion().stage(), CandidateStage::E4);
    assert_eq!(procedure.support_evidence_ids(), original_support);
    assert_eq!(procedure.challenge_evidence_ids(), original_challenge);
    assert_eq!(
        procedure.provenance().source_signature(),
        original_signature
    );
    assert_eq!(
        procedure.provenance().expansion().canonical_signature(),
        procedure.expansion().canonical_signature()
    );

    add_observation(
        &mut ledger,
        905,
        packet(100, 20, 100),
        7,
        packet(100, 22, 100),
        AuditReferences::new(vec![procedure.procedure_id()], Vec::new(), Vec::new()).unwrap(),
        ActMode::Procedure,
    );
    match grammar.revalidate(&procedure, &ledger) {
        sparse_structural_rule_learner::v2::grammar_admission::RevisionOutcome::Suspended(
            revision,
        ) => {
            assert_eq!(revision.status(), ProcedureStatus::Suspended);
            assert_eq!(revision.version(), 2);
            assert_eq!(revision.contradiction_evidence_ids().len(), 1);
            assert_eq!(grammar.procedures()[0].status(), ProcedureStatus::Active);
            assert_eq!(grammar.revisions().len(), 1);
        }
        other => panic!("expected a new suspension record, got {other:?}"),
    }
}
