//! Deterministic v2 experiment orchestration and artifact packaging.
//!
//! This module is the evaluator-side runner.  The proposed learner and B0/B1
//! receive only [`StepInput`](super::opaque_protocol::StepInput) values and
//! return opaque decisions through the interaction ledger.  B2 is implemented
//! in a separate controller path in this module and is deliberately excluded
//! from learner state and learner resource totals.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::evaluator_world::{
    fixed_schedule, EpisodeRole, EvaluatorWorld, Partition, RunSeeds, RunVariant, SplitMix64,
    DEVELOPMENT_EPISODE_COUNT, EPISODE_COUNT, FINAL_EPISODE_COUNT, TRAINING_EPISODE_COUNT,
};
use super::grammar_admission::{
    AdmissionOutcome, Candidate, CandidateStage, GenerationReport, GenerationStatus, GrammarV2,
    ProcedureRecord, ProcedureRevision, ProcedureStatus, RejectionReason,
};
use super::grounding_decision::{
    DecisionReason, FrozenGroundingLearner, GroundingError, GroundingLearner, LogicalCost,
    RecordedDecision,
};
use super::interaction_ledger::{EvidenceProvenance, InteractionLedger, LedgerError, LedgerEvent};
use super::opaque_protocol::{
    AbstainReason, ActMode, ActionId, AuditReferences, Decision, EvidenceId, HypothesisId,
    ObservationPacket, PreviousTerminal, ScopeToken, StepInput, StepResult, TerminalStatus,
};
use super::resource_scale::{
    deterministic_scale_points, LogicalResource, MeasurementError, MemorySample, ResourceMeter,
    ResourceSnapshot, ScaleSweep, Stage, V2_GRAMMAR_VERSION, V2_PROTOCOL_VERSION,
};
use super::Evaluator;

pub const SPEC_VERSION: &str = "opaque-interactive-structural-discovery-v2-v2.0";
pub const SOURCE_FINGERPRINT: &str = "sparse-structural-rule-learner-v2-std-only";
pub const RERUN_COUNT: usize = 3;
pub const HELD_OUT_APPLICABLE_COUNT: u64 = 6;
pub const HELD_OUT_NON_APPLICABLE_COUNT: u64 = 6;
pub const FROZEN_RETENTION_COUNT: u64 = 4;
pub const MIN_STRUCTURAL_SUPPORT: u64 = 2;
pub const MIN_STRUCTURAL_CONTEXTS: u64 = 2;
pub const MIN_TRANSFER_COVERAGE: u64 = 5;
pub const MIN_TRANSFER_SUCCESS: u64 = 5;
pub const MIN_PACKET_ATTEMPTS: u64 = 5;
const TRAINING_GENERATION_RECORD_LIMIT: usize = 6;

/// The protocol is embedded only for artifact publication.  It is never read
/// by a learner driver during a run.
pub const FROZEN_PROTOCOL: &str = include_str!(
    "../../docs/protocols/opaque-interactive-structural-discovery-v2-protocol-v1.0.md"
);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Predictor {
    Proposed,
    B0ExactInteractionRetrieval,
    B1SeededRandomExploration,
    B2EvaluatorController,
}

impl Predictor {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::B0ExactInteractionRetrieval => "b0-exact-interaction-retrieval",
            Self::B1SeededRandomExploration => "b1-seeded-random-exploration",
            Self::B2EvaluatorController => "b2-evaluator-controller",
        }
    }

    pub const fn is_learner(self) -> bool {
        !matches!(self, Self::B2EvaluatorController)
    }

    pub const fn all() -> [Self; 4] {
        [
            Self::Proposed,
            Self::B0ExactInteractionRetrieval,
            Self::B1SeededRandomExploration,
            Self::B2EvaluatorController,
        ]
    }
}

pub type Variant = RunVariant;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureClassification {
    FalsifiedByLeakage,
    FalsifiedByDeterminism,
    FalsifiedByBudget,
    FalsifiedByResourceLimit,
    InsufficientEvidence,
    SearchExhausted,
    UnsupportedByGrammar,
    IncorrectHypothesis,
    FalsifiedByHeldOutGeneralization,
    FalsifiedByFalsePositiveControl,
    FalsifiedByAbstentionQuality,
    FalsifiedByRetention,
    FalsifiedByEvidenceTraceability,
    SecondaryRobustnessFailed,
}

impl FailureClassification {
    pub const fn name(self) -> &'static str {
        match self {
            Self::FalsifiedByLeakage => "falsified-by-leakage",
            Self::FalsifiedByDeterminism => "falsified-by-determinism",
            Self::FalsifiedByBudget => "falsified-by-budget",
            Self::FalsifiedByResourceLimit => "falsified-by-resource-limit",
            Self::InsufficientEvidence => "insufficient-evidence",
            Self::SearchExhausted => "search-exhausted",
            Self::UnsupportedByGrammar => "unsupported-by-grammar",
            Self::IncorrectHypothesis => "incorrect-hypothesis",
            Self::FalsifiedByHeldOutGeneralization => "falsified-by-held-out-generalization",
            Self::FalsifiedByFalsePositiveControl => "falsified-by-false-positive-control",
            Self::FalsifiedByAbstentionQuality => "falsified-by-abstention-quality",
            Self::FalsifiedByRetention => "falsified-by-retention",
            Self::FalsifiedByEvidenceTraceability => "falsified-by-evidence-traceability",
            Self::SecondaryRobustnessFailed => "secondary-robustness-failed",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailureRecord {
    pub subject: String,
    pub classification: FailureClassification,
    pub detail: String,
    pub evidence_ids: Vec<EvidenceId>,
}

impl FailureRecord {
    fn new(
        subject: impl Into<String>,
        classification: FailureClassification,
        detail: impl Into<String>,
        evidence_ids: Vec<EvidenceId>,
    ) -> Self {
        Self {
            subject: subject.into(),
            classification,
            detail: detail.into(),
            evidence_ids,
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"classification\":\"{}\",\"detail\":\"{}\",\"evidence_ids\":{},\"subject\":\"{}\"}}",
            self.classification.name(),
            json_escape(&self.detail),
            evidence_ids_json(&self.evidence_ids),
            json_escape(&self.subject),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rate {
    numerator: u64,
    denominator: u64,
    rate_micros: u64,
}

impl Rate {
    pub fn new(numerator: u64, denominator: u64) -> Result<Self, ExperimentError> {
        if denominator == 0 || numerator > denominator {
            return Err(ExperimentError::InvalidMetricDenominator {
                numerator,
                denominator,
            });
        }
        Ok(Self {
            numerator,
            denominator,
            rate_micros: ((1_000_000_u128 * numerator as u128) / denominator as u128) as u64,
        })
    }

    pub const fn numerator(self) -> u64 {
        self.numerator
    }

    pub const fn denominator(self) -> u64 {
        self.denominator
    }

    pub const fn rate_micros(self) -> u64 {
        self.rate_micros
    }

    fn to_json(self) -> String {
        format!(
            "{{\"denominator\":{},\"numerator\":{},\"rate_micros\":{}}}",
            self.denominator, self.numerator, self.rate_micros
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpisodeMetric {
    pub order: usize,
    pub label: String,
    pub partition: String,
    pub role: String,
    pub episode_scope: ScopeToken,
    pub decision_sequences: Vec<u64>,
    pub action_count: u64,
    pub terminal: TerminalStatus,
    pub task_success: bool,
    pub prediction_attempts: u64,
    pub exact_predictions: u64,
    pub procedure_backed_actions: u64,
    pub procedure_backed_predictions: u64,
    pub abstention_count: u64,
    pub incomplete_count: u64,
    pub false_positive: bool,
    pub retention_pass: bool,
    pub grounding_evidence_count: u64,
}

impl EpisodeMetric {
    fn to_json(&self) -> String {
        format!(
            "{{\"abstention_count\":{},\"action_count\":{},\"decision_sequences\":{},\"episode_scope\":\"{}\",\"exact_predictions\":{},\"false_positive\":{},\"grounding_evidence_count\":{},\"incomplete_count\":{},\"label\":\"{}\",\"order\":{},\"partition\":\"{}\",\"prediction_attempts\":{},\"procedure_backed_actions\":{},\"procedure_backed_predictions\":{},\"retention_pass\":{},\"role\":\"{}\",\"task_success\":{},\"terminal\":\"{}\"}}",
            self.abstention_count,
            self.action_count,
            u64_array_json(&self.decision_sequences),
            self.episode_scope,
            self.exact_predictions,
            self.false_positive,
            self.grounding_evidence_count,
            self.incomplete_count,
            json_escape(&self.label),
            self.order,
            json_escape(&self.partition),
            self.prediction_attempts,
            self.procedure_backed_actions,
            self.procedure_backed_predictions,
            self.retention_pass,
            json_escape(&self.role),
            self.task_success,
            terminal_name(self.terminal),
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DiscoveryMetrics {
    pub admitted_e3_procedures: u64,
    pub support_interactions: u64,
    pub support_context_atoms: u64,
    pub e0_only_credit: bool,
    pub grounding_evidence_count: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BudgetStatus {
    pub over_limit_charges: u64,
    pub unrecorded_charges: u64,
    pub exhaustion_events: u64,
    pub confident_after_exhaustion: u64,
}

impl BudgetStatus {
    fn honest(self) -> bool {
        self.over_limit_charges == 0
            && self.unrecorded_charges == 0
            && self.confident_after_exhaustion == 0
    }

    fn to_json(self) -> String {
        format!(
            "{{\"confident_after_exhaustion\":{},\"exhaustion_events\":{},\"over_limit_charges\":{},\"unrecorded_charges\":{}}}",
            self.confident_after_exhaustion,
            self.exhaustion_events,
            self.over_limit_charges,
            self.unrecorded_charges,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Metrics {
    pub held_out_applicable_prediction_coverage: Rate,
    pub held_out_applicable_packet_accuracy: Rate,
    pub held_out_applicable_task_success: Rate,
    pub held_out_non_applicable_false_positives: Rate,
    pub held_out_non_applicable_abstention_quality: Rate,
    pub frozen_retention: Rate,
    pub held_out_applicable_prediction_attempts: u64,
    pub prediction_attempts: u64,
    pub exact_predictions: u64,
    pub task_success_count: u64,
    pub abstention_count: u64,
    pub incomplete_count: u64,
    pub structural_procedure_actions: u64,
    pub structural_procedure_predictions: u64,
    pub grounding_evidence_count: u64,
    pub discovery: DiscoveryMetrics,
    pub episodes: Vec<EpisodeMetric>,
}

impl Metrics {
    fn from_episodes(
        episodes: Vec<EpisodeMetric>,
        discovery: DiscoveryMetrics,
    ) -> Result<Self, ExperimentError> {
        let applicable: Vec<_> = episodes
            .iter()
            .filter(|episode| episode.role == "held-out-applicable")
            .collect();
        let non_applicable: Vec<_> = episodes
            .iter()
            .filter(|episode| episode.role == "held-out-non-applicable")
            .collect();
        let retention: Vec<_> = episodes
            .iter()
            .filter(|episode| episode.role.starts_with("frozen-retention"))
            .collect();

        let coverage = applicable
            .iter()
            .filter(|episode| episode.procedure_backed_predictions > 0)
            .count() as u64;
        let task_success = applicable
            .iter()
            .filter(|episode| episode.task_success)
            .count() as u64;
        let attempts: u64 = applicable
            .iter()
            .map(|episode| episode.prediction_attempts)
            .sum();
        let exact = applicable
            .iter()
            .map(|episode| episode.exact_predictions)
            .sum();
        let false_positives = non_applicable
            .iter()
            .filter(|episode| episode.false_positive)
            .count() as u64;
        let abstention_quality = non_applicable
            .iter()
            .filter(|episode| !episode.false_positive && episode.procedure_backed_actions == 0)
            .count() as u64;
        let retention_pass = retention
            .iter()
            .filter(|episode| episode.retention_pass)
            .count() as u64;

        let mut prediction_attempts = 0;
        let mut exact_predictions = 0;
        let mut task_success_count = 0;
        let mut abstention_count = 0;
        let mut incomplete_count = 0;
        let mut procedure_actions = 0;
        let mut procedure_predictions = 0;
        let mut grounding_evidence_count = 0;
        for episode in &episodes {
            prediction_attempts += episode.prediction_attempts;
            exact_predictions += episode.exact_predictions;
            task_success_count += u64::from(episode.task_success);
            abstention_count += episode.abstention_count;
            incomplete_count += episode.incomplete_count;
            procedure_actions += episode.procedure_backed_actions;
            procedure_predictions += episode.procedure_backed_predictions;
            grounding_evidence_count += episode.grounding_evidence_count;
        }

        Ok(Self {
            held_out_applicable_prediction_coverage: Rate::new(
                coverage,
                HELD_OUT_APPLICABLE_COUNT,
            )?,
            held_out_applicable_packet_accuracy: Rate::new(
                attempts.min(exact),
                HELD_OUT_APPLICABLE_COUNT,
            )?,
            held_out_applicable_task_success: Rate::new(task_success, HELD_OUT_APPLICABLE_COUNT)?,
            held_out_non_applicable_false_positives: Rate::new(
                false_positives,
                HELD_OUT_NON_APPLICABLE_COUNT,
            )?,
            held_out_non_applicable_abstention_quality: Rate::new(
                abstention_quality,
                HELD_OUT_NON_APPLICABLE_COUNT,
            )?,
            frozen_retention: Rate::new(retention_pass, FROZEN_RETENTION_COUNT)?,
            held_out_applicable_prediction_attempts: attempts,
            prediction_attempts,
            exact_predictions,
            task_success_count,
            abstention_count,
            incomplete_count,
            structural_procedure_actions: procedure_actions,
            structural_procedure_predictions: procedure_predictions,
            grounding_evidence_count,
            discovery,
            episodes,
        })
    }

    pub fn to_json(&self) -> String {
        format!(
            "{{\"abstention_count\":{},\"discovery\":{},\"episodes\":{},\"exact_predictions\":{},\"frozen_retention\":{},\"grounding_evidence_count\":{},\"held_out_applicable_packet_accuracy\":{},\"held_out_applicable_prediction_attempts\":{},\"held_out_applicable_prediction_coverage\":{},\"held_out_applicable_task_success\":{},\"held_out_non_applicable_abstention_quality\":{},\"held_out_non_applicable_false_positives\":{},\"incomplete_count\":{},\"prediction_attempts\":{},\"structural_procedure_actions\":{},\"structural_procedure_predictions\":{},\"task_success_count\":{}}}",
            self.abstention_count,
            discovery_json(self.discovery),
            json_array(self.episodes.iter().map(EpisodeMetric::to_json)),
            self.exact_predictions,
            self.frozen_retention.to_json(),
            self.grounding_evidence_count,
            self.held_out_applicable_packet_accuracy.to_json(),
            self.held_out_applicable_prediction_attempts,
            self.held_out_applicable_prediction_coverage.to_json(),
            self.held_out_applicable_task_success.to_json(),
            self.held_out_non_applicable_abstention_quality.to_json(),
            self.held_out_non_applicable_false_positives.to_json(),
            self.incomplete_count,
            self.prediction_attempts,
            self.structural_procedure_actions,
            self.structural_procedure_predictions,
            self.task_success_count,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StageOperations {
    pub canonicalization: u64,
    pub retrieval: u64,
    pub admission: u64,
    pub revalidation: u64,
    pub candidate_generation: u64,
    pub decision_action: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicalResourceReport {
    pub included: bool,
    pub query_calls: u64,
    pub environment_actions: u64,
    pub candidate_generation_attempts: u64,
    pub structural_comparisons: u64,
    pub active_hypotheses: u64,
    pub operation_steps: u64,
    pub retained_interaction_records: u64,
    pub retained_procedure_records: u64,
    pub ledger_size_bytes: u64,
    pub stages: StageOperations,
}

impl LogicalResourceReport {
    fn excluded() -> Self {
        Self {
            included: false,
            query_calls: 0,
            environment_actions: 0,
            candidate_generation_attempts: 0,
            structural_comparisons: 0,
            active_hypotheses: 0,
            operation_steps: 0,
            retained_interaction_records: 0,
            retained_procedure_records: 0,
            ledger_size_bytes: 0,
            stages: StageOperations {
                canonicalization: 0,
                retrieval: 0,
                admission: 0,
                revalidation: 0,
                candidate_generation: 0,
                decision_action: 0,
            },
        }
    }

    fn from_meters(
        ledger: &ResourceMeter,
        grammar: Option<&ResourceMeter>,
        ledger_size_bytes: u64,
        interaction_count: u64,
    ) -> Self {
        let left = ledger.counters();
        let right = grammar.map(ResourceMeter::counters);
        let add = |a: u64, b: u64| a.saturating_add(b);
        let counter = |resource: LogicalResource| {
            add(
                left_value(left, resource),
                right.map(|value| left_value(value, resource)).unwrap_or(0),
            )
        };
        let stage = |stage: Stage| {
            ledger.stage(stage).operation_count().saturating_add(
                grammar
                    .map(|meter| meter.stage(stage).operation_count())
                    .unwrap_or(0),
            )
        };
        Self {
            included: true,
            query_calls: counter(LogicalResource::QueryCalls),
            environment_actions: counter(LogicalResource::EnvironmentActions),
            candidate_generation_attempts: counter(LogicalResource::CandidateGenerationAttempts),
            structural_comparisons: counter(LogicalResource::StructuralComparisons),
            active_hypotheses: left
                .active_hypotheses()
                .max(right.map(|value| value.active_hypotheses()).unwrap_or(0)),
            operation_steps: counter(LogicalResource::OperationSteps),
            retained_interaction_records: interaction_count,
            retained_procedure_records: right
                .map(|value| value.retained_procedure_records())
                .unwrap_or(0),
            ledger_size_bytes,
            stages: StageOperations {
                canonicalization: stage(Stage::Canonicalization),
                retrieval: stage(Stage::Retrieval),
                admission: stage(Stage::Admission),
                revalidation: stage(Stage::Revalidation),
                candidate_generation: stage(Stage::CandidateGeneration),
                decision_action: stage(Stage::DecisionAction),
            },
        }
    }

    pub fn to_json(&self) -> String {
        let stages = format!(
            "{{\"admission\":{},\"canonicalization\":{},\"candidate_generation\":{},\"decision_action\":{},\"revalidation\":{},\"retrieval\":{}}}",
            self.stages.admission,
            self.stages.canonicalization,
            self.stages.candidate_generation,
            self.stages.decision_action,
            self.stages.revalidation,
            self.stages.retrieval,
        );
        format!(
            "{{\"active_hypotheses\":{},\"candidate_generation_attempts\":{},\"environment_actions\":{},\"included\":{},\"ledger_record_count\":{},\"ledger_size_bytes\":{},\"operation_steps\":{},\"procedure_count\":{},\"query_calls\":{},\"retained_interaction_records\":{},\"retained_procedure_records\":{},\"stages\":{},\"structural_comparisons\":{}}}",
            self.active_hypotheses,
            self.candidate_generation_attempts,
            self.environment_actions,
            self.included,
            self.retained_interaction_records,
            self.ledger_size_bytes,
            self.operation_steps,
            self.retained_procedure_records,
            self.query_calls,
            self.retained_interaction_records,
            self.retained_procedure_records,
            stages,
            self.structural_comparisons,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicalResourceReport {
    pub included: bool,
    pub wall_clock_ns: u64,
    pub cpu_time_ns: u64,
    pub peak_process_memory_bytes: MemorySample,
    pub process_count: u64,
    pub worker_count: u64,
    pub gpu_count: u64,
    pub network_connection_count: u64,
    pub stage_operations: StageOperations,
}

impl PhysicalResourceReport {
    fn excluded() -> Self {
        Self {
            included: false,
            wall_clock_ns: 0,
            cpu_time_ns: 0,
            peak_process_memory_bytes: MemorySample::Missing,
            process_count: 0,
            worker_count: 0,
            gpu_count: 0,
            network_connection_count: 0,
            stage_operations: StageOperations {
                canonicalization: 0,
                retrieval: 0,
                admission: 0,
                revalidation: 0,
                candidate_generation: 0,
                decision_action: 0,
            },
        }
    }

    pub fn complete(&self) -> bool {
        self.included && !matches!(self.peak_process_memory_bytes, MemorySample::Missing)
    }

    pub fn to_json(&self) -> String {
        format!(
            "{{\"cpu_time_ns\":{},\"gpu_count\":{},\"included\":{},\"network_connection_count\":{},\"peak_process_memory_bytes\":{},\"process_count\":{},\"stage_operations\":{},\"wall_clock_ns\":{},\"worker_count\":{}}}",
            self.cpu_time_ns,
            self.gpu_count,
            self.included,
            self.network_connection_count,
            memory_value(self.peak_process_memory_bytes),
            self.process_count,
            stage_operations_json(self.stage_operations),
            self.wall_clock_ns,
            self.worker_count,
        )
    }

    fn deterministic_core_json(&self) -> String {
        format!(
            "{{\"included\":{},\"memory_state\":\"{}\",\"stage_operations\":{}}}",
            self.included,
            match self.peak_process_memory_bytes {
                MemorySample::Missing => "missing",
                MemorySample::Measured(_) => "measured",
            },
            stage_operations_json(self.stage_operations),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeakageStatus {
    pub prohibited_input_fields: u64,
    pub evaluator_calls_from_learner: u64,
    pub decision_before_result_violations: u64,
    pub final_result_selection_uses: u64,
    pub evaluator_only_controller_isolated: bool,
}

impl LeakageStatus {
    fn clean() -> Self {
        Self {
            prohibited_input_fields: 0,
            evaluator_calls_from_learner: 0,
            decision_before_result_violations: 0,
            final_result_selection_uses: 0,
            evaluator_only_controller_isolated: true,
        }
    }

    fn is_clean(&self) -> bool {
        self.prohibited_input_fields == 0
            && self.evaluator_calls_from_learner == 0
            && self.decision_before_result_violations == 0
            && self.final_result_selection_uses == 0
            && self.evaluator_only_controller_isolated
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"decision_before_result_violations\":{},\"evaluator_calls_from_learner\":{},\"evaluator_only_controller_isolated\":{},\"final_result_selection_uses\":{},\"prohibited_input_fields\":{}}}",
            self.decision_before_result_violations,
            self.evaluator_calls_from_learner,
            self.evaluator_only_controller_isolated,
            self.final_result_selection_uses,
            self.prohibited_input_fields,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionTraceRecord {
    pub sequence: u64,
    pub input: StepInput,
    pub decision: Decision,
    pub reason: String,
    pub competing_prediction_count: u64,
    pub disagreement_action_count: u64,
    pub cost: LogicalCost,
    pub decision_before_result: bool,
    pub result: Option<StepResult>,
}

impl DecisionTraceRecord {
    fn to_json(&self) -> String {
        format!(
            "{{\"competing_prediction_count\":{},\"cost\":{},\"decision\":{},\"decision_before_result\":{},\"disagreement_action_count\":{},\"input\":{},\"reason\":\"{}\",\"result\":{},\"sequence\":{}}}",
            self.competing_prediction_count,
            logical_cost_json(self.cost),
            decision_json(&self.decision),
            self.decision_before_result,
            self.disagreement_action_count,
            input_json(self.input),
            json_escape(&self.reason),
            self.result.map(step_result_json).unwrap_or_else(|| "null".to_owned()),
            self.sequence,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunReport {
    pub protocol_version: String,
    pub spec_version: String,
    pub grammar_version: String,
    pub variant: RunVariant,
    pub predictor: Predictor,
    pub rerun_index: usize,
    pub seeds: RunSeeds,
    pub schedule_fingerprint: String,
    pub run_scope: ScopeToken,
    pub decision_traces: Vec<DecisionTraceRecord>,
    pub interaction_ledger_jsonl: String,
    pub episodes: Vec<EpisodeMetric>,
    pub metrics: Metrics,
    pub logical_resources: LogicalResourceReport,
    pub scale_sweep: String,
    pub physical_resources: PhysicalResourceReport,
    pub leakage: LeakageStatus,
    pub budget: BudgetStatus,
    pub failures: Vec<FailureRecord>,
    pub candidate_events: Vec<String>,
    pub procedure_records: Vec<String>,
    pub frozen_state_unchanged: bool,
    pub freeze_boundary: usize,
    pub freeze_called: bool,
}

impl RunReport {
    pub fn deterministic_core_json(&self) -> String {
        format!(
            "{{\"budget\":{},\"candidate_events\":{},\"decision_traces\":{},\"failures\":{},\"freeze_boundary\":{},\"freeze_called\":{},\"frozen_state_unchanged\":{},\"grammar_version\":\"{}\",\"interaction_ledger_jsonl\":\"{}\",\"leakage\":{},\"logical_resources\":{},\"metrics\":{},\"physical_resources\":{},\"predictor\":\"{}\",\"procedure_records\":{},\"protocol_version\":\"{}\",\"run_scope\":\"{}\",\"scale_sweep\":{},\"schedule_fingerprint\":\"{}\",\"seeds\":{},\"source_fingerprint\":\"{}\",\"spec_version\":\"{}\",\"variant\":\"{}\"}}",
            self.budget.to_json(),
            json_array(self.candidate_events.iter().cloned()),
            json_array(self.decision_traces.iter().map(DecisionTraceRecord::to_json)),
            json_array(self.failures.iter().map(FailureRecord::to_json)),
            self.freeze_boundary,
            self.freeze_called,
            self.frozen_state_unchanged,
            self.grammar_version,
            json_escape(&self.interaction_ledger_jsonl),
            self.leakage.to_json(),
            self.logical_resources.to_json(),
            self.metrics.to_json(),
            self.physical_resources.deterministic_core_json(),
            self.predictor.name(),
            json_array(self.procedure_records.iter().cloned()),
            self.protocol_version,
            self.run_scope,
            self.scale_sweep,
            self.schedule_fingerprint,
            seeds_json(self.seeds),
            SOURCE_FINGERPRINT,
            self.spec_version,
            self.variant.name(),
        )
    }

    pub fn deterministic_core_fingerprint(&self) -> String {
        sha256_hex(self.deterministic_core_json().as_bytes())
    }

    /// Stable report serialization intentionally excludes monotonic wall-clock
    /// values. Those values are available from `physical_resource_report_json`
    /// and are not deterministic-core evidence.
    pub fn to_json(&self) -> String {
        self.deterministic_core_json()
    }

    pub fn physical_resource_report_json(&self) -> String {
        self.physical_resources.to_json()
    }

    pub fn decision_trace_jsonl(&self) -> String {
        jsonl(
            self.decision_traces
                .iter()
                .map(DecisionTraceRecord::to_json),
        )
    }

    pub fn interaction_ledger_jsonl(&self) -> &str {
        &self.interaction_ledger_jsonl
    }

    pub fn failures_jsonl(&self) -> String {
        jsonl(self.failures.iter().map(FailureRecord::to_json))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GateResult {
    pub id: String,
    pub passed: bool,
    pub classification: Option<FailureClassification>,
    pub detail: String,
}

impl GateResult {
    fn new(
        id: impl Into<String>,
        passed: bool,
        classification: Option<FailureClassification>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            passed,
            classification,
            detail: detail.into(),
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"classification\":{},\"detail\":\"{}\",\"id\":\"{}\",\"passed\":{}}}",
            self.classification
                .map(|value| format!("\"{}\"", value.name()))
                .unwrap_or_else(|| "null".to_owned()),
            json_escape(&self.detail),
            json_escape(&self.id),
            self.passed,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RerunComparison {
    pub variant: RunVariant,
    pub predictor: Predictor,
    pub expected_count: usize,
    pub present_count: usize,
    pub byte_identical: bool,
    pub core_fingerprints: Vec<String>,
}

impl RerunComparison {
    fn to_json(&self) -> String {
        format!(
            "{{\"byte_identical\":{},\"core_fingerprints\":{},\"expected_count\":{},\"present_count\":{},\"predictor\":\"{}\",\"variant\":\"{}\"}}",
            self.byte_identical,
            json_array(
                self.core_fingerprints
                    .iter()
                    .map(|fingerprint| format!("\"{}\"", json_escape(fingerprint))),
            ),
            self.expected_count,
            self.present_count,
            self.predictor.name(),
            self.variant.name(),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExperimentReport {
    pub protocol_version: String,
    pub spec_version: String,
    pub grammar_version: String,
    pub schedule_fingerprint: String,
    pub runs: Vec<RunReport>,
    pub rerun_comparisons: Vec<RerunComparison>,
    pub primary_gates: Vec<GateResult>,
    pub secondary_gates: Vec<GateResult>,
    pub failures: Vec<FailureRecord>,
}

impl ExperimentReport {
    pub fn primary_passed(&self) -> bool {
        self.primary_gates.iter().all(|gate| gate.passed)
    }

    pub fn secondary_passed(&self) -> bool {
        self.secondary_gates.iter().all(|gate| gate.passed)
    }

    pub fn validation_status(&self) -> &'static str {
        if self.primary_passed() {
            "validated-v2-narrow-claim"
        } else {
            "v2-validation-not-established"
        }
    }

    pub fn deterministic_json(&self) -> String {
        format!(
            "{{\"failures\":{},\"grammar_version\":\"{}\",\"primary_gates\":{},\"protocol_version\":\"{}\",\"rerun_comparisons\":{},\"runs\":{},\"schedule_fingerprint\":\"{}\",\"secondary_gates\":{},\"spec_version\":\"{}\",\"status\":\"{}\"}}",
            json_array(self.failures.iter().map(FailureRecord::to_json)),
            self.grammar_version,
            json_array(self.primary_gates.iter().map(GateResult::to_json)),
            self.protocol_version,
            json_array(self.rerun_comparisons.iter().map(RerunComparison::to_json)),
            json_array(self.runs.iter().map(|run| run.to_json())),
            self.schedule_fingerprint,
            json_array(self.secondary_gates.iter().map(GateResult::to_json)),
            self.spec_version,
            self.validation_status(),
        )
    }

    pub fn deterministic_fingerprint(&self) -> String {
        sha256_hex(self.deterministic_json().as_bytes())
    }
}

#[derive(Debug)]
pub enum ExperimentError {
    InvalidRerunIndex(usize),
    InvalidMetricDenominator { numerator: u64, denominator: u64 },
    Protocol(super::opaque_protocol::ProtocolError),
    Ledger(LedgerError),
    World(super::evaluator_world::WorldError),
    Grounding(GroundingError),
    Promotion(super::grammar_admission::PromotionError),
    Grammar(String),
    Measurement(MeasurementError),
    Io(io::Error),
    ScheduleMismatch { expected: usize, actual: usize },
}

impl fmt::Display for ExperimentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRerunIndex(index) => {
                write!(formatter, "rerun index must be 1..=3, got {index}")
            }
            Self::InvalidMetricDenominator {
                numerator,
                denominator,
            } => {
                write!(formatter, "invalid metric ratio {numerator}/{denominator}")
            }
            Self::Protocol(error) => error.fmt(formatter),
            Self::Ledger(error) => error.fmt(formatter),
            Self::World(error) => error.fmt(formatter),
            Self::Grounding(error) => error.fmt(formatter),
            Self::Promotion(error) => error.fmt(formatter),
            Self::Grammar(error) => write!(formatter, "v2 grammar: {error}"),
            Self::Measurement(error) => error.fmt(formatter),
            Self::Io(error) => error.fmt(formatter),
            Self::ScheduleMismatch { expected, actual } => {
                write!(
                    formatter,
                    "schedule row mismatch: expected {expected}, actual {actual}"
                )
            }
        }
    }
}

impl std::error::Error for ExperimentError {}

impl From<super::opaque_protocol::ProtocolError> for ExperimentError {
    fn from(error: super::opaque_protocol::ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl From<LedgerError> for ExperimentError {
    fn from(error: LedgerError) -> Self {
        Self::Ledger(error)
    }
}

impl From<super::evaluator_world::WorldError> for ExperimentError {
    fn from(error: super::evaluator_world::WorldError) -> Self {
        Self::World(error)
    }
}

impl From<GroundingError> for ExperimentError {
    fn from(error: GroundingError) -> Self {
        Self::Grounding(error)
    }
}

impl From<super::grammar_admission::PromotionError> for ExperimentError {
    fn from(error: super::grammar_admission::PromotionError) -> Self {
        Self::Promotion(error)
    }
}

impl From<MeasurementError> for ExperimentError {
    fn from(error: MeasurementError) -> Self {
        Self::Measurement(error)
    }
}

impl From<io::Error> for ExperimentError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Narrow public runner interface. `run_one` performs one fresh matched run;
/// `run_variant` performs all predictors and all three deterministic reruns;
/// `run_all` performs the primary and permutation extensions.
pub struct ExperimentRunner;

impl ExperimentRunner {
    pub fn run_one(
        variant: RunVariant,
        predictor: Predictor,
    ) -> Result<RunReport, ExperimentError> {
        run_one_rerun(variant, predictor, 1)
    }

    pub fn run_one_rerun(
        variant: RunVariant,
        predictor: Predictor,
        rerun_index: usize,
    ) -> Result<RunReport, ExperimentError> {
        run_one_rerun(variant, predictor, rerun_index)
    }

    pub fn run_variant(variant: RunVariant) -> Result<ExperimentReport, ExperimentError> {
        run_variants(&[variant])
    }

    pub fn run_all() -> Result<ExperimentReport, ExperimentError> {
        run_variants(&[RunVariant::Primary, RunVariant::SymbolActionPermutation])
    }
}

pub fn run_one(variant: RunVariant, predictor: Predictor) -> Result<RunReport, ExperimentError> {
    ExperimentRunner::run_one(variant, predictor)
}

pub fn run_one_rerun(
    variant: RunVariant,
    predictor: Predictor,
    rerun_index: usize,
) -> Result<RunReport, ExperimentError> {
    if !(1..=RERUN_COUNT).contains(&rerun_index) {
        return Err(ExperimentError::InvalidRerunIndex(rerun_index));
    }
    execute_run(variant, predictor, rerun_index)
}

pub fn run_variant(variant: RunVariant) -> Result<ExperimentReport, ExperimentError> {
    ExperimentRunner::run_variant(variant)
}

pub fn run_all() -> Result<ExperimentReport, ExperimentError> {
    ExperimentRunner::run_all()
}

pub fn compare_reruns(reports: &[RunReport]) -> RerunComparison {
    let (variant, predictor) = reports
        .first()
        .map(|report| (report.variant, report.predictor))
        .unwrap_or((RunVariant::Primary, Predictor::Proposed));
    let mut ordered = reports.to_vec();
    ordered.sort_by_key(|report| report.rerun_index);
    let fingerprints: Vec<_> = ordered
        .iter()
        .map(RunReport::deterministic_core_fingerprint)
        .collect();
    let byte_identical = ordered.len() == RERUN_COUNT
        && ordered
            .windows(2)
            .all(|pair| pair[0].deterministic_core_json() == pair[1].deterministic_core_json());
    RerunComparison {
        variant,
        predictor,
        expected_count: RERUN_COUNT,
        present_count: ordered.len(),
        byte_identical,
        core_fingerprints: fingerprints,
    }
}

fn run_variants(variants: &[RunVariant]) -> Result<ExperimentReport, ExperimentError> {
    let mut runs = Vec::new();
    let mut comparisons = Vec::new();
    for variant in variants {
        for predictor in Predictor::all() {
            let mut reruns = Vec::new();
            for rerun_index in 1..=RERUN_COUNT {
                reruns.push(run_one_rerun(*variant, predictor, rerun_index)?);
            }
            comparisons.push(compare_reruns(&reruns));
            runs.extend(reruns);
        }
    }
    let primary_runs = runs
        .iter()
        .filter(|run| run.variant == RunVariant::Primary)
        .cloned()
        .collect::<Vec<_>>();
    let secondary_runs = runs
        .iter()
        .filter(|run| run.variant == RunVariant::SymbolActionPermutation)
        .cloned()
        .collect::<Vec<_>>();
    let primary_gates = evaluate_primary(&primary_runs, &comparisons);
    let secondary_gates = evaluate_secondary(&primary_runs, &secondary_runs, &comparisons);
    let mut failures = Vec::new();
    failures.extend(gate_failures(&primary_gates));
    failures.extend(gate_failures(&secondary_gates));
    Ok(ExperimentReport {
        protocol_version: V2_PROTOCOL_VERSION.to_owned(),
        spec_version: SPEC_VERSION.to_owned(),
        grammar_version: V2_GRAMMAR_VERSION.to_owned(),
        schedule_fingerprint: schedule_fingerprint(),
        runs,
        rerun_comparisons: comparisons,
        primary_gates,
        secondary_gates,
        failures,
    })
}

fn execute_run(
    variant: RunVariant,
    predictor: Predictor,
    rerun_index: usize,
) -> Result<RunReport, ExperimentError> {
    let mut world = EvaluatorWorld::new(variant);
    let run_scope = world.run_scope();
    let mut ledger = InteractionLedger::new(run_scope)?;
    let mut driver = DriverState::new(predictor, run_scope)?;
    let mut episodes = Vec::new();
    let mut all_traces = Vec::new();

    for index in 0..EPISODE_COUNT {
        if world.next_episode_index() != index {
            return Err(ExperimentError::ScheduleMismatch {
                expected: index,
                actual: world.next_episode_index(),
            });
        }
        let spec = fixed_schedule()[index];
        let mut input = <EvaluatorWorld as Evaluator>::reset(&mut world, &ledger)?;
        ledger.record_stage_operation(Stage::Canonicalization, 1)?;
        let mut episode_decisions = Vec::new();
        loop {
            driver.set_variant_context(variant, &world);
            driver.before_decision(index, input, &mut ledger)?;
            ledger.record_stage_operation(Stage::Retrieval, driver.retrieval_operations())?;
            let (receipt, mut trace) = driver.decide(input, &mut ledger)?;
            let decision = receipt.decision().clone();
            let decision_was_recorded = matches!(ledger.events().last(), Some(LedgerEvent::Decision(record)) if record.sequence() == receipt.sequence());
            if !decision_was_recorded {
                driver.note_leakage_order_violation();
            }
            let result =
                <EvaluatorWorld as Evaluator>::apply_recorded(&mut world, &mut ledger, &receipt)?;
            ledger.record_stage_operation(Stage::Canonicalization, 1)?;
            if index >= TRAINING_EPISODE_COUNT + DEVELOPMENT_EPISODE_COUNT {
                ledger.record_stage_operation(Stage::Revalidation, 1)?;
                if index >= EPISODE_COUNT - FINAL_EPISODE_COUNT {
                    ledger.record_stage_operation(Stage::Revalidation, 1)?;
                }
            }
            trace.result = result;
            trace.decision_before_result = decision_was_recorded;
            let view = EpisodeDecision {
                sequence: receipt.sequence(),
                input,
                decision,
                result,
            };
            episode_decisions.push(view);
            driver.after_result(result, &mut ledger)?;
            all_traces.push(trace);
            match result {
                Some(value) if value.terminal() == TerminalStatus::Continue => {
                    input = world.next_input()?;
                    ledger.record_stage_operation(Stage::Canonicalization, 1)?;
                }
                _ => break,
            }
        }
        let terminal = episode_decisions
            .last()
            .and_then(|decision| decision.result.map(|result| result.terminal()))
            .unwrap_or(TerminalStatus::Incomplete);
        episodes.push(score_episode(spec, &episode_decisions, terminal));
        driver.after_episode(index, &mut ledger)?;
        if index == TRAINING_EPISODE_COUNT + DEVELOPMENT_EPISODE_COUNT - 1 {
            driver.freeze(&mut ledger)?;
        }
    }

    let ledger_jsonl = interaction_ledger_jsonl(&ledger);
    let ledger_snapshot = ledger.resource_meter_mut().sample_and_snapshot();
    let grammar_snapshot = driver.sample_resource_snapshot();
    let logical = if predictor.is_learner() {
        LogicalResourceReport::from_meters(
            ledger.resource_meter(),
            driver.grammar_meter(),
            ledger_jsonl.len() as u64,
            ledger.interaction_count() as u64,
        )
    } else {
        LogicalResourceReport::excluded()
    };
    let physical = if predictor.is_learner() {
        physical_from_snapshots(ledger_snapshot, grammar_snapshot)
    } else {
        PhysicalResourceReport::excluded()
    };
    let discovery = driver.discovery_metrics();
    let metrics = Metrics::from_episodes(episodes.clone(), discovery)?;
    let mut failures = driver.failures();
    if predictor.is_learner() && !physical.complete() {
        failures.push(FailureRecord::new(
            "resource-report",
            FailureClassification::FalsifiedByResourceLimit,
            "resident-memory measurement is missing; physical completeness is not claimed",
            Vec::new(),
        ));
    }
    if !driver.frozen_state_unchanged() {
        failures.push(FailureRecord::new(
            "freeze",
            FailureClassification::FalsifiedByLeakage,
            "frozen learner state changed during final evaluation",
            Vec::new(),
        ));
    }
    let budget = driver.budget_status();
    if !budget.honest() {
        failures.push(FailureRecord::new(
            "budget",
            FailureClassification::FalsifiedByBudget,
            "budget accounting contains an over-limit, unrecorded, or confident-after-exhaustion event",
            Vec::new(),
        ));
    }
    Ok(RunReport {
        protocol_version: V2_PROTOCOL_VERSION.to_owned(),
        spec_version: SPEC_VERSION.to_owned(),
        grammar_version: V2_GRAMMAR_VERSION.to_owned(),
        variant,
        predictor,
        rerun_index,
        seeds: RunSeeds::for_variant(variant),
        schedule_fingerprint: schedule_fingerprint(),
        run_scope,
        decision_traces: all_traces,
        interaction_ledger_jsonl: ledger_jsonl,
        episodes,
        metrics,
        logical_resources: logical,
        scale_sweep: canonical_scale_sweep_json(),
        physical_resources: physical,
        leakage: driver.leakage(),
        budget,
        failures,
        candidate_events: driver.candidate_events(),
        procedure_records: driver.procedure_records(),
        frozen_state_unchanged: driver.frozen_state_unchanged(),
        freeze_boundary: TRAINING_EPISODE_COUNT + DEVELOPMENT_EPISODE_COUNT,
        freeze_called: driver.freeze_called(),
    })
}

// The physical report is populated only after the real meters have taken their
// final snapshots. The ledger meter owns the run window; the grammar meter's
// stage samples are merged into the same learner-side report. B2 never reaches
// this adapter because its learner resource report is explicitly excluded.
fn physical_from_snapshots(
    ledger: ResourceSnapshot,
    grammar: Option<ResourceSnapshot>,
) -> PhysicalResourceReport {
    let stage_operations = |stage: Stage| {
        ledger.stage(stage).operation_count().saturating_add(
            grammar
                .map(|snapshot| snapshot.stage(stage).operation_count())
                .unwrap_or(0),
        )
    };
    let peak_memory = match (
        ledger.peak_resident_memory(),
        grammar.map(ResourceSnapshot::peak_resident_memory),
    ) {
        (MemorySample::Measured(left), Some(MemorySample::Measured(right))) => {
            MemorySample::Measured(left.max(right))
        }
        (MemorySample::Measured(value), _) | (_, Some(MemorySample::Measured(value))) => {
            MemorySample::Measured(value)
        }
        _ => MemorySample::Missing,
    };
    PhysicalResourceReport {
        included: true,
        wall_clock_ns: ledger
            .wall_clock_ns()
            .max(grammar.map(ResourceSnapshot::wall_clock_ns).unwrap_or(0)),
        cpu_time_ns: 0,
        peak_process_memory_bytes: peak_memory,
        process_count: 1,
        worker_count: 1,
        gpu_count: 0,
        network_connection_count: 0,
        stage_operations: StageOperations {
            canonicalization: stage_operations(Stage::Canonicalization),
            retrieval: stage_operations(Stage::Retrieval),
            admission: stage_operations(Stage::Admission),
            revalidation: stage_operations(Stage::Revalidation),
            candidate_generation: stage_operations(Stage::CandidateGeneration),
            decision_action: stage_operations(Stage::DecisionAction),
        },
    }
}

struct EpisodeDecision {
    sequence: u64,
    input: StepInput,
    decision: Decision,
    result: Option<StepResult>,
}

enum DriverState {
    Proposed(Box<Proposer>),
    B0(B0Driver),
    B1(B1Driver),
    B2(B2Driver),
}

impl DriverState {
    fn new(predictor: Predictor, run_scope: ScopeToken) -> Result<Self, ExperimentError> {
        Ok(match predictor {
            Predictor::Proposed => Self::Proposed(Box::new(Proposer::new(run_scope)?)),
            Predictor::B0ExactInteractionRetrieval => Self::B0(B0Driver::new()),
            Predictor::B1SeededRandomExploration => Self::B1(B1Driver::new()),
            Predictor::B2EvaluatorController => Self::B2(B2Driver::new()),
        })
    }

    fn before_decision(
        &mut self,
        episode_index: usize,
        input: StepInput,
        ledger: &mut InteractionLedger,
    ) -> Result<(), ExperimentError> {
        match self {
            Self::Proposed(driver) => driver.before_decision(episode_index, input, ledger),
            Self::B0(driver) => driver.before_decision(episode_index, input, ledger),
            Self::B1(driver) => driver.before_decision(episode_index, input, ledger),
            Self::B2(driver) => driver.before_decision(episode_index, input, ledger),
        }
    }

    fn decide(
        &mut self,
        input: StepInput,
        ledger: &mut InteractionLedger,
    ) -> Result<
        (
            super::interaction_ledger::DecisionReceipt,
            DecisionTraceRecord,
        ),
        ExperimentError,
    > {
        match self {
            Self::Proposed(driver) => driver.decide(input, ledger),
            Self::B0(driver) => driver.decide(input, ledger),
            Self::B1(driver) => driver.decide(input, ledger),
            Self::B2(driver) => driver.decide(input, ledger),
        }
    }

    fn after_result(
        &mut self,
        result: Option<StepResult>,
        ledger: &mut InteractionLedger,
    ) -> Result<(), ExperimentError> {
        match self {
            Self::Proposed(driver) => driver.after_result(result, ledger),
            Self::B0(driver) => driver.after_result(result, ledger),
            Self::B1(driver) => driver.after_result(result, ledger),
            Self::B2(driver) => driver.after_result(result, ledger),
        }
    }

    fn after_episode(
        &mut self,
        episode_index: usize,
        ledger: &mut InteractionLedger,
    ) -> Result<(), ExperimentError> {
        match self {
            Self::Proposed(driver) => driver.after_episode(episode_index, ledger),
            Self::B0(driver) => driver.after_episode(episode_index, ledger),
            Self::B1(driver) => driver.after_episode(episode_index, ledger),
            Self::B2(driver) => driver.after_episode(episode_index, ledger),
        }
    }

    fn freeze(&mut self, ledger: &mut InteractionLedger) -> Result<(), ExperimentError> {
        match self {
            Self::Proposed(driver) => driver.freeze(ledger),
            Self::B0(driver) => driver.freeze(ledger),
            Self::B1(driver) => driver.freeze(ledger),
            Self::B2(driver) => driver.freeze(ledger),
        }
    }

    fn retrieval_operations(&self) -> u64 {
        match self {
            Self::Proposed(driver) => driver.retrieval_operations(),
            Self::B0(driver) => driver.retrieval_operations(),
            Self::B1(driver) => driver.retrieval_operations(),
            Self::B2(driver) => driver.retrieval_operations(),
        }
    }

    fn set_variant_context(&mut self, variant: RunVariant, world: &EvaluatorWorld) {
        match self {
            Self::B1(driver) => driver.set_seed(variant.random_baseline_seed()),
            Self::B2(driver) => {
                driver.set_world_context(world.mappings(), world.logical_state_for_audit())
            }
            _ => {}
        }
    }

    fn grammar_meter(&self) -> Option<&ResourceMeter> {
        match self {
            Self::Proposed(driver) => Some(driver.grammar.resource_meter()),
            _ => None,
        }
    }

    fn sample_resource_snapshot(&mut self) -> Option<ResourceSnapshot> {
        match self {
            Self::Proposed(driver) => {
                Some(driver.grammar.resource_meter_mut().sample_and_snapshot())
            }
            _ => None,
        }
    }

    fn discovery_metrics(&self) -> DiscoveryMetrics {
        match self {
            Self::Proposed(driver) => driver.discovery,
            _ => DiscoveryMetrics::default(),
        }
    }

    fn budget_status(&self) -> BudgetStatus {
        match self {
            Self::Proposed(driver) => driver.budget,
            _ => BudgetStatus {
                over_limit_charges: 0,
                unrecorded_charges: 0,
                exhaustion_events: 0,
                confident_after_exhaustion: 0,
            },
        }
    }

    fn failures(&self) -> Vec<FailureRecord> {
        match self {
            Self::Proposed(driver) => driver.failures.clone(),
            _ => Vec::new(),
        }
    }

    fn candidate_events(&self) -> Vec<String> {
        match self {
            Self::Proposed(driver) => driver.candidate_events.clone(),
            _ => Vec::new(),
        }
    }

    fn procedure_records(&self) -> Vec<String> {
        match self {
            Self::Proposed(driver) => driver.procedure_records(),
            _ => Vec::new(),
        }
    }

    fn frozen_state_unchanged(&self) -> bool {
        match self {
            Self::Proposed(driver) => driver.frozen_state_unchanged(),
            _ => true,
        }
    }

    fn freeze_called(&self) -> bool {
        match self {
            Self::Proposed(driver) => driver.freeze_called,
            _ => true,
        }
    }

    fn leakage(&self) -> LeakageStatus {
        match self {
            Self::Proposed(driver) => driver.leakage.clone(),
            _ => LeakageStatus::clean(),
        }
    }

    fn note_leakage_order_violation(&mut self) {
        if let Self::Proposed(driver) = self {
            driver.leakage.decision_before_result_violations += 1;
        }
    }
}

struct Proposer {
    learner: Option<GroundingLearner>,
    frozen: Option<FrozenGroundingLearner>,
    grammar: GrammarV2,
    target: Option<Candidate>,
    target_armed: bool,
    candidate_events: Vec<String>,
    discovery: DiscoveryMetrics,
    failures: Vec<FailureRecord>,
    budget: BudgetStatus,
    leakage: LeakageStatus,
    frozen_before_final: Option<FrozenGroundingLearner>,
    freeze_called: bool,
}

impl Proposer {
    fn new(run_scope: ScopeToken) -> Result<Self, ExperimentError> {
        Ok(Self {
            learner: Some(GroundingLearner::new(run_scope)?),
            frozen: None,
            grammar: GrammarV2::new(run_scope)
                .map_err(|error| ExperimentError::Grammar(format!("{error:?}")))?,
            target: None,
            target_armed: false,
            candidate_events: Vec::new(),
            discovery: DiscoveryMetrics::default(),
            failures: Vec::new(),
            budget: BudgetStatus {
                over_limit_charges: 0,
                unrecorded_charges: 0,
                exhaustion_events: 0,
                confident_after_exhaustion: 0,
            },
            leakage: LeakageStatus::clean(),
            frozen_before_final: None,
            freeze_called: false,
        })
    }

    fn before_decision(
        &mut self,
        episode_index: usize,
        input: StepInput,
        ledger: &mut InteractionLedger,
    ) -> Result<(), ExperimentError> {
        if episode_index != TRAINING_EPISODE_COUNT || self.target_armed {
            return Ok(());
        }
        let Some(target) = self.target.clone() else {
            self.failures.push(FailureRecord::new(
                "P3",
                FailureClassification::InsufficientEvidence,
                "no challenged E3 candidate was available before development",
                Vec::new(),
            ));
            return Ok(());
        };
        let Some(prediction) = target.predict(input.observation(), target.action()) else {
            self.failures.push(FailureRecord::new(
                "P3",
                FailureClassification::IncorrectHypothesis,
                "the selected opaque E3 candidate did not apply to the first development input",
                target.support_evidence_ids().to_vec(),
            ));
            return Ok(());
        };
        let wrong_prediction = input.observation();
        let learner = self
            .learner
            .as_mut()
            .expect("mutable learner before freeze");
        let competing_id = HypothesisId::new(1_000_000 + target.hypothesis_id().value());
        learner.declare_competing_prediction_with_cost(
            ledger,
            competing_id,
            target.action(),
            wrong_prediction,
            target.support_evidence_ids().to_vec(),
            LogicalCost {
                query_calls: 0,
                environment_actions: 0,
                structural_comparisons: 1,
                operation_steps: 1,
            },
        )?;
        learner.declare_hypothesis_separating_action(target.action())?;
        let _ = prediction;
        self.target_armed = true;
        Ok(())
    }

    fn decide(
        &mut self,
        input: StepInput,
        ledger: &mut InteractionLedger,
    ) -> Result<
        (
            super::interaction_ledger::DecisionReceipt,
            DecisionTraceRecord,
        ),
        ExperimentError,
    > {
        if let Some(frozen) = self.frozen.as_ref() {
            let recorded = frozen.step(ledger, input)?;
            return Ok((recorded.receipt.clone(), trace_from_recorded(&recorded)));
        }
        let learner = self
            .learner
            .as_mut()
            .expect("mutable learner before freeze");
        let recorded = learner.step(ledger, input)?;
        if recorded.trace.reason == DecisionReason::IncompleteBudget {
            self.budget.exhaustion_events += 1;
        }
        Ok((recorded.receipt.clone(), trace_from_recorded(&recorded)))
    }

    fn after_result(
        &mut self,
        result: Option<StepResult>,
        ledger: &mut InteractionLedger,
    ) -> Result<(), ExperimentError> {
        if let Some(learner) = self.learner.as_mut() {
            let _ = learner.observe(ledger, result)?;
        }
        Ok(())
    }

    fn after_episode(
        &mut self,
        episode_index: usize,
        ledger: &mut InteractionLedger,
    ) -> Result<(), ExperimentError> {
        if episode_index == TRAINING_EPISODE_COUNT - 1 {
            self.generate_training_candidates(ledger)?;
        }
        if episode_index == TRAINING_EPISODE_COUNT {
            self.admit_development_candidate(ledger)?;
        }
        Ok(())
    }

    fn generate_training_candidates(
        &mut self,
        ledger: &mut InteractionLedger,
    ) -> Result<(), ExperimentError> {
        // The grammar's fixed candidate budget is part of the protocol. The
        // first six completed training interactions are sufficient to contain
        // two varied supports and a contrasting challenge in both variants;
        // bounding this learner-owned view prevents a quadratic all-history
        // search from silently crossing the 512-attempt run ceiling.
        let records = ledger
            .interactions()
            .iter()
            .take(TRAINING_GENERATION_RECORD_LIMIT)
            .cloned()
            .collect::<Vec<_>>();
        let report = self.grammar.generate_interactions(&records);
        self.record_generation_events(&report);
        if matches!(report.status(), GenerationStatus::Incomplete { .. }) {
            self.budget.exhaustion_events += 1;
        }
        let learner = self.learner.as_mut().expect("mutable learner");
        learner.retain_generation(ledger, &report)?;
        learner.enter_development()?;
        self.target = report
            .candidates()
            .iter()
            .filter(|candidate| {
                candidate.stage() == CandidateStage::E3 && !candidate.changed_positions().is_empty()
            })
            .min_by_key(|candidate| candidate.canonical_order())
            .cloned();
        if self.target.is_none() {
            let classification = match report.status() {
                GenerationStatus::Incomplete { .. } => FailureClassification::SearchExhausted,
                GenerationStatus::UnsupportedByGrammar => {
                    FailureClassification::UnsupportedByGrammar
                }
                GenerationStatus::Complete => FailureClassification::InsufficientEvidence,
            };
            self.failures.push(FailureRecord::new(
                "P3",
                classification,
                "training produced no challenged structural E3 candidate",
                Vec::new(),
            ));
        }
        Ok(())
    }

    fn admit_development_candidate(
        &mut self,
        ledger: &mut InteractionLedger,
    ) -> Result<(), ExperimentError> {
        let Some(target) = self.target.clone() else {
            return Ok(());
        };
        match self.grammar.admit(target.clone(), ledger) {
            AdmissionOutcome::Admitted(admitted) => {
                self.discovery.admitted_e3_procedures += 1;
                self.discovery.support_interactions = target.support_evidence_ids().len() as u64;
                self.discovery.grounding_evidence_count =
                    admitted.grounding_evidence_ids().len() as u64;
                let support_ids = target.support_evidence_ids().to_vec();
                let contexts = support_context_count(ledger, &target, &support_ids);
                self.discovery.support_context_atoms = contexts;
                let procedure = self.grammar.promote(ledger, admitted)?;
                let learner = self.learner.as_mut().expect("mutable learner");
                learner.install_procedure(ledger, procedure.clone())?;
                self.candidate_events
                    .push(admission_event_json("admitted", &target, &[]));
            }
            AdmissionOutcome::Rejected(rejected) => {
                self.candidate_events.push(admission_event_json(
                    "rejected",
                    rejected.candidate(),
                    rejected.reasons(),
                ));
                let classification = if rejected
                    .reasons()
                    .contains(&RejectionReason::BudgetExhausted)
                {
                    FailureClassification::SearchExhausted
                } else if rejected
                    .reasons()
                    .contains(&RejectionReason::UnsupportedByGrammar)
                {
                    FailureClassification::UnsupportedByGrammar
                } else {
                    FailureClassification::InsufficientEvidence
                };
                self.failures.push(FailureRecord::new(
                    "P3",
                    classification,
                    "the fixed development admission gates rejected the structural candidate",
                    rejected.candidate().support_evidence_ids().to_vec(),
                ));
            }
            AdmissionOutcome::Incomplete(incomplete) => {
                self.budget.exhaustion_events += 1;
                self.candidate_events.push(admission_event_json(
                    "incomplete",
                    incomplete.candidate(),
                    &[RejectionReason::BudgetExhausted],
                ));
                self.failures.push(FailureRecord::new(
                    "P3",
                    FailureClassification::SearchExhausted,
                    "development admission reached a frozen logical budget",
                    incomplete.candidate().support_evidence_ids().to_vec(),
                ));
            }
        }
        Ok(())
    }

    fn freeze(&mut self, ledger: &mut InteractionLedger) -> Result<(), ExperimentError> {
        let learner = self.learner.take().expect("mutable learner before freeze");
        let frozen = learner.freeze(ledger)?;
        self.frozen_before_final = Some(frozen.clone());
        self.frozen = Some(frozen);
        self.freeze_called = true;
        Ok(())
    }

    fn retrieval_operations(&self) -> u64 {
        self.learner
            .as_ref()
            .map(|learner| learner.procedures().count() as u64)
            .or_else(|| self.frozen.as_ref().map(|_| 1))
            .unwrap_or(0)
    }

    fn frozen_state_unchanged(&self) -> bool {
        match (&self.frozen_before_final, &self.frozen) {
            (Some(before), Some(after)) => before == after,
            _ => true,
        }
    }

    fn record_generation_events(&mut self, report: &GenerationReport) {
        for candidate in report.candidates() {
            self.candidate_events
                .push(candidate_event_json("generated", candidate));
        }
        for rejection in report.rejections() {
            self.candidate_events.push(format!(
                "{{\"event\":\"rejected\",\"evidence_ids\":{},\"reasons\":{},\"stage\":\"{}\"}}",
                evidence_ids_json(rejection.evidence_ids()),
                json_array(
                    rejection
                        .reasons()
                        .iter()
                        .map(|reason| format!("\"{}\"", rejection_name(*reason)))
                ),
                rejection.stage().code(),
            ));
        }
        self.candidate_events.sort();
    }

    fn procedure_records(&self) -> Vec<String> {
        let mut values = Vec::new();
        for procedure in self.grammar.procedures() {
            values.push(procedure_json(procedure));
        }
        for revision in self.grammar.revisions() {
            values.push(revision_json(revision));
        }
        values
    }
}

struct B0Driver {
    entries: BTreeMap<(ObservationPacket, ActionId), ObservationPacket>,
}

impl B0Driver {
    fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    fn before_decision(
        &mut self,
        _episode_index: usize,
        _input: StepInput,
        _ledger: &mut InteractionLedger,
    ) -> Result<(), ExperimentError> {
        Ok(())
    }

    fn decide(
        &mut self,
        input: StepInput,
        ledger: &mut InteractionLedger,
    ) -> Result<
        (
            super::interaction_ledger::DecisionReceipt,
            DecisionTraceRecord,
        ),
        ExperimentError,
    > {
        let selected = input.available_actions().into_iter().find_map(|action| {
            self.entries
                .get(&(input.observation(), action))
                .copied()
                .map(|prediction| (action, prediction))
        });
        let (decision, reason) = match selected {
            Some((action, prediction)) => (
                Decision::act(
                    action,
                    Some(prediction),
                    ActMode::Fallback,
                    AuditReferences::empty(),
                )?,
                "exact_retrieval",
            ),
            None => (
                Decision::abstain(
                    AbstainReason::NoApplicableProcedure,
                    AuditReferences::empty(),
                ),
                "no_exact_interaction",
            ),
        };
        let receipt = ledger.record_decision(input, decision.clone())?;
        Ok((
            receipt.clone(),
            baseline_trace(&receipt, reason, LogicalCost::default()),
        ))
    }

    fn after_result(
        &mut self,
        result: Option<StepResult>,
        ledger: &mut InteractionLedger,
    ) -> Result<(), ExperimentError> {
        if let (Some(result), Some(interaction)) = (result, ledger.interactions().last()) {
            if let Some(action) = interaction.evidence().action() {
                self.entries.insert(
                    (interaction.evidence().before(), action),
                    result.observation(),
                );
            }
        }
        Ok(())
    }

    fn after_episode(
        &mut self,
        _index: usize,
        _ledger: &mut InteractionLedger,
    ) -> Result<(), ExperimentError> {
        Ok(())
    }

    fn freeze(&mut self, ledger: &mut InteractionLedger) -> Result<(), ExperimentError> {
        ledger.freeze()?;
        Ok(())
    }

    fn retrieval_operations(&self) -> u64 {
        2
    }
}

struct B1Driver {
    random: SplitMix64,
    seed_set: bool,
}

impl B1Driver {
    fn new() -> Self {
        Self {
            random: SplitMix64::new(0),
            seed_set: false,
        }
    }

    fn before_decision(
        &mut self,
        episode_index: usize,
        _input: StepInput,
        _ledger: &mut InteractionLedger,
    ) -> Result<(), ExperimentError> {
        // The exact variant seed is installed in `before_variant_seed`.
        if episode_index == usize::MAX {
            unreachable!();
        }
        Ok(())
    }

    fn set_seed(&mut self, seed: u64) {
        if !self.seed_set {
            self.random = SplitMix64::new(seed);
            self.seed_set = true;
        }
    }

    fn decide(
        &mut self,
        input: StepInput,
        ledger: &mut InteractionLedger,
    ) -> Result<
        (
            super::interaction_ledger::DecisionReceipt,
            DecisionTraceRecord,
        ),
        ExperimentError,
    > {
        let actions = input.available_actions();
        let action = actions[(self.random.next_u64() % actions.len() as u64) as usize];
        let decision = Decision::act(action, None, ActMode::Exploration, AuditReferences::empty())?;
        let receipt = ledger.record_decision(input, decision)?;
        Ok((
            receipt.clone(),
            baseline_trace(&receipt, "seeded_random", LogicalCost::default()),
        ))
    }

    fn after_result(
        &mut self,
        _result: Option<StepResult>,
        _ledger: &mut InteractionLedger,
    ) -> Result<(), ExperimentError> {
        Ok(())
    }

    fn after_episode(
        &mut self,
        _index: usize,
        _ledger: &mut InteractionLedger,
    ) -> Result<(), ExperimentError> {
        Ok(())
    }

    fn freeze(&mut self, ledger: &mut InteractionLedger) -> Result<(), ExperimentError> {
        ledger.freeze()?;
        Ok(())
    }

    fn retrieval_operations(&self) -> u64 {
        0
    }
}

struct B2Driver {
    world_mappings: Option<super::evaluator_world::IdentifierMappings>,
    current_world_state: Option<(
        super::evaluator_world::HiddenAtom,
        super::evaluator_world::HiddenAtom,
        u8,
    )>,
}

impl B2Driver {
    fn new() -> Self {
        Self {
            world_mappings: None,
            current_world_state: None,
        }
    }

    fn before_decision(
        &mut self,
        _episode_index: usize,
        _input: StepInput,
        _ledger: &mut InteractionLedger,
    ) -> Result<(), ExperimentError> {
        Ok(())
    }

    fn decide(
        &mut self,
        input: StepInput,
        ledger: &mut InteractionLedger,
    ) -> Result<
        (
            super::interaction_ledger::DecisionReceipt,
            DecisionTraceRecord,
        ),
        ExperimentError,
    > {
        // The actual hidden lookup is installed by `set_world_context` just
        // before this controller is used. In normal execution the fallback
        // wire action is still resolved by the evaluator world, while B2's
        // exact prediction is computed in `controller_decision`.
        let decision = controller_decision(input, self.world_mappings, self.current_world_state)?;
        let receipt = ledger.record_decision(input, decision)?;
        Ok((
            receipt.clone(),
            baseline_trace(
                &receipt,
                "fixed_evaluator_controller",
                LogicalCost::default(),
            ),
        ))
    }

    fn set_world_context(
        &mut self,
        mappings: super::evaluator_world::IdentifierMappings,
        state: Option<(
            super::evaluator_world::HiddenAtom,
            super::evaluator_world::HiddenAtom,
            u8,
        )>,
    ) {
        self.world_mappings = Some(mappings);
        self.current_world_state = state;
    }

    fn after_result(
        &mut self,
        _result: Option<StepResult>,
        _ledger: &mut InteractionLedger,
    ) -> Result<(), ExperimentError> {
        Ok(())
    }

    fn after_episode(
        &mut self,
        _index: usize,
        _ledger: &mut InteractionLedger,
    ) -> Result<(), ExperimentError> {
        Ok(())
    }

    fn freeze(&mut self, ledger: &mut InteractionLedger) -> Result<(), ExperimentError> {
        ledger.freeze()?;
        Ok(())
    }

    fn retrieval_operations(&self) -> u64 {
        0
    }
}

fn controller_decision(
    _input: StepInput,
    mappings: Option<super::evaluator_world::IdentifierMappings>,
    state: Option<(
        super::evaluator_world::HiddenAtom,
        super::evaluator_world::HiddenAtom,
        u8,
    )>,
) -> Result<Decision, ExperimentError> {
    let mappings =
        mappings.ok_or_else(|| ExperimentError::Grammar("B2 mapping not installed".to_owned()))?;
    let (_context, marker, _count) = state
        .ok_or_else(|| ExperimentError::Grammar("B2 hidden state not installed".to_owned()))?;
    let action = mappings.action_id(super::evaluator_world::HiddenAction::A0);
    let next_marker = if marker == super::evaluator_world::HiddenAtom::M0 {
        super::evaluator_world::HiddenAtom::M1
    } else {
        marker
    };
    let context = _context;
    let prediction = ObservationPacket::new([
        mappings.atom_id(context),
        mappings.atom_id(next_marker),
        mappings.atom_id(context),
    ]);
    Decision::act(
        action,
        Some(prediction),
        ActMode::Fallback,
        AuditReferences::empty(),
    )
    .map_err(ExperimentError::from)
}

fn baseline_trace(
    receipt: &super::interaction_ledger::DecisionReceipt,
    reason: &str,
    cost: LogicalCost,
) -> DecisionTraceRecord {
    DecisionTraceRecord {
        sequence: receipt.sequence(),
        input: receipt.record().input(),
        decision: receipt.decision().clone(),
        reason: reason.to_owned(),
        competing_prediction_count: 0,
        disagreement_action_count: 0,
        cost,
        decision_before_result: false,
        result: None,
    }
}

fn trace_from_recorded(recorded: &RecordedDecision) -> DecisionTraceRecord {
    DecisionTraceRecord {
        sequence: recorded.receipt.sequence(),
        input: recorded.trace.input,
        decision: recorded.trace.decision.clone(),
        reason: decision_reason_name(recorded.trace.reason).to_owned(),
        competing_prediction_count: recorded.trace.competing_predictions.len() as u64,
        disagreement_action_count: recorded.trace.disagreement_actions.len() as u64,
        cost: recorded.trace.cost,
        decision_before_result: false,
        result: None,
    }
}

fn score_episode(
    spec: super::evaluator_world::EpisodeSpec,
    decisions: &[EpisodeDecision],
    terminal: TerminalStatus,
) -> EpisodeMetric {
    let mut prediction_attempts = 0;
    let mut exact_predictions = 0;
    let mut procedure_backed_actions = 0;
    let mut procedure_backed_predictions = 0;
    let mut abstention_count = 0;
    let mut grounding_evidence_count = 0;
    let mut sequences = Vec::new();
    for decision in decisions {
        sequences.push(decision.sequence);
        if decision.decision.is_abstain() {
            abstention_count += 1;
        }
        if let Decision::Act(act) = &decision.decision {
            if act.mode() == ActMode::Procedure && !act.audit().procedure_refs().is_empty() {
                procedure_backed_actions += 1;
                grounding_evidence_count += act.audit().evidence_refs().len() as u64;
                if act.prediction().is_some() {
                    procedure_backed_predictions += 1;
                }
            }
        }
        if let Some(prediction) = decision.decision.prediction() {
            prediction_attempts += 1;
            if decision
                .result
                .is_some_and(|result| result.observation() == prediction)
            {
                exact_predictions += 1;
            }
        }
    }
    let false_positive =
        spec.role() == EpisodeRole::HeldOutNonApplicable && prediction_attempts > 0;
    let retention = match spec.role() {
        EpisodeRole::FrozenRetentionApplicable => {
            procedure_backed_predictions > 0 && exact_predictions == prediction_attempts
        }
        EpisodeRole::FrozenRetentionNonApplicable => {
            prediction_attempts == 0 && procedure_backed_actions == 0
        }
        _ => false,
    };
    EpisodeMetric {
        order: spec.order(),
        label: spec.label().to_owned(),
        partition: partition_name(spec.partition()).to_owned(),
        role: role_name(spec.role()).to_owned(),
        episode_scope: decisions
            .first()
            .map(|decision| decision.input.episode_scope())
            .unwrap_or(ScopeToken::new(0)),
        decision_sequences: sequences,
        action_count: decisions
            .iter()
            .filter(|decision| decision.decision.action().is_some())
            .count() as u64,
        terminal,
        task_success: terminal == TerminalStatus::Success,
        prediction_attempts,
        exact_predictions,
        procedure_backed_actions,
        procedure_backed_predictions,
        abstention_count,
        incomplete_count: u64::from(terminal == TerminalStatus::Incomplete),
        false_positive,
        retention_pass: retention,
        grounding_evidence_count,
    }
}

fn evaluate_primary(runs: &[RunReport], comparisons: &[RerunComparison]) -> Vec<GateResult> {
    let proposed = runs_for(runs, RunVariant::Primary, Predictor::Proposed);
    let b0 = runs_for(
        runs,
        RunVariant::Primary,
        Predictor::B0ExactInteractionRetrieval,
    );
    let b1 = runs_for(
        runs,
        RunVariant::Primary,
        Predictor::B1SeededRandomExploration,
    );
    let b2 = runs_for(runs, RunVariant::Primary, Predictor::B2EvaluatorController);
    let first = proposed.first();
    let p1 = proposed.len() == RERUN_COUNT
        && b0.len() == RERUN_COUNT
        && b1.len() == RERUN_COUNT
        && b2.len() == RERUN_COUNT
        && proposed.iter().all(|run| {
            run.episodes.len() == EPISODE_COUNT
                && run.leakage.is_clean()
                && run
                    .decision_traces
                    .iter()
                    .all(|trace| trace.decision_before_result)
                && run.decision_traces.len()
                    == run
                        .metrics
                        .episodes
                        .iter()
                        .map(|e| e.decision_sequences.len())
                        .sum::<usize>()
        });
    let p2 = find_comparison(comparisons, RunVariant::Primary, Predictor::Proposed)
        .is_some_and(|comparison| comparison.byte_identical);
    let p3 = first.is_some_and(|run| {
        run.metrics.discovery.admitted_e3_procedures > 0
            && run.metrics.discovery.support_interactions >= MIN_STRUCTURAL_SUPPORT
            && run.metrics.discovery.support_context_atoms >= MIN_STRUCTURAL_CONTEXTS
            && !run.metrics.discovery.e0_only_credit
    });
    let p4 = first.is_some_and(|run| {
        run.metrics
            .held_out_applicable_prediction_coverage
            .numerator()
            >= MIN_TRANSFER_COVERAGE
    });
    let p5 = first.is_some_and(|run| {
        run.metrics.held_out_applicable_task_success.numerator() >= MIN_TRANSFER_SUCCESS
    });
    let p6 = first.is_some_and(|run| {
        run.metrics.held_out_applicable_packet_accuracy.numerator()
            == run
                .metrics
                .held_out_applicable_packet_accuracy
                .denominator()
            && run.metrics.held_out_applicable_prediction_attempts >= MIN_PACKET_ATTEMPTS
    });
    let p7 = first.is_some_and(|run| {
        run.metrics
            .held_out_non_applicable_false_positives
            .numerator()
            == 0
    });
    let p8 = first.is_some_and(|run| {
        run.metrics
            .held_out_non_applicable_abstention_quality
            .numerator()
            == HELD_OUT_NON_APPLICABLE_COUNT
    });
    let p9 = first.is_some_and(|run| {
        run.metrics.frozen_retention.numerator() == FROZEN_RETENTION_COUNT
            && run.frozen_state_unchanged
            && run
                .procedure_records
                .iter()
                .all(|record| !record.is_empty())
    });
    let p10 =
        first.is_some_and(|run| run.leakage.is_clean() && run.metrics.grounding_evidence_count > 0);
    let p11 = first.is_some_and(|run| run.budget.honest());
    let p12 = proposed.len() == RERUN_COUNT
        && proposed.iter().all(|run| run.physical_resources.complete());
    vec![
        gate(
            "P1",
            p1,
            FailureClassification::FalsifiedByLeakage,
            "protocol integrity, ordering, and 36-row schedule",
        ),
        gate(
            "P2",
            p2,
            FailureClassification::FalsifiedByDeterminism,
            "three primary proposed reruns have identical deterministic cores",
        ),
        gate(
            "P3",
            p3,
            FailureClassification::InsufficientEvidence,
            "run-scoped E3 discovery with varied support",
        ),
        gate(
            "P4",
            p4,
            FailureClassification::FalsifiedByHeldOutGeneralization,
            "held-out applicable procedure-backed coverage",
        ),
        gate(
            "P5",
            p5,
            FailureClassification::FalsifiedByHeldOutGeneralization,
            "held-out applicable task success",
        ),
        gate(
            "P6",
            p6,
            FailureClassification::FalsifiedByHeldOutGeneralization,
            "held-out exact packet accuracy with minimum attempts",
        ),
        gate(
            "P7",
            p7,
            FailureClassification::FalsifiedByFalsePositiveControl,
            "no held-out non-applicable prediction attempts",
        ),
        gate(
            "P8",
            p8,
            FailureClassification::FalsifiedByAbstentionQuality,
            "non-applicable abstention quality",
        ),
        gate(
            "P9",
            p9,
            FailureClassification::FalsifiedByRetention,
            "four frozen retention probes preserve lineage",
        ),
        gate(
            "P10",
            p10,
            FailureClassification::FalsifiedByEvidenceTraceability,
            "pre-decision references and grounding evidence",
        ),
        gate(
            "P11",
            p11,
            FailureClassification::FalsifiedByBudget,
            "budget charges and incomplete statuses are honest",
        ),
        gate(
            "P12",
            p12,
            FailureClassification::FalsifiedByResourceLimit,
            "complete physical resource reports for three reruns",
        ),
    ]
}

fn evaluate_secondary(
    primary: &[RunReport],
    secondary: &[RunReport],
    comparisons: &[RerunComparison],
) -> Vec<GateResult> {
    let proposed = runs_for(
        secondary,
        RunVariant::SymbolActionPermutation,
        Predictor::Proposed,
    );
    let b0 = runs_for(
        secondary,
        RunVariant::SymbolActionPermutation,
        Predictor::B0ExactInteractionRetrieval,
    );
    let b1 = runs_for(
        secondary,
        RunVariant::SymbolActionPermutation,
        Predictor::B1SeededRandomExploration,
    );
    let b2 = runs_for(
        secondary,
        RunVariant::SymbolActionPermutation,
        Predictor::B2EvaluatorController,
    );
    let primary_proposed = runs_for(primary, RunVariant::Primary, Predictor::Proposed);
    let first = proposed.first();
    let s1 = first.is_some_and(|run| {
        run.metrics.discovery.admitted_e3_procedures > 0
            && run.metrics.discovery.support_interactions >= MIN_STRUCTURAL_SUPPORT
            && run.metrics.discovery.support_context_atoms >= MIN_STRUCTURAL_CONTEXTS
    });
    let s2 = first.is_some_and(|run| {
        run.metrics
            .held_out_applicable_prediction_coverage
            .numerator()
            >= MIN_TRANSFER_COVERAGE
            && run.metrics.held_out_applicable_task_success.numerator() >= MIN_TRANSFER_SUCCESS
    });
    let s3 = first.is_some_and(|run| {
        run.metrics.held_out_applicable_packet_accuracy.numerator()
            == run
                .metrics
                .held_out_applicable_packet_accuracy
                .denominator()
            && run.metrics.held_out_applicable_prediction_attempts >= MIN_PACKET_ATTEMPTS
            && run
                .metrics
                .held_out_non_applicable_false_positives
                .numerator()
                == 0
            && run
                .metrics
                .held_out_non_applicable_abstention_quality
                .numerator()
                == HELD_OUT_NON_APPLICABLE_COUNT
    });
    let s4 = first.is_some_and(|run| {
        run.metrics.frozen_retention.numerator() == FROZEN_RETENTION_COUNT
            && run.frozen_state_unchanged
    });
    let s5 = match (primary_proposed.first(), first) {
        (Some(left), Some(right)) => {
            left.metrics
                .held_out_applicable_prediction_coverage
                .numerator()
                .abs_diff(
                    right
                        .metrics
                        .held_out_applicable_prediction_coverage
                        .numerator(),
                )
                <= 1
                && left
                    .metrics
                    .held_out_applicable_task_success
                    .numerator()
                    .abs_diff(right.metrics.held_out_applicable_task_success.numerator())
                    <= 1
                && left.metrics.held_out_applicable_packet_accuracy.numerator()
                    == left
                        .metrics
                        .held_out_applicable_packet_accuracy
                        .denominator()
                && right
                    .metrics
                    .held_out_applicable_packet_accuracy
                    .numerator()
                    == right
                        .metrics
                        .held_out_applicable_packet_accuracy
                        .denominator()
        }
        _ => false,
    };
    let s6 = b0.first().is_some_and(|run| {
        run.metrics
            .held_out_applicable_prediction_coverage
            .numerator()
            == 0
    }) && b2.first().is_some_and(|run| {
        run.metrics.held_out_applicable_task_success.numerator() == HELD_OUT_APPLICABLE_COUNT
            && run.metrics.held_out_applicable_packet_accuracy.numerator()
                == run
                    .metrics
                    .held_out_applicable_packet_accuracy
                    .denominator()
    }) && find_comparison(
        comparisons,
        RunVariant::SymbolActionPermutation,
        Predictor::B1SeededRandomExploration,
    )
    .is_some_and(|comparison| comparison.byte_identical)
        && b1.len() == RERUN_COUNT;
    let s7 = proposed.len() == RERUN_COUNT
        && proposed.iter().all(|run| run.physical_resources.complete());
    vec![
        gate(
            "S1",
            s1,
            FailureClassification::SecondaryRobustnessFailed,
            "permutation structural discovery",
        ),
        gate(
            "S2",
            s2,
            FailureClassification::SecondaryRobustnessFailed,
            "permutation transfer",
        ),
        gate(
            "S3",
            s3,
            FailureClassification::SecondaryRobustnessFailed,
            "permutation accuracy and non-applicability control",
        ),
        gate(
            "S4",
            s4,
            FailureClassification::SecondaryRobustnessFailed,
            "permutation frozen retention",
        ),
        gate(
            "S5",
            s5,
            FailureClassification::SecondaryRobustnessFailed,
            "cross-variant parity",
        ),
        gate(
            "S6",
            s6,
            FailureClassification::SecondaryRobustnessFailed,
            "baseline integrity",
        ),
        gate(
            "S7",
            s7,
            FailureClassification::SecondaryRobustnessFailed,
            "permutation resource completeness",
        ),
    ]
}

fn gate(id: &str, passed: bool, failure: FailureClassification, detail: &str) -> GateResult {
    GateResult::new(id, passed, (!passed).then_some(failure), detail)
}

fn gate_failures(gates: &[GateResult]) -> Vec<FailureRecord> {
    gates
        .iter()
        .filter(|gate| !gate.passed)
        .map(|gate| {
            FailureRecord::new(
                gate.id.clone(),
                gate.classification
                    .unwrap_or(FailureClassification::SecondaryRobustnessFailed),
                gate.detail.clone(),
                Vec::new(),
            )
        })
        .collect()
}

fn runs_for(runs: &[RunReport], variant: RunVariant, predictor: Predictor) -> Vec<&RunReport> {
    let mut result = runs
        .iter()
        .filter(|run| run.variant == variant && run.predictor == predictor)
        .collect::<Vec<_>>();
    result.sort_by_key(|run| run.rerun_index);
    result
}

fn find_comparison(
    comparisons: &[RerunComparison],
    variant: RunVariant,
    predictor: Predictor,
) -> Option<&RerunComparison> {
    comparisons
        .iter()
        .find(|comparison| comparison.variant == variant && comparison.predictor == predictor)
}

/// Write the frozen protocol and one run's report layout.  The caller decides
/// whether to package one run or the complete experiment.  All files are
/// written with LF endings and deterministic JSON; only the physical report
/// contains monotonic runtime values.
pub fn write_run_artifacts(
    report: &RunReport,
    output: impl AsRef<Path>,
) -> Result<Vec<(String, String)>, ExperimentError> {
    let root = output.as_ref();
    let primary_seeds = RunSeeds::for_variant(RunVariant::Primary);
    let permutation_seeds = RunSeeds::for_variant(RunVariant::SymbolActionPermutation);
    fs::create_dir_all(root.join("learner"))?;
    fs::create_dir_all(root.join("baselines").join("b0"))?;
    fs::create_dir_all(root.join("baselines").join("b1"))?;
    fs::create_dir_all(root.join("baselines").join("b2"))?;
    fs::create_dir_all(root.join("evaluator-only"))?;
    write_text(root.join("protocol.md"), FROZEN_PROTOCOL)?;
    write_text(
        root.join("run-manifest.json"),
        &format!(
            "{{\"decision_ceiling\":72,\"final_episode_count\":16,\"freeze_boundary\":20,\"grammar_version\":\"{}\",\"permutation_random_baseline_seed\":{},\"permutation_scope_seed\":{},\"permutation_world_seed\":{},\"predictor\":\"{}\",\"primary_random_baseline_seed\":{},\"primary_scope_seed\":{},\"primary_world_seed\":{},\"protocol_version\":\"{}\",\"random_baseline_seed\":{},\"rerun_index\":{},\"run_scope\":\"{}\",\"schedule_fingerprint\":\"{}\",\"scope_seed\":{},\"source_fingerprint\":\"{}\",\"spec_version\":\"{}\",\"status\":\"executed-but-not-validated\",\"total_episode_count\":36,\"training_episode_count\":12,\"variant\":\"{}\",\"world_seed\":{}}}",
            report.grammar_version,
            permutation_seeds.random_baseline_seed,
            permutation_seeds.scope_seed,
            permutation_seeds.world_seed,
            report.predictor.name(),
            primary_seeds.random_baseline_seed,
            primary_seeds.scope_seed,
            primary_seeds.world_seed,
            report.protocol_version,
            report.seeds.random_baseline_seed,
            report.rerun_index,
            report.run_scope,
            report.schedule_fingerprint,
            report.seeds.scope_seed,
            SOURCE_FINGERPRINT,
            report.spec_version,
            report.variant.name(),
            report.seeds.world_seed,
        ),
    )?;
    write_text(
        root.join("learner").join("decision-trace.jsonl"),
        &report.decision_trace_jsonl(),
    )?;
    write_text(
        root.join("learner").join("interaction-ledger.jsonl"),
        report.interaction_ledger_jsonl(),
    )?;
    write_text(
        root.join("learner").join("candidate-events.jsonl"),
        &jsonl(report.candidate_events.iter().cloned()),
    )?;
    write_text(
        root.join("learner").join("procedure-records.jsonl"),
        &jsonl(report.procedure_records.iter().cloned()),
    )?;
    write_text(root.join("metrics.json"), &report.metrics.to_json())?;
    write_text(
        root.join("acceptance.json"),
        &format!(
            "{{\"run_core_fingerprint\":\"{}\",\"status\":\"{}\"}}",
            report.deterministic_core_fingerprint(),
            if report.failures.is_empty() {
                "no-run-failure"
            } else {
                "run-failure-recorded"
            }
        ),
    )?;
    write_text(root.join("failures.jsonl"), &report.failures_jsonl())?;
    write_text(
        root.join("logical-resource-report.json"),
        &report.logical_resources.to_json(),
    )?;
    write_text(
        root.join("physical-resource-report.json"),
        &report.physical_resource_report_json(),
    )?;
    write_text(
        root.join("evaluator-only").join("world-manifest.json"),
        &world_manifest_json(report.variant, report.seeds),
    )?;
    write_text(
        root.join("evaluator-only").join("episode-manifest.json"),
        &episode_manifest_json(report.variant),
    )?;
    write_text(
        root.join("evaluator-only").join("score-input.json"),
        &score_input_json(report),
    )?;
    let mut hashes = collect_hashes(root)?;
    hashes.sort_by(|left, right| left.0.cmp(&right.0));
    let sums = hashes
        .iter()
        .map(|(path, digest)| format!("{}  {}", digest, path))
        .collect::<Vec<_>>()
        .join("\n");
    write_text(root.join("sha256sums.txt"), &format!("{sums}\n"))?;
    Ok(hashes)
}

pub fn write_experiment_artifacts(
    report: &ExperimentReport,
    output: impl AsRef<Path>,
) -> Result<(), ExperimentError> {
    let root = output.as_ref();
    fs::create_dir_all(root)?;
    write_text(
        root.join("experiment-report.json"),
        &report.deterministic_json(),
    )?;
    write_text(
        root.join("experiment-fingerprint.txt"),
        &format!("{}\n", report.deterministic_fingerprint()),
    )?;
    for run in &report.runs {
        let path = root
            .join(run.variant.name())
            .join(run.predictor.name())
            .join(format!("rerun-{}", run.rerun_index));
        write_run_artifacts(run, path)?;
    }
    for run in &report.runs {
        let path = root
            .join(run.variant.name())
            .join(run.predictor.name())
            .join(format!("rerun-{}", run.rerun_index));
        for (predictor, directory) in [
            (Predictor::B0ExactInteractionRetrieval, "b0"),
            (Predictor::B1SeededRandomExploration, "b1"),
            (Predictor::B2EvaluatorController, "b2"),
        ] {
            if let Some(baseline) = report.runs.iter().find(|candidate| {
                candidate.variant == run.variant
                    && candidate.predictor == predictor
                    && candidate.rerun_index == run.rerun_index
            }) {
                write_text(
                    path.join("baselines").join(directory).join("metrics.json"),
                    &baseline.metrics.to_json(),
                )?;
            }
        }
        refresh_hashes(&path)?;
    }
    let hashes = collect_hashes(root)?;
    write_text(
        root.join("artifact-hashes.json"),
        &json_array(hashes.iter().map(|(path, digest)| {
            format!(
                "{{\"path\":\"{}\",\"sha256\":\"{}\"}}",
                json_escape(path),
                digest
            )
        })),
    )?;
    Ok(())
}

fn write_text(path: PathBuf, content: &str) -> Result<(), ExperimentError> {
    fs::write(path, content.as_bytes()).map_err(ExperimentError::from)
}

fn collect_hashes(root: &Path) -> Result<Vec<(String, String)>, ExperimentError> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort();
    Ok(files
        .into_iter()
        .filter(|(_, path)| path != "sha256sums.txt")
        .map(|(bytes, path)| (path, sha256_hex(bytes.as_bytes())))
        .collect())
}

fn refresh_hashes(root: &Path) -> Result<(), ExperimentError> {
    let mut hashes = collect_hashes(root)?;
    hashes.sort_by(|left, right| left.0.cmp(&right.0));
    let sums = hashes
        .iter()
        .map(|(path, digest)| format!("{}  {}", digest, path))
        .collect::<Vec<_>>()
        .join("\n");
    write_text(root.join("sha256sums.txt"), &format!("{sums}\n"))
}

fn collect_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<(String, String)>,
) -> Result<(), ExperimentError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else {
            let relative = path
                .strip_prefix(root)
                .expect("walked below root")
                .to_string_lossy()
                .replace('\\', "/");
            files.push((
                String::from_utf8_lossy(&fs::read(&path)?).into_owned(),
                relative,
            ));
        }
    }
    Ok(())
}

fn interaction_ledger_jsonl(ledger: &InteractionLedger) -> String {
    jsonl(ledger.interactions().iter().map(|interaction| {
        format!(
            "{{\"decision\":{},\"evidence\":{}}}",
            interaction.decision().to_json(),
            evidence_json(interaction.evidence()),
        )
    }))
}

fn world_manifest_json(variant: RunVariant, seeds: RunSeeds) -> String {
    let mappings = EvaluatorWorld::new(variant).mappings();
    format!(
        "{{\"action_wire_order\":{},\"atom_wire_order\":{},\"random_baseline_seed\":{},\"scope_seed\":{},\"variant\":\"{}\",\"world_seed\":{}}}",
        json_array(mappings.action_wire_order().iter().map(|action| format!("\"{:?}\"", action))),
        json_array(mappings.atom_wire_order().iter().map(|atom| format!("\"{:?}\"", atom))),
        seeds.random_baseline_seed,
        seeds.scope_seed,
        variant.name(),
        seeds.world_seed,
    )
}

fn episode_manifest_json(variant: RunVariant) -> String {
    let world = EvaluatorWorld::new(variant);
    let scopes = world.episode_scopes();
    json_array(fixed_schedule().iter().enumerate().map(|(index, spec)| {
        format!(
            "{{\"episode_scope\":\"{}\",\"initial_context\":\"{:?}\",\"initial_marker\":\"{:?}\",\"label\":\"{}\",\"order\":{},\"partition\":\"{}\",\"role\":\"{}\"}}",
            scopes[index],
            spec.initial_context(),
            spec.initial_marker(),
            spec.label(),
            spec.order(),
            partition_name(spec.partition()),
            role_name(spec.role()),
        )
    }))
}

fn score_input_json(report: &RunReport) -> String {
    json_array(report.episodes.iter().map(|episode| {
        format!(
            "{{\"exact_predictions\":{},\"label\":\"{}\",\"prediction_attempts\":{},\"task_success\":{},\"terminal\":\"{}\"}}",
            episode.exact_predictions,
            json_escape(&episode.label),
            episode.prediction_attempts,
            episode.task_success,
            terminal_name(episode.terminal),
        )
    }))
}

fn support_context_count(
    ledger: &InteractionLedger,
    candidate: &Candidate,
    support_ids: &[EvidenceId],
) -> u64 {
    let positions = candidate.input_pattern().variable_positions();
    let mut values = BTreeSet::new();
    for interaction in ledger.interactions() {
        if support_ids.contains(&interaction.evidence().evidence_id()) {
            values.insert(
                positions
                    .iter()
                    .filter_map(|position| interaction.evidence().before().atom(*position))
                    .map(|atom| atom.value())
                    .collect::<Vec<_>>(),
            );
        }
    }
    values.len() as u64
}

fn candidate_event_json(kind: &str, candidate: &Candidate) -> String {
    format!(
        "{{\"action\":\"{}\",\"changed_positions\":{},\"event\":\"{}\",\"grounding_evidence_ids\":{},\"hypothesis_id\":\"{}\",\"signature\":\"{}\",\"stage\":\"{}\",\"support_evidence_ids\":{},\"scope\":\"{}\"}}",
        candidate.action(),
        usize_array_json(candidate.changed_positions()),
        kind,
        evidence_ids_json(candidate.grounding_evidence_ids()),
        candidate.hypothesis_id(),
        json_escape(candidate.canonical_signature()),
        candidate.stage().code(),
        evidence_ids_json(candidate.support_evidence_ids()),
        candidate.scope().token(),
    )
}

fn admission_event_json(kind: &str, candidate: &Candidate, reasons: &[RejectionReason]) -> String {
    format!(
        "{{\"event\":\"{}\",\"reasons\":{},\"signature\":\"{}\",\"stage\":\"{}\",\"support_evidence_ids\":{}}}",
        kind,
        json_array(reasons.iter().map(|reason| format!("\"{}\"", rejection_name(*reason)))),
        json_escape(candidate.canonical_signature()),
        candidate.stage().code(),
        evidence_ids_json(candidate.support_evidence_ids()),
    )
}

fn procedure_json(procedure: &ProcedureRecord) -> String {
    format!(
        "{{\"expansion_signature\":\"{}\",\"hypothesis_id\":\"{}\",\"procedure_id\":\"{}\",\"scope\":\"{}\",\"status\":\"{}\",\"support_evidence_ids\":{},\"version\":{}}}",
        json_escape(procedure.expansion().canonical_signature()),
        procedure.hypothesis_id(),
        procedure.procedure_id(),
        procedure.scope().token(),
        procedure_status_name(procedure.status()),
        evidence_ids_json(procedure.support_evidence_ids()),
        procedure.version(),
    )
}

fn revision_json(revision: &ProcedureRevision) -> String {
    format!(
        "{{\"contradiction_evidence_ids\":{},\"expansion_signature\":\"{}\",\"procedure_id\":\"{}\",\"status\":\"{}\",\"version\":{}}}",
        evidence_ids_json(revision.contradiction_evidence_ids()),
        json_escape(revision.expansion().canonical_signature()),
        revision.procedure_id(),
        procedure_status_name(revision.status()),
        revision.version(),
    )
}

fn schedule_fingerprint() -> String {
    let schedule = fixed_schedule()
        .iter()
        .map(|spec| {
            format!(
                "{}|{}|{}|{}\n",
                spec.order(),
                spec.label(),
                partition_name(spec.partition()),
                role_name(spec.role()),
            )
        })
        .collect::<String>();
    sha256_hex(schedule.as_bytes())
}

fn canonical_scale_sweep_json() -> String {
    let points =
        deterministic_scale_points(&[6, 16, 64], &[12, 36, 128], &[8, 64, 512], &[1, 8, 32]);
    // Scale points are a deterministic measurement plan. Runtime samples are
    // kept in ResourceMeter reports; default snapshots avoid putting monotonic
    // timing into deterministic-core files.
    ScaleSweep::from_points(&points).to_json()
}

fn partition_name(partition: Partition) -> &'static str {
    match partition {
        Partition::Training => "training",
        Partition::Development => "development",
        Partition::Final => "final",
    }
}

fn role_name(role: EpisodeRole) -> &'static str {
    match role {
        EpisodeRole::DiscoverySupport => "discovery-support",
        EpisodeRole::PreconditionChallenge => "precondition-challenge",
        EpisodeRole::RepeatedSupport => "repeated-support",
        EpisodeRole::RepeatedChallenge => "repeated-challenge",
        EpisodeRole::ApplicableChallenge => "applicable-challenge",
        EpisodeRole::NonApplicableChallenge => "non-applicable-challenge",
        EpisodeRole::RetentionCheckBeforeFreeze => "retention-check-before-freeze",
        EpisodeRole::RetentionChallengeBeforeFreeze => "retention-challenge-before-freeze",
        EpisodeRole::HeldOutApplicable => "held-out-applicable",
        EpisodeRole::HeldOutNonApplicable => "held-out-non-applicable",
        EpisodeRole::FrozenRetentionApplicable => "frozen-retention-applicable",
        EpisodeRole::FrozenRetentionNonApplicable => "frozen-retention-non-applicable",
    }
}

fn terminal_name(status: TerminalStatus) -> &'static str {
    match status {
        TerminalStatus::Continue => "continue",
        TerminalStatus::Success => "success",
        TerminalStatus::Failure => "failure",
        TerminalStatus::Incomplete => "incomplete",
    }
}

fn decision_reason_name(reason: DecisionReason) -> &'static str {
    match reason {
        DecisionReason::Procedure => "procedure",
        DecisionReason::Conflict => "conflict",
        DecisionReason::HypothesisDisagreement => "hypothesis_disagreement",
        DecisionReason::Fallback => "fallback",
        DecisionReason::IncompleteBudget => "incomplete_budget",
    }
}

fn procedure_status_name(status: ProcedureStatus) -> &'static str {
    match status {
        ProcedureStatus::Active => "active",
        ProcedureStatus::Suspended => "suspended",
    }
}

fn rejection_name(reason: RejectionReason) -> &'static str {
    match reason {
        RejectionReason::UnsupportedByGrammar => "unsupported_by_grammar",
        RejectionReason::ActionMismatch => "action_mismatch",
        RejectionReason::UnstableEffect => "unstable_effect",
        RejectionReason::InsufficientSupport => "insufficient_support",
        RejectionReason::MissingContextVariation => "missing_context_variation",
        RejectionReason::OutputInconsistency => "output_inconsistency",
        RejectionReason::MissingStructuralTraceability => "missing_structural_traceability",
        RejectionReason::MissingChallenge => "missing_challenge",
        RejectionReason::IncorrectChallenge => "incorrect_challenge",
        RejectionReason::MissingGroundingEvidence => "missing_grounding_evidence",
        RejectionReason::ScopeConfinement => "scope_confinement",
        RejectionReason::CostCeiling => "cost_ceiling",
        RejectionReason::MissingEvidence => "missing_evidence",
        RejectionReason::Contradiction => "contradiction",
        RejectionReason::BudgetExhausted => "budget_exhausted",
    }
}

fn seeds_json(seeds: RunSeeds) -> String {
    format!(
        "{{\"random_baseline_seed\":{},\"scope_seed\":{},\"world_seed\":{}}}",
        seeds.random_baseline_seed, seeds.scope_seed, seeds.world_seed
    )
}

fn evidence_json(evidence: &super::interaction_ledger::EvidenceRecord) -> String {
    format!(
        "{{\"action\":{},\"after\":{},\"before\":{},\"decision_sequence\":{},\"episode_scope\":\"{}\",\"evidence_id\":\"{}\",\"provenance\":\"{}\",\"run_scope\":\"{}\",\"step_index\":{},\"terminal\":\"{}\"}}",
        evidence.action().map(|action| action.to_string()).unwrap_or_else(|| "null".to_owned()),
        evidence.after().map(packet_json).unwrap_or_else(|| "null".to_owned()),
        packet_json(evidence.before()),
        evidence.decision_sequence(),
        evidence.episode_scope(),
        evidence.evidence_id(),
        match evidence.provenance() {
            EvidenceProvenance::Observed => "observed",
            EvidenceProvenance::Incomplete => "incomplete",
        },
        evidence.run_scope(),
        evidence.step_index(),
        terminal_name(evidence.terminal()),
    )
}

fn step_result_json(result: StepResult) -> String {
    format!(
        "{{\"observation\":{},\"terminal\":\"{}\"}}",
        packet_json(result.observation()),
        terminal_name(result.terminal()),
    )
}

fn input_json(input: StepInput) -> String {
    let [first, second] = input.available_actions();
    format!(
        "{{\"available_actions\":[\"{}\",\"{}\"],\"episode_scope\":\"{}\",\"observation\":{},\"previous_terminal\":\"{}\",\"remaining_action_budget\":{},\"run_scope\":\"{}\",\"step_index\":{}}}",
        first,
        second,
        input.episode_scope(),
        packet_json(input.observation()),
        match input.previous_terminal() {
            PreviousTerminal::None => "none",
            PreviousTerminal::Continue => "continue",
        },
        input.remaining_action_budget(),
        input.run_scope(),
        input.step_index(),
    )
}

fn packet_json(packet: ObservationPacket) -> String {
    let [a, b, c] = packet.atoms();
    format!("{{\"atoms\":[\"{}\",\"{}\",\"{}\"]}}", a, b, c)
}

fn decision_json(decision: &Decision) -> String {
    let audit = decision.audit();
    let procedures = opaque_ids_json(audit.procedure_refs().iter().map(|id| id.value()));
    let evidence = opaque_ids_json(audit.evidence_refs().iter().map(|id| id.value()));
    let hypotheses = opaque_ids_json(audit.hypothesis_refs().iter().map(|id| id.value()));
    match decision {
        Decision::Act(act) => format!(
            "{{\"action\":\"{}\",\"evidence_refs\":{},\"hypothesis_refs\":{},\"mode\":\"{}\",\"prediction\":{},\"procedure_refs\":{},\"type\":\"act\"}}",
            act.action(),
            evidence,
            hypotheses,
            match act.mode() {
                ActMode::Exploration => "exploration",
                ActMode::Procedure => "procedure",
                ActMode::Fallback => "fallback",
            },
            act.prediction().map(packet_json).unwrap_or_else(|| "null".to_owned()),
            procedures,
        ),
        Decision::Abstain(abstain) => format!(
            "{{\"evidence_refs\":{},\"hypothesis_refs\":{},\"procedure_refs\":{},\"reason\":\"{}\",\"type\":\"abstain\"}}",
            evidence,
            hypotheses,
            procedures,
            match abstain.reason() {
                AbstainReason::NoApplicableProcedure => "no_applicable_procedure",
                AbstainReason::Conflict => "conflict",
                AbstainReason::InsufficientEvidence => "insufficient_evidence",
                AbstainReason::UnsupportedByGrammar => "unsupported_by_grammar",
                AbstainReason::IncompleteBudget => "incomplete_budget",
            },
        ),
    }
}

fn logical_cost_json(cost: LogicalCost) -> String {
    format!(
        "{{\"environment_actions\":{},\"operation_steps\":{},\"query_calls\":{},\"structural_comparisons\":{}}}",
        cost.environment_actions,
        cost.operation_steps,
        cost.query_calls,
        cost.structural_comparisons,
    )
}

fn left_value(counters: super::resource_scale::LogicalCounters, resource: LogicalResource) -> u64 {
    match resource {
        LogicalResource::QueryCalls => counters.query_calls(),
        LogicalResource::EnvironmentActions => counters.environment_actions(),
        LogicalResource::CandidateGenerationAttempts => counters.candidate_generation_attempts(),
        LogicalResource::StructuralComparisons => counters.structural_comparisons(),
        LogicalResource::ActiveHypotheses => counters.active_hypotheses(),
        LogicalResource::OperationSteps => counters.operation_steps(),
        LogicalResource::RetainedInteractionRecords => counters.retained_interaction_records(),
        LogicalResource::RetainedProcedureRecords => counters.retained_procedure_records(),
    }
}

fn discovery_json(discovery: DiscoveryMetrics) -> String {
    format!(
        "{{\"admitted_e3_procedures\":{},\"e0_only_credit\":{},\"grounding_evidence_count\":{},\"support_context_atoms\":{},\"support_interactions\":{}}}",
        discovery.admitted_e3_procedures,
        discovery.e0_only_credit,
        discovery.grounding_evidence_count,
        discovery.support_context_atoms,
        discovery.support_interactions,
    )
}

fn stage_operations_json(operations: StageOperations) -> String {
    format!(
        "{{\"admission\":{},\"canonicalization\":{},\"candidate_generation\":{},\"decision_action\":{},\"revalidation\":{},\"retrieval\":{}}}",
        operations.admission,
        operations.canonicalization,
        operations.candidate_generation,
        operations.decision_action,
        operations.revalidation,
        operations.retrieval,
    )
}

fn memory_value(sample: MemorySample) -> String {
    match sample {
        MemorySample::Missing => "null".to_owned(),
        MemorySample::Measured(value) => value.to_string(),
    }
}

fn evidence_ids_json(ids: &[EvidenceId]) -> String {
    opaque_ids_json(ids.iter().map(|id| id.value()))
}

fn opaque_ids_json<I>(ids: I) -> String
where
    I: IntoIterator<Item = u64>,
{
    json_array(ids.into_iter().map(|id| format!("\"{id}\"")))
}

fn u64_array_json(values: &[u64]) -> String {
    json_array(values.iter().map(|value| value.to_string()))
}

fn usize_array_json(values: &[usize]) -> String {
    json_array(values.iter().map(|value| value.to_string()))
}

fn jsonl<I>(values: I) -> String
where
    I: IntoIterator<Item = String>,
{
    let mut output = values.into_iter().collect::<Vec<_>>().join("\n");
    output.push('\n');
    output
}

fn json_array<I>(values: I) -> String
where
    I: IntoIterator<Item = String>,
{
    format!("[{}]", values.into_iter().collect::<Vec<_>>().join(","))
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Small standard-library-only SHA-256 implementation for artifact hashes.
fn sha256_hex(bytes: &[u8]) -> String {
    let digest = sha256(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn sha256(input: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut data = input.to_vec();
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&(input.len() as u64 * 8).to_be_bytes());
    let mut hash = [
        0x6a09e667_u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    for chunk in data.chunks_exact(64) {
        let mut schedule = [0_u32; 64];
        for (index, word) in schedule[..16].iter_mut().enumerate() {
            *word = u32::from_be_bytes([
                chunk[index * 4],
                chunk[index * 4 + 1],
                chunk[index * 4 + 2],
                chunk[index * 4 + 3],
            ]);
        }
        for index in 16..64 {
            let small_sigma0 = schedule[index - 15].rotate_right(7)
                ^ schedule[index - 15].rotate_right(18)
                ^ (schedule[index - 15] >> 3);
            let small_sigma1 = schedule[index - 2].rotate_right(17)
                ^ schedule[index - 2].rotate_right(19)
                ^ (schedule[index - 2] >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(small_sigma0)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(small_sigma1);
        }
        let mut working = hash;
        for index in 0..64 {
            let big_sigma1 = working[4].rotate_right(6)
                ^ working[4].rotate_right(11)
                ^ working[4].rotate_right(25);
            let choose = (working[4] & working[5]) ^ ((!working[4]) & working[6]);
            let temp1 = working[7]
                .wrapping_add(big_sigma1)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(schedule[index]);
            let big_sigma0 = working[0].rotate_right(2)
                ^ working[0].rotate_right(13)
                ^ working[0].rotate_right(22);
            let majority =
                (working[0] & working[1]) ^ (working[0] & working[2]) ^ (working[1] & working[2]);
            let temp2 = big_sigma0.wrapping_add(majority);
            working[7] = working[6];
            working[6] = working[5];
            working[5] = working[4];
            working[4] = working[3].wrapping_add(temp1);
            working[3] = working[2];
            working[2] = working[1];
            working[1] = working[0];
            working[0] = temp1.wrapping_add(temp2);
        }
        for index in 0..8 {
            hash[index] = hash[index].wrapping_add(working[index]);
        }
    }
    let mut output = [0_u8; 32];
    for (index, word) in hash.iter().enumerate() {
        output[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_episode(order: usize, role: &str) -> EpisodeMetric {
        EpisodeMetric {
            order,
            label: format!("F{order:02}"),
            partition: "final".to_owned(),
            role: role.to_owned(),
            episode_scope: ScopeToken::new(order as u64),
            decision_sequences: Vec::new(),
            action_count: 0,
            terminal: TerminalStatus::Failure,
            task_success: false,
            prediction_attempts: 0,
            exact_predictions: 0,
            procedure_backed_actions: 0,
            procedure_backed_predictions: 0,
            abstention_count: 0,
            incomplete_count: 0,
            false_positive: false,
            retention_pass: false,
            grounding_evidence_count: 0,
        }
    }

    #[test]
    fn zero_applicable_prediction_attempts_still_use_protocol_denominator() {
        let mut episodes = (1..=HELD_OUT_APPLICABLE_COUNT as usize)
            .map(|order| empty_episode(order, "held-out-applicable"))
            .collect::<Vec<_>>();
        episodes.extend((7..=12).map(|order| empty_episode(order, "held-out-non-applicable")));
        episodes.extend((13..=14).map(|order| empty_episode(order, "frozen-retention-applicable")));
        episodes
            .extend((15..=16).map(|order| empty_episode(order, "frozen-retention-non-applicable")));

        let metrics = Metrics::from_episodes(episodes, DiscoveryMetrics::default())
            .expect("fixed protocol denominators must make zero attempts representable");

        assert_eq!(metrics.held_out_applicable_prediction_attempts, 0);
        assert_eq!(metrics.held_out_applicable_packet_accuracy.numerator(), 0);
        assert_eq!(metrics.held_out_applicable_packet_accuracy.denominator(), 6);
        assert_eq!(
            metrics
                .held_out_applicable_prediction_coverage
                .denominator(),
            6
        );
        assert_eq!(metrics.held_out_applicable_task_success.denominator(), 6);
        assert_eq!(
            metrics
                .held_out_non_applicable_false_positives
                .denominator(),
            6
        );
        assert_eq!(
            metrics
                .held_out_non_applicable_abstention_quality
                .denominator(),
            6
        );
        assert_eq!(metrics.frozen_retention.denominator(), 4);
    }
}
