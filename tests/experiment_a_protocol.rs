use sparse_structural_rule_learner::experiment_a::protocol::{
    Budget, Metric, ResourceKind, SplitMix64, AMBIGUOUS_FINAL_RECORDS, DEVELOPMENT_RECORDS,
    FINAL_PARTITION_TAG, FINAL_RECORDS, GENERATOR_VERSION, GRAMMAR_VERSION,
    MAX_ACTIVE_HYPOTHESES_PER_RUN, MAX_ACTIVE_HYPOTHESES_PER_STREAM, MAX_ALIGNMENT_OPS_PER_RUN,
    MAX_ALIGNMENT_OPS_PER_STREAM, MAX_ARTIFACT_BYTES_PER_RUN, MAX_ARTIFACT_BYTES_PER_STREAM,
    MAX_ATOMS_PER_RUN, MAX_ATOMS_PER_STREAM, MAX_EVIDENCE_RECORDS_PER_RUN,
    MAX_EVIDENCE_RECORDS_PER_STREAM, MAX_LIBRARY_ENTRIES_PER_RUN, MAX_LIBRARY_ENTRIES_PER_STREAM,
    MAX_RELATION_CANDIDATES_PER_RUN, MAX_RELATION_CANDIDATES_PER_STREAM,
    MAX_RELATION_COMPARISONS_PER_RUN, MAX_RELATION_COMPARISONS_PER_STREAM,
    MAX_RESIDENT_MEMORY_BYTES, MAX_SEGMENTATION_HYPOTHESES_PER_RUN,
    MAX_SEGMENTATION_HYPOTHESES_PER_STREAM, MAX_SPAN_PAIRS_PER_RUN, MAX_SPAN_PAIRS_PER_STREAM,
    MAX_WALL_CLOCK_NS, ORDINAL_MULTIPLIER, PERMUTATION_BASELINE_SEED, PERMUTATION_RERUN_SEED,
    PERMUTATION_SCOPE_SEED, PERMUTATION_STATE_XOR, PERMUTATION_WORLD_SEED, PRIMARY_BASELINE_SEED,
    PRIMARY_RERUN_SEED, PRIMARY_SCOPE_SEED, PRIMARY_WORLD_SEED, PROTOCOL_VERSION,
    SPECIFICATION_VERSION, TOTAL_RECORDS, TRAINING_PARTITION_TAG, TRAINING_RECORDS,
    UNAMBIGUOUS_FINAL_RECORDS,
};

#[test]
fn frozen_version_strings_match_registration() {
    assert_eq!(
        SPECIFICATION_VERSION,
        "architecture-experiment-a-variable-length-boundary-discovery/1.0.0"
    );
    assert_eq!(
        PROTOCOL_VERSION,
        "variable-length-boundary-discovery-experiment-a-protocol-v1.0"
    );
    assert_eq!(GENERATOR_VERSION, "variable-stream-generator-a-v1");
    assert_eq!(GRAMMAR_VERSION, "variable-stream-structure-a-v1");
}

#[test]
fn frozen_partition_counts_match_protocol() {
    assert_eq!(TRAINING_RECORDS, 64);
    assert_eq!(DEVELOPMENT_RECORDS, 32);
    assert_eq!(FINAL_RECORDS, 64);
    assert_eq!(TOTAL_RECORDS, 160);
    assert_eq!(
        TRAINING_RECORDS + DEVELOPMENT_RECORDS + FINAL_RECORDS,
        TOTAL_RECORDS
    );
    assert_eq!(UNAMBIGUOUS_FINAL_RECORDS, 52);
    assert_eq!(AMBIGUOUS_FINAL_RECORDS, 12);
    assert_eq!(
        UNAMBIGUOUS_FINAL_RECORDS + AMBIGUOUS_FINAL_RECORDS,
        FINAL_RECORDS
    );
}

#[test]
fn frozen_seeds_match_protocol() {
    assert_eq!(PRIMARY_WORLD_SEED, 2_026_101_201);
    assert_eq!(PRIMARY_SCOPE_SEED, 2_026_101_211);
    assert_eq!(PRIMARY_BASELINE_SEED, 2_026_101_221);
    assert_eq!(PRIMARY_RERUN_SEED, 2_026_101_231);
    assert_eq!(PERMUTATION_WORLD_SEED, 2_026_101_202);
    assert_eq!(PERMUTATION_SCOPE_SEED, 2_026_101_212);
    assert_eq!(PERMUTATION_BASELINE_SEED, 2_026_101_222);
    assert_eq!(PERMUTATION_RERUN_SEED, 2_026_101_232);
}

#[test]
fn frozen_partition_tags_match_protocol() {
    assert_eq!(TRAINING_PARTITION_TAG, 0x5452_4149_4e00_0001);
    assert_eq!(FINAL_PARTITION_TAG, 0x4649_4e00_0000_0001);
    assert_eq!(ORDINAL_MULTIPLIER, 0xD1B5_4A32_D192_ED03);
    assert_eq!(PERMUTATION_STATE_XOR, 0x94D0_49BB_1331_11EB);
}

#[test]
fn frozen_resource_ceilings_match_protocol() {
    assert_eq!((MAX_ATOMS_PER_STREAM, MAX_ATOMS_PER_RUN), (48, 7_680));
    assert_eq!(
        (MAX_SPAN_PAIRS_PER_STREAM, MAX_SPAN_PAIRS_PER_RUN),
        (1_024, 163_840)
    );
    assert_eq!(
        (MAX_ALIGNMENT_OPS_PER_STREAM, MAX_ALIGNMENT_OPS_PER_RUN),
        (4_096, 655_360)
    );
    assert_eq!(
        (
            MAX_SEGMENTATION_HYPOTHESES_PER_STREAM,
            MAX_SEGMENTATION_HYPOTHESES_PER_RUN
        ),
        (64, 512)
    );
    assert_eq!(
        (
            MAX_RELATION_CANDIDATES_PER_STREAM,
            MAX_RELATION_CANDIDATES_PER_RUN
        ),
        (512, 81_920)
    );
    assert_eq!(
        (
            MAX_RELATION_COMPARISONS_PER_STREAM,
            MAX_RELATION_COMPARISONS_PER_RUN
        ),
        (2_048, 327_680)
    );
    assert_eq!(
        (
            MAX_ACTIVE_HYPOTHESES_PER_STREAM,
            MAX_ACTIVE_HYPOTHESES_PER_RUN
        ),
        (16, 128)
    );
    assert_eq!(
        (MAX_LIBRARY_ENTRIES_PER_STREAM, MAX_LIBRARY_ENTRIES_PER_RUN),
        (8, 32)
    );
    assert_eq!(
        (
            MAX_EVIDENCE_RECORDS_PER_STREAM,
            MAX_EVIDENCE_RECORDS_PER_RUN
        ),
        (256, 40_960)
    );
    assert_eq!(
        (MAX_ARTIFACT_BYTES_PER_STREAM, MAX_ARTIFACT_BYTES_PER_RUN),
        (2 * 1024 * 1024, 64 * 1024 * 1024)
    );
    assert_eq!(MAX_WALL_CLOCK_NS, 120_000_000_000);
    assert_eq!(MAX_RESIDENT_MEMORY_BYTES, 1_073_741_824);
    assert_eq!(ResourceKind::all().len(), 10);
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
fn budget_failed_charge_leaves_counters_unchanged() {
    let mut budget = Budget::default();
    budget.charge(ResourceKind::ReceivedAtoms, 10).unwrap();
    let before_run = budget.run_value(ResourceKind::ReceivedAtoms);
    let before_stream = budget.stream_value(ResourceKind::ReceivedAtoms);
    let over_limit = MAX_ATOMS_PER_STREAM - before_stream + 1;
    let error = budget
        .charge(ResourceKind::ReceivedAtoms, over_limit)
        .unwrap_err();
    assert_eq!(error.scope, "stream");
    assert_eq!(budget.run_value(ResourceKind::ReceivedAtoms), before_run);
    assert_eq!(
        budget.stream_value(ResourceKind::ReceivedAtoms),
        before_stream
    );
    assert!(budget.exhausted().is_some());
}

#[test]
fn budget_begin_stream_resets_stream_but_not_run() {
    let mut budget = Budget::default();
    budget.charge(ResourceKind::ReceivedAtoms, 10).unwrap();
    budget.begin_stream();
    assert_eq!(budget.stream_value(ResourceKind::ReceivedAtoms), 0);
    assert_eq!(budget.run_value(ResourceKind::ReceivedAtoms), 10);
    assert_eq!(budget.peak_value(ResourceKind::ReceivedAtoms), 10);
}

#[test]
fn metric_rejects_zero_denominator_and_computes_floor_rate() {
    assert!(Metric::new(0, 0).is_err());
    let third = Metric::new(1, 3).unwrap();
    assert_eq!(third.rate_micros, 333_333);
    assert!(third.clone().passed_at_least(1, 3));
    assert!(!third.passed_at_least(1, 2));
    let unanimous = Metric::new(52, 52).unwrap();
    assert!(unanimous.passed_at_least(9, 10));
}

#[test]
fn canonical_json_sorts_keys_and_sha256_matches_fips_vector() {
    use sparse_structural_rule_learner::experiment_a::protocol::{json_object, sha256_hex};
    let object = json_object(vec![("b", "2".to_owned()), ("a", "1".to_owned())]);
    assert_eq!(object, "{\"a\":1,\"b\":2}");
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}
