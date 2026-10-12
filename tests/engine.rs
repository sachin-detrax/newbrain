use sparse_structural_rule_learner::{
    dataset::V1Dataset,
    engine::{Learner, LearnerConfig},
    types::{Response, Sequence},
};

#[test]
fn held_out_prediction_and_retention_use_the_same_frozen_interface() {
    let d = V1Dataset::fixed();
    let mut learner = Learner::new(LearnerConfig::v1());
    learner
        .train_phase_a(d.r1_training().as_slice(), d.r1_development().as_slice())
        .unwrap();
    let old = learner.freeze();
    let q = d.r1_final_queries().next().unwrap();
    let expected = Sequence::new([3, 9, 3]);
    let old_execution = old.predict(&q);
    assert!(matches!(&old_execution.response, Response::Prediction(_)));
    assert_eq!(old_execution.response.output(), Some(expected));
    learner
        .train_phase_b(d.r2_training().as_slice(), d.r2_development().as_slice())
        .unwrap();
    let new = learner.freeze();
    assert_eq!(old.predict(&q).response, new.predict(&q).response);
    assert_eq!(old.procedures().len(), 1);
    assert_eq!(new.procedures().len(), 2);
    for q in d.r1_final_queries().skip(5) {
        let execution = new.predict(&q);
        assert!(matches!(&execution.response, Response::Abstention(_)));
    }
}

#[test]
fn both_rule_outputs_scope_and_budget_are_checked() {
    use sparse_structural_rule_learner::{
        budget::{BudgetConfig, BudgetKind, BudgetState},
        types::AbstentionReason,
    };
    let d = V1Dataset::fixed();
    let mut learner = Learner::new(LearnerConfig::v1());
    let report = learner
        .train_phase_a(d.r1_training().as_slice(), d.r1_development().as_slice())
        .unwrap();
    assert_eq!(report.rejected, 3);
    learner
        .train_phase_b(d.r2_training().as_slice(), d.r2_development().as_slice())
        .unwrap();
    let frozen = learner.freeze();
    for q in d.r1_final_queries().take(5) {
        let x = q.input.positions()[0];
        let execution = frozen.predict(&q);
        assert!(matches!(&execution.response, Response::Prediction(_)));
        assert_eq!(execution.response.output(), Some(Sequence::new([x, 9, x])));
    }
    for q in d.r2_final_queries().take(2) {
        let x = q.input.positions()[0];
        let execution = frozen.predict(&q);
        assert!(matches!(&execution.response, Response::Prediction(_)));
        assert_eq!(execution.response.output(), Some(Sequence::new([x, 11, x])));
    }
    let mut q = d.r1_final_queries().next().unwrap();
    q.scope = 1;
    let execution = frozen.predict(&q);
    assert!(matches!(&execution.response, Response::Abstention(_)));
    assert_eq!(
        execution.response.reason(),
        Some(AbstentionReason::NoApplicableProcedure)
    );
    q.scope = 0;
    let mut budget = BudgetState::new(BudgetConfig::v1());
    budget.charge_query(BudgetKind::OperationSteps, 64).unwrap();
    let execution = frozen.predict_with_budget(&q, &mut budget);
    assert!(matches!(&execution.response, Response::Abstention(_)));
    assert_eq!(
        execution.response.reason(),
        Some(AbstentionReason::UnsupportedByBudget)
    );
}
