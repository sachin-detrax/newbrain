use crate::budget::{BudgetConfig, BudgetError, BudgetKind as K, BudgetScope as S, BudgetState};
use crate::engine::{LearnerSnapshot, QueryExecution};
use crate::grammar::{enumerate_g0, Candidate, GenerationError};
use crate::ledger::{CostVector, Ledger};
use crate::types::{AbstentionReason, EvidenceRef, ProcedureId, Query, Response, Transition};

pub trait Predictor {
    fn predict(&self, query: &Query) -> QueryExecution;
}
impl Predictor for LearnerSnapshot {
    fn predict(&self, query: &Query) -> QueryExecution {
        self.predict(query)
    }
}

#[derive(Default)]
pub struct ExactRetrievalBaseline {
    records: Vec<Transition>,
}
impl ExactRetrievalBaseline {
    pub fn new(records: &[Transition]) -> Self {
        Self {
            records: records.to_vec(),
        }
    }
    fn evaluate(&self, q: &Query, b: &mut BudgetState) -> Result<Response, BudgetError> {
        b.set_active_in(S::Query, K::ActiveRecords, 1)?;
        let mut found = None;
        let mut conflict = false;
        let mut ids = vec![];
        let mut refs = vec![];
        for r in &self.records {
            b.charge_query(K::OperationSteps, 5)?;
            b.charge_query(K::CandidateChecks, 1)?;
            b.set_active_in(S::Query, K::ActiveRecords, 2)?;
            if r.scope == q.scope && r.action == q.action && r.input == q.input {
                b.charge_query(K::RetrievedCandidates, 1)?;
                if let Some(previous) = found {
                    b.charge_query(K::OperationSteps, 3)?;
                    conflict |= previous != r.output;
                }
                found = Some(r.output);
                ids.push(ProcedureId(r.record_id.0));
                refs.push(EvidenceRef::record(r.record_id));
            }
        }
        Ok(if conflict {
            Response::abstention(AbstentionReason::Conflict, ids, refs)
        } else {
            match found {
                Some(output) => Response::prediction_many(output, ids, refs)
                    .expect("a matching record provides a procedure ID"),
                None => Response::abstention(AbstentionReason::NoApplicableProcedure, ids, refs),
            }
        })
    }
}
impl Predictor for ExactRetrievalBaseline {
    fn predict(&self, q: &Query) -> QueryExecution {
        let mut budget = BudgetState::new(BudgetConfig::v1());
        execution(self.evaluate(q, &mut budget), &budget)
    }
}

pub struct G0OnlyAblation {
    candidates: Vec<Candidate>,
}
impl G0OnlyAblation {
    pub fn new(records: &[Transition]) -> Result<Self, GenerationError> {
        Ok(Self {
            candidates: enumerate_g0(
                records,
                &mut BudgetState::new(BudgetConfig::v1()),
                &mut Ledger::new(),
            )?,
        })
    }
    fn evaluate(&self, q: &Query, b: &mut BudgetState) -> Result<Response, BudgetError> {
        let mut output = None;
        let mut ids = vec![];
        let mut refs = vec![];
        let mut conflict = false;
        b.set_active_in(S::Query, K::ActiveRecords, 1)?;
        for c in &self.candidates {
            b.charge_query(K::CandidateChecks, 1)?;
            b.charge_query(K::RetrievedCandidates, 1)?;
            b.set_active_in(S::Query, K::ActiveRecords, 2)?;
            if let Some(value) = c.predict(q, b, S::Query)? {
                if let Some(previous) = output {
                    b.charge_query(K::OperationSteps, 3)?;
                    conflict |= previous != value;
                }
                output = Some(value);
                ids.push(c.id());
                refs.extend(c.evidence());
            }
        }
        Ok(if conflict {
            Response::abstention(AbstentionReason::Conflict, ids, refs)
        } else {
            match output {
                Some(output) => Response::prediction_many(output, ids, refs)
                    .expect("a matching candidate provides a procedure ID"),
                None => Response::abstention(AbstentionReason::NoApplicableProcedure, ids, refs),
            }
        })
    }
}
impl Predictor for G0OnlyAblation {
    fn predict(&self, q: &Query) -> QueryExecution {
        let mut budget = BudgetState::new(BudgetConfig::v1());
        execution(self.evaluate(q, &mut budget), &budget)
    }
}
fn execution(result: Result<Response, BudgetError>, budget: &BudgetState) -> QueryExecution {
    let (response, budget_error) = match result {
        Ok(r) => (r, None),
        Err(e) => (
            Response::abstention(AbstentionReason::UnsupportedByBudget, vec![], vec![]),
            Some(e),
        ),
    };
    QueryExecution {
        response,
        budget_error,
        cost: CostVector::capture(budget, S::Query),
    }
}
