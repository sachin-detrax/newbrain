use super::snapshot::LearnerSnapshot;
use super::{Learner, LearnerConfig};
use crate::{
    budget::{BudgetConfig, BudgetKind, BudgetState},
    dataset::V1Dataset,
    types::{AbstentionReason, RecordId, Response, Sequence},
};

fn combine(second_output: u8) -> LearnerSnapshot {
    let d = V1Dataset::fixed();
    let mut first = Learner::new(LearnerConfig::v1());
    first
        .train_phase_a(d.r1_training().as_slice(), d.r1_development().as_slice())
        .unwrap();
    let mut train = d.r1_training().as_slice().to_vec();
    let mut dev = d.r1_development().as_slice().to_vec();
    for r in train.iter_mut().chain(dev.iter_mut()) {
        r.record_id = RecordId(r.record_id.0 + 50);
        if r.input != r.output {
            let [a, _, c] = r.output.positions();
            r.output = Sequence::new([a, second_output, c]);
        }
    }
    let mut second = Learner::new(LearnerConfig::v1());
    second.train_phase_a(&train, &dev).unwrap();
    let mut combined = first.freeze();
    combined.extend(second.freeze().procedures().to_vec());
    combined
}

#[test]
fn agreeing_candidates_preserve_all_references_and_conflicts_abstain() {
    let q = V1Dataset::fixed().r1_final_queries().next().unwrap();
    let agreed = combine(9).predict(&q);
    assert!(matches!(&agreed.response, Response::Prediction(_)));
    assert_eq!(agreed.response.procedure_ids().len(), 2);
    assert_eq!(agreed.response.evidence_refs().len(), 12);
    let conflict = combine(12).predict(&q);
    assert!(matches!(&conflict.response, Response::Abstention(_)));
    assert_eq!(conflict.response.reason(), Some(AbstentionReason::Conflict));
}

#[test]
fn output_agreement_comparison_requires_budget() {
    let q = V1Dataset::fixed().r1_final_queries().next().unwrap();
    let mut b = BudgetState::new(BudgetConfig::v1());
    b.charge_query(BudgetKind::OperationSteps, 44).unwrap();
    let result = combine(9).predict_with_budget(&q, &mut b);
    assert!(matches!(&result.response, Response::Abstention(_)));
    assert_eq!(
        result.response.reason(),
        Some(AbstentionReason::UnsupportedByBudget)
    );
}
