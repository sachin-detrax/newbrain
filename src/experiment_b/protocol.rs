//! Learner-facing protocol values and small standard-library-only utilities for
//! Architecture Experiment B.
//!
//! This module deliberately contains no generator semantics, latent transition
//! tables, oracle plans, row schedule gold, or evaluator scoring.  The public
//! event and output types are the complete input/output boundary of the
//! Experiment B learner.

use std::fmt;

pub const SPECIFICATION_VERSION: &str =
    "architecture-experiment-b-transition-model-and-goal-directed-planning/1.0.0";
pub const PROTOCOL_VERSION: &str =
    "transition-model-and-goal-directed-planning-experiment-b-protocol-v1.0";
pub const GENERATOR_VERSION: &str = "goal-directed-planning-generator-b-v1";
pub const GRAMMAR_VERSION: &str = "goal-directed-planning-grammar-b-v1";

pub const TRAINING_EPISODES: usize = 96;
pub const DEVELOPMENT_EPISODES: usize = 48;
pub const FINAL_EPISODES: usize = 96;
pub const TOTAL_EPISODES: usize = 240;

pub const FINAL_H4_EPISODES: usize = 16;
pub const FINAL_H8_EPISODES: usize = 16;
pub const FINAL_H16_EPISODES: usize = 16;
pub const FINAL_COMPOSITION_EPISODES: usize = 12;
pub const FINAL_STOCHASTIC_EPISODES: usize = 12;
pub const FINAL_DEAD_END_EPISODES: usize = 12;
pub const FINAL_REPLAN_EPISODES: usize = 12;

pub const PRIMARY_WORLD_SEED: u64 = 2_026_101_203;
pub const PRIMARY_SCOPE_SEED: u64 = 2_026_101_213;
pub const PRIMARY_BASELINE_SEED: u64 = 2_026_101_223;
pub const PRIMARY_RERUN_SEED: u64 = 2_026_101_233;
pub const PERMUTATION_WORLD_SEED: u64 = 2_026_101_204;
pub const PERMUTATION_SCOPE_SEED: u64 = 2_026_101_214;
pub const PERMUTATION_BASELINE_SEED: u64 = 2_026_101_224;
pub const PERMUTATION_RERUN_SEED: u64 = 2_026_101_234;

pub const TRAINING_PARTITION_TAG: u64 = 0x5452_4149_4e00_0002;
pub const DEVELOPMENT_PARTITION_TAG: u64 = 0x4445_5600_0000_0002;
pub const FINAL_PARTITION_TAG: u64 = 0x4649_4e00_0000_0002;
pub const ORDINAL_MULTIPLIER: u64 = 0xD1B5_4A32_D192_ED03;
pub const PERMUTATION_STATE_XOR: u64 = 0x94D0_49BB_1331_11EB;

pub const MAX_RECEIVED_OBSERVATIONS_PER_EPISODE: u64 = 64;
pub const MAX_RECEIVED_OBSERVATIONS_PER_RUN: u64 = 15_360;
pub const MAX_ACTIONS_ISSUED_PER_EPISODE: u64 = 16;
pub const MAX_ACTIONS_ISSUED_PER_RUN: u64 = 3_840;
pub const MAX_PLAN_NODES_PER_EPISODE: u64 = 1_024;
pub const MAX_PLAN_NODES_PER_RUN: u64 = 245_760;
pub const MAX_TRANSITION_HYPOTHESES_PER_EPISODE: u64 = 512;
pub const MAX_TRANSITION_HYPOTHESES_PER_RUN: u64 = 122_880;
pub const MAX_TRANSITION_COMPARISONS_PER_EPISODE: u64 = 2_048;
pub const MAX_TRANSITION_COMPARISONS_PER_RUN: u64 = 491_520;
pub const MAX_RETAINED_HYPOTHESES_PER_EPISODE: u64 = 64;
pub const MAX_RETAINED_HYPOTHESES_PER_RUN: u64 = 512;
pub const MAX_MPC_ROLLOUTS_PER_EPISODE: u64 = 256;
pub const MAX_MPC_ROLLOUTS_PER_RUN: u64 = 61_440;
pub const MAX_EVIDENCE_RECORDS_PER_EPISODE: u64 = 256;
pub const MAX_EVIDENCE_RECORDS_PER_RUN: u64 = 61_440;
pub const MAX_ARTIFACT_BYTES_PER_EPISODE: u64 = 2 * 1024 * 1024;
pub const MAX_ARTIFACT_BYTES_PER_RUN: u64 = 512 * 1024 * 1024;
pub const MAX_WALL_CLOCK_NS: u64 = 120_000_000_000;
pub const MAX_RESIDENT_MEMORY_BYTES: u64 = 1_073_741_824;

pub type StateId = u64;
pub type ActionId = u64;
pub type ScopeToken = u64;
pub type EvidenceId = u64;
pub type HypothesisId = u64;
pub type TransitionId = u64;
pub type ModelId = u64;
pub type PlanRef = u64;
pub type ProcedureRef = u64;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalStatus {
    NotTerminated,
    GoalSatisfied,
    HorizonExhausted,
    DeadEnd,
    AbortedByRunner,
}

impl TerminalStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotTerminated => "not_terminated",
            Self::GoalSatisfied => "goal_satisfied",
            Self::HorizonExhausted => "horizon_exhausted",
            Self::DeadEnd => "dead_end",
            Self::AbortedByRunner => "aborted_by_runner",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoalDescriptor {
    pub predicate_id: u64,
    pub target_state: StateId,
    pub budget_cost: u64,
}

impl GoalDescriptor {
    pub const fn new(predicate_id: u64, target_state: StateId, budget_cost: u64) -> Self {
        Self {
            predicate_id,
            target_state,
            budget_cost,
        }
    }
}

/// A learner-facing episode event.  The learner is intentionally not given a
/// row index, partition label, horizon class, latent trajectory, slip draw,
/// gold plan, or evaluator score.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EpisodeEvent {
    Observation {
        run_scope: ScopeToken,
        episode_scope: ScopeToken,
        step_index: u32,
        state: StateId,
        available_actions: Vec<ActionId>,
        action_costs: Vec<u64>,
        goal: GoalDescriptor,
        horizon_remaining: u16,
        previous_terminal: TerminalStatus,
    },
    ActionResult {
        run_scope: ScopeToken,
        episode_scope: ScopeToken,
        step_index: u32,
        resulting_state: StateId,
        cost_paid: u64,
        terminal: TerminalStatus,
    },
    End {
        run_scope: ScopeToken,
        episode_scope: ScopeToken,
        terminal: TerminalStatus,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleControl {
    BeginDevelopment,
    Freeze,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbstainReason {
    InsufficientEvidence,
    UnreachableGoal,
    UnsupportedByGrammar,
    SearchExhausted,
    ResourceLimit,
    ConflictingHypotheses,
}

impl AbstainReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InsufficientEvidence => "insufficient_evidence",
            Self::UnreachableGoal => "unreachable_goal",
            Self::UnsupportedByGrammar => "unsupported_by_grammar",
            Self::SearchExhausted => "search_exhausted",
            Self::ResourceLimit => "resource_limit",
            Self::ConflictingHypotheses => "conflicting_hypotheses",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfidenceClass {
    Deterministic,
    StochasticDeclared,
    Uncertain,
}

impl ConfidenceClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Deterministic => "deterministic",
            Self::StochasticDeclared => "stochastic_declared",
            Self::Uncertain => "uncertain",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceProvenance {
    Observed,
    DerivedSymbolic,
    Inferred,
    Imagined,
    ModelProposed,
    EvaluatorOnly,
}

impl EvidenceProvenance {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::DerivedSymbolic => "derived-symbolic",
            Self::Inferred => "inferred",
            Self::Imagined => "imagined",
            Self::ModelProposed => "model-proposed",
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
    Stale,
}

impl EvidenceStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Disputed => "disputed",
            Self::Ambiguous => "ambiguous",
            Self::Suspended => "suspended",
            Self::Incomplete => "incomplete",
            Self::Stale => "stale",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanStatus {
    Proposed,
    Executing,
    Replanning,
    Completed,
    Abandoned,
}

impl PlanStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Executing => "executing",
            Self::Replanning => "replanning",
            Self::Completed => "completed",
            Self::Abandoned => "abandoned",
        }
    }
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanStep {
    pub action: ActionId,
    pub predicted_next_state: StateId,
    pub declared_cost: u64,
    pub evidence_refs: Vec<EvidenceId>,
    pub hypothesis_refs: Vec<HypothesisId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanBranch {
    pub parent: Option<u64>,
    pub action: ActionId,
    pub predicted_next_state: StateId,
    pub cost: u64,
    pub uncertainty: ConfidenceClass,
    pub source: EvidenceProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionHypothesis {
    pub id: HypothesisId,
    pub signature: String,
    pub support: Vec<EvidenceId>,
    pub challenges: Vec<EvidenceId>,
    pub confidence_class: ConfidenceClass,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceRecord {
    pub id: EvidenceId,
    pub episode_scope: ScopeToken,
    pub provenance: EvidenceProvenance,
    pub status: EvidenceStatus,
    pub signature: String,
    pub dependencies: Vec<EvidenceId>,
    pub creation_order: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Plan {
    pub id: PlanRef,
    pub goal: GoalDescriptor,
    pub status: PlanStatus,
    pub model_id: ModelId,
    pub steps: Vec<PlanStep>,
    pub branches: Vec<PlanBranch>,
    pub evidence_refs: Vec<EvidenceId>,
    pub hypothesis_refs: Vec<HypothesisId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Act {
    pub action: ActionId,
    pub procedure_refs: Vec<ProcedureRef>,
    pub plan_refs: Vec<PlanRef>,
    pub evidence_refs: Vec<EvidenceId>,
    pub hypothesis_refs: Vec<HypothesisId>,
    pub optional_prediction: Option<StateId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Abstain {
    pub reason: AbstainReason,
    pub hypothesis_refs: Vec<HypothesisId>,
    pub evidence_refs: Vec<EvidenceId>,
    pub plan_refs: Vec<PlanRef>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlannerResponse {
    Act(Act),
    Abstain(Abstain),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannerDecision {
    pub episode_scope: ScopeToken,
    pub step_index: u32,
    pub response: PlannerResponse,
    pub grammar_version: String,
    pub model_fingerprint: String,
    pub evidence_refs: Vec<EvidenceId>,
    pub operation_cost: u64,
    pub sequence: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceKind {
    ReceivedObservations,
    ActionsIssued,
    PlanNodes,
    TransitionHypotheses,
    TransitionComparisons,
    RetainedHypotheses,
    MpcRollouts,
    EvidenceRecords,
    SerializedLearnerBytes,
}

impl ResourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReceivedObservations => "received_observations",
            Self::ActionsIssued => "actions_issued",
            Self::PlanNodes => "plan_nodes_expanded",
            Self::TransitionHypotheses => "transition_hypotheses_generated",
            Self::TransitionComparisons => "transition_comparisons",
            Self::RetainedHypotheses => "retained_hypotheses",
            Self::MpcRollouts => "mpc_rollouts",
            Self::EvidenceRecords => "evidence_records",
            Self::SerializedLearnerBytes => "serialized_learner_artifact_bytes",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::ReceivedObservations => 0,
            Self::ActionsIssued => 1,
            Self::PlanNodes => 2,
            Self::TransitionHypotheses => 3,
            Self::TransitionComparisons => 4,
            Self::RetainedHypotheses => 5,
            Self::MpcRollouts => 6,
            Self::EvidenceRecords => 7,
            Self::SerializedLearnerBytes => 8,
        }
    }

    pub const fn all() -> [Self; 9] {
        [
            Self::ReceivedObservations,
            Self::ActionsIssued,
            Self::PlanNodes,
            Self::TransitionHypotheses,
            Self::TransitionComparisons,
            Self::RetainedHypotheses,
            Self::MpcRollouts,
            Self::EvidenceRecords,
            Self::SerializedLearnerBytes,
        ]
    }

    const fn limits(self) -> (u64, u64) {
        match self {
            Self::ReceivedObservations => (
                MAX_RECEIVED_OBSERVATIONS_PER_EPISODE,
                MAX_RECEIVED_OBSERVATIONS_PER_RUN,
            ),
            Self::ActionsIssued => (
                MAX_ACTIONS_ISSUED_PER_EPISODE,
                MAX_ACTIONS_ISSUED_PER_RUN,
            ),
            Self::PlanNodes => (MAX_PLAN_NODES_PER_EPISODE, MAX_PLAN_NODES_PER_RUN),
            Self::TransitionHypotheses => (
                MAX_TRANSITION_HYPOTHESES_PER_EPISODE,
                MAX_TRANSITION_HYPOTHESES_PER_RUN,
            ),
            Self::TransitionComparisons => (
                MAX_TRANSITION_COMPARISONS_PER_EPISODE,
                MAX_TRANSITION_COMPARISONS_PER_RUN,
            ),
            Self::RetainedHypotheses => (
                MAX_RETAINED_HYPOTHESES_PER_EPISODE,
                MAX_RETAINED_HYPOTHESES_PER_RUN,
            ),
            Self::MpcRollouts => (MAX_MPC_ROLLOUTS_PER_EPISODE, MAX_MPC_ROLLOUTS_PER_RUN),
            Self::EvidenceRecords => (
                MAX_EVIDENCE_RECORDS_PER_EPISODE,
                MAX_EVIDENCE_RECORDS_PER_RUN,
            ),
            Self::SerializedLearnerBytes => (
                MAX_ARTIFACT_BYTES_PER_EPISODE,
                MAX_ARTIFACT_BYTES_PER_RUN,
            ),
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
    run: [u64; 9],
    episode: [u64; 9],
    peak: [u64; 9],
    exhausted: Option<BudgetError>,
}

impl Budget {
    pub fn begin_episode(&mut self) {
        self.episode = [0; 9];
    }

    pub fn charge(&mut self, resource: ResourceKind, amount: u64) -> Result<(), BudgetError> {
        let index = resource.index();
        let (episode_limit, run_limit) = resource.limits();
        let episode_used = self.episode[index];
        let run_used = self.run[index];
        if episode_used
            .checked_add(amount)
            .is_none_or(|next| next > episode_limit)
        {
            let error = BudgetError {
                resource,
                used: episode_used,
                requested: amount,
                limit: episode_limit,
                scope: "episode",
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
        self.episode[index] += amount;
        self.run[index] += amount;
        self.peak[index] = self.peak[index].max(self.episode[index]);
        Ok(())
    }

    pub fn set_gauge(&mut self, resource: ResourceKind, value: u64) -> Result<(), BudgetError> {
        let index = resource.index();
        let (episode_limit, run_limit) = resource.limits();
        if value > episode_limit {
            let error = BudgetError {
                resource,
                used: self.episode[index],
                requested: value,
                limit: episode_limit,
                scope: "episode",
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
        self.episode[index] = value;
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

    pub fn episode_value(&self, resource: ResourceKind) -> u64 {
        self.episode[resource.index()]
    }

    pub fn peak_value(&self, resource: ResourceKind) -> u64 {
        self.peak[resource.index()]
    }

    pub fn run_values(&self) -> [u64; 9] {
        self.run
    }

    pub fn peak_values(&self) -> [u64; 9] {
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

pub fn goal_descriptor_json(goal: &GoalDescriptor) -> String {
    json_object(vec![
        ("budget_cost", goal.budget_cost.to_string()),
        ("predicate_id", json_string(&goal.predicate_id.to_string())),
        (
            "target_state",
            json_string(&goal.target_state.to_string()),
        ),
    ])
}

pub fn plan_step_json(step: &PlanStep) -> String {
    json_object(vec![
        ("action", json_string(&step.action.to_string())),
        ("declared_cost", step.declared_cost.to_string()),
        (
            "evidence_refs",
            ids_json(step.evidence_refs.iter().copied()),
        ),
        (
            "hypothesis_refs",
            ids_json(step.hypothesis_refs.iter().copied()),
        ),
        (
            "predicted_next_state",
            json_string(&step.predicted_next_state.to_string()),
        ),
    ])
}

pub fn plan_branch_json(branch: &PlanBranch) -> String {
    let parent = match branch.parent {
        Some(id) => json_string(&id.to_string()),
        None => "null".to_owned(),
    };
    json_object(vec![
        ("action", json_string(&branch.action.to_string())),
        ("cost", branch.cost.to_string()),
        ("parent", parent),
        (
            "predicted_next_state",
            json_string(&branch.predicted_next_state.to_string()),
        ),
        ("source", json_string(branch.source.as_str())),
        ("uncertainty", json_string(branch.uncertainty.as_str())),
    ])
}

pub fn transition_hypothesis_json(hypothesis: &TransitionHypothesis) -> String {
    json_object(vec![
        (
            "challenges",
            ids_json(hypothesis.challenges.iter().copied()),
        ),
        (
            "confidence_class",
            json_string(hypothesis.confidence_class.as_str()),
        ),
        ("id", json_string(&hypothesis.id.to_string())),
        ("signature", json_string(&hypothesis.signature)),
        ("support", ids_json(hypothesis.support.iter().copied())),
    ])
}

pub fn evidence_record_json(record: &EvidenceRecord) -> String {
    json_object(vec![
        ("creation_order", record.creation_order.to_string()),
        (
            "dependencies",
            ids_json(record.dependencies.iter().copied()),
        ),
        (
            "episode_scope",
            json_string(&record.episode_scope.to_string()),
        ),
        ("id", json_string(&record.id.to_string())),
        ("provenance", json_string(record.provenance.as_str())),
        ("signature", json_string(&record.signature)),
        ("status", json_string(record.status.as_str())),
    ])
}

pub fn plan_json(plan: &Plan) -> String {
    json_object(vec![
        (
            "branches",
            json_array(plan.branches.iter().map(plan_branch_json)),
        ),
        (
            "evidence_refs",
            ids_json(plan.evidence_refs.iter().copied()),
        ),
        ("goal", goal_descriptor_json(&plan.goal)),
        (
            "hypothesis_refs",
            ids_json(plan.hypothesis_refs.iter().copied()),
        ),
        ("id", json_string(&plan.id.to_string())),
        ("model_id", json_string(&plan.model_id.to_string())),
        ("status", json_string(plan.status.as_str())),
        ("steps", json_array(plan.steps.iter().map(plan_step_json))),
    ])
}

pub fn act_json(act: &Act) -> String {
    let prediction = match act.optional_prediction {
        Some(state) => json_string(&state.to_string()),
        None => "null".to_owned(),
    };
    json_object(vec![
        ("action", json_string(&act.action.to_string())),
        (
            "evidence_refs",
            ids_json(act.evidence_refs.iter().copied()),
        ),
        (
            "hypothesis_refs",
            ids_json(act.hypothesis_refs.iter().copied()),
        ),
        ("optional_prediction", prediction),
        ("plan_refs", ids_json(act.plan_refs.iter().copied())),
        (
            "procedure_refs",
            ids_json(act.procedure_refs.iter().copied()),
        ),
    ])
}

pub fn abstain_json(abstain: &Abstain) -> String {
    json_object(vec![
        (
            "evidence_refs",
            ids_json(abstain.evidence_refs.iter().copied()),
        ),
        (
            "hypothesis_refs",
            ids_json(abstain.hypothesis_refs.iter().copied()),
        ),
        ("plan_refs", ids_json(abstain.plan_refs.iter().copied())),
        ("reason", json_string(abstain.reason.as_str())),
    ])
}

pub fn planner_response_json(response: &PlannerResponse) -> String {
    match response {
        PlannerResponse::Act(act) => json_object(vec![
            ("act", act_json(act)),
            ("type", json_string("act")),
        ]),
        PlannerResponse::Abstain(abstain) => json_object(vec![
            ("abstain", abstain_json(abstain)),
            ("type", json_string("abstain")),
        ]),
    }
}

pub fn planner_decision_json(decision: &PlannerDecision) -> String {
    json_object(vec![
        (
            "episode_scope",
            json_string(&decision.episode_scope.to_string()),
        ),
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
        ("response", planner_response_json(&decision.response)),
        ("sequence", decision.sequence.to_string()),
        ("step_index", decision.step_index.to_string()),
    ])
}
