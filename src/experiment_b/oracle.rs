use super::generator::{bfs_plan, GeneratedEpisode};
use super::protocol::{ActionId, StateId};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoldPlanStep {
    pub from_state: StateId,
    pub action: ActionId,
    pub resulting_state: StateId,
    pub cost: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoldPlan {
    pub steps: Vec<GoldPlanStep>,
    pub total_cost: u64,
    pub reaches_goal: bool,
}

impl GoldPlan {
    pub fn start_state(&self) -> Option<StateId> {
        self.steps.first().map(|step| step.from_state)
    }

    pub fn final_state(&self) -> Option<StateId> {
        self.steps.last().map(|step| step.resulting_state)
    }
}

pub fn gold_plans(episode: &GeneratedEpisode) -> Vec<GoldPlan> {
    match bfs_plan(&episode.table, episode.wire_start, episode.goal_target, 4096) {
        None => Vec::new(),
        Some((actions, _)) => {
            let mut steps = Vec::with_capacity(actions.len());
            let mut cursor = episode.wire_start;
            let mut total_cost = 0u64;
            for action in actions {
                let (next, cost) = match episode.table.get(&(cursor, action)) {
                    Some(pair) => *pair,
                    None => return Vec::new(),
                };
                steps.push(GoldPlanStep {
                    from_state: cursor,
                    action,
                    resulting_state: next,
                    cost,
                });
                total_cost += cost;
                cursor = next;
            }
            vec![GoldPlan { steps, total_cost, reaches_goal: cursor == episode.goal_target }]
        }
    }
}

pub fn goal_satisfied(episode: &GeneratedEpisode, final_observed_state: StateId) -> bool {
    final_observed_state == episode.goal_target
}

pub fn validate_episode(episode: &GeneratedEpisode) -> Result<(), String> {
    let ordinal = episode.global_ordinal;
    if episode.horizon != 4 && episode.horizon != 8 && episode.horizon != 16 {
        return Err(format!("episode {ordinal} horizon {} not in {{4,8,16}}", episode.horizon));
    }
    for ((from, action), (to, _)) in episode.table.iter() {
        if *from == 0 || *action == 0 || *to == 0 {
            return Err(format!("episode {ordinal} contains zero identity"));
        }
        match episode.available.get(from) {
            Some(actions) if actions.iter().any(|(id, _)| id == action) => {}
            _ => return Err(format!("episode {ordinal} table edge missing from available")),
        }
    }
    for (state, actions) in episode.available.iter() {
        if actions.is_empty() {
            return Err(format!("episode {ordinal} state {state} has no actions"));
        }
        for (action, _) in actions {
            if !episode.table.contains_key(&(*state, *action)) {
                return Err(format!("episode {ordinal} available action without edge"));
            }
        }
    }
    if episode.slip_schedule.len() != episode.horizon as usize {
        return Err(format!("episode {ordinal} slip schedule length mismatch"));
    }
    let plans = gold_plans(episode);
    if episode.reachable {
        if plans.is_empty() || !plans[0].reaches_goal {
            return Err(format!("episode {ordinal} reachable without gold plan"));
        }
        if plans[0].steps.len() > episode.horizon as usize {
            return Err(format!("episode {ordinal} gold plan exceeds horizon"));
        }
        if plans[0].total_cost != episode.min_cost {
            return Err(format!(
                "episode {ordinal} gold cost {} differs from manifest {}",
                plans[0].total_cost, episode.min_cost
            ));
        }
        if plans[0].total_cost > episode.goal_budget {
            return Err(format!("episode {ordinal} gold cost exceeds budget"));
        }
    } else {
        if !plans.is_empty() {
            return Err(format!("episode {ordinal} unreachable with gold plan"));
        }
        let states: BTreeSet<u64> = episode
            .table
            .keys()
            .map(|(state, _)| *state)
            .chain(episode.table.values().map(|(state, _)| *state))
            .collect();
        if states.contains(&episode.goal_target) {
            return Err(format!("episode {ordinal} dead-end goal touches the graph"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::generator::{generate_run, SymbolMode};
    use super::super::protocol::{
        DEVELOPMENT_PARTITION_TAG, FINAL_PARTITION_TAG, PRIMARY_SCOPE_SEED, PRIMARY_WORLD_SEED,
        TRAINING_PARTITION_TAG,
    };
    use super::super::schedule::Stratum;
    use super::*;

    const DEV_TAG: u64 = DEVELOPMENT_PARTITION_TAG;
    const FINAL_TAG: u64 = FINAL_PARTITION_TAG;

    fn primary_episodes() -> Vec<GeneratedEpisode> {
        generate_run(
            PRIMARY_WORLD_SEED,
            PRIMARY_SCOPE_SEED,
            TRAINING_PARTITION_TAG,
            DEV_TAG,
            FINAL_TAG,
            SymbolMode::Primary,
        )
        .episodes
    }

    fn permutation_episodes() -> Vec<GeneratedEpisode> {
        generate_run(
            PRIMARY_WORLD_SEED,
            PRIMARY_SCOPE_SEED,
            TRAINING_PARTITION_TAG,
            DEV_TAG,
            FINAL_TAG,
            SymbolMode::Permutation,
        )
        .episodes
    }

    #[test]
    fn every_primary_episode_validates() {
        for episode in primary_episodes() {
            validate_episode(&episode).unwrap_or_else(|error| {
                panic!("primary episode {} failed: {error}", episode.global_ordinal)
            });
        }
    }

    #[test]
    fn every_permuted_episode_validates() {
        for episode in permutation_episodes() {
            validate_episode(&episode).unwrap_or_else(|error| {
                panic!("permuted episode {} failed: {error}", episode.global_ordinal)
            });
        }
    }

    #[test]
    fn gold_plans_match_reachability() {
        for episode in primary_episodes() {
            let plans = gold_plans(&episode);
            if episode.reachable {
                assert_eq!(plans.len(), 1, "episode {}", episode.global_ordinal);
                assert!(plans[0].reaches_goal);
                assert_eq!(plans[0].final_state(), Some(episode.goal_target));
                assert_eq!(plans[0].start_state(), Some(episode.wire_start));
            } else {
                assert!(plans.is_empty(), "episode {}", episode.global_ordinal);
            }
        }
    }

    #[test]
    fn dead_end_rows_validate_as_unreachable() {
        let mut count = 0;
        for episode in primary_episodes() {
            if matches!(episode.partition, super::super::schedule::Partition::Final)
                && matches!(
                    super::super::schedule::FINAL_ROWS[(episode.row_ordinal - 1) as usize].stratum,
                    Some(Stratum::DeadEnd)
                )
            {
                count += 1;
                assert!(!episode.reachable);
                assert!(gold_plans(&episode).is_empty());
            }
        }
        assert_eq!(count, 12);
    }

    #[test]
    fn replan_rows_carry_exactly_one_slip_and_admissible_plans() {
        for episode in primary_episodes() {
            if matches!(episode.partition, super::super::schedule::Partition::Final)
                && (85..=96).contains(&episode.row_ordinal)
            {
                assert_eq!(episode.slip_schedule.iter().filter(|slip| **slip).count(), 1);
                assert!(!gold_plans(&episode).is_empty());
            }
        }
    }

    #[test]
    fn goal_satisfied_matches_target_state() {
        for episode in primary_episodes() {
            assert!(goal_satisfied(&episode, episode.goal_target));
            assert!(!goal_satisfied(&episode, episode.wire_start) || episode.wire_start == episode.goal_target);
        }
    }
}
