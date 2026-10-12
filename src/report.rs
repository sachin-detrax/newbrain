use crate::adversarial::ExtensionResult;
use crate::evaluator::{response_name, EvaluationReport, RunReport};
use crate::types::Response;
use std::fs;
use std::io;
use std::path::Path;

pub fn write_report(directory: &Path, report: &RunReport) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    fs::write(directory.join("report.txt"), stable_report(report))?;
    fs::write(directory.join("ledger.txt"), report.ledger.stable_text())
}

pub fn write_extensions(directory: &Path, results: &[ExtensionResult]) -> io::Result<()> {
    let mut text = String::from("name|property|passed|detail\n");
    for result in results {
        text.push_str(&format!(
            "{}|{}|{}|{}\n",
            result.name,
            result.property,
            result.passed,
            escape(&result.detail)
        ));
    }
    fs::write(directory.join("adversarial.txt"), text)
}

pub fn stable_report(report: &RunReport) -> String {
    let mut text = String::new();
    text.push_str(&format!(
        "status={}\n",
        if report.passed() { "passed" } else { "failed" }
    ));
    text.push_str(&format!(
        "dataset_fingerprint={}\n",
        report.dataset_fingerprint
    ));
    text.push_str(&format!("grammar_version={}\n", report.grammar_version));
    text.push_str(&format!("target_os={}\n", std::env::consts::OS));
    text.push_str(&format!("target_arch={}\n", std::env::consts::ARCH));
    for (index, training) in report.training.iter().enumerate() {
        text.push_str(&format!(
            "training_{}=admitted:{}|rejected:{}|cost:{:?}\n",
            index + 1,
            training.admitted,
            training.rejected,
            training.cost
        ));
    }
    write_evaluation(&mut text, "main_before", &report.main_before);
    write_evaluation(&mut text, "main_after", &report.main_after);
    write_evaluation(&mut text, "main_r2", &report.main_r2);
    write_evaluation(&mut text, "exact", &report.exact);
    write_evaluation(&mut text, "g0", &report.g0);
    write_evaluation(&mut text, "no_development", &report.no_development);
    write_evaluation(&mut text, "oracle", &report.oracle);
    text.push_str("procedures\n");
    for procedure in &report.procedures {
        text.push_str(procedure);
        text.push('\n');
    }
    text.push_str("gates\n");
    for gate in &report.gates {
        text.push_str(&format!("{}={}\n", gate.name, gate.passed));
    }
    text
}

fn write_evaluation(text: &mut String, name: &str, report: &EvaluationReport) {
    text.push_str(&format!("{}_metrics={:?}\n", name, report.metrics));
    for case in &report.cases {
        let response = response_name(&case.execution.response);
        let detail = match &case.execution.response {
            Response::Prediction(_) => format!(
                "output={:?}|procedures={:?}|evidence={:?}",
                case.execution
                    .response
                    .output()
                    .expect("prediction has output"),
                case.execution.response.procedure_ids(),
                case.execution.response.evidence_refs()
            ),
            Response::Abstention(_) => format!(
                "reason={:?}|procedures={:?}|evidence={:?}",
                case.execution
                    .response
                    .reason()
                    .expect("abstention has a reason"),
                case.execution.response.procedure_ids(),
                case.execution.response.evidence_refs()
            ),
        };
        text.push_str(&format!(
            "{}_case_{}=applicable:{}|correct:{}|status:{}|{}|cost:{:?}\n",
            name,
            case.query.query_id.0,
            case.applicable,
            case.correct,
            response,
            detail,
            case.execution.cost
        ));
    }
}

fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
