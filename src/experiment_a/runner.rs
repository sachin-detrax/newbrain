use super::generator::GeneratedRecord;
use super::learner::{ExperimentALearner, FrozenExperimentALearner, LearnerError};
use super::protocol::{AnalysisDecision, Budget, BudgetError, ResourceKind, StreamEvent};
use std::fmt;
use std::time::Instant;

pub const MUTABLE_RECORDS: usize = 96;
pub const TRAINING_RECORDS: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunnerError {
    Learner(LearnerError),
    Budget(BudgetError),
    PhaseOrder(String),
}

impl fmt::Display for RunnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Learner(error) => write!(formatter, "learner failed: {error}"),
            Self::Budget(error) => write!(formatter, "runner budget exhausted: {error}"),
            Self::PhaseOrder(detail) => write!(formatter, "phase order violated: {detail}"),
        }
    }
}

impl std::error::Error for RunnerError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamOutcome {
    pub global_ordinal: u64,
    pub stream_scope: u64,
    pub wire_len: usize,
    pub decision: AnalysisDecision,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunTrace {
    pub outcomes: Vec<StreamOutcome>,
    pub development_called_after: usize,
    pub frozen_after: usize,
    pub fingerprint_before_final: String,
    pub fingerprint_after_final: String,
    pub received_atoms_run: u64,
    pub analysis_operation_cost: u64,
    pub wall_clock_ns: u128,
}

fn feed_record<L: ExperimentALearner>(
    record: &GeneratedRecord,
    learner: &mut L,
    budget: &mut Budget,
) -> Result<AnalysisDecision, RunnerError> {
    budget.begin_stream();
    for atom in &record.wire_tokens {
        budget
            .charge(ResourceKind::ReceivedAtoms, 1)
            .map_err(RunnerError::Budget)?;
        learner
            .ingest(StreamEvent::Token {
                run_scope: record.run_scope,
                stream_scope: record.stream_scope,
                atom: *atom,
            })
            .map_err(RunnerError::Learner)?;
    }
    learner
        .ingest(StreamEvent::End {
            run_scope: record.run_scope,
            stream_scope: record.stream_scope,
        })
        .map_err(RunnerError::Learner)?;
    learner.analyze().map_err(RunnerError::Learner)
}

fn feed_record_frozen<L: FrozenExperimentALearner>(
    record: &GeneratedRecord,
    learner: &mut L,
    budget: &mut Budget,
) -> Result<AnalysisDecision, RunnerError> {
    budget.begin_stream();
    for atom in &record.wire_tokens {
        budget
            .charge(ResourceKind::ReceivedAtoms, 1)
            .map_err(RunnerError::Budget)?;
        learner
            .ingest(StreamEvent::Token {
                run_scope: record.run_scope,
                stream_scope: record.stream_scope,
                atom: *atom,
            })
            .map_err(RunnerError::Learner)?;
    }
    learner
        .ingest(StreamEvent::End {
            run_scope: record.run_scope,
            stream_scope: record.stream_scope,
        })
        .map_err(RunnerError::Learner)?;
    learner.analyze().map_err(RunnerError::Learner)
}

pub fn drive_learner<L: ExperimentALearner>(
    records: &[GeneratedRecord],
    mut learner: L,
) -> Result<(RunTrace, L::Frozen), RunnerError> {
    if records.len() != 160 {
        return Err(RunnerError::PhaseOrder(format!(
            "expected 160 records, got {}",
            records.len()
        )));
    }
    let started = Instant::now();
    let mut budget = Budget::default();
    let mut outcomes = Vec::with_capacity(160);
    for record in &records[..TRAINING_RECORDS] {
        let decision = feed_record(record, &mut learner, &mut budget)?;
        outcomes.push(StreamOutcome {
            global_ordinal: record.global_ordinal,
            stream_scope: record.stream_scope,
            wire_len: record.wire_tokens.len(),
            decision,
        });
    }
    learner.begin_development().map_err(RunnerError::Learner)?;
    let development_called_after = outcomes.len();
    for record in &records[TRAINING_RECORDS..MUTABLE_RECORDS] {
        let decision = feed_record(record, &mut learner, &mut budget)?;
        outcomes.push(StreamOutcome {
            global_ordinal: record.global_ordinal,
            stream_scope: record.stream_scope,
            wire_len: record.wire_tokens.len(),
            decision,
        });
    }
    let mut frozen = learner.freeze().map_err(RunnerError::Learner)?;
    let frozen_after = outcomes.len();
    let fingerprint_before_final = frozen.model_fingerprint().to_owned();
    for record in &records[MUTABLE_RECORDS..] {
        let decision = feed_record_frozen(record, &mut frozen, &mut budget)?;
        outcomes.push(StreamOutcome {
            global_ordinal: record.global_ordinal,
            stream_scope: record.stream_scope,
            wire_len: record.wire_tokens.len(),
            decision,
        });
    }
    let fingerprint_after_final = frozen.model_fingerprint().to_owned();
    let trace = RunTrace {
        analysis_operation_cost: outcomes.iter().map(|o| o.decision.operation_cost).sum(),
        received_atoms_run: budget.run_value(ResourceKind::ReceivedAtoms),
        outcomes,
        development_called_after,
        frozen_after,
        fingerprint_before_final,
        fingerprint_after_final,
        wall_clock_ns: started.elapsed().as_nanos(),
    };
    Ok((trace, frozen))
}
