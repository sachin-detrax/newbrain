mod snapshot;
#[cfg(test)]
mod snapshot_tests;
use crate::admission::{admit_with_policy, AdmissionResult};
use crate::budget::{BudgetConfig, BudgetError, BudgetKind as K, BudgetScope as S, BudgetState};
use crate::grammar::{enumerate_g0, enumerate_g1, GenerationError};
use crate::ledger::{CostVector, Ledger};
use crate::types::{EvidenceRef, RecordId, Transition};
pub use snapshot::{LearnerSnapshot, QueryExecution};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LearnerConfig {
    development_gate: bool,
}
impl LearnerConfig {
    pub fn v1() -> Self {
        Self {
            development_gate: true,
        }
    }
    pub fn without_development_gate() -> Self {
        Self {
            development_gate: false,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrainingError {
    Generation(GenerationError),
    Budget(BudgetError),
    RepeatedRecord(RecordId),
}
impl From<BudgetError> for TrainingError {
    fn from(e: BudgetError) -> Self {
        Self::Budget(e)
    }
}
impl From<GenerationError> for TrainingError {
    fn from(e: GenerationError) -> Self {
        Self::Generation(e)
    }
}
impl fmt::Display for TrainingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for TrainingError {}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrainingReport {
    pub admitted: usize,
    pub rejected: usize,
    pub cost: CostVector,
}

pub struct Learner {
    config: LearnerConfig,
    state: LearnerSnapshot,
    training: Vec<Transition>,
    record_ids: BTreeSet<RecordId>,
    budget: BudgetState,
    ledger: Ledger,
}
impl Learner {
    pub fn new(config: LearnerConfig) -> Self {
        Self {
            config,
            state: LearnerSnapshot::default(),
            training: Vec::new(),
            record_ids: BTreeSet::new(),
            budget: BudgetState::new(BudgetConfig::v1()),
            ledger: Ledger::new(),
        }
    }
    pub fn train_phase_a(
        &mut self,
        records: &[Transition],
        development: &[Transition],
    ) -> Result<TrainingReport, TrainingError> {
        self.train(records, development)
    }
    pub fn train_phase_b(
        &mut self,
        records: &[Transition],
        development: &[Transition],
    ) -> Result<TrainingReport, TrainingError> {
        self.train(records, development)
    }
    pub fn freeze(&self) -> LearnerSnapshot {
        self.state.clone()
    }
    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }
    pub fn retained_record_ids(&self) -> &BTreeSet<RecordId> {
        &self.record_ids
    }
    pub fn into_ledger(self) -> Ledger {
        self.ledger
    }

    fn train(
        &mut self,
        records: &[Transition],
        development: &[Transition],
    ) -> Result<TrainingReport, TrainingError> {
        self.budget.reset_local(S::TrainingUpdate);
        let result = self.train_inner(records, development);
        if let Err(
            TrainingError::Budget(error)
            | TrainingError::Generation(GenerationError::Budget(error)),
        ) = &result
        {
            self.ledger.record_budget_failure(
                error,
                None,
                Vec::new(),
                CostVector::capture(&self.budget, S::TrainingUpdate),
            )?;
        }
        result
    }
    fn train_inner(
        &mut self,
        records: &[Transition],
        development: &[Transition],
    ) -> Result<TrainingReport, TrainingError> {
        let rejected_before = self.ledger.rejected_candidates().len();
        let mut seen = self.record_ids.clone();
        for r in records.iter().chain(development) {
            if !seen.insert(r.record_id) {
                return Err(TrainingError::RepeatedRecord(r.record_id));
            }
        }
        let mut all_training = self.training.clone();
        all_training.extend_from_slice(records);
        let active = u64::try_from(all_training.len() + development.len())
            .map_err(|_| GenerationError::IdentifierExhausted)?;
        self.budget.set_active(K::ActiveTransitionRecords, active)?;
        for _ in records.iter().chain(development) {
            self.budget.charge_total(K::TotalEventRecords, 1)?;
            self.budget.charge(K::OperationSteps, 1)?;
        }
        let exact = enumerate_g0(records, &mut self.budget, &mut self.ledger)?;
        for c in exact {
            self.ledger.record_rejection_with_evidence(
                c.id(),
                "exact-memory-only",
                c.evidence(),
                CostVector::capture(&self.budget, S::TrainingUpdate),
            )?;
        }
        let generated = enumerate_g1(records, &mut self.budget, &mut self.ledger)?;
        let mut staged = Vec::new();
        for candidate in generated {
            match admit_with_policy(
                candidate,
                &all_training,
                development,
                &mut self.budget,
                &mut self.ledger,
                self.config.development_gate,
            )? {
                AdmissionResult::Admitted(p) => staged.push(*p),
                AdmissionResult::Rejected(_) => {}
            }
        }
        let report = TrainingReport {
            admitted: staged.len(),
            rejected: self.ledger.rejected_candidates().len() - rejected_before,
            cost: CostVector::capture(&self.budget, S::TrainingUpdate),
        };
        self.state.extend(staged);
        self.training = all_training;
        self.record_ids = seen;
        Ok(report)
    }
}

pub fn evidence_for(records: &[Transition]) -> Vec<EvidenceRef> {
    records
        .iter()
        .map(|r| EvidenceRef::record(r.record_id))
        .collect()
}
