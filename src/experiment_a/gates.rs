use super::a3::LibraryReport;
use super::physical::MemoryState;
use super::scorer::FinalMetrics;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum FailureClassification {
    FalsifiedByLeakage = 1,
    FalsifiedByProtocol = 2,
    FalsifiedByDeterminism = 3,
    FalsifiedByResourceLimit = 4,
    FalsifiedByBudget = 5,
    InsufficientEvidence = 6,
    SearchExhausted = 7,
    UnsupportedByGrammar = 8,
    IncorrectHypothesis = 9,
    FalsifiedByBoundaryGeneralization = 10,
    FalsifiedByStructuralTransfer = 11,
    FalsifiedByFalsePositiveControl = 12,
    FalsifiedByAmbiguityHonesty = 13,
    FalsifiedByEvidenceTraceability = 14,
    FalsifiedByAbstentionPolicy = 15,
    ComparatorLibraryFailed = 16,
}

impl FailureClassification {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FalsifiedByLeakage => "falsified-by-leakage",
            Self::FalsifiedByProtocol => "falsified-by-protocol",
            Self::FalsifiedByDeterminism => "falsified-by-determinism",
            Self::FalsifiedByResourceLimit => "falsified-by-resource-limit",
            Self::FalsifiedByBudget => "falsified-by-budget",
            Self::InsufficientEvidence => "insufficient-evidence",
            Self::SearchExhausted => "search-exhausted",
            Self::UnsupportedByGrammar => "unsupported-by-grammar",
            Self::IncorrectHypothesis => "incorrect-hypothesis",
            Self::FalsifiedByBoundaryGeneralization => "falsified-by-boundary-generalization",
            Self::FalsifiedByStructuralTransfer => "falsified-by-structural-transfer",
            Self::FalsifiedByFalsePositiveControl => "falsified-by-false-positive-control",
            Self::FalsifiedByAmbiguityHonesty => "falsified-by-ambiguity-honesty",
            Self::FalsifiedByEvidenceTraceability => "falsified-by-evidence-traceability",
            Self::FalsifiedByAbstentionPolicy => "falsified-by-abstention-policy",
            Self::ComparatorLibraryFailed => "comparator-library-failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunResultState {
    ValidatedA1NarrowClaim,
    ValidatedA3Comparator,
    NegativeA1NarrowResult,
    NegativeA3ComparatorResult,
    NoResult,
    InvalidatedByLeakage,
}

impl RunResultState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ValidatedA1NarrowClaim => "validated-a1-narrow-claim",
            Self::ValidatedA3Comparator => "validated-a3-comparator",
            Self::NegativeA1NarrowResult => "negative-a1-narrow-result",
            Self::NegativeA3ComparatorResult => "negative-a3-comparator-result",
            Self::NoResult => "no-result",
            Self::InvalidatedByLeakage => "invalidated-by-leakage",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GateResult {
    pub id: &'static str,
    pub passed: bool,
    pub numerator: u64,
    pub denominator: u64,
    pub detail: String,
}

fn gate(
    id: &'static str,
    passed: bool,
    numerator: u64,
    denominator: u64,
    detail: String,
) -> GateResult {
    GateResult {
        id,
        passed,
        numerator,
        denominator,
        detail,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunIntegrity {
    pub records_delivered: usize,
    pub in_order: bool,
    pub wall_clock_ns: u128,
    pub memory_bytes: Option<u64>,
    pub memory_state: MemoryState,
}

pub const MAX_WALL_CLOCK_NS: u128 = 120_000_000_000;
pub const MAX_MEMORY_BYTES: u64 = 1_073_741_824;

pub fn evaluate_a1_run(metrics: &FinalMetrics, integrity: &RunIntegrity) -> Vec<GateResult> {
    let mut gates = Vec::with_capacity(13);
    gates.push(gate(
        "A1-P1",
        integrity.records_delivered == 160 && integrity.in_order,
        integrity.records_delivered as u64,
        160,
        format!(
            "delivered {}/160 in_order={}",
            integrity.records_delivered, integrity.in_order
        ),
    ));
    gates.push(gate(
        "A1-P3",
        metrics.tp + metrics.fs > 0 && metrics.tp * 10 >= (metrics.tp + metrics.fs) * 9,
        metrics.tp,
        metrics.tp + metrics.fs,
        "micro boundary precision >= 0.90".to_owned(),
    ));
    gates.push(gate(
        "A1-P4",
        metrics.tp + metrics.fm > 0 && metrics.tp * 10 >= (metrics.tp + metrics.fm) * 9,
        metrics.tp,
        metrics.tp + metrics.fm,
        "micro boundary recall >= 0.90".to_owned(),
    ));
    gates.push(gate(
        "A1-P5",
        metrics.records_fm * 10 <= 52 && metrics.records_fs * 10 <= 52,
        metrics.records_fm + metrics.records_fs,
        104,
        format!("fm {}/52 fs {}/52", metrics.records_fm, metrics.records_fs),
    ));
    gates.push(gate(
        "A1-P6",
        metrics.selected >= 48,
        metrics.selected,
        52,
        "selected analysis coverage >= 48/52".to_owned(),
    ));
    gates.push(gate(
        "A1-P7",
        metrics.l_success + metrics.m_success >= 24
            && metrics.l_success >= 13
            && metrics.m_success >= 9,
        metrics.l_success + metrics.m_success,
        28,
        format!("L {}/16 M {}/12", metrics.l_success, metrics.m_success),
    ));
    gates.push(gate(
        "A1-P8",
        metrics.o_correct >= 6 && metrics.r_correct >= 6,
        metrics.o_correct + metrics.r_correct,
        16,
        format!("O {}/8 R {}/8", metrics.o_correct, metrics.r_correct),
    ));
    gates.push(gate(
        "A1-P9",
        metrics.n_fp == 0,
        metrics.n_fp,
        8,
        "negative false positives exactly 0/8".to_owned(),
    ));
    gates.push(gate(
        "A1-P10",
        metrics.a_honest >= 10,
        metrics.a_honest,
        12,
        "ambiguity honesty >= 10/12".to_owned(),
    ));
    let traceable = metrics.pre_decision_total > 0
        && metrics.pre_decision_ok == metrics.pre_decision_total
        && metrics.orphan_refs == 0
        && metrics.late_refs == 0;
    gates.push(gate(
        "A1-P11",
        traceable,
        metrics.pre_decision_ok,
        metrics.pre_decision_total,
        format!(
            "pre-decision {}/{} orphan {} late {}",
            metrics.pre_decision_ok,
            metrics.pre_decision_total,
            metrics.orphan_refs,
            metrics.late_refs
        ),
    ));
    gates.push(gate(
        "A1-P12",
        metrics.silent_empty == 0 && metrics.post_exhaustion_confident == 0,
        metrics.silent_empty + metrics.post_exhaustion_confident,
        1,
        "no silent defaults and no confident analysis after exhaustion".to_owned(),
    ));
    let memory_ok = match integrity.memory_bytes {
        Some(bytes) => bytes <= MAX_MEMORY_BYTES,
        None => false,
    };
    gates.push(gate(
        "A1-P13",
        integrity.wall_clock_ns <= MAX_WALL_CLOCK_NS
            && integrity.memory_state == MemoryState::Measured
            && memory_ok,
        if integrity.memory_state == MemoryState::Measured {
            1
        } else {
            0
        },
        1,
        "physical ceilings checked per rerun; samples live in physical-resource-report.json"
            .to_owned(),
    ));
    gates
}

pub fn evaluate_a1_aggregate(agg: &FinalMetrics) -> Vec<GateResult> {
    let integrity = RunIntegrity {
        records_delivered: 480,
        in_order: true,
        wall_clock_ns: 0,
        memory_bytes: Some(0),
        memory_state: MemoryState::Measured,
    };
    let mut gates = evaluate_a1_run(agg, &integrity);
    gates.retain(|gate| {
        matches!(
            gate.id,
            "A1-P3" | "A1-P4" | "A1-P5" | "A1-P6" | "A1-P7" | "A1-P8" | "A1-P9" | "A1-P10"
        )
    });
    gates
}

pub fn classify_a1(
    per_run: &[Vec<GateResult>],
    aggregate: &[GateResult],
    p2_pass: bool,
    leakage: bool,
    admitted: bool,
) -> (RunResultState, Option<FailureClassification>) {
    if leakage {
        return (
            RunResultState::InvalidatedByLeakage,
            Some(FailureClassification::FalsifiedByLeakage),
        );
    }
    if per_run.iter().any(|gates| gate_fails(gates, "A1-P1")) {
        return (
            RunResultState::NoResult,
            Some(FailureClassification::FalsifiedByProtocol),
        );
    }
    if !p2_pass {
        return (
            RunResultState::NoResult,
            Some(FailureClassification::FalsifiedByDeterminism),
        );
    }
    if per_run.iter().any(|gates| gate_fails(gates, "A1-P13")) {
        return (
            RunResultState::NoResult,
            Some(FailureClassification::FalsifiedByResourceLimit),
        );
    }
    if per_run.iter().any(|gates| gate_fails(gates, "A1-P12")) {
        return (
            RunResultState::NegativeA1NarrowResult,
            Some(FailureClassification::FalsifiedByBudget),
        );
    }
    if !admitted {
        return (
            RunResultState::NegativeA1NarrowResult,
            Some(FailureClassification::InsufficientEvidence),
        );
    }
    for (ids, classification) in [
        (
            &["A1-P3", "A1-P4", "A1-P5", "A1-P6"] as &[&str],
            FailureClassification::FalsifiedByBoundaryGeneralization,
        ),
        (
            &["A1-P7", "A1-P8"],
            FailureClassification::FalsifiedByStructuralTransfer,
        ),
        (
            &["A1-P9"],
            FailureClassification::FalsifiedByFalsePositiveControl,
        ),
        (
            &["A1-P10"],
            FailureClassification::FalsifiedByAmbiguityHonesty,
        ),
        (
            &["A1-P11"],
            FailureClassification::FalsifiedByEvidenceTraceability,
        ),
    ] {
        if aggregate
            .iter()
            .any(|gate| ids.contains(&gate.id) && !gate.passed)
        {
            return (RunResultState::NegativeA1NarrowResult, Some(classification));
        }
    }
    (RunResultState::ValidatedA1NarrowClaim, None)
}

fn gate_fails(gates: &[GateResult], id: &str) -> bool {
    gates.iter().any(|gate| gate.id == id && !gate.passed)
}

pub fn evaluate_a0_run(
    metrics: &FinalMetrics,
    integrity: &RunIntegrity,
    span_pairs_considered: u64,
) -> Vec<GateResult> {
    vec![
        gate(
            "A0-C1",
            integrity.records_delivered == 160 && integrity.in_order,
            integrity.records_delivered as u64,
            160,
            "same 160 logical records in order".to_owned(),
        ),
        gate(
            "A0-C2",
            span_pairs_considered == 0 && metrics.tp == 0 && metrics.fs == 0,
            span_pairs_considered,
            1,
            "no segmentation work and empty boundary sets".to_owned(),
        ),
        gate(
            "A0-C4",
            metrics.late_refs == 0
                && metrics.orphan_refs == 0
                && integrity.wall_clock_ns <= MAX_WALL_CLOCK_NS
                && integrity.memory_state == MemoryState::Measured
                && integrity
                    .memory_bytes
                    .map(|bytes| bytes <= MAX_MEMORY_BYTES)
                    .unwrap_or(false),
            if integrity.memory_state == MemoryState::Measured {
                1
            } else {
                0
            },
            1,
            "zero leakage and physical ceilings".to_owned(),
        ),
    ]
}

pub fn evaluate_a3_run(
    metrics: &FinalMetrics,
    integrity: &RunIntegrity,
    library: &LibraryReport,
) -> Vec<GateResult> {
    let mut gates = evaluate_a1_run(metrics, integrity);
    gates.push(gate(
        "A3-C1",
        library.promoted
            && library.entries.len() == 1
            && library.entries[0].uses.len() >= 6
            && library.entries[0].records.len() >= 4
            && library.entries[0].payload_lengths.len() >= 3,
        library
            .entries
            .iter()
            .map(|entry| entry.uses.len() as u64)
            .sum(),
        6,
        "library admission with use/record/length/layout coverage".to_owned(),
    ));
    gates.push(gate(
        "A3-C2",
        library.utility_passes(),
        library.numerator,
        library.denominator,
        format!(
            "utility {}/{} removed {}",
            library.numerator, library.denominator, library.repeated_expansions_removed
        ),
    ));
    let provenance = library.entries.iter().all(|entry| {
        entry
            .uses
            .iter()
            .all(|(ordinal, _)| *ordinal >= 1 && *ordinal <= 64)
    });
    gates.push(gate(
        "A3-C3",
        provenance,
        if provenance { 1 } else { 0 },
        1,
        "every expansion resolves to concrete training supports".to_owned(),
    ));
    gates.push(gate(
        "A3-C5",
        library.entries.len() <= 32,
        library.entries.len() as u64,
        32,
        "library boundedness".to_owned(),
    ));
    gates
}

pub fn permutation_parity(primary: &FinalMetrics, permuted: &FinalMetrics) -> (bool, String) {
    let precision_rate = |metrics: &FinalMetrics| -> Option<(u64, u64)> {
        let den = metrics.tp + metrics.fs;
        metrics
            .tp
            .checked_mul(1_000_000)?
            .checked_div(den)
            .map(|rate| (rate, den))
    };
    let recall_rate = |metrics: &FinalMetrics| -> Option<(u64, u64)> {
        let den = metrics.tp + metrics.fm;
        metrics
            .tp
            .checked_mul(1_000_000)?
            .checked_div(den)
            .map(|rate| (rate, den))
    };
    let within = |first: Option<(u64, u64)>, second: Option<(u64, u64)>| -> bool {
        match (first, second) {
            (Some((a, _)), Some((b, _))) => a.abs_diff(b) <= 50_000,
            _ => false,
        }
    };
    let transfer_diff =
        (primary.l_success + primary.m_success).abs_diff(permuted.l_success + permuted.m_success);
    let parity = within(precision_rate(primary), precision_rate(permuted))
        && within(recall_rate(primary), recall_rate(permuted))
        && transfer_diff <= 2
        && primary.n_fp == 0
        && permuted.n_fp == 0
        && primary.a_honest >= 10
        && permuted.a_honest >= 10;
    (
        parity,
        format!(
            "transfer_diff {transfer_diff}/28 n_fp {}/{} honest {}/{}",
            primary.n_fp, permuted.n_fp, primary.a_honest, permuted.a_honest
        ),
    )
}
