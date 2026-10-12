use std::fmt;

pub type Symbol = u8;
pub type Action = u8;
pub type Scope = u8;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RecordId(pub u16);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct QueryId(pub u16);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProcedureId(pub u16);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Sequence([Symbol; 3]);

impl Sequence {
    pub const fn new(positions: [Symbol; 3]) -> Self {
        Self(positions)
    }

    pub const fn positions(self) -> [Symbol; 3] {
        self.0
    }

    pub const fn get(self, position: usize) -> Option<Symbol> {
        match position {
            0..=2 => Some(self.0[position]),
            _ => None,
        }
    }
}

impl fmt::Display for Sequence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "[{}, {}, {}]", self.0[0], self.0[1], self.0[2])
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EvidenceRef {
    record_id: RecordId,
}

impl EvidenceRef {
    pub const fn record(record_id: RecordId) -> Self {
        Self { record_id }
    }

    pub const fn record_id(self) -> RecordId {
        self.record_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transition {
    pub input: Sequence,
    pub action: Action,
    pub output: Sequence,
    pub scope: Scope,
    pub record_id: RecordId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Query {
    pub input: Sequence,
    pub action: Action,
    pub scope: Scope,
    pub query_id: QueryId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbstentionReason {
    NoApplicableProcedure,
    Conflict,
    Incomplete,
    UnsupportedByBudget,
    UnsupportedByGrammar,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredictionPayload {
    output: Sequence,
    procedure_ids: Vec<ProcedureId>,
    evidence_refs: Vec<EvidenceRef>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbstentionPayload {
    reason: AbstentionReason,
    procedure_ids: Vec<ProcedureId>,
    evidence_refs: Vec<EvidenceRef>,
}

#[derive(Clone, Eq, PartialEq)]
pub enum Response {
    Prediction(PredictionPayload),
    Abstention(AbstentionPayload),
}

impl fmt::Debug for Response {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Prediction(payload) => formatter
                .debug_struct("Prediction")
                .field("output", &payload.output)
                .field("procedure_ids", &payload.procedure_ids)
                .field("evidence_refs", &payload.evidence_refs)
                .finish(),
            Self::Abstention(payload) => formatter
                .debug_struct("Abstention")
                .field("reason", &payload.reason)
                .field("procedure_ids", &payload.procedure_ids)
                .field("evidence_refs", &payload.evidence_refs)
                .finish(),
        }
    }
}

impl Response {
    pub fn output(&self) -> Option<Sequence> {
        match self {
            Self::Prediction(payload) => Some(payload.output),
            Self::Abstention(_) => None,
        }
    }

    pub fn reason(&self) -> Option<AbstentionReason> {
        match self {
            Self::Prediction(_) => None,
            Self::Abstention(payload) => Some(payload.reason),
        }
    }

    pub fn procedure_ids(&self) -> &[ProcedureId] {
        match self {
            Self::Prediction(payload) => &payload.procedure_ids,
            Self::Abstention(payload) => &payload.procedure_ids,
        }
    }

    pub fn evidence_refs(&self) -> &[EvidenceRef] {
        match self {
            Self::Prediction(payload) => &payload.evidence_refs,
            Self::Abstention(payload) => &payload.evidence_refs,
        }
    }

    pub fn prediction(
        output: Sequence,
        procedure_id: ProcedureId,
        evidence_refs: Vec<EvidenceRef>,
    ) -> Self {
        Self::prediction_many(output, vec![procedure_id], evidence_refs)
            .expect("single procedure prediction must have a procedure ID")
    }

    pub fn prediction_many(
        output: Sequence,
        procedure_ids: Vec<ProcedureId>,
        evidence_refs: Vec<EvidenceRef>,
    ) -> Option<Self> {
        (!procedure_ids.is_empty()).then_some(Self::Prediction(PredictionPayload {
            output,
            procedure_ids,
            evidence_refs,
        }))
    }

    pub fn abstention(
        reason: AbstentionReason,
        procedure_ids: Vec<ProcedureId>,
        evidence_refs: Vec<EvidenceRef>,
    ) -> Self {
        Self::Abstention(AbstentionPayload {
            reason,
            procedure_ids,
            evidence_refs,
        })
    }
}
