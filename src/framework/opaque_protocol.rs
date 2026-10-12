//! Reusable learner-facing opaque wire contract.
//!
//! This module deliberately contains only opaque values, transport metadata,
//! and validation. It preserves the fixed v2 packet semantics for compatibility
//! but has no hidden-world imports or lookups.

use std::fmt;

pub const OBSERVATION_PACKET_LEN: usize = 3;
pub const AVAILABLE_ACTION_COUNT: usize = 2;

macro_rules! opaque_identifier {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u64);

        impl $name {
            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            pub const fn value(self) -> u64 {
                self.0
            }

            pub const fn is_zero(self) -> bool {
                self.0 == 0
            }
        }

        impl From<u64> for $name {
            fn from(value: u64) -> Self {
                Self::new(value)
            }
        }

        impl From<$name> for u64 {
            fn from(value: $name) -> Self {
                value.value()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, formatter)
            }
        }
    };
}

opaque_identifier!(AtomId);
opaque_identifier!(ActionId);
opaque_identifier!(ScopeToken);
opaque_identifier!(ProcedureId);
opaque_identifier!(EvidenceId);
opaque_identifier!(HypothesisId);

/// A packet is always three ordered opaque atoms. Other slice lengths are
/// rejected instead of truncated or padded.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObservationPacket {
    atoms: [AtomId; OBSERVATION_PACKET_LEN],
}

impl ObservationPacket {
    pub const fn new(atoms: [AtomId; OBSERVATION_PACKET_LEN]) -> Self {
        Self { atoms }
    }

    pub fn try_from_slice(atoms: &[AtomId]) -> Result<Self, ProtocolError> {
        if atoms.len() != OBSERVATION_PACKET_LEN {
            return Err(ProtocolError::InvalidPacketLength {
                expected: OBSERVATION_PACKET_LEN,
                actual: atoms.len(),
            });
        }

        Ok(Self::new([atoms[0], atoms[1], atoms[2]]))
    }

    pub const fn atoms(self) -> [AtomId; OBSERVATION_PACKET_LEN] {
        self.atoms
    }

    pub const fn atom(self, index: usize) -> Option<AtomId> {
        if index < OBSERVATION_PACKET_LEN {
            Some(self.atoms[index])
        } else {
            None
        }
    }
}

impl TryFrom<&[AtomId]> for ObservationPacket {
    type Error = ProtocolError;

    fn try_from(atoms: &[AtomId]) -> Result<Self, Self::Error> {
        Self::try_from_slice(atoms)
    }
}

/// All terminal values used by the protocol, including the local terminal
/// status used to close an abstained episode.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TerminalStatus {
    Continue,
    Success,
    Failure,
    Incomplete,
}

impl TerminalStatus {
    pub const fn is_terminal(self) -> bool {
        !matches!(self, Self::Continue)
    }

    pub const fn is_environment_result(self) -> bool {
        matches!(self, Self::Continue | Self::Success | Self::Failure)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PreviousTerminal {
    None,
    Continue,
}

/// The environment can never return `Incomplete` as a post-action result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StepResult {
    observation: ObservationPacket,
    terminal: TerminalStatus,
}

impl StepResult {
    pub fn new(
        observation: ObservationPacket,
        terminal: TerminalStatus,
    ) -> Result<Self, ProtocolError> {
        if !terminal.is_environment_result() {
            return Err(ProtocolError::InvalidStepResultTerminal(terminal));
        }
        Ok(Self {
            observation,
            terminal,
        })
    }

    pub const fn observation(self) -> ObservationPacket {
        self.observation
    }

    pub const fn terminal(self) -> TerminalStatus {
        self.terminal
    }
}

/// A validated learner input.  Construction enforces the fixed two-slot
/// action/budget schedule; a ledger additionally validates episode sequencing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StepInput {
    run_scope: ScopeToken,
    episode_scope: ScopeToken,
    step_index: u8,
    observation: ObservationPacket,
    available_actions: [ActionId; AVAILABLE_ACTION_COUNT],
    remaining_action_budget: u8,
    previous_terminal: PreviousTerminal,
}

impl StepInput {
    pub fn new(
        run_scope: ScopeToken,
        episode_scope: ScopeToken,
        step_index: u8,
        observation: ObservationPacket,
        available_actions: [ActionId; AVAILABLE_ACTION_COUNT],
        remaining_action_budget: u8,
        previous_terminal: PreviousTerminal,
    ) -> Result<Self, ProtocolError> {
        if run_scope.is_zero() {
            return Err(ProtocolError::ZeroScopeToken);
        }
        if episode_scope.is_zero() {
            return Err(ProtocolError::ZeroScopeToken);
        }
        if run_scope == episode_scope {
            return Err(ProtocolError::ScopesMustDiffer);
        }
        if available_actions[0] == available_actions[1] {
            return Err(ProtocolError::DuplicateAvailableAction);
        }
        if available_actions[0] > available_actions[1] {
            return Err(ProtocolError::ActionsNotInCanonicalOrder);
        }

        match (step_index, remaining_action_budget, previous_terminal) {
            (0, 2, PreviousTerminal::None) | (1, 1, PreviousTerminal::Continue) => Ok(Self {
                run_scope,
                episode_scope,
                step_index,
                observation,
                available_actions,
                remaining_action_budget,
                previous_terminal,
            }),
            (step, _, _) if step > 1 => Err(ProtocolError::InvalidStepIndex(step)),
            (step, budget, previous) => Err(ProtocolError::InvalidStepMetadata {
                step_index: step,
                remaining_action_budget: budget,
                previous_terminal: previous,
            }),
        }
    }

    pub const fn run_scope(self) -> ScopeToken {
        self.run_scope
    }

    pub const fn episode_scope(self) -> ScopeToken {
        self.episode_scope
    }

    pub const fn step_index(self) -> u8 {
        self.step_index
    }

    pub const fn observation(self) -> ObservationPacket {
        self.observation
    }

    pub const fn available_actions(self) -> [ActionId; AVAILABLE_ACTION_COUNT] {
        self.available_actions
    }

    pub const fn remaining_action_budget(self) -> u8 {
        self.remaining_action_budget
    }

    pub const fn previous_terminal(self) -> PreviousTerminal {
        self.previous_terminal
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActMode {
    Exploration,
    Procedure,
    Fallback,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AbstainReason {
    NoApplicableProcedure,
    Conflict,
    InsufficientEvidence,
    UnsupportedByGrammar,
    IncompleteBudget,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AuditReferenceKind {
    Procedure,
    Evidence,
    Hypothesis,
}

/// References are copied into immutable decision records.  Repeated IDs in a
/// single reference list are rejected so the trace cannot hide duplicate work.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditReferences {
    procedure_refs: Vec<ProcedureId>,
    evidence_refs: Vec<EvidenceId>,
    hypothesis_refs: Vec<HypothesisId>,
}

pub type AuditRefs = AuditReferences;

impl AuditReferences {
    pub fn new(
        mut procedure_refs: Vec<ProcedureId>,
        mut evidence_refs: Vec<EvidenceId>,
        mut hypothesis_refs: Vec<HypothesisId>,
    ) -> Result<Self, ProtocolError> {
        procedure_refs.sort_unstable();
        evidence_refs.sort_unstable();
        hypothesis_refs.sort_unstable();
        ensure_unique(
            AuditReferenceKind::Procedure,
            procedure_refs.iter().map(|id| id.value()),
        )?;
        ensure_unique(
            AuditReferenceKind::Evidence,
            evidence_refs.iter().map(|id| id.value()),
        )?;
        ensure_unique(
            AuditReferenceKind::Hypothesis,
            hypothesis_refs.iter().map(|id| id.value()),
        )?;
        Ok(Self {
            procedure_refs,
            evidence_refs,
            hypothesis_refs,
        })
    }

    pub fn from_slices(
        procedure_refs: &[ProcedureId],
        evidence_refs: &[EvidenceId],
        hypothesis_refs: &[HypothesisId],
    ) -> Result<Self, ProtocolError> {
        Self::new(
            procedure_refs.to_vec(),
            evidence_refs.to_vec(),
            hypothesis_refs.to_vec(),
        )
    }

    pub fn empty() -> Self {
        Self {
            procedure_refs: Vec::new(),
            evidence_refs: Vec::new(),
            hypothesis_refs: Vec::new(),
        }
    }

    pub fn procedure_refs(&self) -> &[ProcedureId] {
        &self.procedure_refs
    }

    pub fn evidence_refs(&self) -> &[EvidenceId] {
        &self.evidence_refs
    }

    pub fn hypothesis_refs(&self) -> &[HypothesisId] {
        &self.hypothesis_refs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActDecision {
    action: ActionId,
    prediction: Option<ObservationPacket>,
    mode: ActMode,
    audit: AuditReferences,
}

impl ActDecision {
    pub fn new(
        action: ActionId,
        prediction: Option<ObservationPacket>,
        mode: ActMode,
        audit: AuditReferences,
    ) -> Result<Self, ProtocolError> {
        if matches!(mode, ActMode::Procedure) && audit.procedure_refs().is_empty() {
            return Err(ProtocolError::ProcedureReferenceRequired);
        }
        Ok(Self {
            action,
            prediction,
            mode,
            audit,
        })
    }

    pub const fn action(&self) -> ActionId {
        self.action
    }

    pub const fn prediction(&self) -> Option<ObservationPacket> {
        self.prediction
    }

    pub const fn mode(&self) -> ActMode {
        self.mode
    }

    pub fn audit(&self) -> &AuditReferences {
        &self.audit
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbstainDecision {
    reason: AbstainReason,
    audit: AuditReferences,
}

impl AbstainDecision {
    pub fn new(reason: AbstainReason, audit: AuditReferences) -> Self {
        Self { reason, audit }
    }

    pub const fn reason(&self) -> AbstainReason {
        self.reason
    }

    pub fn audit(&self) -> &AuditReferences {
        &self.audit
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Decision {
    Act(ActDecision),
    Abstain(AbstainDecision),
}

impl Decision {
    pub fn act(
        action: ActionId,
        prediction: Option<ObservationPacket>,
        mode: ActMode,
        audit: AuditReferences,
    ) -> Result<Self, ProtocolError> {
        Ok(Self::Act(ActDecision::new(
            action, prediction, mode, audit,
        )?))
    }

    pub fn abstain(reason: AbstainReason, audit: AuditReferences) -> Self {
        Self::Abstain(AbstainDecision::new(reason, audit))
    }

    pub fn validate_for(&self, input: &StepInput) -> Result<(), ProtocolError> {
        match self {
            Self::Act(decision) => {
                if !input.available_actions.contains(&decision.action) {
                    return Err(ProtocolError::ActionUnavailable(decision.action));
                }
                Ok(())
            }
            Self::Abstain(_) => Ok(()),
        }
    }

    pub const fn is_abstain(&self) -> bool {
        matches!(self, Self::Abstain(_))
    }

    pub const fn action(&self) -> Option<ActionId> {
        match self {
            Self::Act(decision) => Some(decision.action),
            Self::Abstain(_) => None,
        }
    }

    pub const fn prediction(&self) -> Option<ObservationPacket> {
        match self {
            Self::Act(decision) => decision.prediction,
            Self::Abstain(_) => None,
        }
    }

    pub fn audit(&self) -> &AuditReferences {
        match self {
            Self::Act(decision) => &decision.audit,
            Self::Abstain(decision) => &decision.audit,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    InvalidPacketLength {
        expected: usize,
        actual: usize,
    },
    UnavailableEvidence,
    InvalidStepResultTerminal(TerminalStatus),
    ZeroScopeToken,
    ScopesMustDiffer,
    DuplicateAvailableAction,
    ActionsNotInCanonicalOrder,
    InvalidStepIndex(u8),
    InvalidStepMetadata {
        step_index: u8,
        remaining_action_budget: u8,
        previous_terminal: PreviousTerminal,
    },
    ProcedurePredictionRequired,
    ProcedureReferenceRequired,
    ActionUnavailable(ActionId),
    DuplicateAuditReference {
        kind: AuditReferenceKind,
        value: u64,
    },
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPacketLength { expected, actual } => {
                write!(formatter, "expected packet length {expected}, got {actual}")
            }
            Self::UnavailableEvidence => {
                write!(formatter, "unavailable evidence cannot complete an observation packet")
            }
            Self::InvalidStepResultTerminal(status) => {
                write!(formatter, "{status:?} cannot be returned as a StepResult")
            }
            Self::ZeroScopeToken => write!(formatter, "scope tokens must be nonzero"),
            Self::ScopesMustDiffer => write!(formatter, "run and episode scopes must differ"),
            Self::DuplicateAvailableAction => write!(formatter, "available actions must differ"),
            Self::ActionsNotInCanonicalOrder => {
                write!(formatter, "available actions are not in exact identifier order")
            }
            Self::InvalidStepIndex(index) => write!(formatter, "invalid step index {index}"),
            Self::InvalidStepMetadata {
                step_index,
                remaining_action_budget,
                previous_terminal,
            } => write!(
                formatter,
                "invalid metadata for step {step_index}: remaining budget {remaining_action_budget}, previous terminal {previous_terminal:?}"
            ),
            Self::ProcedurePredictionRequired => {
                write!(formatter, "procedure decisions require a packet prediction")
            }
            Self::ProcedureReferenceRequired => {
                write!(formatter, "procedure decisions require a procedure reference")
            }
            Self::ActionUnavailable(action) => write!(formatter, "action {action} is unavailable"),
            Self::DuplicateAuditReference { kind, value } => {
                write!(formatter, "duplicate {kind:?} audit reference {value}")
            }
        }
    }
}

impl std::error::Error for ProtocolError {}

fn ensure_unique<I>(kind: AuditReferenceKind, values: I) -> Result<(), ProtocolError>
where
    I: IntoIterator<Item = u64>,
{
    let mut seen = Vec::new();
    for value in values {
        if seen.contains(&value) {
            return Err(ProtocolError::DuplicateAuditReference { kind, value });
        }
        seen.push(value);
    }
    Ok(())
}
