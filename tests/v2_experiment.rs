use sparse_structural_rule_learner::v2::evaluator_world::RunVariant;
use sparse_structural_rule_learner::v2::experiment::{
    compare_reruns, run_one, run_one_rerun, ExperimentRunner, FailureClassification, Predictor,
    RERUN_COUNT,
};

#[test]
fn runner_keeps_the_exact_schedule_and_freeze_boundary() {
    let report = run_one(RunVariant::Primary, Predictor::B1SeededRandomExploration).unwrap();
    assert_eq!(report.episodes.len(), 36);
    assert_eq!(report.episodes.first().unwrap().label, "T01");
    assert_eq!(report.episodes[11].label, "T12");
    assert_eq!(report.episodes[12].label, "D01");
    assert_eq!(report.episodes[19].label, "D08");
    assert_eq!(report.episodes[20].label, "F01");
    assert_eq!(report.episodes.last().unwrap().label, "F16");
    assert_eq!(report.freeze_boundary, 20);
    assert!(report.freeze_called);
    assert_eq!(
        report.decision_traces.len(),
        report
            .episodes
            .iter()
            .map(|episode| episode.decision_sequences.len())
            .sum::<usize>()
    );
    #[cfg(any(target_os = "linux", windows))]
    {
        assert!(report.physical_resources.wall_clock_ns > 0);
        assert!(matches!(
            report.physical_resources.peak_process_memory_bytes,
            sparse_structural_rule_learner::v2::resource_scale::MemorySample::Measured(bytes)
                if bytes > 0
        ));
        assert_eq!(report.physical_resources.process_count, 1);
        assert_eq!(report.physical_resources.worker_count, 1);
        assert_eq!(report.physical_resources.gpu_count, 0);
        assert_eq!(report.physical_resources.network_connection_count, 0);
        assert!(report.physical_resources.stage_operations.canonicalization > 0);
    }
}

#[test]
fn baselines_are_isolated_from_learner_state_and_resource_totals() {
    let b0 = run_one(RunVariant::Primary, Predictor::B0ExactInteractionRetrieval).unwrap();
    let b1 = run_one(RunVariant::Primary, Predictor::B1SeededRandomExploration).unwrap();
    let b2 = run_one(RunVariant::Primary, Predictor::B2EvaluatorController).unwrap();

    assert!(b0.candidate_events.is_empty());
    assert!(b0.procedure_records.is_empty());
    assert_eq!(
        b0.metrics
            .held_out_applicable_prediction_coverage
            .numerator(),
        0
    );
    assert!(b1.candidate_events.is_empty());
    assert!(b1.procedure_records.is_empty());
    assert!(!b2.logical_resources.included);
    assert!(!b2.physical_resources.included);
    assert_eq!(b2.metrics.held_out_applicable_task_success.numerator(), 6);
    assert_eq!(
        b2.metrics.held_out_applicable_packet_accuracy.numerator(),
        6
    );
}

#[test]
fn deterministic_core_and_three_rerun_comparison_are_stable() {
    let reports = (1..=RERUN_COUNT)
        .map(|index| {
            run_one_rerun(
                RunVariant::Primary,
                Predictor::B1SeededRandomExploration,
                index,
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let comparison = compare_reruns(&reports);
    assert_eq!(comparison.present_count, RERUN_COUNT);
    assert!(comparison.byte_identical);
    assert_eq!(comparison.core_fingerprints.len(), RERUN_COUNT);
    assert!(reports
        .windows(2)
        .all(|pair| pair[0].to_json() == pair[1].to_json()));
}

#[test]
fn proposed_run_records_freeze_and_leakage_status() {
    let report = run_one(RunVariant::Primary, Predictor::Proposed).unwrap();
    assert!(report.freeze_called);
    assert!(report.frozen_state_unchanged);
    assert_eq!(report.leakage.prohibited_input_fields, 0);
    assert_eq!(report.leakage.evaluator_calls_from_learner, 0);
    assert_eq!(report.leakage.decision_before_result_violations, 0);
    assert_eq!(report.leakage.final_result_selection_uses, 0);
    assert!(report
        .failures
        .iter()
        .all(|failure| failure.classification != FailureClassification::FalsifiedByLeakage));
}

#[test]
fn metric_denominators_and_gate_threshold_inputs_are_explicit() {
    assert!(sparse_structural_rule_learner::v2::Rate::new(0, 0).is_err());
    let report = run_one(RunVariant::Primary, Predictor::B2EvaluatorController).unwrap();
    assert_eq!(
        report
            .metrics
            .held_out_applicable_task_success
            .denominator(),
        6
    );
    assert_eq!(
        report
            .metrics
            .held_out_non_applicable_false_positives
            .denominator(),
        6
    );
    assert_eq!(report.metrics.frozen_retention.denominator(), 4);
    assert_eq!(
        report
            .metrics
            .held_out_applicable_packet_accuracy
            .denominator(),
        6
    );
    assert_eq!(
        report.metrics.held_out_applicable_task_success.numerator(),
        6
    );
    assert_eq!(
        report
            .metrics
            .held_out_applicable_packet_accuracy
            .numerator(),
        6
    );

    let zero_attempts = run_one(RunVariant::Primary, Predictor::B1SeededRandomExploration).unwrap();
    assert_eq!(
        zero_attempts
            .metrics
            .held_out_applicable_prediction_attempts,
        0
    );
    assert_eq!(
        zero_attempts
            .metrics
            .held_out_applicable_packet_accuracy
            .numerator(),
        0
    );
    assert_eq!(
        zero_attempts
            .metrics
            .held_out_applicable_packet_accuracy
            .denominator(),
        6
    );
}

#[test]
fn all_runner_packages_primary_and_secondary_gate_arrays_separately() {
    let report = ExperimentRunner::run_all().unwrap();
    assert_eq!(report.runs.len(), 2 * 4 * RERUN_COUNT);
    assert_eq!(report.primary_gates.len(), 12);
    assert_eq!(report.secondary_gates.len(), 7);
    assert!(report
        .deterministic_json()
        .contains("\"core_fingerprints\":[\""));
    #[cfg(any(target_os = "linux", windows))]
    {
        assert!(report
            .primary_gates
            .iter()
            .any(|gate| gate.id == "P12" && gate.passed));
        assert!(report
            .secondary_gates
            .iter()
            .any(|gate| gate.id == "S7" && gate.passed));
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        assert!(report
            .primary_gates
            .iter()
            .any(|gate| gate.id == "P12" && !gate.passed));
        assert!(report
            .secondary_gates
            .iter()
            .any(|gate| gate.id == "S7" && !gate.passed));
    }
}
