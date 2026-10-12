use super::a0::A0Learner;
use super::a1::A1Learner;
use super::a3::{A3Learner, LibraryReport};
use super::artifact;
use super::gates::{
    self, FailureClassification, GateResult, RunIntegrity, RunResultState, MAX_MEMORY_BYTES,
    MAX_WALL_CLOCK_NS,
};
use super::generator::{generate_run, GeneratedRecord, SymbolMode};
use super::learner::FrozenExperimentALearner;
use super::physical::sample_resident_memory_bytes;
use super::protocol::{
    json_object, json_string, GENERATOR_VERSION, GRAMMAR_VERSION, PRIMARY_SCOPE_SEED,
    PRIMARY_WORLD_SEED, PROTOCOL_VERSION, SPECIFICATION_VERSION, TRAINING_PARTITION_TAG,
};
use super::runner::{drive_learner, RunTrace};
use super::scorer::{score_final, FinalMetrics};

pub const DEV_TAG: u64 = 0x4445_5600_0000_0001;
pub const FINAL_TAG: u64 = 0x4649_4e00_0000_0001;
pub const PERMUTATION_WORLD_SEED: u64 = 2_026_101_202;
pub const PERMUTATION_SCOPE_SEED: u64 = 2_026_101_212;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Variant {
    A0,
    A1,
    A3,
}

impl Variant {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::A0 => "A0",
            Self::A1 => "A1",
            Self::A3 => "A3",
        }
    }
}

pub fn seeds_for(mode: SymbolMode) -> (u64, u64) {
    match mode {
        SymbolMode::Primary => (PRIMARY_WORLD_SEED, PRIMARY_SCOPE_SEED),
        SymbolMode::Permutation => (PERMUTATION_WORLD_SEED, PERMUTATION_SCOPE_SEED),
    }
}

pub fn mode_str(mode: SymbolMode) -> &'static str {
    match mode {
        SymbolMode::Primary => "primary",
        SymbolMode::Permutation => "permutation",
    }
}

#[derive(Clone, Debug)]
pub struct SingleReport {
    pub variant: Variant,
    pub mode: SymbolMode,
    pub rerun: u32,
    pub world_seed: u64,
    pub scope_seed: u64,
    pub records: Vec<GeneratedRecord>,
    pub trace: RunTrace,
    pub metrics: FinalMetrics,
    pub admitted: bool,
    pub admitted_fingerprint: Option<String>,
    pub library: Option<LibraryReport>,
    pub gates: Vec<GateResult>,
    pub integrity: RunIntegrity,
    pub core: Vec<(String, String)>,
    pub physical_json: String,
    pub evaluator_only: Vec<(String, String)>,
    pub manifest_json: String,
    pub classification: Option<FailureClassification>,
}

pub fn run_single(variant: Variant, mode: SymbolMode, rerun: u32) -> SingleReport {
    let (world_seed, scope_seed) = seeds_for(mode);
    let run = generate_run(
        world_seed,
        scope_seed,
        TRAINING_PARTITION_TAG,
        DEV_TAG,
        FINAL_TAG,
        mode,
    );
    let (trace, admitted, admitted_fingerprint, library) = match variant {
        Variant::A0 => {
            let (trace, _) = drive_learner(&run.records, A0Learner::new()).unwrap();
            (trace, false, None, None)
        }
        Variant::A1 => {
            let (trace, frozen) = drive_learner(&run.records, A1Learner::new()).unwrap();
            let admitted = frozen.admitted();
            let fingerprint = admitted.then(|| frozen.model_fingerprint().to_owned());
            (trace, admitted, fingerprint, None)
        }
        Variant::A3 => {
            let (trace, frozen) = drive_learner(&run.records, A3Learner::new()).unwrap();
            let admitted = frozen.a1().admitted();
            let fingerprint = admitted.then(|| frozen.model_fingerprint().to_owned());
            let library = Some(frozen.library().clone());
            (trace, admitted, fingerprint, library)
        }
    };
    let metrics = score_final(&run.records, &trace.outcomes, admitted);
    let (memory_bytes, memory_state) = sample_resident_memory_bytes();
    let integrity = RunIntegrity {
        records_delivered: trace.outcomes.len(),
        in_order: trace
            .outcomes
            .iter()
            .enumerate()
            .all(|(index, outcome)| outcome.global_ordinal == (index + 1) as u64),
        wall_clock_ns: trace.wall_clock_ns,
        memory_bytes,
        memory_state,
    };
    let gates = match variant {
        Variant::A0 => gates::evaluate_a0_run(&metrics, &integrity, 0),
        Variant::A1 => gates::evaluate_a1_run(&metrics, &integrity),
        Variant::A3 => gates::evaluate_a3_run(&metrics, &integrity, library.as_ref().unwrap()),
    };
    let leakage = metrics.late_refs > 0;
    let classification = classify_single(variant, &gates, leakage, admitted);
    let core = build_core(
        variant,
        &run.records,
        &trace,
        &metrics,
        &gates,
        library.as_ref(),
        admitted_fingerprint.as_deref(),
    );
    let physical_json =
        artifact::physical_resource_report_json(trace.wall_clock_ns, (memory_bytes, memory_state));
    let evaluator_only = vec![
        (
            "evaluator-only/generator-manifest.json".to_owned(),
            artifact::generator_manifest_json(
                variant.as_str(),
                mode_str(mode),
                world_seed,
                scope_seed,
                &run.records,
            ),
        ),
        (
            "evaluator-only/latent-record-manifest.json".to_owned(),
            artifact::generator_manifest_json(
                variant.as_str(),
                mode_str(mode),
                world_seed,
                scope_seed,
                &run.records,
            ),
        ),
        (
            "evaluator-only/score-input.json".to_owned(),
            artifact::score_input_json(&run.records),
        ),
    ];
    let manifest_json = json_object(vec![
        (
            "classification",
            json_string(
                classification
                    .map(|c| c.as_str())
                    .unwrap_or("pending-group"),
            ),
        ),
        ("final_records", "64".to_owned()),
        ("generator_version", json_string(GENERATOR_VERSION)),
        ("grammar_version", json_string(GRAMMAR_VERSION)),
        ("mode", json_string(mode_str(mode))),
        ("protocol_version", json_string(PROTOCOL_VERSION)),
        ("rerun", rerun.to_string()),
        (
            "resource_limits",
            json_object(vec![
                ("max_resident_memory_bytes", MAX_MEMORY_BYTES.to_string()),
                ("max_wall_clock_ns", MAX_WALL_CLOCK_NS.to_string()),
            ]),
        ),
        ("scope_seed", json_string(&scope_seed.to_string())),
        ("specification_version", json_string(SPECIFICATION_VERSION)),
        ("total_records", "160".to_owned()),
        ("training_records", "64".to_owned()),
        ("development_records", "32".to_owned()),
        ("variant", json_string(variant.as_str())),
        ("world_seed", json_string(&world_seed.to_string())),
    ]);
    SingleReport {
        variant,
        mode,
        rerun,
        world_seed,
        scope_seed,
        records: run.records,
        trace,
        metrics,
        admitted,
        admitted_fingerprint,
        library,
        gates,
        integrity,
        core,
        physical_json,
        evaluator_only,
        manifest_json,
        classification,
    }
}

fn classify_single(
    variant: Variant,
    gates: &[GateResult],
    leakage: bool,
    admitted: bool,
) -> Option<FailureClassification> {
    if leakage {
        return Some(FailureClassification::FalsifiedByLeakage);
    }
    let fails = |id: &str| gates.iter().any(|gate| gate.id == id && !gate.passed);
    if fails("A1-P1") || fails("A0-C1") {
        return Some(FailureClassification::FalsifiedByProtocol);
    }
    if fails("A1-P13") || fails("A0-C4") {
        return Some(FailureClassification::FalsifiedByResourceLimit);
    }
    if fails("A1-P12") {
        return Some(FailureClassification::FalsifiedByBudget);
    }
    if variant != Variant::A0 && !admitted {
        return Some(FailureClassification::InsufficientEvidence);
    }
    if fails("A1-P3") || fails("A1-P4") || fails("A1-P5") || fails("A1-P6") {
        return Some(FailureClassification::FalsifiedByBoundaryGeneralization);
    }
    if fails("A1-P7") || fails("A1-P8") {
        return Some(FailureClassification::FalsifiedByStructuralTransfer);
    }
    if fails("A1-P9") {
        return Some(FailureClassification::FalsifiedByFalsePositiveControl);
    }
    if fails("A1-P10") {
        return Some(FailureClassification::FalsifiedByAmbiguityHonesty);
    }
    if fails("A1-P11") {
        return Some(FailureClassification::FalsifiedByEvidenceTraceability);
    }
    if variant == Variant::A3
        && (fails("A3-C1") || fails("A3-C2") || fails("A3-C3") || fails("A3-C5"))
    {
        return Some(FailureClassification::ComparatorLibraryFailed);
    }
    None
}

fn build_core(
    variant: Variant,
    records: &[GeneratedRecord],
    trace: &RunTrace,
    metrics: &FinalMetrics,
    gates: &[GateResult],
    library: Option<&LibraryReport>,
    admitted_fingerprint: Option<&str>,
) -> Vec<(String, String)> {
    let enumerates = variant != Variant::A0;
    let library_entries = library
        .map(|report| report.entries.len() as u64)
        .unwrap_or(0);
    vec![
        (
            "learner/stream-events.jsonl".to_owned(),
            artifact::stream_events_jsonl(records),
        ),
        (
            "learner/analysis-trace.jsonl".to_owned(),
            artifact::analysis_trace_jsonl(trace),
        ),
        (
            "learner/candidate-events.jsonl".to_owned(),
            artifact::candidate_events_jsonl(records, enumerates),
        ),
        (
            "learner/relation-records.jsonl".to_owned(),
            artifact::relation_records_jsonl(trace, admitted_fingerprint),
        ),
        (
            "learner/library-records.jsonl".to_owned(),
            artifact::library_records_jsonl(library),
        ),
        (
            "learner/evidence-records.jsonl".to_owned(),
            artifact::evidence_records_jsonl(records),
        ),
        ("metrics.json".to_owned(), artifact::metrics_json(metrics)),
        (
            "acceptance.json".to_owned(),
            artifact::acceptance_json(gates),
        ),
        ("failures.jsonl".to_owned(), artifact::failures_jsonl(gates)),
        (
            "logical-resource-report.json".to_owned(),
            artifact::logical_resource_report_json(
                trace,
                records,
                library_entries,
                admitted_fingerprint.map(|_| 1).unwrap_or(0),
            ),
        ),
    ]
}

pub fn cores_equal(first: &[(String, String)], second: &[(String, String)]) -> bool {
    if first.len() != second.len() {
        return false;
    }
    first
        .iter()
        .zip(second.iter())
        .all(|(a, b)| a.0 == b.0 && a.1 == b.1)
}

#[derive(Debug)]
pub struct GroupReport {
    pub variant: Variant,
    pub mode: SymbolMode,
    pub singles: Vec<SingleReport>,
    pub p2_pass: bool,
    pub aggregate: FinalMetrics,
    pub aggregate_gates: Vec<GateResult>,
    pub state: RunResultState,
    pub classification: Option<FailureClassification>,
}

pub fn run_group(variant: Variant, mode: SymbolMode, reruns: u32) -> GroupReport {
    let mut singles = Vec::new();
    for rerun in 1..=reruns {
        singles.push(run_single(variant, mode, rerun));
    }
    let p2_pass = singles
        .iter()
        .skip(1)
        .all(|report| cores_equal(&singles[0].core, &report.core));
    let aggregate = artifact::aggregate_metrics(
        &singles
            .iter()
            .map(|report| report.metrics.clone())
            .collect::<Vec<_>>(),
    );
    let aggregate_gates = gates::evaluate_a1_aggregate(&aggregate);
    let per_run: Vec<Vec<GateResult>> = singles.iter().map(|report| report.gates.clone()).collect();
    let leakage = singles.iter().any(|report| report.metrics.late_refs > 0);
    let admitted = singles
        .iter()
        .all(|report| variant == Variant::A0 || report.admitted);
    let (mut state, mut classification) =
        gates::classify_a1(&per_run, &aggregate_gates, p2_pass, leakage, admitted);
    if variant == Variant::A3 && state == RunResultState::ValidatedA1NarrowClaim {
        let a3_ok = singles.iter().all(|report| {
            report
                .gates
                .iter()
                .filter(|gate| gate.id.starts_with("A3-"))
                .all(|gate| gate.passed)
        });
        if a3_ok {
            state = RunResultState::ValidatedA3Comparator;
            classification = None;
        } else {
            state = RunResultState::NegativeA3ComparatorResult;
            classification = Some(FailureClassification::ComparatorLibraryFailed);
        }
    }
    if variant == Variant::A0 {
        state = RunResultState::NoResult;
        classification = None;
    }
    GroupReport {
        variant,
        mode,
        singles,
        p2_pass,
        aggregate,
        aggregate_gates,
        state,
        classification,
    }
}

pub fn write_group_layout(root: &std::path::Path, group: &GroupReport) -> std::io::Result<()> {
    for single in &group.singles {
        let dir = root.join(format!(
            "{}-{}-rerun{}",
            single.variant.as_str(),
            mode_str(single.mode),
            single.rerun
        ));
        for (relative, bytes) in &single.core {
            artifact::write_text_file(&dir, relative, bytes)?;
        }
        artifact::write_text_file(&dir, "physical-resource-report.json", &single.physical_json)?;
        for (relative, bytes) in &single.evaluator_only {
            artifact::write_text_file(&dir, relative, bytes)?;
        }
        artifact::write_text_file(&dir, "run-manifest.json", &single.manifest_json)?;
        artifact::write_text_file(&dir, "protocol.md", artifact::PROTOCOL_MD)?;
        artifact::write_text_file(&dir, "specification.md", artifact::SPECIFICATION_MD)?;
        let mut all: Vec<(String, String)> = single.core.clone();
        all.push((
            "physical-resource-report.json".to_owned(),
            single.physical_json.clone(),
        ));
        for entry in &single.evaluator_only {
            all.push(entry.clone());
        }
        all.push(("run-manifest.json".to_owned(), single.manifest_json.clone()));
        all.push(("protocol.md".to_owned(), artifact::PROTOCOL_MD.to_owned()));
        all.push((
            "specification.md".to_owned(),
            artifact::SPECIFICATION_MD.to_owned(),
        ));
        let sums = artifact::sha256sums_text(&all);
        artifact::write_text_file(&dir, "sha256sums.txt", &sums)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a1_primary_group_validates() {
        let group = run_group(Variant::A1, SymbolMode::Primary, 3);
        assert!(group.p2_pass);
        assert_eq!(
            group.state,
            RunResultState::ValidatedA1NarrowClaim,
            "classification {:?}",
            group.classification
        );
    }

    #[test]
    fn a3_primary_group_validates_as_comparator() {
        let group = run_group(Variant::A3, SymbolMode::Primary, 3);
        assert!(group.p2_pass);
        assert_eq!(
            group.state,
            RunResultState::ValidatedA3Comparator,
            "classification {:?}",
            group.classification
        );
    }

    #[test]
    fn permutation_group_keeps_parity() {
        let primary = run_group(Variant::A1, SymbolMode::Primary, 3);
        let permuted = run_group(Variant::A1, SymbolMode::Permutation, 3);
        let (parity, detail) = gates::permutation_parity(&primary.aggregate, &permuted.aggregate);
        assert!(parity, "{detail}");
    }
}
