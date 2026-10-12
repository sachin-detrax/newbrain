use std::collections::HashSet;

use sparse_structural_rule_learner::v2::evaluator_world::{
    fixed_schedule, new_permutation_run, EpisodeRole, EvaluatorWorld, HiddenAction, HiddenAtom,
    IdentifierMappings, Partition,
};
use sparse_structural_rule_learner::v2::interaction_ledger::{
    AuditScope, EvidenceProvenance, InteractionLedger, LedgerError, LedgerEvent, RunPhase,
};
use sparse_structural_rule_learner::v2::partial_observation::{PartialAtom, PartialObservation};
use sparse_structural_rule_learner::v2::{
    AbstainReason, ActMode, ActionId, AtomId, AuditReferences, Decision, LearnerState,
    ObservationPacket, PreviousTerminal, ProcedureId, ProtocolError, ScopeToken, StepInput,
    StepResult, TerminalStatus,
};

fn packet(first: u64, second: u64, third: u64) -> ObservationPacket {
    ObservationPacket::new([AtomId::new(first), AtomId::new(second), AtomId::new(third)])
}

fn input(
    run_scope: u64,
    episode_scope: u64,
    step_index: u8,
    previous_terminal: PreviousTerminal,
    remaining_budget: u8,
) -> StepInput {
    StepInput::new(
        ScopeToken::new(run_scope),
        ScopeToken::new(episode_scope),
        step_index,
        packet(4, 9, 4),
        [ActionId::new(0), ActionId::new(1)],
        remaining_budget,
        previous_terminal,
    )
    .unwrap()
}

#[test]
fn protocol_validation_keeps_the_wire_contract_exact() {
    assert!(matches!(
        ObservationPacket::try_from_slice(&[AtomId::new(1), AtomId::new(2)]),
        Err(ProtocolError::InvalidPacketLength {
            expected: 3,
            actual: 2
        })
    ));
    assert!(matches!(
        StepInput::new(
            ScopeToken::new(1),
            ScopeToken::new(2),
            1,
            packet(1, 2, 1),
            [ActionId::new(0), ActionId::new(1)],
            2,
            PreviousTerminal::Continue,
        ),
        Err(ProtocolError::InvalidStepMetadata { .. })
    ));
    assert!(matches!(
        StepResult::new(packet(1, 2, 1), TerminalStatus::Incomplete),
        Err(ProtocolError::InvalidStepResultTerminal(
            TerminalStatus::Incomplete
        ))
    ));

    let decision = Decision::act(
        ActionId::new(9),
        None,
        ActMode::Fallback,
        AuditReferences::empty(),
    )
    .unwrap();
    assert!(matches!(
        decision.validate_for(&input(1, 2, 0, PreviousTerminal::None, 2)),
        Err(ProtocolError::ActionUnavailable(action)) if action == ActionId::new(9)
    ));
    assert!(Decision::act(
        ActionId::new(0),
        None,
        ActMode::Procedure,
        AuditReferences::empty(),
    )
    .is_err());

    let mut ledger = InteractionLedger::new(ScopeToken::new(11)).unwrap();
    ledger
        .register_procedure(ProcedureId::new(7), AuditScope::Run, true)
        .unwrap();
    let procedure = Decision::act(
        ActionId::new(0),
        Some(packet(4, 9, 4)),
        ActMode::Procedure,
        AuditReferences::new(vec![ProcedureId::new(7)], Vec::new(), Vec::new()).unwrap(),
    )
    .unwrap();
    let receipt = ledger
        .record_decision(input(11, 12, 0, PreviousTerminal::None, 2), procedure)
        .unwrap();
    assert_eq!(receipt.decision().prediction(), Some(packet(4, 9, 4)));
}

#[test]
fn decision_is_immutable_before_result_and_abstention_has_no_packet() {
    let mut world = EvaluatorWorld::primary();
    let first_input = world.reset().unwrap();
    let mut ledger = InteractionLedger::new(first_input.run_scope()).unwrap();
    let decision = Decision::act(
        ActionId::new(0),
        None,
        ActMode::Fallback,
        AuditReferences::empty(),
    )
    .unwrap();
    let receipt = ledger.record_decision(first_input, decision).unwrap();

    assert_eq!(ledger.decision_count(), 1);
    assert_eq!(ledger.interaction_count(), 0);
    assert!(ledger.pending_decision().is_some());
    assert!(matches!(ledger.events(), [LedgerEvent::Decision(_)]));
    let duplicate = Decision::act(
        ActionId::new(0),
        None,
        ActMode::Fallback,
        AuditReferences::empty(),
    )
    .unwrap();
    assert!(matches!(
        ledger.record_decision(first_input, duplicate),
        Err(LedgerError::DecisionAlreadyPending(1))
    ));

    let result = world
        .apply_recorded(&mut ledger, &receipt)
        .unwrap()
        .unwrap();
    assert_eq!(result.terminal(), TerminalStatus::Continue);
    let evidence = ledger.interactions().last().unwrap().evidence().clone();
    assert_eq!(ledger.interaction_count(), 1);
    assert!(matches!(
        ledger.events(),
        [LedgerEvent::Decision(_), LedgerEvent::Interaction(_)]
    ));
    assert_eq!(evidence.provenance(), EvidenceProvenance::Observed);
    assert!(evidence.after().is_some());

    let second_input = world.next_input().unwrap();
    let abstain = Decision::abstain(
        AbstainReason::InsufficientEvidence,
        AuditReferences::empty(),
    );
    let abstain_receipt = ledger.record_decision(second_input, abstain).unwrap();
    assert!(!abstain_receipt.expects_result());
    assert!(matches!(
        ledger.events(),
        [
            LedgerEvent::Decision(_),
            LedgerEvent::Interaction(_),
            LedgerEvent::Decision(_)
        ]
    ));
    assert_eq!(
        world.apply_recorded(&mut ledger, &abstain_receipt).unwrap(),
        None
    );
    let abstention = ledger.interactions().last().unwrap().evidence();
    assert_eq!(abstention.terminal(), TerminalStatus::Incomplete);
    assert_eq!(abstention.action(), None);
    assert_eq!(abstention.after(), None);
    assert_eq!(abstention.provenance(), EvidenceProvenance::Incomplete);
    let next_episode = world.reset().unwrap();
    let next_abstention = Decision::abstain(
        AbstainReason::NoApplicableProcedure,
        AuditReferences::empty(),
    );
    let next_receipt = ledger
        .record_decision(next_episode, next_abstention)
        .unwrap();
    assert_eq!(
        ledger.decisions().last().unwrap().phase(),
        RunPhase::Mutable
    );
    assert_eq!(
        world.apply_recorded(&mut ledger, &next_receipt).unwrap(),
        None
    );
}

#[test]
fn mappings_match_the_protocol_splitmix_and_fisher_yates_vectors() {
    let primary = IdentifierMappings::from_world_seed(2_026_101_101);
    assert_eq!(
        primary.atom_wire_order(),
        [
            HiddenAtom::C1,
            HiddenAtom::C3,
            HiddenAtom::C2,
            HiddenAtom::M1,
            HiddenAtom::C0,
            HiddenAtom::M0,
        ]
    );
    assert_eq!(
        primary.action_wire_order(),
        [HiddenAction::A1, HiddenAction::A0]
    );

    let permutation = IdentifierMappings::from_world_seed(2_026_101_102);
    assert_eq!(
        permutation.atom_wire_order(),
        [
            HiddenAtom::C2,
            HiddenAtom::C3,
            HiddenAtom::C0,
            HiddenAtom::C1,
            HiddenAtom::M1,
            HiddenAtom::M0,
        ]
    );
    assert_eq!(
        permutation.action_wire_order(),
        [HiddenAction::A0, HiddenAction::A1]
    );
    assert_ne!(primary, permutation);
}

#[test]
fn scope_tokens_are_nonzero_unique_and_permutation_runs_are_fresh() {
    let primary = EvaluatorWorld::primary();
    let permutation = EvaluatorWorld::permutation();
    let mut primary_tokens = HashSet::new();
    primary_tokens.insert(primary.run_scope());
    primary_tokens.extend(primary.episode_scopes().iter().copied());
    assert_eq!(primary_tokens.len(), 37);
    assert!(primary_tokens.iter().all(|token| !token.is_zero()));

    let permutation_tokens: HashSet<_> = std::iter::once(permutation.run_scope())
        .chain(permutation.episode_scopes().iter().copied())
        .collect();
    assert_eq!(permutation_tokens.len(), 37);
    assert!(primary_tokens.is_disjoint(&permutation_tokens));

    let primary_state = LearnerState::new(primary.run_scope()).unwrap();
    let permutation_run = new_permutation_run();
    assert_eq!(permutation_run.learner_state().decision_count(), 0);
    assert_ne!(
        primary_state.run_scope(),
        permutation_run.learner_state().run_scope()
    );
}

#[test]
fn hidden_world_transition_and_terminal_timing_are_exact() {
    let mut world = EvaluatorWorld::primary();
    let mappings = world.mappings();
    let unchanged_action = mappings.action_id(HiddenAction::A1);
    let changing_action = mappings.action_id(HiddenAction::A0);

    let first_input = world.reset().unwrap();
    let mut ledger = InteractionLedger::new(first_input.run_scope()).unwrap();
    let first_decision = Decision::act(
        unchanged_action,
        None,
        ActMode::Fallback,
        AuditReferences::empty(),
    )
    .unwrap();
    let first_receipt = ledger.record_decision(first_input, first_decision).unwrap();
    let first = world
        .apply_recorded(&mut ledger, &first_receipt)
        .unwrap()
        .unwrap();
    assert_eq!(first.terminal(), TerminalStatus::Continue);
    assert_eq!(
        first.observation().atom(1),
        first_input.observation().atom(1)
    );
    let next = world.next_input().unwrap();
    assert_eq!(next.step_index(), 1);
    assert_eq!(next.remaining_action_budget(), 1);
    assert_eq!(next.previous_terminal(), PreviousTerminal::Continue);

    let next_decision = Decision::act(
        changing_action,
        None,
        ActMode::Fallback,
        AuditReferences::empty(),
    )
    .unwrap();
    let next_receipt = ledger.record_decision(next, next_decision).unwrap();
    let success = world
        .apply_recorded(&mut ledger, &next_receipt)
        .unwrap()
        .unwrap();
    assert_eq!(success.terminal(), TerminalStatus::Success);
    assert_eq!(
        success.observation().atom(1),
        Some(mappings.atom_id(HiddenAtom::M1))
    );

    let mut failure_world = EvaluatorWorld::primary();
    let failure_input = failure_world.reset().unwrap();
    let mut failure_ledger = InteractionLedger::new(failure_input.run_scope()).unwrap();
    let failure_first = Decision::act(
        unchanged_action,
        None,
        ActMode::Fallback,
        AuditReferences::empty(),
    )
    .unwrap();
    let failure_receipt = failure_ledger
        .record_decision(failure_input, failure_first)
        .unwrap();
    assert_eq!(
        failure_world
            .apply_recorded(&mut failure_ledger, &failure_receipt)
            .unwrap()
            .unwrap()
            .terminal(),
        TerminalStatus::Continue
    );
    let failure_next_input = failure_world.next_input().unwrap();
    let failure_second = Decision::act(
        unchanged_action,
        None,
        ActMode::Fallback,
        AuditReferences::empty(),
    )
    .unwrap();
    let failure_second_receipt = failure_ledger
        .record_decision(failure_next_input, failure_second)
        .unwrap();
    assert_eq!(
        failure_world
            .apply_recorded(&mut failure_ledger, &failure_second_receipt)
            .unwrap()
            .unwrap()
            .terminal(),
        TerminalStatus::Failure
    );
}

#[test]
fn held_out_schedule_is_separate_from_mutable_support() {
    let schedule = fixed_schedule();
    assert_eq!(schedule.len(), 36);
    assert_eq!(
        schedule[..12]
            .iter()
            .filter(|row| row.partition() == Partition::Training)
            .count(),
        12
    );
    assert_eq!(
        schedule[12..20]
            .iter()
            .filter(|row| row.partition() == Partition::Development)
            .count(),
        8
    );
    assert!(schedule[..20]
        .iter()
        .all(|row| { matches!(row.initial_context(), HiddenAtom::C0 | HiddenAtom::C1) }));
    assert!(schedule[20..]
        .iter()
        .all(|row| row.partition() == Partition::Final));
    assert!(schedule[20..32]
        .iter()
        .all(|row| { matches!(row.initial_context(), HiddenAtom::C2 | HiddenAtom::C3) }));
    assert!(schedule[20..32].iter().all(|row| {
        matches!(
            row.role(),
            EpisodeRole::HeldOutApplicable | EpisodeRole::HeldOutNonApplicable
        )
    }));
    assert!(schedule[32..]
        .iter()
        .all(|row| { matches!(row.initial_context(), HiddenAtom::C0 | HiddenAtom::C1) }));
    assert_eq!(schedule[0].label(), "T01");
    assert_eq!(schedule[35].label(), "F16");
}

#[test]
fn partial_evidence_remains_unknown_instead_of_becoming_negative() {
    let partial = PartialObservation::new([
        PartialAtom::Available(AtomId::new(7)),
        PartialAtom::Unavailable,
        PartialAtom::Available(AtomId::new(7)),
    ]);
    assert_eq!(partial.known_equal(0, 2), Some(true));
    assert_eq!(partial.known_equal(0, 1), None);
    assert_eq!(partial.known_equal(1, 1), None);
    assert_eq!(
        partial.try_complete(),
        Err(ProtocolError::UnavailableEvidence)
    );

    let complete = PartialObservation::from_packet(packet(7, 8, 7));
    assert_eq!(complete.known_equal(0, 2), Some(true));
    assert_eq!(complete.known_equal(0, 1), Some(false));
}

#[test]
fn scope_and_step_validation_rejects_cross_episode_reuse() {
    let first = input(10, 20, 0, PreviousTerminal::None, 2);
    let mut ledger = InteractionLedger::new(first.run_scope()).unwrap();
    let receipt = ledger
        .record_decision(
            first,
            Decision::abstain(
                AbstainReason::NoApplicableProcedure,
                AuditReferences::empty(),
            ),
        )
        .unwrap();
    assert!(!receipt.expects_result());
    ledger.record_result(&receipt, None).unwrap();

    let reused = input(10, 20, 0, PreviousTerminal::None, 2);
    assert!(matches!(
        ledger.record_decision(
            reused,
            Decision::abstain(AbstainReason::NoApplicableProcedure, AuditReferences::empty()),
        ),
        Err(LedgerError::EpisodeScopeReused(scope)) if scope == ScopeToken::new(20)
    ));
}
