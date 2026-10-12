use sparse_structural_rule_learner::experiment_b::protocol::{
    Budget, Metric, ResourceKind, SplitMix64, DEVELOPMENT_EPISODES, DEVELOPMENT_PARTITION_TAG,
    FINAL_COMPOSITION_EPISODES, FINAL_DEAD_END_EPISODES, FINAL_EPISODES, FINAL_H16_EPISODES,
    FINAL_H4_EPISODES, FINAL_H8_EPISODES, FINAL_PARTITION_TAG, FINAL_REPLAN_EPISODES,
    FINAL_STOCHASTIC_EPISODES, GENERATOR_VERSION, GRAMMAR_VERSION,
    MAX_ACTIONS_ISSUED_PER_EPISODE, MAX_ACTIONS_ISSUED_PER_RUN, MAX_ARTIFACT_BYTES_PER_EPISODE,
    MAX_ARTIFACT_BYTES_PER_RUN, MAX_EVIDENCE_RECORDS_PER_EPISODE, MAX_EVIDENCE_RECORDS_PER_RUN,
    MAX_MPC_ROLLOUTS_PER_EPISODE, MAX_MPC_ROLLOUTS_PER_RUN, MAX_PLAN_NODES_PER_EPISODE,
    MAX_PLAN_NODES_PER_RUN, MAX_RECEIVED_OBSERVATIONS_PER_EPISODE,
    MAX_RECEIVED_OBSERVATIONS_PER_RUN, MAX_RESIDENT_MEMORY_BYTES,
    MAX_RETAINED_HYPOTHESES_PER_EPISODE, MAX_RETAINED_HYPOTHESES_PER_RUN,
    MAX_TRANSITION_COMPARISONS_PER_EPISODE, MAX_TRANSITION_COMPARISONS_PER_RUN,
    MAX_TRANSITION_HYPOTHESES_PER_EPISODE, MAX_TRANSITION_HYPOTHESES_PER_RUN, MAX_WALL_CLOCK_NS,
    ORDINAL_MULTIPLIER, PERMUTATION_BASELINE_SEED, PERMUTATION_RERUN_SEED,
    PERMUTATION_SCOPE_SEED, PERMUTATION_STATE_XOR, PERMUTATION_WORLD_SEED, PRIMARY_BASELINE_SEED,
    PRIMARY_RERUN_SEED, PRIMARY_SCOPE_SEED, PRIMARY_WORLD_SEED, PROTOCOL_VERSION,
    SPECIFICATION_VERSION, TOTAL_EPISODES, TRAINING_EPISODES, TRAINING_PARTITION_TAG,
};

#[test]
fn frozen_version_strings_match_registration() {
    assert_eq!(
        SPECIFICATION_VERSION,
        "architecture-experiment-b-transition-model-and-goal-directed-planning/1.0.0"
    );
    assert_eq!(
        PROTOCOL_VERSION,
        "transition-model-and-goal-directed-planning-experiment-b-protocol-v1.0"
    );
    assert_eq!(GENERATOR_VERSION, "goal-directed-planning-generator-b-v1");
    assert_eq!(GRAMMAR_VERSION, "goal-directed-planning-grammar-b-v1");
}

#[test]
fn frozen_partition_counts_match_protocol() {
    assert_eq!(TRAINING_EPISODES, 96);
    assert_eq!(DEVELOPMENT_EPISODES, 48);
    assert_eq!(FINAL_EPISODES, 96);
    assert_eq!(TOTAL_EPISODES, 240);
    assert_eq!(
        TRAINING_EPISODES + DEVELOPMENT_EPISODES + FINAL_EPISODES,
        TOTAL_EPISODES
    );
    assert_eq!(FINAL_H4_EPISODES, 16);
    assert_eq!(FINAL_H8_EPISODES, 16);
    assert_eq!(FINAL_H16_EPISODES, 16);
    assert_eq!(FINAL_COMPOSITION_EPISODES, 12);
    assert_eq!(FINAL_STOCHASTIC_EPISODES, 12);
    assert_eq!(FINAL_DEAD_END_EPISODES, 12);
    assert_eq!(FINAL_REPLAN_EPISODES, 12);
    assert_eq!(
        FINAL_H4_EPISODES
            + FINAL_H8_EPISODES
            + FINAL_H16_EPISODES
            + FINAL_COMPOSITION_EPISODES
            + FINAL_STOCHASTIC_EPISODES
            + FINAL_DEAD_END_EPISODES
            + FINAL_REPLAN_EPISODES,
        FINAL_EPISODES
    );
}

#[test]
fn frozen_seeds_match_protocol() {
    assert_eq!(PRIMARY_WORLD_SEED, 2_026_101_203);
    assert_eq!(PRIMARY_SCOPE_SEED, 2_026_101_213);
    assert_eq!(PRIMARY_BASELINE_SEED, 2_026_101_223);
    assert_eq!(PRIMARY_RERUN_SEED, 2_026_101_233);
    assert_eq!(PERMUTATION_WORLD_SEED, 2_026_101_204);
    assert_eq!(PERMUTATION_SCOPE_SEED, 2_026_101_214);
    assert_eq!(PERMUTATION_BASELINE_SEED, 2_026_101_224);
    assert_eq!(PERMUTATION_RERUN_SEED, 2_026_101_234);
}

#[test]
fn frozen_partition_tags_match_protocol() {
    assert_eq!(TRAINING_PARTITION_TAG, 0x5452_4149_4e00_0002);
    assert_eq!(DEVELOPMENT_PARTITION_TAG, 0x4445_5600_0000_0002);
    assert_eq!(FINAL_PARTITION_TAG, 0x4649_4e00_0000_0002);
    assert_eq!(ORDINAL_MULTIPLIER, 0xD1B5_4A32_D192_ED03);
    assert_eq!(PERMUTATION_STATE_XOR, 0x94D0_49BB_1331_11EB);
}

#[test]
fn frozen_resource_ceilings_match_protocol() {
    assert_eq!(
        (
            MAX_RECEIVED_OBSERVATIONS_PER_EPISODE,
            MAX_RECEIVED_OBSERVATIONS_PER_RUN
        ),
        (64, 15_360)
    );
    assert_eq!(
        (MAX_ACTIONS_ISSUED_PER_EPISODE, MAX_ACTIONS_ISSUED_PER_RUN),
        (16, 3_840)
    );
    assert_eq!(
        (MAX_PLAN_NODES_PER_EPISODE, MAX_PLAN_NODES_PER_RUN),
        (1_024, 245_760)
    );
    assert_eq!(
        (
            MAX_TRANSITION_HYPOTHESES_PER_EPISODE,
            MAX_TRANSITION_HYPOTHESES_PER_RUN
        ),
        (512, 122_880)
    );
    assert_eq!(
        (
            MAX_TRANSITION_COMPARISONS_PER_EPISODE,
            MAX_TRANSITION_COMPARISONS_PER_RUN
        ),
        (2_048, 491_520)
    );
    assert_eq!(
        (
            MAX_RETAINED_HYPOTHESES_PER_EPISODE,
            MAX_RETAINED_HYPOTHESES_PER_RUN
        ),
        (64, 512)
    );
    assert_eq!(
        (MAX_MPC_ROLLOUTS_PER_EPISODE, MAX_MPC_ROLLOUTS_PER_RUN),
        (256, 61_440)
    );
    assert_eq!(
        (
            MAX_EVIDENCE_RECORDS_PER_EPISODE,
            MAX_EVIDENCE_RECORDS_PER_RUN
        ),
        (256, 61_440)
    );
    assert_eq!(
        (MAX_ARTIFACT_BYTES_PER_EPISODE, MAX_ARTIFACT_BYTES_PER_RUN),
        (2 * 1024 * 1024, 512 * 1024 * 1024)
    );
    assert_eq!(MAX_WALL_CLOCK_NS, 120_000_000_000);
    assert_eq!(MAX_RESIDENT_MEMORY_BYTES, 1_073_741_824);
    assert_eq!(ResourceKind::all().len(), 9);
}

#[test]
fn splitmix64_is_deterministic_per_seed() {
    let mut first = SplitMix64::new(0);
    let mut second = SplitMix64::new(0);
    let first_draws = [first.next_u64(), first.next_u64(), first.next_u64()];
    let second_draws = [second.next_u64(), second.next_u64(), second.next_u64()];
    assert_eq!(first_draws, second_draws);
}

#[test]
fn splitmix64_diverges_across_seeds() {
    let mut primary = SplitMix64::new(PRIMARY_WORLD_SEED);
    let mut permutation = SplitMix64::new(PERMUTATION_WORLD_SEED);
    let primary_draws = [primary.next_u64(), primary.next_u64(), primary.next_u64()];
    let permutation_draws = [
        permutation.next_u64(),
        permutation.next_u64(),
        permutation.next_u64(),
    ];
    assert_ne!(primary_draws, permutation_draws);
}

#[test]
fn splitmix64_transition_matches_frozen_constants() {
    let mut rng = SplitMix64::new(0);
    let first = rng.next_u64();
    assert_ne!(first, 0);
    let mut reference = 0_u64;
    reference = reference.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = reference;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    let expected = z ^ (z >> 31);
    assert_eq!(first, expected);
}

#[test]
fn budget_failed_charge_leaves_counters_unchanged() {
    let mut budget = Budget::default();
    budget.charge(ResourceKind::ReceivedObservations, 10).unwrap();
    let before_run = budget.run_value(ResourceKind::ReceivedObservations);
    let before_episode = budget.episode_value(ResourceKind::ReceivedObservations);
    let over_limit = MAX_RECEIVED_OBSERVATIONS_PER_EPISODE - before_episode + 1;
    let error = budget
        .charge(ResourceKind::ReceivedObservations, over_limit)
        .unwrap_err();
    assert_eq!(error.scope, "episode");
    assert_eq!(
        budget.run_value(ResourceKind::ReceivedObservations),
        before_run
    );
    assert_eq!(
        budget.episode_value(ResourceKind::ReceivedObservations),
        before_episode
    );
    assert!(budget.exhausted().is_some());
}

#[test]
fn budget_begin_episode_resets_episode_but_not_run() {
    let mut budget = Budget::default();
    budget.charge(ResourceKind::ReceivedObservations, 10).unwrap();
    budget.begin_episode();
    assert_eq!(budget.episode_value(ResourceKind::ReceivedObservations), 0);
    assert_eq!(budget.run_value(ResourceKind::ReceivedObservations), 10);
    assert_eq!(budget.peak_value(ResourceKind::ReceivedObservations), 10);
}

#[test]
fn metric_rejects_zero_denominator_and_computes_floor_rate() {
    assert!(Metric::new(0, 0).is_err());
    let third = Metric::new(1, 3).unwrap();
    assert_eq!(third.rate_micros, 333_333);
    assert!(third.clone().passed_at_least(1, 3));
    assert!(!third.passed_at_least(1, 2));
    let unanimous = Metric::new(96, 96).unwrap();
    assert!(unanimous.passed_at_least(9, 10));
}

#[test]
fn canonical_json_sorts_keys_and_sha256_matches_fips_vector() {
    use sparse_structural_rule_learner::experiment_b::protocol::{json_object, sha256_hex};
    let object = json_object(vec![("b", "2".to_owned()), ("a", "1".to_owned())]);
    assert_eq!(object, "{\"a\":1,\"b\":2}");
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}
