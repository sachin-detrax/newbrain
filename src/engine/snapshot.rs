use crate::admission::AdmittedProcedure;
use crate::budget::{BudgetConfig, BudgetError, BudgetKind as K, BudgetScope as S, BudgetState};
use crate::ledger::CostVector;
use crate::types::{AbstentionReason, Query, Response};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LearnerSnapshot {
    procedures: Vec<AdmittedProcedure>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryExecution {
    pub response: Response,
    pub cost: CostVector,
    pub budget_error: Option<BudgetError>,
}

impl LearnerSnapshot {
    pub(super) fn extend(&mut self, procedures: Vec<AdmittedProcedure>) {
        self.procedures.extend(procedures);
        self.procedures.sort_by(|a, b| {
            a.candidate()
                .signature()
                .cmp(b.candidate().signature())
                .then(a.id().cmp(&b.id()))
        });
    }
    pub fn procedures(&self) -> &[AdmittedProcedure] {
        &self.procedures
    }
    pub fn predict(&self, query: &Query) -> QueryExecution {
        self.predict_with_budget(query, &mut BudgetState::new(BudgetConfig::v1()))
    }
    pub fn predict_with_budget(&self, query: &Query, budget: &mut BudgetState) -> QueryExecution {
        match self.evaluate(query, budget) {
            Ok(response) => QueryExecution {
                response,
                cost: CostVector::capture(budget, S::Query),
                budget_error: None,
            },
            Err(error) => QueryExecution {
                response: Response::abstention(
                    AbstentionReason::UnsupportedByBudget,
                    vec![],
                    vec![],
                ),
                cost: CostVector::capture(budget, S::Query),
                budget_error: Some(error),
            },
        }
    }
    fn evaluate(&self, query: &Query, budget: &mut BudgetState) -> Result<Response, BudgetError> {
        let mut output = None;
        let mut conflict = false;
        let mut ids = Vec::new();
        let mut refs = Vec::new();
        budget.set_active_in(S::Query, K::ActiveRecords, 1)?;
        for procedure in &self.procedures {
            budget.charge_query(K::OperationSteps, 2)?;
            let sig = procedure.candidate().signature();
            if sig.scope != query.scope || sig.action != query.action {
                continue;
            }
            budget.charge_query(K::RetrievedCandidates, 1)?;
            budget.charge_query(K::CandidateChecks, 1)?;
            budget.set_active_in(S::Query, K::ActiveRecords, 2)?;
            if let Some(value) = procedure.candidate().predict(query, budget, S::Query)? {
                if let Some(previous) = output {
                    budget.charge_query(K::OperationSteps, 3)?;
                    conflict |= previous != value;
                }
                output = Some(value);
                ids.push(procedure.id());
                refs.extend(procedure.support_refs());
                refs.extend_from_slice(procedure.challenge_refs());
            }
        }
        refs.sort();
        refs.dedup();
        if conflict {
            return Ok(Response::abstention(AbstentionReason::Conflict, ids, refs));
        }
        Ok(match output {
            Some(output) => Response::prediction_many(output, ids, refs)
                .expect("an applicable procedure provides a procedure ID"),
            None => Response::abstention(AbstentionReason::NoApplicableProcedure, ids, refs),
        })
    }
}
