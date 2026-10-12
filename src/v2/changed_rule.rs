//! Evaluator-only changed-transition/revision extension.
//!
//! The primary [`super::evaluator_world::EvaluatorWorld`] is intentionally not
//! changed by this module.  A changed-rule probe receives a fresh run scope and
//! keeps only an opaque reference to the old procedure/version.  It does not
//! install or reuse the old procedure in the new run.  Its contradiction and
//! revision trace is an evaluator-side artifact for testing the learner's
//! revalidation boundary.

use std::fmt;

use super::grammar_admission::ProcedureRecord;
use super::opaque_protocol::{EvidenceId, ObservationPacket, ProcedureId, ScopeToken};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcedureVersionRef {
    procedure_id: ProcedureId,
    version: u32,
}

impl ProcedureVersionRef {
    pub const fn new(procedure_id: ProcedureId, version: u32) -> Self {
        Self {
            procedure_id,
            version,
        }
    }

    pub const fn procedure_id(self) -> ProcedureId {
        self.procedure_id
    }
    pub const fn version(self) -> u32 {
        self.version
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RevisionEvent {
    Contradiction {
        evidence_id: EvidenceId,
        predicted: ObservationPacket,
        observed: ObservationPacket,
    },
    Revision {
        old: ProcedureVersionRef,
        new: ProcedureVersionRef,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangedRuleRevisionTrace {
    fresh_run_scope: ScopeToken,
    old: ProcedureVersionRef,
    new: Option<ProcedureVersionRef>,
    contradiction_evidence_ids: Vec<EvidenceId>,
    events: Vec<RevisionEvent>,
}

impl ChangedRuleRevisionTrace {
    pub const fn fresh_run_scope(&self) -> ScopeToken {
        self.fresh_run_scope
    }
    pub const fn old(&self) -> ProcedureVersionRef {
        self.old
    }
    pub const fn new_procedure(&self) -> Option<ProcedureVersionRef> {
        self.new
    }
    pub fn contradiction_evidence_ids(&self) -> &[EvidenceId] {
        &self.contradiction_evidence_ids
    }
    pub fn events(&self) -> &[RevisionEvent] {
        &self.events
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangedRuleError {
    ZeroFreshScope,
    ReusesOldRunScope,
    NotAContradiction,
    RevisionAlreadyRecorded,
    RevisionVersionNotAdvanced,
}

impl fmt::Display for ChangedRuleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "v2 changed-rule extension: {self:?}")
    }
}

impl std::error::Error for ChangedRuleError {}

/// An evaluator-only fresh-run trace.  The old procedure itself is not stored,
/// so construction cannot silently make it available to a new learner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangedRuleRevision {
    trace: ChangedRuleRevisionTrace,
}

impl ChangedRuleRevision {
    pub fn new(
        old: &ProcedureRecord,
        fresh_run_scope: ScopeToken,
    ) -> Result<Self, ChangedRuleError> {
        if fresh_run_scope.is_zero() {
            return Err(ChangedRuleError::ZeroFreshScope);
        }
        if fresh_run_scope == old.expansion().run_scope() {
            return Err(ChangedRuleError::ReusesOldRunScope);
        }
        Ok(Self {
            trace: ChangedRuleRevisionTrace {
                fresh_run_scope,
                old: ProcedureVersionRef::new(old.procedure_id(), old.version()),
                new: None,
                contradiction_evidence_ids: Vec::new(),
                events: Vec::new(),
            },
        })
    }

    pub const fn fresh_run_scope(&self) -> ScopeToken {
        self.trace.fresh_run_scope()
    }
    pub const fn old_procedure(&self) -> ProcedureVersionRef {
        self.trace.old()
    }
    pub const fn new_procedure(&self) -> Option<ProcedureVersionRef> {
        self.trace.new_procedure()
    }

    pub fn trace(&self) -> &ChangedRuleRevisionTrace {
        &self.trace
    }

    /// Record only a returned opaque contradiction.  An absent prediction is
    /// unresolved evidence, not a contradiction and is therefore rejected by
    /// this evaluator-only extension.
    pub fn record_contradiction(
        &mut self,
        evidence_id: EvidenceId,
        predicted: Option<ObservationPacket>,
        observed: ObservationPacket,
    ) -> Result<(), ChangedRuleError> {
        let Some(predicted) = predicted else {
            return Err(ChangedRuleError::NotAContradiction);
        };
        if predicted == observed {
            return Err(ChangedRuleError::NotAContradiction);
        }
        self.trace.contradiction_evidence_ids.push(evidence_id);
        self.trace.contradiction_evidence_ids.sort_unstable();
        self.trace.contradiction_evidence_ids.dedup();
        self.trace.events.push(RevisionEvent::Contradiction {
            evidence_id,
            predicted,
            observed,
        });
        Ok(())
    }

    /// Append a new version marker.  This does not rewrite the old version or
    /// any old evidence and does not install a procedure in the fresh run.
    pub fn revise(&mut self, new_version: u32) -> Result<ProcedureVersionRef, ChangedRuleError> {
        if self.trace.new.is_some() {
            return Err(ChangedRuleError::RevisionAlreadyRecorded);
        }
        if new_version <= self.trace.old.version {
            return Err(ChangedRuleError::RevisionVersionNotAdvanced);
        }
        let new = ProcedureVersionRef::new(self.trace.old.procedure_id, new_version);
        self.trace.events.push(RevisionEvent::Revision {
            old: self.trace.old,
            new,
        });
        self.trace.new = Some(new);
        Ok(new)
    }
}
