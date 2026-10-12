use sparse_structural_rule_learner::experiment_a::a0::A0Learner;
use sparse_structural_rule_learner::experiment_a::generator::{generate_run, SymbolMode};
use sparse_structural_rule_learner::experiment_a::oracle::gold_parses;
use sparse_structural_rule_learner::experiment_a::protocol::{
    AnalysisResponse, PRIMARY_SCOPE_SEED, PRIMARY_WORLD_SEED, TRAINING_PARTITION_TAG,
};
use sparse_structural_rule_learner::experiment_a::runner::drive_learner;

const DEV_TAG: u64 = 0x4445_5600_0000_0001;
const FINAL_TAG: u64 = 0x4649_4e00_0000_0001;

#[test]
fn a0_runs_end_to_end_with_empty_boundary_sets() {
    let run = generate_run(
        PRIMARY_WORLD_SEED,
        PRIMARY_SCOPE_SEED,
        TRAINING_PARTITION_TAG,
        DEV_TAG,
        FINAL_TAG,
        SymbolMode::Primary,
    );
    let (trace, _) = drive_learner(&run.records, A0Learner::new()).unwrap();
    assert_eq!(trace.outcomes.len(), 160);
    assert_eq!(
        trace.fingerprint_before_final,
        trace.fingerprint_after_final
    );
    for outcome in &trace.outcomes {
        match &outcome.decision.response {
            AnalysisResponse::Selected { parse } => {
                assert!(parse.boundaries.is_empty());
                assert_eq!(parse.spans.len(), 1);
                assert_eq!(parse.spans[0].end, outcome.wire_len);
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}

#[test]
fn a0_recalls_zero_gold_boundaries_on_unambiguous_finals() {
    let run = generate_run(
        PRIMARY_WORLD_SEED,
        PRIMARY_SCOPE_SEED,
        TRAINING_PARTITION_TAG,
        DEV_TAG,
        FINAL_TAG,
        SymbolMode::Primary,
    );
    let (trace, _) = drive_learner(&run.records, A0Learner::new()).unwrap();
    let mut checked = 0;
    for (record, outcome) in run.records.iter().zip(trace.outcomes.iter()).skip(96) {
        if record.ambiguous_core_offset.is_some() {
            continue;
        }
        let gold = &gold_parses(record)[0];
        if gold.boundaries.is_empty() {
            continue;
        }
        checked += 1;
        match &outcome.decision.response {
            AnalysisResponse::Selected { parse } => {
                for boundary in &gold.boundaries {
                    assert!(!parse.boundaries.contains(boundary));
                }
            }
            other => panic!("unexpected {other:?}"),
        }
    }
    assert!(checked > 0);
}

#[test]
fn a0_reruns_are_deterministic() {
    let run = generate_run(
        PRIMARY_WORLD_SEED,
        PRIMARY_SCOPE_SEED,
        TRAINING_PARTITION_TAG,
        DEV_TAG,
        FINAL_TAG,
        SymbolMode::Primary,
    );
    let (first, _) = drive_learner(&run.records, A0Learner::new()).unwrap();
    let (second, _) = drive_learner(&run.records, A0Learner::new()).unwrap();
    assert_eq!(first.outcomes, second.outcomes);
    assert_eq!(
        first.fingerprint_before_final,
        second.fingerprint_before_final
    );
}
