pub mod admission;
pub mod adversarial;
pub mod baselines;
pub mod budget;
pub mod dataset;
pub mod engine;
pub mod evaluator;
/// Architecture Experiment A — variable-length boundary discovery (isolated namespace).
pub mod experiment_a;
/// Architecture Experiment B — transition model and goal-directed planning (isolated namespace).
pub mod experiment_b;
/// Reusable environment-independent learner primitives.
pub mod framework;
pub mod grammar;
pub mod ledger;
/// MVP-6 evaluator-side environment adapters.
pub mod mvp6;
pub mod report;
pub mod runtime;
pub mod types;
pub mod v2;

pub use framework::{
    FrozenLearner, Learner, LearnerFreezeFailure, LearnerState, LedgerDecision,
    LedgerFrozenLearner, LedgerLearner,
};
pub use runtime::{
    apply_environment, AbstainReason, ActMode, ActionId, AtomId, AuditReferences, BudgetFailure,
    ControllerError, ControllerState, Decision, DecisionReceipt, Environment,
    EnvironmentActionError, EnvironmentError, EnvironmentTurn, EpisodeBudget, EpisodeSummary,
    FailureClassification, FrameworkState, GenericRunController, InteractionLedger, LearnerStores,
    LedgerError, LedgerEvent, Lifecycle, LifecycleError, ObservationPacket, PersistenceError,
    PreviousTerminal, ProcedureStore, ProcedureVersion, ProtocolError, RunController, RunPhase,
    RunSummary, ScopeToken, StateFingerprint, StepInput, StepResult, StoreError,
    StoredHypothesisStatus, StoredProcedure, TerminalStatus, PERSISTENCE_FORMAT,
    PERSISTENCE_SCHEMA_VERSION,
};
/// Compatibility export for the benchmark-specific v2 evaluator bridge.
pub use v2::Evaluator;
