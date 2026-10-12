use sparse_structural_rule_learner::{
    admission::{admit, AdmissionResult, RejectionReason},
    budget::{BudgetConfig, BudgetState},
    dataset::V1Dataset,
    grammar::enumerate_g1,
    ledger::Ledger,
    types::{EvidenceRef, RecordId},
};

#[test]
fn missing_challenge_rejects_with_candidate_specific_incomplete_evidence() {
    // Given a generated R1 candidate and three valid development challenges.
    let data = V1Dataset::fixed();
    let mut budget = BudgetState::new(BudgetConfig::v1());
    let mut ledger = Ledger::new();
    let candidate = enumerate_g1(data.r1_training().as_slice(), &mut budget, &mut ledger)
        .unwrap()
        .remove(0);
    let candidate_id = candidate.id();
    let development = &data.r1_development().as_slice()[..3];
    let rejected_before = ledger.rejected_candidates().len();

    // When the candidate is checked for admission.
    let result = admit(
        candidate,
        data.r1_training().as_slice(),
        development,
        &mut budget,
        &mut ledger,
    )
    .expect("admission checks should complete");

    // Then missing coverage is distinguished from a contradiction in the audit.
    assert_eq!(
        result,
        AdmissionResult::Rejected(RejectionReason::IncompleteChallenges)
    );
    let all_rejections = ledger.rejected_candidates();
    let rejections = &all_rejections[rejected_before..];
    assert_eq!(rejections.len(), 1);
    assert_eq!(rejections[0].procedure_ids, vec![candidate_id]);
    assert_eq!(rejections[0].reason, "IncompleteChallenges");
    assert_eq!(
        rejections[0].evidence_refs,
        vec![
            EvidenceRef::record(RecordId(3)),
            EvidenceRef::record(RecordId(4)),
            EvidenceRef::record(RecordId(5))
        ]
    );
}

#[test]
fn non_support_contradiction_rejects_and_records_the_counterexample() {
    use sparse_structural_rule_learner::types::{EvidenceRef, RecordId};
    let d = V1Dataset::fixed();
    let mut b = BudgetState::new(BudgetConfig::v1());
    let mut l = Ledger::new();
    let c = enumerate_g1(d.r1_training().as_slice(), &mut b, &mut l)
        .unwrap()
        .remove(0);
    let mut training = d.r1_training().as_slice().to_vec();
    let mut counterexample = training[0].clone();
    counterexample.record_id = RecordId(90);
    counterexample.output = counterexample.input;
    training.push(counterexample);
    assert_eq!(
        admit(c, &training, d.r1_development().as_slice(), &mut b, &mut l).unwrap(),
        AdmissionResult::Rejected(RejectionReason::TrainingContradiction)
    );
    assert!(l
        .rejected_candidates()
        .last()
        .unwrap()
        .evidence_refs
        .contains(&EvidenceRef::record(RecordId(90))));
}

#[test]
fn grounded_candidate_has_support_and_challenge_evidence() {
    let d = V1Dataset::fixed();
    let mut b = BudgetState::new(BudgetConfig::v1());
    let mut l = Ledger::new();
    let c = enumerate_g1(d.r1_training().as_slice(), &mut b, &mut l)
        .unwrap()
        .remove(0);
    let result = admit(
        c,
        d.r1_training().as_slice(),
        d.r1_development().as_slice(),
        &mut b,
        &mut l,
    )
    .unwrap();
    match result {
        AdmissionResult::Admitted(p) => {
            assert_eq!(p.support_refs().len(), 2);
            assert_eq!(p.challenge_refs().len(), 4);
        }
        AdmissionResult::Rejected(reason) => panic!("{reason:?}"),
    }
}

#[test]
fn mismatched_development_outcome_rejects_candidate() {
    let d = V1Dataset::fixed();
    let mut b = BudgetState::new(BudgetConfig::v1());
    let mut l = Ledger::new();
    let c = enumerate_g1(d.r1_training().as_slice(), &mut b, &mut l)
        .unwrap()
        .remove(0);
    let mut dev = d.r1_development().as_slice().to_vec();
    dev[0].output = dev[0].input;
    assert_eq!(
        admit(c, d.r1_training().as_slice(), &dev, &mut b, &mut l).unwrap(),
        AdmissionResult::Rejected(RejectionReason::DevelopmentContradiction)
    );
}
