use sparse_structural_rule_learner::v2::resource_scale::{
    deterministic_scale_points, LogicalResource, MeasurementError, MemorySample, ResourceMeter,
    ResourceScope, ScaleResult, ScaleSweep, Stage,
};

#[test]
fn v2_counters_accumulate_and_stage_counters_remain_separate() {
    let mut meter = ResourceMeter::new();
    meter.begin_episode();
    meter.begin_decision();
    meter.record_query_call().unwrap();
    meter.record_environment_action().unwrap();
    meter.record_candidate_generation().unwrap();
    meter.record_structural_comparison().unwrap();
    meter.record_operation_step().unwrap();
    meter.set_active_hypotheses(3).unwrap();
    meter.set_retained_interaction_records(2).unwrap();
    meter.set_retained_procedure_records(1).unwrap();
    meter.set_ledger_size_bytes(17);
    meter
        .record_stage_operation(Stage::Canonicalization, 4)
        .unwrap();
    meter.record_stage_operation(Stage::Retrieval, 2).unwrap();
    meter.record_stage_operation(Stage::Admission, 5).unwrap();
    meter
        .record_stage_operation(Stage::Revalidation, 1)
        .unwrap();

    let counters = meter.counters();
    assert_eq!(counters.query_calls(), 1);
    assert_eq!(counters.environment_actions(), 1);
    assert_eq!(counters.candidate_generation_attempts(), 1);
    assert_eq!(counters.structural_comparisons(), 1);
    assert_eq!(counters.active_hypotheses(), 3);
    assert_eq!(counters.operation_steps(), 1);
    assert_eq!(counters.retained_interaction_records(), 2);
    assert_eq!(counters.retained_procedure_records(), 1);
    assert_eq!(counters.ledger_size_bytes(), 17);
    assert_eq!(meter.stage(Stage::Canonicalization).operation_count(), 4);
    assert_eq!(meter.stage(Stage::Retrieval).operation_count(), 2);
    assert_eq!(meter.stage(Stage::Admission).operation_count(), 5);
    assert_eq!(meter.stage(Stage::Revalidation).operation_count(), 1);
    assert_eq!(meter.stage(Stage::CandidateGeneration).operation_count(), 1);
    assert_eq!(meter.stage(Stage::DecisionAction).operation_count(), 2);
}

#[test]
fn missing_memory_is_explicit_and_injected_samples_track_the_peak() {
    let missing = ResourceMeter::new();
    assert_eq!(missing.peak_resident_memory(), MemorySample::Missing);
    assert_eq!(
        missing.validate_limits(),
        Err(MeasurementError::MissingMemoryMeasurement)
    );

    let mut samples = vec![Some(40), Some(12), None];
    let mut measured = ResourceMeter::with_memory_sampler(move || samples.remove(0));
    assert_eq!(
        measured.sample_peak_resident_memory(),
        MemorySample::Measured(40)
    );
    assert_eq!(
        measured.sample_peak_resident_memory(),
        MemorySample::Measured(40)
    );
    assert_eq!(
        measured.sample_peak_resident_memory(),
        MemorySample::Measured(40)
    );
}

#[cfg(any(target_os = "linux", windows))]
#[test]
fn runtime_meter_uses_the_platform_sampler() {
    let mut meter = ResourceMeter::runtime();

    let sample = meter.sample_peak_resident_memory();

    assert!(matches!(sample, MemorySample::Measured(bytes) if bytes > 0));
}

#[cfg(not(any(target_os = "linux", windows)))]
#[test]
fn unsupported_runtime_platform_remains_explicitly_missing() {
    let mut meter = ResourceMeter::runtime();

    assert_eq!(meter.sample_peak_resident_memory(), MemorySample::Missing);
}

#[test]
fn logical_limit_rejects_before_mutating_the_counter() {
    let mut meter = ResourceMeter::new();
    meter.begin_episode();
    meter.begin_decision();
    meter
        .try_charge(LogicalResource::CandidateGenerationAttempts, 64)
        .unwrap();
    let before = meter.counters();
    let error = meter
        .try_charge(LogicalResource::CandidateGenerationAttempts, 1)
        .unwrap_err();

    let logical_error = match error {
        MeasurementError::LogicalLimit(error) => error,
        other => panic!("expected a logical limit error, got {other:?}"),
    };
    assert_eq!(
        logical_error.resource(),
        LogicalResource::CandidateGenerationAttempts
    );
    assert_eq!(logical_error.scope(), ResourceScope::Decision);
    assert_eq!(logical_error.used(), 64);
    assert_eq!(logical_error.requested(), 1);
    assert_eq!(logical_error.limit(), 64);
    assert_eq!(meter.counters(), before);
}

#[test]
fn scale_points_and_serialization_are_canonical() {
    let first_points = deterministic_scale_points(&[8, 2, 2], &[4, 1], &[3], &[2, 1]);
    let second_points = deterministic_scale_points(&[2, 8], &[1, 4], &[3], &[1, 2]);
    assert_eq!(first_points, second_points);
    assert_eq!(first_points.len(), 8);

    let first = ScaleSweep::new(
        first_points
            .iter()
            .copied()
            .map(|point| ScaleResult::new(point, Default::default()))
            .collect(),
    );
    let second = ScaleSweep::new(
        second_points
            .iter()
            .rev()
            .copied()
            .map(|point| ScaleResult::new(point, Default::default()))
            .collect(),
    );
    assert_eq!(first.to_json(), second.to_json());
    assert!(first
        .to_json()
        .contains("\"peak_resident_memory_bytes\":null"));
    assert!(first
        .to_json()
        .contains("\"peak_resident_memory_state\":\"missing\""));
}

#[test]
fn wall_clock_measurement_is_monotonic_and_stage_windows_are_recorded() {
    let mut meter = ResourceMeter::new();
    let before = meter.elapsed_wall_clock_ns();
    meter.start_stage(Stage::Canonicalization).unwrap();
    meter
        .record_stage_operation(Stage::Canonicalization, 1)
        .unwrap();
    meter.finish_stage(Stage::Canonicalization).unwrap();
    let after = meter.elapsed_wall_clock_ns();

    assert!(after >= before);
    assert_eq!(meter.stage(Stage::Canonicalization).operation_count(), 1);
}
