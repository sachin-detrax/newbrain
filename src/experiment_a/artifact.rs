use super::a1::v1_enumerate;
use super::a3::LibraryReport;
use super::gates::{FailureClassification, GateResult};
use super::generator::GeneratedRecord;
use super::oracle::gold_parses;
use super::protocol::{analysis_json, ids_json, json_array, json_object, json_string, sha256_hex};
use super::runner::RunTrace;
use super::schedule::{Partition, FINAL_ROWS};
use super::scorer::FinalMetrics;

pub const PROTOCOL_MD: &str = include_str!(
    "../../docs/protocols/variable-length-boundary-discovery-experiment-a-protocol-v1.0.md"
);
pub const SPECIFICATION_MD: &str =
    include_str!("../../docs/specs/variable-length-boundary-discovery-experiment-a.md");

pub fn metric_json(numerator: u64, denominator: u64) -> String {
    if denominator == 0 {
        return json_object(vec![
            ("denominator", "0".to_owned()),
            ("numerator", numerator.to_string()),
            ("rate_micros", "null".to_owned()),
        ]);
    }
    let rate = ((u128::from(numerator) * 1_000_000) / u128::from(denominator)) as u64;
    json_object(vec![
        ("denominator", denominator.to_string()),
        ("numerator", numerator.to_string()),
        ("rate_micros", rate.to_string()),
    ])
}

fn record_label(record: &GeneratedRecord) -> String {
    let prefix = match record.partition {
        Partition::Training => "T",
        Partition::Development => "D",
        Partition::Final => "F",
    };
    format!("{prefix}{:03}", record.row_ordinal)
}

pub fn stream_events_jsonl(records: &[GeneratedRecord]) -> String {
    let mut out = String::new();
    for record in records {
        let atoms = json_array(
            record
                .wire_tokens
                .iter()
                .map(|atom| json_string(&atom.to_string())),
        );
        out.push_str(&json_object(vec![
            ("atoms", atoms),
            ("label", json_string(&record_label(record))),
            ("ordinal", record.global_ordinal.to_string()),
            ("redraws", record.redraws.to_string()),
            ("run_scope", json_string(&record.run_scope.to_string())),
            (
                "stream_scope",
                json_string(&record.stream_scope.to_string()),
            ),
        ]));
        out.push('\n');
    }
    out
}

pub fn analysis_trace_jsonl(trace: &RunTrace) -> String {
    let mut out = String::new();
    for outcome in &trace.outcomes {
        out.push_str(&json_object(vec![
            ("analysis", analysis_json(&outcome.decision)),
            ("ordinal", outcome.global_ordinal.to_string()),
        ]));
        out.push('\n');
    }
    out
}

pub fn candidate_events_jsonl(records: &[GeneratedRecord], enumerates: bool) -> String {
    let mut out = String::new();
    for record in records.iter().take(96) {
        let (matches, considered) = if enumerates {
            v1_enumerate(&record.wire_tokens)
        } else {
            (Vec::new(), 0)
        };
        let spans = json_array(matches.iter().map(|(start, end)| {
            json_object(vec![("end", end.to_string()), ("start", start.to_string())])
        }));
        out.push_str(&json_object(vec![
            ("considered", considered.to_string()),
            ("matches", spans),
            ("ordinal", record.global_ordinal.to_string()),
        ]));
        out.push('\n');
    }
    out
}

pub fn relation_records_jsonl(trace: &RunTrace, admitted_fingerprint: Option<&str>) -> String {
    match admitted_fingerprint {
        None => String::new(),
        Some(fingerprint) => {
            let support: Vec<u64> = trace
                .outcomes
                .iter()
                .skip(96)
                .find(|outcome| !outcome.decision.evidence_refs.is_empty())
                .map(|outcome| outcome.decision.evidence_refs.clone())
                .unwrap_or_default();
            let mut line = json_object(vec![
                (
                    "grammar_version",
                    json_string(super::protocol::GRAMMAR_VERSION),
                ),
                ("model_fingerprint", json_string(fingerprint)),
                (
                    "relation_version",
                    json_string(super::a1::A1_RELATION_VERSION),
                ),
                ("support", ids_json(support)),
            ]);
            line.push('\n');
            line
        }
    }
}

pub fn library_records_jsonl(library: Option<&LibraryReport>) -> String {
    let mut out = String::new();
    for entry in library
        .map(|report| report.entries.as_slice())
        .unwrap_or(&[])
    {
        let uses = json_array(entry.uses.iter().map(|(ordinal, payload)| {
            json_object(vec![
                ("ordinal", ordinal.to_string()),
                ("payload", payload.to_string()),
            ])
        }));
        let lengths = json_array(
            entry
                .payload_lengths
                .iter()
                .map(|length| length.to_string()),
        );
        let records = ids_json(entry.records.iter().copied());
        out.push_str(&json_object(vec![
            ("payload_lengths", lengths),
            ("records", records),
            ("uses", uses),
        ]));
        out.push('\n');
    }
    out
}

pub fn evidence_records_jsonl(records: &[GeneratedRecord]) -> String {
    let mut out = String::new();
    for record in records {
        let mut bytes = Vec::new();
        for atom in &record.wire_tokens {
            bytes.extend_from_slice(&atom.to_be_bytes());
        }
        out.push_str(&json_object(vec![
            ("id", json_string(&record.global_ordinal.to_string())),
            ("label", json_string(&record_label(record))),
            ("wire_sha", json_string(&sha256_hex(&bytes))),
        ]));
        out.push('\n');
    }
    out
}

pub fn metrics_json(metrics: &FinalMetrics) -> String {
    let mut fields: Vec<(&str, String)> = vec![
        ("ambiguity_honesty", metric_json(metrics.a_honest, 12)),
        (
            "boundary_precision",
            metric_json(metrics.tp, metrics.tp + metrics.fs),
        ),
        (
            "boundary_recall",
            metric_json(metrics.tp, metrics.tp + metrics.fm),
        ),
        ("composition_transfer", metric_json(metrics.m_success, 12)),
        (
            "exact_boundary_sets",
            metric_json(metrics.exact, metrics.unambiguous_total),
        ),
        ("false_merge_records", metric_json(metrics.records_fm, 52)),
        ("false_split_records", metric_json(metrics.records_fs, 52)),
        ("length_transfer", metric_json(metrics.l_success, 16)),
        ("negative_false_positives", metric_json(metrics.n_fp, 8)),
        ("optional_correct", metric_json(metrics.o_correct, 8)),
        ("reorder_correct", metric_json(metrics.r_correct, 8)),
        ("selected_coverage", metric_json(metrics.selected, 52)),
        (
            "total_transfer",
            metric_json(metrics.l_success + metrics.m_success, 28),
        ),
    ];
    fields.push(("unambiguous_total", metrics.unambiguous_total.to_string()));
    json_object(fields)
}

pub fn acceptance_json(gates: &[GateResult]) -> String {
    let entries = json_array(gates.iter().map(|gate| {
        json_object(vec![
            ("denominator", gate.denominator.to_string()),
            ("detail", json_string(&gate.detail)),
            ("id", json_string(gate.id)),
            ("numerator", gate.numerator.to_string()),
            ("passed", gate.passed.to_string()),
        ])
    }));
    json_object(vec![("gates", entries)])
}

pub fn classification_for_gate(id: &str) -> &'static str {
    if id == "A1-P1" {
        FailureClassification::FalsifiedByProtocol.as_str()
    } else if matches!(id, "A1-P3" | "A1-P4" | "A1-P5" | "A1-P6") {
        FailureClassification::FalsifiedByBoundaryGeneralization.as_str()
    } else if matches!(id, "A1-P7" | "A1-P8") {
        FailureClassification::FalsifiedByStructuralTransfer.as_str()
    } else if id == "A1-P9" {
        FailureClassification::FalsifiedByFalsePositiveControl.as_str()
    } else if id == "A1-P10" {
        FailureClassification::FalsifiedByAmbiguityHonesty.as_str()
    } else if id == "A1-P11" {
        FailureClassification::FalsifiedByEvidenceTraceability.as_str()
    } else if id == "A1-P12" {
        FailureClassification::FalsifiedByBudget.as_str()
    } else if id == "A1-P13" {
        FailureClassification::FalsifiedByResourceLimit.as_str()
    } else {
        FailureClassification::ComparatorLibraryFailed.as_str()
    }
}

pub fn failures_jsonl(gates: &[GateResult]) -> String {
    let mut out = String::new();
    for gate in gates.iter().filter(|gate| !gate.passed) {
        out.push_str(&json_object(vec![
            (
                "classification",
                json_string(classification_for_gate(gate.id)),
            ),
            ("detail", json_string(&gate.detail)),
            ("id", json_string(gate.id)),
        ]));
        out.push('\n');
    }
    out
}

pub fn logical_resource_report_json(
    trace: &RunTrace,
    records: &[GeneratedRecord],
    library_entries: u64,
    admitted_relations: u64,
) -> String {
    json_object(vec![
        ("active_hypotheses", "0".to_owned()),
        ("alignment_operations", "0".to_owned()),
        ("artifact_bytes", "0".to_owned()),
        ("evidence_records", records.len().to_string()),
        ("library_entries", library_entries.to_string()),
        ("received_atoms", trace.received_atoms_run.to_string()),
        ("relation_candidates", "0".to_owned()),
        ("relation_records", admitted_relations.to_string()),
        (
            "span_boundary_pairs",
            trace.analysis_operation_cost.to_string(),
        ),
    ])
}

pub fn physical_resource_report_json(
    wall_clock_ns: u128,
    memory: (Option<u64>, super::physical::MemoryState),
) -> String {
    json_object(vec![
        ("cpu_time_ns", wall_clock_ns.to_string()),
        ("gpu_count", "0".to_owned()),
        ("network_connection_count", "0".to_owned()),
        (
            "peak_process_memory_bytes",
            memory
                .0
                .map(|bytes| bytes.to_string())
                .unwrap_or_else(|| "null".to_owned()),
        ),
        ("peak_process_memory_state", json_string(memory.1.as_str())),
        ("process_count", "1".to_owned()),
        ("wall_clock_ns", wall_clock_ns.to_string()),
        ("worker_count", "1".to_owned()),
    ])
}

pub fn generator_manifest_json(
    variant: &str,
    mode: &str,
    world_seed: u64,
    scope_seed: u64,
    records: &[GeneratedRecord],
) -> String {
    let entries = json_array(records.iter().map(|record| {
        let latent = json_array(record.latent_echo_spans.iter().map(|(start, end)| {
            json_object(vec![("end", end.to_string()), ("start", start.to_string())])
        }));
        let noise = json_array(record.noise_vector.iter().map(|count| count.to_string()));
        let order = json_array(record.emitted_order.iter().map(|slot| slot.to_string()));
        json_object(vec![
            ("emitted_order", order),
            ("label", json_string(&record_label(record))),
            ("latent_echo_spans", latent),
            ("noise_vector", noise),
            ("ordinal", record.global_ordinal.to_string()),
            ("redraws", record.redraws.to_string()),
        ])
    }));
    json_object(vec![
        (
            "generator_version",
            json_string(super::protocol::GENERATOR_VERSION),
        ),
        ("mode", json_string(mode)),
        ("records", entries),
        ("scope_seed", json_string(&scope_seed.to_string())),
        ("variant", json_string(variant)),
        ("world_seed", json_string(&world_seed.to_string())),
    ])
}

pub fn score_input_json(records: &[GeneratedRecord]) -> String {
    let mut out = String::new();
    for record in records.iter().skip(96) {
        let parses = gold_parses(record);
        let sets =
            json_array(parses.iter().map(|parse| {
                json_array(parse.boundaries.iter().map(|boundary| boundary.to_string()))
            }));
        let spans = json_array(record.latent_echo_spans.iter().map(|(start, end)| {
            json_object(vec![("end", end.to_string()), ("start", start.to_string())])
        }));
        let row = &FINAL_ROWS[(record.row_ordinal - 1) as usize];
        let stratum = match row.stratum.unwrap() {
            super::schedule::Stratum::Length => "L",
            super::schedule::Stratum::Composition => "M",
            super::schedule::Stratum::Optional => "O",
            super::schedule::Stratum::Reorder => "R",
            super::schedule::Stratum::Negative => "N",
            super::schedule::Stratum::Ambiguity => "A",
        };
        out.push_str(&json_object(vec![
            ("gold_boundary_sets", sets),
            ("gold_relation_spans", spans),
            ("label", json_string(&record_label(record))),
            ("ordinal", record.global_ordinal.to_string()),
            ("stratum", json_string(stratum)),
        ]));
        out.push('\n');
    }
    out
}

pub fn sha256sums_text(files: &[(String, String)]) -> String {
    let mut sorted: Vec<(&String, &String)> =
        files.iter().map(|(path, bytes)| (path, bytes)).collect();
    sorted.sort_by(|first, second| first.0.cmp(second.0));
    let mut out = String::new();
    for (path, bytes) in sorted {
        out.push_str(&sha256_hex(bytes.as_bytes()));
        out.push_str("  ");
        out.push_str(path);
        out.push('\n');
    }
    out
}

pub fn write_text_file(root: &std::path::Path, relative: &str, bytes: &str) -> std::io::Result<()> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)
}

pub fn aggregate_metrics(reports: &[FinalMetrics]) -> FinalMetrics {
    let mut total = FinalMetrics::default();
    for metrics in reports {
        total.tp += metrics.tp;
        total.fs += metrics.fs;
        total.fm += metrics.fm;
        total.records_fs += metrics.records_fs;
        total.records_fm += metrics.records_fm;
        total.selected += metrics.selected;
        total.exact += metrics.exact;
        total.unambiguous_total += metrics.unambiguous_total;
        total.l_success += metrics.l_success;
        total.m_success += metrics.m_success;
        total.o_correct += metrics.o_correct;
        total.r_correct += metrics.r_correct;
        total.n_fp += metrics.n_fp;
        total.a_honest += metrics.a_honest;
        total.silent_empty += metrics.silent_empty;
        total.post_exhaustion_confident += metrics.post_exhaustion_confident;
        total.orphan_refs += metrics.orphan_refs;
        total.late_refs += metrics.late_refs;
        total.pre_decision_ok += metrics.pre_decision_ok;
        total.pre_decision_total += metrics.pre_decision_total;
    }
    total
}
