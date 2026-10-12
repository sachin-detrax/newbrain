//! Reusable frozen-policy action selection over learner-owned, opaque grammar
//! records.
//!
//! Mutable `step` and frozen `step` both return a ledger receipt: the decision
//! and its explanation exist before reveal. Only the mutable type can ingest
//! the matching returned result. The external ledger owns transport history
//! and accounting; frozen selection reads prior *actions*, never final evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::grammar_admission::{
    Candidate, CandidateScope, CandidateStage, GenerationReport, GenerationStatus, ProcedureRecord,
    ProcedureStatus,
};
use super::interaction_ledger::{
    AuditScope, DecisionReceipt, InteractionLedger, LedgerError, RunPhase,
};
use super::opaque_protocol::{
    AbstainReason, ActMode, ActionId, AuditReferences, Decision, EvidenceId, HypothesisId,
    ObservationPacket, ProcedureId, ProtocolError, ScopeToken, StepInput, StepResult,
    OBSERVATION_PACKET_LEN,
};
use super::partial_observation::PartialObservation;
use super::resource_scale::{LogicalResource, MeasurementError, ResourceMeter, Stage};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DerivedStatus {
    Conditional,
    Confirmed,
    Observed,
    Stale,
    Incomplete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangeReason {
    DependencyChanged,
    TransitionChanged,
    Contradiction,
    MissingEvidence,
    BudgetExhausted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivedResult {
    pub decision_sequence: u64,
    pub status: DerivedStatus,
    pub reason: Option<ChangeReason>,
    pub procedure_refs: Vec<ProcedureId>,
    pub evidence_refs: Vec<EvidenceId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HypothesisStatus {
    Active,
    Contradicted(EvidenceId),
    Stale,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveHypothesis {
    candidate: Candidate,
    status: HypothesisStatus,
}

impl ActiveHypothesis {
    pub fn candidate(&self) -> &Candidate {
        &self.candidate
    }
    pub const fn status(&self) -> HypothesisStatus {
        self.status
    }
}

/// Costs count bounded candidate matching, output construction, packet
/// comparison, and policy operations. Query/action charges live in the ledger.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LogicalCost {
    pub query_calls: u64,
    pub environment_actions: u64,
    pub structural_comparisons: u64,
    pub operation_steps: u64,
}

impl LogicalCost {
    fn total(self) -> u64 {
        self.query_calls
            .saturating_add(self.environment_actions)
            .saturating_add(self.structural_comparisons)
            .saturating_add(self.operation_steps)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompetingPrediction {
    pub hypothesis_id: HypothesisId,
    pub procedure_id: Option<ProcedureId>,
    pub procedure_version: Option<u32>,
    pub action: ActionId,
    pub prediction: ObservationPacket,
    pub evidence_refs: Vec<EvidenceId>,
    pub cost: LogicalCost,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisagreementAction {
    pub action: ActionId,
    pub hypothesis_refs: Vec<HypothesisId>,
    pub cost: LogicalCost,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionReason {
    Procedure,
    Conflict,
    HypothesisDisagreement,
    Fallback,
    IncompleteBudget,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MutablePhase {
    Discovery,
    Development,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionTrace {
    pub input: StepInput,
    pub decision: Decision,
    pub reason: DecisionReason,
    pub competing_predictions: Vec<CompetingPrediction>,
    pub disagreement_actions: Vec<DisagreementAction>,
    pub cost: LogicalCost,
}

impl DecisionTrace {
    pub fn chosen_action(&self) -> Option<ActionId> {
        self.decision.action()
    }
    pub fn mode(&self) -> Option<ActMode> {
        match &self.decision {
            Decision::Act(decision) => Some(decision.mode()),
            Decision::Abstain(_) => None,
        }
    }
    pub fn procedure_refs(&self) -> &[ProcedureId] {
        self.decision.audit().procedure_refs()
    }
    pub fn evidence_refs(&self) -> &[EvidenceId] {
        self.decision.audit().evidence_refs()
    }
    pub fn hypothesis_refs(&self) -> &[HypothesisId] {
        self.decision.audit().hypothesis_refs()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordedDecision {
    pub receipt: DecisionReceipt,
    pub trace: DecisionTrace,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GroundingError {
    Protocol(ProtocolError),
    Ledger(LedgerError),
    Measurement(MeasurementError),
    ScopeConfinement,
    FrozenLedger,
    MutableLedger,
    PendingDecision,
    InvalidPhaseTransition,
    NoPendingDecision,
    ResultNotRecorded,
    ResultMismatch,
    MissingEvidence(EvidenceId),
    DuplicateProcedure(ProcedureId),
    UnknownProcedure(ProcedureId),
    HypothesisCollision(HypothesisId),
    SuspendedProcedure,
}

impl fmt::Display for GroundingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v2 grounding: {self:?}")
    }
}
impl std::error::Error for GroundingError {}
impl From<ProtocolError> for GroundingError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}
impl From<LedgerError> for GroundingError {
    fn from(error: LedgerError) -> Self {
        Self::Ledger(error)
    }
}
impl From<MeasurementError> for GroundingError {
    fn from(error: MeasurementError) -> Self {
        Self::Measurement(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Policy {
    run_scope: ScopeToken,
    procedures: BTreeMap<ProcedureId, ProcedureRecord>,
    hypotheses: BTreeMap<HypothesisId, ActiveHypothesis>,
    declared_predictions: Vec<CompetingPrediction>,
    stale_procedures: BTreeSet<ProcedureId>,
    separating_actions: BTreeSet<ActionId>,
    search_incomplete: bool,
}

/// Discovery and development share this mutable interface. Phase selection
/// belongs to the runner; no partition names enter the learner.
#[derive(Clone)]
pub struct GroundingLearner {
    policy: Policy,
    phase: MutablePhase,
    pending: Option<RecordedDecision>,
    traces: Vec<RecordedDecision>,
    derived: Vec<DerivedResult>,
}

impl GroundingLearner {
    pub fn new(run_scope: ScopeToken) -> Result<Self, GroundingError> {
        if run_scope.is_zero() {
            return Err(ProtocolError::ZeroScopeToken.into());
        }
        Ok(Self {
            policy: Policy {
                run_scope,
                procedures: BTreeMap::new(),
                hypotheses: BTreeMap::new(),
                declared_predictions: Vec::new(),
                stale_procedures: BTreeSet::new(),
                separating_actions: BTreeSet::new(),
                search_incomplete: false,
            },
            phase: MutablePhase::Discovery,
            pending: None,
            traces: Vec::new(),
            derived: Vec::new(),
        })
    }

    pub fn run_scope(&self) -> ScopeToken {
        self.policy.run_scope
    }
    pub const fn phase(&self) -> MutablePhase {
        self.phase
    }
    pub fn hypotheses(&self) -> impl Iterator<Item = &ActiveHypothesis> {
        self.policy.hypotheses.values()
    }
    pub fn active_hypothesis_ids(&self) -> Vec<HypothesisId> {
        let mut ids = self
            .policy
            .hypotheses
            .iter()
            .filter_map(|(id, hypothesis)| {
                (hypothesis.status == HypothesisStatus::Active).then_some(*id)
            })
            .collect::<Vec<_>>();
        ids.extend(
            self.policy
                .declared_predictions
                .iter()
                .map(|claim| claim.hypothesis_id),
        );
        unique(ids)
    }
    pub fn procedures(&self) -> impl Iterator<Item = &ProcedureRecord> {
        self.policy.procedures.values()
    }
    pub fn traces(&self) -> &[RecordedDecision] {
        &self.traces
    }
    pub fn derived_results(&self) -> &[DerivedResult] {
        &self.derived
    }

    pub fn declare_hypothesis_separating_action(
        &mut self,
        action: ActionId,
    ) -> Result<(), GroundingError> {
        self.policy.separating_actions.insert(action);
        Ok(())
    }

    pub fn enter_development(&mut self) -> Result<(), GroundingError> {
        if self.pending.is_some() {
            return Err(GroundingError::PendingDecision);
        }
        if self.phase == MutablePhase::Development {
            return Err(GroundingError::InvalidPhaseTransition);
        }
        self.phase = MutablePhase::Development;
        Ok(())
    }

    /// Declare a prediction produced from learner-owned opaque evidence. This
    /// is the explicit hypothesis seam used when two retained explanations
    /// disagree before the next result is revealed.
    pub fn declare_competing_prediction(
        &mut self,
        ledger: &mut InteractionLedger,
        hypothesis_id: HypothesisId,
        action: ActionId,
        prediction: ObservationPacket,
        evidence_refs: Vec<EvidenceId>,
    ) -> Result<(), GroundingError> {
        self.declare_competing_prediction_with_cost(
            ledger,
            hypothesis_id,
            action,
            prediction,
            evidence_refs,
            LogicalCost::default(),
        )
    }

    pub fn declare_competing_prediction_with_cost(
        &mut self,
        ledger: &mut InteractionLedger,
        hypothesis_id: HypothesisId,
        action: ActionId,
        prediction: ObservationPacket,
        evidence_refs: Vec<EvidenceId>,
        cost: LogicalCost,
    ) -> Result<(), GroundingError> {
        self.validate_mutable(ledger)?;
        if self.policy.hypotheses.contains_key(&hypothesis_id)
            || self
                .policy
                .declared_predictions
                .iter()
                .any(|claim| claim.hypothesis_id == hypothesis_id)
        {
            return Err(GroundingError::HypothesisCollision(hypothesis_id));
        }
        for evidence in &evidence_refs {
            if !ledger.has_evidence(*evidence) {
                return Err(GroundingError::MissingEvidence(*evidence));
            }
        }
        ledger.ensure_hypothesis(hypothesis_id, AuditScope::Run)?;
        self.policy.declared_predictions.push(CompetingPrediction {
            hypothesis_id,
            procedure_id: None,
            procedure_version: None,
            action,
            prediction,
            evidence_refs: unique(evidence_refs),
            cost,
        });
        self.policy
            .declared_predictions
            .sort_by_key(|claim| (claim.action, claim.hypothesis_id));
        Ok(())
    }

    /// Retain the grammar's canonical bounded hypotheses, not evaluator data.
    /// E3 candidates remain learner-owned expansions; actual disagreement is
    /// checked against the current input before exploration is selected.
    pub fn retain_generation(
        &mut self,
        ledger: &mut InteractionLedger,
        report: &GenerationReport,
    ) -> Result<(), GroundingError> {
        self.validate_mutable(ledger)?;
        if matches!(report.status(), GenerationStatus::Incomplete { .. }) {
            self.policy.search_incomplete = true;
        }
        let mut candidates = BTreeMap::new();
        for candidate in report.candidates() {
            if !matches!(candidate.stage(), CandidateStage::E2 | CandidateStage::E3) {
                continue;
            }
            validate_candidate(candidate, ledger)?;
            candidates.insert(candidate.hypothesis_id(), candidate.clone());
        }
        let count = self
            .policy
            .hypotheses
            .keys()
            .chain(candidates.keys())
            .copied()
            .collect::<BTreeSet<_>>()
            .len();
        if let Err(error) = ledger
            .resource_meter_mut()
            .set_active_hypotheses(count as u64)
        {
            self.policy.search_incomplete = true;
            return Err(error.into());
        }
        for (id, candidate) in candidates {
            if let Some(existing) = self.policy.hypotheses.get(&id) {
                if existing.candidate != candidate {
                    return Err(GroundingError::HypothesisCollision(id));
                }
                continue;
            }
            ensure_hypothesis(ledger, id, candidate.scope())?;
            self.policy.hypotheses.insert(
                id,
                ActiveHypothesis {
                    candidate,
                    status: HypothesisStatus::Active,
                },
            );
        }
        Ok(())
    }

    /// Consume the grammar's admission proof and full expansion. Promotion may
    /// already have registered this ID in the same ledger; exact scope/checked
    /// registration is verified in either case.
    pub fn install_procedure(
        &mut self,
        ledger: &mut InteractionLedger,
        procedure: ProcedureRecord,
    ) -> Result<(), GroundingError> {
        self.validate_mutable(ledger)?;
        validate_candidate(procedure.expansion(), ledger)?;
        if procedure.status() != ProcedureStatus::Active {
            return Err(GroundingError::SuspendedProcedure);
        }
        let id = procedure.procedure_id();
        if self.policy.procedures.contains_key(&id) {
            return Err(GroundingError::DuplicateProcedure(id));
        }
        ledger
            .resource_meter_mut()
            .set_retained_procedure_records((self.policy.procedures.len() + 1) as u64)?;
        ledger.ensure_procedure(id, audit_scope(procedure.scope()), true)?;
        ensure_hypothesis(ledger, procedure.hypothesis_id(), procedure.scope())?;
        self.policy.procedures.insert(id, procedure);
        Ok(())
    }

    pub fn step(
        &mut self,
        ledger: &mut InteractionLedger,
        input: StepInput,
    ) -> Result<RecordedDecision, GroundingError> {
        self.validate_mutable(ledger)?;
        let recorded = record(&self.policy, ledger, input)?;
        self.derived.push(DerivedResult {
            decision_sequence: recorded.receipt.sequence(),
            status: if recorded.trace.decision.is_abstain() {
                DerivedStatus::Incomplete
            } else {
                DerivedStatus::Conditional
            },
            reason: (recorded.trace.reason == DecisionReason::IncompleteBudget)
                .then_some(ChangeReason::BudgetExhausted),
            procedure_refs: recorded.trace.decision.audit().procedure_refs().to_vec(),
            evidence_refs: recorded.trace.decision.audit().evidence_refs().to_vec(),
        });
        self.traces.push(recorded.clone());
        self.pending = Some(recorded.clone());
        Ok(recorded)
    }

    /// Only a matching, mutable, ledger-recorded reveal is ingested. `None`
    /// closes an abstention without creating observation/identity evidence.
    pub fn observe(
        &mut self,
        ledger: &mut InteractionLedger,
        result: Option<StepResult>,
    ) -> Result<DerivedResult, GroundingError> {
        validate_ledger(&self.policy, ledger, false)?;
        let pending = self
            .pending
            .clone()
            .ok_or(GroundingError::NoPendingDecision)?;
        let interaction = ledger
            .interactions()
            .last()
            .cloned()
            .ok_or(GroundingError::ResultNotRecorded)?;
        if interaction.decision() != pending.receipt.record()
            || interaction.decision().phase() != RunPhase::Mutable
            || interaction.evidence().after() != result.map(StepResult::observation)
            || result.is_some_and(|value| value.terminal() != interaction.evidence().terminal())
        {
            return Err(GroundingError::ResultMismatch);
        }
        let evidence = interaction.evidence().evidence_id();
        let sequence = pending.receipt.sequence();
        let procedures = pending.trace.decision.audit().procedure_refs().to_vec();
        let mut status = if result.is_some() {
            DerivedStatus::Observed
        } else {
            DerivedStatus::Incomplete
        };
        let mut reason = if result.is_none() {
            Some(ChangeReason::MissingEvidence)
        } else {
            None
        };
        let mut contradicted_hypotheses = Vec::new();
        let mut contradicted_procedures = Vec::new();
        if let Some(result) = result {
            for prediction in &pending.trace.competing_predictions {
                if Some(prediction.action) != pending.trace.decision.action() {
                    continue;
                }
                if ledger
                    .resource_meter_mut()
                    .record_structural_comparison()
                    .is_err()
                {
                    status = DerivedStatus::Incomplete;
                    reason = Some(ChangeReason::BudgetExhausted);
                    break;
                }
                if prediction.prediction != result.observation() {
                    contradicted_hypotheses.push(prediction.hypothesis_id);
                    if let Some(id) = prediction.procedure_id {
                        contradicted_procedures.push(id);
                    }
                }
            }
            if status != DerivedStatus::Incomplete {
                if let Some(prediction) = pending.trace.decision.prediction() {
                    status = if prediction == result.observation() {
                        DerivedStatus::Confirmed
                    } else {
                        DerivedStatus::Stale
                    };
                }
                if !contradicted_hypotheses.is_empty() {
                    reason = Some(ChangeReason::Contradiction);
                }
                for id in contradicted_hypotheses {
                    if let Some(hypothesis) = self.policy.hypotheses.get_mut(&id) {
                        hypothesis.status = HypothesisStatus::Contradicted(evidence);
                    }
                }
                self.policy.stale_procedures.extend(contradicted_procedures);
            } else {
                // A partially completed check cannot certify any hypothesis.
                self.policy.search_incomplete = true;
            }
        }
        let derived = DerivedResult {
            decision_sequence: sequence,
            status,
            reason,
            procedure_refs: procedures,
            evidence_refs: vec![evidence],
        };
        self.pending = None;
        self.derived.push(derived.clone());
        Ok(derived)
    }

    /// Append invalidation status without rewriting any pre-action trace,
    /// procedure expansion, evidence, or previous derivation.
    pub fn invalidate_dependency(
        &mut self,
        procedure: ProcedureId,
        reason: ChangeReason,
    ) -> Result<(), GroundingError> {
        if !self.policy.procedures.contains_key(&procedure) {
            return Err(GroundingError::UnknownProcedure(procedure));
        }
        self.policy.stale_procedures.insert(procedure);
        for record in &self.traces {
            if record
                .trace
                .decision
                .audit()
                .procedure_refs()
                .contains(&procedure)
            {
                self.derived.push(DerivedResult {
                    decision_sequence: record.receipt.sequence(),
                    status: DerivedStatus::Stale,
                    reason: Some(reason),
                    procedure_refs: vec![procedure],
                    evidence_refs: record.trace.decision.audit().evidence_refs().to_vec(),
                });
            }
        }
        Ok(())
    }

    /// Freeze only the learner policy after validating the mutable ledger.
    /// The generic controller uses this operation and commits the ledger
    /// separately, so a failed learner hook cannot partially freeze transport.
    pub fn freeze_policy(
        self,
        ledger: &InteractionLedger,
    ) -> Result<FrozenGroundingLearner, GroundingError> {
        self.validate_mutable(ledger)?;
        Ok(FrozenGroundingLearner {
            policy: self.policy,
        })
    }

    /// Legacy receipt-backed API. It retains the historical behavior of
    /// freezing the ledger as part of the learner operation; the generic
    /// controller uses [`Self::freeze_policy`] instead.
    pub fn freeze(
        self,
        ledger: &mut InteractionLedger,
    ) -> Result<FrozenGroundingLearner, GroundingError> {
        let frozen = self.freeze_policy(ledger)?;
        ledger.freeze()?;
        Ok(frozen)
    }

    fn validate_mutable(&self, ledger: &InteractionLedger) -> Result<(), GroundingError> {
        validate_ledger(&self.policy, ledger, false)?;
        if self.pending.is_some() {
            return Err(GroundingError::PendingDecision);
        }
        Ok(())
    }
}

/// All learned state is immutable. Only transport accounting and decision
/// history in the caller's frozen ledger change. No final result ingestion API
/// is provided, and final evidence is never scanned by selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenGroundingLearner {
    policy: Policy,
}

impl FrozenGroundingLearner {
    pub fn run_scope(&self) -> ScopeToken {
        self.policy.run_scope
    }
    pub fn step(
        &self,
        ledger: &mut InteractionLedger,
        input: StepInput,
    ) -> Result<RecordedDecision, GroundingError> {
        validate_ledger(&self.policy, ledger, true)?;
        record(&self.policy, ledger, input)
    }
}

fn record(
    policy: &Policy,
    ledger: &mut InteractionLedger,
    input: StepInput,
) -> Result<RecordedDecision, GroundingError> {
    if input.run_scope() != policy.run_scope {
        return Err(GroundingError::ScopeConfinement);
    }
    // History contributes only which actions were emitted in this episode.
    let tried = ledger
        .decisions()
        .iter()
        .filter(|record| record.episode_scope() == input.episode_scope())
        .filter_map(|record| record.decision().action())
        .collect::<BTreeSet<_>>();
    ledger.prepare_decision(input)?;
    let trace = choose(policy, input, &tried, ledger.resource_meter_mut());
    trace.decision.validate_for(&input)?;
    let receipt = ledger.record_decision(input, trace.decision.clone())?;
    Ok(RecordedDecision { receipt, trace })
}

fn choose(
    policy: &Policy,
    input: StepInput,
    tried: &BTreeSet<ActionId>,
    meter: &mut ResourceMeter,
) -> DecisionTrace {
    let before = meter.counters();
    let mut trace = DecisionTrace {
        input,
        decision: Decision::abstain(AbstainReason::IncompleteBudget, AuditReferences::empty()),
        reason: DecisionReason::IncompleteBudget,
        competing_predictions: Vec::new(),
        disagreement_actions: Vec::new(),
        cost: LogicalCost::default(),
    };
    if let Ok((decision, reason)) = select(policy, input, tried, meter, &mut trace) {
        trace.decision = decision;
        trace.reason = reason;
    }
    let after = meter.counters();
    trace.cost = LogicalCost {
        query_calls: 1,
        environment_actions: if trace.decision.action().is_some() {
            1
        } else {
            0
        },
        structural_comparisons: after.structural_comparisons() - before.structural_comparisons(),
        operation_steps: after.operation_steps() - before.operation_steps(),
    };
    trace
}

fn select(
    policy: &Policy,
    input: StepInput,
    tried: &BTreeSet<ActionId>,
    meter: &mut ResourceMeter,
    trace: &mut DecisionTrace,
) -> Result<(Decision, DecisionReason), ()> {
    if policy.search_incomplete || input.remaining_action_budget() == 0 {
        return Err(());
    }
    meter
        .check_charge(LogicalResource::EnvironmentActions, 1)
        .map_err(|_| ())?;
    meter
        .check_charge(LogicalResource::StructuralComparisons, 1)
        .map_err(|_| ())?;
    meter
        .check_charge(LogicalResource::OperationSteps, 1)
        .map_err(|_| ())?;
    let active = policy
        .hypotheses
        .values()
        .filter(|h| h.status == HypothesisStatus::Active)
        .count();
    meter.set_active_hypotheses(active as u64).map_err(|_| ())?;
    work(meter)?;
    let mut procedures = Vec::new();
    for procedure in policy.procedures.values() {
        work(meter)?;
        if policy.stale_procedures.contains(&procedure.procedure_id()) {
            continue;
        }
        if let Some(mut prediction) = predict(procedure.expansion(), input, meter)? {
            prediction.procedure_id = Some(procedure.procedure_id());
            prediction.procedure_version = Some(procedure.version());
            procedures.push(prediction);
        }
    }
    procedures.sort_by_key(|p| (p.action, p.procedure_id));
    trace
        .competing_predictions
        .extend(procedures.iter().cloned());
    let mut conflict = false;
    for (i, left) in procedures.iter().enumerate() {
        for right in &procedures[i + 1..] {
            meter.record_structural_comparison().map_err(|_| ())?;
            if left.action != right.action || left.prediction != right.prediction {
                conflict = true;
            }
        }
    }
    if !conflict {
        if let Some(first) = procedures.first() {
            let audit = references(std::slice::from_ref(first));
            return Ok((
                Decision::act(
                    first.action,
                    Some(first.prediction),
                    ActMode::Procedure,
                    audit,
                )
                .map_err(|_| ())?,
                DecisionReason::Procedure,
            ));
        }
    }
    let mut hypotheses = Vec::new();
    for hypothesis in policy.hypotheses.values() {
        work(meter)?;
        if hypothesis.status != HypothesisStatus::Active {
            continue;
        }
        if let Some(prediction) = predict(&hypothesis.candidate, input, meter)? {
            hypotheses.push(prediction);
        }
    }
    hypotheses.extend(
        policy
            .declared_predictions
            .iter()
            .filter(|claim| input.available_actions().contains(&claim.action))
            .cloned(),
    );
    trace
        .competing_predictions
        .extend(hypotheses.iter().cloned());
    if !conflict && procedures.is_empty() {
        for (i, left) in hypotheses.iter().enumerate() {
            for right in &hypotheses[i + 1..] {
                meter.record_structural_comparison().map_err(|_| ())?;
                if left.action == right.action && left.prediction != right.prediction {
                    conflict = true;
                }
            }
        }
    }
    for action in &policy.separating_actions {
        work(meter)?;
        if !input.available_actions().contains(action) {
            continue;
        }
        let predictions: Vec<_> = hypotheses
            .iter()
            .filter(|p| p.action == *action)
            .cloned()
            .collect();
        let mut different = false;
        for (i, left) in predictions.iter().enumerate() {
            for right in &predictions[i + 1..] {
                meter.record_structural_comparison().map_err(|_| ())?;
                if left.prediction != right.prediction {
                    different = true;
                }
            }
        }
        if different {
            trace.disagreement_actions.push(DisagreementAction {
                action: *action,
                hypothesis_refs: predictions.iter().map(|p| p.hypothesis_id).collect(),
                cost: LogicalCost {
                    query_calls: 0,
                    environment_actions: 0,
                    structural_comparisons: predictions
                        .iter()
                        .map(|p| p.cost.structural_comparisons)
                        .sum(),
                    operation_steps: predictions.iter().map(|p| p.cost.operation_steps).sum(),
                },
            });
        }
    }
    trace
        .disagreement_actions
        .sort_by_key(|a| (a.cost.total(), a.action));
    if let Some(separating) = trace.disagreement_actions.first() {
        let refs: Vec<_> = hypotheses
            .into_iter()
            .filter(|p| p.action == separating.action)
            .collect();
        return Ok((
            Decision::act(
                separating.action,
                None,
                ActMode::Exploration,
                references(&refs),
            )
            .map_err(|_| ())?,
            DecisionReason::HypothesisDisagreement,
        ));
    }
    if conflict {
        let conflict_refs = if procedures.is_empty() {
            &hypotheses
        } else {
            &procedures
        };
        return Ok((
            Decision::abstain(AbstainReason::Conflict, references(conflict_refs)),
            DecisionReason::Conflict,
        ));
    }
    for action in input.available_actions() {
        work(meter)?;
        if !tried.contains(&action) {
            return Ok((
                Decision::act(action, None, ActMode::Fallback, AuditReferences::empty())
                    .map_err(|_| ())?,
                DecisionReason::Fallback,
            ));
        }
    }
    Err(())
}

fn predict(
    candidate: &Candidate,
    input: StepInput,
    meter: &mut ResourceMeter,
) -> Result<Option<CompetingPrediction>, ()> {
    if candidate.run_scope() != input.run_scope()
        || !scope_applies(candidate.scope(), input)
        || !input.available_actions().contains(&candidate.action())
    {
        return Ok(None);
    }
    // Charge the complete bounded matching/construction work before executing
    // the existing grammar. No position or identifier carries semantic meaning.
    let cost = LogicalCost {
        query_calls: 0,
        environment_actions: 0,
        structural_comparisons: OBSERVATION_PACKET_LEN as u64,
        operation_steps: candidate.output_operations().len() as u64,
    };
    meter
        .try_charge(
            LogicalResource::StructuralComparisons,
            cost.structural_comparisons,
        )
        .map_err(|_| ())?;
    meter
        .try_charge(LogicalResource::OperationSteps, cost.operation_steps)
        .map_err(|_| ())?;
    Ok(candidate
        .predict(input.observation(), candidate.action())
        .map(|prediction| CompetingPrediction {
            hypothesis_id: candidate.hypothesis_id(),
            procedure_id: None,
            procedure_version: None,
            action: candidate.action(),
            prediction,
            evidence_refs: candidate_evidence(candidate),
            cost,
        }))
}

fn work(meter: &mut ResourceMeter) -> Result<(), ()> {
    meter.record_operation_step().map_err(|_| ())?;
    meter
        .record_stage_operation(Stage::DecisionAction, 1)
        .map_err(|_| ())
}

fn references(predictions: &[CompetingPrediction]) -> AuditReferences {
    AuditReferences::new(
        unique(predictions.iter().filter_map(|p| p.procedure_id).collect()),
        unique(
            predictions
                .iter()
                .flat_map(|p| p.evidence_refs.iter().copied())
                .collect(),
        ),
        unique(predictions.iter().map(|p| p.hypothesis_id).collect()),
    )
    .expect("references are deduplicated")
}

fn candidate_evidence(candidate: &Candidate) -> Vec<EvidenceId> {
    unique(
        candidate
            .support_evidence_ids()
            .iter()
            .chain(candidate.challenge_evidence_ids())
            .chain(candidate.grounding_evidence_ids())
            .copied()
            .collect(),
    )
}

fn validate_candidate(
    candidate: &Candidate,
    ledger: &InteractionLedger,
) -> Result<(), GroundingError> {
    if candidate.run_scope() != ledger.run_scope()
        || matches!(candidate.scope(), CandidateScope::Run(run) if run != ledger.run_scope())
    {
        return Err(GroundingError::ScopeConfinement);
    }
    for id in candidate_evidence(candidate) {
        if !ledger.interactions().iter().any(|record| {
            record.evidence().evidence_id() == id
                && record.evidence().run_scope() == ledger.run_scope()
                && record.decision().phase() == RunPhase::Mutable
                && record.evidence().after().is_some()
                && match candidate.scope() {
                    CandidateScope::Run(_) => true,
                    CandidateScope::Episode(episode) => {
                        record.evidence().episode_scope() == episode
                    }
                }
        }) {
            return Err(GroundingError::MissingEvidence(id));
        }
    }
    Ok(())
}

fn validate_ledger(
    policy: &Policy,
    ledger: &InteractionLedger,
    frozen: bool,
) -> Result<(), GroundingError> {
    if policy.run_scope != ledger.run_scope() {
        return Err(GroundingError::ScopeConfinement);
    }
    match (frozen, ledger.is_frozen()) {
        (false, true) => Err(GroundingError::FrozenLedger),
        (true, false) => Err(GroundingError::MutableLedger),
        _ => Ok(()),
    }
}

fn ensure_hypothesis(
    ledger: &mut InteractionLedger,
    id: HypothesisId,
    scope: CandidateScope,
) -> Result<(), GroundingError> {
    ledger.ensure_hypothesis(id, audit_scope(scope))?;
    Ok(())
}

fn audit_scope(scope: CandidateScope) -> AuditScope {
    match scope {
        CandidateScope::Run(_) => AuditScope::Run,
        CandidateScope::Episode(episode) => AuditScope::Episode(episode),
    }
}

fn scope_applies(scope: CandidateScope, input: StepInput) -> bool {
    match scope {
        CandidateScope::Run(run) => run == input.run_scope(),
        CandidateScope::Episode(episode) => episode == input.episode_scope(),
    }
}

fn unique<T: Ord>(mut values: Vec<T>) -> Vec<T> {
    values.sort_unstable();
    values.dedup();
    values
}

/// Partial observations are an extension to the primary complete-packet wire
/// protocol. Unknown slots neither refute a prediction nor establish identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartialAssessment {
    pub unchanged_positions: [Option<bool>; OBSERVATION_PACKET_LEN],
    pub prediction_matches: Option<bool>,
    pub status: DerivedStatus,
}

pub fn assess_partial(
    before: PartialObservation,
    after: PartialObservation,
    prediction: Option<ObservationPacket>,
) -> PartialAssessment {
    let unchanged_positions = std::array::from_fn(|i| before.known_unchanged(after, i));
    let prediction_matches = prediction.and_then(|prediction| {
        let predicted = PartialObservation::from_packet(prediction);
        let positions: Vec<_> = (0..OBSERVATION_PACKET_LEN)
            .map(|i| predicted.known_unchanged(after, i))
            .collect();
        if positions.contains(&Some(false)) {
            Some(false)
        } else if positions.iter().all(|p| *p == Some(true)) {
            Some(true)
        } else {
            None
        }
    });
    let status = match prediction_matches {
        Some(false) => DerivedStatus::Stale,
        Some(true) => DerivedStatus::Confirmed,
        None => DerivedStatus::Incomplete,
    };
    PartialAssessment {
        unchanged_positions,
        prediction_matches,
        status,
    }
}

/// Use the existing receipt-owning grounding path through the generic
/// controller without recording the same decision twice.
impl super::LedgerLearner for GroundingLearner {
    type Error = GroundingError;
    type Frozen = FrozenGroundingLearner;

    fn step_with_ledger(
        &mut self,
        ledger: &mut InteractionLedger,
        input: StepInput,
    ) -> Result<super::LedgerDecision, Self::Error> {
        let recorded = GroundingLearner::step(self, ledger, input)?;
        Ok(super::LedgerDecision::Recorded(recorded.receipt))
    }

    fn observe_with_ledger(
        &mut self,
        ledger: &mut InteractionLedger,
        result: Option<StepResult>,
    ) -> Result<(), Self::Error> {
        GroundingLearner::observe(self, ledger, result).map(|_| ())
    }

    fn enter_development(&mut self) -> Result<(), Self::Error> {
        GroundingLearner::enter_development(self)
    }

    fn freeze_with_ledger(
        self,
        ledger: &InteractionLedger,
    ) -> Result<Self::Frozen, super::LearnerFreezeFailure<Self, Self::Error>> {
        let backup = self.clone();
        GroundingLearner::freeze_policy(self, ledger)
            .map_err(|error| super::LearnerFreezeFailure::new(backup, error))
    }
}

impl super::LedgerFrozenLearner for FrozenGroundingLearner {
    type Error = GroundingError;

    fn step_with_ledger(
        &self,
        ledger: &mut InteractionLedger,
        input: StepInput,
    ) -> Result<super::LedgerDecision, Self::Error> {
        let recorded = FrozenGroundingLearner::step(self, ledger, input)?;
        Ok(super::LedgerDecision::Recorded(recorded.receipt))
    }
}
