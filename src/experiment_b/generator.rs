use super::protocol::{SplitMix64, ORDINAL_MULTIPLIER, PERMUTATION_STATE_XOR};
use super::schedule::{all_rows, Layout, Partition, RowSpec};
use std::collections::{BTreeMap, BTreeSet};

pub const GOLDEN_REDRAW_MIX: u64 = 0x9E37_79B9_7F4A_7C15;
pub const MAX_GENERATOR_REDRAWS: u32 = 64;
pub const SLIP_STOCHASTIC_COUNT: usize = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymbolMode {
    Primary,
    Permutation,
}

impl SymbolMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Permutation => "permutation",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedEpisode {
    pub wire_start: u64,
    pub goal_target: u64,
    pub goal_budget: u64,
    pub horizon: u16,
    pub table: BTreeMap<(u64, u64), (u64, u64)>,
    pub available: BTreeMap<u64, Vec<(u64, u64)>>,
    pub slip_schedule: Vec<bool>,
    pub run_scope: u64,
    pub episode_scope: u64,
    pub redraws: u32,
    pub global_ordinal: u64,
    pub partition: Partition,
    pub row_ordinal: u16,
    pub latent_chain: Vec<u64>,
    pub min_cost: u64,
    pub reachable: bool,
    pub construction: Vec<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunEpisodes {
    pub episodes: Vec<GeneratedEpisode>,
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

    fn bounded(&mut self, modulus: u64) -> u64 {
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

pub fn record_seed(world_seed: u64, partition_tag: u64, global_ordinal: u64, redraw: u32) -> u64 {
    world_seed
        ^ partition_tag
        ^ global_ordinal.wrapping_mul(ORDINAL_MULTIPLIER)
        ^ (redraw as u64).wrapping_mul(GOLDEN_REDRAW_MIX)
}

struct ChainParams {
    length: usize,
    forks: usize,
    gate: bool,
    inverted: bool,
    budget_slack: u64,
    dead_end: bool,
    forced_slip: bool,
    stochastic: bool,
}

fn params_for(row: &RowSpec) -> ChainParams {
    let horizon = row.horizon() as usize;
    match row.layout {
        Layout::SimpleChain => ChainParams {
            length: 3,
            forks: 0,
            gate: false,
            inverted: false,
            budget_slack: 2,
            dead_end: false,
            forced_slip: false,
            stochastic: false,
        },
        Layout::BranchingPath => ChainParams {
            length: 6,
            forks: 1,
            gate: false,
            inverted: false,
            budget_slack: 4,
            dead_end: false,
            forced_slip: false,
            stochastic: false,
        },
        Layout::ComplexGraph => ChainParams {
            length: 12,
            forks: 2,
            gate: true,
            inverted: false,
            budget_slack: 4,
            dead_end: false,
            forced_slip: false,
            stochastic: false,
        },
        Layout::AdmissionChallenge => ChainParams {
            length: 3,
            forks: 1,
            gate: false,
            inverted: false,
            budget_slack: 2,
            dead_end: false,
            forced_slip: false,
            stochastic: false,
        },
        Layout::RejectionChallenge => ChainParams {
            length: 6,
            forks: 1,
            gate: false,
            inverted: true,
            budget_slack: 4,
            dead_end: false,
            forced_slip: false,
            stochastic: false,
        },
        Layout::BudgetChallenge => ChainParams {
            length: 12,
            forks: 1,
            gate: false,
            inverted: true,
            budget_slack: 2,
            dead_end: false,
            forced_slip: false,
            stochastic: false,
        },
        Layout::FreshGoal => ChainParams {
            length: if horizon <= 4 { 3 } else if horizon <= 8 { 6 } else { 12 },
            forks: if horizon <= 4 { 0 } else if horizon <= 8 { 1 } else { 2 },
            gate: false,
            inverted: false,
            budget_slack: if horizon <= 4 { 2 } else { 4 },
            dead_end: false,
            forced_slip: false,
            stochastic: false,
        },
        Layout::Composition => ChainParams {
            length: 8,
            forks: 1,
            gate: true,
            inverted: false,
            budget_slack: 4,
            dead_end: false,
            forced_slip: false,
            stochastic: false,
        },
        Layout::Stochastic => ChainParams {
            length: if horizon <= 8 { 6 } else { 14 },
            forks: 1,
            gate: false,
            inverted: false,
            budget_slack: horizon as u64,
            dead_end: false,
            forced_slip: false,
            stochastic: true,
        },
        Layout::DeadEnd => ChainParams {
            length: horizon / 4,
            forks: 2,
            gate: false,
            inverted: false,
            budget_slack: 0,
            dead_end: true,
            forced_slip: false,
            stochastic: false,
        },
        Layout::Replan => ChainParams {
            length: if horizon <= 8 { 6 } else { 14 },
            forks: 1,
            gate: false,
            inverted: false,
            budget_slack: 2,
            dead_end: false,
            forced_slip: true,
            stochastic: false,
        },
    }
}

struct BuiltEpisode {
    chain: Vec<u64>,
    goal: u64,
    table: BTreeMap<(u64, u64), (u64, u64)>,
    available: BTreeMap<u64, Vec<(u64, u64)>>,
    slips: Vec<bool>,
    min_cost: u64,
    reachable: bool,
    construction: Vec<u64>,
}

fn register_edge(
    table: &mut BTreeMap<(u64, u64), (u64, u64)>,
    available: &mut BTreeMap<u64, Vec<(u64, u64)>>,
    from: u64,
    action: u64,
    to: u64,
    cost: u64,
) {
    table.insert((from, action), (to, cost));
    available.entry(from).or_default().push((action, cost));
}

fn build_episode(row: &RowSpec, allocator: &mut Allocator) -> BuiltEpisode {
    let params = params_for(row);
    let horizon = row.horizon() as usize;
    let mut table: BTreeMap<(u64, u64), (u64, u64)> = BTreeMap::new();
    let mut available: BTreeMap<u64, Vec<(u64, u64)>> = BTreeMap::new();
    let mut chain: Vec<u64> = Vec::with_capacity(params.length + 1);
    for _ in 0..=params.length {
        chain.push(allocator.draw());
    }
    let gate = params.gate && params.length >= 4;
    let mut flag_ids: BTreeMap<usize, u64> = BTreeMap::new();
    if gate {
        for node in 0..=params.length {
            flag_ids.insert(node, allocator.draw());
        }
    }
    let wire = |node: usize, flag: u64, chain: &[u64], flag_ids: &BTreeMap<usize, u64>| -> u64 {
        if flag == 0 {
            chain[node]
        } else {
            *flag_ids.get(&node).unwrap()
        }
    };
    let door = params.length * 2 / 3;
    let key = params.length / 3;
    for flag in if gate { vec![0u64, 1] } else { vec![0] } {
        for index in 0..params.length {
            let action = allocator.draw();
            let from = wire(index, flag, &chain, &flag_ids);
            let mut to_flag = flag;
            if gate && index + 1 == key + 1 {
                to_flag = 1;
            }
            let mut to = wire(index + 1, to_flag, &chain, &flag_ids);
            if gate && index == door && flag == 0 {
                to = from;
            }
            register_edge(&mut table, &mut available, from, action, to, 1);
        }
    }
    let mut candidate_positions: Vec<usize> = (1..params.length).collect();
    let mut fork_positions: Vec<usize> = Vec::new();
    while fork_positions.len() < params.forks && !candidate_positions.is_empty() {
        let pick = allocator.bounded(candidate_positions.len() as u64) as usize;
        fork_positions.push(candidate_positions.remove(pick));
    }
    fork_positions.sort_unstable();
    for position in &fork_positions {
        let sink = allocator.draw();
        let fork_action = allocator.draw();
        let back_action = allocator.draw();
        let from = wire(*position, 0, &chain, &flag_ids);
        let (chain_cost, fork_cost) = if params.inverted { (2, 1) } else { (1, 2) };
        if params.inverted {
            let chain_action = table
                .iter()
                .find(|((source, _), (target, _))| {
                    *source == from && *target != from && *target != sink
                })
                .map(|((_, action), _)| *action)
                .unwrap_or(0);
            if chain_action != 0 {
                if let Some(entry) = table.get_mut(&(from, chain_action)) {
                    entry.1 = chain_cost;
                }
                if let Some(list) = available.get_mut(&from) {
                    for (action, cost) in list.iter_mut() {
                        if *action == chain_action {
                            *cost = chain_cost;
                        }
                    }
                }
            }
        }
        register_edge(&mut table, &mut available, from, fork_action, sink, fork_cost);
        register_edge(&mut table, &mut available, sink, back_action, from, 1);
    }
    if params.dead_end {
        for index in 1..=params.length {
            let back_action = allocator.draw();
            register_edge(&mut table, &mut available, chain[index], back_action, chain[index - 1], 1);
        }
    }
    let arrival_flag = if gate { 1 } else { 0 };
    let goal = if params.dead_end {
        allocator.draw()
    } else {
        wire(params.length, arrival_flag, &chain, &flag_ids)
    };
    let mut slips = vec![false; horizon];
    if params.stochastic {
        let mut chosen: Vec<usize> = Vec::new();
        while chosen.len() < SLIP_STOCHASTIC_COUNT {
            let step = allocator.bounded(horizon as u64) as usize;
            if !chosen.contains(&step) {
                chosen.push(step);
            }
        }
        for step in chosen {
            slips[step] = true;
        }
    }
    if params.forced_slip && horizon > 0 {
        slips[0] = true;
    }
    let min_cost = dijkstra_min_cost(&table, chain[0], goal).unwrap_or(0);
    let budget = if params.dead_end {
        horizon as u64
    } else {
        min_cost + params.budget_slack
    };
    let construction = vec![
        params.length as u64,
        params.forks as u64,
        gate as u64,
        params.inverted as u64,
        budget,
        horizon as u64,
    ];
    BuiltEpisode {
        chain,
        goal,
        table,
        available,
        slips,
        min_cost,
        reachable: !params.dead_end,
        construction,
    }
}

pub fn dijkstra_min_cost(
    table: &BTreeMap<(u64, u64), (u64, u64)>,
    start: u64,
    goal: u64,
) -> Option<u64> {
    let mut dist: BTreeMap<u64, u64> = BTreeMap::new();
    dist.insert(start, 0);
    let mut visited: BTreeSet<u64> = BTreeSet::new();
    loop {
        let current = dist
            .iter()
            .filter(|(state, _)| !visited.contains(*state))
            .min_by_key(|(_, cost)| **cost)
            .map(|(state, _)| *state);
        let current = match current {
            Some(state) => state,
            None => return None,
        };
        let cost = dist[&current];
        if current == goal {
            return Some(cost);
        }
        visited.insert(current);
        let mut edges: Vec<(u64, u64)> = table
            .iter()
            .filter(|((from, _), _)| *from == current)
            .map(|((_, action), (to, edge_cost))| (*action, (*to, *edge_cost)))
            .map(|(_, pair)| pair)
            .collect();
        edges.sort_unstable();
        for (next, edge_cost) in edges {
            let candidate = cost + edge_cost;
            if candidate < *dist.get(&next).unwrap_or(&u64::MAX) {
                dist.insert(next, candidate);
            }
        }
    }
}

pub fn bfs_plan(
    table: &BTreeMap<(u64, u64), (u64, u64)>,
    start: u64,
    goal: u64,
    node_cap: usize,
) -> Option<(Vec<u64>, u64)> {
    use std::collections::VecDeque;
    if start == goal {
        return Some((Vec::new(), 0));
    }
    let mut parent: BTreeMap<u64, (u64, u64)> = BTreeMap::new();
    let mut queue = VecDeque::new();
    queue.push_back(start);
    parent.insert(start, (0, 0));
    let mut expanded = 0usize;
    while let Some(state) = queue.pop_front() {
        expanded += 1;
        if expanded > node_cap {
            return None;
        }
        let mut edges: Vec<(u64, u64)> = table
            .iter()
            .filter(|((from, _), _)| *from == state)
            .map(|((_, action), (to, _))| (*action, *to))
            .collect();
        edges.sort_unstable();
        for (action, next) in edges {
            if parent.contains_key(&next) {
                continue;
            }
            parent.insert(next, (state, action));
            if next == goal {
                let mut actions = vec![action];
                let mut cursor = state;
                let mut cost = table[&(cursor, action)].1;
                while cursor != start {
                    let (prev, act) = parent[&cursor];
                    actions.push(act);
                    cost += table[&(prev, act)].1;
                    cursor = prev;
                }
                actions.reverse();
                return Some((actions, cost));
            }
            queue.push_back(next);
        }
    }
    None
}

fn validate_built(row: &RowSpec, built: &BuiltEpisode) -> bool {
    let horizon = row.horizon() as usize;
    if built.chain.is_empty() || horizon == 0 {
        return false;
    }
    let states: BTreeSet<u64> = built
        .table
        .keys()
        .map(|(state, _)| *state)
        .chain(built.table.values().map(|(state, _)| *state))
        .collect();
    if states.len() > 64 {
        return false;
    }
    for actions in built.available.values() {
        if actions.is_empty() || actions.len() > 4 {
            return false;
        }
    }
    let start = built.chain[0];
    let plan = bfs_plan(&built.table, start, built.goal, horizon * 16);
    if built.reachable != plan.is_some() {
        return false;
    }
    if built.reachable {
        let (actions, _) = plan.unwrap();
        if actions.len() > horizon {
            return false;
        }
        if built.min_cost == 0 && !built.slips.iter().any(|slip| *slip) {
            return false;
        }
    }
    if row.layout == Layout::DeadEnd && built.reachable {
        return false;
    }
    true
}

pub fn generate_run(
    world_seed: u64,
    scope_seed: u64,
    training_tag: u64,
    development_tag: u64,
    final_tag: u64,
    mode: SymbolMode,
) -> RunEpisodes {
    let mut scope_rng = SplitMix64::new(scope_seed);
    let mut scope_used = BTreeSet::new();
    let next_scope = |rng: &mut SplitMix64, used: &mut BTreeSet<u64>| -> u64 {
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
    let mut identity_used: BTreeSet<u64> = BTreeSet::new();
    let mut episodes = Vec::with_capacity(240);
    let mut global_ordinal: u64 = 0;
    for row in all_rows() {
        global_ordinal += 1;
        let tag = match row.partition {
            Partition::Training => training_tag,
            Partition::Development => development_tag,
            Partition::Final => final_tag,
        };
        let built;
        let mut redraws: u32 = 0;
        loop {
            let seed = record_seed(world_seed, tag, global_ordinal, redraws);
            let mut allocator = Allocator {
                state: SplitMix64::new(seed),
                used: identity_used.clone(),
            };
            let candidate = build_episode(&row, &mut allocator);
            if validate_built(&row, &candidate) {
                identity_used = allocator.used;
                built = candidate;
                break;
            }
            redraws += 1;
            assert!(redraws <= MAX_GENERATOR_REDRAWS, "generator redraw budget exhausted");
        }
        let episode_scope = next_scope(&mut scope_rng, &mut scope_used);
        let remap = |permuter: &mut Option<Permuter>, id: u64| -> u64 {
            match permuter {
                None => id,
                Some(map) => map.map(id),
            }
        };
        let mut table = BTreeMap::new();
        for ((from, action), (to, cost)) in &built.table {
            table.insert(
                (remap(&mut permuter, *from), remap(&mut permuter, *action)),
                (remap(&mut permuter, *to), *cost),
            );
        }
        let mut available = BTreeMap::new();
        for (state, actions) in &built.available {
            available.insert(
                remap(&mut permuter, *state),
                actions
                    .iter()
                    .map(|(action, cost)| (remap(&mut permuter, *action), *cost))
                    .collect(),
            );
        }
        let wire_start = remap(&mut permuter, built.chain[0]);
        let goal_target = remap(&mut permuter, built.goal);
        let latent_chain: Vec<u64> =
            built.chain.iter().map(|id| remap(&mut permuter, *id)).collect();
        let budget = built.construction[4];
        episodes.push(GeneratedEpisode {
            wire_start,
            goal_target,
            goal_budget: budget,
            horizon: row.horizon(),
            table,
            available,
            slip_schedule: built.slips,
            run_scope,
            episode_scope,
            redraws,
            global_ordinal,
            partition: row.partition,
            row_ordinal: row.ordinal,
            latent_chain,
            min_cost: built.min_cost,
            reachable: built.reachable,
            construction: built.construction,
        });
    }
    RunEpisodes { episodes, run_scope }
}

#[cfg(test)]
mod tests {
    use super::super::protocol::{
        PRIMARY_SCOPE_SEED, PRIMARY_WORLD_SEED, TRAINING_PARTITION_TAG,
    };
    use super::*;

    const DEV_TAG: u64 = 0x4445_5600_0000_0002;
    const FINAL_TAG: u64 = 0x4649_4e00_0000_0002;

    fn primary_run() -> RunEpisodes {
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
        assert_eq!(run.episodes.len(), 240);
        assert_eq!(run.episodes.first().unwrap().global_ordinal, 1);
        assert_eq!(run.episodes.last().unwrap().global_ordinal, 240);
        assert_eq!(run.episodes[95].row_ordinal, 96);
        assert_eq!(run.episodes[96].partition, Partition::Development);
        assert_eq!(run.episodes[144].partition, Partition::Final);
        for pair in run.episodes.windows(2) {
            assert_eq!(pair[1].global_ordinal, pair[0].global_ordinal + 1);
        }
    }

    #[test]
    fn generation_is_deterministic_for_fixed_seeds() {
        assert_eq!(primary_run(), primary_run());
    }

    #[test]
    fn identities_are_globally_unique_and_nonzero() {
        let run = primary_run();
        let mut seen = BTreeSet::new();
        for episode in &run.episodes {
            let mut local = BTreeSet::new();
            for ((from, action), (to, _)) in &episode.table {
                for id in [*from, *action, *to] {
                    assert_ne!(id, 0);
                    local.insert(id);
                }
            }
            local.insert(episode.goal_target);
            for id in local {
                assert!(seen.insert(id), "identity {id} reused across episodes");
            }
        }
    }

    #[test]
    fn reachable_episodes_fit_horizon_and_budget() {
        let run = primary_run();
        for episode in &run.episodes {
            assert!(episode.horizon == 4 || episode.horizon == 8 || episode.horizon == 16);
            assert!(episode.slip_schedule.len() == episode.horizon as usize);
            if episode.reachable {
                let plan = bfs_plan(&episode.table, episode.wire_start, episode.goal_target, 4096)
                    .expect("reachable episode must have a plan");
                assert!(plan.0.len() <= episode.horizon as usize, "episode {}", episode.global_ordinal);
                assert!(plan.1 <= episode.goal_budget, "episode {}", episode.global_ordinal);
            }
        }
    }

    #[test]
    fn replan_rows_carry_exactly_one_forced_slip_at_step_zero() {
        let run = primary_run();
        for episode in &run.episodes {
            if episode.partition == Partition::Final && (85..=96).contains(&episode.row_ordinal) {
                assert_eq!(
                    episode.slip_schedule.iter().filter(|slip| **slip).count(),
                    1,
                    "episode {}",
                    episode.global_ordinal
                );
                assert!(episode.slip_schedule[0], "episode {}", episode.global_ordinal);
            }
        }
    }

    #[test]
    fn stochastic_rows_carry_exactly_two_drawn_slips() {
        let run = primary_run();
        for episode in &run.episodes {
            if episode.partition == Partition::Final && (61..=72).contains(&episode.row_ordinal) {
                assert_eq!(
                    episode.slip_schedule.iter().filter(|slip| **slip).count(),
                    2,
                    "episode {}",
                    episode.global_ordinal
                );
            }
        }
    }

    #[test]
    fn dead_end_goals_are_isolated() {
        let run = primary_run();
        for episode in &run.episodes {
            if episode.partition == Partition::Final && (73..=84).contains(&episode.row_ordinal) {
                assert!(!episode.reachable);
                assert!(bfs_plan(&episode.table, episode.wire_start, episode.goal_target, 4096).is_none());
            }
        }
    }

    #[test]
    fn permutation_mode_preserves_structure_with_disjoint_identities() {
        let primary = primary_run();
        let permuted = generate_run(
            PRIMARY_WORLD_SEED,
            PRIMARY_SCOPE_SEED,
            TRAINING_PARTITION_TAG,
            DEV_TAG,
            FINAL_TAG,
            SymbolMode::Permutation,
        );
        assert_eq!(primary.episodes.len(), permuted.episodes.len());
        for (plain, mapped) in primary.episodes.iter().zip(permuted.episodes.iter()) {
            assert_eq!(plain.table.len(), mapped.table.len());
            assert_eq!(plain.horizon, mapped.horizon);
            assert_eq!(plain.episode_scope, mapped.episode_scope);
            assert_eq!(plain.min_cost, mapped.min_cost);
            assert_eq!(plain.reachable, mapped.reachable);
            assert_eq!(plain.slip_schedule, mapped.slip_schedule);
        }
        let plain_ids: BTreeSet<u64> = primary
            .episodes
            .iter()
            .flat_map(|episode| {
                episode.table.keys().flat_map(|(from, action)| [*from, *action].into_iter())
            })
            .collect();
        let mapped_ids: BTreeSet<u64> = permuted
            .episodes
            .iter()
            .flat_map(|episode| {
                episode.table.keys().flat_map(|(from, action)| [*from, *action].into_iter())
            })
            .collect();
        assert!(plain_ids.intersection(&mapped_ids).next().is_none());
    }
}
