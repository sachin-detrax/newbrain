use super::generator::GeneratedRecord;
use super::protocol::Span;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoldParse {
    pub relation_spans: Vec<Span>,
    pub boundaries: Vec<usize>,
}

fn tiling_boundaries(stream_len: usize, relations: &[(usize, usize)]) -> Vec<usize> {
    let mut points = Vec::new();
    for (start, end) in relations {
        if *start > 0 && *start < stream_len && !points.contains(start) {
            points.push(*start);
        }
        if *end > 0 && *end < stream_len && !points.contains(end) {
            points.push(*end);
        }
    }
    points.sort_unstable();
    points
}

pub fn gold_parses(record: &GeneratedRecord) -> Vec<GoldParse> {
    let n = record.wire_tokens.len();
    match record.ambiguous_core_offset {
        None => vec![GoldParse {
            boundaries: tiling_boundaries(
                n,
                &record
                    .latent_echo_spans
                    .iter()
                    .map(|(start, end)| (*start, *end))
                    .collect::<Vec<_>>(),
            ),
            relation_spans: record
                .latent_echo_spans
                .iter()
                .map(|(start, end)| Span::new(*start, *end))
                .collect(),
        }],
        Some(offset) => {
            let first = record.ambiguous_first_len as usize;
            let split = offset + first + 2;
            let single = GoldParse {
                relation_spans: vec![Span::new(offset, offset + 7)],
                boundaries: tiling_boundaries(n, &[(offset, offset + 7)]),
            };
            let pair = GoldParse {
                relation_spans: vec![Span::new(offset, split), Span::new(split, offset + 7)],
                boundaries: tiling_boundaries(n, &[(offset, split), (split, offset + 7)]),
            };
            vec![single, pair]
        }
    }
}

pub fn validate_record(record: &GeneratedRecord) -> Result<(), String> {
    let n = record.wire_tokens.len();
    if n == 0 {
        return Err(format!("record {} is empty", record.global_ordinal));
    }
    if n > 48 {
        return Err(format!("record {} has length {n}", record.global_ordinal));
    }
    for (start, end) in &record.latent_echo_spans {
        if start >= end || *end > n {
            return Err(format!(
                "record {} has out-of-range span",
                record.global_ordinal
            ));
        }
        let span = &record.wire_tokens[*start..*end];
        if span.len() < 3 || span.len() > 10 {
            return Err(format!(
                "record {} has span of length {}",
                record.global_ordinal,
                span.len()
            ));
        }
        if span.first() != span.last() {
            return Err(format!(
                "record {} has unequal span endpoints",
                record.global_ordinal
            ));
        }
    }
    match record.ambiguous_core_offset {
        None => {
            for start in 0..n {
                for end in start + 3..=n.min(start + 10) {
                    if record.wire_tokens[start] == record.wire_tokens[end - 1]
                        && !record.latent_echo_spans.contains(&(start, end))
                    {
                        return Err(format!(
                            "record {} has unexpected endpoint-equal subspan {start}:{end}",
                            record.global_ordinal
                        ));
                    }
                }
            }
        }
        Some(offset) => {
            if offset + 7 > n {
                return Err(format!(
                    "record {} has out-of-range core",
                    record.global_ordinal
                ));
            }
            if record.latent_echo_spans.len() != 2 {
                return Err(format!(
                    "record {} core lacks two spans",
                    record.global_ordinal
                ));
            }
            let core = &record.wire_tokens[offset..offset + 7];
            if core.first() != core.last() {
                return Err(format!(
                    "record {} core endpoints differ",
                    record.global_ordinal
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::generator::{generate_run, SymbolMode};
    use super::super::protocol::{PRIMARY_SCOPE_SEED, PRIMARY_WORLD_SEED, TRAINING_PARTITION_TAG};
    use super::*;

    const DEV_TAG: u64 = 0x4445_5600_0000_0001;
    const FINAL_TAG: u64 = 0x4649_4e00_0000_0001;

    fn primary_run() -> Vec<GeneratedRecord> {
        generate_run(
            PRIMARY_WORLD_SEED,
            PRIMARY_SCOPE_SEED,
            TRAINING_PARTITION_TAG,
            DEV_TAG,
            FINAL_TAG,
            SymbolMode::Primary,
        )
        .records
    }

    #[test]
    fn every_generated_record_validates() {
        for record in primary_run() {
            validate_record(&record).unwrap();
        }
    }

    #[test]
    fn every_permuted_record_validates() {
        let run = generate_run(
            PRIMARY_WORLD_SEED,
            PRIMARY_SCOPE_SEED,
            TRAINING_PARTITION_TAG,
            DEV_TAG,
            FINAL_TAG,
            SymbolMode::Permutation,
        );
        for record in run.records {
            validate_record(&record).unwrap();
        }
    }

    #[test]
    fn ambiguous_rows_have_two_parses_and_unambiguous_have_one() {
        for record in primary_run() {
            let parses = gold_parses(&record);
            match record.ambiguous_core_offset {
                None => assert_eq!(parses.len(), 1, "record {}", record.global_ordinal),
                Some(_) => assert_eq!(parses.len(), 2, "record {}", record.global_ordinal),
            }
        }
    }

    #[test]
    fn negative_rows_have_empty_boundaries() {
        let run = primary_run();
        for record in &run {
            if record.latent_echo_spans.is_empty() && record.ambiguous_core_offset.is_none() {
                assert_eq!(gold_parses(record)[0].boundaries, Vec::<usize>::new());
            }
        }
    }

    #[test]
    fn ambiguous_spans_match_the_declared_core_geometry() {
        for record in primary_run() {
            if let Some(offset) = record.ambiguous_core_offset {
                let parses = gold_parses(&record);
                assert_eq!(
                    parses[0].relation_spans,
                    vec![Span::new(offset, offset + 7)]
                );
                let split = offset + record.ambiguous_first_len as usize + 2;
                assert_eq!(
                    parses[1].relation_spans,
                    vec![Span::new(offset, split), Span::new(split, offset + 7)]
                );
                assert!(parses[1].boundaries.contains(&split));
            }
        }
    }
}
