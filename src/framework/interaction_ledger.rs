//! Reusable append-only learner-owned transport records.
//!
//! Decisions are serialized into canonical JSONL before a receipt is returned.
//! Only a pending receipt authorizes an evaluator action. Final evidence is
//! retained for audit but never added to the decision-reference index.
//!
//! The existing ResourceMeter charges live decisions and applied actions here.
//! Candidate/storage accounting belongs to the future learner/store; evaluator
//! transition work is not charged as learner operations.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::opaque_protocol::{
    ActMode, ActionId, AuditReferenceKind, Decision, EvidenceId, HypothesisId, ObservationPacket,
    PreviousTerminal, ProcedureId, ProtocolError, ScopeToken, StepInput, StepResult,
    TerminalStatus,
};
use super::resource_scale::{MeasurementError, ResourceMeter, Stage};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunPhase {
    Mutable,
    Frozen,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceProvenance {
    Observed,
    Incomplete,
}

/// The learner store registers the scope of an admitted procedure or hypothesis.
/// Every registration is additionally bound to this ledger's run scope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditScope {
    Run,
    Episode(ScopeToken),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionRecord {
    sequence: u64,
    input: StepInput,
    decision: Decision,
    phase: RunPhase,
}

impl DecisionRecord {
    pub(crate) const fn from_persisted(
        sequence: u64,
        input: StepInput,
        decision: Decision,
        phase: RunPhase,
    ) -> Self {
        Self {
            sequence,
            input,
            decision,
            phase,
        }
    }

    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
    pub const fn input(&self) -> StepInput {
        self.input
    }
    pub const fn run_scope(&self) -> ScopeToken {
        self.input.run_scope()
    }
    pub const fn episode_scope(&self) -> ScopeToken {
        self.input.episode_scope()
    }
    pub const fn step_index(&self) -> u8 {
        self.input.step_index()
    }
    pub const fn observation(&self) -> ObservationPacket {
        self.input.observation()
    }
    pub fn decision(&self) -> &Decision {
        &self.decision
    }
    pub const fn phase(&self) -> RunPhase {
        self.phase
    }

    /// Sorted keys, no whitespace, opaque identifiers as unsigned decimal
    /// strings (lossless even in consumers without native u64 integers).
    pub fn to_json(&self) -> String {
        format!(
            "{{\"decision\":{},\"input\":{},\"phase\":\"{}\",\"sequence\":{}}}",
            decision_json(&self.decision),
            input_json(self.input),
            match self.phase {
                RunPhase::Mutable => "mutable",
                RunPhase::Frozen => "frozen",
            },
            self.sequence,
        )
    }
}

/// Private fields ensure that a receipt is issued only after serialization.
/// A cloned receipt is useful for auditing/retries, but cannot be applied twice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionReceipt {
    record: DecisionRecord,
    evidence_id: EvidenceId,
}

impl DecisionReceipt {
    pub const fn sequence(&self) -> u64 {
        self.record.sequence
    }
    pub const fn action(&self) -> Option<ActionId> {
        self.record.decision.action()
    }
    pub const fn expects_result(&self) -> bool {
        self.action().is_some()
    }
    pub fn record(&self) -> &DecisionRecord {
        &self.record
    }
    pub fn decision(&self) -> &Decision {
        &self.record.decision
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceRecord {
    evidence_id: EvidenceId,
    decision_sequence: u64,
    run_scope: ScopeToken,
    episode_scope: ScopeToken,
    step_index: u8,
    before: ObservationPacket,
    action: Option<ActionId>,
    after: Option<ObservationPacket>,
    terminal: TerminalStatus,
    provenance: EvidenceProvenance,
}

impl EvidenceRecord {
    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn from_persisted(
        evidence_id: EvidenceId,
        decision_sequence: u64,
        run_scope: ScopeToken,
        episode_scope: ScopeToken,
        step_index: u8,
        before: ObservationPacket,
        action: Option<ActionId>,
        after: Option<ObservationPacket>,
        terminal: TerminalStatus,
        provenance: EvidenceProvenance,
    ) -> Self {
        Self {
            evidence_id,
            decision_sequence,
            run_scope,
            episode_scope,
            step_index,
            before,
            action,
            after,
            terminal,
            provenance,
        }
    }

    pub const fn evidence_id(&self) -> EvidenceId {
        self.evidence_id
    }
    pub const fn decision_sequence(&self) -> u64 {
        self.decision_sequence
    }
    pub const fn run_scope(&self) -> ScopeToken {
        self.run_scope
    }
    pub const fn episode_scope(&self) -> ScopeToken {
        self.episode_scope
    }
    pub const fn step_index(&self) -> u8 {
        self.step_index
    }
    pub const fn before(&self) -> ObservationPacket {
        self.before
    }
    pub const fn action(&self) -> Option<ActionId> {
        self.action
    }
    pub const fn after(&self) -> Option<ObservationPacket> {
        self.after
    }
    pub const fn terminal(&self) -> TerminalStatus {
        self.terminal
    }
    pub const fn provenance(&self) -> EvidenceProvenance {
        self.provenance
    }
}

/// `after` is None only for an abstention; predictions never populate evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractionRecord {
    decision: DecisionRecord,
    evidence: EvidenceRecord,
}

impl InteractionRecord {
    pub(crate) const fn from_persisted(decision: DecisionRecord, evidence: EvidenceRecord) -> Self {
        Self { decision, evidence }
    }

    pub fn decision(&self) -> &DecisionRecord {
        &self.decision
    }
    pub fn evidence(&self) -> &EvidenceRecord {
        &self.evidence
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LedgerEvent {
    Decision(DecisionRecord),
    Interaction(InteractionRecord),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LedgerError {
    Protocol(ProtocolError),
    Measurement(MeasurementError),
    InvalidRunScope {
        expected: ScopeToken,
        actual: ScopeToken,
    },
    EpisodeScopeReused(ScopeToken),
    UnexpectedEpisodeStep {
        expected: u8,
        actual: u8,
    },
    ContinuationMismatch,
    DecisionAlreadyPending(u64),
    NoPendingDecision,
    ReceiptMismatch(u64),
    ActionAlreadyAuthorized,
    ActionNotAuthorized,
    ResultForAbstention,
    ResultRequired,
    InvalidTerminalTiming,
    DuplicateLearnerReference {
        kind: AuditReferenceKind,
        value: u64,
    },
    UnknownLearnerReference {
        kind: AuditReferenceKind,
        value: u64,
    },
    ReferenceScopeMismatch,
    UncheckedProcedurePrediction,
    CounterOverflow,
    CannotFreezeActiveEpisode,
    FrozenLearnerMutation,
    AlreadyFrozen,
    EnvironmentResultMissing,
    EnvironmentResultAlreadyRecorded,
}

impl fmt::Display for LedgerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => fmt::Display::fmt(error, formatter),
            Self::Measurement(error) => fmt::Display::fmt(error, formatter),
            other => write!(formatter, "v2 ledger: {other:?}"),
        }
    }
}

impl std::error::Error for LedgerError {}
impl From<ProtocolError> for LedgerError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}
impl From<MeasurementError> for LedgerError {
    fn from(error: MeasurementError) -> Self {
        Self::Measurement(error)
    }
}

/// Errors returned while an environment resolves a receipt-backed turn.
///
/// The ledger variant is deliberately public so an environment adapter can
/// report protocol and ordering failures without erasing their cause.
#[derive(Debug)]
pub enum EnvironmentActionError<E> {
    Ledger(LedgerError),
    Environment(E),
}

impl<E: fmt::Display> fmt::Display for EnvironmentActionError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ledger(error) => error.fmt(formatter),
            Self::Environment(error) => error.fmt(formatter),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for EnvironmentActionError<E> {}

impl<E> From<LedgerError> for EnvironmentActionError<E> {
    fn from(error: LedgerError) -> Self {
        Self::Ledger(error)
    }
}

/// A receipt-backed environment turn.
///
/// The turn is created only from a pending [`DecisionReceipt`].  Calling
/// [`EnvironmentTurn::resolve`] authorizes the action in the ledger before the
/// supplied environment closure runs, then records the returned opaque result
/// in the same ledger.  Abstentions are closed as incomplete without invoking
/// the closure.  This keeps decision-before-action/result ordering in the
/// learner-owned ledger rather than in a runner convention.
pub struct EnvironmentTurn<'a> {
    ledger: &'a mut InteractionLedger,
    receipt: DecisionReceipt,
    result: Option<Option<StepResult>>,
}

impl<'a> EnvironmentTurn<'a> {
    pub const fn action(&self) -> Option<ActionId> {
        self.receipt.action()
    }

    pub fn decision(&self) -> &Decision {
        self.receipt.decision()
    }

    pub const fn is_complete(&self) -> bool {
        self.result.is_some()
    }

    /// Apply the receipt's action and record its result as one atomic protocol
    /// operation.  The closure cannot run until the receipt has been validated
    /// and the ledger has authorized the action.
    pub fn resolve<E, F>(
        &mut self,
        apply: F,
    ) -> Result<Option<StepResult>, EnvironmentActionError<E>>
    where
        E: std::error::Error,
        F: FnOnce(ActionId) -> Result<StepResult, E>,
    {
        if self.result.is_some() {
            return Err(LedgerError::EnvironmentResultAlreadyRecorded.into());
        }

        let result = match self.receipt.action() {
            Some(action) => {
                self.ledger.authorize_action(&self.receipt)?;
                let result = apply(action).map_err(EnvironmentActionError::Environment)?;
                self.ledger.record_result(&self.receipt, Some(result))?;
                Some(result)
            }
            None => {
                self.ledger.record_result(&self.receipt, None)?;
                None
            }
        };
        self.result = Some(result);
        Ok(result)
    }

    /// Complete a compatibility adapter that has already recorded the exact
    /// receipt in this ledger. The v2 evaluator bridge calls this from its
    /// adapter module; reusable environments should use [`Self::resolve`].
    pub(crate) fn adopt_recorded(&mut self, result: Option<StepResult>) -> Result<(), LedgerError> {
        if self.result.is_some() {
            return Err(LedgerError::EnvironmentResultAlreadyRecorded);
        }
        let interaction = self
            .ledger
            .interactions
            .last()
            .ok_or(LedgerError::EnvironmentResultMissing)?;
        if interaction.decision().sequence() != self.receipt.sequence()
            || self.ledger.pending.is_some()
            || interaction.evidence().after() != result.map(StepResult::observation)
            || interaction.evidence().terminal()
                != result
                    .map(StepResult::terminal)
                    .unwrap_or(TerminalStatus::Incomplete)
        {
            return Err(LedgerError::EnvironmentResultMissing);
        }
        self.result = Some(result);
        Ok(())
    }

    pub(crate) fn ledger_mut(&mut self) -> &mut InteractionLedger {
        self.ledger
    }

    pub(crate) fn receipt(&self) -> &DecisionReceipt {
        &self.receipt
    }

    pub fn finish(self) -> Result<Option<StepResult>, LedgerError> {
        self.result.ok_or(LedgerError::EnvironmentResultMissing)
    }
}

pub struct InteractionLedger {
    run_scope: ScopeToken,
    phase: RunPhase,
    next_sequence: u64,
    used_episode_scopes: BTreeSet<ScopeToken>,
    continuation: Option<StepInput>,
    pending: Option<DecisionReceipt>,
    prepared_input: Option<StepInput>,
    action_authorized: bool,
    decisions: Vec<DecisionRecord>,
    interactions: Vec<InteractionRecord>,
    events: Vec<LedgerEvent>,
    decision_trace_jsonl: String,
    // Registration is an audit seam for the future admitted learner store,
    // not a hidden-world oracle or an implementation of admission.
    known_procedures: BTreeMap<ProcedureId, (AuditScope, bool)>,
    known_evidence: BTreeSet<EvidenceId>,
    known_hypotheses: BTreeMap<HypothesisId, AuditScope>,
    resource_meter: ResourceMeter,
}

impl InteractionLedger {
    pub fn new(run_scope: ScopeToken) -> Result<Self, LedgerError> {
        Self::with_resource_meter(run_scope, ResourceMeter::runtime())
    }

    pub fn with_resource_meter(
        run_scope: ScopeToken,
        meter: ResourceMeter,
    ) -> Result<Self, LedgerError> {
        if run_scope.is_zero() {
            return Err(ProtocolError::ZeroScopeToken.into());
        }
        Ok(Self {
            run_scope,
            phase: RunPhase::Mutable,
            next_sequence: 1,
            used_episode_scopes: BTreeSet::new(),
            continuation: None,
            pending: None,
            prepared_input: None,
            action_authorized: false,
            decisions: Vec::new(),
            interactions: Vec::new(),
            events: Vec::new(),
            decision_trace_jsonl: String::new(),
            known_procedures: BTreeMap::new(),
            known_evidence: BTreeSet::new(),
            known_hypotheses: BTreeMap::new(),
            resource_meter: meter,
        })
    }

    pub const fn run_scope(&self) -> ScopeToken {
        self.run_scope
    }
    pub const fn phase(&self) -> RunPhase {
        self.phase
    }
    pub const fn is_frozen(&self) -> bool {
        matches!(self.phase, RunPhase::Frozen)
    }

    pub fn freeze(&mut self) -> Result<(), LedgerError> {
        self.validate_freeze()?;
        self.commit_freeze();
        Ok(())
    }

    pub(crate) fn validate_freeze(&self) -> Result<(), LedgerError> {
        if self.is_frozen() {
            return Err(LedgerError::AlreadyFrozen);
        }
        if self.pending.is_some() || self.continuation.is_some() {
            return Err(LedgerError::CannotFreezeActiveEpisode);
        }
        Ok(())
    }

    pub(crate) fn commit_freeze(&mut self) {
        debug_assert!(self.validate_freeze().is_ok());
        self.phase = RunPhase::Frozen;
    }

    /// Called by the learner's admitted-procedure store before a decision.
    /// Completed output checks require a prediction; pending checks forbid one.
    pub fn register_procedure(
        &mut self,
        id: ProcedureId,
        scope: AuditScope,
        output_checks_complete: bool,
    ) -> Result<(), LedgerError> {
        self.validate_registration_scope(scope)?;
        if self.known_procedures.contains_key(&id) {
            return Err(LedgerError::DuplicateLearnerReference {
                kind: AuditReferenceKind::Procedure,
                value: id.value(),
            });
        }
        self.known_procedures
            .insert(id, (scope, output_checks_complete));
        Ok(())
    }

    pub fn register_hypothesis(
        &mut self,
        id: HypothesisId,
        scope: AuditScope,
    ) -> Result<(), LedgerError> {
        self.validate_registration_scope(scope)?;
        if self.known_hypotheses.contains_key(&id) {
            return Err(LedgerError::DuplicateLearnerReference {
                kind: AuditReferenceKind::Hypothesis,
                value: id.value(),
            });
        }
        self.known_hypotheses.insert(id, scope);
        Ok(())
    }

    pub(super) fn ensure_procedure(
        &mut self,
        id: ProcedureId,
        scope: AuditScope,
        checked: bool,
    ) -> Result<(), LedgerError> {
        self.validate_registration_scope(scope)?;
        match self.known_procedures.get(&id) {
            Some(existing) if *existing == (scope, checked) => Ok(()),
            Some(_) => Err(LedgerError::ReferenceScopeMismatch),
            None => self.register_procedure(id, scope, checked),
        }
    }

    pub(super) fn ensure_hypothesis(
        &mut self,
        id: HypothesisId,
        scope: AuditScope,
    ) -> Result<(), LedgerError> {
        self.validate_registration_scope(scope)?;
        match self.known_hypotheses.get(&id) {
            Some(existing) if *existing == scope => Ok(()),
            Some(_) => Err(LedgerError::ReferenceScopeMismatch),
            None => self.register_hypothesis(id, scope),
        }
    }

    pub(super) fn has_evidence(&self, id: EvidenceId) -> bool {
        self.known_evidence.contains(&id)
    }

    /// Open the logical decision window before selection work. A retry cannot
    /// reset counters or switch inputs and thereby recover an exhausted budget.
    pub(super) fn prepare_decision(&mut self, input: StepInput) -> Result<(), LedgerError> {
        if let Some(pending) = &self.pending {
            return Err(LedgerError::DecisionAlreadyPending(pending.sequence()));
        }
        if input.run_scope() != self.run_scope {
            return Err(LedgerError::InvalidRunScope {
                expected: self.run_scope,
                actual: input.run_scope(),
            });
        }
        self.validate_episode_position(input)?;
        match self.prepared_input {
            Some(prepared) if prepared != input => return Err(LedgerError::ContinuationMismatch),
            Some(_) => return Ok(()),
            None => {}
        }
        if input.step_index() == 0 {
            self.resource_meter.begin_episode();
        }
        self.resource_meter.begin_decision();
        self.prepared_input = Some(input);
        Ok(())
    }

    fn validate_registration_scope(&self, scope: AuditScope) -> Result<(), LedgerError> {
        if self.is_frozen() {
            return Err(LedgerError::FrozenLearnerMutation);
        }
        if let AuditScope::Episode(token) = scope {
            if !self.used_episode_scopes.contains(&token) {
                return Err(LedgerError::ReferenceScopeMismatch);
            }
        }
        Ok(())
    }

    /// Accept one live learner response, validate it, charge its query, and
    /// serialize it before returning the receipt required for result reveal.
    /// Invalid transport attempts do not advance the episode or consume a slot.
    pub fn record_decision(
        &mut self,
        input: StepInput,
        decision: Decision,
    ) -> Result<DecisionReceipt, LedgerError> {
        if let Some(pending) = &self.pending {
            return Err(LedgerError::DecisionAlreadyPending(pending.sequence()));
        }
        if input.run_scope() != self.run_scope {
            return Err(LedgerError::InvalidRunScope {
                expected: self.run_scope,
                actual: input.run_scope(),
            });
        }
        decision.validate_for(&input)?;
        self.validate_references(&decision, input.episode_scope())?;
        self.validate_episode_position(input)?;
        let next = self
            .next_sequence
            .checked_add(1)
            .ok_or(LedgerError::CounterOverflow)?;
        let record = DecisionRecord {
            sequence: self.next_sequence,
            input,
            decision,
            phase: self.phase,
        };
        let serialized = record.to_json();
        self.prepare_decision(input)?;
        self.resource_meter.record_query_call()?;

        self.next_sequence = next;
        self.used_episode_scopes.insert(input.episode_scope());
        self.continuation = None;
        self.decision_trace_jsonl.push_str(&serialized);
        self.decision_trace_jsonl.push('\n');
        self.decisions.push(record.clone());
        self.events.push(LedgerEvent::Decision(record.clone()));
        let receipt = DecisionReceipt {
            evidence_id: EvidenceId::new(record.sequence),
            record,
        };
        self.pending = Some(receipt.clone());
        self.action_authorized = false;
        self.prepared_input = None;
        Ok(receipt)
    }

    /// Open an environment turn for the exact pending receipt.  The receipt is
    /// the only capability an environment needs to resolve a decision; a
    /// fabricated or already-consumed receipt is rejected before any action
    /// callback can run.
    pub fn begin_environment_turn(
        &mut self,
        receipt: &DecisionReceipt,
    ) -> Result<EnvironmentTurn<'_>, LedgerError> {
        self.validate_receipt(receipt)?;
        Ok(EnvironmentTurn {
            ledger: self,
            receipt: receipt.clone(),
            result: None,
        })
    }

    pub(crate) fn validate_receipt(&self, receipt: &DecisionReceipt) -> Result<(), LedgerError> {
        let pending = self
            .pending
            .as_ref()
            .ok_or(LedgerError::NoPendingDecision)?;
        if pending != receipt {
            return Err(LedgerError::ReceiptMismatch(receipt.sequence()));
        }
        Ok(())
    }

    /// Pre-crossing action charge: invoked only after the evaluator checks the
    /// receipt against its currently issued input, before changing hidden state.
    pub(crate) fn authorize_action(
        &mut self,
        receipt: &DecisionReceipt,
    ) -> Result<(), LedgerError> {
        self.validate_receipt(receipt)?;
        if receipt.action().is_none() {
            return Err(LedgerError::ResultForAbstention);
        }
        if self.action_authorized {
            return Err(LedgerError::ActionAlreadyAuthorized);
        }
        self.resource_meter.record_environment_action()?;
        self.action_authorized = true;
        Ok(())
    }

    /// Record an opaque post-action result for a decision that has already been
    /// serialized.  This is the learner-owned transport seam used by adapters
    /// and tests; it accepts no evaluator metadata and never copies a
    /// prediction into the evidence record.
    pub fn record_observed_result(
        &mut self,
        receipt: &DecisionReceipt,
        result: StepResult,
    ) -> Result<EvidenceRecord, LedgerError> {
        self.authorize_action(receipt)?;
        self.record_result(receipt, Some(result))
    }

    /// For transport adapters: a result is accepted only for the exact pending
    /// receipt and after action authorization. No prediction is ever copied here.
    pub fn record_result(
        &mut self,
        receipt: &DecisionReceipt,
        result: Option<StepResult>,
    ) -> Result<EvidenceRecord, LedgerError> {
        self.validate_receipt(receipt)?;
        let (after, terminal, provenance) = match (receipt.action(), result) {
            (None, None) => (
                None,
                TerminalStatus::Incomplete,
                EvidenceProvenance::Incomplete,
            ),
            (None, Some(_)) => return Err(LedgerError::ResultForAbstention),
            (Some(_), None) => return Err(LedgerError::ResultRequired),
            (Some(_), Some(result)) => {
                if !self.action_authorized {
                    return Err(LedgerError::ActionNotAuthorized);
                }
                match (receipt.record.step_index(), result.terminal()) {
                    (0, TerminalStatus::Failure) | (1, TerminalStatus::Continue) => {
                        return Err(LedgerError::InvalidTerminalTiming);
                    }
                    _ => {}
                }
                (
                    Some(result.observation()),
                    result.terminal(),
                    EvidenceProvenance::Observed,
                )
            }
        };
        let input = receipt.record.input;
        let continuation = if let Some(result) = result {
            if result.terminal() == TerminalStatus::Continue {
                Some(StepInput::new(
                    input.run_scope(),
                    input.episode_scope(),
                    1,
                    result.observation(),
                    input.available_actions(),
                    1,
                    PreviousTerminal::Continue,
                )?)
            } else {
                None
            }
        } else {
            None
        };
        let evidence = EvidenceRecord {
            evidence_id: receipt.evidence_id,
            decision_sequence: receipt.sequence(),
            run_scope: input.run_scope(),
            episode_scope: input.episode_scope(),
            step_index: input.step_index(),
            before: input.observation(),
            action: receipt.action(),
            after,
            terminal,
            provenance,
        };
        let interaction = InteractionRecord {
            decision: receipt.record.clone(),
            evidence: evidence.clone(),
        };
        self.resource_meter.record_ledger_record()?;
        if matches!(receipt.record.phase, RunPhase::Mutable) {
            self.known_evidence.insert(evidence.evidence_id);
        }
        self.interactions.push(interaction.clone());
        self.events.push(LedgerEvent::Interaction(interaction));
        self.continuation = continuation;
        self.pending = None;
        self.action_authorized = false;
        Ok(evidence)
    }

    fn validate_episode_position(&self, input: StepInput) -> Result<(), LedgerError> {
        if let Some(expected) = self.continuation {
            if expected != input {
                return Err(LedgerError::ContinuationMismatch);
            }
        } else {
            if input.step_index() != 0 {
                return Err(LedgerError::UnexpectedEpisodeStep {
                    expected: 0,
                    actual: input.step_index(),
                });
            }
            if self.used_episode_scopes.contains(&input.episode_scope()) {
                return Err(LedgerError::EpisodeScopeReused(input.episode_scope()));
            }
        }
        Ok(())
    }

    fn validate_references(
        &self,
        decision: &Decision,
        episode: ScopeToken,
    ) -> Result<(), LedgerError> {
        let audit = decision.audit();
        for id in audit.procedure_refs() {
            let (scope, checked) =
                self.known_procedures
                    .get(id)
                    .ok_or(LedgerError::UnknownLearnerReference {
                        kind: AuditReferenceKind::Procedure,
                        value: id.value(),
                    })?;
            validate_reference_scope(*scope, episode)?;
            if let Decision::Act(act) = decision {
                if act.prediction().is_some() && !checked {
                    return Err(LedgerError::UncheckedProcedurePrediction);
                }
                if act.mode() == ActMode::Procedure && *checked && act.prediction().is_none() {
                    return Err(ProtocolError::ProcedurePredictionRequired.into());
                }
            }
        }
        for id in audit.evidence_refs() {
            if !self.known_evidence.contains(id) {
                return Err(LedgerError::UnknownLearnerReference {
                    kind: AuditReferenceKind::Evidence,
                    value: id.value(),
                });
            }
        }
        for id in audit.hypothesis_refs() {
            let scope =
                self.known_hypotheses
                    .get(id)
                    .ok_or(LedgerError::UnknownLearnerReference {
                        kind: AuditReferenceKind::Hypothesis,
                        value: id.value(),
                    })?;
            validate_reference_scope(*scope, episode)?;
        }
        Ok(())
    }

    pub fn pending_decision(&self) -> Option<&DecisionReceipt> {
        self.pending.as_ref()
    }
    pub fn decision_count(&self) -> usize {
        self.decisions.len()
    }
    pub fn interaction_count(&self) -> usize {
        self.interactions.len()
    }
    pub fn decisions(&self) -> &[DecisionRecord] {
        &self.decisions
    }
    pub fn interactions(&self) -> &[InteractionRecord] {
        &self.interactions
    }
    pub fn events(&self) -> &[LedgerEvent] {
        &self.events
    }
    pub fn decision_trace_jsonl(&self) -> &str {
        &self.decision_trace_jsonl
    }
    pub fn resource_meter(&self) -> &ResourceMeter {
        &self.resource_meter
    }

    pub(crate) fn is_checkpoint_ready(&self) -> bool {
        self.pending.is_none() && self.prepared_input.is_none() && !self.action_authorized
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_persisted(
        run_scope: ScopeToken,
        phase: RunPhase,
        decisions: Vec<DecisionRecord>,
        interactions: Vec<InteractionRecord>,
        known_procedures: Vec<(ProcedureId, AuditScope, bool)>,
        known_evidence: BTreeSet<EvidenceId>,
        known_hypotheses: Vec<(HypothesisId, AuditScope)>,
        resource_meter: ResourceMeter,
    ) -> Self {
        let used_episode_scopes = decisions
            .iter()
            .map(DecisionRecord::episode_scope)
            .collect::<BTreeSet<_>>();
        let continuation = interactions.last().and_then(|interaction| {
            (interaction.evidence().terminal() == TerminalStatus::Continue).then(|| {
                let input = interaction.decision().input();
                StepInput::new(
                    input.run_scope(),
                    input.episode_scope(),
                    1,
                    interaction.evidence().after().expect("continued result"),
                    input.available_actions(),
                    1,
                    PreviousTerminal::Continue,
                )
                .expect("validated persisted continuation")
            })
        });
        let mut decision_trace_jsonl = String::new();
        let mut events = Vec::with_capacity(decisions.len() + interactions.len());
        for decision in &decisions {
            decision_trace_jsonl.push_str(&decision.to_json());
            decision_trace_jsonl.push('\n');
        }
        // The event stream is interleaved in live operation. Rebuild it in the
        // original decision/interaction order rather than the grouped storage
        // order used by the persistence representation.
        for (index, decision) in decisions.iter().enumerate() {
            events.push(LedgerEvent::Decision(decision.clone()));
            if let Some(interaction) = interactions.get(index) {
                events.push(LedgerEvent::Interaction(interaction.clone()));
            }
        }
        Self {
            run_scope,
            phase,
            next_sequence: decisions.len() as u64 + 1,
            used_episode_scopes,
            continuation,
            pending: None,
            prepared_input: None,
            action_authorized: false,
            decisions,
            interactions,
            events,
            decision_trace_jsonl,
            known_procedures: known_procedures
                .into_iter()
                .map(|(id, scope, checked)| (id, (scope, checked)))
                .collect(),
            known_evidence,
            known_hypotheses: known_hypotheses.into_iter().collect(),
            resource_meter,
        }
    }

    pub(crate) fn known_procedures(&self) -> Vec<(ProcedureId, AuditScope, bool)> {
        self.known_procedures
            .iter()
            .map(|(id, (scope, checked))| (*id, *scope, *checked))
            .collect()
    }

    pub(crate) fn known_evidence(&self) -> Vec<EvidenceId> {
        self.known_evidence.iter().copied().collect()
    }

    pub(crate) fn known_hypotheses(&self) -> Vec<(HypothesisId, AuditScope)> {
        self.known_hypotheses
            .iter()
            .map(|(id, scope)| (*id, *scope))
            .collect()
    }

    /// Record a reportable stage operation owned by the runner or an adapter.
    /// The operation is still charged to this ledger's single learner-side
    /// meter; callers cannot replace the meter or write evaluator counters.
    pub fn record_stage_operation(&mut self, stage: Stage, amount: u64) -> Result<(), LedgerError> {
        self.resource_meter.record_stage_operation(stage, amount)?;
        Ok(())
    }

    /// Mutable access is limited to learner-owned logical work.  The evaluator
    /// still reaches the same meter through this ledger, so decision/action and
    /// grounding costs cannot silently diverge into a second accounting seam.
    pub(crate) fn resource_meter_mut(&mut self) -> &mut ResourceMeter {
        &mut self.resource_meter
    }
}

fn validate_reference_scope(scope: AuditScope, episode: ScopeToken) -> Result<(), LedgerError> {
    match scope {
        AuditScope::Run => Ok(()),
        AuditScope::Episode(token) if token == episode => Ok(()),
        _ => Err(LedgerError::ReferenceScopeMismatch),
    }
}

fn packet_json(packet: ObservationPacket) -> String {
    let [a, b, c] = packet.atoms();
    format!("{{\"atoms\":[\"{a}\",\"{b}\",\"{c}\"]}}")
}

fn input_json(input: StepInput) -> String {
    let [a, b] = input.available_actions();
    format!(
        "{{\"available_actions\":[\"{a}\",\"{b}\"],\"episode_scope\":\"{}\",\"observation\":{},\"previous_terminal\":\"{}\",\"remaining_action_budget\":{},\"run_scope\":\"{}\",\"step_index\":{}}}",
        input.episode_scope(), packet_json(input.observation()),
        match input.previous_terminal() { PreviousTerminal::None => "none", PreviousTerminal::Continue => "continue" },
        input.remaining_action_budget(), input.run_scope(), input.step_index(),
    )
}

fn ids_json(ids: impl Iterator<Item = u64>) -> String {
    format!(
        "[{}]",
        ids.map(|id| format!("\"{id}\""))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn decision_json(decision: &Decision) -> String {
    let audit = decision.audit();
    let procedures = ids_json(audit.procedure_refs().iter().map(|id| id.value()));
    let evidence = ids_json(audit.evidence_refs().iter().map(|id| id.value()));
    let hypotheses = ids_json(audit.hypothesis_refs().iter().map(|id| id.value()));
    match decision {
        Decision::Act(act) => format!(
            "{{\"action\":\"{}\",\"evidence_refs\":{},\"hypothesis_refs\":{},\"mode\":\"{}\",\"prediction\":{},\"procedure_refs\":{},\"type\":\"act\"}}",
            act.action(), evidence, hypotheses,
            match act.mode() { ActMode::Exploration => "exploration", ActMode::Procedure => "procedure", ActMode::Fallback => "fallback" },
            act.prediction().map(packet_json).unwrap_or_else(|| "null".to_owned()), procedures,
        ),
        Decision::Abstain(abstain) => {
            use super::opaque_protocol::AbstainReason;
            let reason = match abstain.reason() {
                AbstainReason::NoApplicableProcedure => "no_applicable_procedure",
                AbstainReason::Conflict => "conflict",
                AbstainReason::InsufficientEvidence => "insufficient_evidence",
                AbstainReason::UnsupportedByGrammar => "unsupported_by_grammar",
                AbstainReason::IncompleteBudget => "incomplete_budget",
            };
            format!("{{\"evidence_refs\":{evidence},\"hypothesis_refs\":{hypotheses},\"procedure_refs\":{procedures},\"reason\":\"{reason}\",\"type\":\"abstain\"}}")
        }
    }
}
