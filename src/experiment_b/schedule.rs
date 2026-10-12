use super::protocol::{
    DEVELOPMENT_EPISODES, FINAL_COMPOSITION_EPISODES, FINAL_DEAD_END_EPISODES, FINAL_EPISODES,
    FINAL_H16_EPISODES, FINAL_H4_EPISODES, FINAL_H8_EPISODES, FINAL_REPLAN_EPISODES,
    FINAL_STOCHASTIC_EPISODES, TOTAL_EPISODES, TRAINING_EPISODES,
};

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
            Self::Training => TRAINING_EPISODES,
            Self::Development => DEVELOPMENT_EPISODES,
            Self::Final => FINAL_EPISODES,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Training => "training",
            Self::Development => "development",
            Self::Final => "final",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HorizonClass {
    H4,
    H8,
    H16,
}

impl HorizonClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::H4 => "H4",
            Self::H8 => "H8",
            Self::H16 => "H16",
        }
    }

    pub const fn value(self) -> u16 {
        match self {
            Self::H4 => 4,
            Self::H8 => 8,
            Self::H16 => 16,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Stratum {
    H4,
    H8,
    H16,
    Composition,
    Stochastic,
    DeadEnd,
    Replan,
}

impl Stratum {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::H4 => "H4",
            Self::H8 => "H8",
            Self::H16 => "H16",
            Self::Composition => "C",
            Self::Stochastic => "S",
            Self::DeadEnd => "D",
            Self::Replan => "R",
        }
    }

    pub const fn denominator(self) -> usize {
        match self {
            Self::H4 => FINAL_H4_EPISODES,
            Self::H8 => FINAL_H8_EPISODES,
            Self::H16 => FINAL_H16_EPISODES,
            Self::Composition => FINAL_COMPOSITION_EPISODES,
            Self::Stochastic => FINAL_STOCHASTIC_EPISODES,
            Self::DeadEnd => FINAL_DEAD_END_EPISODES,
            Self::Replan => FINAL_REPLAN_EPISODES,
        }
    }

    pub const fn all() -> [Self; 7] {
        [
            Self::H4,
            Self::H8,
            Self::H16,
            Self::Composition,
            Self::Stochastic,
            Self::DeadEnd,
            Self::Replan,
        ]
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Layout {
    SimpleChain,
    BranchingPath,
    ComplexGraph,
    AdmissionChallenge,
    RejectionChallenge,
    BudgetChallenge,
    FreshGoal,
    Composition,
    Stochastic,
    DeadEnd,
    Replan,
}

impl Layout {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SimpleChain => "simple_chain",
            Self::BranchingPath => "branching_path",
            Self::ComplexGraph => "complex_graph",
            Self::AdmissionChallenge => "admission_challenge",
            Self::RejectionChallenge => "rejection_challenge",
            Self::BudgetChallenge => "budget_challenge",
            Self::FreshGoal => "fresh_goal",
            Self::Composition => "composition_transfer",
            Self::Stochastic => "stochastic_transfer",
            Self::DeadEnd => "dead_end_control",
            Self::Replan => "forced_replanning",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RowSpec {
    pub ordinal: u16,
    pub partition: Partition,
    pub horizon_class: HorizonClass,
    pub stratum: Option<Stratum>,
    pub layout: Layout,
}

impl RowSpec {
    pub fn label(&self) -> String {
        format!("{}{:03}", self.partition.prefix(), self.ordinal)
    }

    pub const fn horizon(&self) -> u16 {
        self.horizon_class.value()
    }
}

const fn training_row_at(index: usize) -> RowSpec {
    if index < 32 {
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Training,
            horizon_class: HorizonClass::H4,
            stratum: None,
            layout: Layout::SimpleChain,
        }
    } else if index < 64 {
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Training,
            horizon_class: HorizonClass::H8,
            stratum: None,
            layout: Layout::BranchingPath,
        }
    } else {
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Training,
            horizon_class: HorizonClass::H16,
            stratum: None,
            layout: Layout::ComplexGraph,
        }
    }
}

const fn development_row_at(index: usize) -> RowSpec {
    if index < 16 {
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Development,
            horizon_class: HorizonClass::H4,
            stratum: None,
            layout: Layout::AdmissionChallenge,
        }
    } else if index < 32 {
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Development,
            horizon_class: HorizonClass::H8,
            stratum: None,
            layout: Layout::RejectionChallenge,
        }
    } else {
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Development,
            horizon_class: HorizonClass::H16,
            stratum: None,
            layout: Layout::BudgetChallenge,
        }
    }
}

const fn mixed_horizon(index: usize) -> HorizonClass {
    if index.is_multiple_of(2) {
        HorizonClass::H8
    } else {
        HorizonClass::H16
    }
}

const fn final_row_at(index: usize) -> RowSpec {
    if index < 16 {
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Final,
            horizon_class: HorizonClass::H4,
            stratum: Some(Stratum::H4),
            layout: Layout::FreshGoal,
        }
    } else if index < 32 {
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Final,
            horizon_class: HorizonClass::H8,
            stratum: Some(Stratum::H8),
            layout: Layout::FreshGoal,
        }
    } else if index < 48 {
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Final,
            horizon_class: HorizonClass::H16,
            stratum: Some(Stratum::H16),
            layout: Layout::FreshGoal,
        }
    } else if index < 60 {
        let local = index - 48;
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Final,
            horizon_class: mixed_horizon(local),
            stratum: Some(Stratum::Composition),
            layout: Layout::Composition,
        }
    } else if index < 72 {
        let local = index - 60;
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Final,
            horizon_class: mixed_horizon(local),
            stratum: Some(Stratum::Stochastic),
            layout: Layout::Stochastic,
        }
    } else if index < 84 {
        let local = index - 72;
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Final,
            horizon_class: mixed_horizon(local),
            stratum: Some(Stratum::DeadEnd),
            layout: Layout::DeadEnd,
        }
    } else {
        let local = index - 84;
        RowSpec {
            ordinal: (index + 1) as u16,
            partition: Partition::Final,
            horizon_class: mixed_horizon(local),
            stratum: Some(Stratum::Replan),
            layout: Layout::Replan,
        }
    }
}

const fn build_training() -> [RowSpec; TRAINING_EPISODES] {
    let mut rows = [training_row_at(0); TRAINING_EPISODES];
    let mut i = 1;
    while i < TRAINING_EPISODES {
        rows[i] = training_row_at(i);
        i += 1;
    }
    rows
}

const fn build_development() -> [RowSpec; DEVELOPMENT_EPISODES] {
    let mut rows = [development_row_at(0); DEVELOPMENT_EPISODES];
    let mut i = 1;
    while i < DEVELOPMENT_EPISODES {
        rows[i] = development_row_at(i);
        i += 1;
    }
    rows
}

const fn build_final() -> [RowSpec; FINAL_EPISODES] {
    let mut rows = [final_row_at(0); FINAL_EPISODES];
    let mut i = 1;
    while i < FINAL_EPISODES {
        rows[i] = final_row_at(i);
        i += 1;
    }
    rows
}

pub const TRAINING_ROWS: [RowSpec; TRAINING_EPISODES] = build_training();
pub const DEVELOPMENT_ROWS: [RowSpec; DEVELOPMENT_EPISODES] = build_development();
pub const FINAL_ROWS: [RowSpec; FINAL_EPISODES] = build_final();

pub fn all_rows() -> Vec<RowSpec> {
    let mut rows = Vec::with_capacity(TOTAL_EPISODES);
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
        assert_eq!(TRAINING_ROWS.len(), 96);
        assert_eq!(DEVELOPMENT_ROWS.len(), 48);
        assert_eq!(FINAL_ROWS.len(), 96);
        assert_eq!(all_rows().len(), TOTAL_EPISODES);
        assert_eq!(TOTAL_EPISODES, 240);
    }

    #[test]
    fn labels_are_unique_and_ordered() {
        let rows = all_rows();
        let mut seen = BTreeSet::new();
        for row in &rows {
            assert!(seen.insert(row.label()), "duplicate label {}", row.label());
        }
        assert_eq!(rows.first().unwrap().label(), "T001");
        assert_eq!(rows[95].label(), "T096");
        assert_eq!(rows[96].label(), "D001");
        assert_eq!(rows[143].label(), "D048");
        assert_eq!(rows[144].label(), "F001");
        assert_eq!(rows.last().unwrap().label(), "F096");
        for (i, row) in rows.iter().enumerate() {
            let expected_ordinal = match row.partition {
                Partition::Training => i + 1,
                Partition::Development => i - 96 + 1,
                Partition::Final => i - 144 + 1,
            };
            assert_eq!(row.ordinal as usize, expected_ordinal);
        }
    }

    #[test]
    fn training_horizon_ranges_match_protocol() {
        for (i, row) in TRAINING_ROWS.iter().enumerate() {
            let expected_horizon = if i < 32 {
                HorizonClass::H4
            } else if i < 64 {
                HorizonClass::H8
            } else {
                HorizonClass::H16
            };
            let expected_layout = match expected_horizon {
                HorizonClass::H4 => Layout::SimpleChain,
                HorizonClass::H8 => Layout::BranchingPath,
                HorizonClass::H16 => Layout::ComplexGraph,
            };
            assert_eq!(row.horizon_class, expected_horizon, "row {}", row.label());
            assert_eq!(row.layout, expected_layout, "row {}", row.label());
            assert!(row.stratum.is_none(), "row {}", row.label());
        }
    }

    #[test]
    fn development_horizon_ranges_match_protocol() {
        for (i, row) in DEVELOPMENT_ROWS.iter().enumerate() {
            let expected_horizon = if i < 16 {
                HorizonClass::H4
            } else if i < 32 {
                HorizonClass::H8
            } else {
                HorizonClass::H16
            };
            let expected_layout = match expected_horizon {
                HorizonClass::H4 => Layout::AdmissionChallenge,
                HorizonClass::H8 => Layout::RejectionChallenge,
                HorizonClass::H16 => Layout::BudgetChallenge,
            };
            assert_eq!(row.horizon_class, expected_horizon, "row {}", row.label());
            assert_eq!(row.layout, expected_layout, "row {}", row.label());
            assert!(row.stratum.is_none(), "row {}", row.label());
        }
    }

    #[test]
    fn final_stratum_counts_match_protocol() {
        let mut counts = [0_usize; 7];
        for row in FINAL_ROWS.iter() {
            match row.stratum.expect("final rows carry a stratum") {
                Stratum::H4 => counts[0] += 1,
                Stratum::H8 => counts[1] += 1,
                Stratum::H16 => counts[2] += 1,
                Stratum::Composition => counts[3] += 1,
                Stratum::Stochastic => counts[4] += 1,
                Stratum::DeadEnd => counts[5] += 1,
                Stratum::Replan => counts[6] += 1,
            }
        }
        assert_eq!(counts, [16, 16, 16, 12, 12, 12, 12]);
        for stratum in Stratum::all() {
            let expected = match stratum {
                Stratum::H4 | Stratum::H8 | Stratum::H16 => 16,
                Stratum::Composition
                | Stratum::Stochastic
                | Stratum::DeadEnd
                | Stratum::Replan => 12,
            };
            assert_eq!(stratum.denominator(), expected, "{}", stratum.as_str());
        }
    }

    #[test]
    fn final_fresh_horizons_match_protocol() {
        for row in FINAL_ROWS[..16].iter() {
            assert_eq!(row.horizon_class, HorizonClass::H4);
            assert_eq!(row.stratum, Some(Stratum::H4));
            assert_eq!(row.layout, Layout::FreshGoal);
        }
        for row in FINAL_ROWS[16..32].iter() {
            assert_eq!(row.horizon_class, HorizonClass::H8);
            assert_eq!(row.stratum, Some(Stratum::H8));
            assert_eq!(row.layout, Layout::FreshGoal);
        }
        for row in FINAL_ROWS[32..48].iter() {
            assert_eq!(row.horizon_class, HorizonClass::H16);
            assert_eq!(row.stratum, Some(Stratum::H16));
            assert_eq!(row.layout, Layout::FreshGoal);
        }
    }

    #[test]
    fn final_transfer_strata_alternate_horizons_within_eight_sixteen() {
        for (range, stratum, layout) in [
            (48..60_usize, Stratum::Composition, Layout::Composition),
            (60..72_usize, Stratum::Stochastic, Layout::Stochastic),
            (72..84_usize, Stratum::DeadEnd, Layout::DeadEnd),
            (84..96_usize, Stratum::Replan, Layout::Replan),
        ] {
            for (local, row) in FINAL_ROWS[range].iter().enumerate() {
                let expected_horizon = if local % 2 == 0 {
                    HorizonClass::H8
                } else {
                    HorizonClass::H16
                };
                assert_eq!(
                    row.horizon_class,
                    expected_horizon,
                    "row {} stratum {}",
                    row.label(),
                    stratum.as_str()
                );
                assert!([4_u16, 8, 16].contains(&row.horizon()));
                assert_eq!(row.stratum, Some(stratum), "row {}", row.label());
                assert_eq!(row.layout, layout, "row {}", row.label());
            }
        }
    }

    #[test]
    fn horizon_class_integers_are_four_eight_sixteen() {
        assert_eq!(HorizonClass::H4.value(), 4);
        assert_eq!(HorizonClass::H8.value(), 8);
        assert_eq!(HorizonClass::H16.value(), 16);
    }

    #[test]
    fn partition_prefixes_and_sizes_match_protocol() {
        assert_eq!(Partition::Training.prefix(), "T");
        assert_eq!(Partition::Development.prefix(), "D");
        assert_eq!(Partition::Final.prefix(), "F");
        assert_eq!(Partition::Training.size(), 96);
        assert_eq!(Partition::Development.size(), 48);
        assert_eq!(Partition::Final.size(), 96);
    }
}
