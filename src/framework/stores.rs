//! First-class learner-owned in-memory stores for MVP-3.
//!
//! The stores deliberately keep opaque protocol records and use ordered
//! standard-library collections. Registration is append-only: an identifier is
//! never silently replaced, and procedure revisions are appended as explicit
//! versions. [`LearnerStores`] is the registration seam for the existing
//! interaction ledger, grammar, grounding, and changed-rule paths. MVP-4 can
//! add serialization at this seam without changing the store invariants.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::grammar_admission::{
    Candidate, CandidateCosts, CandidateScope, ProcedureRecord, ProcedureRevision, ProcedureStatus,
};
use super::interaction_ledger::{EvidenceRecord, InteractionLedger, InteractionRecord};
use super::opaque_protocol::{EvidenceId, HypothesisId, ProcedureId, ScopeToken};

/// The opaque record family named by a store error.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StoreEntity {
    Evidence,
    Hypothesis,
    Procedure,
    Derived,
}

/// Errors shared by the learner-owned MVP-3 stores.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoreError {
    ZeroScopeToken,
    DuplicateId {
        entity: StoreEntity,
        id: u64,
    },
    DuplicateVersion {
        procedure_id: ProcedureId,
        version: u32,
    },
    UnknownId {
        entity: StoreEntity,
        id: u64,
    },
    ScopeMismatch {
        entity: StoreEntity,
        id: u64,
        expected: ScopeToken,
        actual: ScopeToken,
    },
    ScopeInvalidReuse {
        entity: StoreEntity,
        id: u64,
        expected: CandidateScope,
        actual: CandidateScope,
    },
    InvalidTransition {
        hypothesis_id: HypothesisId,
        from: StoredHypothesisStatus,
        to: StoredHypothesisStatus,
    },
    MismatchedReference {
        owner: StoreEntity,
        owner_id: u64,
        reference: StoreEntity,
        reference_id: u64,
    },
    VersionNotAdvanced {
        procedure_id: ProcedureId,
        current: u32,
        requested: u32,
    },
    NotAnAdmittedProcedure(ProcedureId),
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "learner store: {self:?}")
    }
}

impl std::error::Error for StoreError {}

/// Immutable observed or incomplete evidence retained by the learner.
pub struct EvidenceStore {
    run_scope: ScopeToken,
    records: BTreeMap<EvidenceId, EvidenceRecord>,
}

impl EvidenceStore {
    pub fn new(run_scope: ScopeToken) -> Result<Self, StoreError> {
        if run_scope.is_zero() {
            return Err(StoreError::ZeroScopeToken);
        }
        Ok(Self {
            run_scope,
            records: BTreeMap::new(),
        })
    }

    pub const fn run_scope(&self) -> ScopeToken {
        self.run_scope
    }

    /// Append one ledger-owned record. A duplicate identifier is always an
    /// error, even when the replacement value happens to compare equal.
    pub fn register(&mut self, record: EvidenceRecord) -> Result<(), StoreError> {
        self.validate_run_scope(record.evidence_id(), record.run_scope())?;
        if self.records.contains_key(&record.evidence_id()) {
            return Err(StoreError::DuplicateId {
                entity: StoreEntity::Evidence,
                id: record.evidence_id().value(),
            });
        }
        self.records.insert(record.evidence_id(), record);
        Ok(())
    }

    pub fn get(&self, id: EvidenceId) -> Result<&EvidenceRecord, StoreError> {
        self.records.get(&id).ok_or(StoreError::UnknownId {
            entity: StoreEntity::Evidence,
            id: id.value(),
        })
    }

    /// Exact lookup with an explicit episode scope check before reuse.
    pub fn get_in_scope(
        &self,
        id: EvidenceId,
        episode_scope: ScopeToken,
    ) -> Result<&EvidenceRecord, StoreError> {
        let record = self.get(id)?;
        if record.episode_scope() != episode_scope {
            return Err(StoreError::ScopeMismatch {
                entity: StoreEntity::Evidence,
                id: id.value(),
                expected: episode_scope,
                actual: record.episode_scope(),
            });
        }
        Ok(record)
    }

    /// Validate a procedure or hypothesis reference against the record's
    /// candidate scope without exposing mutable storage.
    pub fn register_reference(
        &self,
        id: EvidenceId,
        scope: CandidateScope,
    ) -> Result<(), StoreError> {
        let record = self.get(id)?;
        validate_evidence_scope(id, record, scope)
    }

    pub fn validate_references(
        &self,
        ids: &[EvidenceId],
        scope: CandidateScope,
    ) -> Result<(), StoreError> {
        for id in ids {
            self.register_reference(*id, scope)?;
        }
        Ok(())
    }

    /// Values are ordered by opaque `EvidenceId`, independent of insertion
    /// order. The returned references cannot mutate historical records.
    pub fn iter(&self) -> impl Iterator<Item = &EvidenceRecord> {
        self.records.values()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    fn validate_run_scope(&self, id: EvidenceId, actual: ScopeToken) -> Result<(), StoreError> {
        if actual != self.run_scope {
            return Err(StoreError::ScopeMismatch {
                entity: StoreEntity::Evidence,
                id: id.value(),
                expected: self.run_scope,
                actual,
            });
        }
        Ok(())
    }
}

fn validate_evidence_scope(
    id: EvidenceId,
    record: &EvidenceRecord,
    scope: CandidateScope,
) -> Result<(), StoreError> {
    match scope {
        CandidateScope::Run(run) if record.run_scope() == run => Ok(()),
        CandidateScope::Episode(episode) if record.episode_scope() == episode => Ok(()),
        CandidateScope::Run(run) => Err(StoreError::ScopeMismatch {
            entity: StoreEntity::Evidence,
            id: id.value(),
            expected: run,
            actual: record.run_scope(),
        }),
        CandidateScope::Episode(episode) => Err(StoreError::ScopeMismatch {
            entity: StoreEntity::Evidence,
            id: id.value(),
            expected: episode,
            actual: record.episode_scope(),
        }),
    }
}

/// The status vocabulary for a retained learner hypothesis.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StoredHypothesisStatus {
    Active,
    Rejected,
    Stale,
    Contradicted(EvidenceId),
    Unresolved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HypothesisTransition {
    from: StoredHypothesisStatus,
    to: StoredHypothesisStatus,
}

impl HypothesisTransition {
    pub(crate) const fn from_persisted(
        from: StoredHypothesisStatus,
        to: StoredHypothesisStatus,
    ) -> Self {
        Self { from, to }
    }

    pub const fn from(self) -> StoredHypothesisStatus {
        self.from
    }

    pub const fn to(self) -> StoredHypothesisStatus {
        self.to
    }
}

/// A hypothesis plus its current status and immutable transition history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HypothesisRecord {
    candidate: Candidate,
    status: StoredHypothesisStatus,
    transitions: Vec<HypothesisTransition>,
}

impl HypothesisRecord {
    pub(crate) fn from_persisted(
        candidate: Candidate,
        status: StoredHypothesisStatus,
        transitions: Vec<HypothesisTransition>,
    ) -> Self {
        Self {
            candidate,
            status,
            transitions,
        }
    }

    pub fn candidate(&self) -> &Candidate {
        &self.candidate
    }

    pub const fn hypothesis_id(&self) -> HypothesisId {
        self.candidate.hypothesis_id()
    }

    pub fn scope(&self) -> CandidateScope {
        self.candidate.scope()
    }

    pub const fn status(&self) -> StoredHypothesisStatus {
        self.status
    }

    pub fn transitions(&self) -> &[HypothesisTransition] {
        &self.transitions
    }
}

pub struct HypothesisStore {
    run_scope: ScopeToken,
    records: BTreeMap<HypothesisId, HypothesisRecord>,
}

impl HypothesisStore {
    pub fn new(run_scope: ScopeToken) -> Result<Self, StoreError> {
        if run_scope.is_zero() {
            return Err(StoreError::ZeroScopeToken);
        }
        Ok(Self {
            run_scope,
            records: BTreeMap::new(),
        })
    }

    pub const fn run_scope(&self) -> ScopeToken {
        self.run_scope
    }

    pub fn register(
        &mut self,
        candidate: Candidate,
        evidence: &EvidenceStore,
    ) -> Result<(), StoreError> {
        self.register_with_status(candidate, StoredHypothesisStatus::Active, evidence)
    }

    pub fn register_with_status(
        &mut self,
        candidate: Candidate,
        status: StoredHypothesisStatus,
        evidence: &EvidenceStore,
    ) -> Result<(), StoreError> {
        self.validate_candidate(&candidate, evidence)?;
        let id = candidate.hypothesis_id();
        if self.records.contains_key(&id) {
            return Err(StoreError::DuplicateId {
                entity: StoreEntity::Hypothesis,
                id: id.value(),
            });
        }
        if let StoredHypothesisStatus::Contradicted(evidence_id) = status {
            evidence.register_reference(evidence_id, candidate.scope())?;
        }
        self.records.insert(
            id,
            HypothesisRecord {
                candidate,
                status,
                transitions: Vec::new(),
            },
        );
        Ok(())
    }

    pub fn get(&self, id: HypothesisId) -> Result<&HypothesisRecord, StoreError> {
        self.records.get(&id).ok_or(StoreError::UnknownId {
            entity: StoreEntity::Hypothesis,
            id: id.value(),
        })
    }

    pub fn get_in_scope(
        &self,
        id: HypothesisId,
        scope: CandidateScope,
    ) -> Result<&HypothesisRecord, StoreError> {
        let record = self.get(id)?;
        if record.scope() != scope {
            return Err(StoreError::ScopeInvalidReuse {
                entity: StoreEntity::Hypothesis,
                id: id.value(),
                expected: scope,
                actual: record.scope(),
            });
        }
        Ok(record)
    }

    pub fn validate_reference(
        &self,
        id: HypothesisId,
        scope: CandidateScope,
    ) -> Result<(), StoreError> {
        self.get_in_scope(id, scope).map(|_| ())
    }

    pub fn transition(
        &mut self,
        id: HypothesisId,
        status: StoredHypothesisStatus,
        evidence: &EvidenceStore,
    ) -> Result<(), StoreError> {
        let record = self.records.get(&id).ok_or(StoreError::UnknownId {
            entity: StoreEntity::Hypothesis,
            id: id.value(),
        })?;
        if let StoredHypothesisStatus::Contradicted(evidence_id) = status {
            evidence.register_reference(evidence_id, record.scope())?;
        }
        if !valid_hypothesis_transition(record.status, status) {
            return Err(StoreError::InvalidTransition {
                hypothesis_id: id,
                from: record.status,
                to: status,
            });
        }
        let record = self.records.get_mut(&id).expect("validated above");
        record.transitions.push(HypothesisTransition {
            from: record.status,
            to: status,
        });
        record.status = status;
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = &HypothesisRecord> {
        self.records.values()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    fn validate_candidate(
        &self,
        candidate: &Candidate,
        evidence: &EvidenceStore,
    ) -> Result<(), StoreError> {
        if candidate.run_scope() != self.run_scope {
            return Err(StoreError::ScopeMismatch {
                entity: StoreEntity::Hypothesis,
                id: candidate.hypothesis_id().value(),
                expected: self.run_scope,
                actual: candidate.run_scope(),
            });
        }
        if candidate.scope().token().is_zero() {
            return Err(StoreError::ZeroScopeToken);
        }
        evidence.validate_references(&candidate_evidence_ids(candidate), candidate.scope())
    }
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

fn candidate_evidence_ids(candidate: &Candidate) -> Vec<EvidenceId> {
    let mut ids = candidate.support_evidence_ids().to_vec();
    ids.extend(candidate.challenge_evidence_ids());
    ids.extend(candidate.grounding_evidence_ids());
    ids.extend(candidate.rejection_evidence_ids());
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// A version can be either the original admitted grammar record or the
/// existing changed-rule revision record. Both retain the same opaque ID and
/// append under the same explicit version sequence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProcedureVersion {
    Admitted(Box<ProcedureRecord>),
    Revision(Box<ProcedureRevision>),
}

impl From<ProcedureRecord> for ProcedureVersion {
    fn from(record: ProcedureRecord) -> Self {
        Self::Admitted(Box::new(record))
    }
}

impl From<ProcedureRevision> for ProcedureVersion {
    fn from(revision: ProcedureRevision) -> Self {
        Self::Revision(Box::new(revision))
    }
}

impl ProcedureVersion {
    pub const fn procedure_id(&self) -> ProcedureId {
        match self {
            Self::Admitted(record) => record.procedure_id(),
            Self::Revision(revision) => revision.procedure_id(),
        }
    }

    pub const fn version(&self) -> u32 {
        match self {
            Self::Admitted(record) => record.version(),
            Self::Revision(revision) => revision.version(),
        }
    }

    pub const fn status(&self) -> ProcedureStatus {
        match self {
            Self::Admitted(record) => record.status(),
            Self::Revision(revision) => revision.status(),
        }
    }

    pub fn scope(&self) -> CandidateScope {
        match self {
            Self::Admitted(record) => record.scope(),
            Self::Revision(revision) => revision.expansion().scope(),
        }
    }

    pub fn hypothesis_id(&self) -> HypothesisId {
        match self {
            Self::Admitted(record) => record.hypothesis_id(),
            Self::Revision(revision) => revision.expansion().hypothesis_id(),
        }
    }

    pub fn expansion(&self) -> &Candidate {
        match self {
            Self::Admitted(record) => record.expansion(),
            Self::Revision(revision) => revision.expansion(),
        }
    }

    pub fn evidence_refs(&self) -> Vec<EvidenceId> {
        let mut ids = candidate_evidence_ids(self.expansion());
        if let Self::Revision(revision) = self {
            ids.extend(revision.contradiction_evidence_ids());
            ids.sort_unstable();
            ids.dedup();
        }
        ids
    }

    pub fn structural_signature(&self) -> &str {
        self.expansion().canonical_signature()
    }

    pub fn costs(&self) -> CandidateCosts {
        self.expansion().costs()
    }
}

/// Read-only wrapper with normalized lookup metadata for one procedure
/// version. The underlying protocol record remains available by reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredProcedure {
    version_record: ProcedureVersion,
    evidence_refs: Vec<EvidenceId>,
    structural_signature: String,
}

impl StoredProcedure {
    fn new(version_record: ProcedureVersion) -> Self {
        let evidence_refs = version_record.evidence_refs();
        let structural_signature = version_record.structural_signature().to_owned();
        Self {
            version_record,
            evidence_refs,
            structural_signature,
        }
    }

    pub fn version_record(&self) -> &ProcedureVersion {
        &self.version_record
    }

    pub const fn procedure_id(&self) -> ProcedureId {
        self.version_record.procedure_id()
    }

    pub const fn version(&self) -> u32 {
        self.version_record.version()
    }

    pub const fn status(&self) -> ProcedureStatus {
        self.version_record.status()
    }

    pub fn scope(&self) -> CandidateScope {
        self.version_record.scope()
    }

    pub fn hypothesis_id(&self) -> HypothesisId {
        self.version_record.hypothesis_id()
    }

    pub fn expansion(&self) -> &Candidate {
        self.version_record.expansion()
    }

    pub fn evidence_refs(&self) -> &[EvidenceId] {
        &self.evidence_refs
    }

    pub fn structural_signature(&self) -> &str {
        &self.structural_signature
    }

    pub fn costs(&self) -> CandidateCosts {
        self.version_record.costs()
    }
}

pub struct ProcedureStore {
    run_scope: ScopeToken,
    versions: BTreeMap<ProcedureId, Vec<StoredProcedure>>,
}

impl ProcedureStore {
    pub fn new(run_scope: ScopeToken) -> Result<Self, StoreError> {
        if run_scope.is_zero() {
            return Err(StoreError::ZeroScopeToken);
        }
        Ok(Self {
            run_scope,
            versions: BTreeMap::new(),
        })
    }

    pub const fn run_scope(&self) -> ScopeToken {
        self.run_scope
    }

    /// Register the first admitted version. A procedure ID is not reused by
    /// this method; revisions must use [`Self::append_version`].
    pub fn register(
        &mut self,
        record: ProcedureRecord,
        evidence: &EvidenceStore,
    ) -> Result<(), StoreError> {
        self.register_version(ProcedureVersion::from(record), evidence)
    }

    pub fn register_version(
        &mut self,
        version: ProcedureVersion,
        evidence: &EvidenceStore,
    ) -> Result<(), StoreError> {
        let id = version.procedure_id();
        if self.versions.contains_key(&id) {
            return Err(StoreError::DuplicateId {
                entity: StoreEntity::Procedure,
                id: id.value(),
            });
        }
        if version.version() != 1 {
            return Err(StoreError::VersionNotAdvanced {
                procedure_id: id,
                current: 0,
                requested: version.version(),
            });
        }
        if version.status() != ProcedureStatus::Active {
            return Err(StoreError::NotAnAdmittedProcedure(id));
        }
        self.validate_version(&version, evidence)?;
        self.versions
            .insert(id, vec![StoredProcedure::new(version)]);
        Ok(())
    }

    /// Append a new explicit version without changing any older record.
    pub fn append_version(
        &mut self,
        version: ProcedureVersion,
        evidence: &EvidenceStore,
    ) -> Result<(), StoreError> {
        let id = version.procedure_id();
        let versions = self.versions.get(&id).ok_or(StoreError::UnknownId {
            entity: StoreEntity::Procedure,
            id: id.value(),
        })?;
        let current = versions.last().expect("procedure versions are non-empty");
        if version.version() <= current.version() {
            if version.version() == current.version() {
                return Err(StoreError::DuplicateVersion {
                    procedure_id: id,
                    version: version.version(),
                });
            }
            return Err(StoreError::VersionNotAdvanced {
                procedure_id: id,
                current: current.version(),
                requested: version.version(),
            });
        }
        if version.scope() != current.scope() {
            return Err(StoreError::ScopeInvalidReuse {
                entity: StoreEntity::Procedure,
                id: id.value(),
                expected: current.scope(),
                actual: version.scope(),
            });
        }
        self.validate_version(&version, evidence)?;
        self.versions
            .get_mut(&id)
            .expect("validated above")
            .push(StoredProcedure::new(version));
        Ok(())
    }

    /// Exact lookup returns the newest registered version only after checking
    /// the caller's requested candidate scope.
    pub fn get(
        &self,
        id: ProcedureId,
        scope: CandidateScope,
    ) -> Result<&StoredProcedure, StoreError> {
        validate_requested_scope(self.run_scope, id, scope)?;
        let versions = self.versions.get(&id).ok_or(StoreError::UnknownId {
            entity: StoreEntity::Procedure,
            id: id.value(),
        })?;
        let current = versions.last().expect("procedure versions are non-empty");
        if current.scope() != scope {
            return Err(StoreError::ScopeInvalidReuse {
                entity: StoreEntity::Procedure,
                id: id.value(),
                expected: scope,
                actual: current.scope(),
            });
        }
        Ok(current)
    }

    pub fn get_version(
        &self,
        id: ProcedureId,
        version: u32,
        scope: CandidateScope,
    ) -> Result<&StoredProcedure, StoreError> {
        validate_requested_scope(self.run_scope, id, scope)?;
        let versions = self.versions.get(&id).ok_or(StoreError::UnknownId {
            entity: StoreEntity::Procedure,
            id: id.value(),
        })?;
        let record = versions
            .iter()
            .find(|record| record.version() == version)
            .ok_or(StoreError::UnknownId {
                entity: StoreEntity::Procedure,
                id: id.value(),
            })?;
        if record.scope() != scope {
            return Err(StoreError::ScopeInvalidReuse {
                entity: StoreEntity::Procedure,
                id: id.value(),
                expected: scope,
                actual: record.scope(),
            });
        }
        Ok(record)
    }

    pub fn versions(
        &self,
        id: ProcedureId,
        scope: CandidateScope,
    ) -> Result<&[StoredProcedure], StoreError> {
        validate_requested_scope(self.run_scope, id, scope)?;
        let versions = self.versions.get(&id).ok_or(StoreError::UnknownId {
            entity: StoreEntity::Procedure,
            id: id.value(),
        })?;
        if versions.iter().any(|version| version.scope() != scope) {
            return Err(StoreError::ScopeInvalidReuse {
                entity: StoreEntity::Procedure,
                id: id.value(),
                expected: scope,
                actual: versions[0].scope(),
            });
        }
        Ok(versions)
    }

    /// Structural signatures are returned in procedure-ID/version order.
    pub fn lookup_by_signature(
        &self,
        signature: &str,
        scope: CandidateScope,
    ) -> Result<Vec<&StoredProcedure>, StoreError> {
        validate_requested_scope(self.run_scope, ProcedureId::new(0), scope)?;
        Ok(self
            .iter()
            .filter(|record| record.scope() == scope && record.structural_signature() == signature)
            .collect())
    }

    pub fn by_signature(
        &self,
        signature: &str,
        scope: CandidateScope,
    ) -> Result<Vec<&StoredProcedure>, StoreError> {
        self.lookup_by_signature(signature, scope)
    }

    pub fn iter(&self) -> impl Iterator<Item = &StoredProcedure> {
        self.versions.values().flat_map(|versions| versions.iter())
    }

    pub fn len(&self) -> usize {
        self.versions.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.versions.is_empty()
    }

    fn validate_version(
        &self,
        version: &ProcedureVersion,
        evidence: &EvidenceStore,
    ) -> Result<(), StoreError> {
        let id = version.procedure_id();
        if version.expansion().run_scope() != self.run_scope {
            return Err(StoreError::ScopeMismatch {
                entity: StoreEntity::Procedure,
                id: id.value(),
                expected: self.run_scope,
                actual: version.expansion().run_scope(),
            });
        }
        if version.scope().token().is_zero() {
            return Err(StoreError::ZeroScopeToken);
        }
        evidence.validate_references(&version.evidence_refs(), version.scope())
    }
}

fn validate_requested_scope(
    run_scope: ScopeToken,
    id: ProcedureId,
    scope: CandidateScope,
) -> Result<(), StoreError> {
    if scope.token().is_zero() {
        return Err(StoreError::ZeroScopeToken);
    }
    if matches!(scope, CandidateScope::Run(token) if token != run_scope) {
        return Err(StoreError::ScopeMismatch {
            entity: StoreEntity::Procedure,
            id: id.value(),
            expected: run_scope,
            actual: scope.token(),
        });
    }
    Ok(())
}

/// A derived record uses the existing decision-sequence identity from
/// `DerivedResult`; no new semantic identifier type is introduced.
pub type DerivedId = u64;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DependencySource {
    Evidence(EvidenceId),
    Hypothesis(HypothesisId),
    Procedure(ProcedureId),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DependentRecord {
    Hypothesis(HypothesisId),
    Procedure(ProcedureId),
    Derived(DerivedId),
}

/// Append-only dependency links. Status transitions and procedure versions do
/// not delete or rewrite links, so invalidation can always find every affected
/// derived record deterministically.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DependencyIndex {
    links: BTreeMap<DependencySource, BTreeSet<DependentRecord>>,
}

impl DependencyIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn link(&mut self, source: DependencySource, dependent: DependentRecord) {
        self.links.entry(source).or_default().insert(dependent);
    }

    pub fn link_evidence_to_hypothesis(&mut self, evidence: EvidenceId, hypothesis: HypothesisId) {
        self.link(
            DependencySource::Evidence(evidence),
            DependentRecord::Hypothesis(hypothesis),
        );
    }

    pub fn link_evidence_to_procedure(&mut self, evidence: EvidenceId, procedure: ProcedureId) {
        self.link(
            DependencySource::Evidence(evidence),
            DependentRecord::Procedure(procedure),
        );
    }

    pub fn link_evidence_to_derived(&mut self, evidence: EvidenceId, derived: DerivedId) {
        self.link(
            DependencySource::Evidence(evidence),
            DependentRecord::Derived(derived),
        );
    }

    pub fn link_hypothesis_to_procedure(
        &mut self,
        hypothesis: HypothesisId,
        procedure: ProcedureId,
    ) {
        self.link(
            DependencySource::Hypothesis(hypothesis),
            DependentRecord::Procedure(procedure),
        );
    }

    pub fn link_hypothesis_to_derived(&mut self, hypothesis: HypothesisId, derived: DerivedId) {
        self.link(
            DependencySource::Hypothesis(hypothesis),
            DependentRecord::Derived(derived),
        );
    }

    pub fn link_procedure_to_derived(&mut self, procedure: ProcedureId, derived: DerivedId) {
        self.link(
            DependencySource::Procedure(procedure),
            DependentRecord::Derived(derived),
        );
    }

    pub fn affected_records(&self, source: DependencySource) -> Vec<DependentRecord> {
        self.links
            .get(&source)
            .map(|records| records.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn affected_derived_ids(&self, source: DependencySource) -> Vec<DerivedId> {
        self.affected_records(source)
            .into_iter()
            .filter_map(|record| match record {
                DependentRecord::Derived(id) => Some(id),
                DependentRecord::Hypothesis(_) | DependentRecord::Procedure(_) => None,
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.links.values().map(BTreeSet::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    pub fn links(&self) -> impl Iterator<Item = (DependencySource, DependentRecord)> + '_ {
        self.links.iter().flat_map(|(source, dependents)| {
            dependents
                .iter()
                .copied()
                .map(move |dependent| (*source, dependent))
        })
    }
}

/// The four stores as one learner-owned component. Mutating operations are
/// explicit registration/version/status methods; callers only receive shared
/// access to the underlying historical collections.
pub struct LearnerStores {
    evidence: EvidenceStore,
    hypotheses: HypothesisStore,
    procedures: ProcedureStore,
    dependencies: DependencyIndex,
}

impl LearnerStores {
    pub fn new(run_scope: ScopeToken) -> Result<Self, StoreError> {
        Ok(Self {
            evidence: EvidenceStore::new(run_scope)?,
            hypotheses: HypothesisStore::new(run_scope)?,
            procedures: ProcedureStore::new(run_scope)?,
            dependencies: DependencyIndex::new(),
        })
    }

    pub const fn run_scope(&self) -> ScopeToken {
        self.evidence.run_scope()
    }

    pub fn evidence(&self) -> &EvidenceStore {
        &self.evidence
    }

    pub fn hypotheses(&self) -> &HypothesisStore {
        &self.hypotheses
    }

    pub fn procedures(&self) -> &ProcedureStore {
        &self.procedures
    }

    pub fn dependencies(&self) -> &DependencyIndex {
        &self.dependencies
    }

    pub fn register_evidence(&mut self, record: EvidenceRecord) -> Result<(), StoreError> {
        self.evidence.register(record)
    }

    /// Clean seam from the append-only ledger. MVP-4 persistence can replay
    /// this same evidence registration without changing the ledger protocol.
    pub fn register_interaction(
        &mut self,
        interaction: &InteractionRecord,
    ) -> Result<(), StoreError> {
        self.register_evidence(interaction.evidence().clone())
    }

    pub fn register_ledger(&mut self, ledger: &InteractionLedger) -> Result<(), StoreError> {
        for interaction in ledger.interactions() {
            self.register_interaction(interaction)?;
        }
        Ok(())
    }

    pub fn register_hypothesis(
        &mut self,
        candidate: Candidate,
        status: StoredHypothesisStatus,
    ) -> Result<(), StoreError> {
        let id = candidate.hypothesis_id();
        let evidence_ids = candidate_evidence_ids(&candidate);
        self.hypotheses
            .register_with_status(candidate, status, &self.evidence)?;
        for evidence in evidence_ids {
            self.dependencies.link_evidence_to_hypothesis(evidence, id);
        }
        Ok(())
    }

    pub(crate) fn restore_hypothesis(
        &mut self,
        candidate: Candidate,
        initial_status: StoredHypothesisStatus,
        transitions: &[HypothesisTransition],
    ) -> Result<(), StoreError> {
        let id = candidate.hypothesis_id();
        self.hypotheses
            .register_with_status(candidate, initial_status, &self.evidence)?;
        for transition in transitions {
            self.hypotheses
                .transition(id, transition.to(), &self.evidence)?;
        }
        Ok(())
    }

    pub fn register_procedure(&mut self, record: ProcedureRecord) -> Result<(), StoreError> {
        self.register_procedure_version(ProcedureVersion::from(record))
    }

    pub fn append_procedure_version(
        &mut self,
        version: ProcedureVersion,
    ) -> Result<(), StoreError> {
        self.register_procedure_version_inner(version, false)
    }

    pub fn register_procedure_version(
        &mut self,
        version: ProcedureVersion,
    ) -> Result<(), StoreError> {
        self.register_procedure_version_inner(version, true)
    }

    fn register_procedure_version_inner(
        &mut self,
        version: ProcedureVersion,
        first: bool,
    ) -> Result<(), StoreError> {
        let id = version.procedure_id();
        let scope = version.scope();
        self.hypotheses
            .validate_reference(version.hypothesis_id(), scope)
            .map_err(|error| match error {
                StoreError::ScopeInvalidReuse { .. } => StoreError::MismatchedReference {
                    owner: StoreEntity::Procedure,
                    owner_id: id.value(),
                    reference: StoreEntity::Hypothesis,
                    reference_id: version.hypothesis_id().value(),
                },
                other => other,
            })?;
        if first {
            self.procedures
                .register_version(version.clone(), &self.evidence)?;
        } else {
            self.procedures
                .append_version(version.clone(), &self.evidence)?;
        }
        for evidence in version.evidence_refs() {
            self.dependencies.link_evidence_to_procedure(evidence, id);
        }
        self.dependencies
            .link_hypothesis_to_procedure(version.hypothesis_id(), id);
        Ok(())
    }

    pub(crate) fn restore_procedure_version(
        &mut self,
        version: ProcedureVersion,
    ) -> Result<(), StoreError> {
        let first = self.procedures.is_empty()
            || self
                .procedures
                .iter()
                .all(|record| record.procedure_id() != version.procedure_id());
        if first {
            self.procedures.register_version(version, &self.evidence)
        } else {
            self.procedures.append_version(version, &self.evidence)
        }
    }

    pub(crate) fn restore_dependency(
        &mut self,
        source: DependencySource,
        dependent: DependentRecord,
    ) {
        self.dependencies.link(source, dependent);
    }

    pub fn transition_hypothesis(
        &mut self,
        id: HypothesisId,
        status: StoredHypothesisStatus,
    ) -> Result<(), StoreError> {
        self.hypotheses.transition(id, status, &self.evidence)?;
        if let StoredHypothesisStatus::Contradicted(evidence) = status {
            self.dependencies.link_evidence_to_hypothesis(evidence, id);
        }
        Ok(())
    }

    pub fn link_evidence_to_derived(
        &mut self,
        evidence: EvidenceId,
        derived: DerivedId,
    ) -> Result<(), StoreError> {
        self.evidence.get(evidence)?;
        self.dependencies
            .link_evidence_to_derived(evidence, derived);
        Ok(())
    }

    pub fn link_hypothesis_to_derived(
        &mut self,
        hypothesis: HypothesisId,
        derived: DerivedId,
    ) -> Result<(), StoreError> {
        self.hypotheses.get(hypothesis)?;
        self.dependencies
            .link_hypothesis_to_derived(hypothesis, derived);
        Ok(())
    }

    pub fn link_procedure_to_derived(
        &mut self,
        procedure: ProcedureId,
        derived: DerivedId,
    ) -> Result<(), StoreError> {
        self.procedures
            .get(procedure, CandidateScope::Run(self.run_scope()))?;
        self.dependencies
            .link_procedure_to_derived(procedure, derived);
        Ok(())
    }
}
