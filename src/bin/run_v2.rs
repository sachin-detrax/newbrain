use sparse_structural_rule_learner::v2::evaluator_world::RunVariant;
use sparse_structural_rule_learner::v2::experiment::{
    write_experiment_artifacts, ExperimentRunner,
};
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(report) => {
            println!("status={}", report.validation_status());
            if report.primary_passed() && report.secondary_passed() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("run_v2 failed: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<sparse_structural_rule_learner::v2::ExperimentReport, Box<dyn std::error::Error>>
{
    let (output, variant) = parse_args(env::args().skip(1))?;
    let report = match variant {
        VariantChoice::Primary => ExperimentRunner::run_variant(RunVariant::Primary)?,
        VariantChoice::Permutation => {
            ExperimentRunner::run_variant(RunVariant::SymbolActionPermutation)?
        }
        VariantChoice::All => ExperimentRunner::run_all()?,
    };
    write_experiment_artifacts(&report, &output)?;
    println!("report={}", output.join("experiment-report.json").display());
    Ok(report)
}

#[derive(Clone, Copy)]
enum VariantChoice {
    Primary,
    Permutation,
    All,
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<(PathBuf, VariantChoice), String> {
    let mut output = None;
    let mut variant = VariantChoice::All;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--out" => {
                output = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "missing output directory".to_owned())?,
                ));
            }
            "--variant" => {
                variant = match args
                    .next()
                    .ok_or_else(|| "missing variant".to_owned())?
                    .as_str()
                {
                    "primary" => VariantChoice::Primary,
                    "permutation" | "symbol-action-permutation" => VariantChoice::Permutation,
                    "all" => VariantChoice::All,
                    value => return Err(format!("unknown variant {value}")),
                };
            }
            value => return Err(format!("unexpected argument {value}")),
        }
    }
    Ok((
        output.ok_or_else(|| "missing --out <dir>".to_owned())?,
        variant,
    ))
}
