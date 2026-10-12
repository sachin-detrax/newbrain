use sparse_structural_rule_learner::experiment_a::a1::A1Learner;
use sparse_structural_rule_learner::experiment_a::generator::{generate_run, SymbolMode};
use sparse_structural_rule_learner::experiment_a::learner::{
    ExperimentALearner, FrozenExperimentALearner,
};
use sparse_structural_rule_learner::experiment_a::oracle::gold_parses;
use sparse_structural_rule_learner::experiment_a::protocol::{
    AnalysisResponse, PRIMARY_SCOPE_SEED, PRIMARY_WORLD_SEED, TRAINING_PARTITION_TAG,
};
use sparse_structural_rule_learner::experiment_a::runner::drive_learner;

const DEV_TAG: u64 = 0x4445_5600_0000_0001;
const FINAL_TAG: u64 = 0x4649_4e00_0000_0001;

#[test]
fn a1_admits_the_range_on_the_primary_run() {
    let run = generate_run(
        PRIMARY_WORLD_SEED,
        PRIMARY_SCOPE_SEED,
        TRAINING_PARTITION_TAG,
        DEV_TAG,
        FINAL_TAG,
        SymbolMode::Primary,
    );
    let mut learner = A1Learner::new();
    for record in run.records.iter().take(64) {
        for atom in &record.wire_tokens {
            learner
                .ingest(
                    sparse_structural_rule_learner::experiment_a::protocol::StreamEvent::Token {
                        run_scope: record.run_scope,
                        stream_scope: record.stream_scope,
                        atom: *atom,
                    },
                )
                .unwrap();
        }
        learner
            .ingest(
                sparse_structural_rule_learner::experiment_a::protocol::StreamEvent::End {
                    run_scope: record.run_scope,
                    stream_scope: record.stream_scope,
                },
            )
            .unwrap();
        learner.analyze().unwrap();
    }
    learner.begin_development().unwrap();
    for record in run.records.iter().skip(64).take(32) {
        for atom in &record.wire_tokens {
            learner
                .ingest(
                    sparse_structural_rule_learner::experiment_a::protocol::StreamEvent::Token {
                        run_scope: record.run_scope,
                        stream_scope: record.stream_scope,
                        atom: *atom,
                    },
                )
                .unwrap();
        }
        learner
            .ingest(
                sparse_structural_rule_learner::experiment_a::protocol::StreamEvent::End {
                    run_scope: record.run_scope,
                    stream_scope: record.stream_scope,
                },
            )
            .unwrap();
        learner.analyze().unwrap();
    }
    let frozen = learner.freeze().unwrap();
    assert!(frozen.admitted());
    assert!(!frozen.model_fingerprint().is_empty());
}

#[test]
fn a1_transfers_to_held_out_lengths() {
    let run = generate_run(
        PRIMARY_WORLD_SEED,
        PRIMARY_SCOPE_SEED,
        TRAINING_PARTITION_TAG,
        DEV_TAG,
        FINAL_TAG,
        SymbolMode::Primary,
    );
    let (trace, _) = drive_learner(&run.records, A1Learner::new()).unwrap();
    let mut matched = 0;
    let mut total = 0;
    for (record, outcome) in run
        .records
        .iter()
        .zip(trace.outcomes.iter())
        .skip(96)
        .take(16)
    {
        let gold = &gold_parses(record)[0];
        if gold.boundaries.is_empty() {
            continue;
        }
        total += gold.boundaries.len();
        if let AnalysisResponse::Selected { parse } = &outcome.decision.response {
            for boundary in &gold.boundaries {
                if parse.boundaries.contains(boundary) {
                    matched += 1;
                }
            }
        }
    }
    assert!(total > 0);
    assert!(matched * 100 >= total * 90, "matched {matched}/{total}");
}

#[test]
fn a1_reports_no_relations_on_negative_controls() {
    let run = generate_run(
        PRIMARY_WORLD_SEED,
        PRIMARY_SCOPE_SEED,
        TRAINING_PARTITION_TAG,
        DEV_TAG,
        FINAL_TAG,
        SymbolMode::Primary,
    );
    let (trace, _) = drive_learner(&run.records, A1Learner::new()).unwrap();
    let mut false_positives = 0;
    let mut checked = 0;
    for (record, outcome) in run
        .records
        .iter()
        .zip(trace.outcomes.iter())
        .skip(96 + 44)
        .take(8)
    {
        assert!(record.latent_echo_spans.is_empty());
        checked += 1;
        if let AnalysisResponse::Selected { parse } = &outcome.decision.response {
            if !parse.relation_uses.is_empty() {
                false_positives += 1;
            }
        }
    }
    assert_eq!(checked, 8);
    assert_eq!(false_positives, 0);
}

#[test]
fn a1_marks_ambiguity_with_alternatives_or_abstention() {
    let run = generate_run(
        PRIMARY_WORLD_SEED,
        PRIMARY_SCOPE_SEED,
        TRAINING_PARTITION_TAG,
        DEV_TAG,
        FINAL_TAG,
        SymbolMode::Primary,
    );
    let (trace, _) = drive_learner(&run.records, A1Learner::new()).unwrap();
    let mut honest = 0;
    let mut checked = 0;
    for (record, outcome) in run
        .records
        .iter()
        .zip(trace.outcomes.iter())
        .skip(96 + 52)
        .take(12)
    {
        assert!(record.ambiguous_core_offset.is_some());
        checked += 1;
        match &outcome.decision.response {
            AnalysisResponse::Alternatives { .. } => honest += 1,
            AnalysisResponse::Abstain { .. } => honest += 1,
            AnalysisResponse::Selected { .. } => {}
        }
    }
    assert_eq!(checked, 12);
    assert!(honest >= 10, "honest {honest}/12");
}

#[test]
fn a1_reruns_are_deterministic() {
    let run = generate_run(
        PRIMARY_WORLD_SEED,
        PRIMARY_SCOPE_SEED,
        TRAINING_PARTITION_TAG,
        DEV_TAG,
        FINAL_TAG,
        SymbolMode::Primary,
    );
    let (first, _) = drive_learner(&run.records, A1Learner::new()).unwrap();
    let (second, _) = drive_learner(&run.records, A1Learner::new()).unwrap();
    assert_eq!(first.outcomes, second.outcomes);
}
