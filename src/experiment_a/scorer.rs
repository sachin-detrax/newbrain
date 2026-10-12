use super::generator::GeneratedRecord;
use super::oracle::gold_parses;
use super::protocol::{AnalysisReason, AnalysisResponse, Span};
use super::runner::StreamOutcome;
use super::schedule::{Stratum, FINAL_ROWS};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FinalMetrics {
    pub tp: u64,
    pub fs: u64,
    pub fm: u64,
    pub records_fs: u64,
    pub records_fm: u64,
    pub selected: u64,
    pub exact: u64,
    pub unambiguous_total: u64,
    pub l_success: u64,
    pub m_success: u64,
    pub o_correct: u64,
    pub r_correct: u64,
    pub n_fp: u64,
    pub a_honest: u64,
    pub silent_empty: u64,
    pub post_exhaustion_confident: u64,
    pub orphan_refs: u64,
    pub late_refs: u64,
    pub pre_decision_ok: u64,
    pub pre_decision_total: u64,
}

fn boundaries_of(response: &AnalysisResponse) -> (BTreeSet<usize>, bool) {
    match response {
        AnalysisResponse::Selected { parse } => (parse.boundaries.iter().copied().collect(), true),
        AnalysisResponse::Alternatives { parses, .. } => {
            (parses[0].boundaries.iter().copied().collect(), true)
        }
        AnalysisResponse::Abstain { .. } => (BTreeSet::new(), false),
    }
}

fn relation_spans_of(response: &AnalysisResponse) -> Vec<Span> {
    match response {
        AnalysisResponse::Selected { parse } => {
            parse.relation_uses.iter().map(|use_| use_.span).collect()
        }
        _ => Vec::new(),
    }
}

fn has_relations(response: &AnalysisResponse) -> bool {
    match response {
        AnalysisResponse::Selected { parse } => !parse.relation_uses.is_empty(),
        AnalysisResponse::Alternatives { .. } => true,
        AnalysisResponse::Abstain { .. } => false,
    }
}

pub fn score_final(
    records: &[GeneratedRecord],
    outcomes: &[StreamOutcome],
    admitted: bool,
) -> FinalMetrics {
    let mut metrics = FinalMetrics::default();
    let mut exhausted_seen = false;
    for outcome in outcomes.iter() {
        if let AnalysisResponse::Abstain {
            reason: AnalysisReason::ResourceLimit,
            ..
        } = &outcome.decision.response
        {
            exhausted_seen = true;
        } else if exhausted_seen && has_relations(&outcome.decision.response) {
            metrics.post_exhaustion_confident += 1;
        }
        if let AnalysisResponse::Selected { parse } = &outcome.decision.response {
            if parse.spans.is_empty() {
                metrics.silent_empty += 1;
            }
        }
        for reference in outcome.decision.evidence_refs.iter() {
            metrics.pre_decision_total += 1;
            if *reference >= 1 && *reference <= 96 {
                metrics.pre_decision_ok += 1;
            } else if *reference > 96 {
                metrics.late_refs += 1;
            } else {
                metrics.orphan_refs += 1;
            }
        }
    }
    let finals = records.len().saturating_sub(96);
    for index in 0..finals {
        let record = &records[96 + index];
        let outcome = &outcomes[96 + index];
        let row = &FINAL_ROWS[(record.row_ordinal - 1) as usize];
        let stratum = row.stratum.unwrap();
        let gold = gold_parses(record);
        let (learner_set, selected) = boundaries_of(&outcome.decision.response);
        match stratum {
            Stratum::Ambiguity => {
                if ambiguity_honest(&outcome.decision.response, &gold) {
                    metrics.a_honest += 1;
                }
            }
            Stratum::Negative => {
                if has_relations(&outcome.decision.response) {
                    metrics.n_fp += 1;
                }
                let gold_set: BTreeSet<usize> = BTreeSet::new();
                let (learner_set, selected) = boundaries_of(&outcome.decision.response);
                metrics.fs += learner_set.len() as u64;
                if !learner_set.is_empty() {
                    metrics.records_fs += 1;
                }
                if selected {
                    metrics.selected += 1;
                }
                if learner_set == gold_set {
                    metrics.exact += 1;
                }
                metrics.unambiguous_total += 1;
            }
            _ => {
                let gold_set: BTreeSet<usize> = gold[0].boundaries.iter().copied().collect();
                let tp = learner_set.intersection(&gold_set).count() as u64;
                let fs = learner_set.difference(&gold_set).count() as u64;
                let fm = gold_set.difference(&learner_set).count() as u64;
                metrics.tp += tp;
                metrics.fs += fs;
                metrics.fm += fm;
                if fs > 0 {
                    metrics.records_fs += 1;
                }
                if fm > 0 {
                    metrics.records_fm += 1;
                }
                if selected {
                    metrics.selected += 1;
                }
                if learner_set == gold_set {
                    metrics.exact += 1;
                }
                metrics.unambiguous_total += 1;
                let exact_spans = selected
                    && learner_set == gold_set
                    && spans_match(&outcome.decision.response, record)
                    && !outcome.decision.evidence_refs.is_empty()
                    && admitted;
                match stratum {
                    Stratum::Length if exact_spans => {
                        metrics.l_success += 1;
                    }
                    Stratum::Composition if exact_spans => {
                        metrics.m_success += 1;
                    }
                    Stratum::Optional if exact_spans => {
                        metrics.o_correct += 1;
                    }
                    Stratum::Reorder if exact_spans => {
                        metrics.r_correct += 1;
                    }
                    _ => {}
                }
            }
        }
    }
    metrics
}

fn spans_match(response: &AnalysisResponse, record: &GeneratedRecord) -> bool {
    let mut learner: Vec<(usize, usize)> = relation_spans_of(response)
        .iter()
        .map(|span| (span.start, span.end))
        .collect();
    learner.sort_unstable();
    let mut gold: Vec<(usize, usize)> = record.latent_echo_spans.to_vec();
    gold.sort_unstable();
    learner == gold
}

fn ambiguity_honest(response: &AnalysisResponse, gold: &[super::oracle::GoldParse]) -> bool {
    match response {
        AnalysisResponse::Alternatives { parses, .. } => {
            if gold.len() != 2 {
                return false;
            }
            let learner_sets: BTreeSet<Vec<usize>> = parses
                .iter()
                .map(|parse| {
                    let mut boundaries: Vec<usize> = parse.boundaries.clone();
                    boundaries.sort_unstable();
                    boundaries
                })
                .collect();
            let mut gold_sets: BTreeSet<Vec<usize>> = BTreeSet::new();
            for parse in gold {
                let mut boundaries = parse.boundaries.clone();
                boundaries.sort_unstable();
                gold_sets.insert(boundaries);
            }
            learner_sets == gold_sets
        }
        AnalysisResponse::Abstain {
            reason,
            hypothesis_refs,
            ..
        } => {
            if *reason != AnalysisReason::AmbiguousSegmentation {
                return false;
            }
            let distinct: BTreeSet<u64> = hypothesis_refs.iter().copied().collect();
            distinct.len() >= 2
        }
        AnalysisResponse::Selected { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::a1::A1Learner;
    use super::super::generator::{generate_run, SymbolMode};
    use super::super::protocol::{PRIMARY_SCOPE_SEED, PRIMARY_WORLD_SEED, TRAINING_PARTITION_TAG};
    use super::super::runner::drive_learner;
    use super::*;

    const DEV_TAG: u64 = 0x4445_5600_0000_0001;
    const FINAL_TAG: u64 = 0x4649_4e00_0000_0001;

    #[test]
    fn a1_primary_metrics_pass_behavioral_thresholds() {
        let run = generate_run(
            PRIMARY_WORLD_SEED,
            PRIMARY_SCOPE_SEED,
            TRAINING_PARTITION_TAG,
            DEV_TAG,
            FINAL_TAG,
            SymbolMode::Primary,
        );
        let (trace, _) = drive_learner(&run.records, A1Learner::new()).unwrap();
        let frozen_admitted = true;
        let metrics = score_final(&run.records, &trace.outcomes, frozen_admitted);
        assert_eq!(metrics.unambiguous_total, 52);
        let precision_den = metrics.tp + metrics.fs;
        assert!(precision_den > 0);
        assert!(
            metrics.tp * 10 >= precision_den * 9,
            "precision {}/{}",
            metrics.tp,
            precision_den
        );
        let recall_den = metrics.tp + metrics.fm;
        assert!(
            metrics.tp * 10 >= recall_den * 9,
            "recall {}/{}",
            metrics.tp,
            recall_den
        );
        assert!(metrics.records_fm * 10 <= 52);
        assert!(metrics.records_fs * 10 <= 52);
        assert!(metrics.selected >= 48);
        assert!(metrics.l_success + metrics.m_success >= 24);
        assert!(metrics.l_success >= 13);
        assert!(metrics.m_success >= 9);
        assert!(metrics.o_correct >= 6);
        assert!(metrics.r_correct >= 6);
        assert_eq!(metrics.n_fp, 0);
        assert!(metrics.a_honest >= 10);
        assert_eq!(metrics.late_refs, 0);
        assert_eq!(metrics.post_exhaustion_confident, 0);
    }
}
