use super::protocol::{BudgetError, EpisodeEvent, PlannerDecision};
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
            Self::InvalidScope => write!(formatter, "event scope does not match active episode"),
            Self::Internal(detail) => write!(formatter, "learner failure: {detail}"),
        }
    }
}

impl std::error::Error for LearnerError {}

pub trait ExperimentBLearner {
    type Frozen: FrozenExperimentBLearner;

    fn observe(&mut self, event: EpisodeEvent) -> Result<(), LearnerError>;
    fn plan(&mut self) -> Result<PlannerDecision, LearnerError>;
    fn ingest(&mut self, event: EpisodeEvent) -> Result<(), LearnerError>;
    fn begin_development(&mut self) -> Result<(), LearnerError>;
    fn freeze(self) -> Result<Self::Frozen, LearnerError>;
}

pub trait FrozenExperimentBLearner {
    fn observe(&mut self, event: EpisodeEvent) -> Result<(), LearnerError>;
    fn plan(&mut self) -> Result<PlannerDecision, LearnerError>;
    fn ingest(&mut self, event: EpisodeEvent) -> Result<(), LearnerError>;
    fn model_fingerprint(&self) -> &str;
}
