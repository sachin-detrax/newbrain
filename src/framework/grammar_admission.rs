//! Reusable bounded, opaque-only grammar and admission seam.
//!
//! This module consumes [`InteractionRecord`] values already owned by the
//! learner ledger.  It intentionally has no dependency on the evaluator world:
//! atoms, actions, scopes, and evidence identifiers are compared exactly and
//! are never interpreted.  The only structural generalisation is one bounded
//! equality variable, introduced by E2 from a repeated input value. The v2
//! grammar version and deterministic limits are retained for compatibility.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::interaction_ledger::{
    AuditScope, EvidenceProvenance, InteractionLedger, InteractionRecord, LedgerError, RunPhase,
};
use super::opaque_protocol::{
    ActMode, ActionId, AtomId, Decision, EvidenceId, HypothesisId, ObservationPacket, ProcedureId,
    ScopeToken, OBSERVATION_PACKET_LEN,
};
use super::resource_scale::{
    LogicalResource, MeasurementError, ResourceMeter, Stage, V2_GRAMMAR_VERSION,
};

pub const GRAMMAR_VERSION: &str = V2_GRAMMAR_VERSION;
pub const MAX_VARIABLES: usize = 1;

/// The only candidate stages admitted by the frozen v2 grammar.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CandidateStage {
    E0,
    E1,
    E2,
    E3,
    E4,
}

impl CandidateStage {
    pub const E0_EXACT_REPLAY: Self = Self::E0;
    pub const E1_ALIGNED_EFFECT: Self = Self::E1;
    pub const E2_REPEATED_ROLE: Self = Self::E2;
    pub const E3_APPLICABILITY_GUARD: Self = Self::E3;
    pub const E4_RUN_SCOPE_PROMOTION: Self = Self::E4;

    pub const fn code(self) -> &'static str {
        match self {
            Self::E0 => "E0",
            Self::E1 => "E1",
            Self::E2 => "E2",
            Self::E3 => "E3",
            Self::E4 => "E4",
        }
    }

    const fn edit_count(self) -> u8 {
        match self {
            Self::E0 | Self::E1 => 0,
            Self::E2 | Self::E3 | Self::E4 => 1,
        }
    }
}

/// A bounded variable slot.  v2 has exactly one possible variable slot.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VariableId(u8);

impl VariableId {
    pub const ZERO: Self = Self(0);

    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PatternAtom {
    Constant(AtomId),
    Variable(VariableId),
}

/// A fixed-length input pattern.  There is no wildcard or unbounded predicate.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct InputPattern {
    atoms: [PatternAtom; OBSERVATION_PACKET_LEN],
}

impl InputPattern {
    pub const fn new(atoms: [PatternAtom; OBSERVATION_PACKET_LEN]) -> Self {
        Self { atoms }
    }

    pub const fn atoms(self) -> [PatternAtom; OBSERVATION_PACKET_LEN] {
        self.atoms
    }

    pub fn as_slice(&self) -> &[PatternAtom; OBSERVATION_PACKET_LEN] {
        &self.atoms
    }

    pub fn variable_positions(&self) -> Vec<usize> {
        self.atoms
            .iter()
            .enumerate()
            .filter_map(|(index, atom)| matches!(atom, PatternAtom::Variable(_)).then_some(index))
            .collect()
    }

    fn matches(&self, packet: ObservationPacket) -> bool {
        let mut binding: Option<AtomId> = None;
        for (index, pattern) in self.atoms.iter().enumerate() {
            let Some(atom) = packet.atom(index) else {
                return false;
            };
            match pattern {
                PatternAtom::Constant(expected) if *expected != atom => return false,
                PatternAtom::Constant(_) => {}
                PatternAtom::Variable(variable) if variable.value() == VariableId::ZERO.value() => {
                    if let Some(previous) = binding {
                        if previous != atom {
                            return false;
                        }
                    } else {
                        binding = Some(atom);
                    }
                }
                PatternAtom::Variable(_) => return false,
            }
        }
        true
    }

    fn is_bounded(&self) -> bool {
        self.atoms.iter().all(|atom| match atom {
            PatternAtom::Constant(_) => true,
            PatternAtom::Variable(variable) => variable.value() == VariableId::ZERO.value(),
        })
    }
}

/// Output construction has only two operations: copy an input position or
/// replay an atom that was actually observed in every supporting result.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum OutputOp {
    Copy(usize),
    Observed(AtomId),
}

impl OutputOp {
    pub const fn copy(position: usize) -> Self {
        Self::Copy(position)
    }

    pub const fn observed(atom: AtomId) -> Self {
        Self::Observed(atom)
    }

    pub const fn observed_atom(atom: AtomId) -> Self {
        Self::Observed(atom)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EqualityRelation {
    left: usize,
    right: usize,
}

impl EqualityRelation {
    pub const fn new(left: usize, right: usize) -> Self {
        Self { left, right }
    }

    pub const fn left(self) -> usize {
        self.left
    }

    pub const fn right(self) -> usize {
        self.right
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CandidateScope {
    Episode(ScopeToken),
    Run(ScopeToken),
}

impl CandidateScope {
    pub const fn token(self) -> ScopeToken {
        match self {
            Self::Episode(token) | Self::Run(token) => token,
        }
    }

    pub const fn is_run(self) -> bool {
        matches!(self, Self::Run(_))
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CandidateCosts {
    generation_attempts: u64,
    structural_comparisons: u64,
    operation_steps: u64,
    challenge_comparisons: u64,
}

impl CandidateCosts {
    pub const fn new(
        generation_attempts: u64,
        structural_comparisons: u64,
        operation_steps: u64,
        challenge_comparisons: u64,
    ) -> Self {
        Self {
            generation_attempts,
            structural_comparisons,
            operation_steps,
            challenge_comparisons,
        }
    }

    pub const fn generation_attempts(self) -> u64 {
        self.generation_attempts
    }

    pub const fn structural_comparisons(self) -> u64 {
        self.structural_comparisons
    }

    pub const fn operation_steps(self) -> u64 {
        self.operation_steps
    }

    pub const fn challenge_comparisons(self) -> u64 {
        self.challenge_comparisons
    }

    fn plus(self, other: Self) -> Self {
        Self {
            generation_attempts: self
                .generation_attempts
                .saturating_add(other.generation_attempts),
            structural_comparisons: self
                .structural_comparisons
                .saturating_add(other.structural_comparisons),
            operation_steps: self.operation_steps.saturating_add(other.operation_steps),
            challenge_comparisons: self
                .challenge_comparisons
                .saturating_add(other.challenge_comparisons),
        }
    }

    fn with_challenge(self, comparisons: u64) -> Self {
        Self {
            challenge_comparisons: self.challenge_comparisons.saturating_add(comparisons),
            ..self
        }
    }

    fn within_protocol_limits(self, meter: &ResourceMeter) -> bool {
        self.generation_attempts
            <= meter
                .limits()
                .logical(LogicalResource::CandidateGenerationAttempts)
                .per_decision()
            && self.structural_comparisons
                <= meter
                    .limits()
                    .logical(LogicalResource::StructuralComparisons)
                    .per_decision()
            && self.operation_steps
                <= meter
                    .limits()
                    .logical(LogicalResource::OperationSteps)
                    .per_decision()
            && self.challenge_comparisons
                <= meter
                    .limits()
                    .logical(LogicalResource::StructuralComparisons)
                    .per_decision()
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CandidateOrder {
    stage: CandidateStage,
    structural_edit_count: u8,
    first_support: EvidenceId,
    action: ActionId,
    position_order: Vec<usize>,
    exact_identifier_order: Vec<u64>,
}

impl CandidateOrder {
    pub fn stage(&self) -> CandidateStage {
        self.stage
    }

    pub const fn structural_edit_count(&self) -> u8 {
        self.structural_edit_count
    }

    pub const fn first_support(&self) -> EvidenceId {
        self.first_support
    }

    pub const fn action(&self) -> ActionId {
        self.action
    }

    pub fn position_order(&self) -> &[usize] {
        &self.position_order
    }

    pub fn exact_identifier_order(&self) -> &[u64] {
        &self.exact_identifier_order
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Candidate {
    stage: CandidateStage,
    run_scope: ScopeToken,
    scope: CandidateScope,
    action: ActionId,
    packet_length: usize,
    input: InputPattern,
    output: [OutputOp; OBSERVATION_PACKET_LEN],
    unchanged_positions: Vec<usize>,
    changed_positions: Vec<usize>,
    input_equalities: Vec<EqualityRelation>,
    output_equalities: Vec<EqualityRelation>,
    precondition: Vec<(usize, AtomId)>,
    support_evidence_ids: Vec<EvidenceId>,
    challenge_evidence_ids: Vec<EvidenceId>,
    grounding_evidence_ids: Vec<EvidenceId>,
    rejection_evidence_ids: Vec<EvidenceId>,
    hypothesis_id: HypothesisId,
    procedure_id: Option<ProcedureId>,
    rejection_reasons: Vec<RejectionReason>,
    costs: CandidateCosts,
    signature: String,
}

impl Candidate {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_persisted(
        stage: CandidateStage,
        run_scope: ScopeToken,
        scope: CandidateScope,
        action: ActionId,
        packet_length: usize,
        input: InputPattern,
        output: [OutputOp; OBSERVATION_PACKET_LEN],
        unchanged_positions: Vec<usize>,
        changed_positions: Vec<usize>,
        input_equalities: Vec<EqualityRelation>,
        output_equalities: Vec<EqualityRelation>,
        precondition: Vec<(usize, AtomId)>,
        support_evidence_ids: Vec<EvidenceId>,
        challenge_evidence_ids: Vec<EvidenceId>,
        grounding_evidence_ids: Vec<EvidenceId>,
        rejection_evidence_ids: Vec<EvidenceId>,
        hypothesis_id: HypothesisId,
        procedure_id: Option<ProcedureId>,
        rejection_reasons: Vec<RejectionReason>,
        costs: CandidateCosts,
        signature: String,
    ) -> Option<Self> {
        let mut candidate = Self {
            stage,
            run_scope,
            scope,
            action,
            packet_length,
            input,
            output,
            unchanged_positions,
            changed_positions,
            input_equalities,
            output_equalities,
            precondition,
            support_evidence_ids,
            challenge_evidence_ids,
            grounding_evidence_ids,
            rejection_evidence_ids,
            hypothesis_id,
            procedure_id,
            rejection_reasons,
            costs,
            signature,
        };
        let persisted_signature = candidate.signature.clone();
        candidate.normalize();
        candidate.refresh_signature();
        (candidate.signature == persisted_signature).then_some(candidate)
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        stage: CandidateStage,
        run_scope: ScopeToken,
        scope: CandidateScope,
        action: ActionId,
        input: InputPattern,
        output: [OutputOp; OBSERVATION_PACKET_LEN],
        unchanged_positions: Vec<usize>,
        changed_positions: Vec<usize>,
        input_equalities: Vec<EqualityRelation>,
        output_equalities: Vec<EqualityRelation>,
        precondition: Vec<(usize, AtomId)>,
        support_evidence_ids: Vec<EvidenceId>,
        challenge_evidence_ids: Vec<EvidenceId>,
        hypothesis_id: HypothesisId,
        costs: CandidateCosts,
    ) -> Self {
        let mut candidate = Self {
            stage,
            run_scope,
            scope,
            action,
            packet_length: OBSERVATION_PACKET_LEN,
            input,
            output,
            unchanged_positions,
            changed_positions,
            input_equalities,
            output_equalities,
            precondition,
            support_evidence_ids,
            challenge_evidence_ids,
            grounding_evidence_ids: Vec::new(),
            rejection_evidence_ids: Vec::new(),
            hypothesis_id,
            procedure_id: None,
            rejection_reasons: Vec::new(),
            costs,
            signature: String::new(),
        };
        candidate.normalize();
        candidate.refresh_signature();
        candidate
    }

    fn normalize(&mut self) {
        self.unchanged_positions.sort_unstable();
        self.changed_positions.sort_unstable();
        self.input_equalities.sort_unstable();
        self.output_equalities.sort_unstable();
        self.precondition
            .sort_unstable_by_key(|(position, _)| *position);
        self.support_evidence_ids.sort_unstable();
        self.support_evidence_ids.dedup();
        self.challenge_evidence_ids.sort_unstable();
        self.challenge_evidence_ids.dedup();
        self.grounding_evidence_ids.sort_unstable();
        self.grounding_evidence_ids.dedup();
        self.rejection_evidence_ids.sort_unstable();
        self.rejection_evidence_ids.dedup();
        self.rejection_reasons.sort_unstable();
        self.rejection_reasons.dedup();
    }

    fn refresh_signature(&mut self) {
        self.signature = signature_for(self, true);
    }

    pub const fn stage(&self) -> CandidateStage {
        self.stage
    }

    pub const fn run_scope(&self) -> ScopeToken {
        self.run_scope
    }

    pub const fn scope(&self) -> CandidateScope {
        self.scope
    }

    pub const fn action(&self) -> ActionId {
        self.action
    }

    pub const fn packet_length(&self) -> usize {
        self.packet_length
    }

    pub const fn input_pattern(&self) -> InputPattern {
        self.input
    }

    pub fn output_operations(&self) -> &[OutputOp; OBSERVATION_PACKET_LEN] {
        &self.output
    }

    pub fn unchanged_positions(&self) -> &[usize] {
        &self.unchanged_positions
    }

    pub fn changed_positions(&self) -> &[usize] {
        &self.changed_positions
    }

    pub fn input_equality_relations(&self) -> &[EqualityRelation] {
        &self.input_equalities
    }

    pub fn output_equality_relations(&self) -> &[EqualityRelation] {
        &self.output_equalities
    }

    pub fn precondition(&self) -> &[(usize, AtomId)] {
        &self.precondition
    }

    pub fn support_evidence_ids(&self) -> &[EvidenceId] {
        &self.support_evidence_ids
    }

    pub fn challenge_evidence_ids(&self) -> &[EvidenceId] {
        &self.challenge_evidence_ids
    }

    pub fn grounding_evidence_ids(&self) -> &[EvidenceId] {
        &self.grounding_evidence_ids
    }

    pub fn rejection_evidence_ids(&self) -> &[EvidenceId] {
        &self.rejection_evidence_ids
    }

    pub const fn hypothesis_id(&self) -> HypothesisId {
        self.hypothesis_id
    }

    pub const fn procedure_id(&self) -> Option<ProcedureId> {
        self.procedure_id
    }

    pub fn rejection_reasons(&self) -> &[RejectionReason] {
        &self.rejection_reasons
    }

    pub const fn costs(&self) -> CandidateCosts {
        self.costs
    }

    pub fn canonical_signature(&self) -> &str {
        &self.signature
    }

    pub fn canonical_order(&self) -> CandidateOrder {
        let first_support = self
            .support_evidence_ids
            .first()
            .copied()
            .unwrap_or(EvidenceId::new(u64::MAX));
        let mut position_order = self.changed_positions.clone();
        position_order.extend(self.unchanged_positions.iter().copied());
        position_order.sort_unstable();
        let mut exact_identifier_order = self
            .support_evidence_ids
            .iter()
            .map(|id| id.value())
            .collect::<Vec<_>>();
        exact_identifier_order.push(self.action.value());
        exact_identifier_order.sort_unstable();
        CandidateOrder {
            stage: self.stage,
            structural_edit_count: self.stage.edit_count(),
            first_support,
            action: self.action,
            position_order,
            exact_identifier_order,
        }
    }

    /// Predict only when the exact action and the bounded pattern apply.
    pub fn predict(&self, input: ObservationPacket, action: ActionId) -> Option<ObservationPacket> {
        if self.action != action || !self.input.matches(input) {
            return None;
        }
        let mut atoms = [AtomId::new(0); OBSERVATION_PACKET_LEN];
        for (index, operation) in self.output.iter().enumerate() {
            atoms[index] = match operation {
                OutputOp::Copy(position) => input.atom(*position)?,
                OutputOp::Observed(atom) => *atom,
            };
        }
        Some(ObservationPacket::new(atoms))
    }

    /// Return a new candidate with a changed opaque output operation.  Admission
    /// still verifies that every observed atom is grounded by every support;
    /// this makes output-invention rejection explicit rather than trusting a
    /// caller-provided candidate.
    pub fn with_output_operations(&self, output: [OutputOp; OBSERVATION_PACKET_LEN]) -> Self {
        let mut candidate = self.clone();
        candidate.output = output;
        candidate.refresh_signature();
        candidate
    }

    /// Rebind a candidate to an application-owned hypothesis identity while
    /// retaining its opaque structural evidence. This is used by the MVP demo
    /// to persist an explicit competing hypothesis alongside the generated
    /// candidates; the rewritten value remains a separate immutable candidate.
    pub fn with_hypothesis_id(&self, hypothesis_id: HypothesisId) -> Self {
        let mut candidate = self.clone();
        candidate.hypothesis_id = hypothesis_id;
        candidate.refresh_signature();
        candidate
    }

    /// A scope rewrite creates a separate candidate value.  Admission rejects
    /// it unless the rewritten scope remains inside the current run.
    pub fn with_scope(&self, run_scope: ScopeToken, scope: CandidateScope) -> Self {
        let mut candidate = self.clone();
        candidate.run_scope = run_scope;
        candidate.scope = scope;
        candidate.refresh_signature();
        candidate
    }

    fn with_stage_and_procedure(
        &self,
        stage: CandidateStage,
        scope: CandidateScope,
        procedure_id: ProcedureId,
        costs: CandidateCosts,
    ) -> Self {
        let mut candidate = self.clone();
        candidate.stage = stage;
        candidate.scope = scope;
        candidate.procedure_id = Some(procedure_id);
        candidate.costs = costs;
        candidate.refresh_signature();
        candidate
    }

    fn with_rejection(&self, reasons: Vec<RejectionReason>, evidence_ids: Vec<EvidenceId>) -> Self {
        let mut candidate = self.clone();
        candidate.rejection_reasons = reasons;
        candidate.rejection_evidence_ids = evidence_ids;
        candidate.normalize();
        candidate.refresh_signature();
        candidate
    }

    fn with_challenge_ids(&self, challenge_ids: Vec<EvidenceId>, costs: CandidateCosts) -> Self {
        let mut candidate = self.clone();
        candidate.stage = CandidateStage::E3;
        candidate.challenge_evidence_ids = challenge_ids;
        candidate.costs = costs;
        candidate.normalize();
        candidate.refresh_signature();
        candidate
    }

    fn with_revision(
        &self,
        reasons: Vec<RejectionReason>,
        contradiction_ids: Vec<EvidenceId>,
    ) -> Self {
        self.with_rejection(reasons, contradiction_ids)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RejectionReason {
    UnsupportedByGrammar,
    ActionMismatch,
    UnstableEffect,
    InsufficientSupport,
    MissingContextVariation,
    OutputInconsistency,
    MissingStructuralTraceability,
    MissingChallenge,
    IncorrectChallenge,
    MissingGroundingEvidence,
    ScopeConfinement,
    CostCeiling,
    MissingEvidence,
    Contradiction,
    BudgetExhausted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RejectionRecord {
    stage: CandidateStage,
    signature: Option<String>,
    evidence_ids: Vec<EvidenceId>,
    reasons: Vec<RejectionReason>,
    costs: CandidateCosts,
}

impl RejectionRecord {
    pub const fn stage(&self) -> CandidateStage {
        self.stage
    }

    pub fn signature(&self) -> Option<&str> {
        self.signature.as_deref()
    }

    pub fn evidence_ids(&self) -> &[EvidenceId] {
        &self.evidence_ids
    }

    pub fn reasons(&self) -> &[RejectionReason] {
        &self.reasons
    }

    pub const fn costs(&self) -> CandidateCosts {
        self.costs
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationStatus {
    Complete,
    UnsupportedByGrammar,
    Incomplete {
        stage: CandidateStage,
        resource: LogicalResource,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationReport {
    candidates: Vec<Candidate>,
    rejections: Vec<RejectionRecord>,
    status: GenerationStatus,
}

impl GenerationReport {
    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
    }

    pub fn rejections(&self) -> &[RejectionRecord] {
        &self.rejections
    }

    pub const fn status(&self) -> GenerationStatus {
        self.status
    }

    pub fn candidates_at(&self, stage: CandidateStage) -> Vec<&Candidate> {
        self.candidates
            .iter()
            .filter(|candidate| candidate.stage == stage)
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionGate {
    Support,
    Variation,
    Consistency,
    StructuralTraceability,
    NonApplicabilityChallenge,
    GroundingEvidence,
    ScopeConfinement,
    CostCeiling,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GateResult {
    gate: AdmissionGate,
    passed: bool,
}

impl GateResult {
    pub const fn gate(self) -> AdmissionGate {
        self.gate
    }

    pub const fn passed(self) -> bool {
        self.passed
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AdmissionCosts {
    gate_checks: u64,
    structural_comparisons: u64,
    operation_steps: u64,
}

impl AdmissionCosts {
    pub const fn gate_checks(self) -> u64 {
        self.gate_checks
    }

    pub const fn structural_comparisons(self) -> u64 {
        self.structural_comparisons
    }

    pub const fn operation_steps(self) -> u64 {
        self.operation_steps
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedCandidate {
    candidate: Candidate,
    gates: Vec<GateResult>,
    grounding_evidence_ids: Vec<EvidenceId>,
    costs: AdmissionCosts,
}

impl AdmittedCandidate {
    pub fn candidate(&self) -> &Candidate {
        &self.candidate
    }

    pub fn gates(&self) -> &[GateResult] {
        &self.gates
    }

    pub fn grounding_evidence_ids(&self) -> &[EvidenceId] {
        &self.grounding_evidence_ids
    }

    pub const fn costs(&self) -> AdmissionCosts {
        self.costs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RejectedCandidate {
    candidate: Candidate,
    gates: Vec<GateResult>,
    reasons: Vec<RejectionReason>,
    costs: AdmissionCosts,
}

impl RejectedCandidate {
    pub fn candidate(&self) -> &Candidate {
        &self.candidate
    }

    pub fn gates(&self) -> &[GateResult] {
        &self.gates
    }

    pub fn reasons(&self) -> &[RejectionReason] {
        &self.reasons
    }

    pub const fn costs(&self) -> AdmissionCosts {
        self.costs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncompleteAdmission {
    candidate: Candidate,
    reason: RejectionReason,
    costs: AdmissionCosts,
}

impl IncompleteAdmission {
    pub fn candidate(&self) -> &Candidate {
        &self.candidate
    }

    pub const fn reason(&self) -> RejectionReason {
        self.reason
    }

    pub const fn costs(&self) -> AdmissionCosts {
        self.costs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdmissionOutcome {
    Admitted(AdmittedCandidate),
    Rejected(RejectedCandidate),
    Incomplete(IncompleteAdmission),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcedureStatus {
    Active,
    Suspended,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionProvenance {
    source_signature: String,
    expansion: Candidate,
    support_evidence_ids: Vec<EvidenceId>,
    challenge_evidence_ids: Vec<EvidenceId>,
    rejection_evidence_ids: Vec<EvidenceId>,
    costs: CandidateCosts,
}

impl PromotionProvenance {
    pub(crate) fn from_persisted(
        source_signature: String,
        expansion: Candidate,
        support_evidence_ids: Vec<EvidenceId>,
        challenge_evidence_ids: Vec<EvidenceId>,
        rejection_evidence_ids: Vec<EvidenceId>,
        costs: CandidateCosts,
    ) -> Self {
        Self {
            source_signature,
            expansion,
            support_evidence_ids,
            challenge_evidence_ids,
            rejection_evidence_ids,
            costs,
        }
    }

    pub fn source_signature(&self) -> &str {
        &self.source_signature
    }

    pub fn expansion(&self) -> &Candidate {
        &self.expansion
    }

    pub fn support_evidence_ids(&self) -> &[EvidenceId] {
        &self.support_evidence_ids
    }

    pub fn challenge_evidence_ids(&self) -> &[EvidenceId] {
        &self.challenge_evidence_ids
    }

    pub fn rejection_evidence_ids(&self) -> &[EvidenceId] {
        &self.rejection_evidence_ids
    }

    pub const fn costs(&self) -> CandidateCosts {
        self.costs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcedureRecord {
    procedure_id: ProcedureId,
    version: u32,
    status: ProcedureStatus,
    scope: CandidateScope,
    hypothesis_id: HypothesisId,
    provenance: PromotionProvenance,
}

impl ProcedureRecord {
    pub(crate) fn matches_persisted_source(&self, source: &Candidate) -> bool {
        let mut source = source.clone();
        source.grounding_evidence_ids = self.expansion().grounding_evidence_ids.clone();
        self.provenance.source_signature == source.canonical_signature()
            && source.stage == CandidateStage::E3
            && self.expansion().stage == CandidateStage::E4
            && self.expansion()
                == &source.with_stage_and_procedure(
                    CandidateStage::E4,
                    self.scope,
                    self.procedure_id,
                    self.provenance.costs,
                )
    }

    pub(crate) fn from_persisted(
        procedure_id: ProcedureId,
        version: u32,
        status: ProcedureStatus,
        scope: CandidateScope,
        hypothesis_id: HypothesisId,
        provenance: PromotionProvenance,
    ) -> Self {
        Self {
            procedure_id,
            version,
            status,
            scope,
            hypothesis_id,
            provenance,
        }
    }

    pub const fn procedure_id(&self) -> ProcedureId {
        self.procedure_id
    }

    pub const fn version(&self) -> u32 {
        self.version
    }

    pub const fn status(&self) -> ProcedureStatus {
        self.status
    }

    pub const fn scope(&self) -> CandidateScope {
        self.scope
    }

    pub const fn hypothesis_id(&self) -> HypothesisId {
        self.hypothesis_id
    }

    pub fn provenance(&self) -> &PromotionProvenance {
        &self.provenance
    }

    pub fn expansion(&self) -> &Candidate {
        self.provenance.expansion()
    }

    pub fn support_evidence_ids(&self) -> &[EvidenceId] {
        self.provenance.support_evidence_ids()
    }

    pub fn challenge_evidence_ids(&self) -> &[EvidenceId] {
        self.provenance.challenge_evidence_ids()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcedureRevision {
    procedure_id: ProcedureId,
    version: u32,
    status: ProcedureStatus,
    expansion: Candidate,
    contradiction_evidence_ids: Vec<EvidenceId>,
    preserved_provenance: PromotionProvenance,
}

impl ProcedureRevision {
    pub(crate) fn matches_persisted_original(&self, original: &ProcedureRecord) -> bool {
        self.procedure_id == original.procedure_id
            && original.version.checked_add(1) == Some(self.version)
            && self.status == ProcedureStatus::Suspended
            && !self.contradiction_evidence_ids.is_empty()
            && self.preserved_provenance == original.provenance
            && self.expansion
                == original.expansion().with_revision(
                    vec![RejectionReason::Contradiction],
                    self.contradiction_evidence_ids.clone(),
                )
    }

    pub(crate) fn from_persisted(
        procedure_id: ProcedureId,
        version: u32,
        status: ProcedureStatus,
        expansion: Candidate,
        contradiction_evidence_ids: Vec<EvidenceId>,
        preserved_provenance: PromotionProvenance,
    ) -> Self {
        Self {
            procedure_id,
            version,
            status,
            expansion,
            contradiction_evidence_ids,
            preserved_provenance,
        }
    }

    pub const fn procedure_id(&self) -> ProcedureId {
        self.procedure_id
    }

    pub const fn version(&self) -> u32 {
        self.version
    }

    pub const fn status(&self) -> ProcedureStatus {
        self.status
    }

    pub fn expansion(&self) -> &Candidate {
        &self.expansion
    }

    pub fn contradiction_evidence_ids(&self) -> &[EvidenceId] {
        &self.contradiction_evidence_ids
    }

    pub fn preserved_provenance(&self) -> &PromotionProvenance {
        &self.preserved_provenance
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PromotionError {
    NotAnAdmittedE3,
    ScopeConfinement,
    Budget(MeasurementError),
    Ledger(LedgerError),
    CounterOverflow,
}

impl fmt::Display for PromotionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "v2 promotion: {self:?}")
    }
}

impl std::error::Error for PromotionError {}

impl From<MeasurementError> for PromotionError {
    fn from(error: MeasurementError) -> Self {
        Self::Budget(error)
    }
}

impl From<LedgerError> for PromotionError {
    fn from(error: LedgerError) -> Self {
        Self::Ledger(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RevisionOutcome {
    Unchanged,
    Suspended(Box<ProcedureRevision>),
    Incomplete(RejectionReason),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrammarError {
    ZeroRunScope,
}

impl fmt::Display for GrammarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "v2 grammar: {self:?}")
    }
}

impl std::error::Error for GrammarError {}

/// The learner-owned grammar/store.  IDs are allocated in canonical
/// enumeration order and are never derived from semantic labels.
pub struct GrammarV2 {
    run_scope: ScopeToken,
    meter: ResourceMeter,
    next_hypothesis: u64,
    next_procedure: u64,
    procedures: Vec<ProcedureRecord>,
    revisions: Vec<ProcedureRevision>,
}

impl GrammarV2 {
    pub fn new(run_scope: ScopeToken) -> Result<Self, GrammarError> {
        Self::with_resource_meter(run_scope, ResourceMeter::runtime())
    }

    pub fn with_resource_meter(
        run_scope: ScopeToken,
        meter: ResourceMeter,
    ) -> Result<Self, GrammarError> {
        if run_scope.is_zero() {
            return Err(GrammarError::ZeroRunScope);
        }
        Ok(Self {
            run_scope,
            meter,
            next_hypothesis: 1,
            next_procedure: 1,
            procedures: Vec::new(),
            revisions: Vec::new(),
        })
    }

    pub const fn run_scope(&self) -> ScopeToken {
        self.run_scope
    }

    pub const fn grammar_version(&self) -> &'static str {
        V2_GRAMMAR_VERSION
    }

    pub fn resource_meter(&self) -> &ResourceMeter {
        &self.meter
    }

    pub fn resource_meter_mut(&mut self) -> &mut ResourceMeter {
        &mut self.meter
    }

    pub fn procedures(&self) -> &[ProcedureRecord] {
        &self.procedures
    }

    pub fn revisions(&self) -> &[ProcedureRevision] {
        &self.revisions
    }

    /// Generate from the ledger's immutable interaction view.  Final/frozen
    /// records, incomplete abstentions, and predictions without results are
    /// not candidate evidence.
    pub fn generate(&mut self, ledger: &InteractionLedger) -> GenerationReport {
        self.generate_interactions(ledger.interactions())
    }

    /// The slice form is useful for audit/replay tests and still accepts only
    /// learner-owned ledger records, never evaluator records or expected data.
    pub fn generate_interactions(
        &mut self,
        interaction_records: &[InteractionRecord],
    ) -> GenerationReport {
        self.meter.begin_episode();
        self.meter.begin_decision();

        let mut records = interaction_records.to_vec();
        records.sort_unstable_by_key(|record| {
            (
                record.evidence().evidence_id(),
                record.evidence().run_scope(),
            )
        });

        let mut rejected = Vec::new();
        let mut usable = Vec::new();
        for record in records {
            if record.evidence().run_scope() != self.run_scope {
                rejected.push(rejection(
                    CandidateStage::E0,
                    None,
                    vec![record.evidence().evidence_id()],
                    vec![RejectionReason::ScopeConfinement],
                    CandidateCosts::default(),
                ));
                continue;
            }
            if record.decision().phase() != RunPhase::Mutable {
                continue;
            }
            if record.evidence().provenance() != EvidenceProvenance::Observed
                || record.evidence().action().is_none()
                || record.evidence().after().is_none()
            {
                continue;
            }
            usable.push(record);
        }

        if usable.is_empty() {
            rejected.push(rejection(
                CandidateStage::E0,
                None,
                Vec::new(),
                vec![RejectionReason::UnsupportedByGrammar],
                CandidateCosts::default(),
            ));
            return GenerationReport {
                candidates: Vec::new(),
                rejections: rejected,
                status: GenerationStatus::UnsupportedByGrammar,
            };
        }

        let mut e0 = Vec::new();
        for record in &usable {
            if let Err(error) = self.meter.record_candidate_generation() {
                return incomplete_report(e0, rejected, CandidateStage::E0, error);
            }
            let (unchanged, changed) = effect_positions(record);
            let input_atoms = record.evidence().before().atoms();
            let output_atoms = record.evidence().after().expect("usable result").atoms();
            let output = output_operations(record.evidence().before(), output_atoms);
            if let Err(error) = charge_operations(&mut self.meter, OBSERVATION_PACKET_LEN) {
                return incomplete_report(e0, rejected, CandidateStage::E0, error);
            }
            let candidate = Candidate::new(
                CandidateStage::E0,
                self.run_scope,
                CandidateScope::Episode(record.evidence().episode_scope()),
                record.evidence().action().expect("usable action"),
                InputPattern::new(input_atoms.map(PatternAtom::Constant)),
                output,
                unchanged,
                changed.clone(),
                equalities(record.evidence().before()),
                equalities(ObservationPacket::new(output_atoms)),
                precondition(record.evidence().before(), &changed),
                vec![record.evidence().evidence_id()],
                Vec::new(),
                self.allocate_hypothesis(),
                CandidateCosts::new(1, 0, OBSERVATION_PACKET_LEN as u64, 0),
            );
            e0.push(candidate);
        }

        let mut e1 = Vec::new();
        let mut e2 = Vec::new();
        for left in 0..usable.len() {
            for right in (left + 1)..usable.len() {
                if let Err(error) = self.meter.record_candidate_generation() {
                    let mut candidates = e0;
                    candidates.extend(e1);
                    candidates.extend(merge_candidates(e2));
                    return incomplete_report(candidates, rejected, CandidateStage::E1, error);
                }
                let first = &usable[left];
                let second = &usable[right];
                let first_action = first.evidence().action().expect("usable action");
                let second_action = second.evidence().action().expect("usable action");
                let pair_ids = vec![
                    first.evidence().evidence_id(),
                    second.evidence().evidence_id(),
                ];
                if first_action != second_action {
                    rejected.push(rejection(
                        CandidateStage::E1,
                        None,
                        pair_ids,
                        vec![RejectionReason::ActionMismatch],
                        CandidateCosts::new(1, 0, 0, 0),
                    ));
                    continue;
                }
                let mut comparisons = 0_u64;
                for _ in 0..OBSERVATION_PACKET_LEN {
                    if let Err(error) = self.meter.record_structural_comparison() {
                        let mut candidates = e0;
                        candidates.extend(e1);
                        candidates.extend(merge_candidates(e2));
                        return incomplete_report(candidates, rejected, CandidateStage::E1, error);
                    }
                    comparisons += 1;
                }
                let first_effect = effect_positions(first);
                let second_effect = effect_positions(second);
                let stable = first_effect == second_effect
                    && equalities(first.evidence().before())
                        == equalities(second.evidence().before())
                    && equalities(first.evidence().after().expect("usable result"))
                        == equalities(second.evidence().after().expect("usable result"));
                if !stable {
                    rejected.push(rejection(
                        CandidateStage::E1,
                        None,
                        pair_ids,
                        vec![RejectionReason::UnstableEffect],
                        CandidateCosts::new(1, comparisons, 0, 0),
                    ));
                    continue;
                }
                let (unchanged, changed) = first_effect;
                if let Err(error) = charge_operations(&mut self.meter, OBSERVATION_PACKET_LEN) {
                    let mut candidates = e0;
                    candidates.extend(e1);
                    candidates.extend(merge_candidates(e2));
                    return incomplete_report(candidates, rejected, CandidateStage::E1, error);
                }
                let scope = if first.evidence().episode_scope() == second.evidence().episode_scope()
                {
                    CandidateScope::Episode(first.evidence().episode_scope())
                } else {
                    CandidateScope::Run(self.run_scope)
                };
                e1.push(Candidate::new(
                    CandidateStage::E1,
                    self.run_scope,
                    scope,
                    first_action,
                    InputPattern::new(first.evidence().before().atoms().map(PatternAtom::Constant)),
                    output_operations(
                        first.evidence().before(),
                        first.evidence().after().expect("usable result").atoms(),
                    ),
                    unchanged.clone(),
                    changed.clone(),
                    equalities(first.evidence().before()),
                    equalities(first.evidence().after().expect("usable result")),
                    precondition(first.evidence().before(), &changed),
                    pair_ids.clone(),
                    Vec::new(),
                    self.allocate_hypothesis(),
                    CandidateCosts::new(1, comparisons, OBSERVATION_PACKET_LEN as u64, 0),
                ));

                if let Err(error) = self.meter.record_candidate_generation() {
                    let mut candidates = e0;
                    candidates.extend(e1);
                    candidates.extend(merge_candidates(e2));
                    return incomplete_report(candidates, rejected, CandidateStage::E2, error);
                }
                match e2_from_pair(first, second, (unchanged.clone(), changed.clone())) {
                    Ok(Some((input, output, input_eq, output_eq, variable_positions))) => {
                        let mut e2_changed = changed.clone();
                        e2_changed.sort_unstable();
                        let mut e2_unchanged = unchanged.clone();
                        e2_unchanged.sort_unstable();
                        if let Err(error) =
                            charge_operations(&mut self.meter, OBSERVATION_PACKET_LEN)
                        {
                            let mut candidates = e0;
                            candidates.extend(e1);
                            candidates.extend(merge_candidates(e2));
                            return incomplete_report(
                                candidates,
                                rejected,
                                CandidateStage::E2,
                                error,
                            );
                        }
                        let _ = variable_positions;
                        e2.push(Candidate::new(
                            CandidateStage::E2,
                            self.run_scope,
                            CandidateScope::Run(self.run_scope),
                            first_action,
                            input,
                            output,
                            e2_unchanged,
                            e2_changed.clone(),
                            input_eq,
                            output_eq,
                            precondition(first.evidence().before(), &e2_changed),
                            pair_ids,
                            Vec::new(),
                            self.allocate_hypothesis(),
                            CandidateCosts::new(1, comparisons, OBSERVATION_PACKET_LEN as u64, 0),
                        ));
                    }
                    Ok(None) => rejected.push(rejection(
                        CandidateStage::E2,
                        None,
                        pair_ids,
                        vec![RejectionReason::UnsupportedByGrammar],
                        CandidateCosts::new(1, comparisons, 0, 0),
                    )),
                    Err(reason) => rejected.push(rejection(
                        CandidateStage::E2,
                        None,
                        pair_ids,
                        vec![reason],
                        CandidateCosts::new(1, comparisons, 0, 0),
                    )),
                }
            }
        }

        if usable.len() < 2 {
            rejected.push(rejection(
                CandidateStage::E2,
                None,
                usable
                    .iter()
                    .map(|record| record.evidence().evidence_id())
                    .collect(),
                vec![RejectionReason::InsufficientSupport],
                CandidateCosts::default(),
            ));
        }

        let e2 = merge_candidates(e2);
        let mut e3 = Vec::new();
        for candidate in &e2 {
            let support: BTreeSet<_> = candidate.support_evidence_ids.iter().copied().collect();
            let mut challenges = Vec::new();
            for record in &usable {
                if support.contains(&record.evidence().evidence_id()) {
                    continue;
                }
                if let Err(error) = self.meter.record_candidate_generation() {
                    let mut candidates = e0;
                    candidates.extend(e1);
                    candidates.extend(e2.clone());
                    candidates.extend(e3);
                    return incomplete_report(candidates, rejected, CandidateStage::E3, error);
                }
                if let Err(error) = self.meter.record_structural_comparison() {
                    let mut candidates = e0;
                    candidates.extend(e1);
                    candidates.extend(e2.clone());
                    candidates.extend(e3);
                    return incomplete_report(candidates, rejected, CandidateStage::E3, error);
                }
                if is_contrasting_challenge(candidate, record) {
                    challenges.push(record.evidence().evidence_id());
                }
            }
            if challenges.is_empty() {
                rejected.push(rejection(
                    CandidateStage::E3,
                    Some(candidate.canonical_signature().to_owned()),
                    candidate.support_evidence_ids.clone(),
                    vec![RejectionReason::MissingChallenge],
                    candidate.costs,
                ));
            } else {
                let costs = candidate.costs.with_challenge(challenges.len() as u64);
                e3.push(candidate.with_challenge_ids(challenges, costs));
            }
        }

        let mut candidates = e0;
        candidates.extend(e1);
        candidates.extend(e2);
        candidates.extend(merge_candidates(e3));
        candidates.sort_by_key(candidate_order_key);
        let status = if candidates.is_empty() {
            GenerationStatus::UnsupportedByGrammar
        } else {
            GenerationStatus::Complete
        };
        GenerationReport {
            candidates,
            rejections: rejected,
            status,
        }
    }

    pub fn admit(&mut self, candidate: Candidate, ledger: &InteractionLedger) -> AdmissionOutcome {
        self.meter.begin_decision();
        let mut gates = Vec::new();
        let mut reasons = Vec::new();
        let mut costs = AdmissionCosts::default();

        let records = evidence_map(ledger);
        let grounding_ids = find_grounding_ids(&candidate, ledger);
        let checks = [
            AdmissionGate::Support,
            AdmissionGate::Variation,
            AdmissionGate::Consistency,
            AdmissionGate::StructuralTraceability,
            AdmissionGate::NonApplicabilityChallenge,
            AdmissionGate::GroundingEvidence,
            AdmissionGate::ScopeConfinement,
            AdmissionGate::CostCeiling,
        ];
        for gate in checks {
            if let Err(_error) = self.meter.record_stage_operation(Stage::Admission, 1) {
                return AdmissionOutcome::Incomplete(IncompleteAdmission {
                    candidate,
                    reason: RejectionReason::BudgetExhausted,
                    costs,
                });
            }
            costs.gate_checks += 1;
            let passed = match gate {
                AdmissionGate::Support => {
                    matches!(candidate.stage, CandidateStage::E2 | CandidateStage::E3)
                        && candidate.support_evidence_ids.len() >= 2
                        && candidate
                            .support_evidence_ids
                            .iter()
                            .all(|id| records.contains_key(id))
                }
                AdmissionGate::Variation => variation_gate(&candidate, &records),
                AdmissionGate::Consistency => {
                    let mut passed = true;
                    for id in &candidate.support_evidence_ids {
                        if let Some(record) = records.get(id) {
                            if let (Some(action), Some(after)) =
                                (record.evidence().action(), record.evidence().after())
                            {
                                if let Err(error) = self.meter.record_structural_comparison() {
                                    return incomplete_admission(candidate, costs, error);
                                }
                                costs.structural_comparisons += 1;
                                if candidate.predict(record.evidence().before(), action)
                                    != Some(after)
                                {
                                    passed = false;
                                }
                                if let Err(error) = self.meter.record_operation_step() {
                                    return incomplete_admission(candidate, costs, error);
                                }
                                costs.operation_steps += 1;
                            } else {
                                passed = false;
                            }
                        } else {
                            passed = false;
                        }
                    }
                    passed
                }
                AdmissionGate::StructuralTraceability => {
                    structural_traceability(&candidate, &records)
                }
                AdmissionGate::NonApplicabilityChallenge => challenge_gate(&candidate, &records),
                AdmissionGate::GroundingEvidence => !grounding_ids.is_empty(),
                AdmissionGate::ScopeConfinement => {
                    scope_gate(&candidate, self.run_scope, &records, &grounding_ids)
                }
                AdmissionGate::CostCeiling => candidate.costs.within_protocol_limits(&self.meter),
            };
            if !passed {
                reasons.push(reason_for_gate(gate, &candidate));
            }
            gates.push(GateResult { gate, passed });
        }

        reasons.sort_unstable();
        reasons.dedup();
        if reasons.is_empty() {
            let mut admitted_candidate = candidate;
            admitted_candidate.grounding_evidence_ids = grounding_ids.clone();
            admitted_candidate.normalize();
            admitted_candidate.refresh_signature();
            AdmissionOutcome::Admitted(AdmittedCandidate {
                candidate: admitted_candidate,
                gates,
                grounding_evidence_ids: grounding_ids,
                costs,
            })
        } else {
            let mut evidence_ids = candidate.support_evidence_ids.clone();
            evidence_ids.extend(candidate.challenge_evidence_ids.iter().copied());
            evidence_ids.extend(grounding_ids.iter().copied());
            evidence_ids.sort_unstable();
            evidence_ids.dedup();
            let rejected_candidate = candidate.with_rejection(reasons.clone(), evidence_ids);
            AdmissionOutcome::Rejected(RejectedCandidate {
                candidate: rejected_candidate,
                gates,
                reasons,
                costs,
            })
        }
    }

    pub fn promote(
        &mut self,
        ledger: &mut InteractionLedger,
        admitted: AdmittedCandidate,
    ) -> Result<ProcedureRecord, PromotionError> {
        let candidate = admitted.candidate;
        if candidate.stage != CandidateStage::E3 {
            return Err(PromotionError::NotAnAdmittedE3);
        }
        if candidate.run_scope != self.run_scope
            || candidate.scope != CandidateScope::Run(self.run_scope)
        {
            return Err(PromotionError::ScopeConfinement);
        }
        self.meter.begin_decision();
        self.meter.record_candidate_generation()?;
        charge_operations(&mut self.meter, OBSERVATION_PACKET_LEN)?;
        let procedure_id = ProcedureId::new(self.next_procedure);
        self.next_procedure = self
            .next_procedure
            .checked_add(1)
            .ok_or(PromotionError::CounterOverflow)?;
        let costs =
            candidate
                .costs
                .plus(CandidateCosts::new(1, 0, OBSERVATION_PACKET_LEN as u64, 0));
        let expansion = candidate.with_stage_and_procedure(
            CandidateStage::E4,
            CandidateScope::Run(self.run_scope),
            procedure_id,
            costs,
        );
        ledger.register_procedure(procedure_id, AuditScope::Run, true)?;
        let provenance = PromotionProvenance {
            source_signature: candidate.canonical_signature().to_owned(),
            expansion: expansion.clone(),
            support_evidence_ids: expansion.support_evidence_ids.clone(),
            challenge_evidence_ids: expansion.challenge_evidence_ids.clone(),
            rejection_evidence_ids: expansion.rejection_evidence_ids.clone(),
            costs,
        };
        let procedure = ProcedureRecord {
            procedure_id,
            version: 1,
            status: ProcedureStatus::Active,
            scope: CandidateScope::Run(self.run_scope),
            hypothesis_id: expansion.hypothesis_id,
            provenance,
        };
        self.procedures.push(procedure.clone());
        self.meter.set_retained_procedure_records(
            (self.procedures.len() + self.revisions.len()) as u64,
        )?;
        Ok(procedure)
    }

    /// Revalidation never edits the original procedure or its evidence.  A
    /// contradiction appends a suspended version carrying the new evidence.
    pub fn revalidate(
        &mut self,
        procedure: &ProcedureRecord,
        ledger: &InteractionLedger,
    ) -> RevisionOutcome {
        self.meter.begin_decision();
        let known: BTreeSet<_> = procedure
            .support_evidence_ids()
            .iter()
            .chain(procedure.challenge_evidence_ids().iter())
            .copied()
            .collect();
        let mut contradictions = Vec::new();
        let mut records = ledger.interactions().to_vec();
        records.sort_unstable_by_key(|record| {
            (
                record.evidence().evidence_id(),
                record.evidence().run_scope(),
            )
        });
        for record in records {
            if known.contains(&record.evidence().evidence_id())
                || record.decision().phase() != RunPhase::Mutable
                || record.evidence().run_scope() != self.run_scope
                || record.evidence().provenance() != EvidenceProvenance::Observed
            {
                continue;
            }
            let Some(action) = record.evidence().action() else {
                continue;
            };
            let Some(after) = record.evidence().after() else {
                continue;
            };
            let Some(predicted) = procedure
                .expansion()
                .predict(record.evidence().before(), action)
            else {
                continue;
            };
            if let Err(_error) = self.meter.record_structural_comparison() {
                return RevisionOutcome::Incomplete(RejectionReason::BudgetExhausted);
            }
            if predicted != after {
                contradictions.push(record.evidence().evidence_id());
            }
        }
        if contradictions.is_empty() {
            return RevisionOutcome::Unchanged;
        }
        if let Err(_error) = self.meter.record_stage_operation(Stage::Revalidation, 1) {
            return RevisionOutcome::Incomplete(RejectionReason::BudgetExhausted);
        }
        let mut reasons = vec![RejectionReason::Contradiction];
        reasons.sort_unstable();
        let expansion = procedure
            .expansion()
            .with_revision(reasons, contradictions.clone());
        let revision = ProcedureRevision {
            procedure_id: procedure.procedure_id,
            version: procedure.version.saturating_add(1),
            status: ProcedureStatus::Suspended,
            expansion,
            contradiction_evidence_ids: contradictions,
            preserved_provenance: procedure.provenance.clone(),
        };
        self.revisions.push(revision.clone());
        let _ = self
            .meter
            .set_retained_procedure_records((self.procedures.len() + self.revisions.len()) as u64);
        revision.into()
    }

    fn allocate_hypothesis(&mut self) -> HypothesisId {
        let id = HypothesisId::new(self.next_hypothesis);
        self.next_hypothesis = self.next_hypothesis.saturating_add(1);
        id
    }
}

impl From<ProcedureRevision> for RevisionOutcome {
    fn from(revision: ProcedureRevision) -> Self {
        Self::Suspended(Box::new(revision))
    }
}

fn output_operations(
    before: ObservationPacket,
    after: [AtomId; OBSERVATION_PACKET_LEN],
) -> [OutputOp; OBSERVATION_PACKET_LEN] {
    std::array::from_fn(|index| {
        if before.atom(index) == Some(after[index]) {
            OutputOp::Copy(index)
        } else {
            OutputOp::Observed(after[index])
        }
    })
}

fn effect_positions(record: &InteractionRecord) -> (Vec<usize>, Vec<usize>) {
    let before = record.evidence().before();
    let after = record.evidence().after().expect("usable interaction");
    let mut unchanged = Vec::new();
    let mut changed = Vec::new();
    for index in 0..OBSERVATION_PACKET_LEN {
        if before.atom(index) == after.atom(index) {
            unchanged.push(index);
        } else {
            changed.push(index);
        }
    }
    (unchanged, changed)
}

fn equalities(packet: ObservationPacket) -> Vec<EqualityRelation> {
    let mut result = Vec::new();
    for left in 0..OBSERVATION_PACKET_LEN {
        for right in (left + 1)..OBSERVATION_PACKET_LEN {
            if packet.atom(left) == packet.atom(right) {
                result.push(EqualityRelation::new(left, right));
            }
        }
    }
    result
}

fn precondition(packet: ObservationPacket, changed: &[usize]) -> Vec<(usize, AtomId)> {
    let mut result = changed
        .iter()
        .filter_map(|position| packet.atom(*position).map(|atom| (*position, atom)))
        .collect::<Vec<_>>();
    result.sort_unstable_by_key(|(position, _)| *position);
    result
}

type E2Candidate = (
    InputPattern,
    [OutputOp; OBSERVATION_PACKET_LEN],
    Vec<EqualityRelation>,
    Vec<EqualityRelation>,
    Vec<usize>,
);

fn e2_from_pair(
    first: &InteractionRecord,
    second: &InteractionRecord,
    effect: (Vec<usize>, Vec<usize>),
) -> Result<Option<E2Candidate>, RejectionReason> {
    let first_before = first.evidence().before();
    let second_before = second.evidence().before();
    let first_after = first.evidence().after().expect("usable interaction");
    let second_after = second.evidence().after().expect("usable interaction");
    let first_input_eq = equalities(first_before);
    if first_input_eq != equalities(second_before) {
        return Ok(None);
    }
    let (unchanged, changed) = effect;
    let mut repeated = None;
    for relation in &first_input_eq {
        if !unchanged.contains(&relation.left()) || !unchanged.contains(&relation.right()) {
            continue;
        }
        let Some(left_first) = first_before.atom(relation.left()) else {
            continue;
        };
        let Some(right_first) = first_before.atom(relation.right()) else {
            continue;
        };
        let Some(left_second) = second_before.atom(relation.left()) else {
            continue;
        };
        let Some(right_second) = second_before.atom(relation.right()) else {
            continue;
        };
        if left_first != right_first || left_second != right_second || left_first == left_second {
            continue;
        }
        repeated = Some((relation.left(), relation.right()));
        break;
    }
    let Some((variable_left, variable_right)) = repeated else {
        return Ok(None);
    };
    let variable_positions = vec![variable_left, variable_right];
    for position in 0..OBSERVATION_PACKET_LEN {
        if variable_positions.contains(&position) {
            continue;
        }
        if first_before.atom(position) != second_before.atom(position) {
            return Ok(None);
        }
    }
    for position in &changed {
        if first_after.atom(*position) != second_after.atom(*position) {
            return Err(RejectionReason::OutputInconsistency);
        }
    }
    let mut pattern = [PatternAtom::Constant(AtomId::new(0)); OBSERVATION_PACKET_LEN];
    for (position, pattern_atom) in pattern.iter_mut().enumerate() {
        *pattern_atom = if variable_positions.contains(&position) {
            PatternAtom::Variable(VariableId::ZERO)
        } else {
            PatternAtom::Constant(first_before.atom(position).expect("fixed packet"))
        };
    }
    Ok(Some((
        InputPattern::new(pattern),
        output_operations(first_before, first_after.atoms()),
        first_input_eq,
        equalities(first_after),
        variable_positions,
    )))
}

fn is_contrasting_challenge(candidate: &Candidate, record: &InteractionRecord) -> bool {
    let Some(action) = record.evidence().action() else {
        return false;
    };
    let Some(after) = record.evidence().after() else {
        return false;
    };
    let precondition_matches = candidate
        .precondition
        .iter()
        .all(|(position, atom)| record.evidence().before().atom(*position) == Some(*atom));
    let action_or_precondition_differs = action != candidate.action || !precondition_matches;
    let predicted_change_absent = candidate
        .changed_positions
        .iter()
        .all(|position| record.evidence().before().atom(*position) == after.atom(*position));
    action_or_precondition_differs && predicted_change_absent
}

fn evidence_map(ledger: &InteractionLedger) -> BTreeMap<EvidenceId, &InteractionRecord> {
    ledger
        .interactions()
        .iter()
        .map(|record| (record.evidence().evidence_id(), record))
        .collect()
}

fn find_grounding_ids(candidate: &Candidate, ledger: &InteractionLedger) -> Vec<EvidenceId> {
    let mut result = Vec::new();
    for record in ledger.interactions() {
        if record.decision().phase() != RunPhase::Mutable
            || record.evidence().provenance() != EvidenceProvenance::Observed
            || record.evidence().run_scope() != candidate.run_scope
        {
            continue;
        }
        let Some(action) = record.evidence().action() else {
            continue;
        };
        let audit = record.decision().decision().audit();
        let hypothesis_match = audit.hypothesis_refs().contains(&candidate.hypothesis_id);
        let procedure_match = candidate
            .procedure_id
            .is_some_and(|id| audit.procedure_refs().contains(&id));
        let selected = match record.decision().decision() {
            Decision::Act(act) => match act.mode() {
                ActMode::Exploration => hypothesis_match,
                ActMode::Procedure => procedure_match,
                ActMode::Fallback => false,
            },
            Decision::Abstain(_) => false,
        };
        if selected && (hypothesis_match || procedure_match || action == candidate.action) {
            result.push(record.evidence().evidence_id());
        }
    }
    result.sort_unstable();
    result.dedup();
    result
}

fn variation_gate(
    candidate: &Candidate,
    records: &BTreeMap<EvidenceId, &InteractionRecord>,
) -> bool {
    if candidate.stage != CandidateStage::E2 && candidate.stage != CandidateStage::E3 {
        return false;
    }
    let positions = candidate.input.variable_positions();
    if positions.is_empty() {
        return false;
    }
    let mut values = BTreeSet::new();
    for id in &candidate.support_evidence_ids {
        let Some(record) = records.get(id) else {
            return false;
        };
        let key = positions
            .iter()
            .filter_map(|position| record.evidence().before().atom(*position))
            .map(|atom| atom.value())
            .collect::<Vec<_>>();
        values.insert(key);
    }
    values.len() >= 2
}

fn structural_traceability(
    candidate: &Candidate,
    records: &BTreeMap<EvidenceId, &InteractionRecord>,
) -> bool {
    if !candidate.input.is_bounded()
        || candidate.packet_length != OBSERVATION_PACKET_LEN
        || candidate.output.len() != OBSERVATION_PACKET_LEN
    {
        return false;
    }
    let mut all_positions = candidate.unchanged_positions.clone();
    all_positions.extend(candidate.changed_positions.iter().copied());
    all_positions.sort_unstable();
    all_positions.dedup();
    if all_positions != (0..OBSERVATION_PACKET_LEN).collect::<Vec<_>>()
        || candidate
            .unchanged_positions
            .iter()
            .any(|position| candidate.changed_positions.contains(position))
    {
        return false;
    }
    let expected_input_eq = candidate.input_equalities.as_slice();
    for id in &candidate.support_evidence_ids {
        let Some(record) = records.get(id) else {
            return false;
        };
        if equalities(record.evidence().before()).as_slice() != expected_input_eq {
            return false;
        }
        let Some(after) = record.evidence().after() else {
            return false;
        };
        if equalities(after).as_slice() != candidate.output_equalities.as_slice() {
            return false;
        }
        for position in &candidate.changed_positions {
            if record.evidence().before().atom(*position) == after.atom(*position) {
                return false;
            }
        }
        for position in &candidate.unchanged_positions {
            if record.evidence().before().atom(*position) != after.atom(*position) {
                return false;
            }
        }
    }
    true
}

fn challenge_gate(
    candidate: &Candidate,
    records: &BTreeMap<EvidenceId, &InteractionRecord>,
) -> bool {
    if candidate.stage != CandidateStage::E3 || candidate.challenge_evidence_ids.is_empty() {
        return false;
    }
    candidate.challenge_evidence_ids.iter().all(|id| {
        records
            .get(id)
            .is_some_and(|record| is_contrasting_challenge(candidate, record))
    })
}

fn scope_gate(
    candidate: &Candidate,
    run_scope: ScopeToken,
    records: &BTreeMap<EvidenceId, &InteractionRecord>,
    grounding_ids: &[EvidenceId],
) -> bool {
    if candidate.run_scope != run_scope || !candidate.scope.is_run() {
        return false;
    }
    candidate
        .support_evidence_ids
        .iter()
        .chain(candidate.challenge_evidence_ids.iter())
        .chain(grounding_ids.iter())
        .all(|id| {
            records.get(id).is_some_and(|record| {
                record.evidence().run_scope() == run_scope
                    && record.decision().phase() == RunPhase::Mutable
                    && record.evidence().provenance() == EvidenceProvenance::Observed
            })
        })
}

fn reason_for_gate(gate: AdmissionGate, candidate: &Candidate) -> RejectionReason {
    match gate {
        AdmissionGate::Support => {
            if candidate.stage != CandidateStage::E2 && candidate.stage != CandidateStage::E3 {
                RejectionReason::UnsupportedByGrammar
            } else {
                RejectionReason::InsufficientSupport
            }
        }
        AdmissionGate::Variation => RejectionReason::MissingContextVariation,
        AdmissionGate::Consistency => RejectionReason::OutputInconsistency,
        AdmissionGate::StructuralTraceability => RejectionReason::MissingStructuralTraceability,
        AdmissionGate::NonApplicabilityChallenge => RejectionReason::MissingChallenge,
        AdmissionGate::GroundingEvidence => RejectionReason::MissingGroundingEvidence,
        AdmissionGate::ScopeConfinement => RejectionReason::ScopeConfinement,
        AdmissionGate::CostCeiling => RejectionReason::CostCeiling,
    }
}

fn candidate_order_key(candidate: &Candidate) -> CandidateOrder {
    candidate.canonical_order()
}

fn structural_key(candidate: &Candidate) -> String {
    signature_for(candidate, false)
}

fn merge_candidates(candidates: Vec<Candidate>) -> Vec<Candidate> {
    let mut sorted = candidates;
    sorted.sort_by_key(candidate_order_key);
    let mut merged: BTreeMap<String, Candidate> = BTreeMap::new();
    for candidate in sorted {
        let key = structural_key(&candidate);
        if let Some(existing) = merged.get_mut(&key) {
            existing
                .support_evidence_ids
                .extend(candidate.support_evidence_ids);
            existing
                .challenge_evidence_ids
                .extend(candidate.challenge_evidence_ids);
            existing
                .rejection_evidence_ids
                .extend(candidate.rejection_evidence_ids);
            existing.costs = existing.costs.plus(candidate.costs);
            existing.normalize();
            existing.refresh_signature();
        } else {
            merged.insert(key, candidate);
        }
    }
    let mut result = merged.into_values().collect::<Vec<_>>();
    result.sort_by_key(candidate_order_key);
    result
}

fn signature_for(candidate: &Candidate, include_support: bool) -> String {
    let pattern = candidate
        .input
        .atoms
        .iter()
        .map(|atom| match atom {
            PatternAtom::Constant(atom) => format!("c{}", atom.value()),
            PatternAtom::Variable(variable) => format!("v{}", variable.value()),
        })
        .collect::<Vec<_>>()
        .join(",");
    let output = candidate
        .output
        .iter()
        .map(|operation| match operation {
            OutputOp::Copy(position) => format!("x{position}"),
            OutputOp::Observed(atom) => format!("o{}", atom.value()),
        })
        .collect::<Vec<_>>()
        .join(",");
    let unchanged = candidate
        .unchanged_positions
        .iter()
        .map(|position| position.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let changed = candidate
        .changed_positions
        .iter()
        .map(|position| position.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let scope = match candidate.scope {
        CandidateScope::Episode(token) => format!("e{}", token.value()),
        CandidateScope::Run(token) => format!("r{}", token.value()),
    };
    let input_equalities = candidate
        .input_equalities
        .iter()
        .map(|relation| format!("{}-{}", relation.left(), relation.right()))
        .collect::<Vec<_>>()
        .join(",");
    let output_equalities = candidate
        .output_equalities
        .iter()
        .map(|relation| format!("{}-{}", relation.left(), relation.right()))
        .collect::<Vec<_>>()
        .join(",");
    let support = candidate
        .support_evidence_ids
        .iter()
        .map(|id| id.value().to_string())
        .collect::<Vec<_>>()
        .join(",");
    let support_part = if include_support {
        support
    } else {
        String::new()
    };
    format!(
        "{V2_GRAMMAR_VERSION}|{}|a{}|l{}|p{}|i{}|q{}|u{}|d{}|o{}|{}|{scope}|s{support_part}",
        candidate.stage.code(),
        candidate.action.value(),
        candidate.packet_length,
        pattern,
        input_equalities,
        output_equalities,
        unchanged,
        changed,
        output,
        candidate.run_scope.value(),
    )
}

fn rejection(
    stage: CandidateStage,
    signature: Option<String>,
    mut evidence_ids: Vec<EvidenceId>,
    mut reasons: Vec<RejectionReason>,
    costs: CandidateCosts,
) -> RejectionRecord {
    evidence_ids.sort_unstable();
    evidence_ids.dedup();
    reasons.sort_unstable();
    reasons.dedup();
    RejectionRecord {
        stage,
        signature,
        evidence_ids,
        reasons,
        costs,
    }
}

fn charge_operations(meter: &mut ResourceMeter, count: usize) -> Result<(), MeasurementError> {
    for _ in 0..count {
        meter.record_operation_step()?;
    }
    Ok(())
}

fn resource_for(error: MeasurementError) -> LogicalResource {
    match error {
        MeasurementError::LogicalLimit(limit) => limit.resource(),
        MeasurementError::CounterOverflow(resource) => resource,
        MeasurementError::StageCounterOverflow(_) => LogicalResource::CandidateGenerationAttempts,
        _ => LogicalResource::OperationSteps,
    }
}

fn incomplete_report(
    mut candidates: Vec<Candidate>,
    mut rejections: Vec<RejectionRecord>,
    stage: CandidateStage,
    error: MeasurementError,
) -> GenerationReport {
    candidates.sort_by_key(candidate_order_key);
    rejections.push(rejection(
        stage,
        None,
        Vec::new(),
        vec![RejectionReason::BudgetExhausted],
        CandidateCosts::default(),
    ));
    GenerationReport {
        candidates,
        rejections,
        status: GenerationStatus::Incomplete {
            stage,
            resource: resource_for(error),
        },
    }
}

fn incomplete_admission(
    candidate: Candidate,
    costs: AdmissionCosts,
    _error: MeasurementError,
) -> AdmissionOutcome {
    AdmissionOutcome::Incomplete(IncompleteAdmission {
        candidate,
        reason: RejectionReason::BudgetExhausted,
        costs,
    })
}
