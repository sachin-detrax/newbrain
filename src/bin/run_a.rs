use sparse_structural_rule_learner::experiment_a::experiment::{
    mode_str, run_group, write_group_layout, Variant,
};
use sparse_structural_rule_learner::experiment_a::gates::RunResultState;
use sparse_structural_rule_learner::experiment_a::generator::SymbolMode;
use std::path::PathBuf;

fn usage() -> String {
    "run_a --variant a0|a1|a3 --mode primary|permutation --reruns N --out DIR".to_owned()
}

fn parse_variant(value: &str) -> Result<Variant, String> {
    match value {
        "a0" => Ok(Variant::A0),
        "a1" => Ok(Variant::A1),
        "a3" => Ok(Variant::A3),
        _ => Err(format!("unknown variant {value}; expected a0, a1, or a3")),
    }
}

fn parse_mode(value: &str) -> Result<SymbolMode, String> {
    match value {
        "primary" => Ok(SymbolMode::Primary),
        "permutation" => Ok(SymbolMode::Permutation),
        _ => Err(format!(
            "unknown mode {value}; expected primary or permutation"
        )),
    }
}

fn main() {
    let mut variant: Option<Result<Variant, String>> = None;
    let mut mode: Option<Result<SymbolMode, String>> = None;
    let mut reruns = 3u32;
    let mut out = None;
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--variant" => variant = args.next().map(|value| parse_variant(&value)),
            "--mode" => mode = args.next().map(|value| parse_mode(&value)),
            "--reruns" => {
                reruns = match args.next().map(|value| value.parse::<u32>()) {
                    Some(Ok(count)) => count,
                    _ => {
                        eprintln!("error: --reruns needs a positive integer");
                        std::process::exit(2);
                    }
                }
            }
            "--out" => out = args.next().map(PathBuf::from),
            _ => {
                eprintln!("error: unknown flag {flag}\nusage: {}", usage());
                std::process::exit(2);
            }
        }
    }
    let variant = match variant {
        Some(Ok(variant)) => variant,
        Some(Err(detail)) => {
            eprintln!("error: {detail}\nusage: {}", usage());
            std::process::exit(2);
        }
        None => {
            eprintln!("error: --variant is required\nusage: {}", usage());
            std::process::exit(2);
        }
    };
    let mode = match mode {
        Some(Ok(mode)) => mode,
        Some(Err(detail)) => {
            eprintln!("error: {detail}\nusage: {}", usage());
            std::process::exit(2);
        }
        None => {
            eprintln!("error: --mode is required\nusage: {}", usage());
            std::process::exit(2);
        }
    };
    let out = match out {
        Some(dir) => dir,
        None => {
            eprintln!("error: --out is required\nusage: {}", usage());
            std::process::exit(2);
        }
    };
    if reruns == 0 {
        eprintln!("error: --reruns needs a positive integer");
        std::process::exit(2);
    }
    let group = run_group(variant, mode, reruns);
    if let Err(error) = write_group_layout(&out, &group) {
        eprintln!("error: failed to write {}: {error}", out.display());
        std::process::exit(2);
    }
    println!(
        "{{\"variant\":\"{}\",\"mode\":\"{}\",\"reruns\":{},\"state\":\"{}\",\"p2\":{},\"classification\":{}}}",
        variant.as_str(),
        mode_str(mode),
        reruns,
        group.state.as_str(),
        group.p2_pass,
        match group.classification {
            Some(classification) => format!("\"{}\"", classification.as_str()),
            None => "null".to_owned(),
        }
    );
    let code = match group.state {
        RunResultState::ValidatedA1NarrowClaim | RunResultState::ValidatedA3Comparator => 0,
        RunResultState::NegativeA1NarrowResult | RunResultState::NegativeA3ComparatorResult => 1,
        RunResultState::NoResult | RunResultState::InvalidatedByLeakage => 2,
    };
    std::process::exit(code);
}
