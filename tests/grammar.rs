use sparse_structural_rule_learner::{
    budget::{BudgetConfig, BudgetState},
    dataset::V1Dataset,
    grammar::{enumerate_g0, enumerate_g1, OutputOp, PatternAtom},
    ledger::Ledger,
};

#[test]
fn two_examples_produce_one_shared_variable_rule() {
    let data = V1Dataset::fixed();
    let mut budget = BudgetState::new(BudgetConfig::v1());
    let rules = enumerate_g1(
        data.r1_training().as_slice(),
        &mut budget,
        &mut Ledger::new(),
    )
    .unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(
        rules[0].pattern(),
        &[
            PatternAtom::Variable(0),
            PatternAtom::Constant(8),
            PatternAtom::Variable(0)
        ]
    );
    assert_eq!(
        rules[0].output(),
        &[OutputOp::Copy(0), OutputOp::Emit(9), OutputOp::Copy(2)]
    );
    assert_eq!(rules[0].supports().len(), 2);
}

#[test]
fn exact_candidates_do_not_generalize() {
    let data = V1Dataset::fixed();
    let rules = enumerate_g0(
        data.r1_training().as_slice(),
        &mut BudgetState::new(BudgetConfig::v1()),
        &mut Ledger::new(),
    )
    .unwrap();
    assert_eq!(rules.len(), 2);
    assert!(rules.iter().all(|r| r
        .pattern()
        .iter()
        .all(|a| matches!(a, PatternAtom::Constant(_)))));
}

#[test]
fn unsupported_pairs_do_not_generate_rules() {
    let data = V1Dataset::fixed();
    let mut records = data.r1_training().as_slice().to_vec();
    records[1].action = 1;
    let rules = enumerate_g1(
        &records,
        &mut BudgetState::new(BudgetConfig::v1()),
        &mut Ledger::new(),
    )
    .unwrap();
    assert!(rules.is_empty());
}

#[test]
fn variable_bindings_transfer_and_reject_unequal_contexts() {
    use sparse_structural_rule_learner::{
        budget::BudgetScope,
        types::{Query, QueryId, Sequence},
    };
    let data = V1Dataset::fixed();
    let rules = enumerate_g1(
        data.r1_training().as_slice(),
        &mut BudgetState::new(BudgetConfig::v1()),
        &mut Ledger::new(),
    )
    .unwrap();
    let query = Query {
        input: Sequence::new([250, 8, 250]),
        action: 0,
        scope: 0,
        query_id: QueryId(99),
    };
    assert_eq!(
        rules[0]
            .predict(
                &query,
                &mut BudgetState::new(BudgetConfig::v1()),
                BudgetScope::Query
            )
            .unwrap(),
        Some(Sequence::new([250, 9, 250]))
    );
    let unequal = Query {
        input: Sequence::new([250, 8, 249]),
        ..query
    };
    assert_eq!(
        rules[0]
            .predict(
                &unequal,
                &mut BudgetState::new(BudgetConfig::v1()),
                BudgetScope::Query
            )
            .unwrap(),
        None
    );
}

#[test]
fn duplicate_ids_are_not_independent_evidence() {
    let data = V1Dataset::fixed();
    let one = data.r1_training().as_slice()[0].clone();
    assert!(enumerate_g1(
        &[one.clone(), one],
        &mut BudgetState::new(BudgetConfig::v1()),
        &mut Ledger::new()
    )
    .is_err());
}

#[test]
fn candidate_pair_budget_stops_generation() {
    use sparse_structural_rule_learner::budget::BudgetKind;
    let data = V1Dataset::fixed();
    let mut budget = BudgetState::new(BudgetConfig::v1());
    budget.charge(BudgetKind::CandidatePairs, 16).unwrap();
    assert!(enumerate_g1(
        data.r1_training().as_slice(),
        &mut budget,
        &mut Ledger::new()
    )
    .is_err());
    assert_eq!(budget.used(BudgetKind::CandidatePairs), 16);
}
