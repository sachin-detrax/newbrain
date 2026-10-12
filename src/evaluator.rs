mod oracle;
mod scoring;
use crate::{
    baselines::{ExactRetrievalBaseline, G0OnlyAblation},
    budget::BudgetError,
    dataset::V1Dataset,
    engine::{Learner, LearnerConfig, TrainingError, TrainingReport},
    grammar::GRAMMAR_VERSION,
    ledger::Ledger,
    types::Response,
};
use scoring::evaluate;
pub use scoring::{performance_gates, CaseResult, EvaluationReport, GateResult, Metrics};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunReport {
    pub dataset_fingerprint: u64,
    pub grammar_version: &'static str,
    pub training: Vec<TrainingReport>,
    pub main_before: EvaluationReport,
    pub main_after: EvaluationReport,
    pub main_r2: EvaluationReport,
    pub exact: EvaluationReport,
    pub g0: EvaluationReport,
    pub no_development: EvaluationReport,
    pub oracle: EvaluationReport,
    pub gates: Vec<GateResult>,
    pub procedures: Vec<String>,
    pub ledger: Ledger,
}
impl RunReport {
    pub fn passed(&self) -> bool {
        self.gates.iter().all(|g| g.passed)
    }
}

pub fn run_primary() -> Result<RunReport, TrainingError> {
    let d = V1Dataset::fixed();
    let mut learner = Learner::new(LearnerConfig::v1());
    let a = learner.train_phase_a(d.r1_training().as_slice(), d.r1_development().as_slice())?;
    let before = learner.freeze();
    let main_before = evaluate(&before, d.r1_final_cases().as_slice());
    let b = learner.train_phase_b(d.r2_training().as_slice(), d.r2_development().as_slice())?;
    let after = learner.freeze();
    let main_after = evaluate(&after, d.r1_final_cases().as_slice());
    let main_r2 = evaluate(&after, d.r2_final_cases().as_slice());
    let allowed = d.all_training_and_development_ids();
    let actual = learner.retained_record_ids().clone();
    let evidence_valid = after.procedures().iter().all(|p| {
        p.support_refs().len() >= 2
            && p.challenge_refs().len() == 4
            && p.support_refs()
                .iter()
                .chain(p.challenge_refs())
                .all(|r| allowed.contains(&r.record_id()))
    });
    let procedures = after
        .procedures()
        .iter()
        .map(|p| {
            format!(
                "{:?}|{:?}|support={:?}|challenges={:?}",
                p.id(),
                p.candidate().signature(),
                p.support_refs(),
                p.challenge_refs()
            )
        })
        .collect();
    let mut ledger = learner.into_ledger();
    for result in [&main_before, &main_after, &main_r2] {
        record_responses(&mut ledger, result)?;
    }
    let exact = evaluate(
        &ExactRetrievalBaseline::new(d.r1_training().as_slice()),
        d.r1_final_cases().as_slice(),
    );
    let g0 = evaluate(
        &G0OnlyAblation::new(d.r1_training().as_slice())?,
        d.r1_final_cases().as_slice(),
    );
    let mut ablated = Learner::new(LearnerConfig::without_development_gate());
    ablated.train_phase_a(d.r1_training().as_slice(), d.r1_development().as_slice())?;
    let no_development = evaluate(&ablated.freeze(), d.r1_final_cases().as_slice());
    let oracle = evaluate(&oracle::OracleReference, d.r1_final_cases().as_slice());
    let mut gates = performance_gates(&main_before.metrics, &main_after.metrics, &main_r2.metrics);
    gates.push(GateResult {
        name: "Learner retained only permitted record IDs",
        passed: actual == allowed,
    });
    gates.push(GateResult {
        name: "Procedure evidence traceability",
        passed: evidence_valid,
    });
    let ids: BTreeSet<_> = after.procedures().iter().map(|p| p.id()).collect();
    gates.push(GateResult {
        name: "Unique admitted procedure IDs",
        passed: ids.len() == after.procedures().len(),
    });
    Ok(RunReport {
        dataset_fingerprint: d.fingerprint(),
        grammar_version: GRAMMAR_VERSION,
        training: vec![a, b],
        main_before,
        main_after,
        main_r2,
        exact,
        g0,
        no_development,
        oracle,
        gates,
        procedures,
        ledger,
    })
}

fn record_responses(ledger: &mut Ledger, report: &EvaluationReport) -> Result<(), BudgetError> {
    for case in &report.cases {
        ledger.record_response(&case.execution.response, case.execution.cost.clone())?;
    }
    Ok(())
}

pub fn response_name(response: &Response) -> &'static str {
    match response {
        Response::Prediction(_) => "prediction",
        Response::Abstention(_) => "abstention",
    }
}
