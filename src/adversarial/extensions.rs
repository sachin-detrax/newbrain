use super::support::{
    agrees, fixture, map_query, map_sequence, map_transition, result, shift_symbol, train,
    transition, Fixture,
};
use super::ExtensionResult;
use crate::admission::RejectionReason;
use crate::budget::{BudgetConfig, BudgetKind, BudgetState};
use crate::engine::{Learner, LearnerConfig, TrainingError};
use crate::types::{AbstentionReason, Query, QueryId, Response, Sequence};

#[cfg(test)]
#[path = "extension_tests.rs"]
mod tests;

pub(super) fn run() -> Result<Vec<ExtensionResult>, TrainingError> {
    let fixture = fixture();
    Ok(vec![
        symbol_permutation(&fixture)?,
        reordered_training(&fixture)?,
        irrelevant_record(&fixture)?,
        duplicate_source_id(&fixture)?,
        separate_scope_contradiction(&fixture)?,
        removed_development_challenge(&fixture)?,
        corrupted_development_output(&fixture)?,
        exhausted_query_budget(&fixture)?,
        larger_symbol_cardinality(&fixture)?,
        multiple_unrelated_rules(&fixture)?,
    ])
}

fn symbol_permutation(fixture: &Fixture) -> Result<ExtensionResult, TrainingError> {
    let training: Vec<_> = fixture
        .training
        .iter()
        .map(|record| map_transition(record, shift_symbol))
        .collect();
    let development: Vec<_> = fixture
        .development
        .iter()
        .map(|record| map_transition(record, shift_symbol))
        .collect();
    let snapshot = train(LearnerConfig::v1(), &training, &development)?;
    let (passed, detail) = agrees(
        &snapshot,
        &map_query(fixture.query, shift_symbol),
        map_sequence(fixture.expected, shift_symbol),
    );
    Ok(result(
        "symbol-permutation",
        "symbol invariance",
        passed,
        detail,
    ))
}

fn reordered_training(fixture: &Fixture) -> Result<ExtensionResult, TrainingError> {
    let mut reordered = fixture.training.clone();
    reordered.reverse();
    let snapshot = train(LearnerConfig::v1(), &reordered, &fixture.development)?;
    let (passed, detail) = agrees(&snapshot, &fixture.query, fixture.expected);
    Ok(result(
        "reordered-training",
        "order robustness",
        passed,
        detail,
    ))
}

fn irrelevant_record(fixture: &Fixture) -> Result<ExtensionResult, TrainingError> {
    let mut records = fixture.training.clone();
    records.push(transition(25, [70, 71, 70], 1, [70, 72, 70], 7));
    let snapshot = train(LearnerConfig::v1(), &records, &fixture.development)?;
    let (passed, detail) = agrees(&snapshot, &fixture.query, fixture.expected);
    Ok(result(
        "irrelevant-record",
        "evidence independence",
        passed,
        detail,
    ))
}

fn duplicate_source_id(fixture: &Fixture) -> Result<ExtensionResult, TrainingError> {
    let mut records = fixture.training.clone();
    records.push(fixture.training[0].clone());
    let detail = match train(LearnerConfig::v1(), &records, &fixture.development) {
        Err(TrainingError::RepeatedRecord(id)) => {
            format!("rejected duplicate source record {id:?}")
        }
        Err(error) => format!("unexpected training error: {error:?}"),
        Ok(_) => "duplicate source record was accepted".to_string(),
    };
    let passed = detail.starts_with("rejected duplicate source record");
    Ok(result(
        "duplicate-source-id",
        "evidence independence",
        passed,
        detail,
    ))
}

fn separate_scope_contradiction(fixture: &Fixture) -> Result<ExtensionResult, TrainingError> {
    let mut records = fixture.training.clone();
    records.push(transition(26, [0, 8, 0], 0, [0, 7, 0], 1));
    let snapshot = train(LearnerConfig::v1(), &records, &fixture.development)?;
    let (passed, detail) = agrees(&snapshot, &fixture.query, fixture.expected);
    Ok(result(
        "separate-scope-contradiction",
        "scope control",
        passed,
        detail,
    ))
}

fn removed_development_challenge(fixture: &Fixture) -> Result<ExtensionResult, TrainingError> {
    let reduced: Vec<_> = fixture.development.iter().take(3).cloned().collect();
    let mut gated = Learner::new(LearnerConfig::v1());
    gated.train_phase_a(&fixture.training, &reduced)?;
    let ungated = train(
        LearnerConfig::without_development_gate(),
        &fixture.training,
        &reduced,
    )?;
    let gated_execution = gated.freeze().predict(&fixture.query);
    let (ungated_passed, ungated_detail) = agrees(&ungated, &fixture.query, fixture.expected);
    let rejection = gated
        .ledger()
        .rejected_candidates()
        .into_iter()
        .find(|event| {
            event.reason == format!("{:?}", RejectionReason::IncompleteChallenges)
                && ungated
                    .procedures()
                    .iter()
                    .any(|procedure| event.procedure_ids.as_slice() == [procedure.id()])
        });
    let abstained = matches!(&gated_execution.response, Response::Abstention(_));
    let abstained = abstained
        && gated_execution.response.reason() == Some(AbstentionReason::NoApplicableProcedure);
    let passed = rejection.is_some() && abstained && ungated_passed;
    Ok(result(
        "removed-development-challenge",
        "under-testing detection",
        passed,
        format!(
            "main={:?}; A1={ungated_detail}; admission_rejection={:?}",
            gated_execution.response,
            rejection.map(|event| (&event.reason, &event.procedure_ids, &event.evidence_refs))
        ),
    ))
}

fn corrupted_development_output(fixture: &Fixture) -> Result<ExtensionResult, TrainingError> {
    let mut corrupted = fixture.development.clone();
    corrupted[0].output = Sequence::new([3, 8, 3]);
    let gated = train(LearnerConfig::v1(), &fixture.training, &corrupted)?;
    let ungated = train(
        LearnerConfig::without_development_gate(),
        &fixture.training,
        &corrupted,
    )?;
    let gated_execution = gated.predict(&fixture.query);
    let (ungated_passed, ungated_detail) = agrees(&ungated, &fixture.query, fixture.expected);
    let passed = matches!(&gated_execution.response, Response::Abstention(_)) && ungated_passed;
    let passed = passed
        && gated_execution.response.reason() == Some(AbstentionReason::NoApplicableProcedure);
    Ok(result(
        "corrupted-development-output",
        "corruption handling",
        passed,
        format!("main={:?}; A1={ungated_detail}", gated_execution.response),
    ))
}

fn exhausted_query_budget(fixture: &Fixture) -> Result<ExtensionResult, TrainingError> {
    let snapshot = train(LearnerConfig::v1(), &fixture.training, &fixture.development)?;
    let mut budget = BudgetState::new(BudgetConfig::v1());
    budget.charge_query(BudgetKind::OperationSteps, 64)?;
    let execution = snapshot.predict_with_budget(&fixture.query, &mut budget);
    let passed =
        execution.budget_error.is_some() && matches!(&execution.response, Response::Abstention(_));
    let passed =
        passed && execution.response.reason() == Some(AbstentionReason::UnsupportedByBudget);
    Ok(result(
        "query-exhaustion",
        "budget honesty",
        passed,
        format!("{:?}", execution.response),
    ))
}

fn larger_symbol_cardinality(fixture: &Fixture) -> Result<ExtensionResult, TrainingError> {
    let mut records = fixture.training.clone();
    records.push(transition(25, [20, 8, 20], 0, [20, 9, 20], 0));
    let query = Query {
        input: Sequence::new([21, 8, 21]),
        action: 0,
        scope: 0,
        query_id: QueryId(101),
    };
    let snapshot = train(LearnerConfig::v1(), &records, &fixture.development)?;
    let (passed, detail) = agrees(&snapshot, &query, Sequence::new([21, 9, 21]));
    Ok(result(
        "larger-symbol-cardinality",
        "scaling",
        passed,
        detail,
    ))
}

fn multiple_unrelated_rules(fixture: &Fixture) -> Result<ExtensionResult, TrainingError> {
    let unrelated = vec![
        transition(25, [30, 12, 30], 0, [30, 13, 30], 2),
        transition(26, [31, 12, 31], 0, [31, 13, 31], 2),
        transition(27, [40, 14, 40], 1, [40, 15, 40], 3),
        transition(28, [41, 14, 41], 1, [41, 15, 41], 3),
    ];
    let mut learner = Learner::new(LearnerConfig::without_development_gate());
    learner.train_phase_a(&fixture.training, &fixture.development)?;
    let before = learner.freeze();
    let (before_passed, before_detail) = agrees(&before, &fixture.query, fixture.expected);
    learner.train_phase_b(&unrelated, &[])?;
    let after = learner.freeze();
    let (after_passed, after_detail) = agrees(&after, &fixture.query, fixture.expected);
    let passed =
        before_passed && after_passed && after.procedures().len() >= before.procedures().len() + 2;
    Ok(result(
        "multiple-unrelated-rules",
        "scaling",
        passed,
        format!(
            "before_count={}; after_count={}; before={before_detail}; after={after_detail}",
            before.procedures().len(),
            after.procedures().len()
        ),
    ))
}
