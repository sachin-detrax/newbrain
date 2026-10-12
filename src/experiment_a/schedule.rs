#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Partition {
    Training,
    Development,
    Final,
}

impl Partition {
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Training => "T",
            Self::Development => "D",
            Self::Final => "F",
        }
    }

    pub const fn size(self) -> usize {
        match self {
            Self::Training => 64,
            Self::Development => 32,
            Self::Final => 64,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Stratum {
    Length,
    Composition,
    Optional,
    Reorder,
    Negative,
    Ambiguity,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NuisanceLevel {
    None,
    Low,
    High,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Layout {
    OneEcho,
    OneEchoDistractors,
    Compose2,
    ReorderedCompose2,
    Compose3,
    ReorderedCompose3,
    Optional,
    Negative,
    Ambiguous,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RowSpec {
    pub ordinal: u16,
    pub partition: Partition,
    pub stratum: Option<Stratum>,
    pub layout: Layout,
    pub nuisance: NuisanceLevel,
    pub lengths: [u8; 3],
    pub nlengths: u8,
    pub present: bool,
    pub reference: Option<u16>,
}

impl RowSpec {
    pub fn label(&self) -> String {
        format!("{}{:03}", self.partition.prefix(), self.ordinal)
    }

    pub fn payload_lengths(&self) -> &[u8] {
        &self.lengths[..self.nlengths as usize]
    }
}

const fn cycle_primary(i: usize) -> u8 {
    match i % 4 {
        0 => 1,
        1 => 2,
        2 => 3,
        _ => 5,
    }
}

const fn cycle_heldout(i: usize) -> u8 {
    match i % 4 {
        0 => 4,
        1 => 6,
        2 => 7,
        _ => 8,
    }
}

const TRAINING_COMPOSE2: [(u8, u8); 6] = [(1, 2), (2, 3), (3, 5), (5, 1), (1, 3), (2, 5)];
const TRAINING_REORDERED: [(u8, u8); 6] = [(1, 2), (2, 3), (3, 5), (2, 1), (3, 2), (5, 3)];
const DEV_COMPOSE2: [(u8, u8); 4] = [(1, 2), (2, 3), (3, 5), (5, 1)];
const DEV_REORDERED: [(u8, u8); 4] = [(1, 2), (2, 3), (2, 1), (3, 2)];
const FINAL_COMPOSE3: [(u8, u8, u8); 4] = [(4, 6, 7), (6, 7, 8), (7, 8, 4), (8, 4, 6)];
const FINAL_REORDERED2: [(u8, u8); 4] = [(4, 6), (6, 7), (7, 8), (8, 4)];

const fn pair_lengths(pair: (u8, u8)) -> [u8; 3] {
    [pair.0, pair.1, 0]
}

const fn training_row_at(index: usize) -> RowSpec {
    if index < 16 {
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Training,
            stratum: None,
            layout: Layout::OneEcho,
            nuisance: NuisanceLevel::None,
            lengths: [cycle_primary(index), 0, 0],
            nlengths: 1,
            present: true,
            reference: None,
        }
    } else if index < 32 {
        let local = index - 16;
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Training,
            stratum: None,
            layout: Layout::OneEchoDistractors,
            nuisance: NuisanceLevel::High,
            lengths: [cycle_primary(local), 0, 0],
            nlengths: 1,
            present: true,
            reference: None,
        }
    } else if index < 38 {
        let local = index - 32;
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Training,
            stratum: None,
            layout: Layout::Compose2,
            nuisance: NuisanceLevel::None,
            lengths: pair_lengths(TRAINING_COMPOSE2[local]),
            nlengths: 2,
            present: true,
            reference: None,
        }
    } else if index < 44 {
        let local = index - 38;
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Training,
            stratum: None,
            layout: Layout::ReorderedCompose2,
            nuisance: NuisanceLevel::None,
            lengths: pair_lengths(TRAINING_REORDERED[local]),
            nlengths: 2,
            present: true,
            reference: None,
        }
    } else if index < 52 {
        let local = index - 44;
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Training,
            stratum: None,
            layout: Layout::Optional,
            nuisance: NuisanceLevel::None,
            lengths: [cycle_primary(local), 0, 0],
            nlengths: 1,
            present: local.is_multiple_of(2),
            reference: None,
        }
    } else if index < 60 {
        let local = index - 52;
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Training,
            stratum: None,
            layout: Layout::Negative,
            nuisance: NuisanceLevel::None,
            lengths: [cycle_primary(local), 0, 0],
            nlengths: 1,
            present: true,
            reference: Some((local + 1) as u16),
        }
    } else {
        let local = index - 60;
        let pair = if local.is_multiple_of(2) {
            (1, 2)
        } else {
            (2, 1)
        };
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Training,
            stratum: None,
            layout: Layout::Ambiguous,
            nuisance: NuisanceLevel::None,
            lengths: pair_lengths(pair),
            nlengths: 2,
            present: true,
            reference: None,
        }
    }
}

const fn development_row_at(index: usize) -> RowSpec {
    if index < 8 {
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Development,
            stratum: None,
            layout: Layout::OneEchoDistractors,
            nuisance: NuisanceLevel::High,
            lengths: [cycle_primary(index), 0, 0],
            nlengths: 1,
            present: true,
            reference: None,
        }
    } else if index < 12 {
        let local = index - 8;
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Development,
            stratum: None,
            layout: Layout::Compose2,
            nuisance: NuisanceLevel::None,
            lengths: pair_lengths(DEV_COMPOSE2[local]),
            nlengths: 2,
            present: true,
            reference: None,
        }
    } else if index < 16 {
        let local = index - 12;
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Development,
            stratum: None,
            layout: Layout::ReorderedCompose2,
            nuisance: NuisanceLevel::None,
            lengths: pair_lengths(DEV_REORDERED[local]),
            nlengths: 2,
            present: true,
            reference: None,
        }
    } else if index < 20 {
        let local = index - 16;
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Development,
            stratum: None,
            layout: Layout::Optional,
            nuisance: NuisanceLevel::None,
            lengths: [cycle_primary(local), 0, 0],
            nlengths: 1,
            present: local.is_multiple_of(2),
            reference: None,
        }
    } else if index < 24 {
        let local = index - 20;
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Development,
            stratum: None,
            layout: Layout::Negative,
            nuisance: NuisanceLevel::None,
            lengths: [cycle_primary(local), 0, 0],
            nlengths: 1,
            present: true,
            reference: Some((local + 1) as u16),
        }
    } else {
        let local = index - 24;
        let pair = if local.is_multiple_of(2) {
            (1, 2)
        } else {
            (2, 1)
        };
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Development,
            stratum: None,
            layout: Layout::Ambiguous,
            nuisance: NuisanceLevel::None,
            lengths: pair_lengths(pair),
            nlengths: 2,
            present: true,
            reference: None,
        }
    }
}

const fn final_row_at(index: usize) -> RowSpec {
    if index < 8 {
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Final,
            stratum: Some(Stratum::Length),
            layout: Layout::OneEchoDistractors,
            nuisance: NuisanceLevel::Low,
            lengths: [cycle_heldout(index), 0, 0],
            nlengths: 1,
            present: true,
            reference: None,
        }
    } else if index < 16 {
        let local = index - 8;
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Final,
            stratum: Some(Stratum::Length),
            layout: Layout::OneEchoDistractors,
            nuisance: NuisanceLevel::High,
            lengths: [cycle_heldout(local), 0, 0],
            nlengths: 1,
            present: true,
            reference: None,
        }
    } else if index < 28 {
        let local = (index - 16) % 4;
        let triple = FINAL_COMPOSE3[local];
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Final,
            stratum: Some(Stratum::Composition),
            layout: Layout::Compose3,
            nuisance: NuisanceLevel::None,
            lengths: [triple.0, triple.1, triple.2],
            nlengths: 3,
            present: true,
            reference: None,
        }
    } else if index < 36 {
        let local = index - 28;
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Final,
            stratum: Some(Stratum::Optional),
            layout: Layout::Optional,
            nuisance: NuisanceLevel::None,
            lengths: [cycle_heldout(local), 0, 0],
            nlengths: 1,
            present: local.is_multiple_of(2),
            reference: None,
        }
    } else if index < 40 {
        let local = index - 36;
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Final,
            stratum: Some(Stratum::Reorder),
            layout: Layout::ReorderedCompose2,
            nuisance: NuisanceLevel::None,
            lengths: pair_lengths(FINAL_REORDERED2[local]),
            nlengths: 2,
            present: true,
            reference: None,
        }
    } else if index < 44 {
        let local = index - 40;
        let triple = FINAL_COMPOSE3[local];
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Final,
            stratum: Some(Stratum::Reorder),
            layout: Layout::ReorderedCompose3,
            nuisance: NuisanceLevel::None,
            lengths: [triple.0, triple.1, triple.2],
            nlengths: 3,
            present: true,
            reference: None,
        }
    } else if index < 52 {
        let local = index - 44;
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Final,
            stratum: Some(Stratum::Negative),
            layout: Layout::Negative,
            nuisance: NuisanceLevel::Low,
            lengths: [cycle_heldout(local), 0, 0],
            nlengths: 1,
            present: true,
            reference: Some((local + 1) as u16),
        }
    } else {
        let local = index - 52;
        let pair = if local.is_multiple_of(2) {
            (1, 2)
        } else {
            (2, 1)
        };
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Final,
            stratum: Some(Stratum::Ambiguity),
            layout: Layout::Ambiguous,
            nuisance: NuisanceLevel::None,
            lengths: pair_lengths(pair),
            nlengths: 2,
            present: true,
            reference: None,
        }
    }
}

const fn build_training() -> [RowSpec; 64] {
    let mut rows = [training_row_at(0); 64];
    let mut i = 1;
    while i < 64 {
        rows[i] = training_row_at(i);
        i += 1;
    }
    rows
}

const fn build_development() -> [RowSpec; 32] {
    let mut rows = [development_row_at(0); 32];
    let mut i = 1;
    while i < 32 {
        rows[i] = development_row_at(i);
        i += 1;
    }
    rows
}

const fn build_final() -> [RowSpec; 64] {
    let mut rows = [final_row_at(0); 64];
    let mut i = 1;
    while i < 64 {
        rows[i] = final_row_at(i);
        i += 1;
    }
    rows
}

pub const TRAINING_ROWS: [RowSpec; 64] = build_training();
pub const DEVELOPMENT_ROWS: [RowSpec; 32] = build_development();
pub const FINAL_ROWS: [RowSpec; 64] = build_final();

pub fn all_rows() -> Vec<RowSpec> {
    let mut rows = Vec::with_capacity(160);
    rows.extend_from_slice(&TRAINING_ROWS);
    rows.extend_from_slice(&DEVELOPMENT_ROWS);
    rows.extend_from_slice(&FINAL_ROWS);
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn partition_sizes_match_protocol() {
        assert_eq!(TRAINING_ROWS.len(), 64);
        assert_eq!(DEVELOPMENT_ROWS.len(), 32);
        assert_eq!(FINAL_ROWS.len(), 64);
        assert_eq!(all_rows().len(), 160);
    }

    #[test]
    fn labels_are_unique_and_ordered() {
        let rows = all_rows();
        let mut seen = BTreeSet::new();
        for row in &rows {
            assert!(seen.insert(row.label()), "duplicate label {}", row.label());
        }
        assert_eq!(rows.first().unwrap().label(), "T001");
        assert_eq!(rows[63].label(), "T064");
        assert_eq!(rows[64].label(), "D001");
        assert_eq!(rows[95].label(), "D032");
        assert_eq!(rows[96].label(), "F001");
        assert_eq!(rows.last().unwrap().label(), "F064");
        for (i, row) in rows.iter().enumerate() {
            let expected_ordinal = match row.partition {
                Partition::Training => i + 1,
                Partition::Development => i - 64 + 1,
                Partition::Final => i - 96 + 1,
            };
            assert_eq!(row.ordinal as usize, expected_ordinal);
        }
    }

    #[test]
    fn training_layout_ranges_match_protocol() {
        for (i, row) in TRAINING_ROWS.iter().enumerate() {
            let expected = if i < 16 {
                Layout::OneEcho
            } else if i < 32 {
                Layout::OneEchoDistractors
            } else if i < 38 {
                Layout::Compose2
            } else if i < 44 {
                Layout::ReorderedCompose2
            } else if i < 52 {
                Layout::Optional
            } else if i < 60 {
                Layout::Negative
            } else {
                Layout::Ambiguous
            };
            assert_eq!(row.layout, expected, "row {}", row.label());
        }
        let plain: Vec<u8> = TRAINING_ROWS[..16].iter().map(|r| r.lengths[0]).collect();
        assert_eq!(plain, vec![1, 2, 3, 5, 1, 2, 3, 5, 1, 2, 3, 5, 1, 2, 3, 5]);
        let distracted: Vec<u8> = TRAINING_ROWS[16..32].iter().map(|r| r.lengths[0]).collect();
        assert_eq!(distracted, plain);
    }

    #[test]
    fn training_reordered_pairs_match_protocol() {
        let pairs: Vec<[u8; 3]> = TRAINING_ROWS[38..44].iter().map(|r| r.lengths).collect();
        assert_eq!(
            pairs,
            vec![
                [1, 2, 0],
                [2, 3, 0],
                [3, 5, 0],
                [2, 1, 0],
                [3, 2, 0],
                [5, 3, 0]
            ]
        );
    }

    #[test]
    fn optional_rows_alternate_presence() {
        for (k, row) in TRAINING_ROWS[44..52].iter().enumerate() {
            assert_eq!(row.present, k % 2 == 0, "row {}", row.label());
        }
        for (k, row) in DEVELOPMENT_ROWS[16..20].iter().enumerate() {
            assert_eq!(row.present, k % 2 == 0, "row {}", row.label());
        }
        for (k, row) in FINAL_ROWS[28..36].iter().enumerate() {
            assert_eq!(row.present, k % 2 == 0, "row {}", row.label());
        }
    }

    #[test]
    fn negative_rows_reference_valid_same_partition_rows() {
        for row in TRAINING_ROWS[52..60].iter() {
            let reference = row.reference.unwrap();
            assert!((1..=8).contains(&reference));
            let target = &TRAINING_ROWS[(reference - 1) as usize];
            assert_eq!(target.payload_lengths(), row.payload_lengths());
            assert_ne!(target.layout, Layout::Negative);
        }
        for row in DEVELOPMENT_ROWS[20..24].iter() {
            let reference = row.reference.unwrap();
            assert!((1..=4).contains(&reference));
            let target = &DEVELOPMENT_ROWS[(reference - 1) as usize];
            assert_eq!(target.payload_lengths(), row.payload_lengths());
        }
        for row in FINAL_ROWS[44..52].iter() {
            let reference = row.reference.unwrap();
            assert!((1..=8).contains(&reference));
            let target = &FINAL_ROWS[(reference - 1) as usize];
            assert_eq!(target.payload_lengths(), row.payload_lengths());
        }
    }

    #[test]
    fn final_stratum_counts_match_protocol() {
        let mut counts = [0usize; 6];
        for row in FINAL_ROWS.iter() {
            match row.stratum.unwrap() {
                Stratum::Length => counts[0] += 1,
                Stratum::Composition => counts[1] += 1,
                Stratum::Optional => counts[2] += 1,
                Stratum::Reorder => counts[3] += 1,
                Stratum::Negative => counts[4] += 1,
                Stratum::Ambiguity => counts[5] += 1,
            }
        }
        assert_eq!(counts, [16, 12, 8, 8, 8, 12]);
    }

    #[test]
    fn final_length_cycle_is_held_out() {
        let low: Vec<u8> = FINAL_ROWS[..8].iter().map(|r| r.lengths[0]).collect();
        assert_eq!(low, vec![4, 6, 7, 8, 4, 6, 7, 8]);
        let high: Vec<u8> = FINAL_ROWS[8..16].iter().map(|r| r.lengths[0]).collect();
        assert_eq!(high, low);
        for row in FINAL_ROWS[..16].iter() {
            assert!(![1, 2, 3, 5].contains(&row.lengths[0]));
        }
    }

    #[test]
    fn final_composition_triples_match_protocol() {
        let triples: Vec<[u8; 3]> = FINAL_ROWS[16..28].iter().map(|r| r.lengths).collect();
        assert_eq!(
            triples,
            vec![
                [4, 6, 7],
                [6, 7, 8],
                [7, 8, 4],
                [8, 4, 6],
                [4, 6, 7],
                [6, 7, 8],
                [7, 8, 4],
                [8, 4, 6],
                [4, 6, 7],
                [6, 7, 8],
                [7, 8, 4],
                [8, 4, 6],
            ]
        );
    }

    #[test]
    fn ambiguous_rows_alternate_pair_order() {
        for rows in [
            &TRAINING_ROWS[60..64],
            &DEVELOPMENT_ROWS[24..32],
            &FINAL_ROWS[52..64],
        ] {
            for (k, row) in rows.iter().enumerate() {
                let expected = if k % 2 == 0 { [1, 2, 0] } else { [2, 1, 0] };
                assert_eq!(row.lengths, expected, "row {}", row.label());
                assert_eq!(row.layout, Layout::Ambiguous);
            }
        }
    }

    #[test]
    fn development_reordered_pairs_match_protocol() {
        let pairs: Vec<[u8; 3]> = DEVELOPMENT_ROWS[12..16].iter().map(|r| r.lengths).collect();
        assert_eq!(pairs, vec![[1, 2, 0], [2, 3, 0], [2, 1, 0], [3, 2, 0]]);
    }
}
