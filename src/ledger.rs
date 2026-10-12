use crate::budget::{BudgetConfig, BudgetError, BudgetKind, BudgetScope, BudgetState};
use crate::types::{EvidenceRef, ProcedureId, Response};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CostVector {
    pub candidate_pairs: u64,
    pub generated_candidates: u64,
    pub support_comparisons: u64,
    pub development_checks: u64,
    pub operation_steps: u64,
    pub active_transition_records: u64,
    pub active_candidate_records: u64,
    pub retrieved_candidates: u64,
    pub candidate_checks: u64,
    pub active_records: u64,
}

impl CostVector {
    pub fn capture(budget: &BudgetState, scope: BudgetScope) -> Self {
        let used = |kind| budget.used_in(scope, kind);
        let peak = |kind| budget.peak_in(scope, kind);
        Self {
            candidate_pairs: used(BudgetKind::CandidatePairs),
            generated_candidates: used(BudgetKind::GeneratedCandidates),
            support_comparisons: used(BudgetKind::SupportComparisons),
            development_checks: used(BudgetKind::DevelopmentChecks),
            operation_steps: used(BudgetKind::OperationSteps),
            active_transition_records: peak(BudgetKind::ActiveTransitionRecords),
            active_candidate_records: peak(BudgetKind::ActiveCandidateRecords),
            retrieved_candidates: used(BudgetKind::RetrievedCandidates),
            candidate_checks: used(BudgetKind::CandidateChecks),
            active_records: peak(BudgetKind::ActiveRecords),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LedgerEventKind {
    CandidateGenerated,
    Check,
    CandidateAdmitted,
    CandidateRejected,
    BudgetExceeded,
    Note,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LedgerEvent {
    pub sequence: u64,
    pub kind: LedgerEventKind,
    pub procedure_ids: Vec<ProcedureId>,
    pub evidence_refs: Vec<EvidenceRef>,
    pub response_status: Option<String>,
    pub reason: String,
    /// Cumulative snapshot within the current update or query, not a delta.
    pub cost: CostVector,
}

/// One run owns one ledger. All appends charge its private, non-resettable
/// retention budget. Exhaustion returns an error without adding a 129th event;
/// the caller must propagate it as run failure/incomplete, not log recursively.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ledger {
    events: Vec<LedgerEvent>,
    retention: BudgetState,
}

impl Default for Ledger {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            retention: BudgetState::new(BudgetConfig::v1()),
        }
    }
}

impl Ledger {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn events(&self) -> &[LedgerEvent] {
        &self.events
    }
    pub fn rejected_candidates(&self) -> Vec<&LedgerEvent> {
        self.events
            .iter()
            .filter(|e| e.kind == LedgerEventKind::CandidateRejected)
            .collect()
    }

    pub fn record_candidate_generated(
        &mut self,
        id: ProcedureId,
        refs: Vec<EvidenceRef>,
        cost: CostVector,
    ) -> Result<(), BudgetError> {
        self.append(LedgerEvent {
            sequence: 0,
            kind: LedgerEventKind::CandidateGenerated,
            procedure_ids: vec![id],
            evidence_refs: refs,
            response_status: None,
            reason: "candidate-generated".into(),
            cost,
        })
    }
    pub fn record_check(
        &mut self,
        id: ProcedureId,
        passed: bool,
        reason: impl Into<String>,
        refs: Vec<EvidenceRef>,
        cost: CostVector,
    ) -> Result<(), BudgetError> {
        self.append(LedgerEvent {
            sequence: 0,
            kind: LedgerEventKind::Check,
            procedure_ids: vec![id],
            evidence_refs: refs,
            response_status: Some(if passed { "passed" } else { "failed" }.into()),
            reason: reason.into(),
            cost,
        })
    }
    pub fn record_admission(
        &mut self,
        id: ProcedureId,
        refs: Vec<EvidenceRef>,
        cost: CostVector,
    ) -> Result<(), BudgetError> {
        self.append(LedgerEvent {
            sequence: 0,
            kind: LedgerEventKind::CandidateAdmitted,
            procedure_ids: vec![id],
            evidence_refs: refs,
            response_status: Some("admitted".into()),
            reason: "candidate-admitted".into(),
            cost,
        })
    }
    pub fn record_rejection(
        &mut self,
        id: ProcedureId,
        reason: impl Into<String>,
        cost: CostVector,
    ) -> Result<(), BudgetError> {
        self.record_rejection_with_evidence(id, reason, Vec::new(), cost)
    }
    pub fn record_rejection_with_evidence(
        &mut self,
        id: ProcedureId,
        reason: impl Into<String>,
        refs: Vec<EvidenceRef>,
        cost: CostVector,
    ) -> Result<(), BudgetError> {
        self.append(LedgerEvent {
            sequence: 0,
            kind: LedgerEventKind::CandidateRejected,
            procedure_ids: vec![id],
            evidence_refs: refs,
            response_status: None,
            reason: reason.into(),
            cost,
        })
    }
    pub fn record_budget_failure(
        &mut self,
        error: &BudgetError,
        id: Option<ProcedureId>,
        refs: Vec<EvidenceRef>,
        cost: CostVector,
    ) -> Result<(), BudgetError> {
        self.append(LedgerEvent {
            sequence: 0,
            kind: LedgerEventKind::BudgetExceeded,
            procedure_ids: id.into_iter().collect(),
            evidence_refs: refs,
            response_status: Some("incomplete".into()),
            reason: format!("scope={:?}; {error}", error.scope()),
            cost,
        })
    }
    pub fn record_response(
        &mut self,
        response: &Response,
        cost: CostVector,
    ) -> Result<(), BudgetError> {
        let (status, ids, refs, reason) = match response {
            Response::Prediction(_) => (
                "prediction".into(),
                response.procedure_ids(),
                response.evidence_refs(),
                format!(
                    "output={}",
                    response.output().expect("prediction has output")
                ),
            ),
            Response::Abstention(_) => {
                let response_reason = response.reason().expect("abstention has a reason");
                (
                    format!("abstention:{response_reason:?}"),
                    response.procedure_ids(),
                    response.evidence_refs(),
                    format!("{response_reason:?}"),
                )
            }
        };
        self.append(LedgerEvent {
            sequence: 0,
            kind: LedgerEventKind::Note,
            procedure_ids: ids.to_vec(),
            evidence_refs: refs.to_vec(),
            response_status: Some(status),
            reason,
            cost,
        })
    }

    pub fn stable_text(&self) -> String {
        self.events
            .iter()
            .map(|e| {
                format!(
                    "{}|{:?}|{:?}|{}|evidence={:?}|cost={:?}|{}\n",
                    e.sequence,
                    e.kind,
                    e.procedure_ids,
                    escape_field(e.response_status.as_deref().unwrap_or("")),
                    e.evidence_refs,
                    e.cost,
                    escape_field(&e.reason)
                )
            })
            .collect()
    }

    fn append(&mut self, mut event: LedgerEvent) -> Result<(), BudgetError> {
        let sequence = self
            .retention
            .used_in(BudgetScope::TotalRun, BudgetKind::TotalEventRecords);
        self.retention
            .charge_total(BudgetKind::TotalEventRecords, 1)?;
        event.sequence = sequence;
        self.events.push(event);
        Ok(())
    }
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
