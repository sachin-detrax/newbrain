use sparse_structural_rule_learner::framework::grammar_admission::ProcedureRevision;
use sparse_structural_rule_learner::framework::interaction_ledger::AuditScope;
use sparse_structural_rule_learner::framework::{
    ActMode, ActionId, AdmissionOutcome, AtomId, AuditReferences, Candidate, CandidateScope,
    CandidateStage, Decision, DependencyIndex, DependencySource, DependentRecord, EvidenceId,
    EvidenceStore, GrammarV2, HypothesisStore, InteractionLedger, LearnerStores, ObservationPacket,
    PreviousTerminal, ProcedureId, ProcedureStore, ProcedureVersion, ScopeToken, StepInput,
    StepResult, StoreEntity, StoreError, StoredHypothesisStatus, TerminalStatus,
};

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
    ledger: InteractionLedger,
    candidate: Candidate,
    procedure: sparse_structural_rule_learner::framework::ProcedureRecord,
    revision: ProcedureRevision,
}

fn fixture() -> Fixture {
    let mut ledger = InteractionLedger::new(ScopeToken::new(900)).unwrap();
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

    let mut grammar = GrammarV2::new(ledger.run_scope()).unwrap();
    let candidate = grammar
        .generate(&ledger)
        .candidates()
        .iter()
        .find(|candidate| candidate.stage() == CandidateStage::E3)
        .expect("fixture must produce an admitted E3 candidate")
        .clone();
    ledger
        .register_hypothesis(candidate.hypothesis_id(), AuditScope::Run)
        .unwrap();
    add_observation(
        &mut ledger,
        904,
        packet(99, 20, 99),
        7,
        None,
        packet(99, 21, 99),
        AuditReferences::new(Vec::new(), Vec::new(), vec![candidate.hypothesis_id()]).unwrap(),
        ActMode::Exploration,
    );
    let admitted = match grammar.admit(candidate.clone(), &ledger) {
        AdmissionOutcome::Admitted(admitted) => admitted,
        other => panic!("fixture candidate was not admitted: {other:?}"),
    };
    let procedure = grammar.promote(&mut ledger, admitted).unwrap();

    add_observation(
        &mut ledger,
        905,
        packet(100, 20, 100),
        7,
        Some(packet(100, 21, 100)),
        packet(100, 22, 100),
        AuditReferences::new(vec![procedure.procedure_id()], Vec::new(), Vec::new()).unwrap(),
        ActMode::Procedure,
    );
    let revision = match grammar.revalidate(&procedure, &ledger) {
        sparse_structural_rule_learner::framework::RevisionOutcome::Suspended(revision) => {
            *revision
        }
        other => panic!("fixture procedure was not contradicted: {other:?}"),
    };

    Fixture {
        ledger,
        candidate,
        procedure,
        revision,
    }
}

fn register_evidence_in_reverse(fixture: &Fixture) -> EvidenceStore {
    let mut store = EvidenceStore::new(fixture.ledger.run_scope()).unwrap();
    for interaction in fixture.ledger.interactions().iter().rev() {
        store.register(interaction.evidence().clone()).unwrap();
    }
    store
}

#[test]
fn evidence_lookup_and_iteration_are_exact_and_deterministic() {
    let fixture = fixture();
    let store = register_evidence_in_reverse(&fixture);
    let ids: Vec<_> = store.iter().map(|record| record.evidence_id()).collect();

    assert_eq!(ids, (1..=5).map(EvidenceId::new).collect::<Vec<_>>());
    assert_eq!(
        store.get(EvidenceId::new(2)).unwrap().after(),
        Some(packet(11, 21, 11))
    );
    assert_eq!(
        store
            .get_in_scope(EvidenceId::new(2), ScopeToken::new(902))
            .unwrap()
            .episode_scope(),
        ScopeToken::new(902)
    );
}

#[test]
fn incomplete_evidence_is_retained_without_fabricating_an_observation() {
    let run_scope = ScopeToken::new(1_100);
    let episode_scope = ScopeToken::new(1_101);
    let input = StepInput::new(
        run_scope,
        episode_scope,
        0,
        packet(1, 2, 1),
        [ActionId::new(7), ActionId::new(9)],
        2,
        PreviousTerminal::None,
    )
    .unwrap();
    let mut ledger = InteractionLedger::new(run_scope).unwrap();
    let receipt = ledger
        .record_decision(
            input,
            Decision::abstain(
                sparse_structural_rule_learner::framework::AbstainReason::InsufficientEvidence,
                AuditReferences::empty(),
            ),
        )
        .unwrap();
    let evidence = ledger.record_result(&receipt, None).unwrap();
    let mut store = EvidenceStore::new(run_scope).unwrap();
    store.register(evidence.clone()).unwrap();

    assert_eq!(evidence.after(), None);
    assert_eq!(evidence.terminal(), TerminalStatus::Incomplete);
    assert_eq!(
        evidence.provenance(),
        sparse_structural_rule_learner::framework::EvidenceProvenance::Incomplete
    );
    assert_eq!(store.get(evidence.evidence_id()).unwrap(), &evidence);
}

#[test]
fn duplicate_ids_and_scope_reuse_are_rejected_without_replacement() {
    let fixture = fixture();
    let evidence = register_evidence_in_reverse(&fixture);
    let duplicate = evidence.get(EvidenceId::new(1)).unwrap().clone();
    let mut evidence = evidence;
    assert!(matches!(
        evidence.register(duplicate),
        Err(StoreError::DuplicateId {
            entity: StoreEntity::Evidence,
            id: 1
        })
    ));

    let mut hypotheses = HypothesisStore::new(fixture.ledger.run_scope()).unwrap();
    hypotheses
        .register(fixture.candidate.clone(), &evidence)
        .unwrap();
    assert!(matches!(
        hypotheses.register(fixture.candidate.clone(), &evidence),
        Err(StoreError::DuplicateId {
            entity: StoreEntity::Hypothesis,
            ..
        })
    ));

    let mut procedures = ProcedureStore::new(fixture.ledger.run_scope()).unwrap();
    procedures
        .register(fixture.procedure.clone(), &evidence)
        .unwrap();
    assert!(matches!(
        procedures.register(fixture.procedure.clone(), &evidence),
        Err(StoreError::DuplicateId {
            entity: StoreEntity::Procedure,
            ..
        })
    ));
    assert!(matches!(
        procedures.get(
            fixture.procedure.procedure_id(),
            CandidateScope::Run(ScopeToken::new(999))
        ),
        Err(StoreError::ScopeMismatch {
            entity: StoreEntity::Procedure,
            ..
        })
    ));
}

#[test]
fn hypothesis_status_transitions_are_explicit_and_historical() {
    let fixture = fixture();
    let evidence = register_evidence_in_reverse(&fixture);
    let mut store = HypothesisStore::new(fixture.ledger.run_scope()).unwrap();
    store
        .register(fixture.candidate.clone(), &evidence)
        .unwrap();
    let id = fixture.candidate.hypothesis_id();

    store
        .transition(id, StoredHypothesisStatus::Unresolved, &evidence)
        .unwrap();
    store
        .transition(id, StoredHypothesisStatus::Active, &evidence)
        .unwrap();
    store
        .transition(
            id,
            StoredHypothesisStatus::Contradicted(EvidenceId::new(5)),
            &evidence,
        )
        .unwrap();
    let record = store.get(id).unwrap();
    assert_eq!(
        record.status(),
        StoredHypothesisStatus::Contradicted(EvidenceId::new(5))
    );
    assert_eq!(
        record
            .transitions()
            .iter()
            .map(|transition| (transition.from(), transition.to()))
            .collect::<Vec<_>>(),
        vec![
            (
                StoredHypothesisStatus::Active,
                StoredHypothesisStatus::Unresolved
            ),
            (
                StoredHypothesisStatus::Unresolved,
                StoredHypothesisStatus::Active
            ),
            (
                StoredHypothesisStatus::Active,
                StoredHypothesisStatus::Contradicted(EvidenceId::new(5)),
            ),
        ]
    );
    assert!(matches!(
        store.transition(id, StoredHypothesisStatus::Active, &evidence),
        Err(StoreError::InvalidTransition { .. })
    ));

    let mut rejected = HypothesisStore::new(fixture.ledger.run_scope()).unwrap();
    rejected
        .register_with_status(
            fixture.candidate.clone(),
            StoredHypothesisStatus::Rejected,
            &evidence,
        )
        .unwrap();
    assert_eq!(
        rejected.get(id).unwrap().status(),
        StoredHypothesisStatus::Rejected
    );
}

#[test]
fn procedure_versions_keep_old_records_and_preserve_evidence_references() {
    let fixture = fixture();
    let evidence = register_evidence_in_reverse(&fixture);
    let scope = CandidateScope::Run(fixture.ledger.run_scope());
    let mut procedures = ProcedureStore::new(fixture.ledger.run_scope()).unwrap();
    procedures
        .register(fixture.procedure.clone(), &evidence)
        .unwrap();
    let original = procedures
        .get_version(fixture.procedure.procedure_id(), 1, scope)
        .unwrap();
    let original_refs = original.evidence_refs().to_vec();
    let signature = original.structural_signature().to_owned();

    procedures
        .append_version(ProcedureVersion::from(fixture.revision.clone()), &evidence)
        .unwrap();
    let versions = procedures
        .versions(fixture.procedure.procedure_id(), scope)
        .unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].version(), 1);
    assert_eq!(
        versions[0].status(),
        sparse_structural_rule_learner::framework::ProcedureStatus::Active
    );
    assert_eq!(versions[1].version(), 2);
    assert_eq!(
        versions[1].status(),
        sparse_structural_rule_learner::framework::ProcedureStatus::Suspended
    );
    assert_eq!(versions[0].evidence_refs(), original_refs.as_slice());
    assert!(versions[1].evidence_refs().contains(&EvidenceId::new(5)));

    let by_signature = procedures.lookup_by_signature(&signature, scope).unwrap();
    assert_eq!(
        by_signature
            .iter()
            .map(|record| (record.procedure_id(), record.version()))
            .collect::<Vec<_>>(),
        vec![
            (fixture.procedure.procedure_id(), 1),
            (fixture.procedure.procedure_id(), 2)
        ]
    );
    assert!(matches!(
        procedures.append_version(ProcedureVersion::from(fixture.revision), &evidence),
        Err(StoreError::DuplicateVersion { version: 2, .. })
    ));
}

#[test]
fn dependency_lookup_remains_after_contradiction_and_version_changes() {
    let fixture = fixture();
    let mut stores = LearnerStores::new(fixture.ledger.run_scope()).unwrap();
    stores.register_ledger(&fixture.ledger).unwrap();
    stores
        .register_hypothesis(fixture.candidate.clone(), StoredHypothesisStatus::Active)
        .unwrap();
    stores
        .register_procedure(fixture.procedure.clone())
        .unwrap();
    stores
        .link_evidence_to_derived(EvidenceId::new(1), 700)
        .unwrap();
    stores
        .link_hypothesis_to_derived(fixture.candidate.hypothesis_id(), 700)
        .unwrap();
    stores
        .link_procedure_to_derived(fixture.procedure.procedure_id(), 700)
        .unwrap();
    stores
        .transition_hypothesis(
            fixture.candidate.hypothesis_id(),
            StoredHypothesisStatus::Stale,
        )
        .unwrap();
    stores
        .append_procedure_version(ProcedureVersion::from(fixture.revision))
        .unwrap();

    assert_eq!(
        stores
            .dependencies()
            .affected_derived_ids(DependencySource::Evidence(EvidenceId::new(1))),
        vec![700]
    );
    assert_eq!(
        stores
            .dependencies()
            .affected_records(DependencySource::Procedure(
                fixture.procedure.procedure_id(),
            )),
        vec![DependentRecord::Derived(700)]
    );
    assert_eq!(stores.procedures().len(), 2);
}

#[test]
fn end_to_end_opaque_store_registration_uses_framework_records() {
    let fixture = fixture();
    let mut stores = LearnerStores::new(fixture.ledger.run_scope()).unwrap();
    stores.register_ledger(&fixture.ledger).unwrap();
    stores
        .register_hypothesis(fixture.candidate.clone(), StoredHypothesisStatus::Active)
        .unwrap();
    stores
        .register_procedure(fixture.procedure.clone())
        .unwrap();

    let procedure = stores
        .procedures()
        .get(
            fixture.procedure.procedure_id(),
            CandidateScope::Run(fixture.ledger.run_scope()),
        )
        .unwrap();
    assert_eq!(procedure.procedure_id(), fixture.procedure.procedure_id());
    assert_eq!(procedure.hypothesis_id(), fixture.candidate.hypothesis_id());
    assert!(procedure.evidence_refs().contains(&EvidenceId::new(1)));
    assert_eq!(
        stores
            .hypotheses()
            .get(fixture.candidate.hypothesis_id())
            .unwrap()
            .status(),
        StoredHypothesisStatus::Active
    );
    assert_eq!(stores.evidence().len(), fixture.ledger.interaction_count());
    assert!(!stores.dependencies().is_empty());
}

#[test]
fn dependency_index_orders_links_and_keeps_distinct_opaque_kinds() {
    let mut index = DependencyIndex::new();
    index.link_evidence_to_derived(EvidenceId::new(9), 3);
    index.link_evidence_to_derived(EvidenceId::new(9), 1);
    index.link_evidence_to_procedure(EvidenceId::new(9), ProcedureId::new(4));
    index.link_hypothesis_to_derived(
        sparse_structural_rule_learner::framework::HypothesisId::new(2),
        2,
    );

    assert_eq!(
        index.affected_derived_ids(DependencySource::Evidence(EvidenceId::new(9))),
        vec![1, 3]
    );
    assert_eq!(
        index.affected_records(DependencySource::Evidence(EvidenceId::new(9))),
        vec![
            DependentRecord::Procedure(ProcedureId::new(4)),
            DependentRecord::Derived(1),
            DependentRecord::Derived(3),
        ]
    );
}
