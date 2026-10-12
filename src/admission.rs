use crate::budget::{BudgetError, BudgetKind as K, BudgetScope as S, BudgetState};
use crate::grammar::{Candidate, CandidateKind, PatternAtom, GRAMMAR_VERSION};
use crate::ledger::{CostVector, Ledger};
use crate::types::{EvidenceRef, ProcedureId, Query, QueryId, RecordId, Transition};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RejectionReason {
    InsufficientSupport,
    SupportMismatch,
    TrainingContradiction,
    NoStructuralVariation,
    DevelopmentContradiction,
    IncompleteChallenges,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdmissionResult {
    Admitted(Box<AdmittedProcedure>),
    Rejected(RejectionReason),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedProcedure {
    candidate: Candidate,
    challenges: Vec<EvidenceRef>,
    cost: CostVector,
}
impl AdmittedProcedure {
    pub fn id(&self) -> ProcedureId {
        self.candidate.id()
    }
    pub fn candidate(&self) -> &Candidate {
        &self.candidate
    }
    pub fn support_refs(&self) -> Vec<EvidenceRef> {
        self.candidate.evidence()
    }
    pub fn challenge_refs(&self) -> &[EvidenceRef] {
        &self.challenges
    }
    pub fn cost(&self) -> &CostVector {
        &self.cost
    }
    pub fn grammar_version(&self) -> &'static str {
        GRAMMAR_VERSION
    }
}

fn query(record: &Transition) -> Query {
    Query {
        input: record.input,
        action: record.action,
        scope: record.scope,
        query_id: QueryId(0),
    }
}

fn gate(
    candidate: &Candidate,
    name: &str,
    failure: Option<RejectionReason>,
    refs: Vec<EvidenceRef>,
    budget: &mut BudgetState,
    ledger: &mut Ledger,
) -> Result<Option<AdmissionResult>, BudgetError> {
    budget.charge(K::OperationSteps, 1)?;
    ledger.record_check(
        candidate.id(),
        failure.is_none(),
        name,
        refs.clone(),
        CostVector::capture(budget, S::TrainingUpdate),
    )?;
    match failure {
        None => Ok(None),
        Some(reason) => {
            ledger.record_rejection_with_evidence(
                candidate.id(),
                format!("{reason:?}"),
                refs,
                CostVector::capture(budget, S::TrainingUpdate),
            )?;
            Ok(Some(AdmissionResult::Rejected(reason)))
        }
    }
}

pub fn admit(
    candidate: Candidate,
    training: &[Transition],
    development: &[Transition],
    budget: &mut BudgetState,
    ledger: &mut Ledger,
) -> Result<AdmissionResult, BudgetError> {
    admit_with_policy(candidate, training, development, budget, ledger, true)
}

pub(crate) fn admit_with_policy(
    candidate: Candidate,
    training: &[Transition],
    development: &[Transition],
    budget: &mut BudgetState,
    ledger: &mut Ledger,
    development_gate: bool,
) -> Result<AdmissionResult, BudgetError> {
    let refs = candidate.evidence();
    let support: Vec<_> = training
        .iter()
        .filter(|r| candidate.supports().contains(&r.record_id))
        .collect();
    let unique: BTreeSet<_> = support.iter().map(|r| r.record_id).collect();
    let a = (candidate.signature().kind != CandidateKind::Generalized
        || unique.len() < 2
        || unique.len() != candidate.supports().len())
    .then_some(RejectionReason::InsufficientSupport);
    if let Some(result) = gate(&candidate, "A:support", a, refs.clone(), budget, ledger)? {
        return Ok(result);
    }
    let mut consistent = true;
    for r in &support {
        budget.charge(K::SupportComparisons, 3)?;
        consistent &= candidate.predict(&query(r), budget, S::TrainingUpdate)? == Some(r.output);
    }
    if let Some(result) = gate(
        &candidate,
        "B:consistency",
        (!consistent).then_some(RejectionReason::SupportMismatch),
        refs.clone(),
        budget,
        ledger,
    )? {
        return Ok(result);
    }
    let mut contradiction = false;
    let mut checked_training = refs.clone();
    for r in training {
        budget.charge(K::SupportComparisons, 3)?;
        if let Some(output) = candidate.predict(&query(r), budget, S::TrainingUpdate)? {
            contradiction |= output != r.output;
            if output != r.output {
                checked_training.push(EvidenceRef::record(r.record_id));
            }
        }
    }
    if let Some(result) = gate(
        &candidate,
        "C:contradictions",
        contradiction.then_some(RejectionReason::TrainingContradiction),
        checked_training,
        budget,
        ledger,
    )? {
        return Ok(result);
    }
    let mut variation = false;
    for (i, atom) in candidate.pattern().iter().enumerate() {
        if let PatternAtom::Variable(_) = atom {
            let mut values = BTreeSet::new();
            for r in &support {
                budget.charge(K::OperationSteps, 1)?;
                values.insert(r.input.positions()[i]);
            }
            variation |= values.len() > 1;
        }
    }
    if let Some(result) = gate(
        &candidate,
        "D:variation",
        (!variation).then_some(RejectionReason::NoStructuralVariation),
        refs.clone(),
        budget,
        ledger,
    )? {
        return Ok(result);
    }
    let development = if development_gate { development } else { &[] };
    let challenge_ids: BTreeSet<RecordId> = development.iter().map(|r| r.record_id).collect();
    let overlap = challenge_ids
        .iter()
        .any(|id| training.iter().any(|r| r.record_id == *id));
    let mut positive = 0;
    let mut negative = 0;
    let mut valid = true;
    for r in development {
        budget.charge(K::DevelopmentChecks, 1)?;
        let prediction = candidate.predict(&query(r), budget, S::TrainingUpdate)?;
        let passed = if r.input != r.output {
            positive += 1;
            prediction == Some(r.output)
        } else {
            negative += 1;
            prediction.is_none()
        };
        valid &= passed;
        ledger.record_check(
            candidate.id(),
            passed,
            "E:development-case",
            vec![EvidenceRef::record(r.record_id)],
            CostVector::capture(budget, S::TrainingUpdate),
        )?;
    }
    let failure = if !development_gate {
        None
    } else if !valid {
        Some(RejectionReason::DevelopmentContradiction)
    } else if overlap
        || development.len() != 4
        || challenge_ids.len() != 4
        || positive != 1
        || negative != 3
    {
        Some(RejectionReason::IncompleteChallenges)
    } else {
        None
    };
    let challenges: Vec<_> = challenge_ids.into_iter().map(EvidenceRef::record).collect();
    if let Some(result) = gate(
        &candidate,
        "E:development",
        failure,
        challenges.clone(),
        budget,
        ledger,
    )? {
        return Ok(result);
    }
    if let Some(result) = gate(&candidate, "F:cost", None, refs.clone(), budget, ledger)? {
        return Ok(result);
    }
    let cost = CostVector::capture(budget, S::TrainingUpdate);
    let mut all_refs = refs;
    all_refs.extend(challenges.clone());
    ledger.record_admission(candidate.id(), all_refs, cost.clone())?;
    Ok(AdmissionResult::Admitted(Box::new(AdmittedProcedure {
        candidate,
        challenges,
        cost,
    })))
}
