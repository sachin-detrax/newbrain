use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use sparse_structural_rule_learner::framework::interaction_ledger::AuditScope;
use sparse_structural_rule_learner::framework::{
    ActMode, ActionId, AdmissionOutcome, AtomId, AuditReferences, CandidateScope, CandidateStage,
    Decision, DependencySource, EvidenceId, FrameworkState, GrammarV2, GroundingLearner,
    HypothesisId, InteractionLedger, LearnerState, LearnerStores, ObservationPacket,
    PreviousTerminal, ProcedureId, ProcedureVersion, ScopeToken, StepInput, StepResult,
    StoredHypothesisStatus, TerminalStatus,
};

static NEXT_PATH: AtomicU64 = AtomicU64::new(0);

fn packet(a: u64, b: u64, c: u64) -> ObservationPacket {
    ObservationPacket::new([AtomId::new(a), AtomId::new(b), AtomId::new(c)])
}

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

struct Fixture {
    state: FrameworkState,
    procedure_id: ProcedureId,
    hypothesis_id: HypothesisId,
}

fn fixture() -> Fixture {
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
        .candidates()
        .iter()
        .find(|candidate| candidate.stage() == CandidateStage::E3)
        .expect("fixture must produce an E3 candidate")
        .clone();
    let hypothesis_id = candidate.hypothesis_id();
    ledger
        .register_hypothesis(hypothesis_id, AuditScope::Run)
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
        AdmissionOutcome::Admitted(admitted) => admitted,
        other => panic!("fixture candidate was not admitted: {other:?}"),
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
        sparse_structural_rule_learner::framework::RevisionOutcome::Suspended(revision) => {
            *revision
        }
        other => panic!("fixture procedure was not contradicted: {other:?}"),
    };

    let mut stores = LearnerStores::new(run_scope).unwrap();
    stores.register_ledger(&ledger).unwrap();
    stores
        .register_hypothesis(candidate, StoredHypothesisStatus::Active)
        .unwrap();
    stores.register_procedure(procedure).unwrap();
    stores
        .transition_hypothesis(hypothesis_id, StoredHypothesisStatus::Unresolved)
        .unwrap();
    stores
        .transition_hypothesis(hypothesis_id, StoredHypothesisStatus::Active)
        .unwrap();
    stores
        .transition_hypothesis(
            hypothesis_id,
            StoredHypothesisStatus::Contradicted(EvidenceId::new(5)),
        )
        .unwrap();
    stores
        .append_procedure_version(ProcedureVersion::from(revision))
        .unwrap();
    stores
        .link_evidence_to_derived(EvidenceId::new(1), 700)
        .unwrap();
    stores
        .link_hypothesis_to_derived(hypothesis_id, 700)
        .unwrap();
    stores.link_procedure_to_derived(procedure_id, 700).unwrap();

    let learner = LearnerState::new(run_scope).unwrap();
    let state = FrameworkState::from_parts(learner, ledger, stores).unwrap();
    Fixture {
        state,
        procedure_id,
        hypothesis_id,
    }
}

fn temp_path(label: &str) -> PathBuf {
    let serial = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "newbrain-mvp4-{label}-{}-{serial}.json",
        std::process::id()
    ))
}

fn write_bytes(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
}

#[test]
fn round_trip_preserves_stores_versions_status_history_dependencies_and_fingerprint() {
    let fixture = fixture();
    let path = temp_path("round-trip");
    let expected = fixture.state.fingerprint().unwrap();
    assert_eq!(fixture.state.save(&path).unwrap(), expected);

    let restored = FrameworkState::load(&path).unwrap();
    assert_eq!(restored.fingerprint().unwrap(), expected);
    assert_eq!(restored.ledger().interaction_count(), 5);
    assert_eq!(restored.stores().evidence().len(), 5);
    assert_eq!(restored.stores().procedures().len(), 2);
    assert_eq!(
        restored
            .stores()
            .procedures()
            .versions(
                fixture.procedure_id,
                CandidateScope::Run(ScopeToken::new(900))
            )
            .unwrap()
            .iter()
            .map(|procedure| procedure.version())
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(
        restored
            .stores()
            .hypotheses()
            .get(fixture.hypothesis_id)
            .unwrap()
            .transitions()
            .len(),
        3
    );
    assert_eq!(
        restored
            .stores()
            .dependencies()
            .affected_derived_ids(DependencySource::Procedure(fixture.procedure_id)),
        vec![700]
    );

    let second_path = temp_path("round-trip-second");
    assert_eq!(restored.save(&second_path).unwrap(), expected);
    assert_eq!(fs::read(&path).unwrap(), fs::read(&second_path).unwrap());
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(second_path);
}

#[test]
fn restart_style_restore_reuses_procedure_on_held_out_opaque_record() {
    let fixture = fixture();
    let path = temp_path("restart");
    fixture.state.save(&path).unwrap();
    let restored = FrameworkState::load(&path).unwrap();
    let procedure = restored
        .stores()
        .procedures()
        .get(
            fixture.procedure_id,
            CandidateScope::Run(ScopeToken::new(900)),
        )
        .unwrap();
    assert_eq!(
        procedure
            .expansion()
            .predict(packet(500, 20, 500), ActionId::new(7)),
        Some(packet(500, 21, 500))
    );
    let _ = fs::remove_file(path);
}

#[test]
fn newly_constructed_learner_uses_restored_active_procedure_evidence() {
    let fixture = fixture();
    let path = temp_path("restart-learner");
    fixture.state.save(&path).unwrap();
    let mut restored = FrameworkState::load(&path).unwrap();
    let active_procedure = match restored
        .stores()
        .procedures()
        .get_version(
            fixture.procedure_id,
            1,
            CandidateScope::Run(ScopeToken::new(900)),
        )
        .unwrap()
        .version_record()
    {
        ProcedureVersion::Admitted(procedure) => (**procedure).clone(),
        ProcedureVersion::Revision(_) => panic!("version one must be admitted"),
    };
    let mut learner = GroundingLearner::new(ScopeToken::new(900)).unwrap();
    let ledger = restored.ledger_mut().unwrap();
    learner.install_procedure(ledger, active_procedure).unwrap();
    let input = StepInput::new(
        ScopeToken::new(900),
        ScopeToken::new(906),
        0,
        packet(777, 20, 777),
        [ActionId::new(7), ActionId::new(9)],
        2,
        PreviousTerminal::None,
    )
    .unwrap();
    let recorded = learner.step(ledger, input).unwrap();
    assert_eq!(recorded.trace.chosen_action(), Some(ActionId::new(7)));
    assert_eq!(
        recorded.trace.decision.prediction(),
        Some(packet(777, 21, 777))
    );
    assert_eq!(recorded.trace.procedure_refs(), &[fixture.procedure_id]);
    let _ = fs::remove_file(path);
}

#[test]
fn subprocess_restart_restores_state_without_a_cli() {
    if std::env::var_os("NEWBRAIN_MVP4_CHILD").is_some() {
        return;
    }
    let fixture = fixture();
    let path = temp_path("subprocess");
    fixture.state.save(&path).unwrap();
    let executable = std::env::current_exe().unwrap();
    let status = Command::new(executable)
        .args(["--exact", "subprocess_child_restores_state", "--nocapture"])
        .env("NEWBRAIN_MVP4_CHILD", "1")
        .env("NEWBRAIN_MVP4_STATE", &path)
        .status()
        .unwrap();
    assert!(status.success());
    let _ = fs::remove_file(path);
}

#[test]
fn subprocess_child_restores_state() {
    if std::env::var_os("NEWBRAIN_MVP4_CHILD").is_none() {
        return;
    }
    let path = std::env::var_os("NEWBRAIN_MVP4_STATE").unwrap();
    let restored = FrameworkState::load(path).unwrap();
    let procedure = restored
        .stores()
        .procedures()
        .get(
            ProcedureId::new(1),
            CandidateScope::Run(ScopeToken::new(900)),
        )
        .unwrap();
    assert_eq!(
        procedure
            .expansion()
            .predict(packet(888, 20, 888), ActionId::new(7)),
        Some(packet(888, 21, 888))
    );
}

#[test]
fn schema_malformed_duplicate_and_truncated_documents_are_rejected() {
    let fixture = fixture();
    let path = temp_path("reject");
    fixture.state.save(&path).unwrap();
    let valid = fs::read_to_string(&path).unwrap();

    let schema_path = temp_path("schema");
    write_bytes(
        &schema_path,
        valid
            .replace("\"schema_version\":1", "\"schema_version\":99")
            .as_bytes(),
    );
    assert!(matches!(
        FrameworkState::load(&schema_path),
        Err(sparse_structural_rule_learner::PersistenceError::UnsupportedSchemaVersion(99))
    ));

    let duplicate_path = temp_path("duplicate");
    write_bytes(
        &duplicate_path,
        valid
            .replace(
                "\"known_evidence\":[\"1\",\"2\",\"3\",\"4\",\"5\"]",
                "\"known_evidence\":[\"1\",\"2\",\"3\",\"4\",\"5\",\"5\"]",
            )
            .as_bytes(),
    );
    assert!(FrameworkState::load(&duplicate_path).is_err());

    let truncated_path = temp_path("truncated");
    write_bytes(&truncated_path, &valid.as_bytes()[..valid.len() - 1]);
    assert!(matches!(
        FrameworkState::load(&truncated_path),
        Err(sparse_structural_rule_learner::PersistenceError::IncompleteFile)
            | Err(sparse_structural_rule_learner::PersistenceError::MalformedRecord(_))
    ));

    let malformed_path = temp_path("malformed");
    write_bytes(&malformed_path, b"{\"format\":");
    assert!(FrameworkState::load(&malformed_path).is_err());
    for path in [
        path,
        schema_path,
        duplicate_path,
        truncated_path,
        malformed_path,
    ] {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn scope_and_reference_tampering_is_rejected_without_partial_state() {
    let fixture = fixture();
    let path = temp_path("scope-source");
    fixture.state.save(&path).unwrap();
    let valid = fs::read_to_string(&path).unwrap();
    let tampered = valid.replacen("\"run_scope\":\"900\"", "\"run_scope\":\"901\"", 1);
    let tampered_path = temp_path("scope-tampered");
    write_bytes(&tampered_path, tampered.as_bytes());
    assert!(matches!(
        FrameworkState::load(&tampered_path),
        Err(sparse_structural_rule_learner::PersistenceError::ScopeViolation(_))
            | Err(sparse_structural_rule_learner::PersistenceError::MismatchedReference(_))
    ));
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(tampered_path);
}

#[test]
fn frozen_state_survives_restore_and_rejects_learner_store_mutation() {
    let mut fixture = fixture();
    fixture.state.freeze().unwrap();
    let path = temp_path("frozen");
    let fingerprint = fixture.state.save(&path).unwrap();
    let mut restored = FrameworkState::load(&path).unwrap();
    assert!(restored.is_frozen());
    assert!(matches!(
        restored.stores_mut(),
        Err(sparse_structural_rule_learner::PersistenceError::FrozenStateMutation)
    ));
    assert!(matches!(
        restored.ledger_mut(),
        Err(sparse_structural_rule_learner::PersistenceError::FrozenStateMutation)
    ));
    assert_eq!(restored.fingerprint().unwrap(), fingerprint);
    let _ = fs::remove_file(path);
}

#[test]
fn failed_atomic_destination_does_not_change_existing_snapshot() {
    let fixture = fixture();
    let path = temp_path("atomic-existing");
    fixture.state.save(&path).unwrap();
    let before = fs::read(&path).unwrap();
    let missing_parent = std::env::temp_dir().join(format!(
        "newbrain-mvp4-missing-parent-{}",
        std::process::id()
    ));
    let bad_destination = missing_parent.join("state.json");
    assert!(fixture.state.save(&bad_destination).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    let _ = fs::remove_file(path);
}
