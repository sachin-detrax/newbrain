use std::collections::BTreeSet;
use std::slice::Iter;

use crate::types::{Query, QueryId, RecordId, Sequence, Transition};

const X0: u8 = 0;
const X1: u8 = 1;
const X2: u8 = 2;
const X3: u8 = 3;
const X4: u8 = 4;
const X5: u8 = 5;
const X6: u8 = 6;
const X7: u8 = 7;
const M0: u8 = 8;
const M1: u8 = 9;
const N0: u8 = 10;
const N1: u8 = 11;
const A0: u8 = 0;
const A1: u8 = 1;
const W1: u8 = 0;
const W2: u8 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluatorCase {
    pub query: Query,
    pub expected: Sequence,
    pub applicable: bool,
}

macro_rules! transition_partition {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name(Vec<Transition>);

        impl $name {
            pub fn len(&self) -> usize {
                self.0.len()
            }

            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }

            pub fn iter(&self) -> Iter<'_, Transition> {
                self.0.iter()
            }

            pub fn as_slice(&self) -> &[Transition] {
                &self.0
            }
        }

        impl AsRef<[Transition]> for $name {
            fn as_ref(&self) -> &[Transition] {
                self.as_slice()
            }
        }

        impl<'a> IntoIterator for &'a $name {
            type Item = &'a Transition;
            type IntoIter = Iter<'a, Transition>;

            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
    };
}

macro_rules! evaluator_partition {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name(Vec<EvaluatorCase>);

        impl $name {
            pub fn len(&self) -> usize {
                self.0.len()
            }

            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }

            pub fn iter(&self) -> Iter<'_, EvaluatorCase> {
                self.0.iter()
            }

            pub fn as_slice(&self) -> &[EvaluatorCase] {
                &self.0
            }
        }

        impl AsRef<[EvaluatorCase]> for $name {
            fn as_ref(&self) -> &[EvaluatorCase] {
                self.as_slice()
            }
        }

        impl<'a> IntoIterator for &'a $name {
            type Item = &'a EvaluatorCase;
            type IntoIter = Iter<'a, EvaluatorCase>;

            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
    };
}

transition_partition!(R1Training);
transition_partition!(R1Development);
evaluator_partition!(R1Final);
transition_partition!(R2Training);
transition_partition!(R2Development);
evaluator_partition!(R2Final);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct V1Dataset {
    r1_training: R1Training,
    r1_development: R1Development,
    r1_final: R1Final,
    r2_training: R2Training,
    r2_development: R2Development,
    r2_final: R2Final,
}

impl V1Dataset {
    pub fn fixed() -> Self {
        Self {
            r1_training: R1Training(vec![
                transition(1, [X0, M0, X0], A0, [X0, M1, X0], W1),
                transition(2, [X1, M0, X1], A0, [X1, M1, X1], W1),
            ]),
            r1_development: R1Development(vec![
                transition(3, [X2, M0, X2], A0, [X2, M1, X2], W1),
                transition(4, [X2, M0, X3], A0, [X2, M0, X3], W1),
                transition(5, [X2, M1, X2], A0, [X2, M1, X2], W1),
                transition(6, [X2, M0, X2], A1, [X2, M0, X2], W1),
            ]),
            r1_final: R1Final(vec![
                evaluator_case(7, [X3, M0, X3], A0, [X3, M1, X3], W1, true),
                evaluator_case(8, [X4, M0, X4], A0, [X4, M1, X4], W1, true),
                evaluator_case(9, [X5, M0, X5], A0, [X5, M1, X5], W1, true),
                evaluator_case(10, [X6, M0, X6], A0, [X6, M1, X6], W1, true),
                evaluator_case(11, [X7, M0, X7], A0, [X7, M1, X7], W1, true),
                evaluator_case(12, [X3, M0, X4], A0, [X3, M0, X4], W1, false),
                evaluator_case(13, [X4, M1, X4], A0, [X4, M1, X4], W1, false),
                evaluator_case(14, [X5, M0, X5], A1, [X5, M0, X5], W1, false),
            ]),
            r2_training: R2Training(vec![
                transition(15, [X0, N0, X0], A1, [X0, N1, X0], W2),
                transition(16, [X1, N0, X1], A1, [X1, N1, X1], W2),
            ]),
            r2_development: R2Development(vec![
                transition(17, [X2, N0, X2], A1, [X2, N1, X2], W2),
                transition(18, [X2, N0, X3], A1, [X2, N0, X3], W2),
                transition(19, [X2, N1, X2], A1, [X2, N1, X2], W2),
                transition(20, [X2, N0, X2], A0, [X2, N0, X2], W2),
            ]),
            r2_final: R2Final(vec![
                evaluator_case(21, [X2, N0, X2], A1, [X2, N1, X2], W2, true),
                evaluator_case(22, [X3, N0, X3], A1, [X3, N1, X3], W2, true),
                evaluator_case(23, [X4, N0, X5], A1, [X4, N0, X5], W2, false),
                evaluator_case(24, [X5, N1, X5], A1, [X5, N1, X5], W2, false),
            ]),
        }
    }

    pub fn r1_training(&self) -> &R1Training {
        &self.r1_training
    }

    pub fn r1_development(&self) -> &R1Development {
        &self.r1_development
    }

    pub fn r1_final_queries(&self) -> impl Iterator<Item = Query> + '_ {
        self.r1_final.iter().map(|case| case.query)
    }

    #[allow(dead_code)]
    pub(crate) fn r1_final_cases(&self) -> &R1Final {
        &self.r1_final
    }

    pub fn r2_training(&self) -> &R2Training {
        &self.r2_training
    }

    pub fn r2_development(&self) -> &R2Development {
        &self.r2_development
    }

    pub fn r2_final_queries(&self) -> impl Iterator<Item = Query> + '_ {
        self.r2_final.iter().map(|case| case.query)
    }

    #[allow(dead_code)]
    pub(crate) fn r2_final_cases(&self) -> &R2Final {
        &self.r2_final
    }

    pub fn all_training_and_development_ids(&self) -> BTreeSet<RecordId> {
        self.training_and_development_transitions()
            .map(|transition| transition.record_id)
            .collect()
    }

    pub fn learner_transitions(&self) -> impl Iterator<Item = &Transition> {
        self.training_and_development_transitions()
    }

    pub fn fingerprint(&self) -> u64 {
        let mut state = 14_695_981_039_346_656_037u64;
        hash_tag(&mut state, b"r1-training");
        for transition in self.r1_training.iter() {
            hash_transition(&mut state, transition);
        }
        hash_tag(&mut state, b"r1-development");
        for transition in self.r1_development.iter() {
            hash_transition(&mut state, transition);
        }
        hash_tag(&mut state, b"r1-final");
        for case in self.r1_final.iter() {
            hash_case(&mut state, case);
        }
        hash_tag(&mut state, b"r2-training");
        for transition in self.r2_training.iter() {
            hash_transition(&mut state, transition);
        }
        hash_tag(&mut state, b"r2-development");
        for transition in self.r2_development.iter() {
            hash_transition(&mut state, transition);
        }
        hash_tag(&mut state, b"r2-final");
        for case in self.r2_final.iter() {
            hash_case(&mut state, case);
        }
        state
    }

    fn training_and_development_transitions(&self) -> impl Iterator<Item = &Transition> {
        self.r1_training
            .iter()
            .chain(self.r1_development.iter())
            .chain(self.r2_training.iter())
            .chain(self.r2_development.iter())
    }
}

fn transition(
    record_id: u16,
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
        record_id: RecordId(record_id),
    }
}

fn evaluator_case(
    query_id: u16,
    input: [u8; 3],
    action: u8,
    expected: [u8; 3],
    scope: u8,
    applicable: bool,
) -> EvaluatorCase {
    EvaluatorCase {
        query: Query {
            input: Sequence::new(input),
            action,
            scope,
            query_id: QueryId(query_id),
        },
        expected: Sequence::new(expected),
        applicable,
    }
}

fn hash_tag(state: &mut u64, tag: &[u8]) {
    hash_bytes(state, tag);
    hash_byte(state, 0xff);
}

fn hash_transition(state: &mut u64, transition: &Transition) {
    hash_u16(state, transition.record_id.0);
    hash_sequence(state, transition.input);
    hash_byte(state, transition.action);
    hash_sequence(state, transition.output);
    hash_byte(state, transition.scope);
}

fn hash_case(state: &mut u64, case: &EvaluatorCase) {
    hash_u16(state, case.query.query_id.0);
    hash_sequence(state, case.query.input);
    hash_byte(state, case.query.action);
    hash_byte(state, case.query.scope);
    hash_sequence(state, case.expected);
    hash_byte(state, u8::from(case.applicable));
}

fn hash_sequence(state: &mut u64, sequence: Sequence) {
    for symbol in sequence.positions() {
        hash_byte(state, symbol);
    }
}

fn hash_u16(state: &mut u64, value: u16) {
    for byte in value.to_le_bytes() {
        hash_byte(state, byte);
    }
}

fn hash_bytes(state: &mut u64, bytes: &[u8]) {
    for &byte in bytes {
        hash_byte(state, byte);
    }
}

fn hash_byte(state: &mut u64, byte: u8) {
    *state ^= u64::from(byte);
    *state = state.wrapping_mul(1_099_511_628_211);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_cases_match_the_evaluator_partition() {
        let data = V1Dataset::fixed();
        assert_eq!(
            data.r1_final_cases()
                .iter()
                .map(|case| (case.query, case.expected, case.applicable))
                .collect::<Vec<_>>(),
            vec![
                (
                    evaluator_query(7, [X3, M0, X3], A0, W1),
                    Sequence::new([X3, M1, X3]),
                    true
                ),
                (
                    evaluator_query(8, [X4, M0, X4], A0, W1),
                    Sequence::new([X4, M1, X4]),
                    true
                ),
                (
                    evaluator_query(9, [X5, M0, X5], A0, W1),
                    Sequence::new([X5, M1, X5]),
                    true
                ),
                (
                    evaluator_query(10, [X6, M0, X6], A0, W1),
                    Sequence::new([X6, M1, X6]),
                    true
                ),
                (
                    evaluator_query(11, [X7, M0, X7], A0, W1),
                    Sequence::new([X7, M1, X7]),
                    true
                ),
                (
                    evaluator_query(12, [X3, M0, X4], A0, W1),
                    Sequence::new([X3, M0, X4]),
                    false
                ),
                (
                    evaluator_query(13, [X4, M1, X4], A0, W1),
                    Sequence::new([X4, M1, X4]),
                    false
                ),
                (
                    evaluator_query(14, [X5, M0, X5], A1, W1),
                    Sequence::new([X5, M0, X5]),
                    false
                ),
            ]
        );
        assert_eq!(
            data.r2_final_cases()
                .iter()
                .map(|case| (case.query, case.expected, case.applicable))
                .collect::<Vec<_>>(),
            vec![
                (
                    evaluator_query(21, [X2, N0, X2], A1, W2),
                    Sequence::new([X2, N1, X2]),
                    true
                ),
                (
                    evaluator_query(22, [X3, N0, X3], A1, W2),
                    Sequence::new([X3, N1, X3]),
                    true
                ),
                (
                    evaluator_query(23, [X4, N0, X5], A1, W2),
                    Sequence::new([X4, N0, X5]),
                    false
                ),
                (
                    evaluator_query(24, [X5, N1, X5], A1, W2),
                    Sequence::new([X5, N1, X5]),
                    false
                ),
            ]
        );
    }

    fn evaluator_query(query_id: u16, input: [u8; 3], action: u8, scope: u8) -> Query {
        Query {
            input: Sequence::new(input),
            action,
            scope,
            query_id: QueryId(query_id),
        }
    }
}
