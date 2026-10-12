//! Evaluator-only world and deterministic v2 generators.
//!
//! This module owns logical labels, seeds, mappings, hidden state, the fixed
//! schedule, and transition behavior.  None of those values are fields on the
//! learner-facing [`StepInput`](super::opaque_protocol::StepInput).  The
//! `apply_recorded` entry point requires a receipt created by the interaction
//! ledger, making the decision-before-result order explicit at the seam.

use std::collections::HashSet;
use std::fmt;

use super::interaction_ledger::{DecisionReceipt, InteractionLedger, LedgerError};
use super::opaque_protocol::{
    ActionId, AtomId, ObservationPacket, PreviousTerminal, ProtocolError, ScopeToken, StepInput,
    StepResult, TerminalStatus, AVAILABLE_ACTION_COUNT,
};
use super::{Evaluator, LearnerState};

pub const EPISODE_COUNT: usize = 36;
pub const TRAINING_EPISODE_COUNT: usize = 12;
pub const DEVELOPMENT_EPISODE_COUNT: usize = 8;
pub const FINAL_EPISODE_COUNT: usize = 16;
pub const ACTION_CEILING: u8 = 2;

pub const PRIMARY_WORLD_SEED: u64 = 2_026_101_101;
pub const PRIMARY_SCOPE_SEED: u64 = 2_026_101_201;
pub const PRIMARY_RANDOM_BASELINE_SEED: u64 = 2_026_101_301;
pub const PERMUTATION_WORLD_SEED: u64 = 2_026_101_102;
pub const PERMUTATION_SCOPE_SEED: u64 = 2_026_101_202;
pub const PERMUTATION_RANDOM_BASELINE_SEED: u64 = 2_026_101_302;
pub const ACTION_SEED_XOR: u64 = 0xD1B54A32D192ED03;

/// The exact SplitMix64 step frozen by protocol v1.0.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
}

/// Fisher-Yates draws from `length - 1` down to `1`, consuming one
/// SplitMix64 output for every swap exactly as specified by the protocol.
pub fn fisher_yates<T: Copy>(labels: &[T], seed: u64) -> Vec<T> {
    let mut result = labels.to_vec();
    let mut random = SplitMix64::new(seed);
    for index in (1..result.len()).rev() {
        let other = (random.next_u64() % (index as u64 + 1)) as usize;
        result.swap(index, other);
    }
    result
}

pub fn fisher_yates_indices(length: usize, seed: u64) -> Vec<usize> {
    let labels: Vec<usize> = (0..length).collect();
    fisher_yates(&labels, seed)
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum HiddenAtom {
    C0,
    C1,
    C2,
    C3,
    M0,
    M1,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum HiddenAction {
    A0,
    A1,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IdentifierMappings {
    atom_wire_order: [HiddenAtom; 6],
    action_wire_order: [HiddenAction; AVAILABLE_ACTION_COUNT],
}

impl IdentifierMappings {
    pub fn primary() -> Self {
        Self::from_world_seed(PRIMARY_WORLD_SEED)
    }

    pub fn permutation() -> Self {
        Self::from_world_seed(PERMUTATION_WORLD_SEED)
    }

    pub fn from_world_seed(world_seed: u64) -> Self {
        let atom_labels = [
            HiddenAtom::C0,
            HiddenAtom::C1,
            HiddenAtom::C2,
            HiddenAtom::C3,
            HiddenAtom::M0,
            HiddenAtom::M1,
        ];
        let action_labels = [HiddenAction::A0, HiddenAction::A1];
        let atoms = fisher_yates(&atom_labels, world_seed);
        let actions = fisher_yates(&action_labels, world_seed ^ ACTION_SEED_XOR);
        Self {
            atom_wire_order: [atoms[0], atoms[1], atoms[2], atoms[3], atoms[4], atoms[5]],
            action_wire_order: [actions[0], actions[1]],
        }
    }

    pub const fn atom_wire_order(self) -> [HiddenAtom; 6] {
        self.atom_wire_order
    }

    pub const fn action_wire_order(self) -> [HiddenAction; AVAILABLE_ACTION_COUNT] {
        self.action_wire_order
    }

    pub fn atom_id(self, atom: HiddenAtom) -> AtomId {
        for (index, candidate) in self.atom_wire_order.iter().enumerate() {
            if *candidate == atom {
                return AtomId::new(index as u64);
            }
        }
        unreachable!("all hidden atoms are present in the mapping")
    }

    pub fn action_id(self, action: HiddenAction) -> ActionId {
        for (index, candidate) in self.action_wire_order.iter().enumerate() {
            if *candidate == action {
                return ActionId::new(index as u64);
            }
        }
        unreachable!("all hidden actions are present in the mapping")
    }

    pub fn hidden_atom(self, id: AtomId) -> Option<HiddenAtom> {
        self.atom_wire_order.get(id.value() as usize).copied()
    }

    pub fn hidden_action(self, id: ActionId) -> Option<HiddenAction> {
        self.action_wire_order.get(id.value() as usize).copied()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RunVariant {
    Primary,
    SymbolActionPermutation,
}

impl RunVariant {
    pub const fn world_seed(self) -> u64 {
        match self {
            Self::Primary => PRIMARY_WORLD_SEED,
            Self::SymbolActionPermutation => PERMUTATION_WORLD_SEED,
        }
    }

    pub const fn scope_seed(self) -> u64 {
        match self {
            Self::Primary => PRIMARY_SCOPE_SEED,
            Self::SymbolActionPermutation => PERMUTATION_SCOPE_SEED,
        }
    }

    pub const fn random_baseline_seed(self) -> u64 {
        match self {
            Self::Primary => PRIMARY_RANDOM_BASELINE_SEED,
            Self::SymbolActionPermutation => PERMUTATION_RANDOM_BASELINE_SEED,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::SymbolActionPermutation => "symbol-action-permutation",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Partition {
    Training,
    Development,
    Final,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EpisodeRole {
    DiscoverySupport,
    PreconditionChallenge,
    RepeatedSupport,
    RepeatedChallenge,
    ApplicableChallenge,
    NonApplicableChallenge,
    RetentionCheckBeforeFreeze,
    RetentionChallengeBeforeFreeze,
    HeldOutApplicable,
    HeldOutNonApplicable,
    FrozenRetentionApplicable,
    FrozenRetentionNonApplicable,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EpisodeSpec {
    order: usize,
    label: &'static str,
    initial_context: HiddenAtom,
    initial_marker: HiddenAtom,
    partition: Partition,
    role: EpisodeRole,
}

impl EpisodeSpec {
    pub const fn order(self) -> usize {
        self.order
    }

    pub const fn label(self) -> &'static str {
        self.label
    }

    pub const fn initial_context(self) -> HiddenAtom {
        self.initial_context
    }

    pub const fn initial_marker(self) -> HiddenAtom {
        self.initial_marker
    }

    pub const fn partition(self) -> Partition {
        self.partition
    }

    pub const fn role(self) -> EpisodeRole {
        self.role
    }
}

const fn episode(
    order: usize,
    label: &'static str,
    context: HiddenAtom,
    marker: HiddenAtom,
    partition: Partition,
    role: EpisodeRole,
) -> EpisodeSpec {
    EpisodeSpec {
        order,
        label,
        initial_context: context,
        initial_marker: marker,
        partition,
        role,
    }
}

const SCHEDULE: [EpisodeSpec; EPISODE_COUNT] = [
    episode(
        1,
        "T01",
        HiddenAtom::C0,
        HiddenAtom::M0,
        Partition::Training,
        EpisodeRole::DiscoverySupport,
    ),
    episode(
        2,
        "T02",
        HiddenAtom::C1,
        HiddenAtom::M0,
        Partition::Training,
        EpisodeRole::DiscoverySupport,
    ),
    episode(
        3,
        "T03",
        HiddenAtom::C0,
        HiddenAtom::M1,
        Partition::Training,
        EpisodeRole::PreconditionChallenge,
    ),
    episode(
        4,
        "T04",
        HiddenAtom::C1,
        HiddenAtom::M1,
        Partition::Training,
        EpisodeRole::PreconditionChallenge,
    ),
    episode(
        5,
        "T05",
        HiddenAtom::C0,
        HiddenAtom::M0,
        Partition::Training,
        EpisodeRole::RepeatedSupport,
    ),
    episode(
        6,
        "T06",
        HiddenAtom::C1,
        HiddenAtom::M0,
        Partition::Training,
        EpisodeRole::RepeatedSupport,
    ),
    episode(
        7,
        "T07",
        HiddenAtom::C0,
        HiddenAtom::M1,
        Partition::Training,
        EpisodeRole::RepeatedChallenge,
    ),
    episode(
        8,
        "T08",
        HiddenAtom::C1,
        HiddenAtom::M1,
        Partition::Training,
        EpisodeRole::RepeatedChallenge,
    ),
    episode(
        9,
        "T09",
        HiddenAtom::C0,
        HiddenAtom::M0,
        Partition::Training,
        EpisodeRole::DiscoverySupport,
    ),
    episode(
        10,
        "T10",
        HiddenAtom::C1,
        HiddenAtom::M0,
        Partition::Training,
        EpisodeRole::DiscoverySupport,
    ),
    episode(
        11,
        "T11",
        HiddenAtom::C0,
        HiddenAtom::M1,
        Partition::Training,
        EpisodeRole::RepeatedChallenge,
    ),
    episode(
        12,
        "T12",
        HiddenAtom::C1,
        HiddenAtom::M1,
        Partition::Training,
        EpisodeRole::RepeatedChallenge,
    ),
    episode(
        13,
        "D01",
        HiddenAtom::C0,
        HiddenAtom::M0,
        Partition::Development,
        EpisodeRole::ApplicableChallenge,
    ),
    episode(
        14,
        "D02",
        HiddenAtom::C1,
        HiddenAtom::M0,
        Partition::Development,
        EpisodeRole::ApplicableChallenge,
    ),
    episode(
        15,
        "D03",
        HiddenAtom::C0,
        HiddenAtom::M1,
        Partition::Development,
        EpisodeRole::NonApplicableChallenge,
    ),
    episode(
        16,
        "D04",
        HiddenAtom::C1,
        HiddenAtom::M1,
        Partition::Development,
        EpisodeRole::NonApplicableChallenge,
    ),
    episode(
        17,
        "D05",
        HiddenAtom::C0,
        HiddenAtom::M0,
        Partition::Development,
        EpisodeRole::RetentionCheckBeforeFreeze,
    ),
    episode(
        18,
        "D06",
        HiddenAtom::C1,
        HiddenAtom::M0,
        Partition::Development,
        EpisodeRole::RetentionCheckBeforeFreeze,
    ),
    episode(
        19,
        "D07",
        HiddenAtom::C0,
        HiddenAtom::M1,
        Partition::Development,
        EpisodeRole::RetentionChallengeBeforeFreeze,
    ),
    episode(
        20,
        "D08",
        HiddenAtom::C1,
        HiddenAtom::M1,
        Partition::Development,
        EpisodeRole::RetentionChallengeBeforeFreeze,
    ),
    episode(
        21,
        "F01",
        HiddenAtom::C2,
        HiddenAtom::M0,
        Partition::Final,
        EpisodeRole::HeldOutApplicable,
    ),
    episode(
        22,
        "F02",
        HiddenAtom::C3,
        HiddenAtom::M0,
        Partition::Final,
        EpisodeRole::HeldOutApplicable,
    ),
    episode(
        23,
        "F03",
        HiddenAtom::C2,
        HiddenAtom::M1,
        Partition::Final,
        EpisodeRole::HeldOutNonApplicable,
    ),
    episode(
        24,
        "F04",
        HiddenAtom::C3,
        HiddenAtom::M1,
        Partition::Final,
        EpisodeRole::HeldOutNonApplicable,
    ),
    episode(
        25,
        "F05",
        HiddenAtom::C2,
        HiddenAtom::M0,
        Partition::Final,
        EpisodeRole::HeldOutApplicable,
    ),
    episode(
        26,
        "F06",
        HiddenAtom::C3,
        HiddenAtom::M0,
        Partition::Final,
        EpisodeRole::HeldOutApplicable,
    ),
    episode(
        27,
        "F07",
        HiddenAtom::C2,
        HiddenAtom::M1,
        Partition::Final,
        EpisodeRole::HeldOutNonApplicable,
    ),
    episode(
        28,
        "F08",
        HiddenAtom::C3,
        HiddenAtom::M1,
        Partition::Final,
        EpisodeRole::HeldOutNonApplicable,
    ),
    episode(
        29,
        "F09",
        HiddenAtom::C2,
        HiddenAtom::M0,
        Partition::Final,
        EpisodeRole::HeldOutApplicable,
    ),
    episode(
        30,
        "F10",
        HiddenAtom::C3,
        HiddenAtom::M0,
        Partition::Final,
        EpisodeRole::HeldOutApplicable,
    ),
    episode(
        31,
        "F11",
        HiddenAtom::C2,
        HiddenAtom::M1,
        Partition::Final,
        EpisodeRole::HeldOutNonApplicable,
    ),
    episode(
        32,
        "F12",
        HiddenAtom::C3,
        HiddenAtom::M1,
        Partition::Final,
        EpisodeRole::HeldOutNonApplicable,
    ),
    episode(
        33,
        "F13",
        HiddenAtom::C0,
        HiddenAtom::M0,
        Partition::Final,
        EpisodeRole::FrozenRetentionApplicable,
    ),
    episode(
        34,
        "F14",
        HiddenAtom::C1,
        HiddenAtom::M0,
        Partition::Final,
        EpisodeRole::FrozenRetentionApplicable,
    ),
    episode(
        35,
        "F15",
        HiddenAtom::C0,
        HiddenAtom::M1,
        Partition::Final,
        EpisodeRole::FrozenRetentionNonApplicable,
    ),
    episode(
        36,
        "F16",
        HiddenAtom::C1,
        HiddenAtom::M1,
        Partition::Final,
        EpisodeRole::FrozenRetentionNonApplicable,
    ),
];

pub fn fixed_schedule() -> &'static [EpisodeSpec; EPISODE_COUNT] {
    &SCHEDULE
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunSeeds {
    pub world_seed: u64,
    pub scope_seed: u64,
    pub random_baseline_seed: u64,
}

impl RunSeeds {
    pub const fn for_variant(variant: RunVariant) -> Self {
        Self {
            world_seed: variant.world_seed(),
            scope_seed: variant.scope_seed(),
            random_baseline_seed: variant.random_baseline_seed(),
        }
    }
}

#[derive(Debug)]
pub enum WorldError {
    Protocol(ProtocolError),
    Ledger(LedgerError),
    NoEpisodeAvailable,
    EpisodeAlreadyActive,
    NoActiveEpisode,
    InvalidAction(ActionId),
    ActionBudgetExhausted,
    ReceiptRunScopeMismatch,
    ReceiptEpisodeScopeMismatch,
    ReceiptStepMismatch { expected: u8, actual: u8 },
    ReceiptHasNoAction,
}

impl fmt::Display for WorldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => std::fmt::Display::fmt(error, formatter),
            Self::Ledger(error) => std::fmt::Display::fmt(error, formatter),
            Self::NoEpisodeAvailable => write!(formatter, "the fixed schedule is exhausted"),
            Self::EpisodeAlreadyActive => {
                write!(formatter, "the current episode must finish first")
            }
            Self::NoActiveEpisode => write!(formatter, "there is no active episode"),
            Self::InvalidAction(action) => write!(formatter, "invalid evaluator action {action}"),
            Self::ActionBudgetExhausted => write!(formatter, "episode action budget is exhausted"),
            Self::ReceiptRunScopeMismatch => write!(formatter, "receipt belongs to another run"),
            Self::ReceiptEpisodeScopeMismatch => {
                write!(formatter, "receipt belongs to another episode")
            }
            Self::ReceiptStepMismatch { expected, actual } => {
                write!(
                    formatter,
                    "receipt step {actual} does not match expected step {expected}"
                )
            }
            Self::ReceiptHasNoAction => write!(formatter, "an abstention cannot be applied"),
        }
    }
}

impl std::error::Error for WorldError {}

impl From<ProtocolError> for WorldError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl From<LedgerError> for WorldError {
    fn from(error: LedgerError) -> Self {
        Self::Ledger(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ActiveEpisode {
    scope: ScopeToken,
    context: HiddenAtom,
    marker: HiddenAtom,
    action_count: u8,
}

/// The hidden transition adapter.  Logical fields are intentionally available
/// only through evaluator audit methods in this named module.
pub struct EvaluatorWorld {
    variant: RunVariant,
    seeds: RunSeeds,
    mappings: IdentifierMappings,
    run_scope: ScopeToken,
    episode_scopes: [ScopeToken; EPISODE_COUNT],
    next_episode: usize,
    active: Option<ActiveEpisode>,
}

impl EvaluatorWorld {
    pub fn new(variant: RunVariant) -> Self {
        let seeds = RunSeeds::for_variant(variant);
        let mappings = IdentifierMappings::from_world_seed(seeds.world_seed);
        let (run_scope, episode_scopes) = generate_scopes(seeds.scope_seed);
        Self {
            variant,
            seeds,
            mappings,
            run_scope,
            episode_scopes,
            next_episode: 0,
            active: None,
        }
    }

    pub fn primary() -> Self {
        Self::new(RunVariant::Primary)
    }

    /// Constructs the secondary run with its own mapping, scope stream, and
    /// fresh learner state.  It never reuses primary run tokens.
    pub fn permutation() -> Self {
        Self::new(RunVariant::SymbolActionPermutation)
    }

    pub fn new_permutation_run() -> IsolatedRun {
        IsolatedRun::permutation()
    }

    pub const fn variant(&self) -> RunVariant {
        self.variant
    }

    pub const fn seeds(&self) -> RunSeeds {
        self.seeds
    }

    pub const fn mappings(&self) -> IdentifierMappings {
        self.mappings
    }

    pub const fn run_scope(&self) -> ScopeToken {
        self.run_scope
    }

    pub const fn episode_scope(&self, index: usize) -> Option<ScopeToken> {
        if index < EPISODE_COUNT {
            Some(self.episode_scopes[index])
        } else {
            None
        }
    }

    pub const fn episode_scopes(&self) -> &[ScopeToken; EPISODE_COUNT] {
        &self.episode_scopes
    }

    pub fn schedule(&self) -> &'static [EpisodeSpec; EPISODE_COUNT] {
        fixed_schedule()
    }

    pub const fn next_episode_index(&self) -> usize {
        self.next_episode
    }

    pub fn reset(&mut self) -> Result<StepInput, WorldError> {
        if self.active.is_some() {
            return Err(WorldError::EpisodeAlreadyActive);
        }
        if self.next_episode >= EPISODE_COUNT {
            return Err(WorldError::NoEpisodeAvailable);
        }
        let index = self.next_episode;
        self.next_episode += 1;
        let spec = SCHEDULE[index];
        let active = ActiveEpisode {
            scope: self.episode_scopes[index],
            context: spec.initial_context,
            marker: spec.initial_marker,
            action_count: 0,
        };
        self.active = Some(active);
        self.input_for(active, 0, PreviousTerminal::None, ACTION_CEILING)
    }

    fn apply_hidden(&mut self, action: ActionId) -> Result<StepResult, WorldError> {
        let mut active = self.active.ok_or(WorldError::NoActiveEpisode)?;
        if active.action_count >= ACTION_CEILING {
            return Err(WorldError::ActionBudgetExhausted);
        }
        let hidden_action = self
            .mappings
            .hidden_action(action)
            .ok_or(WorldError::InvalidAction(action))?;

        if hidden_action == HiddenAction::A0 && active.marker == HiddenAtom::M0 {
            active.marker = HiddenAtom::M1;
        }
        active.action_count += 1;
        let terminal = if active.marker == HiddenAtom::M1 {
            TerminalStatus::Success
        } else if active.action_count == ACTION_CEILING {
            TerminalStatus::Failure
        } else {
            TerminalStatus::Continue
        };
        let result = StepResult::new(self.packet(active.context, active.marker), terminal)?;
        if terminal == TerminalStatus::Continue {
            self.active = Some(active);
        } else {
            self.active = None;
        }
        Ok(result)
    }

    pub fn next_input(&self) -> Result<StepInput, WorldError> {
        let active = self.active.ok_or(WorldError::NoActiveEpisode)?;
        if active.action_count != 1 {
            return Err(WorldError::ActionBudgetExhausted);
        }
        self.input_for(active, 1, PreviousTerminal::Continue, 1)
    }

    /// Resolve only after the ledger has serialized the decision. The ledger
    /// also receives the returned result before this call returns.
    pub fn apply_recorded(
        &mut self,
        ledger: &mut InteractionLedger,
        receipt: &DecisionReceipt,
    ) -> Result<Option<StepResult>, WorldError> {
        ledger.validate_receipt(receipt)?;
        if receipt.record().run_scope() != self.run_scope {
            return Err(WorldError::ReceiptRunScopeMismatch);
        }
        let active = self.active.ok_or(WorldError::NoActiveEpisode)?;
        if receipt.record().episode_scope() != active.scope {
            return Err(WorldError::ReceiptEpisodeScopeMismatch);
        }
        if receipt.record().step_index() != active.action_count {
            return Err(WorldError::ReceiptStepMismatch {
                expected: active.action_count,
                actual: receipt.record().step_index(),
            });
        }
        let action = match receipt.action() {
            Some(action) => action,
            None => {
                ledger.record_result(receipt, None)?;
                self.active = None;
                return Ok(None);
            }
        };
        ledger.authorize_action(receipt)?;
        let result = self.apply_hidden(action)?;
        ledger.record_result(receipt, Some(result))?;
        Ok(Some(result))
    }

    fn input_for(
        &self,
        active: ActiveEpisode,
        step_index: u8,
        previous_terminal: PreviousTerminal,
        remaining_budget: u8,
    ) -> Result<StepInput, WorldError> {
        StepInput::new(
            self.run_scope,
            active.scope,
            step_index,
            self.packet(active.context, active.marker),
            [ActionId::new(0), ActionId::new(1)],
            remaining_budget,
            previous_terminal,
        )
        .map_err(WorldError::from)
    }

    fn packet(&self, context: HiddenAtom, marker: HiddenAtom) -> ObservationPacket {
        ObservationPacket::new([
            self.mappings.atom_id(context),
            self.mappings.atom_id(marker),
            self.mappings.atom_id(context),
        ])
    }

    pub fn logical_state_for_audit(&self) -> Option<(HiddenAtom, HiddenAtom, u8)> {
        self.active
            .map(|active| (active.context, active.marker, active.action_count))
    }
}

impl Evaluator for EvaluatorWorld {
    type Error = WorldError;

    fn reset(&mut self, ledger: &InteractionLedger) -> Result<StepInput, Self::Error> {
        if ledger.run_scope() != self.run_scope {
            return Err(WorldError::ReceiptRunScopeMismatch);
        }
        EvaluatorWorld::reset(self)
    }

    fn apply_recorded(
        &mut self,
        ledger: &mut InteractionLedger,
        receipt: &DecisionReceipt,
    ) -> Result<Option<StepResult>, Self::Error> {
        EvaluatorWorld::apply_recorded(self, ledger, receipt)
    }
}

pub struct IsolatedRun {
    world: EvaluatorWorld,
    learner_state: LearnerState,
}

impl IsolatedRun {
    pub fn primary() -> Self {
        let world = EvaluatorWorld::primary();
        let learner_state =
            LearnerState::new(world.run_scope()).expect("generated scope is nonzero");
        Self {
            world,
            learner_state,
        }
    }

    /// A fresh learner state is deliberately allocated here instead of being
    /// copied from the primary run.
    pub fn permutation() -> Self {
        let world = EvaluatorWorld::permutation();
        let learner_state =
            LearnerState::new(world.run_scope()).expect("generated scope is nonzero");
        Self {
            world,
            learner_state,
        }
    }

    pub fn world(&self) -> &EvaluatorWorld {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut EvaluatorWorld {
        &mut self.world
    }

    pub const fn learner_state(&self) -> &LearnerState {
        &self.learner_state
    }

    pub fn learner_state_mut(&mut self) -> &mut LearnerState {
        &mut self.learner_state
    }
}

pub fn new_permutation_run() -> IsolatedRun {
    IsolatedRun::permutation()
}

fn generate_scopes(seed: u64) -> (ScopeToken, [ScopeToken; EPISODE_COUNT]) {
    let mut random = SplitMix64::new(seed);
    let mut issued = HashSet::new();
    let run_scope = next_scope(&mut random, &mut issued);
    let mut episode_scopes = [ScopeToken::new(0); EPISODE_COUNT];
    for scope in &mut episode_scopes {
        *scope = next_scope(&mut random, &mut issued);
    }
    (run_scope, episode_scopes)
}

fn next_scope(random: &mut SplitMix64, issued: &mut HashSet<ScopeToken>) -> ScopeToken {
    loop {
        let value = ScopeToken::new(random.next_u64());
        if !value.is_zero() && issued.insert(value) {
            return value;
        }
    }
}
