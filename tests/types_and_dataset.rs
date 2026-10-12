use std::collections::BTreeSet;

use sparse_structural_rule_learner::dataset::V1Dataset;
use sparse_structural_rule_learner::types::{
    AbstentionReason, Action, EvidenceRef, ProcedureId, Query, QueryId, RecordId, Response, Scope,
    Sequence, Symbol, Transition,
};

#[test]
fn sequence_is_exactly_three_opaque_symbols() {
    let sequence = Sequence::new([7, 2, 7]);
    assert_eq!(sequence.positions(), [7, 2, 7]);
    assert_eq!(sequence.get(0), Some(7));
    assert_eq!(sequence.get(3), None);
}

#[test]
fn prediction_and_abstention_are_distinct_responses() {
    let prediction = Response::prediction(
        Sequence::new([7, 3, 7]),
        ProcedureId(1),
        vec![EvidenceRef::record(RecordId(1))],
    );
    let abstention = Response::abstention(AbstentionReason::NoApplicableProcedure, vec![], vec![]);

    assert!(matches!(prediction, Response::Prediction(_)));
    assert!(matches!(abstention, Response::Abstention(_)));
}

#[test]
fn prediction_many_preserves_output_procedure_ids_and_every_evidence_reference() {
    let output = Sequence::new([7, 3, 7]);
    let procedure_ids = vec![ProcedureId(1), ProcedureId(2)];
    let evidence_refs = vec![
        EvidenceRef::record(RecordId(10)),
        EvidenceRef::record(RecordId(11)),
        EvidenceRef::record(RecordId(12)),
    ];
    let prediction =
        Response::prediction_many(output, procedure_ids.clone(), evidence_refs.clone())
            .expect("a non-empty procedure list should construct a prediction");

    assert!(matches!(&prediction, Response::Prediction(_)));
    assert_eq!(prediction.output(), Some(output));
    assert_eq!(prediction.procedure_ids(), procedure_ids.as_slice());
    assert_eq!(prediction.evidence_refs(), evidence_refs.as_slice());

    assert!(Response::prediction_many(Sequence::new([7, 3, 7]), vec![], vec![]).is_none());
}

#[test]
fn transition_and_query_keep_training_output_separate_from_query() {
    let transition = Transition {
        input: Sequence::new([7, 2, 7]),
        action: 4 as Action,
        output: Sequence::new([7, 3, 7]),
        scope: 1 as Scope,
        record_id: RecordId(9),
    };
    let query = Query {
        input: transition.input,
        action: transition.action,
        scope: transition.scope,
        query_id: QueryId(4),
    };

    assert_eq!(transition.output, Sequence::new([7, 3, 7]));
    assert_eq!(query.input, transition.input);
}

#[test]
fn opaque_ids_are_orderable_and_evidence_preserves_record_identity() {
    let symbol: Symbol = 7;
    assert_eq!(symbol, 7);
    assert!(QueryId(1) < QueryId(2));
    assert_eq!(EvidenceRef::record(RecordId(9)).record_id(), RecordId(9));
}

#[test]
fn primary_partitions_have_the_specified_records() {
    let data = V1Dataset::fixed();

    assert_eq!(data.r1_training().len(), 2);
    assert_eq!(data.r1_development().len(), 4);
    assert_eq!(data.r1_final_queries().count(), 8);
    assert_eq!(data.r2_training().len(), 2);
    assert_eq!(data.r2_development().len(), 4);
    assert_eq!(data.r2_final_queries().count(), 4);

    assert_eq!(
        data.r1_training()
            .iter()
            .map(|record| record.record_id)
            .collect::<Vec<_>>(),
        vec![RecordId(1), RecordId(2)]
    );
    assert_eq!(
        data.r1_development()
            .iter()
            .map(|record| record.record_id)
            .collect::<Vec<_>>(),
        vec![RecordId(3), RecordId(4), RecordId(5), RecordId(6)]
    );
    assert_eq!(
        data.r1_final_queries()
            .map(|query| query.query_id)
            .collect::<Vec<_>>(),
        vec![
            QueryId(7),
            QueryId(8),
            QueryId(9),
            QueryId(10),
            QueryId(11),
            QueryId(12),
            QueryId(13),
            QueryId(14),
        ]
    );
    assert_eq!(
        data.r2_training()
            .iter()
            .map(|record| record.record_id)
            .collect::<Vec<_>>(),
        vec![RecordId(15), RecordId(16)]
    );
    assert_eq!(
        data.r2_development()
            .iter()
            .map(|record| record.record_id)
            .collect::<Vec<_>>(),
        vec![RecordId(17), RecordId(18), RecordId(19), RecordId(20)]
    );
    assert_eq!(
        data.r2_development()
            .iter()
            .map(|record| (record.input, record.action, record.output, record.scope))
            .collect::<Vec<_>>(),
        vec![
            (Sequence::new([2, 10, 2]), 1, Sequence::new([2, 11, 2]), 1,),
            (Sequence::new([2, 10, 3]), 1, Sequence::new([2, 10, 3]), 1,),
            (Sequence::new([2, 11, 2]), 1, Sequence::new([2, 11, 2]), 1,),
            (Sequence::new([2, 10, 2]), 0, Sequence::new([2, 10, 2]), 1,),
        ]
    );
    assert_eq!(
        data.r2_final_queries()
            .map(|query| query.query_id)
            .collect::<Vec<_>>(),
        vec![QueryId(21), QueryId(22), QueryId(23), QueryId(24)]
    );
}

#[test]
fn fixed_dataset_fingerprint_is_deterministic() {
    let first = V1Dataset::fixed();
    let second = V1Dataset::fixed();

    assert_eq!(first.fingerprint(), 13_935_465_607_801_348_367);
    assert_eq!(first.fingerprint(), second.fingerprint());
}

#[test]
fn learner_inputs_do_not_contain_final_outputs() {
    let data = V1Dataset::fixed();
    let expected_learner_ids = [
        RecordId(1),
        RecordId(2),
        RecordId(3),
        RecordId(4),
        RecordId(5),
        RecordId(6),
        RecordId(15),
        RecordId(16),
        RecordId(17),
        RecordId(18),
        RecordId(19),
        RecordId(20),
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let final_query_ids = data
        .r1_final_queries()
        .chain(data.r2_final_queries())
        .map(|query| query.query_id)
        .collect::<Vec<_>>();

    let actual_learner_ids = data
        .learner_transitions()
        .map(|record| record.record_id)
        .collect::<BTreeSet<_>>();

    assert_eq!(actual_learner_ids, expected_learner_ids);
    assert_eq!(final_query_ids.len(), 12);
}
