use crate::{
    baselines::Predictor,
    dataset::EvaluatorCase,
    engine::QueryExecution,
    types::{Query, Response},
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Metrics {
    pub applicable: usize,
    pub predictions: usize,
    pub correct: usize,
    pub non_applicable: usize,
    pub false_predictions: usize,
    pub budget_failures: usize,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseResult {
    pub query: Query,
    pub execution: QueryExecution,
    pub applicable: bool,
    pub correct: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluationReport {
    pub cases: Vec<CaseResult>,
    pub metrics: Metrics,
}

pub(crate) fn evaluate(predictor: &impl Predictor, cases: &[EvaluatorCase]) -> EvaluationReport {
    let mut report = EvaluationReport {
        cases: vec![],
        metrics: Metrics::default(),
    };
    for case in cases {
        let execution = predictor.predict(&case.query);
        // Scoring reads evaluator labels only after the response exists.
        let correct = match &execution.response {
            Response::Prediction(_) => execution.response.output() == Some(case.expected),
            Response::Abstention(_) => false,
        };
        let predicted = matches!(&execution.response, Response::Prediction(_));
        if case.applicable {
            report.metrics.applicable += 1;
            report.metrics.predictions += usize::from(predicted);
            report.metrics.correct += usize::from(correct);
        } else {
            report.metrics.non_applicable += 1;
            report.metrics.false_predictions += usize::from(predicted);
        }
        report.metrics.budget_failures += usize::from(execution.budget_error.is_some());
        report.cases.push(CaseResult {
            query: case.query,
            execution,
            applicable: case.applicable,
            correct,
        });
    }
    report
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GateResult {
    pub name: &'static str,
    pub passed: bool,
}

pub fn performance_gates(before: &Metrics, after: &Metrics, r2: &Metrics) -> Vec<GateResult> {
    vec![
        GateResult {
            name: "R1 coverage 5/5",
            passed: before.applicable == 5 && before.predictions == 5,
        },
        GateResult {
            name: "R1 exact correctness 5/5",
            passed: before.correct == 5,
        },
        GateResult {
            name: "R1 non-applicability",
            passed: before.non_applicable == 3
                && before.false_predictions == 0
                && after.false_predictions == 0,
        },
        GateResult {
            name: "R1 retention",
            passed: after.applicable == 5 && after.predictions == 5 && after.correct == 5,
        },
        GateResult {
            name: "R2 coverage and accuracy 2/2",
            passed: r2.applicable == 2 && r2.predictions == 2 && r2.correct == 2,
        },
        GateResult {
            name: "R2 non-applicability",
            passed: r2.non_applicable == 2 && r2.false_predictions == 0,
        },
        GateResult {
            name: "Query budgets",
            passed: before.budget_failures + after.budget_failures + r2.budget_failures == 0,
        },
    ]
}
