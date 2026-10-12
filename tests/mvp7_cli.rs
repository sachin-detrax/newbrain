use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_PATH: AtomicU64 = AtomicU64::new(0);

fn temp_root(label: &str) -> PathBuf {
    let serial = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "newbrain-mvp7-{label}-{}-{serial}",
        std::process::id()
    ))
}

fn run_cli(
    environment: &str,
    store: &std::path::Path,
    report: &std::path::Path,
    extra: &[&str],
) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_run_mvp"))
        .args([
            "--environment",
            environment,
            "--store",
            store.to_str().expect("temporary store is UTF-8"),
            "--report",
            report.to_str().expect("temporary report is UTF-8"),
        ])
        .args(extra)
        .output()
        .expect("run_mvp should start")
}

fn read_store(store: &PathBuf) -> String {
    let mut contents = String::new();
    for entry in fs::read_dir(store).expect("store directory exists") {
        let path = entry.expect("store entry should be readable").path();
        if path.is_file() {
            contents.push_str(&fs::read_to_string(path).expect("learner artifact is UTF-8"));
        }
    }
    contents
}

fn assert_successful_report(report: &str, environment: &str) {
    for field in [
        "\"status\":\"validated-mvp-narrow-claim\"",
        "\"induction_hook\"",
        "\"actions_selected\"",
        "\"abstentions\"",
        "\"procedures\"",
        "\"restart\"",
        "\"stale_revision_status\"",
        "\"budgets\"",
        "\"failures\"",
        "\"leakage\"",
        "\"phases\"",
    ] {
        assert!(report.contains(field), "report is missing {field}");
    }
    assert!(report.contains(&format!("\"environment\":\"{environment}\"")));
    assert!(report.contains("\"reused_after_restart\":true"));
    assert!(report.contains("\"procedure_id\":\"1\""));
}

#[test]
fn argument_parsing_accepts_help_and_rejects_invalid_arguments() {
    let help = Command::new(env!("CARGO_BIN_EXE_run_mvp"))
        .arg("--help")
        .output()
        .expect("run_mvp help should start");
    assert!(help.status.success());
    let help_text = String::from_utf8(help.stdout).unwrap();
    assert!(help_text.contains("--environment structural-a|structural-b"));
    assert!(help_text.contains("--load"));

    let invalid = Command::new(env!("CARGO_BIN_EXE_run_mvp"))
        .args(["--environment", "unknown", "--store", "x", "--report", "y"])
        .output()
        .expect("run_mvp invalid invocation should start");
    assert!(!invalid.status.success());
    assert!(String::from_utf8(invalid.stderr)
        .unwrap()
        .contains("unknown environment"));
}

#[test]
fn structural_a_runs_end_to_end_and_writes_inspectable_artifacts() {
    let store = temp_root("a");
    let report_path = temp_root("a-report").with_extension("json");
    let output = run_cli("structural-a", &store, &report_path, &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report = fs::read_to_string(&report_path).unwrap();
    assert_successful_report(&report, "structural-a");
    for name in [
        "state.json",
        "final-state.json",
        "learner-trace.jsonl",
        "candidate-events.jsonl",
        "report.jsonl",
    ] {
        assert!(store.join(name).is_file(), "missing {name}");
    }
    assert_eq!(
        report,
        fs::read_to_string(store.join("report.jsonl"))
            .unwrap()
            .trim_end()
    );
}

#[test]
fn structural_b_runs_through_the_same_cli_path_and_reuses_on_explicit_load() {
    let store = temp_root("b");
    let report_path = temp_root("b-report").with_extension("json");
    let output = run_cli("structural-b", &store, &report_path, &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_successful_report(&fs::read_to_string(&report_path).unwrap(), "structural-b");

    let loaded_report_path = temp_root("b-loaded-report").with_extension("json");
    let loaded = run_cli("structural-b", &store, &loaded_report_path, &["--load"]);
    assert!(
        loaded.status.success(),
        "{}",
        String::from_utf8_lossy(&loaded.stderr)
    );
    let loaded_report = fs::read_to_string(loaded_report_path).unwrap();
    assert_successful_report(&loaded_report, "structural-b");
    assert!(loaded_report.contains("loaded-and-restarted"));
}

#[test]
fn repeated_logical_runs_are_deterministic_and_learner_artifacts_have_no_labels() {
    let first_store = temp_root("deterministic-first");
    let first_report_path = temp_root("deterministic-first-report").with_extension("json");
    let first = run_cli("structural-b", &first_store, &first_report_path, &[]);
    assert!(first.status.success());

    let second_store = temp_root("deterministic-second");
    let second_report_path = temp_root("deterministic-second-report").with_extension("json");
    let second = run_cli("structural-b", &second_store, &second_report_path, &[]);
    assert!(second.status.success());
    assert_eq!(
        fs::read(&first_report_path).unwrap(),
        fs::read(&second_report_path).unwrap(),
        "logical MVP-7 reports should be byte-identical across fresh runs"
    );

    let artifacts = read_store(&first_store);
    for evaluator_label in [
        "HiddenAtom",
        "HiddenAction",
        "North",
        "South",
        "East",
        "West",
        "Dormant",
        "Armed",
        "Lit",
        "StructuralToyEnvironment",
        "score",
        "seed",
        "schedule",
    ] {
        assert!(
            !artifacts.contains(evaluator_label),
            "learner artifacts leaked {evaluator_label}"
        );
    }
}

#[test]
fn disabled_induction_writes_a_report_but_returns_a_gate_failure() {
    let store = temp_root("gate-failure");
    let report_path = temp_root("gate-failure-report").with_extension("json");
    let output = run_cli(
        "structural-a",
        &store,
        &report_path,
        &["--disable-induction"],
    );
    assert!(!output.status.success());
    let report = fs::read_to_string(report_path).unwrap();
    assert!(report.contains("\"status\":\"mvp-gate-failed\""));
    assert!(report.contains("\"mvp-gate-failure\""));
    assert!(store.join("state.json").is_file());
    assert!(store.join("final-state.json").is_file());
}
