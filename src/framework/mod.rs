//! Reusable learner framework primitives.
//!
//! This module is the environment-independent boundary for the validated
//! structural learner. It owns the opaque protocol, receipt-backed interaction
//! ledger, bounded grammar and admission policy, grounding policy, partial
//! observation seam, resource accounting, and learner-owned MVP-3 stores. None
//! of these modules know about evaluator worlds, hidden labels, benchmark seeds,
//! schedules, experiment runners, or scoring.
//!
//! [`crate::v2`] remains a compatibility and benchmark namespace. Its learner
//! module paths re-export these implementations, while its evaluator adapter,
//! deterministic world, frozen schedule, and experiment runner stay outside
//! this reusable boundary.

pub mod controller;
pub mod grammar_admission;
pub mod grounding_decision;
pub mod interaction_ledger;
pub mod opaque_protocol;
pub mod partial_observation;
pub mod persistence;
pub mod resource_scale;
pub mod stores;

pub use controller::*;
pub use grammar_admission::*;
pub use grounding_decision::*;
pub use interaction_ledger::*;
pub use opaque_protocol::*;
pub use partial_observation::*;
pub use persistence::{
    FrameworkState, PersistenceError, StateFingerprint, PERSISTENCE_FORMAT,
    PERSISTENCE_SCHEMA_VERSION,
};
pub use resource_scale::*;
pub use stores::{
    DependencyIndex, DependencySource, DependentRecord, DerivedId, EvidenceStore, HypothesisRecord,
    HypothesisStore, HypothesisTransition, LearnerStores, ProcedureStore, ProcedureVersion,
    StoreEntity, StoreError, StoredHypothesisStatus, StoredProcedure,
};

/// The learner-side mutable lifecycle boundary.
///
/// A mutable learner decides from an opaque input, then ingests the matching
/// observed result only after the environment has recorded the decision in an
/// [`InteractionLedger`]. Implementations may use the grammar and grounding
/// seams above, but do not receive evaluator-only state through this trait.
pub trait Learner {
    type Frozen: FrozenLearner;

    fn step(&mut self, input: StepInput) -> Result<Decision, ProtocolError>;
    fn observe(&mut self, result: StepResult) -> Result<(), ProtocolError>;
    fn freeze(self) -> Self::Frozen;
}

/// The frozen learner boundary is deliberately non-mutating.
pub trait FrozenLearner {
    fn step(&self, input: StepInput) -> Result<Decision, ProtocolError>;
}

/// The result of a ledger-aware learner decision.
///
/// The compatibility [`Learner`] trait returns an unrecorded decision because
/// the MVP-5 controller owns the call to `InteractionLedger::record_decision`.
/// A structural learner such as [`GroundingLearner`] already needs the ledger
/// while selecting and records its receipt as part of that operation. This
/// enum makes that ownership explicit so a controller can adopt the existing
/// receipt without silently recording a second decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LedgerDecision {
    Unrecorded(Decision),
    Recorded(DecisionReceipt),
}

/// A failed consuming freeze returns the mutable learner so a controller can
/// restore it without losing the caller's state. The ledger is deliberately
/// read-only during the hook; the controller owns the final ledger commit.
#[derive(Debug)]
pub struct LearnerFreezeFailure<L, E> {
    learner: L,
    error: E,
}

impl<L, E> LearnerFreezeFailure<L, E> {
    pub fn new(learner: L, error: E) -> Self {
        Self { learner, error }
    }

    pub fn into_parts(self) -> (L, E) {
        (self.learner, self.error)
    }
}

impl LedgerDecision {
    pub fn decision(&self) -> &Decision {
        match self {
            Self::Unrecorded(decision) => decision,
            Self::Recorded(receipt) => receipt.decision(),
        }
    }
}

/// A learner lifecycle that is allowed to participate in receipt-backed
/// selection and observation.
///
/// This is an additive extension to [`Learner`]. The blanket implementation
/// below preserves the MVP-5 API: ordinary learners still return an unrecorded
/// decision and receive only an observed result. Ledger-aware learners can
/// instead validate references, record a receipt, and ingest an abstention
/// closure using the same generic controller.
pub trait LedgerLearner {
    type Frozen: LedgerFrozenLearner<Error = Self::Error>;
    type Error: std::error::Error + 'static;

    fn step_with_ledger(
        &mut self,
        ledger: &mut InteractionLedger,
        input: StepInput,
    ) -> Result<LedgerDecision, Self::Error>;

    fn observe_with_ledger(
        &mut self,
        ledger: &mut InteractionLedger,
        result: Option<StepResult>,
    ) -> Result<(), Self::Error>;

    fn enter_development(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn freeze_with_ledger(
        self,
        ledger: &InteractionLedger,
    ) -> Result<Self::Frozen, LearnerFreezeFailure<Self, Self::Error>>
    where
        Self: Sized;
}

/// The frozen half of [`LedgerLearner`].
pub trait LedgerFrozenLearner {
    type Error: std::error::Error + 'static;

    fn step_with_ledger(
        &self,
        ledger: &mut InteractionLedger,
        input: StepInput,
    ) -> Result<LedgerDecision, Self::Error>;
}

impl<T> LedgerLearner for T
where
    T: Learner,
{
    type Error = ProtocolError;
    type Frozen = T::Frozen;

    fn step_with_ledger(
        &mut self,
        _ledger: &mut InteractionLedger,
        input: StepInput,
    ) -> Result<LedgerDecision, Self::Error> {
        self.step(input).map(LedgerDecision::Unrecorded)
    }

    fn observe_with_ledger(
        &mut self,
        _ledger: &mut InteractionLedger,
        result: Option<StepResult>,
    ) -> Result<(), Self::Error> {
        if let Some(result) = result {
            self.observe(result)?;
        }
        Ok(())
    }

    fn freeze_with_ledger(
        self,
        _ledger: &InteractionLedger,
    ) -> Result<Self::Frozen, LearnerFreezeFailure<Self, Self::Error>> {
        Ok(self.freeze())
    }
}

impl<T> LedgerFrozenLearner for T
where
    T: FrozenLearner,
{
    type Error = ProtocolError;

    fn step_with_ledger(
        &self,
        _ledger: &mut InteractionLedger,
        input: StepInput,
    ) -> Result<LedgerDecision, Self::Error> {
        self.step(input).map(LedgerDecision::Unrecorded)
    }
}

/// Minimal learner-owned run state for compatibility with the existing v2
/// world. It contains no labels, transitions, schedules, or evaluator seeds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearnerState {
    run_scope: ScopeToken,
    decision_count: u64,
    frozen: bool,
}

impl LearnerState {
    pub(crate) fn from_persisted(
        run_scope: ScopeToken,
        decision_count: u64,
        frozen: bool,
    ) -> Result<Self, ProtocolError> {
        if run_scope.is_zero() {
            return Err(ProtocolError::ZeroScopeToken);
        }
        Ok(Self {
            run_scope,
            decision_count,
            frozen,
        })
    }

    pub fn new(run_scope: ScopeToken) -> Result<Self, ProtocolError> {
        if run_scope.is_zero() {
            return Err(ProtocolError::ZeroScopeToken);
        }
        Ok(Self {
            run_scope,
            decision_count: 0,
            frozen: false,
        })
    }

    pub const fn run_scope(&self) -> ScopeToken {
        self.run_scope
    }

    pub const fn decision_count(&self) -> u64 {
        self.decision_count
    }

    pub const fn is_frozen(&self) -> bool {
        self.frozen
    }

    pub fn note_decision(&mut self) {
        if !self.frozen {
            self.decision_count = self.decision_count.saturating_add(1);
        }
    }

    pub fn freeze(&mut self) {
        self.frozen = true;
    }
}
