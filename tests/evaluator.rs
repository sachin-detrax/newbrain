use sparse_structural_rule_learner::evaluator::run_primary;

#[test]
fn primary_report_scores_learning_retention_and_exact_retrieval() {
    let run = run_primary().unwrap();
    assert!(run.gates.iter().all(|g| g.passed));
    assert_eq!(run.main_before.metrics.correct, 5);
    assert_eq!(run.main_after.metrics.correct, 5);
    assert_eq!(run.main_r2.metrics.correct, 2);
    assert_eq!(run.exact.metrics.correct, 0);
    assert_eq!(run.g0.metrics.correct, 0);
    assert_eq!(run.main_after.metrics.false_predictions, 0);
}

#[test]
fn false_predictions_and_missing_coverage_fail_acceptance() {
    use sparse_structural_rule_learner::evaluator::{performance_gates, Metrics};
    let good = Metrics {
        applicable: 5,
        predictions: 5,
        correct: 5,
        non_applicable: 3,
        false_predictions: 0,
        budget_failures: 0,
    };
    let r2 = Metrics {
        applicable: 2,
        predictions: 2,
        correct: 2,
        non_applicable: 2,
        false_predictions: 0,
        budget_failures: 0,
    };
    let bad = Metrics {
        false_predictions: 1,
        ..good.clone()
    };
    assert!(performance_gates(&bad, &good, &r2)
        .iter()
        .any(|g| !g.passed));
    let abstaining = Metrics {
        predictions: 0,
        correct: 0,
        ..good.clone()
    };
    assert!(performance_gates(&abstaining, &good, &r2)
        .iter()
        .any(|g| !g.passed));
}

#[test]
fn three_runs_have_identical_semantic_results_and_audits() {
    let a = run_primary().unwrap();
    let b = run_primary().unwrap();
    let c = run_primary().unwrap();
    assert_eq!(a, b);
    assert_eq!(b, c);
}
