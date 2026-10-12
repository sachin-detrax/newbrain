use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_PATH: AtomicU64 = AtomicU64::new(0);

fn temp_root(label: &str) -> PathBuf {
    let serial = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "newbrain-expa-{label}-{}-{serial}",
        std::process::id()
    ))
}

fn run_cli(variant: &str, mode: &str, out: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_run_a"))
        .args([
            "--variant",
            variant,
            "--mode",
            mode,
            "--reruns",
            "1",
            "--out",
            out.to_str().expect("temporary out is UTF-8"),
        ])
        .output()
        .expect("run_a should start")
}

#[test]
fn cli_rejects_unknown_variant() {
    let out = temp_root("bad-variant");
    let output = Command::new(env!("CARGO_BIN_EXE_run_a"))
        .args([
            "--variant",
            "a9",
            "--mode",
            "primary",
            "--reruns",
            "1",
            "--out",
            out.to_str().unwrap(),
        ])
        .output()
        .expect("run_a should start");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn cli_validates_a1_primary() {
    let out = temp_root("a1-primary");
    let output = run_cli("a1", "primary", &out);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert!(stdout.contains("\"state\":\"validated-a1-narrow-claim\""));
    let run_dir = out.join("A1-primary-rerun1");
    for relative in [
        "protocol.md",
        "specification.md",
        "run-manifest.json",
        "learner/stream-events.jsonl",
        "learner/analysis-trace.jsonl",
        "learner/candidate-events.jsonl",
        "learner/relation-records.jsonl",
        "learner/library-records.jsonl",
        "learner/evidence-records.jsonl",
        "metrics.json",
        "acceptance.json",
        "failures.jsonl",
        "logical-resource-report.json",
        "physical-resource-report.json",
        "evaluator-only/generator-manifest.json",
        "evaluator-only/score-input.json",
        "sha256sums.txt",
    ] {
        assert!(run_dir.join(relative).is_file(), "missing {relative}");
    }
}

#[test]
fn cli_reports_a0_baseline_without_claim() {
    let out = temp_root("a0-primary");
    let output = run_cli("a0", "primary", &out);
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert!(stdout.contains("\"state\":\"no-result\""));
}
