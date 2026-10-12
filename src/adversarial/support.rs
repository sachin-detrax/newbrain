use super::ExtensionResult;
use crate::dataset::V1Dataset;
use crate::engine::{Learner, LearnerConfig, LearnerSnapshot, TrainingError};
use crate::types::{Query, QueryId, Response, Sequence, Transition};

pub(super) struct Fixture {
    pub(super) training: Vec<Transition>,
    pub(super) development: Vec<Transition>,
    pub(super) query: Query,
    pub(super) expected: Sequence,
}

pub(super) fn fixture() -> Fixture {
    let data = V1Dataset::fixed();
    Fixture {
        training: data.r1_training().as_slice().to_vec(),
        development: data.r1_development().as_slice().to_vec(),
        query: Query {
            input: Sequence::new([3, 8, 3]),
            action: 0,
            scope: 0,
            query_id: QueryId(100),
        },
        expected: Sequence::new([3, 9, 3]),
    }
}

pub(super) fn train(
    config: LearnerConfig,
    training: &[Transition],
    development: &[Transition],
) -> Result<LearnerSnapshot, TrainingError> {
    let mut learner = Learner::new(config);
    learner.train_phase_a(training, development)?;
    Ok(learner.freeze())
}

pub(super) fn agrees(
    snapshot: &LearnerSnapshot,
    query: &Query,
    expected: Sequence,
) -> (bool, String) {
    let execution = snapshot.predict(query);
    let passed = matches!(&execution.response, Response::Prediction(_))
        && execution.response.output() == Some(expected);
    (passed, format!("{:?}", execution.response))
}

pub(super) fn result(
    name: &'static str,
    property: &'static str,
    passed: bool,
    detail: String,
) -> ExtensionResult {
    ExtensionResult {
        name,
        property,
        passed,
        detail,
    }
}

pub(super) fn transition(
    id: u16,
    input: [u8; 3],
    action: u8,
    output: [u8; 3],
    scope: u8,
) -> Transition {
    Transition {
        input: Sequence::new(input),
        action,
        output: Sequence::new(output),
        scope,
        record_id: crate::types::RecordId(id),
    }
}

pub(super) fn map_sequence(sequence: Sequence, map: fn(u8) -> u8) -> Sequence {
    Sequence::new(sequence.positions().map(map))
}

pub(super) fn map_transition(record: &Transition, map: fn(u8) -> u8) -> Transition {
    Transition {
        input: map_sequence(record.input, map),
        action: record.action,
        output: map_sequence(record.output, map),
        scope: record.scope,
        record_id: record.record_id,
    }
}

pub(super) fn map_query(query: Query, map: fn(u8) -> u8) -> Query {
    Query {
        input: map_sequence(query.input, map),
        ..query
    }
}

pub(super) fn shift_symbol(symbol: u8) -> u8 {
    symbol.wrapping_add(40)
}
