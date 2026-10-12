use super::protocol::{SplitMix64, ORDINAL_MULTIPLIER, PERMUTATION_STATE_XOR};
use super::schedule::{all_rows, Layout, Partition, RowSpec};
use std::collections::{BTreeMap, BTreeSet};

pub const GOLDEN_REDRAW_MIX: u64 = 0x9E37_79B9_7F4A_7C15;
pub const MAX_GENERATOR_REDRAWS: u32 = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymbolMode {
    Primary,
    Permutation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedRecord {
    pub wire_tokens: Vec<u64>,
    pub run_scope: u64,
    pub stream_scope: u64,
    pub redraws: u32,
    pub latent_echo_spans: Vec<(usize, usize)>,
    pub ambiguous_core_offset: Option<usize>,
    pub ambiguous_first_len: u8,
    pub global_ordinal: u64,
    pub partition: Partition,
    pub row_ordinal: u16,
    pub noise_vector: Vec<u64>,
    pub emitted_order: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunRecords {
    pub records: Vec<GeneratedRecord>,
    pub run_scope: u64,
}

struct Allocator {
    state: SplitMix64,
    used: BTreeSet<u64>,
}

impl Allocator {
    fn draw(&mut self) -> u64 {
        loop {
            let value = self.state.next_u64();
            if value != 0 && !self.used.contains(&value) {
                self.used.insert(value);
                return value;
            }
        }
    }

    fn draw_bounded(&mut self, modulus: u64) -> u64 {
        self.state.next_u64() % modulus
    }
}

struct Permuter {
    state: SplitMix64,
    table: BTreeMap<u64, u64>,
    used_wire: BTreeSet<u64>,
}

impl Permuter {
    fn new(world_seed: u64) -> Self {
        Self {
            state: SplitMix64::new(world_seed ^ PERMUTATION_STATE_XOR),
            table: BTreeMap::new(),
            used_wire: BTreeSet::new(),
        }
    }

    fn map(&mut self, logical: u64) -> u64 {
        if let Some(wire) = self.table.get(&logical) {
            return *wire;
        }
        loop {
            let wire = self.state.next_u64();
            if wire != 0 && !self.used_wire.contains(&wire) {
                self.used_wire.insert(wire);
                self.table.insert(logical, wire);
                return wire;
            }
        }
    }
}

fn fisher_yates(order: &mut [usize], rng: &mut SplitMix64) {
    let n = order.len();
    for i in 0..n {
        let j = i + (rng.next_u64() % ((n - i) as u64)) as usize;
        order.swap(i, j);
    }
}

fn partition_tag(partition: Partition, training: u64, development: u64, final_tag: u64) -> u64 {
    match partition {
        Partition::Training => training,
        Partition::Development => development,
        Partition::Final => final_tag,
    }
}

pub fn record_seed(
    world_seed: u64,
    partition_tag_value: u64,
    global_ordinal: u64,
    redraw: u32,
) -> u64 {
    world_seed
        ^ partition_tag_value
        ^ global_ordinal.wrapping_mul(ORDINAL_MULTIPLIER)
        ^ (redraw as u64).wrapping_mul(GOLDEN_REDRAW_MIX)
}

struct Segment {
    endpoint: u64,
    payload: Vec<u64>,
    quasi: bool,
    second_endpoint: u64,
}

fn draw_echo(allocator: &mut Allocator, payload_len: u8) -> Segment {
    let endpoint = allocator.draw();
    let mut payload = Vec::with_capacity(payload_len as usize);
    for _ in 0..payload_len {
        payload.push(allocator.draw());
    }
    Segment {
        endpoint,
        payload,
        quasi: false,
        second_endpoint: endpoint,
    }
}

fn draw_quasi(allocator: &mut Allocator, payload_len: u8) -> Segment {
    let endpoint = allocator.draw();
    let mut payload = Vec::with_capacity(payload_len as usize);
    for _ in 0..payload_len {
        payload.push(allocator.draw());
    }
    let second_endpoint = loop {
        let value = allocator.draw();
        if value != endpoint {
            break value;
        }
    };
    Segment {
        endpoint,
        payload,
        quasi: true,
        second_endpoint,
    }
}

fn draw_noise(allocator: &mut Allocator, count: u64) -> Vec<u64> {
    let mut atoms = Vec::with_capacity(count as usize);
    while atoms.len() < count as usize {
        let value = allocator.draw();
        if !atoms.contains(&value) {
            atoms.push(value);
        }
    }
    atoms
}

fn emit_segment(segment: &Segment) -> Vec<u64> {
    let mut tokens = Vec::with_capacity(segment.payload.len() + 2);
    tokens.push(segment.endpoint);
    tokens.extend_from_slice(&segment.payload);
    tokens.push(if segment.quasi {
        segment.second_endpoint
    } else {
        segment.endpoint
    });
    tokens
}

struct BuiltRecord {
    tokens: Vec<u64>,
    echo_spans: Vec<(usize, usize)>,
    ambiguous_core_offset: Option<usize>,
    ambiguous_first_len: u8,
    noise_vector: Vec<u64>,
    emitted_order: Vec<usize>,
}

fn noise_len(allocator: &mut Allocator) -> u64 {
    1 + allocator.draw_bounded(3)
}

fn take_noise(allocator: &mut Allocator, ledger: &mut Vec<u64>, count: u64) -> Vec<u64> {
    ledger.push(count);
    draw_noise(allocator, count)
}

fn build_tokens(
    row: &RowSpec,
    allocator: &mut Allocator,
    reference_noise: Option<&[u64]>,
) -> BuiltRecord {
    let mut noise_vector: Vec<u64> = Vec::new();
    match row.layout {
        Layout::OneEcho | Layout::OneEchoDistractors => {
            let (prefix_count, suffix_count) = match row.nuisance {
                super::schedule::NuisanceLevel::None => (0, 0),
                super::schedule::NuisanceLevel::Low => (noise_len(allocator), noise_len(allocator)),
                super::schedule::NuisanceLevel::High => (
                    noise_len(allocator) + noise_len(allocator),
                    noise_len(allocator) + noise_len(allocator),
                ),
            };
            let mut tokens = Vec::new();
            if prefix_count > 0 {
                tokens.extend(take_noise(allocator, &mut noise_vector, prefix_count));
            }
            let segment = draw_echo(allocator, row.lengths[0]);
            let span_start = tokens.len();
            tokens.extend(emit_segment(&segment));
            let span = (span_start, tokens.len());
            if suffix_count > 0 {
                tokens.extend(take_noise(allocator, &mut noise_vector, suffix_count));
            }
            BuiltRecord {
                tokens,
                echo_spans: vec![span],
                ambiguous_core_offset: None,
                ambiguous_first_len: 0,
                noise_vector,
                emitted_order: vec![0],
            }
        }
        Layout::Compose2
        | Layout::ReorderedCompose2
        | Layout::Compose3
        | Layout::ReorderedCompose3 => {
            let slot_count = if matches!(row.layout, Layout::Compose3 | Layout::ReorderedCompose3) {
                3
            } else {
                2
            };
            let mut segments = Vec::with_capacity(slot_count);
            for slot in 0..slot_count {
                segments.push(draw_echo(allocator, row.lengths[slot]));
            }
            let mut order: Vec<usize> = (0..slot_count).collect();
            if matches!(
                row.layout,
                Layout::ReorderedCompose2 | Layout::ReorderedCompose3
            ) {
                fisher_yates(&mut order, &mut allocator.state);
            }
            let prefix = noise_len(allocator);
            let suffix = noise_len(allocator);
            let mut gaps = Vec::with_capacity(slot_count.saturating_sub(1));
            for _ in 0..slot_count.saturating_sub(1) {
                gaps.push(noise_len(allocator));
            }
            let mut tokens = take_noise(allocator, &mut noise_vector, prefix);
            let mut echo_spans = vec![(0, 0); slot_count];
            for (position, slot) in order.iter().enumerate() {
                if position > 0 {
                    let gap = gaps[position - 1];
                    tokens.extend(take_noise(allocator, &mut noise_vector, gap));
                }
                let start = tokens.len();
                tokens.extend(emit_segment(&segments[*slot]));
                echo_spans[*slot] = (start, tokens.len());
            }
            tokens.extend(take_noise(allocator, &mut noise_vector, suffix));
            let mut full_noise = vec![prefix];
            full_noise.extend_from_slice(&gaps);
            full_noise.push(suffix);
            BuiltRecord {
                tokens,
                echo_spans,
                ambiguous_core_offset: None,
                ambiguous_first_len: 0,
                noise_vector: full_noise,
                emitted_order: order,
            }
        }
        Layout::Optional => {
            let first = draw_echo(allocator, row.lengths[0]);
            let second = draw_echo(allocator, row.lengths[0]);
            let prefix = noise_len(allocator);
            let gap = noise_len(allocator);
            let suffix = noise_len(allocator);
            let mut tokens = take_noise(allocator, &mut noise_vector, prefix);
            let first_start = tokens.len();
            tokens.extend(emit_segment(&first));
            let first_span = (first_start, tokens.len());
            let mut echo_spans = vec![first_span];
            let mut emitted_order = vec![0];
            if row.present {
                tokens.extend(take_noise(allocator, &mut noise_vector, gap));
                let second_start = tokens.len();
                tokens.extend(emit_segment(&second));
                echo_spans.push((second_start, tokens.len()));
                emitted_order.push(1);
            } else {
                noise_vector.push(gap);
            }
            tokens.extend(take_noise(allocator, &mut noise_vector, suffix));
            BuiltRecord {
                tokens,
                echo_spans,
                ambiguous_core_offset: None,
                ambiguous_first_len: 0,
                noise_vector: vec![prefix, gap, suffix],
                emitted_order,
            }
        }
        Layout::Negative => {
            let reference = reference_noise.unwrap_or(&[]);
            let mut tokens = Vec::new();
            let mut cursor = 0;
            let take = |cursor: &mut usize| -> u64 {
                let value = *reference.get(*cursor).unwrap_or(&0);
                *cursor += 1;
                value
            };
            let prefix = take(&mut cursor);
            if prefix > 0 {
                tokens.extend(draw_noise(allocator, prefix));
            }
            let segment = draw_quasi(allocator, row.lengths[0]);
            tokens.extend(emit_segment(&segment));
            while cursor < reference.len().saturating_sub(1) {
                let gap = take(&mut cursor);
                if gap > 0 {
                    tokens.extend(draw_noise(allocator, gap));
                }
            }
            if let Some(last) = reference.last() {
                if *last > 0 && reference.len() > 1 {
                    tokens.extend(draw_noise(allocator, *last));
                }
            }
            BuiltRecord {
                tokens,
                echo_spans: Vec::new(),
                ambiguous_core_offset: None,
                ambiguous_first_len: 0,
                noise_vector: reference.to_vec(),
                emitted_order: Vec::new(),
            }
        }
        Layout::Ambiguous => {
            let first_len = row.lengths[0];
            let second_len = row.lengths[1];
            let prefix = noise_len(allocator);
            let suffix = noise_len(allocator);
            let mut tokens = take_noise(allocator, &mut noise_vector, prefix);
            let core_offset = tokens.len();
            let shared = allocator.draw();
            let mut first_payload = Vec::with_capacity(first_len as usize);
            for _ in 0..first_len {
                first_payload.push(allocator.draw());
            }
            let mut second_payload = Vec::with_capacity(second_len as usize);
            for _ in 0..second_len {
                second_payload.push(allocator.draw());
            }
            tokens.push(shared);
            tokens.extend_from_slice(&first_payload);
            tokens.push(shared);
            tokens.push(shared);
            tokens.extend_from_slice(&second_payload);
            tokens.push(shared);
            let first_span = (core_offset, core_offset + first_len as usize + 2);
            let second_span = (
                core_offset + first_len as usize + 2,
                core_offset + first_len as usize + second_len as usize + 4,
            );
            tokens.extend(take_noise(allocator, &mut noise_vector, suffix));
            BuiltRecord {
                tokens,
                echo_spans: vec![first_span, second_span],
                ambiguous_core_offset: Some(core_offset),
                ambiguous_first_len: first_len,
                noise_vector: vec![prefix, suffix],
                emitted_order: vec![0, 1],
            }
        }
    }
}

fn accidental_parse_exists(
    tokens: &[u64],
    allowed: &[(usize, usize)],
    core: Option<(usize, usize)>,
) -> bool {
    let n = tokens.len();
    for start in 0..n {
        for end in start + 3..=n.min(start + 10) {
            if tokens[start] == tokens[end - 1] && !allowed.contains(&(start, end)) {
                let inside_core = core
                    .map(|(lo, hi)| start >= lo && end <= hi)
                    .unwrap_or(false);
                if !inside_core {
                    return true;
                }
            }
        }
    }
    false
}

pub fn generate_run(
    world_seed: u64,
    scope_seed: u64,
    training_tag: u64,
    development_tag: u64,
    final_tag: u64,
    mode: SymbolMode,
) -> RunRecords {
    let mut scope_rng = SplitMix64::new(scope_seed);
    let mut scope_used = BTreeSet::new();
    let next_scope = move |rng: &mut SplitMix64, used: &mut BTreeSet<u64>| -> u64 {
        loop {
            let value = rng.next_u64();
            if value != 0 && !used.contains(&value) {
                used.insert(value);
                return value;
            }
        }
    };
    let run_scope = next_scope(&mut scope_rng, &mut scope_used);
    let mut permuter = match mode {
        SymbolMode::Primary => None,
        SymbolMode::Permutation => Some(Permuter::new(world_seed)),
    };
    let mut atom_used: BTreeSet<u64> = BTreeSet::new();
    let mut noise_cache: BTreeMap<(Partition, u16), Vec<u64>> = BTreeMap::new();
    let mut records = Vec::with_capacity(160);
    let mut global_ordinal: u64 = 0;
    for row in all_rows() {
        global_ordinal += 1;
        let tag = partition_tag(row.partition, training_tag, development_tag, final_tag);
        let reference_noise = row
            .reference
            .and_then(|ordinal| noise_cache.get(&(row.partition, ordinal)).cloned());
        let reference_vec = reference_noise.clone().unwrap_or_default();
        let built;
        let mut redraws: u32 = 0;
        loop {
            let seed = record_seed(world_seed, tag, global_ordinal, redraws);
            let mut allocator = Allocator {
                state: SplitMix64::new(seed),
                used: atom_used.clone(),
            };
            let candidate = build_tokens(
                &row,
                &mut allocator,
                row.reference.map(|_| reference_vec.as_slice()),
            );
            let mut allowed = candidate.echo_spans.clone();
            let core_range = candidate
                .ambiguous_core_offset
                .map(|offset| (offset, offset + 7));
            if let Some(offset) = candidate.ambiguous_core_offset {
                allowed.push((offset, offset + 7));
            }
            let ok_length = !candidate.tokens.is_empty() && candidate.tokens.len() <= 48;
            if ok_length && !accidental_parse_exists(&candidate.tokens, &allowed, core_range) {
                atom_used = allocator.used;
                built = candidate;
                break;
            }
            redraws += 1;
            assert!(
                redraws <= MAX_GENERATOR_REDRAWS,
                "generator redraw budget exhausted at {}",
                row.label()
            );
        }
        noise_cache.insert((row.partition, row.ordinal), built.noise_vector.clone());
        let stream_scope = next_scope(&mut scope_rng, &mut scope_used);
        let wire_tokens = match permuter.as_mut() {
            None => built.tokens,
            Some(permuter) => built
                .tokens
                .iter()
                .map(|atom| permuter.map(*atom))
                .collect(),
        };
        records.push(GeneratedRecord {
            wire_tokens,
            run_scope,
            stream_scope,
            redraws,
            latent_echo_spans: built.echo_spans,
            ambiguous_core_offset: built.ambiguous_core_offset,
            ambiguous_first_len: built.ambiguous_first_len,
            global_ordinal,
            partition: row.partition,
            row_ordinal: row.ordinal,
            noise_vector: built.noise_vector,
            emitted_order: built.emitted_order,
        });
    }
    RunRecords { records, run_scope }
}

#[cfg(test)]
mod tests {
    use super::super::protocol::{PRIMARY_SCOPE_SEED, PRIMARY_WORLD_SEED, TRAINING_PARTITION_TAG};
    use super::super::schedule::{DEVELOPMENT_ROWS, FINAL_ROWS, TRAINING_ROWS};
    use super::*;

    const DEV_TAG: u64 = 0x4445_5600_0000_0001;
    const FINAL_TAG: u64 = 0x4649_4e00_0000_0001;

    fn primary_run() -> RunRecords {
        generate_run(
            PRIMARY_WORLD_SEED,
            PRIMARY_SCOPE_SEED,
            TRAINING_PARTITION_TAG,
            DEV_TAG,
            FINAL_TAG,
            SymbolMode::Primary,
        )
    }

    #[test]
    fn run_covers_the_full_schedule_in_order() {
        let run = primary_run();
        assert_eq!(run.records.len(), 160);
        assert_eq!(run.records.first().unwrap().global_ordinal, 1);
        assert_eq!(run.records.last().unwrap().global_ordinal, 160);
        assert_eq!(run.records[63].row_ordinal, 64);
        assert_eq!(run.records[64].partition, Partition::Development);
        assert_eq!(run.records[96].partition, Partition::Final);
        for pair in run.records.windows(2) {
            assert_eq!(pair[1].global_ordinal, pair[0].global_ordinal + 1);
        }
    }

    #[test]
    fn generation_is_deterministic_for_fixed_seeds() {
        let first = primary_run();
        let second = primary_run();
        assert_eq!(first, second);
    }

    #[test]
    fn atoms_are_globally_unique_except_echo_endpoints() {
        let run = primary_run();
        let mut seen: BTreeMap<u64, usize> = BTreeMap::new();
        for record in &run.records {
            for atom in &record.wire_tokens {
                *seen.entry(*atom).or_insert(0) += 1;
            }
        }
        for record in &run.records {
            for (start, end) in &record.latent_echo_spans {
                let span = &record.wire_tokens[*start..*end];
                assert_eq!(span.first(), span.last());
            }
            if let Some(offset) = record.ambiguous_core_offset {
                let core = &record.wire_tokens[offset..offset + 7];
                assert_eq!(core.first(), core.last());
                assert_eq!(core[record.ambiguous_first_len as usize + 1], core[0]);
                assert_eq!(core[record.ambiguous_first_len as usize + 2], core[0]);
            }
        }
        for (atom, count) in &seen {
            if *count == 1 || *count == 2 {
                continue;
            }
            let in_ambiguous_core = run.records.iter().any(|record| {
                record
                    .ambiguous_core_offset
                    .map(|offset| {
                        let core = &record.wire_tokens[offset..offset + 7];
                        core.contains(atom)
                    })
                    .unwrap_or(false)
            });
            assert!(
                *count == 4 && in_ambiguous_core,
                "atom {atom} appears {count} times outside any ambiguous core"
            );
        }
        assert!(!seen.contains_key(&0));
    }

    #[test]
    fn streams_respect_the_maximum_length() {
        let run = primary_run();
        for record in &run.records {
            assert!(!record.wire_tokens.is_empty());
            assert!(
                record.wire_tokens.len() <= 48,
                "record {}",
                record.global_ordinal
            );
        }
    }

    #[test]
    fn negative_rows_mirror_reference_noise_vectors() {
        let run = primary_run();
        let by_key: BTreeMap<(Partition, u16), &GeneratedRecord> = run
            .records
            .iter()
            .map(|record| ((record.partition, record.row_ordinal), record))
            .collect();
        for record in &run.records {
            let row = match record.partition {
                Partition::Training => &TRAINING_ROWS[(record.row_ordinal - 1) as usize],
                Partition::Development => &DEVELOPMENT_ROWS[(record.row_ordinal - 1) as usize],
                Partition::Final => &FINAL_ROWS[(record.row_ordinal - 1) as usize],
            };
            if let Some(reference) = row.reference {
                let target = by_key[&(record.partition, reference)];
                assert_eq!(
                    record.noise_vector, target.noise_vector,
                    "record {}",
                    record.global_ordinal
                );
                assert!(record.latent_echo_spans.is_empty());
            }
        }
    }

    #[test]
    fn permutation_mode_preserves_structure_with_disjoint_atoms() {
        let primary = primary_run();
        let permuted = generate_run(
            PRIMARY_WORLD_SEED,
            PRIMARY_SCOPE_SEED,
            TRAINING_PARTITION_TAG,
            DEV_TAG,
            FINAL_TAG,
            SymbolMode::Permutation,
        );
        assert_eq!(primary.records.len(), permuted.records.len());
        for (plain, mapped) in primary.records.iter().zip(permuted.records.iter()) {
            assert_eq!(plain.wire_tokens.len(), mapped.wire_tokens.len());
            assert_eq!(plain.latent_echo_spans, mapped.latent_echo_spans);
            assert_eq!(plain.stream_scope, mapped.stream_scope);
            assert_eq!(plain.noise_vector, mapped.noise_vector);
            for (start, end) in &plain.latent_echo_spans {
                let plain_span = &plain.wire_tokens[*start..*end];
                let mapped_span = &mapped.wire_tokens[*start..*end];
                assert_eq!(mapped_span.first(), mapped_span.last());
                assert_eq!(plain_span.len(), mapped_span.len());
            }
        }
        let plain_atoms: BTreeSet<u64> = primary
            .records
            .iter()
            .flat_map(|r| r.wire_tokens.iter().copied())
            .collect();
        let mapped_atoms: BTreeSet<u64> = permuted
            .records
            .iter()
            .flat_map(|r| r.wire_tokens.iter().copied())
            .collect();
        assert!(plain_atoms.intersection(&mapped_atoms).next().is_none());
    }

    #[test]
    fn final_reorder_subgroups_leave_canonical_order() {
        let run = primary_run();
        let compose2: Vec<&GeneratedRecord> = run.records[96 + 36..96 + 40].iter().collect();
        assert!(compose2
            .iter()
            .any(|record| record.emitted_order != vec![0, 1]));
        let compose3: Vec<&GeneratedRecord> = run.records[96 + 40..96 + 44].iter().collect();
        assert!(compose3
            .iter()
            .any(|record| record.emitted_order != vec![0, 1, 2]));
    }
}
