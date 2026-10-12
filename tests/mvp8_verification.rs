//! End-to-end verification of the bounded MVP claim.
//!
//! This test deliberately sits above the individual MVP tests.  It checks the
//! public runtime boundary with a small local environment, exercises the
//! learner-owned stores through their public APIs, and invokes the packaged
//! `run_mvp` binary so that the claim includes the files a user would inspect.
//! It does not import evaluator-only types in the contract probe and does not
//! alter any checked-in v1/v2 artifact.

use std::cell::{Cell, RefCell};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use sparse_structural_rule_learner::dataset::V1Dataset;
use sparse_structural_rule_learner::framework::{
    ActMode, ActionId, AdmissionOutcome, AtomId, AuditReferences, CandidateScope, CandidateStage,
    Decision, EvidenceId, FrameworkState, GrammarV2, HypothesisId, InteractionLedger, LearnerState,
    LearnerStores, ProcedureId, ProcedureStatus, ProcedureVersion, ScopeToken, StepInput,
    StepResult, StoredHypothesisStatus, TerminalStatus,
};
use sparse_structural_rule_learner::runtime::{
    Environment, EnvironmentActionError, EnvironmentTurn, EpisodeBudget, GenericRunController,
    Learner as RuntimeLearner, RunPhase,
};
use sparse_structural_rule_learner::{
    AbstainReason, ObservationPacket, PreviousTerminal, ProtocolError,
};

static NEXT_PATH: AtomicU64 = AtomicU64::new(0);

fn project_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn temp_root(label: &str) -> PathBuf {
    let serial = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "newbrain-mvp8-{label}-{}-{serial}",
        std::process::id()
    ))
}

fn packet(first: u64, second: u64, third: u64) -> ObservationPacket {
    ObservationPacket::new([AtomId::new(first), AtomId::new(second), AtomId::new(third)])
}

fn assert_gate(id: &str, passed: bool, detail: impl fmt::Display) {
    assert!(passed, "{id} failed: {detail}");
}

fn sha256(path: &Path) -> String {
    let output = if cfg!(windows) {
        Command::new("certutil")
            .args([
                "-hashfile",
                path.to_str().expect("artifact path is UTF-8"),
                "SHA256",
            ])
            .output()
            .expect("certutil should be available on Windows")
    } else {
        Command::new("sha256sum")
            .arg(path)
            .output()
            .expect("sha256sum should be available on Unix")
    };
    assert!(
        output.status.success(),
        "SHA-256 command failed for {}: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let hash = stdout
        .split_whitespace()
        .find(|token| token.len() == 64 && token.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .expect("SHA-256 command should print a 64-character digest");
    hash.to_ascii_lowercase()
}

fn assert_json_object(text: &str, label: &str) {
    let bytes = text.trim().as_bytes();
    assert!(!bytes.is_empty(), "{label} is empty");
    assert_eq!(bytes[0], b'{', "{label} must start with a JSON object");
    assert_eq!(
        bytes[bytes.len() - 1],
        b'}',
        "{label} must end with a JSON object"
    );

    let mut depth = 0_i32;
    let mut in_string = false;
    let mut escaped = false;
    for byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match *byte {
            b'"' => in_string = true,
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth -= 1,
            _ => {}
        }
        assert!(depth >= 0, "{label} closes JSON containers out of order");
    }
    assert!(
        !in_string && !escaped,
        "{label} contains an unterminated string"
    );
    assert_eq!(depth, 0, "{label} has unbalanced JSON containers");
}

fn quoted_field<'a>(json: &'a str, field: &str) -> &'a str {
    let marker = format!("\"{field}\":\"");
    let start = json
        .find(&marker)
        .unwrap_or_else(|| panic!("report is missing string field {field}"))
        + marker.len();
    let rest = &json[start..];
    let end = rest
        .find('"')
        .unwrap_or_else(|| panic!("report field {field} is not a closed string"));
    &rest[..end]
}

fn numeric_field(json: &str, field: &str) -> u64 {
    let marker = format!("\"{field}\":");
    let start = json
        .find(&marker)
        .unwrap_or_else(|| panic!("report is missing numeric field {field}"))
        + marker.len();
    let digits = json[start..]
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    digits
        .parse()
        .unwrap_or_else(|_| panic!("report field {field} is not a number"))
}

fn run_cli(environment: &str, store: &Path, report: &Path, extra: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_run_mvp"))
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
        .expect("run_mvp should start");
    assert!(
        output.status.success(),
        "run_mvp {environment} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report_text = fs::read_to_string(report).expect("run_mvp should write its report");
    assert_json_object(&report_text, "run_mvp report");
    report_text
}

fn assert_report_and_store(report: &str, report_path: &Path, store: &Path, environment: &str) {
    for field in [
        "\"artifact_paths\"",
        "\"actions_selected\"",
        "\"abstentions\"",
        "\"procedures\"",
        "\"restart\"",
        "\"stale_revision_status\"",
        "\"budgets\"",
        "\"failures\"",
        "\"leakage\"",
        "\"phases\"",
        "\"final_evaluation\"",
    ] {
        assert!(report.contains(field), "report is missing {field}");
    }
    assert!(report.contains("\"status\":\"validated-mvp-narrow-claim\""));
    assert!(report.contains(&format!("\"environment\":\"{environment}\"")));
    assert!(report.contains("\"reused_after_restart\":true"));
    assert!(report.contains("\"status\":\"within-limits\""));
    assert!(report.contains("\"status\":\"clean\""));
    assert!(report.contains("\"mode\":\"procedure\""));
    assert!(numeric_field(report, "procedure_backed_actions") > 0);
    assert!(numeric_field(report, "successes") > 0);

    for field in ["checkpoint_fingerprint", "final_fingerprint"] {
        let fingerprint = quoted_field(report, field);
        assert_eq!(
            fingerprint.len(),
            16,
            "{field} should be an FNV-1a-64 digest"
        );
        assert!(
            fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "{field} should contain only hexadecimal digits"
        );
    }

    for name in [
        "state.json",
        "final-state.json",
        "learner-trace.jsonl",
        "candidate-events.jsonl",
        "report.jsonl",
    ] {
        assert!(
            store.join(name).is_file(),
            "missing packaged artifact {name}"
        );
    }

    let checkpoint = FrameworkState::load(store.join("state.json"))
        .expect("mutable checkpoint should be a valid FrameworkState");
    assert!(
        !checkpoint.is_frozen(),
        "the resume checkpoint should remain mutable"
    );
    let mut final_state = FrameworkState::load(store.join("final-state.json"))
        .expect("final checkpoint should be a valid FrameworkState");
    assert!(
        final_state.is_frozen(),
        "the final checkpoint should be frozen"
    );
    assert!(
        final_state.stores_mut().is_err(),
        "frozen stores became mutable"
    );
    assert!(
        final_state.ledger_mut().is_err(),
        "frozen ledger became mutable"
    );
    assert_eq!(
        quoted_field(report, "checkpoint_fingerprint"),
        checkpoint.fingerprint().unwrap()
    );
    assert_eq!(
        quoted_field(report, "final_fingerprint"),
        final_state.fingerprint().unwrap()
    );

    let report_copy = fs::read_to_string(store.join("report.jsonl")).unwrap();
    assert_eq!(report, report_copy.trim_end(), "report JSONL copy drifted");
    for name in [
        "learner-trace.jsonl",
        "candidate-events.jsonl",
        "report.jsonl",
    ] {
        for (line_number, line) in fs::read_to_string(store.join(name))
            .unwrap()
            .lines()
            .enumerate()
        {
            assert_json_object(line, &format!("{name} line {}", line_number + 1));
        }
    }

    let all_store_text = [
        "state.json",
        "final-state.json",
        "learner-trace.jsonl",
        "candidate-events.jsonl",
        "report.jsonl",
    ]
    .into_iter()
    .map(|name| fs::read_to_string(store.join(name)).unwrap())
    .collect::<Vec<_>>()
    .join("\n");
    for evaluator_only in [
        "HiddenAtom",
        "HiddenAction",
        "StructuralToyEnvironment",
        "North",
        "South",
        "East",
        "West",
        "Dormant",
        "Armed",
        "Lit",
        "score",
        "seed",
        "schedule",
    ] {
        assert!(
            !all_store_text.contains(evaluator_only),
            "learner artifact leaked evaluator-only value {evaluator_only}"
        );
    }

    assert_eq!(
        fs::read_to_string(report_path).unwrap(),
        report,
        "requested report and in-memory report differ"
    );
}

#[derive(Debug)]
enum ProbeError {
    Protocol(ProtocolError),
}

impl fmt::Display for ProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ProbeError {}

#[derive(Clone, Debug)]
struct ProbeEnvironment {
    events: Rc<RefCell<Vec<&'static str>>>,
    resolved_actions: Rc<Cell<u32>>,
}

impl ProbeEnvironment {
    fn new(events: Rc<RefCell<Vec<&'static str>>>) -> Self {
        Self {
            events,
            resolved_actions: Rc::new(Cell::new(0)),
        }
    }
}

impl Environment for ProbeEnvironment {
    type Error = ProbeError;

    fn reset(&mut self, ledger: &InteractionLedger) -> Result<StepInput, Self::Error> {
        StepInput::new(
            ledger.run_scope(),
            ScopeToken::new(808),
            0,
            packet(7, 8, 7),
            [ActionId::new(0), ActionId::new(1)],
            2,
            PreviousTerminal::None,
        )
        .map_err(ProbeError::Protocol)
    }

    fn apply(
        &mut self,
        turn: &mut EnvironmentTurn<'_>,
    ) -> Result<(), EnvironmentActionError<Self::Error>> {
        self.events.borrow_mut().push("environment-apply");
        let events = Rc::clone(&self.events);
        let resolved_actions = Rc::clone(&self.resolved_actions);
        turn.resolve(|_| {
            events.borrow_mut().push("environment-result");
            resolved_actions.set(resolved_actions.get() + 1);
            StepResult::new(packet(7, 9, 7), TerminalStatus::Success).map_err(ProbeError::Protocol)
        })?;
        Ok(())
    }
}

#[derive(Debug)]
struct ProbeLearner {
    events: Rc<RefCell<Vec<&'static str>>>,
    abstain: Option<AbstainReason>,
    observations: Rc<Cell<u32>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FrozenProbeLearner {
    abstain: Option<AbstainReason>,
    observations: u32,
}

impl RuntimeLearner for ProbeLearner {
    type Frozen = FrozenProbeLearner;

    fn step(&mut self, input: StepInput) -> Result<Decision, ProtocolError> {
        self.events.borrow_mut().push("learner-decision");
        match self.abstain {
            Some(reason) => Ok(Decision::abstain(reason, AuditReferences::empty())),
            None => Decision::act(
                input.available_actions()[0],
                None,
                ActMode::Fallback,
                AuditReferences::empty(),
            ),
        }
    }

    fn observe(&mut self, _result: StepResult) -> Result<(), ProtocolError> {
        self.events.borrow_mut().push("learner-observe");
        self.observations.set(self.observations.get() + 1);
        Ok(())
    }

    fn freeze(self) -> Self::Frozen {
        FrozenProbeLearner {
            abstain: self.abstain,
            observations: self.observations.get(),
        }
    }
}

impl sparse_structural_rule_learner::runtime::FrozenLearner for FrozenProbeLearner {
    fn step(&self, input: StepInput) -> Result<Decision, ProtocolError> {
        match self.abstain {
            Some(reason) => Ok(Decision::abstain(reason, AuditReferences::empty())),
            None => Decision::act(
                input.available_actions()[0],
                None,
                ActMode::Fallback,
                AuditReferences::empty(),
            ),
        }
    }
}

fn probe_controller(abstain: Option<AbstainReason>) -> ProbeRun {
    let events = Rc::new(RefCell::new(Vec::new()));
    let environment = ProbeEnvironment::new(Rc::clone(&events));
    let resolved_actions = Rc::clone(&environment.resolved_actions);
    let learner = ProbeLearner {
        events: Rc::clone(&events),
        abstain,
        observations: Rc::new(Cell::new(0)),
    };
    let controller = GenericRunController::with_budget(
        learner,
        environment,
        ScopeToken::new(807),
        EpisodeBudget::new(1, 0, 0),
    )
    .unwrap();
    (controller, events, resolved_actions)
}

type ProbeRun = (
    GenericRunController<ProbeLearner, ProbeEnvironment>,
    Rc<RefCell<Vec<&'static str>>>,
    Rc<Cell<u32>>,
);

#[allow(clippy::too_many_arguments)]
fn add_observation(
    ledger: &mut InteractionLedger,
    episode: u64,
    before: ObservationPacket,
    action: u64,
    prediction: Option<ObservationPacket>,
    after: ObservationPacket,
    audit: AuditReferences,
    mode: ActMode,
) {
    let input = StepInput::new(
        ledger.run_scope(),
        ScopeToken::new(episode),
        0,
        before,
        [ActionId::new(7), ActionId::new(9)],
        2,
        PreviousTerminal::None,
    )
    .unwrap();
    let decision = Decision::act(ActionId::new(action), prediction, mode, audit).unwrap();
    let receipt = ledger.record_decision(input, decision).unwrap();
    ledger
        .record_observed_result(
            &receipt,
            StepResult::new(after, TerminalStatus::Success).unwrap(),
        )
        .unwrap();
}

struct VersionedFixture {
    state: FrameworkState,
    procedure_id: ProcedureId,
    hypothesis_id: HypothesisId,
}

fn versioned_fixture() -> VersionedFixture {
    let run_scope = ScopeToken::new(900);
    let mut ledger = InteractionLedger::new(run_scope).unwrap();
    add_observation(
        &mut ledger,
        901,
        packet(10, 20, 10),
        7,
        None,
        packet(10, 21, 10),
        AuditReferences::empty(),
        ActMode::Exploration,
    );
    add_observation(
        &mut ledger,
        902,
        packet(11, 20, 11),
        7,
        None,
        packet(11, 21, 11),
        AuditReferences::empty(),
        ActMode::Exploration,
    );
    add_observation(
        &mut ledger,
        903,
        packet(10, 21, 10),
        7,
        None,
        packet(10, 21, 10),
        AuditReferences::empty(),
        ActMode::Exploration,
    );

    let mut grammar = GrammarV2::new(run_scope).unwrap();
    let candidate = grammar
        .generate(&ledger)
        .candidates_at(CandidateStage::E3)
        .into_iter()
        .next()
        .expect("the public grammar should produce an E3 candidate")
        .clone();
    let hypothesis_id = candidate.hypothesis_id();
    ledger
        .register_hypothesis(
            hypothesis_id,
            sparse_structural_rule_learner::framework::AuditScope::Run,
        )
        .unwrap();
    add_observation(
        &mut ledger,
        904,
        packet(99, 20, 99),
        7,
        None,
        packet(99, 21, 99),
        AuditReferences::new(Vec::new(), Vec::new(), vec![hypothesis_id]).unwrap(),
        ActMode::Exploration,
    );
    let admitted = match grammar.admit(candidate.clone(), &ledger) {
        AdmissionOutcome::Admitted(value) => value,
        other => panic!("public grammar did not admit the fixture: {other:?}"),
    };
    let procedure = grammar.promote(&mut ledger, admitted).unwrap();
    let procedure_id = procedure.procedure_id();
    add_observation(
        &mut ledger,
        905,
        packet(100, 20, 100),
        7,
        Some(packet(100, 21, 100)),
        packet(100, 22, 100),
        AuditReferences::new(vec![procedure_id], Vec::new(), Vec::new()).unwrap(),
        ActMode::Procedure,
    );
    let revision = match grammar.revalidate(&procedure, &ledger) {
        sparse_structural_rule_learner::framework::RevisionOutcome::Suspended(value) => *value,
        other => panic!("public grammar did not preserve a contradiction revision: {other:?}"),
    };

    let mut stores = LearnerStores::new(run_scope).unwrap();
    stores.register_ledger(&ledger).unwrap();
    stores
        .register_hypothesis(candidate, StoredHypothesisStatus::Active)
        .unwrap();
    stores.register_procedure(procedure).unwrap();
    stores
        .transition_hypothesis(
            hypothesis_id,
            StoredHypothesisStatus::Contradicted(EvidenceId::new(5)),
        )
        .unwrap();
    stores
        .append_procedure_version(ProcedureVersion::from(revision))
        .unwrap();
    stores.link_procedure_to_derived(procedure_id, 700).unwrap();
    let learner = LearnerState::new(run_scope).unwrap();
    let state = FrameworkState::from_parts(learner, ledger, stores).unwrap();

    VersionedFixture {
        state,
        procedure_id,
        hypothesis_id,
    }
}

#[test]
fn mvp8_verifies_the_bounded_mvp_completion_claim() {
    // M1: the fixed v1 fixture and recorded historical artifacts are stable.
    assert_gate(
        "M1",
        V1Dataset::fixed().fingerprint() == 13_935_465_607_801_348_367,
        "the v1 dataset fingerprint changed",
    );
    let v1_report_paths = [
        project_path("artifacts/v1/run-primary-1/report.txt"),
        project_path("artifacts/v1/run-primary-2/report.txt"),
        project_path("artifacts/v1/run-primary-3/report.txt"),
    ];
    let v1_ledger_paths = [
        project_path("artifacts/v1/run-primary-1/ledger.txt"),
        project_path("artifacts/v1/run-primary-2/ledger.txt"),
        project_path("artifacts/v1/run-primary-3/ledger.txt"),
    ];
    for paths in [&v1_report_paths[..], &v1_ledger_paths[..]] {
        let first = fs::read(&paths[0]).unwrap();
        assert!(paths.iter().all(|path| fs::read(path).unwrap() == first));
    }
    assert_gate(
        "M1",
        sha256(&v1_report_paths[0])
            == "b2bbb0b0120dc9a0727758896572f07cad2c80ad4892ddd26c642dc875c66c74"
            && sha256(&v1_ledger_paths[0])
                == "3db255b266d57487470259f5d04dfb266a54b4eb8b11a77071f28950b64dbf06"
            && sha256(&project_path("artifacts/v1/run-extensions/adversarial.txt"))
                == "d37a9d4b385fab68de6dfddad5612232bf3d7c586fdce69ca2355e638543d1a7"
            && sha256(&project_path(
                "artifacts/v1/run-extensions-corrected-1/adversarial.txt",
            )) == "79fbd5a31fc601f857bb6ad6041376fa8fefa54b696c4ba21d301ac09a99b899",
        "recorded v1 SHA-256 fingerprints changed",
    );

    // M2 and M3: a caller-owned environment/learner pair uses only the public
    // runtime boundary, and the external result follows the recorded decision.
    let (mut contract_controller, events, resolved_actions) = probe_controller(None);
    contract_controller.discovery().unwrap();
    assert_gate(
        "M2",
        contract_controller.summary().action_count() == 1
            && contract_controller.phase() == RunPhase::Discovery,
        "the public runtime contract did not execute a custom environment",
    );
    assert_gate(
        "M3",
        *events.borrow()
            == vec![
                "learner-decision",
                "environment-apply",
                "environment-result",
                "learner-observe",
            ]
            && matches!(
                contract_controller.ledger().events()[0],
                sparse_structural_rule_learner::framework::LedgerEvent::Decision(_)
            )
            && matches!(
                contract_controller.ledger().events()[1],
                sparse_structural_rule_learner::framework::LedgerEvent::Interaction(_)
            )
            && resolved_actions.get() == 1,
        "the environment result was visible before the decision was recorded",
    );

    // M4 and M8: immutable evidence and explicit procedure versions survive a
    // contradiction without replacing the historical version.
    let fixture = versioned_fixture();
    let procedure_versions = fixture
        .state
        .stores()
        .procedures()
        .versions(
            fixture.procedure_id,
            CandidateScope::Run(fixture.state.run_scope()),
        )
        .unwrap();
    let original_evidence = procedure_versions[0].evidence_refs().to_vec();
    let foreign_scope_rejected = fixture
        .state
        .stores()
        .procedures()
        .get(
            fixture.procedure_id,
            CandidateScope::Run(ScopeToken::new(9010)),
        )
        .is_err();
    assert_gate(
        "M4",
        procedure_versions.len() == 2
            && procedure_versions[0].version() == 1
            && procedure_versions[0].status() == ProcedureStatus::Active
            && procedure_versions[1].version() == 2
            && procedure_versions[1].status() == ProcedureStatus::Suspended
            && procedure_versions[0].evidence_refs() == original_evidence.as_slice()
            && procedure_versions[1]
                .evidence_refs()
                .contains(&EvidenceId::new(5))
            && foreign_scope_rejected
            && fixture
                .state
                .stores()
                .hypotheses()
                .get(fixture.hypothesis_id)
                .unwrap()
                .status()
                == StoredHypothesisStatus::Contradicted(EvidenceId::new(5)),
        "evidence, hypotheses, or procedure revisions were not preserved",
    );
    assert_gate(
        "M8",
        fixture.state.stores().evidence().len() == 5
            && fixture.state.stores().dependencies().affected_derived_ids(
                sparse_structural_rule_learner::framework::DependencySource::Procedure(
                    fixture.procedure_id,
                ),
            ) == vec![700],
        "contradiction history is not retained as an explicit versioned record",
    );

    // M4/M5 persistence round-trip: the public state file is canonical and
    // reloadable, including both procedure versions and contradiction status.
    let persistence_path = temp_root("framework-state").with_extension("json");
    let expected_fingerprint = fixture.state.fingerprint().unwrap();
    assert_eq!(
        fixture.state.save(&persistence_path).unwrap(),
        expected_fingerprint
    );
    let restored = FrameworkState::load(&persistence_path).unwrap();
    assert_gate(
        "M5",
        restored.fingerprint().unwrap() == expected_fingerprint
            && restored.stores().procedures().len() == 2
            && restored
                .stores()
                .hypotheses()
                .get(fixture.hypothesis_id)
                .unwrap()
                .status()
                == StoredHypothesisStatus::Contradicted(EvidenceId::new(5)),
        "FrameworkState did not restore the versioned learner-owned records",
    );

    // M7: an explicit unsupported/insufficient-evidence decision is closed as
    // incomplete and never fabricates an observed packet.
    let (mut abstaining_controller, _abstention_events, abstention_resolutions) =
        probe_controller(Some(AbstainReason::InsufficientEvidence));
    abstaining_controller.discovery().unwrap();
    let abstention_episode = &abstaining_controller.summary().episodes[0];
    let abstention_evidence = abstaining_controller.ledger().interactions()[0].evidence();
    assert_gate(
        "M7",
        abstention_episode.abstention_count == 1
            && abstention_episode.terminal == TerminalStatus::Incomplete
            && abstention_episode.failure_classifications
                == vec![sparse_structural_rule_learner::framework::FailureClassification::InsufficientEvidence]
            && abstention_evidence.after().is_none()
            && abstention_evidence.terminal() == TerminalStatus::Incomplete
            && abstention_resolutions.get() == 0,
        "abstention was not justified and closed without an observation",
    );

    // M9/M10: the packaged CLI uses one controller path for both opaque
    // environments and emits inspectable, budgeted, fingerprinted reports.
    let root = temp_root("cli");
    let store_a = root.join("structural-a");
    let report_a_path = root.join("structural-a.json");
    let report_a = run_cli("structural-a", &store_a, &report_a_path, &[]);
    let store_b = root.join("structural-b");
    let report_b_path = root.join("structural-b.json");
    let report_b = run_cli("structural-b", &store_b, &report_b_path, &[]);
    assert_report_and_store(&report_a, &report_a_path, &store_a, "structural-a");
    assert_report_and_store(&report_b, &report_b_path, &store_b, "structural-b");
    assert_gate(
        "M6",
        report_a.contains("\"phase\":\"final_evaluation\"")
            && report_b.contains("\"phase\":\"final_evaluation\"")
            && report_a.contains("\"procedure_backed_actions\":1")
            && report_b.contains("\"procedure_backed_actions\":2"),
        "restored procedures did not transfer to held-out final inputs",
    );
    assert_gate(
        "M9",
        report_a.contains("GenericRunController<GroundingLearner, MVP-6 adapter>")
            && report_b.contains("GenericRunController<GroundingLearner, MVP-6 adapter>")
            && report_a.contains("\"environment\":\"structural-a\"")
            && report_b.contains("\"environment\":\"structural-b\""),
        "the two environments did not use the same controller path",
    );
    assert_gate(
        "M10",
        report_a.contains("\"procedure_evidence_refs\"")
            && report_b.contains("\"procedure_evidence_refs\"")
            && report_a.contains("\"logical_counters\"")
            && report_b.contains("\"logical_counters\"")
            && report_a.contains("\"checkpoint_fingerprint\"")
            && report_b.contains("\"final_fingerprint\"")
            && report_a.contains("\"hidden_evaluator_labels_in_learner_artifacts\":false")
            && report_b.contains("\"hidden_evaluator_labels_in_learner_artifacts\":false"),
        "CLI reports omitted evidence, resource, fingerprint, or leakage fields",
    );

    // M5 fresh-process restore: the second invocation loads the saved
    // checkpoint, rebuilds the policy, and reuses it in final evaluation.
    let loaded_report_path = root.join("structural-b-loaded.json");
    let loaded_report = run_cli("structural-b", &store_b, &loaded_report_path, &["--load"]);
    assert_gate(
        "M5",
        loaded_report.contains("\"mode\":\"loaded-and-restarted\"")
            && loaded_report.contains("\"reused_after_restart\":true")
            && loaded_report.contains("\"replayed_training_episodes\":5"),
        "a fresh run_mvp process did not restore and reuse the saved procedure",
    );

    // M11: compare three independent fresh processes, including the logical
    // report and every learner-facing artifact that should be deterministic.
    let mut reruns = Vec::new();
    for index in 0..3 {
        let store = root.join(format!("rerun-{index}"));
        let report_path = root.join(format!("rerun-{index}.json"));
        let report = run_cli("structural-b", &store, &report_path, &[]);
        reruns.push((report, store));
    }
    for (report, store) in &reruns {
        assert_eq!(report, &reruns[0].0, "fresh reports differ on rerun");
        for name in [
            "state.json",
            "final-state.json",
            "learner-trace.jsonl",
            "candidate-events.jsonl",
        ] {
            assert_eq!(
                fs::read(store.join(name)).unwrap(),
                fs::read(reruns[0].1.join(name)).unwrap(),
                "{name} differs across deterministic reruns"
            );
        }
    }
    assert_gate(
        "M11",
        reruns
            .iter()
            .all(|(report, _)| report.contains("\"status\":\"validated-mvp-narrow-claim\"")),
        "three fresh runs did not produce equivalent logical claims",
    );

    // M12: the package records the exact non-experimental verification suite;
    // the commands themselves are run by the repository verification step.
    let readme = fs::read_to_string(project_path("artifacts/mvp/README.md")).unwrap();
    assert_gate(
        "M12",
        [
            "cargo.exe fmt --all -- --check",
            "cargo.exe check --all-targets",
            "cargo.exe test --all-targets",
            "cargo.exe clippy --all-targets -- -D warnings",
            "cargo.exe test --test mvp8_verification",
        ]
        .into_iter()
        .all(|command| readme.contains(command))
            && (1..=12).all(|gate| readme.contains(&format!("M{gate}")))
            && readme.contains("Architecture experiments A-C remain out of scope"),
        "the MVP-8 package does not record all bounded verification commands and gates",
    );
}
