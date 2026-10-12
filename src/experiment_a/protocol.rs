//! Learner-facing protocol values and small standard-library-only utilities for
//! Architecture Experiment A.
//!
//! This module deliberately contains no generator labels, row schedule, gold
//! structure, or evaluator scoring.  The public event types are the complete
//! input boundary of the Experiment A learner.

use std::fmt;

pub const SPECIFICATION_VERSION: &str =
    "architecture-experiment-a-variable-length-boundary-discovery/1.0.0";
pub const PROTOCOL_VERSION: &str = "variable-length-boundary-discovery-experiment-a-protocol-v1.0";
pub const GENERATOR_VERSION: &str = "variable-stream-generator-a-v1";
pub const GRAMMAR_VERSION: &str = "variable-stream-structure-a-v1";

pub const TRAINING_RECORDS: usize = 64;
pub const DEVELOPMENT_RECORDS: usize = 32;
pub const FINAL_RECORDS: usize = 64;
pub const TOTAL_RECORDS: usize = 160;
pub const UNAMBIGUOUS_FINAL_RECORDS: usize = 52;
pub const AMBIGUOUS_FINAL_RECORDS: usize = 12;

pub const PRIMARY_WORLD_SEED: u64 = 2_026_101_201;
pub const PRIMARY_SCOPE_SEED: u64 = 2_026_101_211;
pub const PRIMARY_BASELINE_SEED: u64 = 2_026_101_221;
pub const PRIMARY_RERUN_SEED: u64 = 2_026_101_231;
pub const PERMUTATION_WORLD_SEED: u64 = 2_026_101_202;
pub const PERMUTATION_SCOPE_SEED: u64 = 2_026_101_212;
pub const PERMUTATION_BASELINE_SEED: u64 = 2_026_101_222;
pub const PERMUTATION_RERUN_SEED: u64 = 2_026_101_232;

pub const TRAINING_PARTITION_TAG: u64 = 0x5452_4149_4e00_0001;
pub const DEVELOPMENT_PARTITION_TAG: u64 = 0x4445_5600_0000_0001;
pub const FINAL_PARTITION_TAG: u64 = 0x4649_4e00_0000_0001;
pub const ORDINAL_MULTIPLIER: u64 = 0xD1B5_4A32_D192_ED03;
pub const PERMUTATION_STATE_XOR: u64 = 0x94D0_49BB_1331_11EB;

pub const MAX_ATOMS_PER_STREAM: u64 = 48;
pub const MAX_ATOMS_PER_RUN: u64 = 7_680;
pub const MAX_SPAN_PAIRS_PER_STREAM: u64 = 1_024;
pub const MAX_SPAN_PAIRS_PER_RUN: u64 = 163_840;
pub const MAX_ALIGNMENT_OPS_PER_STREAM: u64 = 4_096;
pub const MAX_ALIGNMENT_OPS_PER_RUN: u64 = 655_360;
pub const MAX_SEGMENTATION_HYPOTHESES_PER_STREAM: u64 = 64;
pub const MAX_SEGMENTATION_HYPOTHESES_PER_RUN: u64 = 512;
pub const MAX_RELATION_CANDIDATES_PER_STREAM: u64 = 512;
pub const MAX_RELATION_CANDIDATES_PER_RUN: u64 = 81_920;
pub const MAX_RELATION_COMPARISONS_PER_STREAM: u64 = 2_048;
pub const MAX_RELATION_COMPARISONS_PER_RUN: u64 = 327_680;
pub const MAX_ACTIVE_HYPOTHESES_PER_STREAM: u64 = 16;
pub const MAX_ACTIVE_HYPOTHESES_PER_RUN: u64 = 128;
pub const MAX_LIBRARY_ENTRIES_PER_STREAM: u64 = 8;
pub const MAX_LIBRARY_ENTRIES_PER_RUN: u64 = 32;
pub const MAX_EVIDENCE_RECORDS_PER_STREAM: u64 = 256;
pub const MAX_EVIDENCE_RECORDS_PER_RUN: u64 = 40_960;
pub const MAX_ARTIFACT_BYTES_PER_STREAM: u64 = 2 * 1024 * 1024;
pub const MAX_ARTIFACT_BYTES_PER_RUN: u64 = 64 * 1024 * 1024;
pub const MAX_WALL_CLOCK_NS: u64 = 120_000_000_000;
pub const MAX_RESIDENT_MEMORY_BYTES: u64 = 1_073_741_824;

pub type AtomId = u64;
pub type ScopeToken = u64;
pub type EvidenceId = u64;
pub type HypothesisId = u64;
pub type CandidateId = u64;
pub type RelationId = u64;
pub type LibraryId = u64;

/// The exact unsigned SplitMix64 transition frozen by the protocol.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub const fn state(self) -> u64 {
        self.state
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// A learner-facing opaque event.  The learner is intentionally not given a
/// length, ordinal, partition, row role, target, or boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamEvent {
    Token {
        run_scope: ScopeToken,
        stream_scope: ScopeToken,
        atom: AtomId,
    },
    End {
        run_scope: ScopeToken,
        stream_scope: ScopeToken,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleControl {
    BeginDevelopment,
    Freeze,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn len(self) -> usize {
        self.end - self.start
    }

    pub const fn is_empty(self) -> bool {
        self.start >= self.end
    }

    pub const fn is_valid(self, stream_len: usize) -> bool {
        self.start < self.end && self.end <= stream_len
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationUse {
    pub span: Span,
    pub relation_version: String,
    pub bindings: Vec<(usize, AtomId)>,
    pub evidence_refs: Vec<EvidenceId>,
    pub group_refs: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parse {
    pub spans: Vec<Span>,
    pub relation_uses: Vec<RelationUse>,
    pub boundaries: Vec<usize>,
    pub group_refs: Vec<String>,
    pub evidence_refs: Vec<EvidenceId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnalysisReason {
    InsufficientEvidence,
    AmbiguousSegmentation,
    UnsupportedByGrammar,
    SearchExhausted,
    ResourceLimit,
}

impl AnalysisReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InsufficientEvidence => "insufficient_evidence",
            Self::AmbiguousSegmentation => "ambiguous_segmentation",
            Self::UnsupportedByGrammar => "unsupported_by_grammar",
            Self::SearchExhausted => "search_exhausted",
            Self::ResourceLimit => "resource_limit",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnalysisResponse {
    Selected {
        parse: Parse,
    },
    Alternatives {
        parses: [Parse; 2],
        reason: AnalysisReason,
    },
    Abstain {
        reason: AnalysisReason,
        hypothesis_refs: Vec<HypothesisId>,
        evidence_refs: Vec<EvidenceId>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalysisDecision {
    pub stream_scope: ScopeToken,
    pub response: AnalysisResponse,
    pub grammar_version: String,
    pub model_fingerprint: String,
    pub evidence_refs: Vec<EvidenceId>,
    pub operation_cost: u64,
    pub sequence: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LearnerPhase {
    Training,
    Development,
    FrozenFinal,
}

impl LearnerPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Training => "training",
            Self::Development => "development",
            Self::FrozenFinal => "final",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceProvenance {
    Observed,
    DerivedSymbolic,
    Inferred,
    LibraryDerived,
    ModelProposed,
    Simulated,
    EvaluatorOnly,
}

impl EvidenceProvenance {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::DerivedSymbolic => "derived-symbolic",
            Self::Inferred => "inferred",
            Self::LibraryDerived => "library-derived",
            Self::ModelProposed => "model-proposed",
            Self::Simulated => "simulated",
            Self::EvaluatorOnly => "evaluator-only",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceStatus {
    Supported,
    Disputed,
    Ambiguous,
    Suspended,
    Incomplete,
}

impl EvidenceStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Disputed => "disputed",
            Self::Ambiguous => "ambiguous",
            Self::Suspended => "suspended",
            Self::Incomplete => "incomplete",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceRecord {
    pub id: EvidenceId,
    pub stream_scope: ScopeToken,
    pub provenance: EvidenceProvenance,
    pub status: EvidenceStatus,
    pub signature: String,
    pub dependencies: Vec<EvidenceId>,
    pub creation_order: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceKind {
    ReceivedAtoms,
    SpanBoundaryPairs,
    AlignmentOperations,
    SegmentationHypotheses,
    RelationCandidates,
    RelationComparisons,
    ActiveHypotheses,
    LibraryEntries,
    EvidenceRecords,
    SerializedLearnerBytes,
}

impl ResourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReceivedAtoms => "received_atoms",
            Self::SpanBoundaryPairs => "span_boundary_pairs_considered",
            Self::AlignmentOperations => "alignment_operations",
            Self::SegmentationHypotheses => "segmentation_hypotheses_retained",
            Self::RelationCandidates => "relation_candidates_generated",
            Self::RelationComparisons => "relation_comparisons",
            Self::ActiveHypotheses => "active_hypotheses",
            Self::LibraryEntries => "library_entries",
            Self::EvidenceRecords => "evidence_records",
            Self::SerializedLearnerBytes => "serialized_learner_artifact_bytes",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::ReceivedAtoms => 0,
            Self::SpanBoundaryPairs => 1,
            Self::AlignmentOperations => 2,
            Self::SegmentationHypotheses => 3,
            Self::RelationCandidates => 4,
            Self::RelationComparisons => 5,
            Self::ActiveHypotheses => 6,
            Self::LibraryEntries => 7,
            Self::EvidenceRecords => 8,
            Self::SerializedLearnerBytes => 9,
        }
    }

    pub const fn all() -> [Self; 10] {
        [
            Self::ReceivedAtoms,
            Self::SpanBoundaryPairs,
            Self::AlignmentOperations,
            Self::SegmentationHypotheses,
            Self::RelationCandidates,
            Self::RelationComparisons,
            Self::ActiveHypotheses,
            Self::LibraryEntries,
            Self::EvidenceRecords,
            Self::SerializedLearnerBytes,
        ]
    }

    const fn limits(self) -> (u64, u64) {
        match self {
            Self::ReceivedAtoms => (MAX_ATOMS_PER_STREAM, MAX_ATOMS_PER_RUN),
            Self::SpanBoundaryPairs => (MAX_SPAN_PAIRS_PER_STREAM, MAX_SPAN_PAIRS_PER_RUN),
            Self::AlignmentOperations => (MAX_ALIGNMENT_OPS_PER_STREAM, MAX_ALIGNMENT_OPS_PER_RUN),
            Self::SegmentationHypotheses => (
                MAX_SEGMENTATION_HYPOTHESES_PER_STREAM,
                MAX_SEGMENTATION_HYPOTHESES_PER_RUN,
            ),
            Self::RelationCandidates => (
                MAX_RELATION_CANDIDATES_PER_STREAM,
                MAX_RELATION_CANDIDATES_PER_RUN,
            ),
            Self::RelationComparisons => (
                MAX_RELATION_COMPARISONS_PER_STREAM,
                MAX_RELATION_COMPARISONS_PER_RUN,
            ),
            Self::ActiveHypotheses => (
                MAX_ACTIVE_HYPOTHESES_PER_STREAM,
                MAX_ACTIVE_HYPOTHESES_PER_RUN,
            ),
            Self::LibraryEntries => (MAX_LIBRARY_ENTRIES_PER_STREAM, MAX_LIBRARY_ENTRIES_PER_RUN),
            Self::EvidenceRecords => (
                MAX_EVIDENCE_RECORDS_PER_STREAM,
                MAX_EVIDENCE_RECORDS_PER_RUN,
            ),
            Self::SerializedLearnerBytes => {
                (MAX_ARTIFACT_BYTES_PER_STREAM, MAX_ARTIFACT_BYTES_PER_RUN)
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BudgetError {
    pub resource: ResourceKind,
    pub used: u64,
    pub requested: u64,
    pub limit: u64,
    pub scope: &'static str,
}

impl fmt::Display for BudgetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} budget would cross {} ceiling: used {}, requested {}, limit {}",
            self.resource.as_str(),
            self.scope,
            self.used,
            self.requested,
            self.limit
        )
    }
}

impl std::error::Error for BudgetError {}

/// Atomic preflight accounting.  A failed charge leaves all counters unchanged.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Budget {
    run: [u64; 10],
    stream: [u64; 10],
    peak: [u64; 10],
    exhausted: Option<BudgetError>,
}

impl Budget {
    pub fn begin_stream(&mut self) {
        self.stream = [0; 10];
    }

    pub fn charge(&mut self, resource: ResourceKind, amount: u64) -> Result<(), BudgetError> {
        let index = resource.index();
        let (stream_limit, run_limit) = resource.limits();
        let stream_used = self.stream[index];
        let run_used = self.run[index];
        if stream_used
            .checked_add(amount)
            .is_none_or(|next| next > stream_limit)
        {
            let error = BudgetError {
                resource,
                used: stream_used,
                requested: amount,
                limit: stream_limit,
                scope: "stream",
            };
            self.exhausted = Some(error.clone());
            return Err(error);
        }
        if run_used
            .checked_add(amount)
            .is_none_or(|next| next > run_limit)
        {
            let error = BudgetError {
                resource,
                used: run_used,
                requested: amount,
                limit: run_limit,
                scope: "run",
            };
            self.exhausted = Some(error.clone());
            return Err(error);
        }
        self.stream[index] += amount;
        self.run[index] += amount;
        self.peak[index] = self.peak[index].max(self.stream[index]);
        Ok(())
    }

    pub fn set_gauge(&mut self, resource: ResourceKind, value: u64) -> Result<(), BudgetError> {
        let index = resource.index();
        let (stream_limit, run_limit) = resource.limits();
        if value > stream_limit {
            let error = BudgetError {
                resource,
                used: self.stream[index],
                requested: value,
                limit: stream_limit,
                scope: "stream",
            };
            self.exhausted = Some(error.clone());
            return Err(error);
        }
        if value > run_limit {
            let error = BudgetError {
                resource,
                used: self.run[index],
                requested: value,
                limit: run_limit,
                scope: "run",
            };
            self.exhausted = Some(error.clone());
            return Err(error);
        }
        self.stream[index] = value;
        self.peak[index] = self.peak[index].max(value);
        self.run[index] = self.run[index].max(value);
        Ok(())
    }

    pub fn exhausted(&self) -> Option<&BudgetError> {
        self.exhausted.as_ref()
    }

    pub fn run_value(&self, resource: ResourceKind) -> u64 {
        self.run[resource.index()]
    }

    pub fn stream_value(&self, resource: ResourceKind) -> u64 {
        self.stream[resource.index()]
    }

    pub fn peak_value(&self, resource: ResourceKind) -> u64 {
        self.peak[resource.index()]
    }

    pub fn run_values(&self) -> [u64; 10] {
        self.run
    }

    pub fn peak_values(&self) -> [u64; 10] {
        self.peak
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Metric {
    pub numerator: u64,
    pub denominator: u64,
    pub rate_micros: u64,
}

impl Metric {
    pub fn new(numerator: u64, denominator: u64) -> Result<Self, String> {
        if denominator == 0 {
            return Err("metric denominator must be nonzero".to_owned());
        }
        Ok(Self {
            numerator,
            denominator,
            rate_micros: ((u128::from(numerator) * 1_000_000) / u128::from(denominator)) as u64,
        })
    }

    pub fn passed_at_least(self, numerator: u64, denominator: u64) -> bool {
        u128::from(self.numerator) * u128::from(denominator)
            >= u128::from(numerator) * u128::from(self.denominator)
    }

    pub fn to_json(&self) -> String {
        json_object(vec![
            ("denominator", self.denominator.to_string()),
            ("numerator", self.numerator.to_string()),
            ("rate_micros", self.rate_micros.to_string()),
        ])
    }
}

pub fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

pub fn json_array(values: impl IntoIterator<Item = String>) -> String {
    format!("[{}]", values.into_iter().collect::<Vec<_>>().join(","))
}

pub fn json_object(mut fields: Vec<(&str, String)>) -> String {
    fields.sort_unstable_by_key(|(key, _)| *key);
    format!(
        "{{{}}}",
        fields
            .into_iter()
            .map(|(key, value)| format!("{}:{value}", json_string(key)))
            .collect::<Vec<_>>()
            .join(",")
    )
}

pub fn ids_json(values: impl IntoIterator<Item = u64>) -> String {
    json_array(
        values
            .into_iter()
            .map(|value| json_string(&value.to_string())),
    )
}

pub fn spans_json(values: &[Span]) -> String {
    json_array(values.iter().map(|span| {
        json_object(vec![
            ("end", span.end.to_string()),
            ("start", span.start.to_string()),
        ])
    }))
}

pub fn parse_signature(parse: &Parse) -> String {
    let spans = parse
        .spans
        .iter()
        .map(|span| format!("{}:{}", span.start, span.end))
        .collect::<Vec<_>>()
        .join(";");
    let relations = parse
        .relation_uses
        .iter()
        .map(|relation| format!("{}:{}", relation.span.start, relation.span.end))
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "spans={spans}|relations={relations}|boundaries={:?}",
        parse.boundaries
    )
}

/// A compact, dependency-free SHA-256 implementation used for model and
/// artifact fingerprints.  It follows FIPS 180-4's 512-bit block schedule.
pub fn sha256_hex(bytes: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];
    let mut h = [
        0x6a09_e667_u32,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    let bit_len = (bytes.len() as u128 * 8) as u64;
    let mut padded = bytes.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());
    for block in padded.chunks_exact(64) {
        let mut schedule = [0_u32; 64];
        for (index, word) in schedule[..16].iter_mut().enumerate() {
            let start = index * 4;
            *word = u32::from_be_bytes([
                block[start],
                block[start + 1],
                block[start + 2],
                block[start + 3],
            ]);
        }
        for index in 16..64 {
            let s0 = schedule[index - 15].rotate_right(7)
                ^ schedule[index - 15].rotate_right(18)
                ^ (schedule[index - 15] >> 3);
            let s1 = schedule[index - 2].rotate_right(17)
                ^ schedule[index - 2].rotate_right(19)
                ^ (schedule[index - 2] >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(s0)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(s1);
        }
        let mut state = h;
        for index in 0..64 {
            let s1 =
                state[4].rotate_right(6) ^ state[4].rotate_right(11) ^ state[4].rotate_right(25);
            let choice = (state[4] & state[5]) ^ ((!state[4]) & state[6]);
            let temp1 = state[7]
                .wrapping_add(s1)
                .wrapping_add(choice)
                .wrapping_add(K[index])
                .wrapping_add(schedule[index]);
            let s0 =
                state[0].rotate_right(2) ^ state[0].rotate_right(13) ^ state[0].rotate_right(22);
            let majority = (state[0] & state[1]) ^ (state[0] & state[2]) ^ (state[1] & state[2]);
            let temp2 = s0.wrapping_add(majority);
            state = [
                temp1.wrapping_add(temp2),
                state[0],
                state[1],
                state[2],
                state[3].wrapping_add(temp1),
                state[4],
                state[5],
                state[6],
            ];
        }
        for index in 0..8 {
            h[index] = h[index].wrapping_add(state[index]);
        }
    }
    h.iter().map(|word| format!("{word:08x}")).collect()
}

pub fn analysis_response_json(response: &AnalysisResponse) -> String {
    match response {
        AnalysisResponse::Selected { parse } => json_object(vec![
            ("parse", parse_json(parse)),
            ("type", json_string("selected")),
        ]),
        AnalysisResponse::Alternatives { parses, reason } => json_object(vec![
            ("parses", json_array(parses.iter().map(parse_json))),
            ("reason", json_string(reason.as_str())),
            ("type", json_string("alternatives")),
        ]),
        AnalysisResponse::Abstain {
            reason,
            hypothesis_refs,
            evidence_refs,
        } => json_object(vec![
            ("evidence_refs", ids_json(evidence_refs.iter().copied())),
            ("hypothesis_refs", ids_json(hypothesis_refs.iter().copied())),
            ("reason", json_string(reason.as_str())),
            ("type", json_string("abstain")),
        ]),
    }
}

pub fn relation_use_json(relation: &RelationUse) -> String {
    json_object(vec![
        (
            "bindings",
            json_array(relation.bindings.iter().map(|(position, atom)| {
                json_object(vec![
                    ("atom", json_string(&atom.to_string())),
                    ("position", position.to_string()),
                ])
            })),
        ),
        (
            "evidence_refs",
            ids_json(relation.evidence_refs.iter().copied()),
        ),
        (
            "group_refs",
            json_array(relation.group_refs.iter().map(|value| json_string(value))),
        ),
        ("relation_version", json_string(&relation.relation_version)),
        (
            "span",
            json_object(vec![
                ("end", relation.span.end.to_string()),
                ("start", relation.span.start.to_string()),
            ]),
        ),
    ])
}

pub fn parse_json(parse: &Parse) -> String {
    json_object(vec![
        (
            "boundaries",
            json_array(parse.boundaries.iter().map(|v| v.to_string())),
        ),
        (
            "evidence_refs",
            ids_json(parse.evidence_refs.iter().copied()),
        ),
        (
            "group_refs",
            json_array(parse.group_refs.iter().map(|value| json_string(value))),
        ),
        (
            "relation_uses",
            json_array(parse.relation_uses.iter().map(relation_use_json)),
        ),
        ("spans", spans_json(&parse.spans)),
    ])
}

pub fn analysis_json(decision: &AnalysisDecision) -> String {
    json_object(vec![
        ("analysis", analysis_response_json(&decision.response)),
        (
            "evidence_refs",
            ids_json(decision.evidence_refs.iter().copied()),
        ),
        ("grammar_version", json_string(&decision.grammar_version)),
        (
            "model_fingerprint",
            json_string(&decision.model_fingerprint),
        ),
        ("operation_cost", decision.operation_cost.to_string()),
        ("sequence", decision.sequence.to_string()),
        (
            "stream_scope",
            json_string(&decision.stream_scope.to_string()),
        ),
    ])
}
