use super::learner::{ExperimentALearner, FrozenExperimentALearner, LearnerError};
use super::protocol::{
    sha256_hex, AnalysisDecision, AnalysisResponse, Parse, RelationUse, Span, StreamEvent,
    GRAMMAR_VERSION,
};

pub const A0_RELATION_VERSION: &str = "a0-whole-stream-equality-v1";
pub const A0_MODEL_VERSION: &str = "a0-fixed-position-equality-v1";

#[derive(Clone, Debug, Eq, PartialEq)]
struct SignatureEntry {
    length: usize,
    signature: Vec<(usize, usize)>,
}

fn equality_signature(tokens: &[u64]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    for i in 0..tokens.len() {
        for j in (i + 1)..tokens.len() {
            if tokens[i] == tokens[j] {
                pairs.push((i, j));
            }
        }
    }
    pairs
}

fn residual_parse(
    stream_scope: u64,
    length: usize,
    sequence: u64,
    fingerprint: &str,
) -> AnalysisDecision {
    AnalysisDecision {
        stream_scope,
        response: AnalysisResponse::Selected {
            parse: Parse {
                spans: vec![Span::new(0, length)],
                relation_uses: Vec::new(),
                boundaries: Vec::new(),
                group_refs: Vec::new(),
                evidence_refs: Vec::new(),
            },
        },
        grammar_version: GRAMMAR_VERSION.to_owned(),
        model_fingerprint: fingerprint.to_owned(),
        evidence_refs: Vec::new(),
        operation_cost: 0,
        sequence,
    }
}

fn matched_parse(
    stream_scope: u64,
    length: usize,
    evidence_index: u64,
    sequence: u64,
    fingerprint: &str,
) -> AnalysisDecision {
    AnalysisDecision {
        stream_scope,
        response: AnalysisResponse::Selected {
            parse: Parse {
                spans: vec![Span::new(0, length)],
                relation_uses: vec![RelationUse {
                    span: Span::new(0, length),
                    relation_version: A0_RELATION_VERSION.to_owned(),
                    bindings: Vec::new(),
                    evidence_refs: vec![evidence_index],
                    group_refs: Vec::new(),
                }],
                boundaries: Vec::new(),
                group_refs: Vec::new(),
                evidence_refs: vec![evidence_index],
            },
        },
        grammar_version: GRAMMAR_VERSION.to_owned(),
        model_fingerprint: fingerprint.to_owned(),
        evidence_refs: vec![evidence_index],
        operation_cost: 0,
        sequence,
    }
}

pub struct A0Learner {
    buffer: Vec<u64>,
    scope: u64,
    sequence: u64,
    table: Vec<SignatureEntry>,
}

pub struct A0FrozenLearner {
    buffer: Vec<u64>,
    scope: u64,
    sequence: u64,
    table: Vec<SignatureEntry>,
    fingerprint: String,
}

fn table_fingerprint(table: &[SignatureEntry]) -> String {
    let mut bytes = String::from(A0_MODEL_VERSION);
    for entry in table {
        bytes.push_str(&format!("|{}:", entry.length));
        for (i, j) in &entry.signature {
            bytes.push_str(&format!("{i},{j};"));
        }
    }
    sha256_hex(bytes.as_bytes())
}

fn lookup(table: &[SignatureEntry], tokens: &[u64]) -> Option<u64> {
    let signature = equality_signature(tokens);
    if signature.is_empty() {
        return None;
    }
    for (index, entry) in table.iter().enumerate() {
        if entry.length == tokens.len() && entry.signature == signature {
            return Some(index as u64);
        }
    }
    None
}

impl A0Learner {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            scope: 0,
            sequence: 0,
            table: Vec::new(),
        }
    }
}

impl Default for A0Learner {
    fn default() -> Self {
        Self::new()
    }
}

impl ExperimentALearner for A0Learner {
    type Frozen = A0FrozenLearner;

    fn ingest(&mut self, event: StreamEvent) -> Result<(), LearnerError> {
        match event {
            StreamEvent::Token {
                stream_scope, atom, ..
            } => {
                if self.buffer.is_empty() {
                    self.scope = stream_scope;
                }
                if stream_scope != self.scope {
                    return Err(LearnerError::InvalidScope);
                }
                self.buffer.push(atom);
                Ok(())
            }
            StreamEvent::End { stream_scope, .. } => {
                if stream_scope != self.scope {
                    return Err(LearnerError::InvalidScope);
                }
                Ok(())
            }
        }
    }

    fn analyze(&mut self) -> Result<AnalysisDecision, LearnerError> {
        self.sequence += 1;
        let entry = SignatureEntry {
            length: self.buffer.len(),
            signature: equality_signature(&self.buffer),
        };
        let decision = residual_parse(
            self.scope,
            self.buffer.len(),
            self.sequence,
            A0_MODEL_VERSION,
        );
        self.table.push(entry);
        self.buffer.clear();
        Ok(decision)
    }

    fn begin_development(&mut self) -> Result<(), LearnerError> {
        Ok(())
    }

    fn freeze(self) -> Result<Self::Frozen, LearnerError> {
        let fingerprint = table_fingerprint(&self.table);
        Ok(A0FrozenLearner {
            buffer: Vec::new(),
            scope: self.scope,
            sequence: self.sequence,
            table: self.table,
            fingerprint,
        })
    }
}

impl FrozenExperimentALearner for A0FrozenLearner {
    fn ingest(&mut self, event: StreamEvent) -> Result<(), LearnerError> {
        match event {
            StreamEvent::Token {
                stream_scope, atom, ..
            } => {
                if self.buffer.is_empty() {
                    self.scope = stream_scope;
                }
                if stream_scope != self.scope {
                    return Err(LearnerError::InvalidScope);
                }
                self.buffer.push(atom);
                Ok(())
            }
            StreamEvent::End { stream_scope, .. } => {
                if stream_scope != self.scope {
                    return Err(LearnerError::InvalidScope);
                }
                Ok(())
            }
        }
    }

    fn analyze(&mut self) -> Result<AnalysisDecision, LearnerError> {
        self.sequence += 1;
        let fingerprint = self.fingerprint.clone();
        let decision = match lookup(&self.table, &self.buffer) {
            Some(index) => matched_parse(
                self.scope,
                self.buffer.len(),
                index,
                self.sequence,
                &fingerprint,
            ),
            None => residual_parse(self.scope, self.buffer.len(), self.sequence, &fingerprint),
        };
        self.buffer.clear();
        Ok(decision)
    }

    fn model_fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutable_phase_memorizes_without_segmenting() {
        let mut learner = A0Learner::new();
        for atom in [10u64, 20, 10] {
            learner
                .ingest(StreamEvent::Token {
                    run_scope: 1,
                    stream_scope: 2,
                    atom,
                })
                .unwrap();
        }
        learner
            .ingest(StreamEvent::End {
                run_scope: 1,
                stream_scope: 2,
            })
            .unwrap();
        let decision = learner.analyze().unwrap();
        match decision.response {
            AnalysisResponse::Selected { parse } => {
                assert_eq!(parse.spans, vec![Span::new(0, 3)]);
                assert!(parse.relation_uses.is_empty());
                assert!(parse.boundaries.is_empty());
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(learner.table.len(), 1);
    }

    #[test]
    fn frozen_match_requires_identical_length_and_signature() {
        let mut learner = A0Learner::new();
        for atom in [10u64, 20, 10] {
            learner
                .ingest(StreamEvent::Token {
                    run_scope: 1,
                    stream_scope: 2,
                    atom,
                })
                .unwrap();
        }
        learner
            .ingest(StreamEvent::End {
                run_scope: 1,
                stream_scope: 2,
            })
            .unwrap();
        learner.analyze().unwrap();
        let mut frozen = learner.freeze().unwrap();
        let feed = |frozen: &mut A0FrozenLearner, scope: u64, tokens: &[u64]| {
            for atom in tokens {
                frozen
                    .ingest(StreamEvent::Token {
                        run_scope: 1,
                        stream_scope: scope,
                        atom: *atom,
                    })
                    .unwrap();
            }
            frozen
                .ingest(StreamEvent::End {
                    run_scope: 1,
                    stream_scope: scope,
                })
                .unwrap();
            frozen.analyze().unwrap()
        };
        match feed(&mut frozen, 9, &[10, 20, 10]).response {
            AnalysisResponse::Selected { parse } => assert_eq!(parse.relation_uses.len(), 1),
            other => panic!("unexpected {other:?}"),
        }
        match feed(&mut frozen, 10, &[20, 20, 10]).response {
            AnalysisResponse::Selected { parse } => assert!(parse.relation_uses.is_empty()),
            other => panic!("unexpected {other:?}"),
        }
        match feed(&mut frozen, 11, &[10, 20, 30, 10]).response {
            AnalysisResponse::Selected { parse } => assert!(parse.relation_uses.is_empty()),
            other => panic!("unexpected {other:?}"),
        }
        match feed(&mut frozen, 12, &[10, 20, 30]).response {
            AnalysisResponse::Selected { parse } => assert!(parse.relation_uses.is_empty()),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn frozen_fingerprint_is_deterministic() {
        let mut first = A0Learner::new();
        let mut second = A0Learner::new();
        for learner in [&mut first, &mut second] {
            for atom in [10u64, 20, 10] {
                learner
                    .ingest(StreamEvent::Token {
                        run_scope: 1,
                        stream_scope: 2,
                        atom,
                    })
                    .unwrap();
            }
            learner
                .ingest(StreamEvent::End {
                    run_scope: 1,
                    stream_scope: 2,
                })
                .unwrap();
            learner.analyze().unwrap();
        }
        assert_eq!(
            first.freeze().unwrap().model_fingerprint(),
            second.freeze().unwrap().model_fingerprint()
        );
    }
}
