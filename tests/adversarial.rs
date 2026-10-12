use sparse_structural_rule_learner::adversarial::run_extensions;
use sparse_structural_rule_learner::dataset::V1Dataset;
use sparse_structural_rule_learner::engine::{Learner, LearnerConfig};
use sparse_structural_rule_learner::types::{AbstentionReason, Query, QueryId, Response, Sequence};

#[test]
fn under_testing_extension_passes_when_missing_evidence_is_detected() {
    // Given the fixed extension fixtures, including one removed challenge.
    // When the extension evaluator runs.
    let results = run_extensions().expect("extensions should execute");

    // Then conservative rejection is scored as successful detection.
    let result = results
        .iter()
        .find(|result| result.name == "removed-development-challenge")
        .expect("under-testing extension should be present");
    assert!(result.passed, "{}", result.detail);
}

#[test]
fn gated_learner_abstains_when_a_development_challenge_is_missing() {
    // Given R1 support and only three of its four development challenges.
    let data = V1Dataset::fixed();
    let mut learner = Learner::new(LearnerConfig::v1());
    let report = learner
        .train_phase_a(
            data.r1_training().as_slice(),
            &data.r1_development().as_slice()[..3],
        )
        .expect("rejection is a completed training outcome");
    let query = Query {
        input: Sequence::new([3, 8, 3]),
        action: 0,
        scope: 0,
        query_id: QueryId(100),
    };

    // When queried on an otherwise applicable held-out input.
    let execution = learner.freeze().predict(&query);

    // Then the candidate is not admitted and the query abstains, without exhaustion.
    assert_eq!(report.admitted, 0);
    assert!(execution.budget_error.is_none());
    assert!(matches!(&execution.response, Response::Abstention(_)));
    assert_eq!(
        execution.response.reason(),
        Some(AbstentionReason::NoApplicableProcedure)
    );
}

#[test]
fn ungated_learner_predicts_when_a_development_challenge_is_missing() {
    // Given the same reduced challenges with development gating disabled.
    let data = V1Dataset::fixed();
    let mut learner = Learner::new(LearnerConfig::without_development_gate());
    learner
        .train_phase_a(
            data.r1_training().as_slice(),
            &data.r1_development().as_slice()[..3],
        )
        .expect("A1 should train");
    let query = Query {
        input: Sequence::new([3, 8, 3]),
        action: 0,
        scope: 0,
        query_id: QueryId(100),
    };

    // When queried on the same held-out input.
    let execution = learner.freeze().predict(&query);

    // Then A1 predicts the exact expected output despite incomplete challenges.
    assert!(execution.budget_error.is_none());
    assert!(matches!(&execution.response, Response::Prediction(_)));
    assert_eq!(execution.response.output(), Some(Sequence::new([3, 9, 3])));
}

#[test]
fn task9_reports_ten_independent_extensions() {
    let results = run_extensions().expect("Task9 extensions should execute");
    assert_eq!(results.len(), 10);
    assert!(results.iter().all(|result| !result.name.is_empty()));
    assert!(results.iter().all(|result| !result.property.is_empty()));
    assert!(results.iter().all(|result| !result.detail.is_empty()));

    let names: Vec<_> = results.iter().map(|result| result.name).collect();
    assert_eq!(
        names,
        vec![
            "symbol-permutation",
            "reordered-training",
            "irrelevant-record",
            "duplicate-source-id",
            "separate-scope-contradiction",
            "removed-development-challenge",
            "corrupted-development-output",
            "query-exhaustion",
            "larger-symbol-cardinality",
            "multiple-unrelated-rules",
        ]
    );
}

#[test]
fn task9_does_not_mutate_the_fixed_primary_fixture() {
    let before = V1Dataset::fixed().fingerprint();
    let _ = run_extensions().expect("Task9 extensions should execute");
    assert_eq!(before, V1Dataset::fixed().fingerprint());
}
