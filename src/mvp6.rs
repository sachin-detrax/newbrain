//! MVP-6 environment adapters.
//!
//! The adapters in this module are evaluator-side implementations of the
//! public [`crate::runtime::Environment`] boundary. Their constructors expose
//! only a run token and opaque protocol behavior. Hidden labels, mappings,
//! schedules, seeds, transition state, and scoring remain private to the
//! evaluator implementation.

use std::fmt;

use crate::runtime::{Environment, EnvironmentActionError, EnvironmentTurn};
use crate::v2::evaluator_world::{EvaluatorWorld, WorldError};
use crate::v2::Evaluator;
use crate::{
    ActionId, AtomId, InteractionLedger, PreviousTerminal, ProtocolError, ScopeToken, StepInput,
    StepResult, TerminalStatus,
};

/// Adapter A: the existing v2 evaluator world through the reusable environment
/// interface. It deliberately has no public accessor for the wrapped world.
pub struct V2EnvironmentAdapter {
    world: EvaluatorWorld,
}

impl V2EnvironmentAdapter {
    /// Construct the frozen v2 primary mapping and schedule.
    pub fn primary() -> Self {
        Self {
            world: EvaluatorWorld::primary(),
        }
    }

    /// Construct the existing v2 symbol/action permutation run.
    pub fn permutation() -> Self {
        Self {
            world: EvaluatorWorld::permutation(),
        }
    }

    pub const fn run_scope(&self) -> ScopeToken {
        self.world.run_scope()
    }
}

#[derive(Debug)]
pub struct V2EnvironmentError(WorldError);

impl fmt::Display for V2EnvironmentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::error::Error for V2EnvironmentError {}

impl Environment for V2EnvironmentAdapter {
    type Error = V2EnvironmentError;

    fn reset(&mut self, ledger: &InteractionLedger) -> Result<StepInput, Self::Error> {
        Evaluator::reset(&mut self.world, ledger).map_err(V2EnvironmentError)
    }

    fn apply(
        &mut self,
        turn: &mut EnvironmentTurn<'_>,
    ) -> Result<(), EnvironmentActionError<Self::Error>> {
        let receipt = turn.receipt().clone();
        let result = Evaluator::apply_recorded(&mut self.world, turn.ledger_mut(), &receipt)
            .map_err(V2EnvironmentError)
            .map_err(EnvironmentActionError::Environment)?;
        turn.adopt_recorded(result)
            .map_err(EnvironmentActionError::Ledger)
    }
}

#[derive(Clone, Copy)]
enum ToyContext {
    North,
    South,
    East,
    West,
}

#[derive(Clone, Copy)]
enum ToyMarker {
    Dormant,
    Armed,
    Lit,
    Distracted,
}

#[derive(Clone, Copy)]
enum ToyStage {
    Initial,
    Armed,
    Failed,
}

#[derive(Clone, Copy)]
struct ToyEpisodeSpec {
    context: ToyContext,
    marker: ToyMarker,
}

// This schedule is intentionally unrelated to the v2 C0..C3/M0..M1 world:
// it contains two structural supports, a non-applicable challenge, and held
// out context combinations that use a two-action prime/commit transition.
const TOY_SCHEDULE: [ToyEpisodeSpec; 7] = [
    ToyEpisodeSpec {
        context: ToyContext::North,
        marker: ToyMarker::Dormant,
    },
    ToyEpisodeSpec {
        context: ToyContext::South,
        marker: ToyMarker::Dormant,
    },
    ToyEpisodeSpec {
        context: ToyContext::East,
        marker: ToyMarker::Lit,
    },
    ToyEpisodeSpec {
        context: ToyContext::West,
        marker: ToyMarker::Dormant,
    },
    ToyEpisodeSpec {
        context: ToyContext::East,
        marker: ToyMarker::Dormant,
    },
    ToyEpisodeSpec {
        context: ToyContext::North,
        marker: ToyMarker::Dormant,
    },
    ToyEpisodeSpec {
        context: ToyContext::South,
        marker: ToyMarker::Dormant,
    },
];

const PRIME_ACTION: ActionId = ActionId::new(17);
const COMMIT_ACTION: ActionId = ActionId::new(29);
const TOY_SCOPE_SEED: u64 = 0xA6_17_20_26_10_11_31;

#[derive(Clone, Copy)]
struct ActiveToyEpisode {
    scope: ScopeToken,
    context: ToyContext,
    marker: ToyMarker,
    stage: ToyStage,
    action_count: u8,
}

/// Adapter B: a deterministic structural toy world with a different opaque
/// vocabulary, schedule, and transition family than Adapter A.
///
/// Its successful transition is a two-step prime/commit relation. The first
/// step preserves the repeated context role while changing the middle marker;
/// the second step commits only from the armed state. The held-out contexts
/// therefore cannot be solved by replaying one exact packet.
pub struct StructuralToyEnvironment {
    run_scope: ScopeToken,
    episode_scopes: [ScopeToken; TOY_SCHEDULE.len()],
    next_episode: usize,
    active: Option<ActiveToyEpisode>,
}

impl StructuralToyEnvironment {
    pub fn new() -> Self {
        let (run_scope, episode_scopes) = toy_scopes();
        Self {
            run_scope,
            episode_scopes,
            next_episode: 0,
            active: None,
        }
    }

    pub const fn run_scope(&self) -> ScopeToken {
        self.run_scope
    }
}

impl Default for StructuralToyEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum StructuralToyError {
    Protocol(ProtocolError),
    NoEpisodeAvailable,
    EpisodeAlreadyActive,
    NoActiveEpisode,
    InvalidAction(ActionId),
    ActionBudgetExhausted,
    ReceiptRunScopeMismatch,
}

impl fmt::Display for StructuralToyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => error.fmt(formatter),
            Self::NoEpisodeAvailable => {
                formatter.write_str("the structural toy schedule is exhausted")
            }
            Self::EpisodeAlreadyActive => {
                formatter.write_str("the structural toy episode is active")
            }
            Self::NoActiveEpisode => {
                formatter.write_str("there is no active structural toy episode")
            }
            Self::InvalidAction(action) => write!(formatter, "invalid opaque action {action}"),
            Self::ActionBudgetExhausted => {
                formatter.write_str("the structural toy action budget is exhausted")
            }
            Self::ReceiptRunScopeMismatch => {
                formatter.write_str("the receipt belongs to another run")
            }
        }
    }
}

impl std::error::Error for StructuralToyError {}

impl From<ProtocolError> for StructuralToyError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl Environment for StructuralToyEnvironment {
    type Error = StructuralToyError;

    fn reset(&mut self, ledger: &InteractionLedger) -> Result<StepInput, Self::Error> {
        if ledger.run_scope() != self.run_scope {
            return Err(StructuralToyError::ReceiptRunScopeMismatch);
        }
        if self.active.is_some() {
            return Err(StructuralToyError::EpisodeAlreadyActive);
        }
        if self.next_episode >= TOY_SCHEDULE.len() {
            return Err(StructuralToyError::NoEpisodeAvailable);
        }
        let index = self.next_episode;
        let spec = TOY_SCHEDULE
            .get(index)
            .copied()
            .ok_or(StructuralToyError::NoEpisodeAvailable)?;
        self.next_episode += 1;
        let active = ActiveToyEpisode {
            scope: self.episode_scopes[index],
            context: spec.context,
            marker: spec.marker,
            stage: ToyStage::Initial,
            action_count: 0,
        };
        self.active = Some(active);
        StepInput::new(
            self.run_scope,
            active.scope,
            0,
            packet(active.context, active.marker),
            [PRIME_ACTION, COMMIT_ACTION],
            2,
            PreviousTerminal::None,
        )
        .map_err(StructuralToyError::from)
    }

    fn apply(
        &mut self,
        turn: &mut EnvironmentTurn<'_>,
    ) -> Result<(), EnvironmentActionError<Self::Error>> {
        turn.resolve(|action| self.apply_action(action))?;
        Ok(())
    }
}

impl StructuralToyEnvironment {
    fn apply_action(&mut self, action: ActionId) -> Result<StepResult, StructuralToyError> {
        let mut active = self.active.ok_or(StructuralToyError::NoActiveEpisode)?;
        if active.action_count >= 2 {
            return Err(StructuralToyError::ActionBudgetExhausted);
        }
        if action != PRIME_ACTION && action != COMMIT_ACTION {
            return Err(StructuralToyError::InvalidAction(action));
        }

        let first_action = active.action_count == 0;
        let success = matches!(active.stage, ToyStage::Armed) && action == COMMIT_ACTION;
        let prime_transition = first_action
            && matches!(active.stage, ToyStage::Initial)
            && matches!(active.marker, ToyMarker::Dormant)
            && action == PRIME_ACTION;

        if prime_transition {
            active.stage = ToyStage::Armed;
            active.marker = ToyMarker::Armed;
        } else if success {
            active.stage = ToyStage::Failed;
            active.marker = ToyMarker::Lit;
        } else if !first_action {
            active.stage = ToyStage::Failed;
            active.marker = ToyMarker::Distracted;
        }

        active.action_count += 1;
        let terminal = if success {
            TerminalStatus::Success
        } else if active.action_count == 2 {
            TerminalStatus::Failure
        } else {
            TerminalStatus::Continue
        };
        let result = StepResult::new(packet(active.context, active.marker), terminal)?;
        if terminal == TerminalStatus::Continue {
            self.active = Some(active);
        } else {
            self.active = None;
        }
        Ok(result)
    }
}

fn packet(context: ToyContext, marker: ToyMarker) -> crate::ObservationPacket {
    crate::ObservationPacket::new([
        wire_context(context),
        wire_marker(marker),
        wire_context(context),
    ])
}

fn wire_context(context: ToyContext) -> AtomId {
    match context {
        ToyContext::North => AtomId::new(101),
        ToyContext::South => AtomId::new(113),
        ToyContext::East => AtomId::new(127),
        ToyContext::West => AtomId::new(139),
    }
}

fn wire_marker(marker: ToyMarker) -> AtomId {
    match marker {
        ToyMarker::Dormant => AtomId::new(701),
        ToyMarker::Armed => AtomId::new(709),
        ToyMarker::Lit => AtomId::new(719),
        ToyMarker::Distracted => AtomId::new(727),
    }
}

fn toy_scopes() -> (ScopeToken, [ScopeToken; TOY_SCHEDULE.len()]) {
    let mut state = TOY_SCOPE_SEED;
    let run_scope = next_scope(&mut state);
    let episode_scopes = std::array::from_fn(|_| next_scope(&mut state));
    (run_scope, episode_scopes)
}

fn next_scope(state: &mut u64) -> ScopeToken {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ScopeToken::new(*state | 1)
}
