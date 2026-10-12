use super::protocol::{AnalysisDecision, BudgetError, StreamEvent};
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LearnerError {
    Budget(BudgetError),
    Protocol(String),
    InvalidScope,
    Internal(String),
}

impl fmt::Display for LearnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Budget(error) => write!(formatter, "budget exhausted: {error}"),
            Self::Protocol(detail) => write!(formatter, "protocol violation: {detail}"),
            Self::InvalidScope => write!(formatter, "event scope does not match active stream"),
            Self::Internal(detail) => write!(formatter, "learner failure: {detail}"),
        }
    }
}

impl std::error::Error for LearnerError {}

pub trait ExperimentALearner {
    type Frozen: FrozenExperimentALearner;

    fn ingest(&mut self, event: StreamEvent) -> Result<(), LearnerError>;
    fn analyze(&mut self) -> Result<AnalysisDecision, LearnerError>;
    fn begin_development(&mut self) -> Result<(), LearnerError>;
    fn freeze(self) -> Result<Self::Frozen, LearnerError>;
}

pub trait FrozenExperimentALearner {
    fn ingest(&mut self, event: StreamEvent) -> Result<(), LearnerError>;
    fn analyze(&mut self) -> Result<AnalysisDecision, LearnerError>;
    fn model_fingerprint(&self) -> &str;
}
