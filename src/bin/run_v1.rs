use sparse_structural_rule_learner::{
    adversarial::run_extensions,
    evaluator::run_primary,
    report::{write_extensions, write_report},
};
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(passed) => {
            if passed {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("run_v1 failed: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let (output, extensions_requested) = output_directory(env::args().skip(1))?;
    let report = run_primary()?;
    write_report(&output, &report)?;
    if extensions_requested {
        write_extensions(&output, &run_extensions()?)?;
    }
    println!("report={}", output.join("report.txt").display());
    println!(
        "status={}",
        if report.passed() { "passed" } else { "failed" }
    );
    Ok(report.passed())
}

fn output_directory(mut args: impl Iterator<Item = String>) -> Result<(PathBuf, bool), String> {
    let flag = args.next().ok_or_else(|| "missing --out".to_string())?;
    if flag != "--out" {
        return Err(format!("expected --out, got {flag}"));
    }
    let path = args
        .next()
        .ok_or_else(|| "missing output directory".to_string())?;
    let extensions = match args.next().as_deref() {
        None => false,
        Some("--extensions") => true,
        Some(argument) => return Err(format!("unexpected argument {argument}")),
    };
    if args.next().is_some() {
        return Err("unexpected argument".to_string());
    }
    Ok((PathBuf::from(path), extensions))
}
