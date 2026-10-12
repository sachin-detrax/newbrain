use super::generator::GeneratedEpisode;
use super::learner::{ExperimentBLearner, FrozenExperimentBLearner, LearnerError};
use super::protocol::{
    Act, Budget, BudgetError, EpisodeEvent, GoalDescriptor, PlannerDecision, PlannerResponse,
    ResourceKind, TerminalStatus,
};
use std::fmt;
use std::time::Instant;

pub const TRAINING_EPISODES: usize = 96;
pub const MUTABLE_EPISODES: usize = 144;
pub const GOAL_PREDICATE: u64 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunnerError {
    Learner(LearnerError),
    Budget(BudgetError),
    PhaseOrder(String),
    Protocol(String),
}

impl fmt::Display for RunnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Learner(error) => write!(formatter, "learner failed: {error}"),
            Self::Budget(error) => write!(formatter, "runner budget exhausted: {error}"),
            Self::PhaseOrder(detail) => write!(formatter, "phase order violated: {detail}"),
            Self::Protocol(detail) => write!(formatter, "protocol violated: {detail}"),
        }
    }
}

impl std::error::Error for RunnerError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpisodeOutcome {
    pub global_ordinal: u64,
    pub episode_scope: u64,
    pub steps_taken: usize,
    pub decisions: Vec<PlannerDecision>,
    pub actions: Vec<u64>,
    pub total_cost: u64,
    pub terminal: TerminalStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunTrace {
    pub episodes: Vec<EpisodeOutcome>,
    pub development_called_after: usize,
    pub frozen_after: usize,
    pub fingerprint_before_final: String,
    pub fingerprint_after_final: String,
    pub received_observations_run: u64,
    pub actions_issued_run: u64,
    pub wall_clock_ns: u128,
}

fn observation_for(
    episode: &GeneratedEpisode,
    goal: &GoalDescriptor,
    state: u64,
    step_index: u32,
    previous_terminal: TerminalStatus,
) -> EpisodeEvent {
    let mut available_actions = Vec::new();
    let mut action_costs = Vec::new();
    if let Some(actions) = episode.available.get(&state) {
        for (action, cost) in actions {
            available_actions.push(*action);
            action_costs.push(*cost);
        }
    }
    EpisodeEvent::Observation {
        run_scope: episode.run_scope,
        episode_scope: episode.episode_scope,
        step_index,
        state,
        available_actions,
        action_costs,
        goal: goal.clone(),
        horizon_remaining: episode.horizon.saturating_sub(step_index as u16),
        previous_terminal,
    }
}

fn apply_action(
    episode: &GeneratedEpisode,
    state: u64,
    action: u64,
    step_index: usize,
) -> Result<(u64, u64), RunnerError> {
    let (next, cost) = episode
        .table
        .get(&(state, action))
        .copied()
        .ok_or_else(|| RunnerError::Protocol(format!("undeclared action {action} at state {state}")))?;
    if episode.slip_schedule.get(step_index).copied().unwrap_or(false) {
        Ok((state, cost))
    } else {
        Ok((next, cost))
    }
}

fn drive_episode<L: ExperimentBLearner>(
    episode: &GeneratedEpisode,
    goal: &GoalDescriptor,
    learner: &mut L,
    budget: &mut Budget,
) -> Result<EpisodeOutcome, RunnerError> {
    budget.begin_episode();
    let mut state = episode.wire_start;
    let mut decisions = Vec::new();
    let mut actions = Vec::new();
    let mut total_cost = 0u64;
    let mut terminal = TerminalStatus::NotTerminated;
    learner
        .observe(observation_for(episode, goal, state, 0, TerminalStatus::NotTerminated))
        .map_err(RunnerError::Learner)?;
    budget.charge(ResourceKind::ReceivedObservations, 1).map_err(RunnerError::Budget)?;
    let mut step_index: usize = 0;
    loop {
        let decision = learner.plan().map_err(RunnerError::Learner)?;
        let act: Act = match &decision.response {
            PlannerResponse::Act(act) => act.clone(),
            PlannerResponse::Abstain(_) => {
                decisions.push(decision);
                learner
                    .observe(EpisodeEvent::End {
                        run_scope: episode.run_scope,
                        episode_scope: episode.episode_scope,
                        terminal: TerminalStatus::NotTerminated,
                    })
                    .map_err(RunnerError::Learner)?;
                budget.charge(ResourceKind::ReceivedObservations, 1).map_err(RunnerError::Budget)?;
                break;
            }
        };
        decisions.push(decision);
        actions.push(act.action);
        budget.charge(ResourceKind::ActionsIssued, 1).map_err(RunnerError::Budget)?;
        let (next, cost) = apply_action(episode, state, act.action, step_index)?;
        total_cost += cost;
        step_index += 1;
        terminal = if next == goal.target_state {
            TerminalStatus::GoalSatisfied
        } else if step_index >= episode.horizon as usize {
            TerminalStatus::HorizonExhausted
        } else {
            TerminalStatus::NotTerminated
        };
        learner
            .ingest(EpisodeEvent::ActionResult {
                run_scope: episode.run_scope,
                episode_scope: episode.episode_scope,
                step_index: step_index as u32,
                resulting_state: next,
                cost_paid: cost,
                terminal,
            })
            .map_err(RunnerError::Learner)?;
        if terminal != TerminalStatus::NotTerminated {
            learner
                .observe(EpisodeEvent::End {
                    run_scope: episode.run_scope,
                    episode_scope: episode.episode_scope,
                    terminal,
                })
                .map_err(RunnerError::Learner)?;
            budget.charge(ResourceKind::ReceivedObservations, 1).map_err(RunnerError::Budget)?;
            break;
        }
        state = next;
        learner
            .observe(observation_for(episode, goal, state, step_index as u32, TerminalStatus::NotTerminated))
            .map_err(RunnerError::Learner)?;
        budget.charge(ResourceKind::ReceivedObservations, 1).map_err(RunnerError::Budget)?;
    }
    Ok(EpisodeOutcome {
        global_ordinal: episode.global_ordinal,
        episode_scope: episode.episode_scope,
        steps_taken: step_index,
        decisions,
        actions,
        total_cost,
        terminal,
    })
}

fn drive_episode_frozen<L: FrozenExperimentBLearner>(
    episode: &GeneratedEpisode,
    goal: &GoalDescriptor,
    learner: &mut L,
    budget: &mut Budget,
) -> Result<EpisodeOutcome, RunnerError> {
    budget.begin_episode();
    let mut state = episode.wire_start;
    let mut decisions = Vec::new();
    let mut actions = Vec::new();
    let mut total_cost = 0u64;
    let mut terminal = TerminalStatus::NotTerminated;
    learner
        .observe(observation_for(episode, goal, state, 0, TerminalStatus::NotTerminated))
        .map_err(RunnerError::Learner)?;
    budget.charge(ResourceKind::ReceivedObservations, 1).map_err(RunnerError::Budget)?;
    let mut step_index: usize = 0;
    loop {
        let decision = learner.plan().map_err(RunnerError::Learner)?;
        let act: Act = match &decision.response {
            PlannerResponse::Act(act) => act.clone(),
            PlannerResponse::Abstain(_) => {
                decisions.push(decision);
                learner
                    .observe(EpisodeEvent::End {
                        run_scope: episode.run_scope,
                        episode_scope: episode.episode_scope,
                        terminal: TerminalStatus::NotTerminated,
                    })
                    .map_err(RunnerError::Learner)?;
                budget.charge(ResourceKind::ReceivedObservations, 1).map_err(RunnerError::Budget)?;
                break;
            }
        };
        decisions.push(decision);
        actions.push(act.action);
        budget.charge(ResourceKind::ActionsIssued, 1).map_err(RunnerError::Budget)?;
        let (next, cost) = apply_action(episode, state, act.action, step_index)?;
        total_cost += cost;
        step_index += 1;
        terminal = if next == goal.target_state {
            TerminalStatus::GoalSatisfied
        } else if step_index >= episode.horizon as usize {
            TerminalStatus::HorizonExhausted
        } else {
            TerminalStatus::NotTerminated
        };
        learner
            .ingest(EpisodeEvent::ActionResult {
                run_scope: episode.run_scope,
                episode_scope: episode.episode_scope,
                step_index: step_index as u32,
                resulting_state: next,
                cost_paid: cost,
                terminal,
            })
            .map_err(RunnerError::Learner)?;
        if terminal != TerminalStatus::NotTerminated {
            learner
                .observe(EpisodeEvent::End {
                    run_scope: episode.run_scope,
                    episode_scope: episode.episode_scope,
                    terminal,
                })
                .map_err(RunnerError::Learner)?;
            budget.charge(ResourceKind::ReceivedObservations, 1).map_err(RunnerError::Budget)?;
            break;
        }
        state = next;
        learner
            .observe(observation_for(episode, goal, state, step_index as u32, TerminalStatus::NotTerminated))
            .map_err(RunnerError::Learner)?;
        budget.charge(ResourceKind::ReceivedObservations, 1).map_err(RunnerError::Budget)?;
    }
    Ok(EpisodeOutcome {
        global_ordinal: episode.global_ordinal,
        episode_scope: episode.episode_scope,
        steps_taken: step_index,
        decisions,
        actions,
        total_cost,
        terminal,
    })
}

fn goal_for(episode: &GeneratedEpisode) -> GoalDescriptor {
    GoalDescriptor::new(GOAL_PREDICATE, episode.goal_target, episode.goal_budget)
}

pub fn drive_learner<L: ExperimentBLearner>(
    episodes: &[GeneratedEpisode],
    mut learner: L,
) -> Result<(RunTrace, L::Frozen), RunnerError> {
    if episodes.len() != 240 {
        return Err(RunnerError::PhaseOrder(format!(
            "expected 240 episodes, got {}",
            episodes.len()
        )));
    }
    let started = Instant::now();
    let mut budget = Budget::default();
    let mut outcomes = Vec::with_capacity(240);
    for episode in &episodes[..TRAINING_EPISODES] {
        outcomes.push(drive_episode(episode, &goal_for(episode), &mut learner, &mut budget)?);
    }
    learner.begin_development().map_err(RunnerError::Learner)?;
    let development_called_after = outcomes.len();
    for episode in &episodes[TRAINING_EPISODES..MUTABLE_EPISODES] {
        outcomes.push(drive_episode(episode, &goal_for(episode), &mut learner, &mut budget)?);
    }
    let mut frozen = learner.freeze().map_err(RunnerError::Learner)?;
    let frozen_after = outcomes.len();
    let fingerprint_before_final = frozen.model_fingerprint().to_owned();
    for episode in &episodes[MUTABLE_EPISODES..] {
        outcomes.push(drive_episode_frozen(episode, &goal_for(episode), &mut frozen, &mut budget)?);
    }
    let fingerprint_after_final = frozen.model_fingerprint().to_owned();
    Ok((
        RunTrace {
            received_observations_run: budget.run_value(ResourceKind::ReceivedObservations),
            actions_issued_run: budget.run_value(ResourceKind::ActionsIssued),
            episodes: outcomes,
            development_called_after,
            frozen_after,
            fingerprint_before_final,
            fingerprint_after_final,
            wall_clock_ns: started.elapsed().as_nanos(),
        },
        frozen,
    ))
}
