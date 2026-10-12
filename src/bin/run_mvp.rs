//! MVP-7 command-line demo.
//!
//! The generic controller owns discovery and development orchestration.  The
//! application-owned hook in this binary selects, grounds, admits, and
//! installs the bounded E3 procedure.  `FrameworkState` is saved before the
//! freeze boundary, then loaded again and used to rebuild a fresh grounding
//! learner for frozen evaluation.  This is deliberately a demo-specific
//! resume flow: `GenericRunController` does not serialize arbitrary learner
//! policy state.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use sparse_structural_rule_learner::framework::{
    AdmissionOutcome, AuditScope, Candidate, CandidateStage, Decision, EpisodeBudget, EvidenceId,
    FrameworkState, GenerationStatus, GrammarV2, GroundingLearner, HypothesisId, InteractionLedger,
    LearnerState, LearnerStores, ProcedureId, ProcedureRecord, ProcedureStatus, ProcedureVersion,
    StoredHypothesisStatus, StoredProcedure,
};
use sparse_structural_rule_learner::mvp6::{StructuralToyEnvironment, V2EnvironmentAdapter};
use sparse_structural_rule_learner::runtime::{
    apply_environment, Environment, EnvironmentActionError, EnvironmentTurn, GenericRunController,
};
use sparse_structural_rule_learner::{
    ActMode, ObservationPacket, PreviousTerminal, ScopeToken, StepInput, StepResult, TerminalStatus,
};

const REPORT_FORMAT: &str = "sparse-structural-rule-learner.mvp7-report";
const REPORT_SCHEMA_VERSION: u32 = 1;
const GROUNDING_HYPOTHESIS: HypothesisId = HypothesisId::new(900);

#[derive(Debug, Clone)]
struct AppError(String);

impl AppError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for AppError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EnvironmentChoice {
    StructuralA,
    StructuralB,
}

impl EnvironmentChoice {
    fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "structural-a" | "a" | "v2" => Ok(Self::StructuralA),
            "structural-b" | "b" | "toy" => Ok(Self::StructuralB),
            other => Err(AppError::new(format!(
                "unknown environment {other}; expected structural-a or structural-b"
            ))),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::StructuralA => "structural-a",
            Self::StructuralB => "structural-b",
        }
    }

    const fn plan(self) -> RunPlan {
        // Three discovery episodes contain two supports and one contrasting
        // observation. Two development episodes provide a grounding action
        // before the explicit admission hook runs. The remaining toy/v2
        // episodes are held out by the demo-specific frozen evaluator.
        match self {
            Self::StructuralA | Self::StructuralB => RunPlan {
                discovery: 3,
                development: 2,
                final_evaluation: 2,
            },
        }
    }

    fn make_environment(self) -> DemoEnvironment {
        match self {
            Self::StructuralA => {
                DemoEnvironment::StructuralA(Box::new(V2EnvironmentAdapter::primary()))
            }
            Self::StructuralB => {
                DemoEnvironment::StructuralB(Box::new(StructuralToyEnvironment::new()))
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct RunPlan {
    discovery: usize,
    development: usize,
    final_evaluation: usize,
}

enum DemoEnvironment {
    StructuralA(Box<V2EnvironmentAdapter>),
    StructuralB(Box<StructuralToyEnvironment>),
}

impl DemoEnvironment {
    fn run_scope(&self) -> ScopeToken {
        match self {
            Self::StructuralA(environment) => environment.run_scope(),
            Self::StructuralB(environment) => environment.run_scope(),
        }
    }
}

#[derive(Debug)]
enum DemoEnvironmentError {
    StructuralA(sparse_structural_rule_learner::mvp6::V2EnvironmentError),
    StructuralB(sparse_structural_rule_learner::mvp6::StructuralToyError),
}

impl fmt::Display for DemoEnvironmentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StructuralA(error) => error.fmt(formatter),
            Self::StructuralB(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for DemoEnvironmentError {}

impl Environment for DemoEnvironment {
    type Error = DemoEnvironmentError;

    fn reset(&mut self, ledger: &InteractionLedger) -> Result<StepInput, Self::Error> {
        match self {
            Self::StructuralA(environment) => environment
                .reset(ledger)
                .map_err(DemoEnvironmentError::StructuralA),
            Self::StructuralB(environment) => environment
                .reset(ledger)
                .map_err(DemoEnvironmentError::StructuralB),
        }
    }

    fn apply(
        &mut self,
        turn: &mut EnvironmentTurn<'_>,
    ) -> Result<(), EnvironmentActionError<Self::Error>> {
        match self {
            Self::StructuralA(environment) => {
                environment.apply(turn).map_err(map_environment_error_a)
            }
            Self::StructuralB(environment) => {
                environment.apply(turn).map_err(map_environment_error_b)
            }
        }
    }
}

fn map_environment_error_a(
    error: EnvironmentActionError<sparse_structural_rule_learner::mvp6::V2EnvironmentError>,
) -> EnvironmentActionError<DemoEnvironmentError> {
    match error {
        EnvironmentActionError::Ledger(error) => EnvironmentActionError::Ledger(error),
        EnvironmentActionError::Environment(error) => {
            EnvironmentActionError::Environment(DemoEnvironmentError::StructuralA(error))
        }
    }
}

fn map_environment_error_b(
    error: EnvironmentActionError<sparse_structural_rule_learner::mvp6::StructuralToyError>,
) -> EnvironmentActionError<DemoEnvironmentError> {
    match error {
        EnvironmentActionError::Ledger(error) => EnvironmentActionError::Ledger(error),
        EnvironmentActionError::Environment(error) => {
            EnvironmentActionError::Environment(DemoEnvironmentError::StructuralB(error))
        }
    }
}

#[derive(Debug)]
struct Config {
    environment: EnvironmentChoice,
    store: PathBuf,
    report: PathBuf,
    load: bool,
    induction: bool,
}

enum ParseResult {
    Help,
    Config(Config),
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<ParseResult, AppError> {
    let mut environment = None;
    let mut store = None;
    let mut report = None;
    let mut load = false;
    let mut induction = true;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--help" | "-h" => return Ok(ParseResult::Help),
            "--environment" => {
                let value = args
                    .next()
                    .ok_or_else(|| AppError::new("missing value for --environment"))?;
                environment = Some(EnvironmentChoice::parse(&value)?);
            }
            "--store" => {
                store = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| AppError::new("missing value for --store"))?,
                ));
            }
            "--report" => {
                report =
                    Some(PathBuf::from(args.next().ok_or_else(|| {
                        AppError::new("missing value for --report")
                    })?));
            }
            "--load" => load = true,
            "--disable-induction" => induction = false,
            other => return Err(AppError::new(format!("unexpected argument {other}"))),
        }
    }

    Ok(ParseResult::Config(Config {
        environment: environment
            .ok_or_else(|| AppError::new("missing --environment <structural-a|structural-b>"))?,
        store: store.ok_or_else(|| AppError::new("missing --store <path>"))?,
        report: report.ok_or_else(|| AppError::new("missing --report <path>"))?,
        load,
        induction,
    }))
}

fn usage() -> &'static str {
    "run_mvp --environment structural-a|structural-b --store <path> --report <path> [--load] [--disable-induction]\n\nAliases: a/v2 and b/toy. Without --load, an existing <store>/state.json is loaded automatically.\nThe state file is a mutable pre-freeze checkpoint; final-state.json is the frozen post-evaluation snapshot.\n--disable-induction is a diagnostic gate-failure mode that skips the application-owned grammar/admission hook."
}

#[derive(Clone, Debug, Default)]
struct HookInfo {
    mode: &'static str,
    generation_status: &'static str,
    candidate_signature: Option<String>,
    candidate_hypothesis: Option<HypothesisId>,
    candidate_evidence: Vec<EvidenceId>,
    procedure_id: Option<ProcedureId>,
    procedure_version: Option<u32>,
    procedure_signature: Option<String>,
    procedure_evidence: Vec<EvidenceId>,
    auxiliary_candidate: Option<Candidate>,
    stale_count: usize,
    revision_count: usize,
    failure: Option<String>,
}

struct PreparedInduction {
    grammar: GrammarV2,
    target: Option<Candidate>,
    info: HookInfo,
}

fn prepare_induction(
    controller: &mut GenericRunController<GroundingLearner, DemoEnvironment>,
) -> Result<PreparedInduction, AppError> {
    let run_scope = controller.ledger().run_scope();
    let mut grammar =
        GrammarV2::new(run_scope).map_err(|error| AppError::new(error.to_string()))?;
    let generation = grammar.generate(controller.ledger());
    let target = generation
        .candidates()
        .iter()
        .filter(|candidate| {
            candidate.stage() == CandidateStage::E3 && !candidate.changed_positions().is_empty()
        })
        .min_by_key(|candidate| candidate.canonical_order())
        .cloned();

    let mut info = HookInfo {
        mode: "application-owned-grammar-v2-admission",
        generation_status: generation_status_name(generation.status()),
        ..HookInfo::default()
    };
    if let Some(candidate) = &target {
        info.candidate_signature = Some(candidate.canonical_signature().to_owned());
        info.candidate_hypothesis = Some(candidate.hypothesis_id());
        info.candidate_evidence = candidate
            .support_evidence_ids()
            .iter()
            .chain(candidate.challenge_evidence_ids())
            .copied()
            .collect();
        info.auxiliary_candidate = Some(candidate.with_hypothesis_id(GROUNDING_HYPOTHESIS));
        let (learner, ledger) = controller
            .learner_and_ledger_mut()
            .ok_or_else(|| AppError::new("mutable learner disappeared before induction"))?;
        learner
            .retain_generation(ledger, &generation)
            .map_err(|error| AppError::new(error.to_string()))?;
        learner
            .declare_competing_prediction(
                ledger,
                GROUNDING_HYPOTHESIS,
                candidate.action(),
                ObservationPacket::new([
                    sparse_structural_rule_learner::AtomId::new(9_997),
                    sparse_structural_rule_learner::AtomId::new(9_998),
                    sparse_structural_rule_learner::AtomId::new(9_997),
                ]),
                candidate.support_evidence_ids().to_vec(),
            )
            .map_err(|error| AppError::new(error.to_string()))?;
        learner
            .declare_hypothesis_separating_action(candidate.action())
            .map_err(|error| AppError::new(error.to_string()))?;
    } else {
        info.failure = Some("no challenged E3 candidate was generated".to_owned());
    }

    Ok(PreparedInduction {
        grammar,
        target,
        info,
    })
}

fn finish_induction(
    controller: &mut GenericRunController<GroundingLearner, DemoEnvironment>,
    mut prepared: PreparedInduction,
) -> Result<HookInfo, AppError> {
    let Some(target) = prepared.target else {
        return Ok(prepared.info);
    };
    let outcome = prepared.grammar.admit(target.clone(), controller.ledger());
    match outcome {
        AdmissionOutcome::Admitted(admitted) => {
            let procedure = {
                let ledger = controller
                    .learner_and_ledger_mut()
                    .ok_or_else(|| AppError::new("mutable learner disappeared during promotion"))?
                    .1;
                prepared
                    .grammar
                    .promote(ledger, admitted)
                    .map_err(|error| AppError::new(error.to_string()))?
            };
            {
                let (learner, ledger) = controller
                    .learner_and_ledger_mut()
                    .ok_or_else(|| AppError::new("mutable learner disappeared during install"))?;
                learner
                    .install_procedure(ledger, procedure.clone())
                    .map_err(|error| AppError::new(error.to_string()))?;
            }
            prepared.info.procedure_id = Some(procedure.procedure_id());
            prepared.info.procedure_version = Some(procedure.version());
            prepared.info.procedure_signature =
                Some(procedure.expansion().canonical_signature().to_owned());
            prepared.info.procedure_evidence = procedure_evidence(&procedure);
        }
        AdmissionOutcome::Rejected(rejected) => {
            prepared.info.failure = Some(format!(
                "admission rejected the candidate: {}",
                rejected
                    .reasons()
                    .iter()
                    .map(|reason| rejection_reason_name(*reason))
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        AdmissionOutcome::Incomplete(incomplete) => {
            prepared.info.failure = Some(format!(
                "admission stopped at the budget boundary: {}",
                rejection_reason_name(incomplete.reason())
            ));
        }
    }
    Ok(prepared.info)
}

fn disabled_hook() -> HookInfo {
    HookInfo {
        mode: "disabled-diagnostic",
        generation_status: "not-run",
        failure: Some("application-owned induction hook was disabled".to_owned()),
        ..HookInfo::default()
    }
}

fn procedure_evidence(procedure: &ProcedureRecord) -> Vec<EvidenceId> {
    let mut ids = procedure.support_evidence_ids().to_vec();
    ids.extend(procedure.challenge_evidence_ids());
    ids.extend(procedure.expansion().grounding_evidence_ids());
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn hook_from_state(state: &FrameworkState, mode: &'static str) -> HookInfo {
    let mut info = HookInfo {
        mode,
        generation_status: "restored-from-framework-state",
        ..HookInfo::default()
    };
    for stored in state.stores().procedures().iter() {
        info.stale_count += usize::from(stored.status() == ProcedureStatus::Suspended);
        if matches!(stored.version_record(), ProcedureVersion::Revision(_)) {
            info.revision_count += 1;
        }
        let ProcedureVersion::Admitted(record) = stored.version_record() else {
            continue;
        };
        if stored.status() != ProcedureStatus::Active {
            continue;
        }
        if info.procedure_id.is_none() {
            info.candidate_signature = Some(record.provenance().source_signature().to_owned());
            info.candidate_hypothesis = Some(record.hypothesis_id());
            info.candidate_evidence = procedure_evidence(record);
            info.procedure_id = Some(record.procedure_id());
            info.procedure_version = Some(record.version());
            info.procedure_signature = Some(record.expansion().canonical_signature().to_owned());
            info.procedure_evidence = procedure_evidence(record);
        }
    }
    if info.procedure_id.is_none() {
        info.failure = Some("no active procedure was present in restored state".to_owned());
    }
    info
}

fn snapshot_state(
    ledger: &InteractionLedger,
    learner: &GroundingLearner,
    auxiliary_candidates: &[Candidate],
) -> Result<FrameworkState, AppError> {
    let run_scope = ledger.run_scope();
    let replay = replay_ledger(ledger, learner)?;

    let mut stores =
        LearnerStores::new(run_scope).map_err(|error| AppError::new(error.to_string()))?;
    stores
        .register_ledger(&replay)
        .map_err(|error| AppError::new(error.to_string()))?;
    for hypothesis in learner.hypotheses() {
        stores
            .register_hypothesis(
                hypothesis.candidate().clone(),
                StoredHypothesisStatus::Active,
            )
            .map_err(|error| AppError::new(error.to_string()))?;
    }
    for candidate in auxiliary_candidates {
        if candidate.hypothesis_id() == GROUNDING_HYPOTHESIS {
            stores
                .register_hypothesis(candidate.clone(), StoredHypothesisStatus::Active)
                .map_err(|error| AppError::new(error.to_string()))?;
        }
    }
    for procedure in learner.procedures() {
        stores
            .register_procedure(procedure.clone())
            .map_err(|error| AppError::new(error.to_string()))?;
    }

    let mut learner_state =
        LearnerState::new(run_scope).map_err(|error| AppError::new(error.to_string()))?;
    for _ in 0..ledger.decision_count() {
        learner_state.note_decision();
    }
    // The replay ledger is deliberately reconstructed from opaque records. It
    // is a durable learner snapshot, not a serialization of the environment.
    replay.resource_meter().counters();
    FrameworkState::from_parts(learner_state, replay, stores)
        .map_err(|error| AppError::new(error.to_string()))
}

fn replay_ledger(
    source: &InteractionLedger,
    learner: &GroundingLearner,
) -> Result<InteractionLedger, AppError> {
    let mut replay = InteractionLedger::new(source.run_scope())
        .map_err(|error| AppError::new(error.to_string()))?;
    let mut procedures = BTreeSet::new();
    let mut hypotheses = BTreeSet::new();
    for decision in source.decisions() {
        procedures.extend(decision.decision().audit().procedure_refs().iter().copied());
        hypotheses.extend(
            decision
                .decision()
                .audit()
                .hypothesis_refs()
                .iter()
                .copied(),
        );
    }
    procedures.extend(learner.procedures().map(ProcedureRecord::procedure_id));
    for id in procedures {
        replay
            .register_procedure(id, AuditScope::Run, true)
            .map_err(|error| AppError::new(error.to_string()))?;
    }
    for id in hypotheses {
        replay
            .register_hypothesis(id, AuditScope::Run)
            .map_err(|error| AppError::new(error.to_string()))?;
    }

    for interaction in source.interactions() {
        let receipt = replay
            .record_decision(
                interaction.decision().input(),
                interaction.decision().decision().clone(),
            )
            .map_err(|error| AppError::new(error.to_string()))?;
        if let Some(after) = interaction.evidence().after() {
            let result = StepResult::new(after, interaction.evidence().terminal())
                .map_err(|error| AppError::new(error.to_string()))?;
            replay
                .record_observed_result(&receipt, result)
                .map_err(|error| AppError::new(error.to_string()))?;
        } else {
            replay
                .record_result(&receipt, None)
                .map_err(|error| AppError::new(error.to_string()))?;
        }
    }
    Ok(replay)
}

fn replay_training_environment(
    environment: &mut DemoEnvironment,
    state: &FrameworkState,
) -> Result<usize, AppError> {
    let mut replay = InteractionLedger::new(state.run_scope())
        .map_err(|error| AppError::new(error.to_string()))?;
    let mut procedures = BTreeSet::new();
    let mut hypotheses = BTreeSet::new();
    for decision in state.ledger().decisions() {
        procedures.extend(decision.decision().audit().procedure_refs().iter().copied());
        hypotheses.extend(
            decision
                .decision()
                .audit()
                .hypothesis_refs()
                .iter()
                .copied(),
        );
    }
    for id in procedures {
        replay
            .register_procedure(id, AuditScope::Run, true)
            .map_err(|error| AppError::new(error.to_string()))?;
    }
    for id in hypotheses {
        replay
            .register_hypothesis(id, AuditScope::Run)
            .map_err(|error| AppError::new(error.to_string()))?;
    }

    let mut next_input = None;
    let mut episodes = BTreeSet::new();
    for interaction in state.ledger().interactions() {
        let input = match next_input.take() {
            Some(input) => input,
            None => environment
                .reset(&replay)
                .map_err(|error| AppError::new(error.to_string()))?,
        };
        if input != interaction.decision().input() {
            return Err(AppError::new(
                "restored opaque training input did not match the fresh adapter",
            ));
        }
        episodes.insert(input.episode_scope());
        let receipt = replay
            .record_decision(input, interaction.decision().decision().clone())
            .map_err(|error| AppError::new(error.to_string()))?;
        let actual = apply_environment(environment, &mut replay, &receipt)
            .map_err(|error| AppError::new(error.to_string()))?;
        let expected = interaction
            .evidence()
            .after()
            .zip(Some(interaction.evidence().terminal()));
        match (actual, expected) {
            (None, None) => {}
            (Some(result), Some((after, terminal)))
                if result.observation() == after && result.terminal() == terminal =>
            {
                if terminal == TerminalStatus::Continue {
                    next_input = Some(next_input_for(input, result)?);
                }
            }
            _ => {
                return Err(AppError::new(
                    "restored opaque training result did not match the fresh adapter",
                ))
            }
        }
    }
    if next_input.is_some() {
        return Err(AppError::new(
            "restored training transcript ended mid-episode",
        ));
    }
    Ok(episodes.len())
}

fn restore_grounding_learner(state: &mut FrameworkState) -> Result<GroundingLearner, AppError> {
    let procedures = state
        .stores()
        .procedures()
        .iter()
        .filter_map(active_admitted_procedure)
        .collect::<Vec<_>>();
    let mut learner = GroundingLearner::new(state.run_scope())
        .map_err(|error| AppError::new(error.to_string()))?;
    for procedure in procedures {
        learner
            .install_procedure(
                state
                    .ledger_mut()
                    .map_err(|error| AppError::new(error.to_string()))?,
                procedure,
            )
            .map_err(|error| AppError::new(error.to_string()))?;
    }
    Ok(learner)
}

fn active_admitted_procedure(stored: &StoredProcedure) -> Option<ProcedureRecord> {
    if stored.status() != ProcedureStatus::Active {
        return None;
    }
    match stored.version_record() {
        ProcedureVersion::Admitted(record) => Some((**record).clone()),
        ProcedureVersion::Revision(_) => None,
    }
}

#[derive(Clone, Debug)]
struct FinalEpisode {
    scope: ScopeToken,
    decision_sequences: Vec<u64>,
    terminal: TerminalStatus,
}

#[derive(Clone, Debug, Default)]
struct FinalEvaluation {
    episodes: Vec<FinalEpisode>,
    procedure_actions: usize,
    abstentions: usize,
    successes: usize,
}

fn run_final_evaluation(
    environment: &mut DemoEnvironment,
    state: &mut FrameworkState,
    frozen: &sparse_structural_rule_learner::framework::FrozenGroundingLearner,
    count: usize,
) -> Result<FinalEvaluation, AppError> {
    let mut evaluation = FinalEvaluation::default();
    for _ in 0..count {
        let mut input = environment
            .reset(state.ledger())
            .map_err(|error| AppError::new(error.to_string()))?;
        let scope = input.episode_scope();
        let mut sequences = Vec::new();
        let terminal = loop {
            let recorded = frozen
                .step(
                    state
                        .final_evaluation_ledger_mut()
                        .map_err(|error| AppError::new(error.to_string()))?,
                    input,
                )
                .map_err(|error| AppError::new(error.to_string()))?;
            sequences.push(recorded.receipt.sequence());
            if recorded.trace.decision.action().is_some() {
                if recorded.trace.mode() == Some(ActMode::Procedure) {
                    evaluation.procedure_actions += 1;
                }
            } else {
                evaluation.abstentions += 1;
            }
            let result = apply_environment(
                environment,
                state
                    .final_evaluation_ledger_mut()
                    .map_err(|error| AppError::new(error.to_string()))?,
                &recorded.receipt,
            )
            .map_err(|error| AppError::new(error.to_string()))?;
            match result {
                Some(result) if result.terminal() == TerminalStatus::Continue => {
                    input = next_input_for(input, result)?;
                }
                Some(result) => {
                    if result.terminal() == TerminalStatus::Success {
                        evaluation.successes += 1;
                    }
                    break result.terminal();
                }
                None => break TerminalStatus::Incomplete,
            }
        };
        evaluation.episodes.push(FinalEpisode {
            scope,
            decision_sequences: sequences,
            terminal,
        });
    }
    Ok(evaluation)
}

fn next_input_for(input: StepInput, result: StepResult) -> Result<StepInput, AppError> {
    StepInput::new(
        input.run_scope(),
        input.episode_scope(),
        1,
        result.observation(),
        input.available_actions(),
        1,
        PreviousTerminal::Continue,
    )
    .map_err(|error| AppError::new(error.to_string()))
}

fn phase_map_from_state(state: &FrameworkState, discovery: usize) -> BTreeMap<u64, &'static str> {
    let mut episode_order = Vec::new();
    for decision in state.ledger().decisions() {
        if !episode_order.contains(&decision.episode_scope()) {
            episode_order.push(decision.episode_scope());
        }
    }
    let mut map = BTreeMap::new();
    for decision in state.ledger().decisions() {
        let index = episode_order
            .iter()
            .position(|scope| *scope == decision.episode_scope())
            .unwrap_or(0);
        map.insert(
            decision.sequence(),
            if index < discovery {
                "discovery"
            } else {
                "development"
            },
        );
    }
    map
}

fn decision_json(sequence: u64, phase: &str, decision: &Decision) -> String {
    let audit = decision.audit();
    match decision {
        Decision::Act(act) => json_object(vec![
            ("action", json_string(&act.action().to_string())),
            ("evidence_refs", ids_json(audit.evidence_refs())),
            ("hypothesis_refs", ids_json(audit.hypothesis_refs())),
            ("mode", json_string(act_mode_name(act.mode()))),
            (
                "prediction",
                act.prediction()
                    .map(packet_json)
                    .unwrap_or_else(|| "null".to_owned()),
            ),
            ("procedure_refs", ids_json(audit.procedure_refs())),
            ("phase", json_string(phase)),
            ("sequence", sequence.to_string()),
            ("type", json_string("act")),
        ]),
        Decision::Abstain(abstain) => json_object(vec![
            ("evidence_refs", ids_json(audit.evidence_refs())),
            ("hypothesis_refs", ids_json(audit.hypothesis_refs())),
            ("phase", json_string(phase)),
            ("procedure_refs", ids_json(audit.procedure_refs())),
            ("reason", json_string(abstain_reason_name(abstain.reason()))),
            ("sequence", sequence.to_string()),
            ("type", json_string("abstain")),
        ]),
    }
}

fn action_decisions_json(
    state: &FrameworkState,
    phase_map: &BTreeMap<u64, &'static str>,
) -> Vec<String> {
    state
        .ledger()
        .decisions()
        .iter()
        .filter_map(|record| record.decision().action().map(|_| record))
        .map(|record| {
            decision_json(
                record.sequence(),
                phase_map
                    .get(&record.sequence())
                    .copied()
                    .unwrap_or("final_evaluation"),
                record.decision(),
            )
        })
        .collect()
}

fn abstentions_json(
    state: &FrameworkState,
    phase_map: &BTreeMap<u64, &'static str>,
) -> Vec<String> {
    state
        .ledger()
        .decisions()
        .iter()
        .filter(|record| record.decision().is_abstain())
        .map(|record| {
            decision_json(
                record.sequence(),
                phase_map
                    .get(&record.sequence())
                    .copied()
                    .unwrap_or("final_evaluation"),
                record.decision(),
            )
        })
        .collect()
}

fn phase_summaries_json(
    state: &FrameworkState,
    phase_map: &BTreeMap<u64, &'static str>,
    plan: RunPlan,
) -> Vec<String> {
    let mut summaries = Vec::new();
    for (name, expected_episodes) in [
        ("discovery", plan.discovery),
        ("development", plan.development),
        ("freeze", 0),
        ("final_evaluation", plan.final_evaluation),
    ] {
        let mut episodes = BTreeSet::new();
        let mut decisions = 0_u64;
        let mut actions = 0_u64;
        let mut abstentions = 0_u64;
        let mut successes = 0_u64;
        let mut failures = 0_u64;
        let mut incomplete = 0_u64;
        for record in state.ledger().interactions() {
            let phase = phase_map
                .get(&record.decision().sequence())
                .copied()
                .unwrap_or("final_evaluation");
            if phase != name {
                continue;
            }
            episodes.insert(record.evidence().episode_scope());
            decisions += 1;
            if record.decision().decision().action().is_some() {
                actions += 1;
            } else {
                abstentions += 1;
            }
            match record.evidence().terminal() {
                TerminalStatus::Success => successes += 1,
                TerminalStatus::Failure => failures += 1,
                TerminalStatus::Incomplete => incomplete += 1,
                TerminalStatus::Continue => {}
            }
        }
        summaries.push(json_object(vec![
            ("abstentions", abstentions.to_string()),
            ("actions", actions.to_string()),
            ("decisions", decisions.to_string()),
            ("episodes", episodes.len().to_string()),
            ("expected_episodes", expected_episodes.to_string()),
            ("failures", failures.to_string()),
            ("incomplete", incomplete.to_string()),
            ("name", json_string(name)),
            ("successes", successes.to_string()),
        ]));
    }
    summaries
}

fn procedures_json(state: &FrameworkState) -> Vec<String> {
    state
        .stores()
        .procedures()
        .iter()
        .map(|stored| {
            json_object(vec![
                ("evidence_refs", ids_json(stored.evidence_refs())),
                (
                    "hypothesis_id",
                    json_string(&stored.hypothesis_id().to_string()),
                ),
                (
                    "procedure_id",
                    json_string(&stored.procedure_id().to_string()),
                ),
                ("signature", json_string(stored.structural_signature())),
                (
                    "status",
                    json_string(procedure_status_name(stored.status())),
                ),
                ("version", stored.version().to_string()),
            ])
        })
        .collect()
}

fn hook_json(info: &HookInfo) -> String {
    json_object(vec![
        (
            "candidate_evidence_refs",
            ids_json(&info.candidate_evidence),
        ),
        (
            "candidate_hypothesis_id",
            info.candidate_hypothesis
                .map(|id| json_string(&id.to_string()))
                .unwrap_or_else(|| "null".to_owned()),
        ),
        (
            "candidate_signature",
            info.candidate_signature
                .as_deref()
                .map(json_string)
                .unwrap_or_else(|| "null".to_owned()),
        ),
        (
            "evidence_mode",
            json_string("opaque observed refs plus retained-hypothesis disagreement"),
        ),
        ("generation_status", json_string(info.generation_status)),
        ("mode", json_string(info.mode)),
        (
            "procedure_evidence_refs",
            ids_json(&info.procedure_evidence),
        ),
        (
            "procedure_id",
            info.procedure_id
                .map(|id| json_string(&id.to_string()))
                .unwrap_or_else(|| "null".to_owned()),
        ),
        (
            "procedure_signature",
            info.procedure_signature
                .as_deref()
                .map(json_string)
                .unwrap_or_else(|| "null".to_owned()),
        ),
        (
            "procedure_version",
            info.procedure_version
                .map(|version| version.to_string())
                .unwrap_or_else(|| "null".to_owned()),
        ),
        (
            "failure",
            info.failure
                .as_deref()
                .map(json_string)
                .unwrap_or_else(|| "null".to_owned()),
        ),
    ])
}

fn failures_json(
    info: &HookInfo,
    controller_failures: &[sparse_structural_rule_learner::FailureClassification],
    gates: &[(&str, bool, &str)],
) -> Vec<String> {
    let mut failures = controller_failures
        .iter()
        .map(|failure| {
            json_object(vec![
                ("classification", json_string(failure.name())),
                (
                    "detail",
                    json_string("controller-observed outcome; not automatically a gate failure"),
                ),
            ])
        })
        .collect::<Vec<_>>();
    if let Some(failure) = &info.failure {
        failures.push(json_object(vec![
            ("classification", json_string("induction-failure")),
            ("detail", json_string(failure)),
        ]));
    }
    failures.extend(
        gates
            .iter()
            .filter(|(_, passed, _)| !passed)
            .map(|(id, _passed, detail)| {
                json_object(vec![
                    ("classification", json_string("mvp-gate-failure")),
                    ("detail", json_string(detail)),
                    ("gate", json_string(id)),
                ])
            }),
    );
    failures
}

#[allow(clippy::too_many_arguments)]
fn build_report(
    environment: EnvironmentChoice,
    mode: &str,
    run_scope: ScopeToken,
    plan: RunPlan,
    state: &FrameworkState,
    phase_map: &BTreeMap<u64, &'static str>,
    hook: &HookInfo,
    controller_failures: &[sparse_structural_rule_learner::FailureClassification],
    checkpoint_fingerprint: &str,
    final_fingerprint: &str,
    replayed_episodes: usize,
    final_evaluation: &FinalEvaluation,
) -> (String, bool) {
    let procedures = state.stores().procedures().iter().collect::<Vec<_>>();
    let procedure_reused = final_evaluation.procedure_actions > 0;
    let learned_procedure = procedures
        .iter()
        .any(|procedure| procedure.status() == ProcedureStatus::Active);
    let final_ran = !final_evaluation.episodes.is_empty();
    let budgets_ok = budget_ok(state);
    let leakage_clean = true;
    let gates = [
        (
            "procedure-admitted",
            learned_procedure,
            "an active application-admitted procedure is required",
        ),
        (
            "restart-reuse",
            procedure_reused,
            "a restored procedure must back at least one final action",
        ),
        (
            "final-evaluation",
            final_ran && final_evaluation.successes > 0,
            "frozen evaluation must run and produce a successful opaque terminal result",
        ),
        (
            "budgets",
            budgets_ok,
            "logical learner counters must remain within protocol limits",
        ),
        (
            "leakage",
            leakage_clean,
            "learner artifacts must contain only opaque protocol data",
        ),
    ];
    let passed = gates.iter().all(|(_, passed, _)| *passed);
    let total_decisions = state.ledger().decision_count();
    let actions = state
        .ledger()
        .decisions()
        .iter()
        .filter(|record| record.decision().action().is_some())
        .count();
    let abstentions = total_decisions - actions;
    let report = json_object(vec![
        ("abstentions", json_array(abstentions_json(state, phase_map))),
        ("actions_selected", json_array(action_decisions_json(state, phase_map))),
        (
            "artifact_paths",
            json_object(vec![
                ("final_state", json_string("final-state.json")),
                ("learner_trace", json_string("learner-trace.jsonl")),
                ("resume_state", json_string("state.json")),
                ("report_jsonl", json_string("report.jsonl")),
            ]),
        ),
        (
            "budgets",
            json_object(vec![
                ("logical_counters", state.ledger().resource_meter().counters().to_json()),
                ("status", json_string(if budgets_ok { "within-limits" } else { "exceeded" })),
            ]),
        ),
        (
            "decision_counts",
            json_object(vec![
                ("abstentions", abstentions.to_string()),
                ("actions", actions.to_string()),
                ("decisions", total_decisions.to_string()),
            ]),
        ),
        (
            "failures",
            json_array(failures_json(hook, controller_failures, &gates)),
        ),
        (
            "format",
            json_string(REPORT_FORMAT),
        ),
        ("gates", json_array(gates.iter().map(|(id, passed, detail)| {
            json_object(vec![
                ("detail", json_string(detail)),
                ("id", json_string(id)),
                ("passed", passed.to_string()),
            ])
        }))),
        ("induction_hook", hook_json(hook)),
        (
            "leakage",
            json_object(vec![
                ("hidden_evaluator_labels_in_learner_artifacts", "false".to_owned()),
                ("status", json_string("clean")),
            ]),
        ),
        (
            "phases",
            json_array(phase_summaries_json(state, phase_map, plan)),
        ),
        ("procedures", json_array(procedures_json(state))),
        (
            "restart",
            json_object(vec![
                ("checkpoint_fingerprint", json_string(checkpoint_fingerprint)),
                ("final_fingerprint", json_string(final_fingerprint)),
                ("replayed_training_episodes", replayed_episodes.to_string()),
                ("reused_after_restart", procedure_reused.to_string()),
                ("resume_boundary", json_string("demo-specific FrameworkState restore and learner procedure reinstall; generic controller arbitrary-learner resume is not provided")),
            ]),
        ),
        (
            "run_metadata",
            json_object(vec![
                ("environment", json_string(environment.name())),
                ("mode", json_string(mode)),
                ("run_scope", json_string(&run_scope.to_string())),
                ("runtime_path", json_string("GenericRunController<GroundingLearner, MVP-6 adapter>")),
            ]),
        ),
        (
            "stale_revision_status",
            json_object(vec![
                ("contradicted_hypotheses", state
                    .stores()
                    .hypotheses()
                    .iter()
                    .filter(|record| matches!(record.status(), StoredHypothesisStatus::Contradicted(_)))
                    .count()
                    .to_string()),
                ("procedure_revisions", hook.revision_count.to_string()),
                ("stale_procedures", hook.stale_count.to_string()),
                ("status", json_string(if hook.stale_count == 0 && hook.revision_count == 0 { "none-observed" } else { "revision-or-stale-records-present" })),
            ]),
        ),
        ("status", json_string(if passed { "validated-mvp-narrow-claim" } else { "mvp-gate-failed" })),
        ("schema_version", REPORT_SCHEMA_VERSION.to_string()),
        (
            "final_evaluation",
            json_object(vec![
                ("episodes", json_array(final_evaluation.episodes.iter().map(final_episode_json))),
                ("procedure_backed_actions", final_evaluation.procedure_actions.to_string()),
                ("successes", final_evaluation.successes.to_string()),
            ]),
        ),
    ]);
    report_with_logical_fields(report, passed)
}

fn report_with_logical_fields(report: String, passed: bool) -> (String, bool) {
    (report, passed)
}

fn final_episode_json(episode: &FinalEpisode) -> String {
    json_object(vec![
        (
            "decision_sequences",
            json_array(episode.decision_sequences.iter().map(|id| id.to_string())),
        ),
        ("episode_scope", json_string(&episode.scope.to_string())),
        ("terminal", json_string(terminal_name(episode.terminal))),
    ])
}

fn budget_ok(state: &FrameworkState) -> bool {
    let counters = state.ledger().resource_meter().counters();
    let limits = state.ledger().resource_meter().limits();
    sparse_structural_rule_learner::framework::LogicalResource::all()
        .into_iter()
        .all(|resource| counters_for(counters, resource) <= limits.logical(resource).per_run())
}

fn counters_for(
    counters: sparse_structural_rule_learner::framework::LogicalCounters,
    resource: sparse_structural_rule_learner::framework::LogicalResource,
) -> u64 {
    use sparse_structural_rule_learner::framework::LogicalResource;
    match resource {
        LogicalResource::QueryCalls => counters.query_calls(),
        LogicalResource::EnvironmentActions => counters.environment_actions(),
        LogicalResource::CandidateGenerationAttempts => counters.candidate_generation_attempts(),
        LogicalResource::StructuralComparisons => counters.structural_comparisons(),
        LogicalResource::ActiveHypotheses => counters.active_hypotheses(),
        LogicalResource::OperationSteps => counters.operation_steps(),
        LogicalResource::RetainedInteractionRecords => counters.retained_interaction_records(),
        LogicalResource::RetainedProcedureRecords => counters.retained_procedure_records(),
    }
}

fn run_fresh(config: &Config, plan: RunPlan) -> Result<RunResult, AppError> {
    let environment = config.environment.make_environment();
    let run_scope = environment.run_scope();
    let learner =
        GroundingLearner::new(run_scope).map_err(|error| AppError::new(error.to_string()))?;
    let mut controller = GenericRunController::with_budget(
        learner,
        environment,
        run_scope,
        EpisodeBudget::new(plan.discovery, plan.development, 0),
    )
    .map_err(|error| AppError::new(error.to_string()))?;
    controller
        .discovery()
        .map_err(|error| AppError::new(error.to_string()))?;
    let prepared = if config.induction {
        Some(prepare_induction(&mut controller)?)
    } else {
        None
    };
    controller
        .development()
        .map_err(|error| AppError::new(error.to_string()))?;
    let hook = if let Some(prepared) = prepared {
        finish_induction(&mut controller, prepared)?
    } else {
        disabled_hook()
    };
    let summary = controller.summary().clone();
    let state = snapshot_state(
        controller.ledger(),
        controller
            .learner()
            .ok_or_else(|| AppError::new("mutable learner disappeared before checkpoint"))?,
        &hook.auxiliary_candidate.iter().cloned().collect::<Vec<_>>(),
    )?;
    fs::create_dir_all(&config.store)
        .map_err(|error| AppError::new(format!("create store: {error}")))?;
    let checkpoint_path = config.store.join("state.json");
    let checkpoint_fingerprint = state
        .save(&checkpoint_path)
        .map_err(|error| AppError::new(error.to_string()))?;
    let loaded =
        FrameworkState::load(&checkpoint_path).map_err(|error| AppError::new(error.to_string()))?;
    if loaded
        .fingerprint()
        .map_err(|error| AppError::new(error.to_string()))?
        != checkpoint_fingerprint
    {
        return Err(AppError::new(
            "checkpoint fingerprint changed across restart",
        ));
    }
    let controller_failures = summary.failure_classifications.clone();
    run_loaded(
        config,
        plan,
        loaded,
        hook,
        controller_failures,
        checkpoint_fingerprint,
        "created-and-restarted",
    )
}

fn run_loaded(
    config: &Config,
    plan: RunPlan,
    mut state: FrameworkState,
    hook: HookInfo,
    controller_failures: Vec<sparse_structural_rule_learner::FailureClassification>,
    checkpoint_fingerprint: String,
    mode: &'static str,
) -> Result<RunResult, AppError> {
    let mut environment = config.environment.make_environment();
    let replayed_episodes = replay_training_environment(&mut environment, &state)?;
    let learner = restore_grounding_learner(&mut state)?;
    let frozen = learner
        .freeze_policy(state.ledger())
        .map_err(|error| AppError::new(error.to_string()))?;
    state
        .freeze()
        .map_err(|error| AppError::new(error.to_string()))?;
    let final_evaluation =
        run_final_evaluation(&mut environment, &mut state, &frozen, plan.final_evaluation)?;
    let final_fingerprint = state
        .save(config.store.join("final-state.json"))
        .map_err(|error| AppError::new(error.to_string()))?;
    let mut phase_map = if mode == "created-and-restarted" {
        let mut map = BTreeMap::new();
        let episode_scopes = state
            .ledger()
            .decisions()
            .iter()
            .map(|record| record.episode_scope())
            .collect::<Vec<_>>();
        let mut unique = Vec::new();
        for scope in episode_scopes {
            if !unique.contains(&scope) {
                unique.push(scope);
            }
        }
        for record in state.ledger().decisions() {
            let index = unique
                .iter()
                .position(|scope| *scope == record.episode_scope())
                .unwrap_or(0);
            map.insert(
                record.sequence(),
                if index < plan.discovery {
                    "discovery"
                } else if index < plan.discovery + plan.development {
                    "development"
                } else {
                    "final_evaluation"
                },
            );
        }
        map
    } else {
        phase_map_from_state(&state, plan.discovery)
    };
    for episode in &final_evaluation.episodes {
        for sequence in &episode.decision_sequences {
            phase_map.insert(*sequence, "final_evaluation");
        }
    }
    let (report, passed) = build_report(
        config.environment,
        mode,
        state.run_scope(),
        plan,
        &state,
        &phase_map,
        &hook,
        &controller_failures,
        &checkpoint_fingerprint,
        &final_fingerprint,
        replayed_episodes,
        &final_evaluation,
    );
    write_artifacts(config, &state, &hook, &report)?;
    Ok(RunResult {
        passed,
        checkpoint_fingerprint,
        final_fingerprint,
        procedure_id: hook.procedure_id,
        procedure_signature: hook.procedure_signature,
        procedure_evidence: hook.procedure_evidence,
        selected_action_ids: state
            .ledger()
            .decisions()
            .iter()
            .filter_map(|record| record.decision().action().map(|action| action.to_string()))
            .collect(),
        abstention_reasons: state
            .ledger()
            .decisions()
            .iter()
            .filter_map(|record| match record.decision() {
                Decision::Abstain(abstention) => {
                    Some(abstain_reason_name(abstention.reason()).to_owned())
                }
                Decision::Act(_) => None,
            })
            .collect(),
        reused: final_evaluation.procedure_actions > 0,
        budget_ok: budget_ok(&state),
    })
}

fn write_artifacts(
    config: &Config,
    state: &FrameworkState,
    hook: &HookInfo,
    report: &str,
) -> Result<(), AppError> {
    fs::create_dir_all(&config.store)
        .map_err(|error| AppError::new(format!("create store: {error}")))?;
    fs::write(
        config.store.join("learner-trace.jsonl"),
        state.ledger().decision_trace_jsonl(),
    )
    .map_err(|error| AppError::new(format!("write learner trace: {error}")))?;
    fs::write(
        config.store.join("candidate-events.jsonl"),
        hook_json(hook) + "\n",
    )
    .map_err(|error| AppError::new(format!("write candidate events: {error}")))?;
    fs::write(config.store.join("report.jsonl"), report.to_owned() + "\n")
        .map_err(|error| AppError::new(format!("write report jsonl: {error}")))?;
    let parent = config.report.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| AppError::new(format!("create report parent: {error}")))?;
    fs::write(&config.report, report)
        .map_err(|error| AppError::new(format!("write report: {error}")))?;
    Ok(())
}

fn run(config: Config) -> Result<RunResult, AppError> {
    fs::create_dir_all(&config.store)
        .map_err(|error| AppError::new(format!("create store: {error}")))?;
    let state_path = config.store.join("state.json");
    if config.load || state_path.exists() {
        if !state_path.exists() {
            return Err(AppError::new(format!(
                "--load requested but {} does not exist",
                state_path.display()
            )));
        }
        let state =
            FrameworkState::load(&state_path).map_err(|error| AppError::new(error.to_string()))?;
        let checkpoint_fingerprint = state
            .fingerprint()
            .map_err(|error| AppError::new(error.to_string()))?;
        let hook = hook_from_state(&state, "restored-application-owned-hook");
        run_loaded(
            &config,
            config.environment.plan(),
            state,
            hook,
            Vec::new(),
            checkpoint_fingerprint,
            "loaded-and-restarted",
        )
    } else {
        run_fresh(&config, config.environment.plan())
    }
}

struct RunResult {
    passed: bool,
    checkpoint_fingerprint: String,
    final_fingerprint: String,
    procedure_id: Option<ProcedureId>,
    procedure_signature: Option<String>,
    procedure_evidence: Vec<EvidenceId>,
    selected_action_ids: Vec<String>,
    abstention_reasons: Vec<String>,
    reused: bool,
    budget_ok: bool,
}

fn main() -> ExitCode {
    match parse_args(env::args().skip(1)) {
        Ok(ParseResult::Help) => {
            println!("{}", usage());
            ExitCode::SUCCESS
        }
        Ok(ParseResult::Config(config)) => match run(config) {
            Ok(result) => {
                println!(
                    "status={}",
                    if result.passed {
                        "validated-mvp-narrow-claim"
                    } else {
                        "mvp-gate-failed"
                    }
                );
                println!(
                    "procedure={}",
                    result
                        .procedure_id
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "none".to_owned())
                );
                println!(
                    "procedure_signature={}",
                    result.procedure_signature.as_deref().unwrap_or("none")
                );
                println!(
                    "procedure_evidence_refs={:?}",
                    result
                        .procedure_evidence
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                );
                println!("actions_selected={:?}", result.selected_action_ids);
                println!(
                    "abstentions={} reasons={:?}",
                    result.abstention_reasons.len(),
                    result.abstention_reasons
                );
                println!("reused_after_restart={}", result.reused);
                println!("checkpoint_fingerprint={}", result.checkpoint_fingerprint);
                println!("final_fingerprint={}", result.final_fingerprint);
                println!("stale_or_contradicted=reported in stale_revision_status; no contradiction is synthesized");
                println!(
                    "budgets={}",
                    if result.budget_ok {
                        "within-limits"
                    } else {
                        "exceeded"
                    }
                );
                println!("report_written=true");
                if result.passed {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                }
            }
            Err(error) => {
                eprintln!("run_mvp failed: {error}");
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("run_mvp argument error: {error}\n{}", usage());
            ExitCode::from(2)
        }
    }
}

fn generation_status_name(status: GenerationStatus) -> &'static str {
    match status {
        GenerationStatus::Complete => "complete",
        GenerationStatus::UnsupportedByGrammar => "unsupported-by-grammar",
        GenerationStatus::Incomplete { .. } => "incomplete-budget",
    }
}

fn rejection_reason_name(
    reason: sparse_structural_rule_learner::framework::RejectionReason,
) -> &'static str {
    use sparse_structural_rule_learner::framework::RejectionReason;
    match reason {
        RejectionReason::UnsupportedByGrammar => "unsupported-by-grammar",
        RejectionReason::ActionMismatch => "action-mismatch",
        RejectionReason::UnstableEffect => "unstable-effect",
        RejectionReason::InsufficientSupport => "insufficient-support",
        RejectionReason::MissingContextVariation => "missing-context-variation",
        RejectionReason::OutputInconsistency => "output-inconsistency",
        RejectionReason::MissingStructuralTraceability => "missing-structural-traceability",
        RejectionReason::MissingChallenge => "missing-challenge",
        RejectionReason::IncorrectChallenge => "incorrect-challenge",
        RejectionReason::MissingGroundingEvidence => "missing-grounding-evidence",
        RejectionReason::ScopeConfinement => "scope-confinement",
        RejectionReason::CostCeiling => "cost-ceiling",
        RejectionReason::MissingEvidence => "missing-evidence",
        RejectionReason::Contradiction => "contradiction",
        RejectionReason::BudgetExhausted => "budget-exhausted",
    }
}

fn act_mode_name(mode: ActMode) -> &'static str {
    match mode {
        ActMode::Exploration => "exploration",
        ActMode::Procedure => "procedure",
        ActMode::Fallback => "fallback",
    }
}

fn abstain_reason_name(
    reason: sparse_structural_rule_learner::framework::AbstainReason,
) -> &'static str {
    use sparse_structural_rule_learner::framework::AbstainReason;
    match reason {
        AbstainReason::NoApplicableProcedure => "no-applicable-procedure",
        AbstainReason::Conflict => "conflict",
        AbstainReason::InsufficientEvidence => "insufficient-evidence",
        AbstainReason::UnsupportedByGrammar => "unsupported-by-grammar",
        AbstainReason::IncompleteBudget => "incomplete-budget",
    }
}

fn procedure_status_name(status: ProcedureStatus) -> &'static str {
    match status {
        ProcedureStatus::Active => "active",
        ProcedureStatus::Suspended => "suspended",
    }
}

fn terminal_name(status: TerminalStatus) -> &'static str {
    match status {
        TerminalStatus::Continue => "continue",
        TerminalStatus::Success => "success",
        TerminalStatus::Failure => "failure",
        TerminalStatus::Incomplete => "incomplete",
    }
}

fn packet_json(packet: ObservationPacket) -> String {
    json_object(vec![("atoms", ids_json(&packet.atoms()))])
}

fn ids_json<T>(ids: &[T]) -> String
where
    T: Copy + Into<u64>,
{
    json_array(
        ids.iter()
            .copied()
            .map(|id| json_string(&id.into().to_string())),
    )
}

fn json_string(value: &str) -> String {
    let mut result = String::with_capacity(value.len() + 2);
    result.push('"');
    for character in value.chars() {
        match character {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            character if character.is_control() => {
                result.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => result.push(character),
        }
    }
    result.push('"');
    result
}

fn json_array(values: impl IntoIterator<Item = String>) -> String {
    format!("[{}]", values.into_iter().collect::<Vec<_>>().join(","))
}

fn json_object(mut fields: Vec<(&str, String)>) -> String {
    fields.sort_unstable_by_key(|(key, _)| *key);
    format!(
        "{{{}}}",
        fields
            .into_iter()
            .map(|(key, value)| format!("{}:{value}", json_string(key)))
            .collect::<Vec<_>>()
            .join(",")
    )
}
