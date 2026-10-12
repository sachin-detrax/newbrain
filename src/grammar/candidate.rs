use crate::budget::{BudgetError, BudgetKind as K, BudgetScope, BudgetState};
use crate::types::{
    Action, EvidenceRef, ProcedureId, Query, RecordId, Scope, Sequence, Symbol, Transition,
};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PatternAtom {
    Constant(Symbol),
    Variable(usize),
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OutputOp {
    Copy(usize),
    Emit(Symbol),
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CandidateKind {
    Exact,
    Generalized,
}
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CandidateSignature {
    pub kind: CandidateKind,
    pub scope: Scope,
    pub action: Action,
    pub pattern: [PatternAtom; 3],
    pub output: [OutputOp; 3],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Candidate {
    id: ProcedureId,
    signature: CandidateSignature,
    supports: BTreeSet<RecordId>,
}
impl Candidate {
    pub(super) fn exact(id: ProcedureId, r: &Transition) -> Self {
        Self {
            id,
            signature: CandidateSignature {
                kind: CandidateKind::Exact,
                scope: r.scope,
                action: r.action,
                pattern: r.input.positions().map(PatternAtom::Constant),
                output: r.output.positions().map(OutputOp::Emit),
            },
            supports: BTreeSet::from([r.record_id]),
        }
    }
    pub(super) fn generalized(
        id: ProcedureId,
        a: &Transition,
        b: &Transition,
        pattern: [PatternAtom; 3],
        output: [OutputOp; 3],
    ) -> Self {
        Self {
            id,
            signature: CandidateSignature {
                kind: CandidateKind::Generalized,
                scope: a.scope,
                action: a.action,
                pattern,
                output,
            },
            supports: BTreeSet::from([a.record_id, b.record_id]),
        }
    }
    pub(super) fn merge_supports(&mut self, refs: &BTreeSet<RecordId>) {
        self.supports.extend(refs);
    }
    pub fn id(&self) -> ProcedureId {
        self.id
    }
    pub fn signature(&self) -> &CandidateSignature {
        &self.signature
    }
    pub fn pattern(&self) -> &[PatternAtom; 3] {
        &self.signature.pattern
    }
    pub fn output(&self) -> &[OutputOp; 3] {
        &self.signature.output
    }
    pub fn supports(&self) -> &BTreeSet<RecordId> {
        &self.supports
    }
    pub fn evidence(&self) -> Vec<EvidenceRef> {
        self.supports
            .iter()
            .copied()
            .map(EvidenceRef::record)
            .collect()
    }

    pub fn predict(
        &self,
        query: &Query,
        budget: &mut BudgetState,
        scope: BudgetScope,
    ) -> Result<Option<Sequence>, BudgetError> {
        budget.charge_in(scope, K::OperationSteps, 2)?;
        if query.scope != self.signature.scope || query.action != self.signature.action {
            return Ok(None);
        }
        let input = query.input.positions();
        let mut bindings = [None; 3];
        for (i, pattern) in self.signature.pattern.iter().enumerate() {
            budget.charge_in(scope, K::OperationSteps, 1)?;
            match *pattern {
                PatternAtom::Constant(value) => {
                    if input[i] != value {
                        return Ok(None);
                    }
                }
                PatternAtom::Variable(slot) => match bindings[slot] {
                    Some(value) if value != input[i] => return Ok(None),
                    Some(_) => {}
                    None => bindings[slot] = Some(input[i]),
                },
            }
        }
        let mut output = [0; 3];
        for (i, op) in self.signature.output.iter().enumerate() {
            budget.charge_in(scope, K::OperationSteps, 1)?;
            output[i] = match *op {
                OutputOp::Copy(j) => input[j],
                OutputOp::Emit(value) => value,
            };
        }
        Ok(Some(Sequence::new(output)))
    }
}
