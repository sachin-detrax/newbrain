mod candidate;
use crate::budget::{BudgetError, BudgetKind as K, BudgetScope as S, BudgetState};
use crate::ledger::{CostVector, Ledger};
use crate::types::{EvidenceRef, ProcedureId, RecordId, Transition};
pub use candidate::{Candidate, CandidateKind, CandidateSignature, OutputOp, PatternAtom};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const GRAMMAR_VERSION: &str = "g0-g1-v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenerationError {
    Budget(BudgetError),
    DuplicateRecord(RecordId),
    IdentifierExhausted,
}
impl From<BudgetError> for GenerationError {
    fn from(error: BudgetError) -> Self {
        Self::Budget(error)
    }
}
impl fmt::Display for GenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for GenerationError {}

fn ordered(records: &[Transition]) -> Result<Vec<&Transition>, GenerationError> {
    let mut ids = BTreeSet::new();
    for r in records {
        if !ids.insert(r.record_id) {
            return Err(GenerationError::DuplicateRecord(r.record_id));
        }
    }
    let mut sorted: Vec<_> = records.iter().collect();
    sorted.sort_by_key(|r| r.record_id);
    Ok(sorted)
}

fn allocate(budget: &mut BudgetState) -> Result<ProcedureId, GenerationError> {
    budget.charge(K::GeneratedCandidates, 1)?;
    budget.charge_total(K::TotalCandidateRecords, 1)?;
    budget.set_active(
        K::ActiveCandidateRecords,
        budget.used(K::GeneratedCandidates),
    )?;
    let id = u16::try_from(budget.used_in(S::TotalRun, K::TotalCandidateRecords))
        .map_err(|_| GenerationError::IdentifierExhausted)?;
    Ok(ProcedureId(id))
}

pub fn enumerate_g0(
    records: &[Transition],
    budget: &mut BudgetState,
    ledger: &mut Ledger,
) -> Result<Vec<Candidate>, GenerationError> {
    let mut result = Vec::new();
    for record in ordered(records)? {
        budget.charge(K::OperationSteps, 6)?;
        let candidate = Candidate::exact(allocate(budget)?, record);
        ledger.record_candidate_generated(
            candidate.id(),
            candidate.evidence(),
            CostVector::capture(budget, S::TrainingUpdate),
        )?;
        result.push(candidate);
    }
    Ok(result)
}

pub fn enumerate_g1(
    records: &[Transition],
    budget: &mut BudgetState,
    ledger: &mut Ledger,
) -> Result<Vec<Candidate>, GenerationError> {
    let sorted = ordered(records)?;
    let mut unique: BTreeMap<CandidateSignature, Candidate> = BTreeMap::new();
    for left in &sorted {
        for right in &sorted {
            if left.record_id == right.record_id {
                continue;
            }
            budget.charge(K::CandidatePairs, 1)?;
            budget.charge(K::OperationSteps, 4)?;
            if left.action != right.action
                || left.scope != right.scope
                || left.input == left.output
                || right.input == right.output
            {
                continue;
            }
            let id = allocate(budget)?;
            let refs = vec![
                EvidenceRef::record(left.record_id),
                EvidenceRef::record(right.record_id),
            ];
            ledger.record_candidate_generated(
                id,
                refs.clone(),
                CostVector::capture(budget, S::TrainingUpdate),
            )?;
            let candidate = unify(id, left, right, budget)?;
            match candidate {
                None => ledger.record_rejection_with_evidence(
                    id,
                    "unstable-output",
                    refs,
                    CostVector::capture(budget, S::TrainingUpdate),
                )?,
                Some(candidate) => {
                    let signature = candidate.signature().clone();
                    if let Some(existing) = unique.get_mut(&signature) {
                        existing.merge_supports(candidate.supports());
                        ledger.record_rejection_with_evidence(
                            id,
                            "duplicate-signature",
                            refs,
                            CostVector::capture(budget, S::TrainingUpdate),
                        )?;
                    } else {
                        unique.insert(signature, candidate);
                    }
                }
            }
        }
    }
    Ok(unique.into_values().collect())
}

fn unify(
    id: ProcedureId,
    a: &Transition,
    b: &Transition,
    budget: &mut BudgetState,
) -> Result<Option<Candidate>, BudgetError> {
    let ai = a.input.positions();
    let bi = b.input.positions();
    let ao = a.output.positions();
    let bo = b.output.positions();
    let mut pattern = [PatternAtom::Constant(0); 3];
    let mut output = [OutputOp::Copy(0); 3];
    let mut variable = 0;
    for i in 0..3 {
        budget.charge(K::SupportComparisons, 1)?;
        budget.charge(K::OperationSteps, 1)?;
        if ai[i] == bi[i] {
            pattern[i] = PatternAtom::Constant(ai[i]);
        } else {
            let mut previous = None;
            for j in 0..i {
                budget.charge(K::SupportComparisons, 2)?;
                budget.charge(K::OperationSteps, 2)?;
                if ai[i] == ai[j] && bi[i] == bi[j] {
                    previous = Some(pattern[j]);
                    break;
                }
            }
            pattern[i] = previous.unwrap_or_else(|| {
                let p = PatternAtom::Variable(variable);
                variable += 1;
                p
            });
        }
        budget.charge(K::SupportComparisons, 3)?;
        budget.charge(K::OperationSteps, 3)?;
        output[i] = if ao[i] == ai[i] && bo[i] == bi[i] {
            OutputOp::Copy(i)
        } else if ao[i] == bo[i] {
            OutputOp::Emit(ao[i])
        } else {
            return Ok(None);
        };
    }
    Ok(Some(Candidate::generalized(id, a, b, pattern, output)))
}
