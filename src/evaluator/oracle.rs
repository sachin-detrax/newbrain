use crate::{
    baselines::Predictor,
    engine::QueryExecution,
    ledger::CostVector,
    types::{AbstentionReason, ProcedureId, Query, Response, Sequence},
};

pub(super) struct OracleReference;
impl Predictor for OracleReference {
    fn predict(&self, q: &Query) -> QueryExecution {
        let [x, m, y] = q.input.positions();
        let next = match (q.scope, q.action, m) {
            (0, 0, 8) => Some(9),
            (1, 1, 10) => Some(11),
            _ => None,
        };
        let response = match next {
            Some(n) if x == y && x < 8 => {
                Response::prediction(Sequence::new([x, n, y]), ProcedureId(0), vec![])
            }
            Some(_) | None => {
                Response::abstention(AbstentionReason::NoApplicableProcedure, vec![], vec![])
            }
        };
        QueryExecution {
            response,
            cost: CostVector::default(),
            budget_error: None,
        }
    }
}
