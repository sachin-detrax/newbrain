use super::{fixture, removed_development_challenge, Sequence};

#[test]
fn under_testing_score_fails_when_rejection_is_a_contradiction() {
    // Given corrupted development evidence, not merely incomplete coverage.
    let mut fixture = fixture();
    fixture.development[0].output = fixture.development[0].input;

    // When the reduced-challenge extension evaluates the real learners.
    let result = removed_development_challenge(&fixture).expect("extension should execute");

    // Then a contradiction rejection cannot stand in for incomplete evidence.
    assert!(!result.passed, "{}", result.detail);
}

#[test]
fn under_testing_score_fails_when_a1_prediction_is_incorrect() {
    // Given an expected output different from A1's learned transformation.
    let mut fixture = fixture();
    fixture.expected = Sequence::new([3, 7, 3]);

    // When the reduced-challenge extension evaluates the real learners.
    let result = removed_development_challenge(&fixture).expect("extension should execute");

    // Then gated rejection alone cannot pass the paired comparison.
    assert!(!result.passed, "{}", result.detail);
}
