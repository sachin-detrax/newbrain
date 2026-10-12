//! v2 compatibility and benchmark APIs.
//!
//! The reusable learner implementation now lives in [`crate::framework`]. The
//! modules below retain the historical `v2::...` paths as re-export shims so
//! existing callers and frozen tests keep compiling. [`evaluator_world`] is the
//! benchmark environment adapter, [`adapter`] isolates the legacy evaluator
//! bridge, and [`experiment`] remains the benchmark runner; hidden labels,
//! seeds, schedules, B2, and scoring do not cross into the framework.

pub mod adapter;
pub mod changed_rule;
pub mod evaluator_world;
pub mod experiment;

/// Compatibility path for the reusable bounded grammar and admission policy.
pub mod grammar_admission {
    pub use crate::framework::grammar_admission::*;
}

/// Compatibility path for the reusable grounding and decision policy.
pub mod grounding_decision {
    pub use crate::framework::grounding_decision::*;
}

/// Compatibility path for the reusable append-only interaction ledger.
pub mod interaction_ledger {
    pub use crate::framework::interaction_ledger::*;
}

/// Compatibility path for the reusable opaque protocol.
pub mod opaque_protocol {
    pub use crate::framework::opaque_protocol::*;
}

/// Compatibility path for the reusable partial-observation seam.
pub mod partial_observation {
    pub use crate::framework::partial_observation::*;
}

/// Compatibility path for the reusable resource accounting implementation.
pub mod resource_scale {
    pub use crate::framework::resource_scale::*;
}

pub use experiment::{
    compare_reruns, run_all, run_one, run_one_rerun, run_variant, write_experiment_artifacts,
    write_run_artifacts, ExperimentError, ExperimentReport, ExperimentRunner,
    FailureClassification, FailureRecord, GateResult, Metrics, Predictor, Rate, RerunComparison,
    RunReport, Variant,
};

pub use adapter::Evaluator;
pub use changed_rule::{
    ChangedRuleError, ChangedRuleRevision, ChangedRuleRevisionTrace, ProcedureVersionRef,
    RevisionEvent,
};
pub use grounding_decision::{
    assess_partial, ActiveHypothesis, ChangeReason, CompetingPrediction, DecisionReason,
    DecisionTrace, DerivedResult, DerivedStatus, DisagreementAction, FrozenGroundingLearner,
    GroundingError, GroundingLearner, HypothesisStatus, LogicalCost, MutablePhase,
    PartialAssessment, RecordedDecision,
};

pub use opaque_protocol::{
    AbstainDecision, AbstainReason, ActDecision, ActMode, ActionId, AtomId, AuditReferences,
    AuditRefs, Decision, EvidenceId, HypothesisId, ObservationPacket, PreviousTerminal,
    ProcedureId, ProtocolError, ScopeToken, StepInput, StepResult, TerminalStatus,
    AVAILABLE_ACTION_COUNT, OBSERVATION_PACKET_LEN,
};

pub use crate::framework::{FrozenLearner, Learner, LearnerState};
