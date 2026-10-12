use super::a1::{A1FrozenLearner, A1Learner, ReparsedStream};
use super::learner::{ExperimentALearner, FrozenExperimentALearner, LearnerError};
use super::protocol::{AnalysisDecision, StreamEvent};
use super::schedule::Partition;
use std::collections::BTreeSet;

pub const A3_LIBRARY_VERSION: &str = "a3-macro-library-v1";
pub const EXPANDED_USE_COST: u64 = 5;
pub const MACRO_BASE_COST: u64 = 3;
pub const MACRO_LENGTH_COST: u64 = 2;
pub const MACRO_INVOCATION_COST: u64 = 2;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryEntry {
    pub payload_lengths: Vec<u8>,
    pub uses: Vec<(u64, u8)>,
    pub records: Vec<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryReport {
    pub entries: Vec<LibraryEntry>,
    pub promoted: bool,
    pub expanded_cost: u64,
    pub library_cost: u64,
    pub numerator: u64,
    pub denominator: u64,
    pub repeated_expansions_removed: u64,
}

impl LibraryReport {
    pub fn utility_passes(&self) -> bool {
        if !self.promoted {
            return false;
        }
        if self.denominator == 0 {
            return false;
        }
        5 * self.numerator >= self.expanded_cost || self.repeated_expansions_removed >= 2
    }
}

fn training_uses(parsed: &[ReparsedStream]) -> Vec<(u64, u8, usize)> {
    let mut uses = Vec::new();
    for stream in parsed {
        if stream.partition != Partition::Training || stream.ambiguous {
            continue;
        }
        for payload in &stream.payload_order {
            uses.push((stream.global_ordinal, *payload, stream.relation_count));
        }
    }
    uses
}

fn dev_uses(parsed: &[ReparsedStream]) -> Vec<(u64, u8, usize)> {
    let mut uses = Vec::new();
    for stream in parsed {
        if stream.partition != Partition::Development || stream.ambiguous {
            continue;
        }
        for payload in &stream.payload_order {
            uses.push((stream.global_ordinal, *payload, stream.relation_count));
        }
    }
    uses
}

pub fn promote_library(parsed: &[ReparsedStream]) -> LibraryReport {
    let training = training_uses(parsed);
    let records: BTreeSet<u64> = training.iter().map(|(ordinal, _, _)| *ordinal).collect();
    let lengths: BTreeSet<u8> = training.iter().map(|(_, payload, _)| *payload).collect();
    let layouts: BTreeSet<usize> = training.iter().map(|(_, _, layout)| *layout).collect();
    let qualifies = training.len() >= 6
        && records.len() >= 4
        && lengths.len() >= 3
        && layouts.contains(&1)
        && layouts.contains(&2);
    if !qualifies {
        return LibraryReport {
            entries: Vec::new(),
            promoted: false,
            expanded_cost: 0,
            library_cost: 0,
            numerator: 0,
            denominator: 1,
            repeated_expansions_removed: 0,
        };
    }
    let development = dev_uses(parsed);
    let mut entry_lengths: Vec<u8> = lengths.into_iter().collect();
    entry_lengths.sort_unstable();
    let mut entry_records: Vec<u64> = records.into_iter().collect();
    entry_records.sort_unstable();
    let entry_uses: Vec<(u64, u8)> = training
        .iter()
        .map(|(ordinal, payload, _)| (*ordinal, *payload))
        .collect();
    let dev_count = development.len() as u64;
    let expanded_cost = dev_count * EXPANDED_USE_COST;
    let library_cost = MACRO_BASE_COST
        + entry_lengths.len() as u64 * MACRO_LENGTH_COST
        + dev_count * MACRO_INVOCATION_COST;
    let numerator = expanded_cost.saturating_sub(library_cost);
    let distinct_shapes: BTreeSet<u8> = entry_uses.iter().map(|(_, payload)| *payload).collect();
    let repeated_expansions_removed = entry_uses.len() as u64 - distinct_shapes.len() as u64;
    let confirmed = development.len() >= 2 && 5 * numerator >= expanded_cost;
    LibraryReport {
        entries: vec![LibraryEntry {
            payload_lengths: entry_lengths,
            uses: entry_uses,
            records: entry_records,
        }],
        promoted: confirmed,
        expanded_cost,
        library_cost,
        numerator,
        denominator: expanded_cost,
        repeated_expansions_removed,
    }
}

pub struct A3Learner {
    inner: A1Learner,
}

pub struct A3FrozenLearner {
    inner: A1FrozenLearner,
    library: LibraryReport,
}

impl A3Learner {
    pub fn new() -> Self {
        Self {
            inner: A1Learner::new(),
        }
    }
}

impl Default for A3Learner {
    fn default() -> Self {
        Self::new()
    }
}

impl ExperimentALearner for A3Learner {
    type Frozen = A3FrozenLearner;

    fn ingest(&mut self, event: StreamEvent) -> Result<(), LearnerError> {
        self.inner.ingest(event)
    }

    fn analyze(&mut self) -> Result<AnalysisDecision, LearnerError> {
        self.inner.analyze()
    }

    fn begin_development(&mut self) -> Result<(), LearnerError> {
        self.inner.begin_development()
    }

    fn freeze(self) -> Result<Self::Frozen, LearnerError> {
        let library = promote_library(&self.inner.parsed_mutable());
        let inner = self.inner.freeze()?;
        Ok(A3FrozenLearner { inner, library })
    }
}

impl FrozenExperimentALearner for A3FrozenLearner {
    fn ingest(&mut self, event: StreamEvent) -> Result<(), LearnerError> {
        self.inner.ingest(event)
    }

    fn analyze(&mut self) -> Result<AnalysisDecision, LearnerError> {
        self.inner.analyze()
    }

    fn model_fingerprint(&self) -> &str {
        self.inner.model_fingerprint()
    }
}

impl A3FrozenLearner {
    pub fn library(&self) -> &LibraryReport {
        &self.library
    }

    pub fn a1(&self) -> &A1FrozenLearner {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::super::generator::{generate_run, SymbolMode};
    use super::super::protocol::{PRIMARY_SCOPE_SEED, PRIMARY_WORLD_SEED, TRAINING_PARTITION_TAG};
    use super::*;

    const DEV_TAG: u64 = 0x4445_5600_0000_0001;
    const FINAL_TAG: u64 = 0x4649_4e00_0000_0001;

    fn frozen_library() -> LibraryReport {
        let run = generate_run(
            PRIMARY_WORLD_SEED,
            PRIMARY_SCOPE_SEED,
            TRAINING_PARTITION_TAG,
            DEV_TAG,
            FINAL_TAG,
            SymbolMode::Primary,
        );
        let mut learner = A3Learner::new();
        for record in run.records.iter().take(64) {
            for atom in &record.wire_tokens {
                learner
                    .ingest(StreamEvent::Token {
                        run_scope: record.run_scope,
                        stream_scope: record.stream_scope,
                        atom: *atom,
                    })
                    .unwrap();
            }
            learner
                .ingest(StreamEvent::End {
                    run_scope: record.run_scope,
                    stream_scope: record.stream_scope,
                })
                .unwrap();
            learner.analyze().unwrap();
        }
        learner.begin_development().unwrap();
        for record in run.records.iter().skip(64).take(32) {
            for atom in &record.wire_tokens {
                learner
                    .ingest(StreamEvent::Token {
                        run_scope: record.run_scope,
                        stream_scope: record.stream_scope,
                        atom: *atom,
                    })
                    .unwrap();
            }
            learner
                .ingest(StreamEvent::End {
                    run_scope: record.run_scope,
                    stream_scope: record.stream_scope,
                })
                .unwrap();
            learner.analyze().unwrap();
        }
        learner.freeze().unwrap().library
    }

    #[test]
    fn library_promotes_on_the_primary_run() {
        let library = frozen_library();
        assert!(library.promoted);
        assert_eq!(library.entries.len(), 1);
        let entry = &library.entries[0];
        assert!(entry.uses.len() >= 6);
        assert!(entry.records.len() >= 4);
        assert!(entry.payload_lengths.len() >= 3);
    }

    #[test]
    fn utility_gate_passes_on_the_primary_run() {
        let library = frozen_library();
        assert!(library.expanded_cost > library.library_cost);
        assert!(library.utility_passes());
        assert!(library.repeated_expansions_removed >= 2);
    }
}
