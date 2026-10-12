use sparse_structural_rule_learner::budget::{BudgetConfig, BudgetKind, BudgetScope, BudgetState};
use sparse_structural_rule_learner::ledger::{CostVector, Ledger};
use sparse_structural_rule_learner::types::{AbstentionReason, EvidenceRef, ProcedureId, Response};

#[test]
fn v1_budget_limits_match_the_specification() {
    let config = BudgetConfig::v1();
    assert_eq!(config.limit(BudgetKind::CandidatePairs), 16);
    assert_eq!(config.limit(BudgetKind::GeneratedCandidates), 32);
    assert_eq!(config.limit(BudgetKind::SupportComparisons), 128);
    assert_eq!(config.limit(BudgetKind::DevelopmentChecks), 32);
    assert_eq!(config.limit(BudgetKind::OperationSteps), 512);
    assert_eq!(config.limit(BudgetKind::ActiveTransitionRecords), 32);
    assert_eq!(config.limit(BudgetKind::ActiveCandidateRecords), 64);
    assert_eq!(config.limit(BudgetKind::RetrievedCandidates), 16);
    assert_eq!(config.limit(BudgetKind::CandidateChecks), 16);
    assert_eq!(config.limit(BudgetKind::ActiveRecords), 16);
    assert_eq!(config.limit(BudgetKind::TotalCandidateRecords), 128);
    assert_eq!(config.limit(BudgetKind::TotalEventRecords), 128);
    assert_eq!(config.limit(BudgetKind::FullStateRebuilds), 0);
    assert_eq!(config.limit(BudgetKind::HiddenStateAccesses), 0);
    assert_eq!(
        config.limit_for(BudgetScope::Query, BudgetKind::OperationSteps),
        64
    );
}

#[test]
fn budget_rejects_the_charge_that_would_cross_the_limit_without_mutating() {
    let mut budget = BudgetState::new(BudgetConfig::v1());
    budget
        .charge(BudgetKind::CandidatePairs, 16)
        .expect("the exact limit should be allowed");
    let error = budget
        .charge(BudgetKind::CandidatePairs, 1)
        .expect_err("the next charge must exceed the limit");

    assert_eq!(error.kind(), BudgetKind::CandidatePairs);
    assert_eq!(error.used(), 16);
    assert_eq!(budget.used(BudgetKind::CandidatePairs), 16);
}

#[test]
fn active_budget_tracks_peak_without_turning_release_into_work() {
    let mut budget = BudgetState::new(BudgetConfig::v1());
    budget
        .set_active(BudgetKind::ActiveCandidateRecords, 5)
        .unwrap();
    budget
        .set_active(BudgetKind::ActiveCandidateRecords, 2)
        .unwrap();

    assert_eq!(budget.peak(BudgetKind::ActiveCandidateRecords), 5);
    assert_eq!(budget.used(BudgetKind::ActiveCandidateRecords), 0);
}

#[test]
fn query_operation_budget_is_separate_from_training_budget() {
    let mut budget = BudgetState::new(BudgetConfig::v1());
    budget
        .charge(BudgetKind::OperationSteps, 512)
        .expect("training update limit should be 512");
    budget
        .charge_query(BudgetKind::OperationSteps, 64)
        .expect("query limit should be 64");
    let error = budget
        .charge_query(BudgetKind::OperationSteps, 1)
        .expect_err("the 65th query operation must be rejected");

    assert_eq!(error.scope(), BudgetScope::Query);
    assert_eq!(
        budget.used_in(BudgetScope::Query, BudgetKind::OperationSteps),
        64
    );
}

#[test]
fn ledger_records_rejections_and_serializes_deterministically() {
    let mut first = Ledger::new();
    first
        .record_rejection(
            ProcedureId(1),
            "development contradiction",
            CostVector::default(),
        )
        .unwrap();
    first
        .record_rejection(ProcedureId(2), "budget", CostVector::default())
        .unwrap();

    let mut second = Ledger::new();
    second
        .record_rejection(
            ProcedureId(1),
            "development contradiction",
            CostVector::default(),
        )
        .unwrap();
    second
        .record_rejection(ProcedureId(2), "budget", CostVector::default())
        .unwrap();

    assert_eq!(first.rejected_candidates().len(), 2);
    assert_eq!(first.stable_text(), second.stable_text());
}

#[test]
fn ledger_serializes_evidence_cost_response_reason_and_budget_context() {
    let mut ledger = Ledger::new();
    let evidence = EvidenceRef::record(sparse_structural_rule_learner::types::RecordId(4));
    let cost = CostVector {
        operation_steps: 7,
        ..CostVector::default()
    };
    let response = Response::abstention(
        AbstentionReason::UnsupportedByBudget,
        vec![ProcedureId(3)],
        vec![evidence],
    );
    ledger.record_response(&response, cost.clone()).unwrap();
    let mut budget = BudgetState::new(BudgetConfig::v1());
    let error = budget
        .charge_query(BudgetKind::OperationSteps, 65)
        .expect_err("the query budget must fail");
    ledger
        .record_budget_failure(&error, Some(ProcedureId(3)), vec![evidence], cost)
        .unwrap();

    let text = ledger.stable_text();
    assert!(text.contains("abstention:UnsupportedByBudget"));
    assert!(text.contains("operation_steps: 7"));
    assert!(text.contains("RecordId(4)"));
    assert!(text.contains("scope=Query"));
}

#[test]
fn ledger_escapes_reason_delimiters_and_newlines() {
    let mut ledger = Ledger::new();
    ledger
        .record_rejection(ProcedureId(1), "line|one\nline-two", CostVector::default())
        .unwrap();
    let text = ledger.stable_text();
    assert!(text.contains("line\\|one\\nline-two"));
    assert_eq!(text.lines().count(), 1);
}

#[test]
fn ledger_rejects_record_129_without_changing_retained_records() {
    let mut ledger = Ledger::new();
    for id in 0..128 {
        ledger
            .record_rejection(ProcedureId(id), "rejected", CostVector::default())
            .unwrap();
    }
    let before = ledger.clone();
    let error = ledger
        .record_rejection(ProcedureId(128), "overflow", CostVector::default())
        .unwrap_err();
    assert_eq!(error.kind(), BudgetKind::TotalEventRecords);
    assert_eq!(error.used(), 128);
    assert_eq!(ledger, before);
}

#[test]
fn response_ledger_preserves_every_procedure_reference() {
    let mut ledger = Ledger::new();
    let response = Response::abstention(
        AbstentionReason::Conflict,
        vec![ProcedureId(1), ProcedureId(7)],
        vec![],
    );
    ledger
        .record_response(&response, CostVector::default())
        .unwrap();
    assert_eq!(
        ledger.events()[0].procedure_ids,
        vec![ProcedureId(1), ProcedureId(7)]
    );
    assert!(ledger.stable_text().contains("ProcedureId(7)"));
}

#[test]
fn failed_active_query_observation_reports_previous_peak_without_mutation() {
    let mut budget = BudgetState::new(BudgetConfig::v1());
    budget
        .set_active_in(BudgetScope::Query, BudgetKind::ActiveRecords, 9)
        .unwrap();
    let before = budget.clone();
    let error = budget
        .set_active_in(BudgetScope::Query, BudgetKind::ActiveRecords, 17)
        .unwrap_err();
    assert_eq!(error.used(), 9);
    assert_eq!(budget, before);
}
