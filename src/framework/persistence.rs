//! Versioned, standard-library-only persistence for the reusable framework.
//!
//! The format is one canonical UTF-8 JSON document. Object keys are sorted
//! lexicographically and arrays use the store/ledger order documented by their
//! owning types. The top-level `fingerprint` is an FNV-1a-64 fingerprint of the
//! same document with that field omitted; excluding the fingerprint avoids a
//! self-referential hash while still making save/load equality checkable.
//!
//! The snapshot contains learner-visible protocol records only. It does not
//! contain evaluator labels, hidden world state, seeds, schedules, or scoring.
//! Logical resource counters, limits, and stage operation counts are retained;
//! wall-clock and resident-memory samples are process-local measurements and
//! are intentionally not restored.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

use super::grammar_admission::{
    Candidate, CandidateCosts, CandidateScope, CandidateStage, EqualityRelation, InputPattern,
    OutputOp, PatternAtom, ProcedureRecord, ProcedureRevision, ProcedureStatus,
    PromotionProvenance, RejectionReason, VariableId,
};
use super::interaction_ledger::{
    AuditScope, DecisionRecord, EvidenceProvenance, EvidenceRecord, InteractionLedger,
    InteractionRecord, RunPhase,
};
use super::opaque_protocol::{
    AbstainReason, ActMode, ActionId, AtomId, AuditReferences, Decision, EvidenceId, HypothesisId,
    ObservationPacket, PreviousTerminal, ProcedureId, ScopeToken, StepInput, TerminalStatus,
};
use super::resource_scale::{
    LogicalCounters, LogicalResource, ProtocolResourceLimits, ResourceLimit, ResourceMeter,
    ResourcePersistenceSnapshot, Stage, V2_GRAMMAR_VERSION, V2_PROTOCOL_VERSION,
};
use super::stores::{
    DependencySource, DependentRecord, HypothesisRecord, HypothesisTransition, LearnerStores,
    ProcedureVersion, StoreError, StoredHypothesisStatus,
};
use super::LearnerState;

pub const PERSISTENCE_FORMAT: &str = "sparse-structural-rule-learner.framework-state";
pub const PERSISTENCE_SCHEMA_VERSION: u32 = 1;

/// The deterministic hexadecimal fingerprint returned by save/load APIs.
pub type StateFingerprint = String;

/// Errors returned by framework state save and restore.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PersistenceError {
    Io {
        operation: &'static str,
        message: String,
    },
    UnsupportedFormat(String),
    UnsupportedSchemaVersion(u64),
    IncompleteFile,
    MalformedRecord(String),
    DuplicateId {
        entity: String,
        id: String,
    },
    DuplicateVersion {
        procedure_id: String,
        version: u32,
    },
    ScopeViolation(String),
    InvalidStatusTransition {
        hypothesis_id: String,
    },
    MismatchedReference(String),
    FingerprintMismatch {
        expected: String,
        actual: String,
    },
    ActiveDecisionCannotBeSaved,
    FrozenStateMutation,
    Store(String),
    Ledger(String),
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { operation, message } => {
                write!(formatter, "persistence {operation}: {message}")
            }
            Self::UnsupportedFormat(format) => {
                write!(formatter, "unsupported persistence format {format}")
            }
            Self::UnsupportedSchemaVersion(version) => {
                write!(
                    formatter,
                    "unsupported persistence schema version {version}"
                )
            }
            Self::IncompleteFile => formatter.write_str("incomplete persistence file"),
            Self::MalformedRecord(message) => {
                write!(formatter, "malformed persistence record: {message}")
            }
            Self::DuplicateId { entity, id } => write!(formatter, "duplicate {entity} id {id}"),
            Self::DuplicateVersion {
                procedure_id,
                version,
            } => {
                write!(
                    formatter,
                    "duplicate procedure {procedure_id} version {version}"
                )
            }
            Self::ScopeViolation(message) => {
                write!(formatter, "persistence scope violation: {message}")
            }
            Self::InvalidStatusTransition { hypothesis_id } => {
                write!(
                    formatter,
                    "invalid status transition for hypothesis {hypothesis_id}"
                )
            }
            Self::MismatchedReference(message) => {
                write!(formatter, "mismatched persistence reference: {message}")
            }
            Self::FingerprintMismatch { expected, actual } => {
                write!(
                    formatter,
                    "state fingerprint mismatch: expected {expected}, got {actual}"
                )
            }
            Self::ActiveDecisionCannotBeSaved => {
                formatter.write_str("an active decision must be resolved before saving")
            }
            Self::FrozenStateMutation => {
                formatter.write_str("frozen framework state cannot be mutated")
            }
            Self::Store(message) => write!(formatter, "persistence store restore: {message}"),
            Self::Ledger(message) => write!(formatter, "persistence ledger restore: {message}"),
        }
    }
}

impl std::error::Error for PersistenceError {}

/// The learner-owned aggregate persisted by MVP-4.
pub struct FrameworkState {
    learner: LearnerState,
    ledger: InteractionLedger,
    stores: LearnerStores,
}

impl FrameworkState {
    pub fn new(run_scope: ScopeToken) -> Result<Self, PersistenceError> {
        Self::with_resource_meter(run_scope, ResourceMeter::runtime())
    }

    pub fn with_resource_meter(
        run_scope: ScopeToken,
        meter: ResourceMeter,
    ) -> Result<Self, PersistenceError> {
        let learner =
            LearnerState::new(run_scope).map_err(|error| protocol_error(error.to_string()))?;
        let ledger = InteractionLedger::with_resource_meter(run_scope, meter)
            .map_err(|error| PersistenceError::Ledger(error.to_string()))?;
        let stores = LearnerStores::new(run_scope)
            .map_err(|error| PersistenceError::Store(error.to_string()))?;
        Ok(Self {
            learner,
            ledger,
            stores,
        })
    }

    pub fn from_parts(
        learner: LearnerState,
        ledger: InteractionLedger,
        stores: LearnerStores,
    ) -> Result<Self, PersistenceError> {
        let state = Self {
            learner,
            ledger,
            stores,
        };
        state.validate_consistency()?;
        Ok(state)
    }

    pub const fn run_scope(&self) -> ScopeToken {
        self.ledger.run_scope()
    }

    pub const fn learner(&self) -> &LearnerState {
        &self.learner
    }

    pub const fn ledger(&self) -> &InteractionLedger {
        &self.ledger
    }

    pub const fn stores(&self) -> &LearnerStores {
        &self.stores
    }

    pub const fn is_frozen(&self) -> bool {
        self.ledger.is_frozen()
    }

    pub fn ledger_mut(&mut self) -> Result<&mut InteractionLedger, PersistenceError> {
        self.ensure_mutable()?;
        Ok(&mut self.ledger)
    }

    /// Borrow the frozen ledger for receipt-backed final evaluation.
    ///
    /// A frozen learner policy and the learner-owned stores remain immutable,
    /// but the transport ledger must still append final decisions and opaque
    /// environment results. This is the same narrow mutation performed by the
    /// generic controller during `FinalEvaluation`; ordinary checkpoint and
    /// store mutation continues to use [`Self::ledger_mut`] and remains
    /// rejected after freeze.
    pub fn final_evaluation_ledger_mut(
        &mut self,
    ) -> Result<&mut InteractionLedger, PersistenceError> {
        if !self.is_frozen() || !self.learner.is_frozen() {
            return Err(PersistenceError::FrozenStateMutation);
        }
        Ok(&mut self.ledger)
    }

    pub fn stores_mut(&mut self) -> Result<&mut LearnerStores, PersistenceError> {
        self.ensure_mutable()?;
        Ok(&mut self.stores)
    }

    /// Freeze the ledger and learner together. Frozen stores are only exposed
    /// through immutable accessors after this call.
    pub fn freeze(&mut self) -> Result<(), PersistenceError> {
        self.ledger
            .freeze()
            .map_err(|error| PersistenceError::Ledger(error.to_string()))?;
        self.learner.freeze();
        Ok(())
    }

    pub fn fingerprint(&self) -> Result<StateFingerprint, PersistenceError> {
        Ok(fingerprint_for(&self.canonical_body()?))
    }

    pub fn save<P: AsRef<Path>>(
        &self,
        destination: P,
    ) -> Result<StateFingerprint, PersistenceError> {
        let body = self.canonical_body()?;
        let fingerprint = fingerprint_for(&body);
        let document = add_fingerprint(&body, &fingerprint)?;
        atomic_write(destination.as_ref(), document.as_bytes())?;
        Ok(fingerprint)
    }

    pub fn load<P: AsRef<Path>>(source: P) -> Result<Self, PersistenceError> {
        let mut file = File::open(source.as_ref()).map_err(|error| io_error("open", error))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|error| io_error("read", error))?;
        if bytes.is_empty() {
            return Err(PersistenceError::IncompleteFile);
        }
        let value = JsonParser::new(&bytes).parse_document()?;
        let persisted_fingerprint = value.get_string("fingerprint")?.to_owned();
        let format = value.get_string("format")?;
        if format != PERSISTENCE_FORMAT {
            return Err(PersistenceError::UnsupportedFormat(format.to_owned()));
        }
        let schema_version = value.get_u64("schema_version")?;
        if schema_version != u64::from(PERSISTENCE_SCHEMA_VERSION) {
            return Err(PersistenceError::UnsupportedSchemaVersion(schema_version));
        }

        let canonical_input = value.to_json();
        let state = restore_state(value)?;
        let body = state.canonical_body()?;
        let actual = fingerprint_for(&body);
        if persisted_fingerprint != actual {
            return Err(PersistenceError::FingerprintMismatch {
                expected: persisted_fingerprint,
                actual,
            });
        }
        if canonical_input != add_fingerprint(&body, &actual)? {
            return Err(PersistenceError::MalformedRecord(
                "record representation is not canonical".to_owned(),
            ));
        }
        Ok(state)
    }

    fn ensure_mutable(&self) -> Result<(), PersistenceError> {
        if self.is_frozen() || self.learner.is_frozen() {
            Err(PersistenceError::FrozenStateMutation)
        } else {
            Ok(())
        }
    }

    fn validate_consistency(&self) -> Result<(), PersistenceError> {
        if self.learner.run_scope() != self.ledger.run_scope()
            || self.ledger.run_scope() != self.stores.run_scope()
        {
            return Err(PersistenceError::ScopeViolation(
                "aggregate components use different run scopes".to_owned(),
            ));
        }
        if self.learner.is_frozen() != self.ledger.is_frozen() {
            return Err(PersistenceError::MalformedRecord(
                "learner and ledger phases differ".to_owned(),
            ));
        }
        if !self.ledger.is_checkpoint_ready() {
            return Err(PersistenceError::ActiveDecisionCannotBeSaved);
        }
        validate_ledger(&self.ledger)?;
        validate_stores(&self.stores, self.run_scope())?;
        validate_dependencies(
            &self.stores,
            &self.stores.dependencies().links().collect::<Vec<_>>(),
        )?;
        validate_cross_references(&self.stores, &self.ledger)?;
        validate_resource_snapshot(&self.ledger.resource_meter().persistence_snapshot())?;
        Ok(())
    }

    fn canonical_body(&self) -> Result<String, PersistenceError> {
        self.validate_consistency()?;
        Ok(json_object(vec![
            ("complete", "true".to_owned()),
            ("format", json_string(PERSISTENCE_FORMAT)),
            ("grammar_version", json_string(V2_GRAMMAR_VERSION)),
            ("learner", learner_json(&self.learner)),
            ("ledger", ledger_json(&self.ledger)),
            ("phase", json_string(phase_name(self.ledger.phase()))),
            ("protocol_version", json_string(V2_PROTOCOL_VERSION)),
            (
                "resources",
                resource_json(&self.ledger.resource_meter().persistence_snapshot()),
            ),
            ("run_scope", json_string(&self.run_scope().to_string())),
            ("schema_version", PERSISTENCE_SCHEMA_VERSION.to_string()),
            ("stores", stores_json(&self.stores)),
        ]))
    }
}

fn protocol_error(message: String) -> PersistenceError {
    PersistenceError::MalformedRecord(message)
}

fn io_error(operation: &'static str, error: io::Error) -> PersistenceError {
    PersistenceError::Io {
        operation,
        message: error.to_string(),
    }
}

fn phase_name(phase: RunPhase) -> &'static str {
    match phase {
        RunPhase::Mutable => "mutable",
        RunPhase::Frozen => "frozen",
    }
}

fn learner_json(learner: &LearnerState) -> String {
    json_object(vec![
        ("decision_count", learner.decision_count().to_string()),
        ("frozen", learner.is_frozen().to_string()),
        ("run_scope", json_string(&learner.run_scope().to_string())),
    ])
}

fn add_fingerprint(body: &str, fingerprint: &str) -> Result<String, PersistenceError> {
    let value = JsonParser::new(body.as_bytes()).parse_document()?;
    let mut fields = value.object_fields()?;
    fields.insert(
        "fingerprint".to_owned(),
        JsonValue::String(fingerprint.to_owned()),
    );
    Ok(json_value_object(&fields))
}

fn atomic_write(destination: &Path, bytes: &[u8]) -> Result<(), PersistenceError> {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| PersistenceError::Io {
            operation: "prepare",
            message: "destination has no valid UTF-8 file name".to_owned(),
        })?;
    let mut temporary = None;
    for attempt in 0..100_u32 {
        let candidate = parent.join(format!(
            ".{file_name}.mvp4-tmp-{}-{attempt}",
            std::process::id()
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                temporary = Some((candidate, file));
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(io_error("create temporary", error)),
        }
    }
    let (temporary_path, mut file) = temporary.ok_or_else(|| PersistenceError::Io {
        operation: "create temporary",
        message: "could not allocate a unique temporary path".to_owned(),
    })?;
    let result = (|| {
        file.write_all(bytes)
            .map_err(|error| io_error("write temporary", error))?;
        file.sync_all()
            .map_err(|error| io_error("sync temporary", error))?;
        drop(file);
        replace_atomically(&temporary_path, destination)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

fn replace_atomically(temporary: &Path, destination: &Path) -> Result<(), PersistenceError> {
    replace_atomically_platform(temporary, destination)
}

#[cfg(not(windows))]
fn replace_atomically_platform(
    temporary: &Path,
    destination: &Path,
) -> Result<(), PersistenceError> {
    // POSIX rename replaces the destination atomically. There is exactly one
    // namespace operation, so a failed write cannot damage the old snapshot.
    fs::rename(temporary, destination).map_err(|error| io_error("rename", error))
}

#[cfg(windows)]
fn replace_atomically_platform(
    temporary: &Path,
    destination: &Path,
) -> Result<(), PersistenceError> {
    use std::os::windows::ffi::OsStrExt;

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(existing: *const u16, replacement: *const u16, flags: u32) -> i32;
    }

    let existing = temporary
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let replacement = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let moved = unsafe {
        MoveFileExW(
            existing.as_ptr(),
            replacement.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(io_error("rename", io::Error::last_os_error()))
    } else {
        Ok(())
    }
}

fn fingerprint_for(body: &str) -> StateFingerprint {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in body.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

fn json_array(values: impl IntoIterator<Item = String>) -> String {
    format!("[{}]", values.into_iter().collect::<Vec<_>>().join(","))
}

fn json_object(mut fields: Vec<(&str, String)>) -> String {
    fields.sort_unstable_by_key(|(key, _)| *key);
    format!(
        "{{{}}}",
        fields
            .into_iter()
            .map(|(key, value)| format!("{}:{value}", json_string(key)))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn json_value_object(fields: &BTreeMap<String, JsonValue>) -> String {
    json_object(
        fields
            .iter()
            .map(|(key, value)| (key.as_str(), value.to_json()))
            .collect(),
    )
}

fn ids_json<I>(ids: I) -> String
where
    I: IntoIterator,
    I::Item: Into<u64>,
{
    json_array(
        ids.into_iter()
            .map(|id| json_string(&id.into().to_string())),
    )
}

fn packet_json(packet: ObservationPacket) -> String {
    json_object(vec![(
        "atoms",
        ids_json(packet.atoms().into_iter().map(AtomId::value)),
    )])
}

fn input_json(input: StepInput) -> String {
    json_object(vec![
        (
            "available_actions",
            ids_json(input.available_actions().into_iter().map(ActionId::value)),
        ),
        (
            "episode_scope",
            json_string(&input.episode_scope().to_string()),
        ),
        ("observation", packet_json(input.observation())),
        (
            "previous_terminal",
            json_string(match input.previous_terminal() {
                PreviousTerminal::None => "none",
                PreviousTerminal::Continue => "continue",
            }),
        ),
        (
            "remaining_action_budget",
            input.remaining_action_budget().to_string(),
        ),
        ("run_scope", json_string(&input.run_scope().to_string())),
        ("step_index", input.step_index().to_string()),
    ])
}

fn audit_json(audit: &AuditReferences) -> String {
    json_object(vec![
        (
            "evidence_refs",
            ids_json(audit.evidence_refs().iter().map(|id| id.value())),
        ),
        (
            "hypothesis_refs",
            ids_json(audit.hypothesis_refs().iter().map(|id| id.value())),
        ),
        (
            "procedure_refs",
            ids_json(audit.procedure_refs().iter().map(|id| id.value())),
        ),
    ])
}

fn decision_json(decision: &Decision) -> String {
    match decision {
        Decision::Act(value) => json_object(vec![
            ("action", json_string(&value.action().to_string())),
            ("audit", audit_json(value.audit())),
            ("mode", json_string(act_mode_name(value.mode()))),
            (
                "prediction",
                value
                    .prediction()
                    .map(packet_json)
                    .unwrap_or_else(|| "null".to_owned()),
            ),
            ("type", json_string("act")),
        ]),
        Decision::Abstain(value) => json_object(vec![
            ("audit", audit_json(value.audit())),
            ("reason", json_string(abstain_reason_name(value.reason()))),
            ("type", json_string("abstain")),
        ]),
    }
}

fn decision_record_json(record: &DecisionRecord) -> String {
    json_object(vec![
        ("decision", decision_json(record.decision())),
        ("input", input_json(record.input())),
        ("phase", json_string(phase_name(record.phase()))),
        ("sequence", record.sequence().to_string()),
    ])
}

fn evidence_json(record: &EvidenceRecord) -> String {
    json_object(vec![
        (
            "action",
            record
                .action()
                .map(|id| json_string(&id.to_string()))
                .unwrap_or_else(|| "null".to_owned()),
        ),
        (
            "after",
            record
                .after()
                .map(packet_json)
                .unwrap_or_else(|| "null".to_owned()),
        ),
        ("before", packet_json(record.before())),
        ("decision_sequence", record.decision_sequence().to_string()),
        (
            "episode_scope",
            json_string(&record.episode_scope().to_string()),
        ),
        (
            "evidence_id",
            json_string(&record.evidence_id().to_string()),
        ),
        (
            "provenance",
            json_string(provenance_name(record.provenance())),
        ),
        ("run_scope", json_string(&record.run_scope().to_string())),
        ("step_index", record.step_index().to_string()),
        ("terminal", json_string(terminal_name(record.terminal()))),
    ])
}

fn interaction_json(record: &InteractionRecord) -> String {
    json_object(vec![
        ("decision", decision_record_json(record.decision())),
        ("evidence", evidence_json(record.evidence())),
    ])
}

fn ledger_json(ledger: &InteractionLedger) -> String {
    let known_procedures = ledger
        .known_procedures()
        .into_iter()
        .map(|(id, scope, checked)| {
            json_object(vec![
                ("checked", checked.to_string()),
                ("id", json_string(&id.to_string())),
                ("scope", audit_scope_json(scope)),
            ])
        });
    let known_hypotheses = ledger.known_hypotheses().into_iter().map(|(id, scope)| {
        json_object(vec![
            ("id", json_string(&id.to_string())),
            ("scope", audit_scope_json(scope)),
        ])
    });
    json_object(vec![
        (
            "decision_trace_jsonl",
            json_string(ledger.decision_trace_jsonl()),
        ),
        (
            "decisions",
            json_array(ledger.decisions().iter().map(decision_record_json)),
        ),
        (
            "interactions",
            json_array(ledger.interactions().iter().map(interaction_json)),
        ),
        (
            "known_evidence",
            ids_json(ledger.known_evidence().into_iter().map(|id| id.value())),
        ),
        ("known_hypotheses", json_array(known_hypotheses)),
        ("known_procedures", json_array(known_procedures)),
        ("phase", json_string(phase_name(ledger.phase()))),
        ("run_scope", json_string(&ledger.run_scope().to_string())),
    ])
}

fn audit_scope_json(scope: AuditScope) -> String {
    match scope {
        AuditScope::Run => json_object(vec![("kind", json_string("run"))]),
        AuditScope::Episode(token) => json_object(vec![
            ("kind", json_string("episode")),
            ("token", json_string(&token.to_string())),
        ]),
    }
}

fn candidate_scope_json(scope: CandidateScope) -> String {
    json_object(vec![
        (
            "kind",
            json_string(if scope.is_run() { "run" } else { "episode" }),
        ),
        ("token", json_string(&scope.token().to_string())),
    ])
}

fn candidate_costs_json(costs: CandidateCosts) -> String {
    json_object(vec![
        (
            "challenge_comparisons",
            costs.challenge_comparisons().to_string(),
        ),
        (
            "generation_attempts",
            costs.generation_attempts().to_string(),
        ),
        ("operation_steps", costs.operation_steps().to_string()),
        (
            "structural_comparisons",
            costs.structural_comparisons().to_string(),
        ),
    ])
}

fn relation_json(relation: EqualityRelation) -> String {
    json_array([relation.left().to_string(), relation.right().to_string()])
}

fn pattern_atom_json(atom: PatternAtom) -> String {
    match atom {
        PatternAtom::Constant(value) => {
            json_object(vec![("constant", json_string(&value.to_string()))])
        }
        PatternAtom::Variable(value) => json_object(vec![("variable", value.value().to_string())]),
    }
}

fn output_op_json(operation: OutputOp) -> String {
    match operation {
        OutputOp::Copy(position) => json_object(vec![("copy", position.to_string())]),
        OutputOp::Observed(value) => {
            json_object(vec![("observed", json_string(&value.to_string()))])
        }
    }
}

fn positions_json(values: &[usize]) -> String {
    json_array(values.iter().map(|value| value.to_string()))
}

fn precondition_json(values: &[(usize, AtomId)]) -> String {
    json_array(values.iter().map(|(position, atom)| {
        json_object(vec![
            ("atom", json_string(&atom.to_string())),
            ("position", position.to_string()),
        ])
    }))
}

fn candidate_json(candidate: &Candidate) -> String {
    json_object(vec![
        ("action", json_string(&candidate.action().to_string())),
        (
            "changed_positions",
            positions_json(candidate.changed_positions()),
        ),
        (
            "challenge_evidence_ids",
            ids_json(
                candidate
                    .challenge_evidence_ids()
                    .iter()
                    .map(|id| id.value()),
            ),
        ),
        ("costs", candidate_costs_json(candidate.costs())),
        (
            "grounding_evidence_ids",
            ids_json(
                candidate
                    .grounding_evidence_ids()
                    .iter()
                    .map(|id| id.value()),
            ),
        ),
        (
            "hypothesis_id",
            json_string(&candidate.hypothesis_id().to_string()),
        ),
        (
            "input_equalities",
            json_array(
                candidate
                    .input_equality_relations()
                    .iter()
                    .copied()
                    .map(relation_json),
            ),
        ),
        (
            "input_pattern",
            json_array(
                candidate
                    .input_pattern()
                    .atoms()
                    .into_iter()
                    .map(pattern_atom_json),
            ),
        ),
        (
            "output",
            json_array(
                candidate
                    .output_operations()
                    .iter()
                    .copied()
                    .map(output_op_json),
            ),
        ),
        (
            "output_equalities",
            json_array(
                candidate
                    .output_equality_relations()
                    .iter()
                    .copied()
                    .map(relation_json),
            ),
        ),
        ("packet_length", candidate.packet_length().to_string()),
        ("precondition", precondition_json(candidate.precondition())),
        (
            "procedure_id",
            candidate
                .procedure_id()
                .map(|id| json_string(&id.to_string()))
                .unwrap_or_else(|| "null".to_owned()),
        ),
        (
            "rejection_evidence_ids",
            ids_json(
                candidate
                    .rejection_evidence_ids()
                    .iter()
                    .map(|id| id.value()),
            ),
        ),
        (
            "rejection_reasons",
            json_array(
                candidate
                    .rejection_reasons()
                    .iter()
                    .copied()
                    .map(|reason| json_string(rejection_reason_name(reason))),
            ),
        ),
        ("run_scope", json_string(&candidate.run_scope().to_string())),
        ("scope", candidate_scope_json(candidate.scope())),
        ("signature", json_string(candidate.canonical_signature())),
        ("stage", json_string(candidate.stage().code())),
        (
            "support_evidence_ids",
            ids_json(candidate.support_evidence_ids().iter().map(|id| id.value())),
        ),
        (
            "unchanged_positions",
            positions_json(candidate.unchanged_positions()),
        ),
    ])
}

fn provenance_json(provenance: &PromotionProvenance) -> String {
    json_object(vec![
        (
            "challenge_evidence_ids",
            ids_json(
                provenance
                    .challenge_evidence_ids()
                    .iter()
                    .map(|id| id.value()),
            ),
        ),
        ("costs", candidate_costs_json(provenance.costs())),
        ("expansion", candidate_json(provenance.expansion())),
        (
            "rejection_evidence_ids",
            ids_json(
                provenance
                    .rejection_evidence_ids()
                    .iter()
                    .map(|id| id.value()),
            ),
        ),
        (
            "source_signature",
            json_string(provenance.source_signature()),
        ),
        (
            "support_evidence_ids",
            ids_json(
                provenance
                    .support_evidence_ids()
                    .iter()
                    .map(|id| id.value()),
            ),
        ),
    ])
}

fn procedure_version_json(version: &ProcedureVersion) -> String {
    match version {
        ProcedureVersion::Admitted(record) => json_object(vec![
            (
                "hypothesis_id",
                json_string(&record.hypothesis_id().to_string()),
            ),
            (
                "procedure_id",
                json_string(&record.procedure_id().to_string()),
            ),
            ("provenance", provenance_json(record.provenance())),
            ("scope", candidate_scope_json(record.scope())),
            (
                "status",
                json_string(procedure_status_name(record.status())),
            ),
            ("type", json_string("admitted")),
            ("version", record.version().to_string()),
        ]),
        ProcedureVersion::Revision(revision) => json_object(vec![
            (
                "contradiction_evidence_ids",
                ids_json(
                    revision
                        .contradiction_evidence_ids()
                        .iter()
                        .map(|id| id.value()),
                ),
            ),
            ("expansion", candidate_json(revision.expansion())),
            (
                "preserved_provenance",
                provenance_json(revision.preserved_provenance()),
            ),
            (
                "procedure_id",
                json_string(&revision.procedure_id().to_string()),
            ),
            (
                "status",
                json_string(procedure_status_name(revision.status())),
            ),
            ("type", json_string("revision")),
            ("version", revision.version().to_string()),
        ]),
    }
}

fn hypothesis_json(record: &HypothesisRecord) -> String {
    json_object(vec![
        ("candidate", candidate_json(record.candidate())),
        (
            "hypothesis_id",
            json_string(&record.hypothesis_id().to_string()),
        ),
        ("status", hypothesis_status_json(record.status())),
        (
            "transitions",
            json_array(record.transitions().iter().map(|transition| {
                json_object(vec![
                    ("from", hypothesis_status_json(transition.from())),
                    ("to", hypothesis_status_json(transition.to())),
                ])
            })),
        ),
    ])
}

fn dependency_json(source: DependencySource, dependent: DependentRecord) -> String {
    json_object(vec![
        ("dependent", dependent_json(dependent)),
        ("source", dependency_source_json(source)),
    ])
}

fn dependent_json(dependent: DependentRecord) -> String {
    match dependent {
        DependentRecord::Hypothesis(id) => json_object(vec![
            ("id", json_string(&id.to_string())),
            ("kind", json_string("hypothesis")),
        ]),
        DependentRecord::Procedure(id) => json_object(vec![
            ("id", json_string(&id.to_string())),
            ("kind", json_string("procedure")),
        ]),
        DependentRecord::Derived(id) => json_object(vec![
            ("id", json_string(&id.to_string())),
            ("kind", json_string("derived")),
        ]),
    }
}

fn dependency_source_json(source: DependencySource) -> String {
    match source {
        DependencySource::Evidence(id) => json_object(vec![
            ("id", json_string(&id.to_string())),
            ("kind", json_string("evidence")),
        ]),
        DependencySource::Hypothesis(id) => json_object(vec![
            ("id", json_string(&id.to_string())),
            ("kind", json_string("hypothesis")),
        ]),
        DependencySource::Procedure(id) => json_object(vec![
            ("id", json_string(&id.to_string())),
            ("kind", json_string("procedure")),
        ]),
    }
}

fn stores_json(stores: &LearnerStores) -> String {
    json_object(vec![
        (
            "dependencies",
            json_array(
                stores
                    .dependencies()
                    .links()
                    .map(|(source, dependent)| dependency_json(source, dependent)),
            ),
        ),
        (
            "evidence",
            json_array(stores.evidence().iter().map(evidence_json)),
        ),
        (
            "hypotheses",
            json_array(stores.hypotheses().iter().map(hypothesis_json)),
        ),
        (
            "procedures",
            json_array(
                stores
                    .procedures()
                    .iter()
                    .map(|record| procedure_version_json(record.version_record())),
            ),
        ),
        ("run_scope", json_string(&stores.run_scope().to_string())),
    ])
}

fn logical_counters_json(counters: LogicalCounters) -> String {
    counters.to_json()
}

fn resource_json(snapshot: &ResourcePersistenceSnapshot) -> String {
    let limits = json_object(
        LogicalResource::all()
            .into_iter()
            .map(|resource| {
                let limit = snapshot.limits().logical(resource);
                (
                    resource.name(),
                    json_object(vec![
                        ("per_decision", limit.per_decision().to_string()),
                        ("per_episode", limit.per_episode().to_string()),
                        ("per_run", limit.per_run().to_string()),
                    ]),
                )
            })
            .collect(),
    );
    let stages = json_object(
        Stage::all()
            .into_iter()
            .map(|stage| (stage.name(), snapshot.stage_operations(stage).to_string()))
            .collect(),
    );
    json_object(vec![
        (
            "decision",
            logical_counters_json(snapshot.decision_counters()),
        ),
        (
            "episode",
            logical_counters_json(snapshot.episode_counters()),
        ),
        ("limits", limits),
        ("run", logical_counters_json(snapshot.run_counters())),
        ("schema_version", "1".to_owned()),
        ("stages", stages),
    ])
}

fn act_mode_name(mode: ActMode) -> &'static str {
    match mode {
        ActMode::Exploration => "exploration",
        ActMode::Procedure => "procedure",
        ActMode::Fallback => "fallback",
    }
}

fn abstain_reason_name(reason: AbstainReason) -> &'static str {
    match reason {
        AbstainReason::NoApplicableProcedure => "no_applicable_procedure",
        AbstainReason::Conflict => "conflict",
        AbstainReason::InsufficientEvidence => "insufficient_evidence",
        AbstainReason::UnsupportedByGrammar => "unsupported_by_grammar",
        AbstainReason::IncompleteBudget => "incomplete_budget",
    }
}

fn provenance_name(provenance: EvidenceProvenance) -> &'static str {
    match provenance {
        EvidenceProvenance::Observed => "observed",
        EvidenceProvenance::Incomplete => "incomplete",
    }
}

fn terminal_name(status: TerminalStatus) -> &'static str {
    match status {
        TerminalStatus::Continue => "continue",
        TerminalStatus::Success => "success",
        TerminalStatus::Failure => "failure",
        TerminalStatus::Incomplete => "incomplete",
    }
}

fn procedure_status_name(status: ProcedureStatus) -> &'static str {
    match status {
        ProcedureStatus::Active => "active",
        ProcedureStatus::Suspended => "suspended",
    }
}

fn rejection_reason_name(reason: RejectionReason) -> &'static str {
    match reason {
        RejectionReason::UnsupportedByGrammar => "unsupported_by_grammar",
        RejectionReason::ActionMismatch => "action_mismatch",
        RejectionReason::UnstableEffect => "unstable_effect",
        RejectionReason::InsufficientSupport => "insufficient_support",
        RejectionReason::MissingContextVariation => "missing_context_variation",
        RejectionReason::OutputInconsistency => "output_inconsistency",
        RejectionReason::MissingStructuralTraceability => "missing_structural_traceability",
        RejectionReason::MissingChallenge => "missing_challenge",
        RejectionReason::IncorrectChallenge => "incorrect_challenge",
        RejectionReason::MissingGroundingEvidence => "missing_grounding_evidence",
        RejectionReason::ScopeConfinement => "scope_confinement",
        RejectionReason::CostCeiling => "cost_ceiling",
        RejectionReason::MissingEvidence => "missing_evidence",
        RejectionReason::Contradiction => "contradiction",
        RejectionReason::BudgetExhausted => "budget_exhausted",
    }
}

fn hypothesis_status_json(status: StoredHypothesisStatus) -> String {
    match status {
        StoredHypothesisStatus::Active => json_object(vec![("kind", json_string("active"))]),
        StoredHypothesisStatus::Rejected => json_object(vec![("kind", json_string("rejected"))]),
        StoredHypothesisStatus::Stale => json_object(vec![("kind", json_string("stale"))]),
        StoredHypothesisStatus::Unresolved => {
            json_object(vec![("kind", json_string("unresolved"))])
        }
        StoredHypothesisStatus::Contradicted(id) => json_object(vec![
            ("evidence_id", json_string(&id.to_string())),
            ("kind", json_string("contradicted")),
        ]),
    }
}

#[derive(Clone, Debug, PartialEq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

impl JsonValue {
    fn to_json(&self) -> String {
        match self {
            Self::Null => "null".to_owned(),
            Self::Bool(value) => value.to_string(),
            Self::Number(value) => value.clone(),
            Self::String(value) => json_string(value),
            Self::Array(values) => json_array(values.iter().map(Self::to_json)),
            Self::Object(values) => json_value_object(values),
        }
    }

    fn object_fields(self) -> Result<BTreeMap<String, Self>, PersistenceError> {
        match self {
            Self::Object(fields) => Ok(fields),
            _ => Err(PersistenceError::MalformedRecord(
                "expected object".to_owned(),
            )),
        }
    }

    fn get_string(&self, key: &str) -> Result<&str, PersistenceError> {
        match self.get(key)? {
            Self::String(value) => Ok(value),
            _ => Err(PersistenceError::MalformedRecord(format!(
                "{key} must be a string"
            ))),
        }
    }

    fn get_u64(&self, key: &str) -> Result<u64, PersistenceError> {
        match self.get(key)? {
            Self::Number(value) => value.parse::<u64>().map_err(|_| {
                PersistenceError::MalformedRecord(format!("{key} must be an unsigned integer"))
            }),
            Self::String(value) => value.parse::<u64>().map_err(|_| {
                PersistenceError::MalformedRecord(format!(
                    "{key} must be an unsigned integer string"
                ))
            }),
            _ => Err(PersistenceError::MalformedRecord(format!(
                "{key} must be an unsigned integer"
            ))),
        }
    }

    fn get(&self, key: &str) -> Result<&Self, PersistenceError> {
        match self {
            Self::Object(fields) => fields.get(key).ok_or_else(|| {
                PersistenceError::MalformedRecord(format!("missing required field {key}"))
            }),
            _ => Err(PersistenceError::MalformedRecord(
                "expected object while reading field".to_owned(),
            )),
        }
    }
}

struct JsonParser<'a> {
    input: &'a [u8],
    position: usize,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, position: 0 }
    }

    fn parse_document(mut self) -> Result<JsonValue, PersistenceError> {
        self.skip_whitespace();
        if self.position == self.input.len() {
            return Err(PersistenceError::IncompleteFile);
        }
        let value = self.parse_value()?;
        self.skip_whitespace();
        if self.position != self.input.len() {
            return Err(PersistenceError::MalformedRecord(
                "trailing data after JSON document".to_owned(),
            ));
        }
        Ok(value)
    }

    fn parse_value(&mut self) -> Result<JsonValue, PersistenceError> {
        self.skip_whitespace();
        let byte = self.current().ok_or(PersistenceError::IncompleteFile)?;
        match byte {
            b'n' => {
                self.expect_bytes(b"null")?;
                Ok(JsonValue::Null)
            }
            b't' => {
                self.expect_bytes(b"true")?;
                Ok(JsonValue::Bool(true))
            }
            b'f' => {
                self.expect_bytes(b"false")?;
                Ok(JsonValue::Bool(false))
            }
            b'"' => Ok(JsonValue::String(self.parse_string()?)),
            b'[' => self.parse_array(),
            b'{' => self.parse_object(),
            b'-' | b'0'..=b'9' => Ok(JsonValue::Number(self.parse_number()?)),
            _ => Err(PersistenceError::MalformedRecord(format!(
                "unexpected JSON byte {byte}"
            ))),
        }
    }

    fn parse_array(&mut self) -> Result<JsonValue, PersistenceError> {
        self.expect_byte(b'[')?;
        self.skip_whitespace();
        let mut values = Vec::new();
        if self.consume_byte(b']') {
            return Ok(JsonValue::Array(values));
        }
        loop {
            values.push(self.parse_value()?);
            self.skip_whitespace();
            if self.consume_byte(b']') {
                break;
            }
            self.expect_byte(b',')?;
        }
        Ok(JsonValue::Array(values))
    }

    fn parse_object(&mut self) -> Result<JsonValue, PersistenceError> {
        self.expect_byte(b'{')?;
        self.skip_whitespace();
        let mut fields = BTreeMap::new();
        if self.consume_byte(b'}') {
            return Ok(JsonValue::Object(fields));
        }
        loop {
            self.skip_whitespace();
            if self.current() != Some(b'"') {
                return Err(PersistenceError::MalformedRecord(
                    "object key must be a string".to_owned(),
                ));
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect_byte(b':')?;
            let value = self.parse_value()?;
            if fields.insert(key.clone(), value).is_some() {
                return Err(PersistenceError::MalformedRecord(format!(
                    "duplicate JSON object key {key}"
                )));
            }
            self.skip_whitespace();
            if self.consume_byte(b'}') {
                break;
            }
            self.expect_byte(b',')?;
        }
        Ok(JsonValue::Object(fields))
    }

    fn parse_string(&mut self) -> Result<String, PersistenceError> {
        self.expect_byte(b'"')?;
        let mut output = String::new();
        loop {
            let byte = self.current().ok_or(PersistenceError::IncompleteFile)?;
            self.position += 1;
            match byte {
                b'"' => return Ok(output),
                b'\\' => {
                    let escaped = self.current().ok_or(PersistenceError::IncompleteFile)?;
                    self.position += 1;
                    match escaped {
                        b'"' => output.push('"'),
                        b'\\' => output.push('\\'),
                        b'/' => output.push('/'),
                        b'b' => output.push('\u{0008}'),
                        b'f' => output.push('\u{000c}'),
                        b'n' => output.push('\n'),
                        b'r' => output.push('\r'),
                        b't' => output.push('\t'),
                        b'u' => {
                            let value = self.parse_unicode_escape()?;
                            let character = char::from_u32(value).ok_or_else(|| {
                                PersistenceError::MalformedRecord(
                                    "invalid Unicode escape".to_owned(),
                                )
                            })?;
                            output.push(character);
                        }
                        _ => {
                            return Err(PersistenceError::MalformedRecord(
                                "invalid JSON string escape".to_owned(),
                            ))
                        }
                    }
                }
                value if value < 0x20 => {
                    return Err(PersistenceError::MalformedRecord(
                        "unescaped control byte in JSON string".to_owned(),
                    ))
                }
                value if value < 0x80 => output.push(value as char),
                _value => {
                    let start = self.position - 1;
                    let character = std::str::from_utf8(&self.input[start..])
                        .map_err(|_| PersistenceError::MalformedRecord("invalid UTF-8".to_owned()))?
                        .chars()
                        .next()
                        .ok_or(PersistenceError::IncompleteFile)?;
                    output.push(character);
                    self.position = start + character.len_utf8();
                }
            }
        }
    }

    fn parse_unicode_escape(&mut self) -> Result<u32, PersistenceError> {
        let mut value = 0_u32;
        for _ in 0..4 {
            let byte = self.current().ok_or(PersistenceError::IncompleteFile)?;
            self.position += 1;
            let digit = match byte {
                b'0'..=b'9' => u32::from(byte - b'0'),
                b'a'..=b'f' => u32::from(byte - b'a' + 10),
                b'A'..=b'F' => u32::from(byte - b'A' + 10),
                _ => {
                    return Err(PersistenceError::MalformedRecord(
                        "invalid Unicode escape".to_owned(),
                    ))
                }
            };
            value = value * 16 + digit;
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<String, PersistenceError> {
        let start = self.position;
        self.consume_byte(b'-');
        match self.current() {
            Some(b'0') => {
                self.position += 1;
            }
            Some(b'1'..=b'9') => {
                self.position += 1;
                while matches!(self.current(), Some(b'0'..=b'9')) {
                    self.position += 1;
                }
            }
            _ => {
                return Err(PersistenceError::MalformedRecord(
                    "invalid JSON number".to_owned(),
                ))
            }
        }
        if self.consume_byte(b'.') {
            if !matches!(self.current(), Some(b'0'..=b'9')) {
                return Err(PersistenceError::MalformedRecord(
                    "invalid JSON fraction".to_owned(),
                ));
            }
            while matches!(self.current(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }
        if matches!(self.current(), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.current(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            if !matches!(self.current(), Some(b'0'..=b'9')) {
                return Err(PersistenceError::MalformedRecord(
                    "invalid JSON exponent".to_owned(),
                ));
            }
            while matches!(self.current(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }
        Ok(String::from_utf8(self.input[start..self.position].to_vec()).expect("number is ASCII"))
    }

    fn expect_bytes(&mut self, expected: &[u8]) -> Result<(), PersistenceError> {
        if self
            .input
            .get(self.position..self.position + expected.len())
            == Some(expected)
        {
            self.position += expected.len();
            Ok(())
        } else if self.position + expected.len() > self.input.len() {
            Err(PersistenceError::IncompleteFile)
        } else {
            Err(PersistenceError::MalformedRecord(
                "invalid JSON literal".to_owned(),
            ))
        }
    }

    fn expect_byte(&mut self, expected: u8) -> Result<(), PersistenceError> {
        match self.current() {
            Some(value) if value == expected => {
                self.position += 1;
                Ok(())
            }
            None => Err(PersistenceError::IncompleteFile),
            Some(value) => Err(PersistenceError::MalformedRecord(format!(
                "expected byte {expected}, got {value}"
            ))),
        }
    }

    fn consume_byte(&mut self, expected: u8) -> bool {
        if self.current() == Some(expected) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn current(&self) -> Option<u8> {
        self.input.get(self.position).copied()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.current(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.position += 1;
        }
    }
}

fn take_field(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<JsonValue, PersistenceError> {
    fields
        .remove(key)
        .ok_or_else(|| PersistenceError::MalformedRecord(format!("missing required field {key}")))
}

fn finish_fields(
    fields: BTreeMap<String, JsonValue>,
    context: &str,
) -> Result<(), PersistenceError> {
    if let Some((key, _)) = fields.into_iter().next() {
        return Err(PersistenceError::MalformedRecord(format!(
            "unknown field {key} in {context}"
        )));
    }
    Ok(())
}

fn as_object(
    value: JsonValue,
    context: &str,
) -> Result<BTreeMap<String, JsonValue>, PersistenceError> {
    value
        .object_fields()
        .map_err(|_| PersistenceError::MalformedRecord(format!("{context} must be an object")))
}

fn as_array(value: JsonValue, context: &str) -> Result<Vec<JsonValue>, PersistenceError> {
    match value {
        JsonValue::Array(values) => Ok(values),
        _ => Err(PersistenceError::MalformedRecord(format!(
            "{context} must be an array"
        ))),
    }
}

fn as_string(value: JsonValue, context: &str) -> Result<String, PersistenceError> {
    match value {
        JsonValue::String(value) => Ok(value),
        _ => Err(PersistenceError::MalformedRecord(format!(
            "{context} must be a string"
        ))),
    }
}

fn as_u64(value: JsonValue, context: &str) -> Result<u64, PersistenceError> {
    match value {
        JsonValue::Number(value) | JsonValue::String(value) => value.parse::<u64>().map_err(|_| {
            PersistenceError::MalformedRecord(format!("{context} must be an unsigned integer"))
        }),
        _ => Err(PersistenceError::MalformedRecord(format!(
            "{context} must be an unsigned integer"
        ))),
    }
}

fn as_bool(value: JsonValue, context: &str) -> Result<bool, PersistenceError> {
    match value {
        JsonValue::Bool(value) => Ok(value),
        _ => Err(PersistenceError::MalformedRecord(format!(
            "{context} must be a boolean"
        ))),
    }
}

fn as_null_or<T>(
    value: JsonValue,
    _context: &str,
    parser: impl FnOnce(JsonValue) -> Result<T, PersistenceError>,
) -> Result<Option<T>, PersistenceError> {
    match value {
        JsonValue::Null => Ok(None),
        other => parser(other).map(Some),
    }
}

fn parse_scope(value: JsonValue, context: &str) -> Result<CandidateScope, PersistenceError> {
    let mut fields = as_object(value, context)?;
    let kind = as_string(take_field(&mut fields, "kind")?, context)?;
    let token = as_u64(take_field(&mut fields, "token")?, context)?;
    finish_fields(fields, context)?;
    let token = ScopeToken::new(token);
    match kind.as_str() {
        "run" => Ok(CandidateScope::Run(token)),
        "episode" => Ok(CandidateScope::Episode(token)),
        _ => Err(PersistenceError::MalformedRecord(format!(
            "unknown candidate scope kind {kind}"
        ))),
    }
}

fn parse_audit_scope(value: JsonValue, context: &str) -> Result<AuditScope, PersistenceError> {
    let mut fields = as_object(value, context)?;
    let kind = as_string(take_field(&mut fields, "kind")?, context)?;
    let scope = match kind.as_str() {
        "run" => AuditScope::Run,
        "episode" => AuditScope::Episode(ScopeToken::new(as_u64(
            take_field(&mut fields, "token")?,
            context,
        )?)),
        _ => {
            return Err(PersistenceError::MalformedRecord(format!(
                "unknown audit scope kind {kind}"
            )))
        }
    };
    finish_fields(fields, context)?;
    Ok(scope)
}

fn parse_phase(value: JsonValue, context: &str) -> Result<RunPhase, PersistenceError> {
    match as_string(value, context)?.as_str() {
        "mutable" => Ok(RunPhase::Mutable),
        "frozen" => Ok(RunPhase::Frozen),
        other => Err(PersistenceError::MalformedRecord(format!(
            "unknown phase {other}"
        ))),
    }
}

fn parse_packet(value: JsonValue, context: &str) -> Result<ObservationPacket, PersistenceError> {
    let mut fields = as_object(value, context)?;
    let atoms = parse_id_values(take_field(&mut fields, "atoms")?, context, false)?;
    finish_fields(fields, context)?;
    if atoms.len() != 3 {
        return Err(PersistenceError::MalformedRecord(format!(
            "{context} must contain exactly three atoms"
        )));
    }
    Ok(ObservationPacket::new([
        AtomId::new(atoms[0]),
        AtomId::new(atoms[1]),
        AtomId::new(atoms[2]),
    ]))
}

fn parse_input(value: JsonValue) -> Result<StepInput, PersistenceError> {
    let mut fields = as_object(value, "input")?;
    let actions = parse_id_values(
        take_field(&mut fields, "available_actions")?,
        "available_actions",
        false,
    )?;
    if actions.len() != 2 {
        return Err(PersistenceError::MalformedRecord(
            "input must contain two available actions".to_owned(),
        ));
    }
    let episode_scope = ScopeToken::new(as_u64(
        take_field(&mut fields, "episode_scope")?,
        "input.episode_scope",
    )?);
    let observation = parse_packet(take_field(&mut fields, "observation")?, "input.observation")?;
    let previous_terminal = match as_string(
        take_field(&mut fields, "previous_terminal")?,
        "input.previous_terminal",
    )?
    .as_str()
    {
        "none" => PreviousTerminal::None,
        "continue" => PreviousTerminal::Continue,
        other => {
            return Err(PersistenceError::MalformedRecord(format!(
                "unknown previous terminal {other}"
            )))
        }
    };
    let remaining_action_budget = u8::try_from(as_u64(
        take_field(&mut fields, "remaining_action_budget")?,
        "input.remaining_action_budget",
    )?)
    .map_err(|_| PersistenceError::MalformedRecord("action budget exceeds u8".to_owned()))?;
    let run_scope = ScopeToken::new(as_u64(
        take_field(&mut fields, "run_scope")?,
        "input.run_scope",
    )?);
    let step_index = u8::try_from(as_u64(
        take_field(&mut fields, "step_index")?,
        "input.step_index",
    )?)
    .map_err(|_| PersistenceError::MalformedRecord("step index exceeds u8".to_owned()))?;
    finish_fields(fields, "input")?;
    StepInput::new(
        run_scope,
        episode_scope,
        step_index,
        observation,
        [ActionId::new(actions[0]), ActionId::new(actions[1])],
        remaining_action_budget,
        previous_terminal,
    )
    .map_err(|error| PersistenceError::MalformedRecord(error.to_string()))
}

fn parse_id_values(
    value: JsonValue,
    context: &str,
    require_sorted_unique: bool,
) -> Result<Vec<u64>, PersistenceError> {
    let values = as_array(value, context)?
        .into_iter()
        .map(|value| as_u64(value, context))
        .collect::<Result<Vec<_>, _>>()?;
    if require_sorted_unique && values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(PersistenceError::MalformedRecord(format!(
            "{context} is not strictly sorted"
        )));
    }
    Ok(values)
}

fn parse_audit(value: JsonValue) -> Result<AuditReferences, PersistenceError> {
    let mut fields = as_object(value, "audit")?;
    let evidence = parse_id_values(
        take_field(&mut fields, "evidence_refs")?,
        "audit.evidence_refs",
        true,
    )?;
    let hypotheses = parse_id_values(
        take_field(&mut fields, "hypothesis_refs")?,
        "audit.hypothesis_refs",
        true,
    )?;
    let procedures = parse_id_values(
        take_field(&mut fields, "procedure_refs")?,
        "audit.procedure_refs",
        true,
    )?;
    finish_fields(fields, "audit")?;
    AuditReferences::new(
        procedures.into_iter().map(ProcedureId::new).collect(),
        evidence.into_iter().map(EvidenceId::new).collect(),
        hypotheses.into_iter().map(HypothesisId::new).collect(),
    )
    .map_err(|error| PersistenceError::MalformedRecord(error.to_string()))
}

fn parse_decision(value: JsonValue) -> Result<Decision, PersistenceError> {
    let mut fields = as_object(value, "decision")?;
    let kind = as_string(take_field(&mut fields, "type")?, "decision.type")?;
    let audit = parse_audit(take_field(&mut fields, "audit")?)?;
    let decision = match kind.as_str() {
        "act" => {
            let action = ActionId::new(as_u64(
                take_field(&mut fields, "action")?,
                "decision.action",
            )?);
            let mode = match as_string(take_field(&mut fields, "mode")?, "decision.mode")?.as_str()
            {
                "exploration" => ActMode::Exploration,
                "procedure" => ActMode::Procedure,
                "fallback" => ActMode::Fallback,
                other => {
                    return Err(PersistenceError::MalformedRecord(format!(
                        "unknown action mode {other}"
                    )))
                }
            };
            let prediction = as_null_or(
                take_field(&mut fields, "prediction")?,
                "decision.prediction",
                |value| parse_packet(value, "decision.prediction"),
            )?;
            Decision::act(action, prediction, mode, audit)
                .map_err(|error| PersistenceError::MalformedRecord(error.to_string()))?
        }
        "abstain" => {
            let reason =
                match as_string(take_field(&mut fields, "reason")?, "decision.reason")?.as_str() {
                    "no_applicable_procedure" => AbstainReason::NoApplicableProcedure,
                    "conflict" => AbstainReason::Conflict,
                    "insufficient_evidence" => AbstainReason::InsufficientEvidence,
                    "unsupported_by_grammar" => AbstainReason::UnsupportedByGrammar,
                    "incomplete_budget" => AbstainReason::IncompleteBudget,
                    other => {
                        return Err(PersistenceError::MalformedRecord(format!(
                            "unknown abstention reason {other}"
                        )))
                    }
                };
            Decision::abstain(reason, audit)
        }
        other => {
            return Err(PersistenceError::MalformedRecord(format!(
                "unknown decision type {other}"
            )))
        }
    };
    finish_fields(fields, "decision")?;
    Ok(decision)
}

fn parse_decision_record(value: JsonValue) -> Result<DecisionRecord, PersistenceError> {
    let mut fields = as_object(value, "decision record")?;
    let decision = parse_decision(take_field(&mut fields, "decision")?)?;
    let input = parse_input(take_field(&mut fields, "input")?)?;
    let phase = parse_phase(take_field(&mut fields, "phase")?, "decision.phase")?;
    let sequence = as_u64(take_field(&mut fields, "sequence")?, "decision.sequence")?;
    finish_fields(fields, "decision record")?;
    decision
        .validate_for(&input)
        .map_err(|error| PersistenceError::MalformedRecord(error.to_string()))?;
    Ok(DecisionRecord::from_persisted(
        sequence, input, decision, phase,
    ))
}

fn parse_evidence(value: JsonValue) -> Result<EvidenceRecord, PersistenceError> {
    let mut fields = as_object(value, "evidence")?;
    let action = as_null_or(
        take_field(&mut fields, "action")?,
        "evidence.action",
        |value| Ok(ActionId::new(as_u64(value, "evidence.action")?)),
    )?;
    let after = as_null_or(
        take_field(&mut fields, "after")?,
        "evidence.after",
        |value| parse_packet(value, "evidence.after"),
    )?;
    let before = parse_packet(take_field(&mut fields, "before")?, "evidence.before")?;
    let decision_sequence = as_u64(
        take_field(&mut fields, "decision_sequence")?,
        "evidence.decision_sequence",
    )?;
    let episode_scope = ScopeToken::new(as_u64(
        take_field(&mut fields, "episode_scope")?,
        "evidence.episode_scope",
    )?);
    let evidence_id = EvidenceId::new(as_u64(
        take_field(&mut fields, "evidence_id")?,
        "evidence.evidence_id",
    )?);
    let provenance = match as_string(
        take_field(&mut fields, "provenance")?,
        "evidence.provenance",
    )?
    .as_str()
    {
        "observed" => EvidenceProvenance::Observed,
        "incomplete" => EvidenceProvenance::Incomplete,
        other => {
            return Err(PersistenceError::MalformedRecord(format!(
                "unknown evidence provenance {other}"
            )))
        }
    };
    let run_scope = ScopeToken::new(as_u64(
        take_field(&mut fields, "run_scope")?,
        "evidence.run_scope",
    )?);
    let step_index = u8::try_from(as_u64(
        take_field(&mut fields, "step_index")?,
        "evidence.step_index",
    )?)
    .map_err(|_| PersistenceError::MalformedRecord("evidence step index exceeds u8".to_owned()))?;
    let terminal =
        match as_string(take_field(&mut fields, "terminal")?, "evidence.terminal")?.as_str() {
            "continue" => TerminalStatus::Continue,
            "success" => TerminalStatus::Success,
            "failure" => TerminalStatus::Failure,
            "incomplete" => TerminalStatus::Incomplete,
            other => {
                return Err(PersistenceError::MalformedRecord(format!(
                    "unknown terminal status {other}"
                )))
            }
        };
    finish_fields(fields, "evidence")?;
    Ok(EvidenceRecord::from_persisted(
        evidence_id,
        decision_sequence,
        run_scope,
        episode_scope,
        step_index,
        before,
        action,
        after,
        terminal,
        provenance,
    ))
}

fn parse_interaction(value: JsonValue) -> Result<InteractionRecord, PersistenceError> {
    let mut fields = as_object(value, "interaction")?;
    let decision = parse_decision_record(take_field(&mut fields, "decision")?)?;
    let evidence = parse_evidence(take_field(&mut fields, "evidence")?)?;
    finish_fields(fields, "interaction")?;
    Ok(InteractionRecord::from_persisted(decision, evidence))
}

fn parse_stage(value: JsonValue) -> Result<CandidateStage, PersistenceError> {
    match as_string(value, "candidate.stage")?.as_str() {
        "E0" => Ok(CandidateStage::E0),
        "E1" => Ok(CandidateStage::E1),
        "E2" => Ok(CandidateStage::E2),
        "E3" => Ok(CandidateStage::E3),
        "E4" => Ok(CandidateStage::E4),
        other => Err(PersistenceError::MalformedRecord(format!(
            "unknown candidate stage {other}"
        ))),
    }
}

fn parse_candidate_costs(value: JsonValue) -> Result<CandidateCosts, PersistenceError> {
    let mut fields = as_object(value, "candidate.costs")?;
    let challenge = as_u64(
        take_field(&mut fields, "challenge_comparisons")?,
        "candidate.costs.challenge_comparisons",
    )?;
    let generation = as_u64(
        take_field(&mut fields, "generation_attempts")?,
        "candidate.costs.generation_attempts",
    )?;
    let operation = as_u64(
        take_field(&mut fields, "operation_steps")?,
        "candidate.costs.operation_steps",
    )?;
    let structural = as_u64(
        take_field(&mut fields, "structural_comparisons")?,
        "candidate.costs.structural_comparisons",
    )?;
    finish_fields(fields, "candidate.costs")?;
    Ok(CandidateCosts::new(
        generation, structural, operation, challenge,
    ))
}

fn parse_positions(value: JsonValue, context: &str) -> Result<Vec<usize>, PersistenceError> {
    let values = parse_id_values(value, context, true)?
        .into_iter()
        .map(|value| {
            usize::try_from(value).map_err(|_| {
                PersistenceError::MalformedRecord(format!("{context} position exceeds usize"))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values.iter().any(|position| *position >= 3) {
        return Err(PersistenceError::MalformedRecord(format!(
            "{context} position is outside the packet"
        )));
    }
    Ok(values)
}

fn parse_relations(
    value: JsonValue,
    context: &str,
) -> Result<Vec<EqualityRelation>, PersistenceError> {
    let values = as_array(value, context)?;
    let mut relations = Vec::with_capacity(values.len());
    for value in values {
        let values = parse_id_values(value, context, false)?;
        if values.len() != 2 || values[0] >= values[1] || values[1] >= 3 {
            return Err(PersistenceError::MalformedRecord(format!(
                "{context} contains an invalid equality relation"
            )));
        }
        relations.push(EqualityRelation::new(
            usize::try_from(values[0]).expect("bounded relation"),
            usize::try_from(values[1]).expect("bounded relation"),
        ));
    }
    if relations.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(PersistenceError::MalformedRecord(format!(
            "{context} is not strictly sorted"
        )));
    }
    Ok(relations)
}

fn parse_pattern(value: JsonValue) -> Result<InputPattern, PersistenceError> {
    let values = as_array(value, "candidate.input_pattern")?;
    if values.len() != 3 {
        return Err(PersistenceError::MalformedRecord(
            "candidate input pattern must have three atoms".to_owned(),
        ));
    }
    let mut atoms = [PatternAtom::Constant(AtomId::new(0)); 3];
    for (index, value) in values.into_iter().enumerate() {
        let mut fields = as_object(value, "candidate.input_pattern atom")?;
        if fields.contains_key("constant") && fields.contains_key("variable") {
            return Err(PersistenceError::MalformedRecord(
                "pattern atom has two variants".to_owned(),
            ));
        }
        let atom = if let Some(value) = fields.remove("constant") {
            PatternAtom::Constant(AtomId::new(as_u64(value, "pattern constant")?))
        } else if let Some(value) = fields.remove("variable") {
            let id = u8::try_from(as_u64(value, "pattern variable")?).map_err(|_| {
                PersistenceError::MalformedRecord("variable id exceeds u8".to_owned())
            })?;
            if id != VariableId::ZERO.value() {
                return Err(PersistenceError::MalformedRecord(
                    "unsupported nonzero variable id".to_owned(),
                ));
            }
            PatternAtom::Variable(VariableId::new(id))
        } else {
            return Err(PersistenceError::MalformedRecord(
                "pattern atom has no variant".to_owned(),
            ));
        };
        finish_fields(fields, "candidate.input_pattern atom")?;
        atoms[index] = atom;
    }
    Ok(InputPattern::new(atoms))
}

fn parse_output(value: JsonValue) -> Result<[OutputOp; 3], PersistenceError> {
    let values = as_array(value, "candidate.output")?;
    if values.len() != 3 {
        return Err(PersistenceError::MalformedRecord(
            "candidate output must have three operations".to_owned(),
        ));
    }
    let mut output = [OutputOp::Copy(0); 3];
    for (index, value) in values.into_iter().enumerate() {
        let mut fields = as_object(value, "candidate.output operation")?;
        let operation = if let Some(value) = fields.remove("copy") {
            let position = usize::try_from(as_u64(value, "output copy")?).map_err(|_| {
                PersistenceError::MalformedRecord("output position exceeds usize".to_owned())
            })?;
            if position >= 3 {
                return Err(PersistenceError::MalformedRecord(
                    "output copy position is outside the packet".to_owned(),
                ));
            }
            OutputOp::Copy(position)
        } else if let Some(value) = fields.remove("observed") {
            OutputOp::Observed(AtomId::new(as_u64(value, "output observed")?))
        } else {
            return Err(PersistenceError::MalformedRecord(
                "output operation has no variant".to_owned(),
            ));
        };
        finish_fields(fields, "candidate.output operation")?;
        output[index] = operation;
    }
    Ok(output)
}

fn parse_precondition(value: JsonValue) -> Result<Vec<(usize, AtomId)>, PersistenceError> {
    let values = as_array(value, "candidate.precondition")?;
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        let mut fields = as_object(value, "candidate.precondition entry")?;
        let atom = AtomId::new(as_u64(
            take_field(&mut fields, "atom")?,
            "precondition atom",
        )?);
        let position = usize::try_from(as_u64(
            take_field(&mut fields, "position")?,
            "precondition position",
        )?)
        .map_err(|_| {
            PersistenceError::MalformedRecord("precondition position exceeds usize".to_owned())
        })?;
        if position >= 3 {
            return Err(PersistenceError::MalformedRecord(
                "precondition position is outside the packet".to_owned(),
            ));
        }
        finish_fields(fields, "candidate.precondition entry")?;
        output.push((position, atom));
    }
    if output.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
        return Err(PersistenceError::MalformedRecord(
            "candidate precondition is not strictly sorted".to_owned(),
        ));
    }
    Ok(output)
}

fn parse_rejection_reason(value: JsonValue) -> Result<RejectionReason, PersistenceError> {
    let value = as_string(value, "candidate.rejection_reason")?;
    let reason = match value.as_str() {
        "unsupported_by_grammar" => RejectionReason::UnsupportedByGrammar,
        "action_mismatch" => RejectionReason::ActionMismatch,
        "unstable_effect" => RejectionReason::UnstableEffect,
        "insufficient_support" => RejectionReason::InsufficientSupport,
        "missing_context_variation" => RejectionReason::MissingContextVariation,
        "output_inconsistency" => RejectionReason::OutputInconsistency,
        "missing_structural_traceability" => RejectionReason::MissingStructuralTraceability,
        "missing_challenge" => RejectionReason::MissingChallenge,
        "incorrect_challenge" => RejectionReason::IncorrectChallenge,
        "missing_grounding_evidence" => RejectionReason::MissingGroundingEvidence,
        "scope_confinement" => RejectionReason::ScopeConfinement,
        "cost_ceiling" => RejectionReason::CostCeiling,
        "missing_evidence" => RejectionReason::MissingEvidence,
        "contradiction" => RejectionReason::Contradiction,
        "budget_exhausted" => RejectionReason::BudgetExhausted,
        other => {
            return Err(PersistenceError::MalformedRecord(format!(
                "unknown rejection reason {other}"
            )))
        }
    };
    Ok(reason)
}

fn parse_candidate(value: JsonValue) -> Result<Candidate, PersistenceError> {
    let mut fields = as_object(value, "candidate")?;
    let action = ActionId::new(as_u64(
        take_field(&mut fields, "action")?,
        "candidate.action",
    )?);
    let changed = parse_positions(
        take_field(&mut fields, "changed_positions")?,
        "candidate.changed_positions",
    )?;
    let challenge = parse_id_values(
        take_field(&mut fields, "challenge_evidence_ids")?,
        "candidate.challenge_evidence_ids",
        true,
    )?
    .into_iter()
    .map(EvidenceId::new)
    .collect::<Vec<_>>();
    let costs = parse_candidate_costs(take_field(&mut fields, "costs")?)?;
    let grounding = parse_id_values(
        take_field(&mut fields, "grounding_evidence_ids")?,
        "candidate.grounding_evidence_ids",
        true,
    )?
    .into_iter()
    .map(EvidenceId::new)
    .collect::<Vec<_>>();
    let hypothesis_id = HypothesisId::new(as_u64(
        take_field(&mut fields, "hypothesis_id")?,
        "candidate.hypothesis_id",
    )?);
    let input_equalities = parse_relations(
        take_field(&mut fields, "input_equalities")?,
        "candidate.input_equalities",
    )?;
    let input = parse_pattern(take_field(&mut fields, "input_pattern")?)?;
    let output = parse_output(take_field(&mut fields, "output")?)?;
    let output_equalities = parse_relations(
        take_field(&mut fields, "output_equalities")?,
        "candidate.output_equalities",
    )?;
    let packet_length = usize::try_from(as_u64(
        take_field(&mut fields, "packet_length")?,
        "candidate.packet_length",
    )?)
    .map_err(|_| {
        PersistenceError::MalformedRecord("candidate packet length exceeds usize".to_owned())
    })?;
    let precondition = parse_precondition(take_field(&mut fields, "precondition")?)?;
    let procedure_id = as_null_or(
        take_field(&mut fields, "procedure_id")?,
        "candidate.procedure_id",
        |value| Ok(ProcedureId::new(as_u64(value, "candidate.procedure_id")?)),
    )?;
    let rejection_evidence = parse_id_values(
        take_field(&mut fields, "rejection_evidence_ids")?,
        "candidate.rejection_evidence_ids",
        true,
    )?
    .into_iter()
    .map(EvidenceId::new)
    .collect::<Vec<_>>();
    let rejection_reasons = as_array(
        take_field(&mut fields, "rejection_reasons")?,
        "candidate.rejection_reasons",
    )?
    .into_iter()
    .map(parse_rejection_reason)
    .collect::<Result<Vec<_>, _>>()?;
    if rejection_reasons.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(PersistenceError::MalformedRecord(
            "candidate rejection reasons are not strictly sorted".to_owned(),
        ));
    }
    let run_scope = ScopeToken::new(as_u64(
        take_field(&mut fields, "run_scope")?,
        "candidate.run_scope",
    )?);
    let scope = parse_scope(take_field(&mut fields, "scope")?, "candidate.scope")?;
    let signature = as_string(take_field(&mut fields, "signature")?, "candidate.signature")?;
    let stage = parse_stage(take_field(&mut fields, "stage")?)?;
    let support = parse_id_values(
        take_field(&mut fields, "support_evidence_ids")?,
        "candidate.support_evidence_ids",
        true,
    )?
    .into_iter()
    .map(EvidenceId::new)
    .collect::<Vec<_>>();
    let unchanged = parse_positions(
        take_field(&mut fields, "unchanged_positions")?,
        "candidate.unchanged_positions",
    )?;
    finish_fields(fields, "candidate")?;
    Candidate::from_persisted(
        stage,
        run_scope,
        scope,
        action,
        packet_length,
        input,
        output,
        unchanged,
        changed,
        input_equalities,
        output_equalities,
        precondition,
        support,
        challenge,
        grounding,
        rejection_evidence,
        hypothesis_id,
        procedure_id,
        rejection_reasons,
        costs,
        signature,
    )
    .ok_or_else(|| {
        PersistenceError::MalformedRecord("candidate canonical signature mismatch".to_owned())
    })
}

fn parse_procedure_status(value: JsonValue) -> Result<ProcedureStatus, PersistenceError> {
    match as_string(value, "procedure.status")?.as_str() {
        "active" => Ok(ProcedureStatus::Active),
        "suspended" => Ok(ProcedureStatus::Suspended),
        other => Err(PersistenceError::MalformedRecord(format!(
            "unknown procedure status {other}"
        ))),
    }
}

fn parse_provenance(value: JsonValue) -> Result<PromotionProvenance, PersistenceError> {
    let mut fields = as_object(value, "provenance")?;
    let challenge = parse_id_values(
        take_field(&mut fields, "challenge_evidence_ids")?,
        "provenance.challenge_evidence_ids",
        true,
    )?
    .into_iter()
    .map(EvidenceId::new)
    .collect();
    let costs = parse_candidate_costs(take_field(&mut fields, "costs")?)?;
    let expansion = parse_candidate(take_field(&mut fields, "expansion")?)?;
    let rejection = parse_id_values(
        take_field(&mut fields, "rejection_evidence_ids")?,
        "provenance.rejection_evidence_ids",
        true,
    )?
    .into_iter()
    .map(EvidenceId::new)
    .collect();
    let source_signature = as_string(
        take_field(&mut fields, "source_signature")?,
        "provenance.source_signature",
    )?;
    let support = parse_id_values(
        take_field(&mut fields, "support_evidence_ids")?,
        "provenance.support_evidence_ids",
        true,
    )?
    .into_iter()
    .map(EvidenceId::new)
    .collect();
    finish_fields(fields, "provenance")?;
    Ok(PromotionProvenance::from_persisted(
        source_signature,
        expansion,
        support,
        challenge,
        rejection,
        costs,
    ))
}

fn parse_procedure_version(value: JsonValue) -> Result<ProcedureVersion, PersistenceError> {
    let mut fields = as_object(value, "procedure version")?;
    let kind = as_string(take_field(&mut fields, "type")?, "procedure.type")?;
    let version = u32::try_from(as_u64(
        take_field(&mut fields, "version")?,
        "procedure.version",
    )?)
    .map_err(|_| PersistenceError::MalformedRecord("procedure version exceeds u32".to_owned()))?;
    let result = match kind.as_str() {
        "admitted" => {
            let hypothesis_id = HypothesisId::new(as_u64(
                take_field(&mut fields, "hypothesis_id")?,
                "procedure.hypothesis_id",
            )?);
            let procedure_id = ProcedureId::new(as_u64(
                take_field(&mut fields, "procedure_id")?,
                "procedure.procedure_id",
            )?);
            let provenance = parse_provenance(take_field(&mut fields, "provenance")?)?;
            let scope = parse_scope(take_field(&mut fields, "scope")?, "procedure.scope")?;
            let status = parse_procedure_status(take_field(&mut fields, "status")?)?;
            ProcedureVersion::from(ProcedureRecord::from_persisted(
                procedure_id,
                version,
                status,
                scope,
                hypothesis_id,
                provenance,
            ))
        }
        "revision" => {
            let contradiction = parse_id_values(
                take_field(&mut fields, "contradiction_evidence_ids")?,
                "procedure.contradiction_evidence_ids",
                true,
            )?
            .into_iter()
            .map(EvidenceId::new)
            .collect();
            let expansion = parse_candidate(take_field(&mut fields, "expansion")?)?;
            let preserved = parse_provenance(take_field(&mut fields, "preserved_provenance")?)?;
            let procedure_id = ProcedureId::new(as_u64(
                take_field(&mut fields, "procedure_id")?,
                "procedure.procedure_id",
            )?);
            let status = parse_procedure_status(take_field(&mut fields, "status")?)?;
            ProcedureVersion::from(ProcedureRevision::from_persisted(
                procedure_id,
                version,
                status,
                expansion,
                contradiction,
                preserved,
            ))
        }
        other => {
            return Err(PersistenceError::MalformedRecord(format!(
                "unknown procedure version type {other}"
            )))
        }
    };
    finish_fields(fields, "procedure version")?;
    Ok(result)
}

fn parse_hypothesis_status(value: JsonValue) -> Result<StoredHypothesisStatus, PersistenceError> {
    let mut fields = as_object(value, "hypothesis status")?;
    let kind = as_string(take_field(&mut fields, "kind")?, "hypothesis status kind")?;
    let status = match kind.as_str() {
        "active" => StoredHypothesisStatus::Active,
        "rejected" => StoredHypothesisStatus::Rejected,
        "stale" => StoredHypothesisStatus::Stale,
        "unresolved" => StoredHypothesisStatus::Unresolved,
        "contradicted" => StoredHypothesisStatus::Contradicted(EvidenceId::new(as_u64(
            take_field(&mut fields, "evidence_id")?,
            "hypothesis status evidence_id",
        )?)),
        other => {
            return Err(PersistenceError::MalformedRecord(format!(
                "unknown hypothesis status {other}"
            )))
        }
    };
    finish_fields(fields, "hypothesis status")?;
    Ok(status)
}

fn parse_hypothesis(
    value: JsonValue,
) -> Result<(HypothesisRecord, Vec<HypothesisTransition>), PersistenceError> {
    let mut fields = as_object(value, "hypothesis")?;
    let candidate = parse_candidate(take_field(&mut fields, "candidate")?)?;
    let hypothesis_id = HypothesisId::new(as_u64(
        take_field(&mut fields, "hypothesis_id")?,
        "hypothesis.hypothesis_id",
    )?);
    let status = parse_hypothesis_status(take_field(&mut fields, "status")?)?;
    let transitions = as_array(
        take_field(&mut fields, "transitions")?,
        "hypothesis.transitions",
    )?
    .into_iter()
    .map(|value| {
        let mut fields = as_object(value, "hypothesis transition")?;
        let from = parse_hypothesis_status(take_field(&mut fields, "from")?)?;
        let to = parse_hypothesis_status(take_field(&mut fields, "to")?)?;
        finish_fields(fields, "hypothesis transition")?;
        Ok(HypothesisTransition::from_persisted(from, to))
    })
    .collect::<Result<Vec<_>, PersistenceError>>()?;
    finish_fields(fields, "hypothesis")?;
    if hypothesis_id != candidate.hypothesis_id() {
        return Err(PersistenceError::MismatchedReference(
            "hypothesis record id differs from candidate id".to_owned(),
        ));
    }
    Ok((
        HypothesisRecord::from_persisted(candidate, status, transitions.clone()),
        transitions,
    ))
}

fn parse_dependency_source(value: JsonValue) -> Result<DependencySource, PersistenceError> {
    let mut fields = as_object(value, "dependency source")?;
    let id = as_u64(take_field(&mut fields, "id")?, "dependency source id")?;
    let kind = as_string(take_field(&mut fields, "kind")?, "dependency source kind")?;
    finish_fields(fields, "dependency source")?;
    match kind.as_str() {
        "evidence" => Ok(DependencySource::Evidence(EvidenceId::new(id))),
        "hypothesis" => Ok(DependencySource::Hypothesis(HypothesisId::new(id))),
        "procedure" => Ok(DependencySource::Procedure(ProcedureId::new(id))),
        other => Err(PersistenceError::MalformedRecord(format!(
            "unknown dependency source kind {other}"
        ))),
    }
}

fn parse_dependent(value: JsonValue) -> Result<DependentRecord, PersistenceError> {
    let mut fields = as_object(value, "dependent record")?;
    let id = as_u64(take_field(&mut fields, "id")?, "dependent id")?;
    let kind = as_string(take_field(&mut fields, "kind")?, "dependent kind")?;
    finish_fields(fields, "dependent record")?;
    match kind.as_str() {
        "hypothesis" => Ok(DependentRecord::Hypothesis(HypothesisId::new(id))),
        "procedure" => Ok(DependentRecord::Procedure(ProcedureId::new(id))),
        "derived" => Ok(DependentRecord::Derived(id)),
        other => Err(PersistenceError::MalformedRecord(format!(
            "unknown dependent kind {other}"
        ))),
    }
}

fn parse_dependency(
    value: JsonValue,
) -> Result<(DependencySource, DependentRecord), PersistenceError> {
    let mut fields = as_object(value, "dependency")?;
    let dependent = parse_dependent(take_field(&mut fields, "dependent")?)?;
    let source = parse_dependency_source(take_field(&mut fields, "source")?)?;
    finish_fields(fields, "dependency")?;
    Ok((source, dependent))
}

struct ParsedLedger {
    run_scope: ScopeToken,
    phase: RunPhase,
    decisions: Vec<DecisionRecord>,
    interactions: Vec<InteractionRecord>,
    decision_trace_jsonl: String,
    known_procedures: Vec<(ProcedureId, AuditScope, bool)>,
    known_evidence: Vec<EvidenceId>,
    known_hypotheses: Vec<(HypothesisId, AuditScope)>,
}

struct ParsedStores {
    run_scope: ScopeToken,
    evidence: Vec<EvidenceRecord>,
    hypotheses: Vec<(HypothesisRecord, Vec<HypothesisTransition>)>,
    procedures: Vec<ProcedureVersion>,
    dependencies: Vec<(DependencySource, DependentRecord)>,
}

fn parse_ledger(value: JsonValue) -> Result<ParsedLedger, PersistenceError> {
    let mut fields = as_object(value, "ledger")?;
    let decision_trace_jsonl = as_string(
        take_field(&mut fields, "decision_trace_jsonl")?,
        "ledger.decision_trace_jsonl",
    )?;
    let decisions = as_array(take_field(&mut fields, "decisions")?, "ledger.decisions")?
        .into_iter()
        .map(parse_decision_record)
        .collect::<Result<Vec<_>, _>>()?;
    let interactions = as_array(
        take_field(&mut fields, "interactions")?,
        "ledger.interactions",
    )?
    .into_iter()
    .map(parse_interaction)
    .collect::<Result<Vec<_>, _>>()?;
    let known_evidence = parse_id_values(
        take_field(&mut fields, "known_evidence")?,
        "ledger.known_evidence",
        true,
    )?
    .into_iter()
    .map(EvidenceId::new)
    .collect::<Vec<_>>();
    let known_hypotheses = as_array(
        take_field(&mut fields, "known_hypotheses")?,
        "ledger.known_hypotheses",
    )?
    .into_iter()
    .map(|value| {
        let mut fields = as_object(value, "known hypothesis")?;
        let id = HypothesisId::new(as_u64(
            take_field(&mut fields, "id")?,
            "known hypothesis id",
        )?);
        let scope = parse_audit_scope(take_field(&mut fields, "scope")?, "known hypothesis scope")?;
        finish_fields(fields, "known hypothesis")?;
        Ok((id, scope))
    })
    .collect::<Result<Vec<_>, PersistenceError>>()?;
    let known_procedures = as_array(
        take_field(&mut fields, "known_procedures")?,
        "ledger.known_procedures",
    )?
    .into_iter()
    .map(|value| {
        let mut fields = as_object(value, "known procedure")?;
        let checked = as_bool(
            take_field(&mut fields, "checked")?,
            "known procedure checked",
        )?;
        let id = ProcedureId::new(as_u64(
            take_field(&mut fields, "id")?,
            "known procedure id",
        )?);
        let scope = parse_audit_scope(take_field(&mut fields, "scope")?, "known procedure scope")?;
        finish_fields(fields, "known procedure")?;
        Ok((id, scope, checked))
    })
    .collect::<Result<Vec<_>, PersistenceError>>()?;
    let phase = parse_phase(take_field(&mut fields, "phase")?, "ledger.phase")?;
    let run_scope = ScopeToken::new(as_u64(
        take_field(&mut fields, "run_scope")?,
        "ledger.run_scope",
    )?);
    finish_fields(fields, "ledger")?;
    ensure_sorted_unique_by(&known_evidence, |id| id.value(), "ledger.known_evidence")?;
    ensure_sorted_unique_by(
        &known_hypotheses,
        |(id, _)| id.value(),
        "ledger.known_hypotheses",
    )?;
    ensure_sorted_unique_by(
        &known_procedures,
        |(id, _, _)| id.value(),
        "ledger.known_procedures",
    )?;
    Ok(ParsedLedger {
        run_scope,
        phase,
        decisions,
        interactions,
        decision_trace_jsonl,
        known_procedures,
        known_evidence,
        known_hypotheses,
    })
}

fn parse_stores(value: JsonValue) -> Result<ParsedStores, PersistenceError> {
    let mut fields = as_object(value, "stores")?;
    let dependencies = as_array(
        take_field(&mut fields, "dependencies")?,
        "stores.dependencies",
    )?
    .into_iter()
    .map(parse_dependency)
    .collect::<Result<Vec<_>, _>>()?;
    let evidence = as_array(take_field(&mut fields, "evidence")?, "stores.evidence")?
        .into_iter()
        .map(parse_evidence)
        .collect::<Result<Vec<_>, _>>()?;
    let hypotheses = as_array(take_field(&mut fields, "hypotheses")?, "stores.hypotheses")?
        .into_iter()
        .map(parse_hypothesis)
        .collect::<Result<Vec<_>, _>>()?;
    let procedures = as_array(take_field(&mut fields, "procedures")?, "stores.procedures")?
        .into_iter()
        .map(parse_procedure_version)
        .collect::<Result<Vec<_>, _>>()?;
    let run_scope = ScopeToken::new(as_u64(
        take_field(&mut fields, "run_scope")?,
        "stores.run_scope",
    )?);
    finish_fields(fields, "stores")?;
    ensure_sorted_unique_by(
        &evidence,
        |record| record.evidence_id().value(),
        "stores.evidence",
    )?;
    ensure_sorted_unique_by(
        &hypotheses,
        |(record, _)| record.hypothesis_id().value(),
        "stores.hypotheses",
    )?;
    ensure_sorted_unique_by(
        &procedures,
        |version| (version.procedure_id().value(), version.version()),
        "stores.procedures",
    )?;
    ensure_sorted_unique_by(&dependencies, |link| *link, "stores.dependencies")?;
    Ok(ParsedStores {
        run_scope,
        evidence,
        hypotheses,
        procedures,
        dependencies,
    })
}

fn ensure_sorted_unique_by<T, K: Ord>(
    values: &[T],
    key: impl Fn(&T) -> K,
    context: &str,
) -> Result<(), PersistenceError> {
    if values.windows(2).any(|pair| key(&pair[0]) >= key(&pair[1])) {
        return Err(PersistenceError::MalformedRecord(format!(
            "{context} is not strictly sorted"
        )));
    }
    Ok(())
}

fn parse_logical_counters(
    value: JsonValue,
    context: &str,
) -> Result<LogicalCounters, PersistenceError> {
    let mut fields = as_object(value, context)?;
    let active = as_u64(take_field(&mut fields, "active_hypotheses")?, context)?;
    let candidates = as_u64(
        take_field(&mut fields, "candidate_generation_attempts")?,
        context,
    )?;
    let actions = as_u64(take_field(&mut fields, "environment_actions")?, context)?;
    let ledger_records = as_u64(take_field(&mut fields, "ledger_record_count")?, context)?;
    let ledger_size = as_u64(take_field(&mut fields, "ledger_size_bytes")?, context)?;
    let operation_steps = as_u64(take_field(&mut fields, "operation_steps")?, context)?;
    let procedures = as_u64(take_field(&mut fields, "procedure_count")?, context)?;
    let queries = as_u64(take_field(&mut fields, "query_calls")?, context)?;
    let retained_interactions = as_u64(
        take_field(&mut fields, "retained_interaction_records")?,
        context,
    )?;
    let retained_procedures = as_u64(
        take_field(&mut fields, "retained_procedure_records")?,
        context,
    )?;
    let comparisons = as_u64(take_field(&mut fields, "structural_comparisons")?, context)?;
    finish_fields(fields, context)?;
    if ledger_records != retained_interactions || procedures != retained_procedures {
        return Err(PersistenceError::MalformedRecord(format!(
            "inconsistent aliases in {context}"
        )));
    }
    Ok(LogicalCounters::from_values(
        queries,
        actions,
        candidates,
        comparisons,
        active,
        operation_steps,
        retained_interactions,
        retained_procedures,
        ledger_size,
    ))
}

fn parse_resource_limit(
    value: JsonValue,
    context: &str,
) -> Result<ResourceLimit, PersistenceError> {
    let mut fields = as_object(value, context)?;
    let per_decision = as_u64(take_field(&mut fields, "per_decision")?, context)?;
    let per_episode = as_u64(take_field(&mut fields, "per_episode")?, context)?;
    let per_run = as_u64(take_field(&mut fields, "per_run")?, context)?;
    finish_fields(fields, context)?;
    Ok(ResourceLimit::new(per_decision, per_episode, per_run))
}

fn parse_resources(value: JsonValue) -> Result<ResourcePersistenceSnapshot, PersistenceError> {
    let mut fields = as_object(value, "resources")?;
    if as_u64(
        take_field(&mut fields, "schema_version")?,
        "resources.schema_version",
    )? != 1
    {
        return Err(PersistenceError::MalformedRecord(
            "unsupported resource snapshot version".to_owned(),
        ));
    }
    let decision =
        parse_logical_counters(take_field(&mut fields, "decision")?, "resources.decision")?;
    let episode = parse_logical_counters(take_field(&mut fields, "episode")?, "resources.episode")?;
    let mut limit_fields = as_object(take_field(&mut fields, "limits")?, "resources.limits")?;
    let mut limits = [ResourceLimit::new(0, 0, 0); 8];
    for resource in LogicalResource::all() {
        limits[resource as usize] = parse_resource_limit(
            take_field(&mut limit_fields, resource.name())?,
            resource.name(),
        )?;
    }
    finish_fields(limit_fields, "resources.limits")?;
    let run = parse_logical_counters(take_field(&mut fields, "run")?, "resources.run")?;
    let mut stage_fields = as_object(take_field(&mut fields, "stages")?, "resources.stages")?;
    let mut stage_operations = [0_u64; 6];
    for stage in Stage::all() {
        stage_operations[stage as usize] =
            as_u64(take_field(&mut stage_fields, stage.name())?, stage.name())?;
    }
    finish_fields(stage_fields, "resources.stages")?;
    finish_fields(fields, "resources")?;
    Ok(ResourcePersistenceSnapshot::from_parts(
        ProtocolResourceLimits::from_logical(limits),
        run,
        decision,
        episode,
        stage_operations,
    ))
}

fn parse_learner(value: JsonValue) -> Result<(ScopeToken, u64, bool), PersistenceError> {
    let mut fields = as_object(value, "learner")?;
    let decision_count = as_u64(
        take_field(&mut fields, "decision_count")?,
        "learner.decision_count",
    )?;
    let frozen = as_bool(take_field(&mut fields, "frozen")?, "learner.frozen")?;
    let run_scope = ScopeToken::new(as_u64(
        take_field(&mut fields, "run_scope")?,
        "learner.run_scope",
    )?);
    finish_fields(fields, "learner")?;
    Ok((run_scope, decision_count, frozen))
}

fn restore_state(value: JsonValue) -> Result<FrameworkState, PersistenceError> {
    let mut fields = as_object(value, "state")?;
    if !as_bool(take_field(&mut fields, "complete")?, "complete")? {
        return Err(PersistenceError::IncompleteFile);
    }
    if as_string(
        take_field(&mut fields, "grammar_version")?,
        "grammar_version",
    )? != V2_GRAMMAR_VERSION
        || as_string(
            take_field(&mut fields, "protocol_version")?,
            "protocol_version",
        )? != V2_PROTOCOL_VERSION
    {
        return Err(PersistenceError::MalformedRecord(
            "unsupported grammar/protocol version".to_owned(),
        ));
    }
    let format = as_string(take_field(&mut fields, "format")?, "format")?;
    if format != PERSISTENCE_FORMAT {
        return Err(PersistenceError::UnsupportedFormat(format));
    }
    let learner = parse_learner(take_field(&mut fields, "learner")?)?;
    let ledger = parse_ledger(take_field(&mut fields, "ledger")?)?;
    let phase = parse_phase(take_field(&mut fields, "phase")?, "phase")?;
    let resources = parse_resources(take_field(&mut fields, "resources")?)?;
    let run_scope = ScopeToken::new(as_u64(take_field(&mut fields, "run_scope")?, "run_scope")?);
    let schema = as_u64(take_field(&mut fields, "schema_version")?, "schema_version")?;
    if schema != u64::from(PERSISTENCE_SCHEMA_VERSION) {
        return Err(PersistenceError::UnsupportedSchemaVersion(schema));
    }
    let stores = parse_stores(take_field(&mut fields, "stores")?)?;
    let _fingerprint = as_string(take_field(&mut fields, "fingerprint")?, "fingerprint")?;
    finish_fields(fields, "state")?;

    if run_scope.is_zero() {
        return Err(PersistenceError::ScopeViolation(
            "run scope is zero".to_owned(),
        ));
    }
    if learner.0 != run_scope || ledger.run_scope != run_scope || stores.run_scope != run_scope {
        return Err(PersistenceError::ScopeViolation(
            "top-level and component run scopes differ".to_owned(),
        ));
    }
    if learner.2 != matches!(phase, RunPhase::Frozen) || ledger.phase != phase {
        return Err(PersistenceError::MalformedRecord(
            "persisted phase metadata disagree".to_owned(),
        ));
    }
    validate_parsed_ledger(&ledger, run_scope)?;
    validate_resource_snapshot(&resources)?;

    let mut restored_stores = LearnerStores::new(run_scope)
        .map_err(|error| PersistenceError::Store(error.to_string()))?;
    for record in &stores.evidence {
        restored_stores
            .register_evidence(record.clone())
            .map_err(store_error)?;
    }
    for (record, transitions) in &stores.hypotheses {
        let initial = transitions
            .first()
            .map(|transition| transition.from())
            .unwrap_or(record.status());
        validate_hypothesis_history(record, transitions)?;
        restored_stores
            .restore_hypothesis(record.candidate().clone(), initial, transitions)
            .map_err(map_store_restore_error)?;
    }
    for version in &stores.procedures {
        restored_stores
            .restore_procedure_version(version.clone())
            .map_err(map_store_restore_error)?;
    }
    validate_dependencies(&restored_stores, &stores.dependencies)?;
    for (source, dependent) in &stores.dependencies {
        restored_stores.restore_dependency(*source, *dependent);
    }

    let meter = ResourceMeter::from_persistence(resources);
    let restored_ledger = InteractionLedger::from_persisted(
        run_scope,
        phase,
        ledger.decisions,
        ledger.interactions,
        ledger.known_procedures,
        ledger.known_evidence.into_iter().collect(),
        ledger.known_hypotheses,
        meter,
    );
    if restored_ledger.decision_trace_jsonl() != ledger.decision_trace_jsonl {
        return Err(PersistenceError::MalformedRecord(
            "ledger decision trace does not match decisions".to_owned(),
        ));
    }
    let restored_learner = LearnerState::from_persisted(run_scope, learner.1, learner.2)
        .map_err(|error| PersistenceError::MalformedRecord(error.to_string()))?;
    let state = FrameworkState {
        learner: restored_learner,
        ledger: restored_ledger,
        stores: restored_stores,
    };
    state.validate_consistency()?;
    Ok(state)
}

fn map_store_restore_error(error: StoreError) -> PersistenceError {
    match error {
        StoreError::DuplicateId { entity, id } => PersistenceError::DuplicateId {
            entity: format!("{entity:?}"),
            id: id.to_string(),
        },
        StoreError::DuplicateVersion {
            procedure_id,
            version,
        } => PersistenceError::DuplicateVersion {
            procedure_id: procedure_id.to_string(),
            version,
        },
        StoreError::InvalidTransition { hypothesis_id, .. } => {
            PersistenceError::InvalidStatusTransition {
                hypothesis_id: hypothesis_id.to_string(),
            }
        }
        other => PersistenceError::Store(other.to_string()),
    }
}

fn store_error(error: StoreError) -> PersistenceError {
    map_store_restore_error(error)
}

fn validate_ledger(ledger: &InteractionLedger) -> Result<(), PersistenceError> {
    let parsed = ParsedLedger {
        run_scope: ledger.run_scope(),
        phase: ledger.phase(),
        decisions: ledger.decisions().to_vec(),
        interactions: ledger.interactions().to_vec(),
        decision_trace_jsonl: ledger.decision_trace_jsonl().to_owned(),
        known_procedures: ledger.known_procedures(),
        known_evidence: ledger.known_evidence(),
        known_hypotheses: ledger.known_hypotheses(),
    };
    validate_parsed_ledger(&parsed, ledger.run_scope())
}

fn validate_parsed_ledger(
    ledger: &ParsedLedger,
    run_scope: ScopeToken,
) -> Result<(), PersistenceError> {
    if run_scope.is_zero() || ledger.run_scope != run_scope {
        return Err(PersistenceError::ScopeViolation(
            "ledger run scope is invalid".to_owned(),
        ));
    }
    if ledger.decisions.len() != ledger.interactions.len() {
        return Err(PersistenceError::MalformedRecord(
            "a saved state cannot contain an unresolved decision".to_owned(),
        ));
    }
    let mut previous_phase = RunPhase::Mutable;
    let mut used_episodes = BTreeSet::new();
    let mut evidence_ids = BTreeSet::new();
    let mut continuation = None;
    for (index, (decision, interaction)) in ledger
        .decisions
        .iter()
        .zip(&ledger.interactions)
        .enumerate()
    {
        let expected_sequence = u64::try_from(index + 1).map_err(|_| {
            PersistenceError::MalformedRecord("decision sequence exceeds u64".to_owned())
        })?;
        if decision.sequence() != expected_sequence
            || interaction.decision() != decision
            || interaction.evidence().decision_sequence() != expected_sequence
            || interaction.evidence().evidence_id().value() != expected_sequence
        {
            return Err(PersistenceError::MalformedRecord(format!(
                "ledger sequence {expected_sequence} is not contiguous"
            )));
        }
        if decision.input().run_scope() != run_scope
            || interaction.evidence().run_scope() != run_scope
            || decision.input().episode_scope().is_zero()
            || interaction.evidence().episode_scope() != decision.input().episode_scope()
        {
            return Err(PersistenceError::ScopeViolation(format!(
                "ledger decision {expected_sequence} crosses the run scope"
            )));
        }
        if matches!(
            (previous_phase, decision.phase()),
            (RunPhase::Frozen, RunPhase::Mutable)
        ) {
            return Err(PersistenceError::MalformedRecord(
                "ledger phase changed from frozen back to mutable".to_owned(),
            ));
        }
        if decision.phase() == RunPhase::Frozen
            && (ledger.phase != RunPhase::Frozen
                || (previous_phase == RunPhase::Mutable && continuation.is_some()))
        {
            return Err(PersistenceError::MalformedRecord(
                "invalid freeze boundary in ledger history".to_owned(),
            ));
        }
        previous_phase = decision.phase();
        decision
            .decision()
            .validate_for(&decision.input())
            .map_err(|error| PersistenceError::MalformedRecord(error.to_string()))?;
        validate_decision_references(
            decision,
            &ledger.known_procedures,
            &ledger.known_evidence,
            &ledger.known_hypotheses,
        )?;

        let input = decision.input();
        if let Some(expected) = continuation {
            if input != expected {
                return Err(PersistenceError::MalformedRecord(format!(
                    "ledger continuation mismatch at decision {expected_sequence}"
                )));
            }
        } else if input.step_index() != 0 || !used_episodes.insert(input.episode_scope()) {
            return Err(PersistenceError::MalformedRecord(format!(
                "ledger episode position is invalid at decision {expected_sequence}"
            )));
        }

        let evidence = interaction.evidence();
        if !evidence_ids.insert(evidence.evidence_id()) {
            return Err(PersistenceError::DuplicateId {
                entity: "ledger evidence".to_owned(),
                id: evidence.evidence_id().to_string(),
            });
        }
        if evidence.before() != input.observation()
            || evidence.action() != decision.decision().action()
            || evidence.step_index() != input.step_index()
        {
            return Err(PersistenceError::MismatchedReference(format!(
                "evidence {} does not match its decision",
                evidence.evidence_id()
            )));
        }
        match (
            decision.decision().action(),
            evidence.after(),
            evidence.provenance(),
            evidence.terminal(),
        ) {
            (None, None, EvidenceProvenance::Incomplete, TerminalStatus::Incomplete) => {
                continuation = None;
            }
            (Some(_), Some(after), EvidenceProvenance::Observed, terminal)
                if terminal.is_environment_result() =>
            {
                if matches!(
                    (input.step_index(), terminal),
                    (0, TerminalStatus::Failure) | (1, TerminalStatus::Continue)
                ) {
                    return Err(PersistenceError::MalformedRecord(
                        "invalid terminal timing in persisted evidence".to_owned(),
                    ));
                }
                continuation = if terminal == TerminalStatus::Continue {
                    Some(
                        StepInput::new(
                            input.run_scope(),
                            input.episode_scope(),
                            1,
                            after,
                            input.available_actions(),
                            1,
                            PreviousTerminal::Continue,
                        )
                        .map_err(|error| PersistenceError::MalformedRecord(error.to_string()))?,
                    )
                } else {
                    None
                };
            }
            _ => {
                return Err(PersistenceError::MalformedRecord(format!(
                    "evidence {} has an invalid action/result combination",
                    evidence.evidence_id()
                )))
            }
        }
    }
    let expected_trace = ledger
        .decisions
        .iter()
        .map(DecisionRecord::to_json)
        .map(|record| format!("{record}\n"))
        .collect::<String>();
    if ledger.decision_trace_jsonl != expected_trace {
        return Err(PersistenceError::MalformedRecord(
            "decision trace is not canonical".to_owned(),
        ));
    }
    ensure_sorted_unique_by(
        &ledger.known_evidence,
        |id| id.value(),
        "ledger.known_evidence",
    )?;
    ensure_sorted_unique_by(
        &ledger.known_hypotheses,
        |(id, _)| id.value(),
        "ledger.known_hypotheses",
    )?;
    ensure_sorted_unique_by(
        &ledger.known_procedures,
        |(id, _, _)| id.value(),
        "ledger.known_procedures",
    )?;
    let mutable_evidence = ledger
        .interactions
        .iter()
        .filter(|record| record.decision().phase() == RunPhase::Mutable)
        .map(|record| record.evidence().evidence_id())
        .collect::<Vec<_>>();
    if ledger.known_evidence != mutable_evidence {
        return Err(PersistenceError::MismatchedReference(
            "known evidence must contain exactly the mutable ledger evidence".to_owned(),
        ));
    }
    for (_, scope, _) in &ledger.known_procedures {
        validate_audit_scope(*scope, &used_episodes, "known procedure")?;
    }
    for (_, scope) in &ledger.known_hypotheses {
        validate_audit_scope(*scope, &used_episodes, "known hypothesis")?;
    }
    Ok(())
}

fn validate_audit_scope(
    scope: AuditScope,
    used_episodes: &BTreeSet<ScopeToken>,
    context: &str,
) -> Result<(), PersistenceError> {
    match scope {
        AuditScope::Run => Ok(()),
        AuditScope::Episode(token) if !token.is_zero() && used_episodes.contains(&token) => Ok(()),
        AuditScope::Episode(token) => Err(PersistenceError::ScopeViolation(format!(
            "{context} references unknown episode {token}"
        ))),
    }
}

fn validate_decision_references(
    decision: &DecisionRecord,
    procedures: &[(ProcedureId, AuditScope, bool)],
    evidence: &[EvidenceId],
    hypotheses: &[(HypothesisId, AuditScope)],
) -> Result<(), PersistenceError> {
    let episode = decision.episode_scope();
    let audit = decision.decision().audit();
    for id in audit.procedure_refs() {
        let (_, scope, checked) = procedures
            .iter()
            .find(|(known, _, _)| known == id)
            .ok_or_else(|| {
                PersistenceError::MismatchedReference(format!("unknown procedure reference {id}"))
            })?;
        if !audit_scope_matches(*scope, episode) {
            return Err(PersistenceError::ScopeViolation(format!(
                "procedure reference {id} is outside the episode"
            )));
        }
        if decision.decision().prediction().is_some() && !checked {
            return Err(PersistenceError::MalformedRecord(
                "prediction uses an unchecked procedure".to_owned(),
            ));
        }
        if matches!(decision.decision(), Decision::Act(act)
            if act.mode() == ActMode::Procedure && *checked && act.prediction().is_none())
        {
            return Err(PersistenceError::MalformedRecord(
                "checked procedure action requires a prediction".to_owned(),
            ));
        }
    }
    for id in audit.evidence_refs() {
        if !evidence.contains(id) || id.value() >= decision.sequence() {
            return Err(PersistenceError::MismatchedReference(format!(
                "unknown evidence reference {id}"
            )));
        }
    }
    for id in audit.hypothesis_refs() {
        let (_, scope) = hypotheses
            .iter()
            .find(|(known, _)| known == id)
            .ok_or_else(|| {
                PersistenceError::MismatchedReference(format!("unknown hypothesis reference {id}"))
            })?;
        if !audit_scope_matches(*scope, episode) {
            return Err(PersistenceError::ScopeViolation(format!(
                "hypothesis reference {id} is outside the episode"
            )));
        }
    }
    Ok(())
}

fn audit_scope_matches(scope: AuditScope, episode: ScopeToken) -> bool {
    matches!(scope, AuditScope::Run)
        || matches!(scope, AuditScope::Episode(token) if token == episode)
}

fn valid_hypothesis_transition(from: StoredHypothesisStatus, to: StoredHypothesisStatus) -> bool {
    if from == to {
        return false;
    }
    match from {
        StoredHypothesisStatus::Active | StoredHypothesisStatus::Unresolved => true,
        StoredHypothesisStatus::Stale => !matches!(to, StoredHypothesisStatus::Stale),
        StoredHypothesisStatus::Rejected | StoredHypothesisStatus::Contradicted(_) => false,
    }
}

fn validate_hypothesis_history(
    record: &HypothesisRecord,
    transitions: &[HypothesisTransition],
) -> Result<(), PersistenceError> {
    let mut current = transitions
        .first()
        .map(|transition| transition.from())
        .unwrap_or(record.status());
    for transition in transitions {
        if transition.from() != current || !valid_hypothesis_transition(current, transition.to()) {
            return Err(PersistenceError::InvalidStatusTransition {
                hypothesis_id: record.hypothesis_id().to_string(),
            });
        }
        current = transition.to();
    }
    if current != record.status() {
        return Err(PersistenceError::InvalidStatusTransition {
            hypothesis_id: record.hypothesis_id().to_string(),
        });
    }
    Ok(())
}

fn validate_stores(stores: &LearnerStores, run_scope: ScopeToken) -> Result<(), PersistenceError> {
    if stores.run_scope() != run_scope || run_scope.is_zero() {
        return Err(PersistenceError::ScopeViolation(
            "store run scope is invalid".to_owned(),
        ));
    }
    let evidence_ids = stores
        .evidence()
        .iter()
        .map(|record| record.evidence_id())
        .collect::<BTreeSet<_>>();
    for record in stores.evidence().iter() {
        if record.run_scope() != run_scope
            || record.episode_scope().is_zero()
            || record.evidence_id().is_zero()
        {
            return Err(PersistenceError::ScopeViolation(format!(
                "evidence {} is outside the run",
                record.evidence_id()
            )));
        }
    }
    let hypothesis_ids = stores
        .hypotheses()
        .iter()
        .map(|record| record.hypothesis_id())
        .collect::<BTreeSet<_>>();
    for record in stores.hypotheses().iter() {
        validate_candidate(record.candidate(), run_scope, &evidence_ids)?;
        validate_candidate_evidence_scope(record.candidate(), stores)?;
        if let StoredHypothesisStatus::Contradicted(evidence) = record.status() {
            stores
                .evidence()
                .register_reference(evidence, record.scope())
                .map_err(map_store_restore_error)?;
        }
        validate_hypothesis_history(record, record.transitions())?;
    }
    let mut versions = BTreeMap::<ProcedureId, u32>::new();
    for record in stores.procedures().iter() {
        validate_candidate(record.expansion(), run_scope, &evidence_ids)?;
        stores
            .evidence()
            .validate_references(record.evidence_refs(), record.scope())
            .map_err(map_store_restore_error)?;
        if record.scope() != record.expansion().scope()
            || record.hypothesis_id() != record.expansion().hypothesis_id()
            || record.expansion().procedure_id() != Some(record.procedure_id())
            || record.scope().token().is_zero()
            || record.expansion().run_scope() != run_scope
            || !hypothesis_ids.contains(&record.hypothesis_id())
        {
            return Err(PersistenceError::MismatchedReference(format!(
                "procedure {} references a missing or mismatched hypothesis",
                record.procedure_id()
            )));
        }
        match record.version_record() {
            ProcedureVersion::Admitted(record) => {
                let source = stores
                    .hypotheses()
                    .get(record.hypothesis_id())
                    .map_err(map_store_restore_error)?;
                if record.scope() != source.scope()
                    || !record.matches_persisted_source(source.candidate())
                {
                    return Err(PersistenceError::MismatchedReference(
                        "procedure expansion differs from source hypothesis".to_owned(),
                    ));
                }
                if record.provenance().expansion() != record.expansion()
                    || record.provenance().support_evidence_ids()
                        != record.expansion().support_evidence_ids()
                    || record.provenance().challenge_evidence_ids()
                        != record.expansion().challenge_evidence_ids()
                    || record.provenance().rejection_evidence_ids()
                        != record.expansion().rejection_evidence_ids()
                    || record.provenance().costs() != record.expansion().costs()
                {
                    return Err(PersistenceError::MismatchedReference(format!(
                        "procedure {} provenance does not match its expansion",
                        record.procedure_id()
                    )));
                }
            }
            ProcedureVersion::Revision(revision) => {
                let original = stores
                    .procedures()
                    .get_version(revision.procedure_id(), 1, record.scope())
                    .map_err(map_store_restore_error)?;
                if !matches!(original.version_record(), ProcedureVersion::Admitted(original) if revision.matches_persisted_original(original))
                {
                    return Err(PersistenceError::MismatchedReference(
                        "revision did not preserve the original procedure".to_owned(),
                    ));
                }
                if revision.preserved_provenance().expansion().run_scope() != run_scope {
                    return Err(PersistenceError::ScopeViolation(format!(
                        "procedure {} preserved provenance is outside the run",
                        revision.procedure_id()
                    )));
                }
            }
        }
        let previous = versions.insert(record.procedure_id(), record.version());
        if let Some(previous) = previous {
            if previous.checked_add(1) != Some(record.version()) {
                return Err(PersistenceError::DuplicateVersion {
                    procedure_id: record.procedure_id().to_string(),
                    version: record.version(),
                });
            }
        } else if record.version() != 1 {
            return Err(PersistenceError::MalformedRecord(format!(
                "procedure {} does not begin at version one",
                record.procedure_id()
            )));
        }
        if matches!(record.version_record(), ProcedureVersion::Admitted(_))
            && (record.version() != 1 || record.status() != ProcedureStatus::Active)
        {
            return Err(PersistenceError::MalformedRecord(
                "admitted procedure version is not active version one".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_candidate(
    candidate: &Candidate,
    run_scope: ScopeToken,
    evidence_ids: &BTreeSet<EvidenceId>,
) -> Result<(), PersistenceError> {
    if candidate.run_scope() != run_scope
        || candidate.scope().token().is_zero()
        || candidate.hypothesis_id().is_zero()
        || candidate.packet_length() != 3
    {
        return Err(PersistenceError::ScopeViolation(format!(
            "candidate {} is outside the run",
            candidate.hypothesis_id()
        )));
    }
    if matches!(candidate.scope(), CandidateScope::Run(token) if token != run_scope) {
        return Err(PersistenceError::ScopeViolation(format!(
            "candidate {} uses another run scope",
            candidate.hypothesis_id()
        )));
    }
    for evidence in candidate
        .support_evidence_ids()
        .iter()
        .chain(candidate.challenge_evidence_ids())
        .chain(candidate.grounding_evidence_ids())
        .chain(candidate.rejection_evidence_ids())
    {
        if !evidence_ids.contains(evidence) {
            return Err(PersistenceError::MismatchedReference(format!(
                "candidate {} references missing evidence {}",
                candidate.hypothesis_id(),
                evidence
            )));
        }
    }
    Ok(())
}

fn validate_candidate_evidence_scope(
    candidate: &Candidate,
    stores: &LearnerStores,
) -> Result<(), PersistenceError> {
    let ids = candidate
        .support_evidence_ids()
        .iter()
        .chain(candidate.challenge_evidence_ids())
        .chain(candidate.grounding_evidence_ids())
        .chain(candidate.rejection_evidence_ids())
        .copied()
        .collect::<Vec<_>>();
    stores
        .evidence()
        .validate_references(&ids, candidate.scope())
        .map_err(map_store_restore_error)
}

fn validate_cross_references(
    stores: &LearnerStores,
    ledger: &InteractionLedger,
) -> Result<(), PersistenceError> {
    let interactions = ledger
        .interactions()
        .iter()
        .map(|record| (record.evidence().evidence_id(), record))
        .collect::<BTreeMap<_, _>>();
    for evidence in stores.evidence().iter() {
        if interactions
            .get(&evidence.evidence_id())
            .map(|record| record.evidence())
            != Some(evidence)
        {
            return Err(PersistenceError::MismatchedReference(format!(
                "store evidence {} differs from its ledger record",
                evidence.evidence_id()
            )));
        }
    }
    let validate_learning_evidence = |id: EvidenceId| -> Result<(), PersistenceError> {
        if interactions
            .get(&id)
            .is_none_or(|record| record.decision().phase() != RunPhase::Mutable)
        {
            return Err(PersistenceError::MismatchedReference(format!(
                "learning reference {id} is not mutable-phase evidence"
            )));
        }
        Ok(())
    };
    for hypothesis in stores.hypotheses().iter() {
        let candidate = hypothesis.candidate();
        for id in candidate
            .support_evidence_ids()
            .iter()
            .chain(candidate.challenge_evidence_ids())
            .chain(candidate.grounding_evidence_ids())
            .chain(candidate.rejection_evidence_ids())
        {
            validate_learning_evidence(*id)?;
        }
        for status in std::iter::once(hypothesis.status()).chain(
            hypothesis
                .transitions()
                .iter()
                .flat_map(|transition| [transition.from(), transition.to()]),
        ) {
            if let StoredHypothesisStatus::Contradicted(id) = status {
                stores
                    .evidence()
                    .register_reference(id, hypothesis.scope())
                    .map_err(map_store_restore_error)?;
                validate_learning_evidence(id)?;
            }
        }
    }
    for procedure in stores.procedures().iter() {
        for id in procedure.evidence_refs() {
            validate_learning_evidence(*id)?;
        }
    }
    for (id, scope) in ledger.known_hypotheses() {
        let record = stores
            .hypotheses()
            .get(id)
            .map_err(map_store_restore_error)?;
        if candidate_audit_scope(record.scope()) != scope {
            return Err(PersistenceError::ScopeViolation(format!(
                "registered hypothesis {id} scope differs"
            )));
        }
    }
    for (id, scope, _checked) in ledger.known_procedures() {
        let record = stores
            .procedures()
            .iter()
            .find(|record| record.procedure_id() == id)
            .ok_or_else(|| {
                PersistenceError::MismatchedReference(format!(
                    "registered procedure {id} is missing"
                ))
            })?;
        if candidate_audit_scope(record.scope()) != scope {
            return Err(PersistenceError::MismatchedReference(format!(
                "registered procedure {id} scope/checks differ"
            )));
        }
    }
    let counters = ledger.resource_meter().counters();
    let actions = ledger
        .decisions()
        .iter()
        .filter(|record| record.decision().action().is_some())
        .count() as u64;
    if counters.query_calls() != ledger.decision_count() as u64
        || counters.environment_actions() != actions
    {
        return Err(PersistenceError::MismatchedReference(
            "ledger resource totals disagree with history".to_owned(),
        ));
    }
    for (source, dependent) in stores.dependencies().links() {
        let source_scope = match source {
            DependencySource::Evidence(id) => {
                let evidence = stores.evidence().get(id).map_err(map_store_restore_error)?;
                CandidateScope::Episode(evidence.episode_scope())
            }
            DependencySource::Hypothesis(id) => stores
                .hypotheses()
                .get(id)
                .map_err(map_store_restore_error)?
                .scope(),
            DependencySource::Procedure(id) => stores
                .procedures()
                .iter()
                .find(|record| record.procedure_id() == id)
                .ok_or_else(|| {
                    PersistenceError::MismatchedReference(format!("missing source procedure {id}"))
                })?
                .scope(),
        };
        let dependent_scope = match dependent {
            DependentRecord::Hypothesis(id) => Some(
                stores
                    .hypotheses()
                    .get(id)
                    .map_err(map_store_restore_error)?
                    .scope(),
            ),
            DependentRecord::Procedure(id) => Some(
                stores
                    .procedures()
                    .iter()
                    .find(|record| record.procedure_id() == id)
                    .ok_or_else(|| {
                        PersistenceError::MismatchedReference(format!(
                            "missing dependent procedure {id}"
                        ))
                    })?
                    .scope(),
            ),
            DependentRecord::Derived(_) => None,
        };
        // Run-scoped knowledge may draw on multiple episodes. Episode-local
        // hypotheses/procedures cannot reuse another episode's assumptions.
        // Raw evidence supports run-scoped derivations and explicit derived
        // decisions across episodes; its opaque source is still retained.
        if !matches!(source, DependencySource::Evidence(_))
            && matches!((source_scope, dependent_scope), (CandidateScope::Episode(a), Some(CandidateScope::Episode(b))) if a != b)
        {
            return Err(PersistenceError::ScopeViolation(
                "dependency crosses episode scopes".to_owned(),
            ));
        }
    }
    Ok(())
}

fn candidate_audit_scope(scope: CandidateScope) -> AuditScope {
    match scope {
        CandidateScope::Run(_) => AuditScope::Run,
        CandidateScope::Episode(token) => AuditScope::Episode(token),
    }
}

fn validate_dependencies(
    stores: &LearnerStores,
    links: &[(DependencySource, DependentRecord)],
) -> Result<(), PersistenceError> {
    let mut expected = BTreeSet::new();
    for record in stores.hypotheses().iter() {
        let candidate = record.candidate();
        for id in candidate
            .support_evidence_ids()
            .iter()
            .chain(candidate.challenge_evidence_ids())
            .chain(candidate.grounding_evidence_ids())
            .chain(candidate.rejection_evidence_ids())
        {
            expected.insert((
                DependencySource::Evidence(*id),
                DependentRecord::Hypothesis(record.hypothesis_id()),
            ));
        }
        for transition in record.transitions() {
            if let StoredHypothesisStatus::Contradicted(id) = transition.to() {
                expected.insert((
                    DependencySource::Evidence(id),
                    DependentRecord::Hypothesis(record.hypothesis_id()),
                ));
            }
        }
    }
    for record in stores.procedures().iter() {
        for id in record.evidence_refs() {
            expected.insert((
                DependencySource::Evidence(*id),
                DependentRecord::Procedure(record.procedure_id()),
            ));
        }
        expected.insert((
            DependencySource::Hypothesis(record.hypothesis_id()),
            DependentRecord::Procedure(record.procedure_id()),
        ));
    }
    let actual = links
        .iter()
        .filter(|(_, dependent)| !matches!(dependent, DependentRecord::Derived(_)))
        .copied()
        .collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(PersistenceError::MismatchedReference(
            "dependency links disagree with knowledge references".to_owned(),
        ));
    }
    let evidence_ids = stores
        .evidence()
        .iter()
        .map(|record| record.evidence_id())
        .collect::<BTreeSet<_>>();
    let hypothesis_ids = stores
        .hypotheses()
        .iter()
        .map(|record| record.hypothesis_id())
        .collect::<BTreeSet<_>>();
    let procedure_ids = stores
        .procedures()
        .iter()
        .map(|record| record.procedure_id())
        .collect::<BTreeSet<_>>();
    for (source, dependent) in links {
        let source_exists = match source {
            DependencySource::Evidence(id) => evidence_ids.contains(id),
            DependencySource::Hypothesis(id) => hypothesis_ids.contains(id),
            DependencySource::Procedure(id) => procedure_ids.contains(id),
        };
        let dependent_exists = match dependent {
            DependentRecord::Hypothesis(id) => hypothesis_ids.contains(id),
            DependentRecord::Procedure(id) => procedure_ids.contains(id),
            DependentRecord::Derived(_) => true,
        };
        if !source_exists || !dependent_exists {
            return Err(PersistenceError::MismatchedReference(format!(
                "dependency link {:?} -> {:?} references a missing record",
                source, dependent
            )));
        }
    }
    Ok(())
}

fn counter_value(counters: LogicalCounters, resource: LogicalResource) -> u64 {
    match resource {
        LogicalResource::QueryCalls => counters.query_calls(),
        LogicalResource::EnvironmentActions => counters.environment_actions(),
        LogicalResource::CandidateGenerationAttempts => counters.candidate_generation_attempts(),
        LogicalResource::StructuralComparisons => counters.structural_comparisons(),
        LogicalResource::ActiveHypotheses => counters.active_hypotheses(),
        LogicalResource::OperationSteps => counters.operation_steps(),
        LogicalResource::RetainedInteractionRecords => counters.retained_interaction_records(),
        LogicalResource::RetainedProcedureRecords => counters.retained_procedure_records(),
    }
}

fn validate_resource_snapshot(
    snapshot: &ResourcePersistenceSnapshot,
) -> Result<(), PersistenceError> {
    if snapshot.limits() != ProtocolResourceLimits::v1_0() {
        return Err(PersistenceError::MalformedRecord(
            "resource limits disagree with resource schema v1".to_owned(),
        ));
    }
    for resource in LogicalResource::all() {
        let limit = snapshot.limits().logical(resource);
        let decision = counter_value(snapshot.decision_counters(), resource);
        let episode = counter_value(snapshot.episode_counters(), resource);
        let run = counter_value(snapshot.run_counters(), resource);
        if decision > limit.per_decision() || episode > limit.per_episode() || run > limit.per_run()
        {
            return Err(PersistenceError::MalformedRecord(format!(
                "resource {} exceeds its persisted limit",
                resource.name()
            )));
        }
        if episode > run || decision > episode {
            return Err(PersistenceError::MalformedRecord(format!(
                "resource {} episode counter exceeds run counter",
                resource.name()
            )));
        }
    }
    Ok(())
}
