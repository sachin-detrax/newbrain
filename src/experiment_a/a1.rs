use super::learner::{ExperimentALearner, FrozenExperimentALearner, LearnerError};
use super::protocol::{
    sha256_hex, AnalysisDecision, AnalysisReason, AnalysisResponse, Budget, Parse, RelationUse,
    ResourceKind, Span, StreamEvent, GRAMMAR_VERSION,
};
use super::schedule::Partition;
use std::collections::{BTreeMap, BTreeSet};

pub const A1_RELATION_VERSION: &str = "a1-range-echo-v1";
pub const A1_MODEL_VERSION: &str = "a1-symbolic-segmentation-v1";
pub const MIN_SPAN_LEN: usize = 3;
pub const MAX_SPAN_LEN: usize = 10;
pub const MIN_PAYLOAD: u8 = 1;
pub const MAX_PAYLOAD: u8 = 8;
pub const MAX_TILING_ALTERNATIVES: usize = 8;

#[derive(Clone, Debug, Eq, PartialEq)]
struct BufferedStream {
    global_ordinal: u64,
    partition: Partition,
    tokens: Vec<u64>,
    matches: Vec<(usize, usize)>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct LengthSupport {
    records: BTreeSet<u64>,
    endpoints: BTreeSet<u64>,
}

pub fn v1_enumerate(tokens: &[u64]) -> (Vec<(usize, usize)>, u64) {
    let n = tokens.len();
    let mut matches = Vec::new();
    let mut considered = 0u64;
    for start in 0..n {
        for end in start + MIN_SPAN_LEN..=n.min(start + MAX_SPAN_LEN) {
            considered += 1;
            if tokens[start] == tokens[end - 1] {
                let payload = (end - start - 2) as u8;
                if (MIN_PAYLOAD..=MAX_PAYLOAD).contains(&payload) {
                    matches.push((start, end));
                }
            }
        }
    }
    (matches, considered)
}

fn tiling_search(candidates: &[(usize, usize)], stream_len: usize) -> Vec<Vec<(usize, usize)>> {
    let mut best: Vec<Vec<(usize, usize)>> = Vec::new();
    let mut best_coverage = 0usize;
    let mut current: Vec<(usize, usize)> = Vec::new();
    let mut used = vec![false; stream_len];
    fn coverage(used: &[bool]) -> usize {
        used.iter().filter(|v| **v).count()
    }
    fn recurse(
        candidates: &[(usize, usize)],
        index: usize,
        current: &mut Vec<(usize, usize)>,
        used: &mut Vec<bool>,
        best: &mut Vec<Vec<(usize, usize)>>,
        best_coverage: &mut usize,
    ) {
        if index == candidates.len() {
            let covered = coverage(used);
            if covered > *best_coverage {
                *best_coverage = covered;
                best.clear();
                best.push(current.clone());
            } else if covered == *best_coverage
                && covered > 0
                && !best.contains(current)
                && best.len() < MAX_TILING_ALTERNATIVES
            {
                best.push(current.clone());
            }
            return;
        }
        recurse(candidates, index + 1, current, used, best, best_coverage);
        let (start, end) = candidates[index];
        if !used[start..end].iter().any(|v| *v) {
            for slot in &mut used[start..end] {
                *slot = true;
            }
            current.push((start, end));
            recurse(candidates, index + 1, current, used, best, best_coverage);
            current.pop();
            for slot in &mut used[start..end] {
                *slot = false;
            }
        }
    }
    recurse(
        candidates,
        0,
        &mut current,
        &mut used,
        &mut best,
        &mut best_coverage,
    );
    best
}

fn tiling_to_parse(
    tiling: &[(usize, usize)],
    stream_len: usize,
    support_refs: &[u64],
    group_refs: Vec<String>,
) -> Parse {
    let mut sorted = tiling.to_vec();
    sorted.sort_unstable();
    let mut spans: Vec<Span> = Vec::new();
    let mut relation_uses: Vec<RelationUse> = Vec::new();
    let mut boundaries: Vec<usize> = Vec::new();
    let mut cursor = 0;
    for (start, end) in &sorted {
        if *start > cursor {
            spans.push(Span::new(cursor, *start));
        }
        spans.push(Span::new(*start, *end));
        relation_uses.push(RelationUse {
            span: Span::new(*start, *end),
            relation_version: A1_RELATION_VERSION.to_owned(),
            bindings: Vec::new(),
            evidence_refs: support_refs.to_vec(),
            group_refs: Vec::new(),
        });
        cursor = *end;
    }
    if cursor < stream_len {
        spans.push(Span::new(cursor, stream_len));
    }
    for span in &spans {
        if span.start > 0 && !boundaries.contains(&span.start) {
            boundaries.push(span.start);
        }
        if span.end < stream_len && !boundaries.contains(&span.end) {
            boundaries.push(span.end);
        }
    }
    boundaries.sort_unstable();
    let evidence_refs = support_refs.to_vec();
    Parse {
        spans,
        relation_uses,
        boundaries,
        group_refs,
        evidence_refs,
    }
}

fn residual_parse(stream_len: usize, group_refs: Vec<String>) -> Parse {
    Parse {
        spans: vec![Span::new(0, stream_len)],
        relation_uses: Vec::new(),
        boundaries: Vec::new(),
        group_refs,
        evidence_refs: Vec::new(),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RangeModel {
    pub fingerprint: String,
    pub support_records: Vec<u64>,
    pub optional_group: bool,
    pub unordered_group: bool,
}

pub struct A1Learner {
    buffer: Vec<u64>,
    scope: u64,
    sequence: u64,
    global_counter: u64,
    streams: Vec<BufferedStream>,
    support: BTreeMap<u8, LengthSupport>,
    contrast: Vec<u64>,
    budget: Budget,
}

pub struct A1FrozenLearner {
    buffer: Vec<u64>,
    scope: u64,
    sequence: u64,
    model: Option<RangeModel>,
    budget: Budget,
}

impl A1Learner {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            scope: 0,
            sequence: 0,
            global_counter: 0,
            streams: Vec::new(),
            support: BTreeMap::new(),
            contrast: Vec::new(),
            budget: Budget::default(),
        }
    }

    fn mutable_parse(&self, stream_len: usize) -> Parse {
        residual_parse(stream_len, Vec::new())
    }

    pub fn parsed_mutable(&self) -> Vec<ReparsedStream> {
        reparse_mutable(&self.streams)
    }
}

impl Default for A1Learner {
    fn default() -> Self {
        Self::new()
    }
}

fn charge_search(budget: &mut Budget, spans: u64) -> Result<(), LearnerError> {
    budget
        .charge(ResourceKind::SpanBoundaryPairs, spans)
        .map_err(LearnerError::Budget)
}

impl ExperimentALearner for A1Learner {
    type Frozen = A1FrozenLearner;

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
        self.global_counter += 1;
        let ordinal = self.global_counter;
        let partition = if ordinal <= 64 {
            Partition::Training
        } else {
            Partition::Development
        };
        let (matches, considered) = v1_enumerate(&self.buffer);
        self.budget.begin_stream();
        if charge_search(&mut self.budget, considered).is_err() {
            let decision = AnalysisDecision {
                stream_scope: self.scope,
                response: AnalysisResponse::Abstain {
                    reason: AnalysisReason::ResourceLimit,
                    hypothesis_refs: Vec::new(),
                    evidence_refs: Vec::new(),
                },
                grammar_version: GRAMMAR_VERSION.to_owned(),
                model_fingerprint: A1_MODEL_VERSION.to_owned(),
                evidence_refs: Vec::new(),
                operation_cost: considered,
                sequence: self.sequence,
            };
            self.buffer.clear();
            return Ok(decision);
        }
        if matches.is_empty() {
            self.contrast.push(ordinal);
        } else {
            for (start, end) in &matches {
                let payload = (*end - *start - 2) as u8;
                let entry = self.support.entry(payload).or_default();
                entry.records.insert(ordinal);
                entry.endpoints.insert(self.buffer[*start]);
            }
        }
        let parse = self.mutable_parse(self.buffer.len());
        let decision = AnalysisDecision {
            stream_scope: self.scope,
            response: AnalysisResponse::Selected { parse },
            grammar_version: GRAMMAR_VERSION.to_owned(),
            model_fingerprint: A1_MODEL_VERSION.to_owned(),
            evidence_refs: Vec::new(),
            operation_cost: considered,
            sequence: self.sequence,
        };
        self.streams.push(BufferedStream {
            global_ordinal: ordinal,
            partition,
            tokens: std::mem::take(&mut self.buffer),
            matches,
        });
        Ok(decision)
    }

    fn begin_development(&mut self) -> Result<(), LearnerError> {
        Ok(())
    }

    fn freeze(self) -> Result<Self::Frozen, LearnerError> {
        let training_records: BTreeSet<u64> = self
            .support
            .values()
            .flat_map(|entry| entry.records.iter().copied())
            .filter(|ordinal| *ordinal <= 64)
            .collect();
        let training_lengths = self
            .support
            .iter()
            .filter(|(_, entry)| entry.records.iter().any(|ordinal| *ordinal <= 64))
            .count();
        let training_endpoints: BTreeSet<u64> = self
            .support
            .values()
            .flat_map(|entry| entry.endpoints.iter().copied())
            .collect();
        let training_contrast = self.contrast.iter().filter(|o| **o <= 64).count();
        let dev_match: BTreeSet<u64> = self
            .support
            .values()
            .flat_map(|entry| entry.records.iter().copied())
            .filter(|ordinal| *ordinal > 64)
            .collect();
        let dev_contrast = self.contrast.iter().filter(|o| **o > 64).count();
        let admitted = training_records.len() >= 4
            && training_lengths >= 3
            && training_endpoints.len() >= 4
            && training_contrast >= 2
            && dev_match.len() >= 2
            && dev_contrast >= 2;
        let model = if admitted {
            let mut support_records: Vec<u64> = training_records
                .iter()
                .copied()
                .chain(dev_match.iter().copied())
                .collect();
            support_records.sort_unstable();
            let reparsed = reparse_mutable(&self.streams);
            let optional_group = optional_supported(&reparsed);
            let unordered_group = unordered_supported(&reparsed);
            let fingerprint = sha256_hex(
                format!(
                    "{A1_MODEL_VERSION}|admitted|support:{}|optional:{optional_group}|unordered:{unordered_group}",
                    support_records.len()
                )
                .as_bytes(),
            );
            Some(RangeModel {
                fingerprint,
                support_records,
                optional_group,
                unordered_group,
            })
        } else {
            None
        };
        Ok(A1FrozenLearner {
            buffer: Vec::new(),
            scope: self.scope,
            sequence: self.sequence,
            model,
            budget: self.budget,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReparsedStream {
    pub partition: Partition,
    pub ambiguous: bool,
    pub relation_count: usize,
    pub payload_order: Vec<u8>,
    pub global_ordinal: u64,
}

fn reparse_mutable(streams: &[BufferedStream]) -> Vec<ReparsedStream> {
    let mut out = Vec::new();
    for stream in streams {
        let tilings = tiling_search(&stream.matches, stream.tokens.len());
        let ambiguous = tilings.len() >= 2;
        let first = tilings.first().cloned().unwrap_or_default();
        let payload_order: Vec<u8> = first.iter().map(|(s, e)| (*e - *s - 2) as u8).collect();
        out.push(ReparsedStream {
            partition: stream.partition,
            ambiguous,
            relation_count: first.len(),
            payload_order,
            global_ordinal: stream.global_ordinal,
        });
    }
    out
}

fn optional_supported(reparsed: &[ReparsedStream]) -> bool {
    let training_one = reparsed
        .iter()
        .filter(|s| s.partition == Partition::Training && !s.ambiguous && s.relation_count == 1)
        .count();
    let training_two = reparsed
        .iter()
        .filter(|s| s.partition == Partition::Training && !s.ambiguous && s.relation_count == 2)
        .count();
    let dev_one = reparsed
        .iter()
        .filter(|s| s.partition == Partition::Development && !s.ambiguous && s.relation_count == 1)
        .count();
    let dev_two = reparsed
        .iter()
        .filter(|s| s.partition == Partition::Development && !s.ambiguous && s.relation_count == 2)
        .count();
    training_one >= 2 && training_two >= 2 && dev_one >= 1 && dev_two >= 1
}

fn unordered_supported(reparsed: &[ReparsedStream]) -> bool {
    let ascending = |order: &[u8]| order.len() == 2 && order[0] < order[1];
    let descending = |order: &[u8]| order.len() == 2 && order[0] > order[1];
    let training_asc = reparsed
        .iter()
        .filter(|s| {
            s.partition == Partition::Training && !s.ambiguous && ascending(&s.payload_order)
        })
        .count();
    let training_desc = reparsed
        .iter()
        .filter(|s| {
            s.partition == Partition::Training && !s.ambiguous && descending(&s.payload_order)
        })
        .count();
    let dev_asc = reparsed
        .iter()
        .filter(|s| {
            s.partition == Partition::Development && !s.ambiguous && ascending(&s.payload_order)
        })
        .count();
    let dev_desc = reparsed
        .iter()
        .filter(|s| {
            s.partition == Partition::Development && !s.ambiguous && descending(&s.payload_order)
        })
        .count();
    training_asc >= 2 && training_desc >= 2 && dev_asc >= 1 && dev_desc >= 1
}

impl A1FrozenLearner {
    pub fn admitted(&self) -> bool {
        self.model.is_some()
    }

    pub fn model(&self) -> Option<&RangeModel> {
        self.model.as_ref()
    }
}

impl FrozenExperimentALearner for A1FrozenLearner {
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
        let stream_len = self.buffer.len();
        let (matches, considered) = v1_enumerate(&self.buffer);
        self.budget.begin_stream();
        if charge_search(&mut self.budget, considered).is_err() {
            let decision = AnalysisDecision {
                stream_scope: self.scope,
                response: AnalysisResponse::Abstain {
                    reason: AnalysisReason::ResourceLimit,
                    hypothesis_refs: Vec::new(),
                    evidence_refs: Vec::new(),
                },
                grammar_version: GRAMMAR_VERSION.to_owned(),
                model_fingerprint: self.fingerprint().to_owned(),
                evidence_refs: Vec::new(),
                operation_cost: considered,
                sequence: self.sequence,
            };
            self.buffer.clear();
            return Ok(decision);
        }
        let decision = match &self.model {
            None => AnalysisDecision {
                stream_scope: self.scope,
                response: AnalysisResponse::Selected {
                    parse: residual_parse(stream_len, Vec::new()),
                },
                grammar_version: GRAMMAR_VERSION.to_owned(),
                model_fingerprint: self.fingerprint().to_owned(),
                evidence_refs: Vec::new(),
                operation_cost: considered,
                sequence: self.sequence,
            },
            Some(model) => {
                let tilings = tiling_search(&matches, stream_len);
                let response = if tilings.is_empty() {
                    AnalysisResponse::Selected {
                        parse: residual_parse(stream_len, Vec::new()),
                    }
                } else if tilings.len() >= 2 {
                    let first = tiling_to_parse(
                        &tilings[0],
                        stream_len,
                        &model.support_records,
                        Vec::new(),
                    );
                    let second = tiling_to_parse(
                        &tilings[1],
                        stream_len,
                        &model.support_records,
                        Vec::new(),
                    );
                    AnalysisResponse::Alternatives {
                        parses: [first, second],
                        reason: AnalysisReason::AmbiguousSegmentation,
                    }
                } else {
                    let groups = group_refs_for(&tilings[0], model);
                    AnalysisResponse::Selected {
                        parse: tiling_to_parse(
                            &tilings[0],
                            stream_len,
                            &model.support_records,
                            groups,
                        ),
                    }
                };
                AnalysisDecision {
                    stream_scope: self.scope,
                    response,
                    grammar_version: GRAMMAR_VERSION.to_owned(),
                    model_fingerprint: model.fingerprint.clone(),
                    evidence_refs: model.support_records.clone(),
                    operation_cost: considered,
                    sequence: self.sequence,
                }
            }
        };
        self.buffer.clear();
        Ok(decision)
    }

    fn model_fingerprint(&self) -> &str {
        match &self.model {
            Some(model) => &model.fingerprint,
            None => A1_MODEL_VERSION,
        }
    }
}

impl A1FrozenLearner {
    fn fingerprint(&self) -> &str {
        self.model_fingerprint()
    }
}

fn group_refs_for(tiling: &[(usize, usize)], model: &RangeModel) -> Vec<String> {
    let mut groups = Vec::new();
    if model.optional_group && tiling.len() <= 2 {
        groups.push(if tiling.len() == 1 {
            "optional-absent".to_owned()
        } else {
            "optional-present".to_owned()
        });
    }
    if model.unordered_group && tiling.len() == 2 {
        groups.push("unordered".to_owned());
    }
    groups
}
