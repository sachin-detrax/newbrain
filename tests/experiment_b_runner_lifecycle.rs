use sparse_structural_rule_learner::experiment_b::generator::{generate_run, SymbolMode};
use sparse_structural_rule_learner::experiment_b::learner::{
    ExperimentBLearner, FrozenExperimentBLearner, LearnerError,
};
use sparse_structural_rule_learner::experiment_b::protocol::{
    Abstain, AbstainReason, Act, EpisodeEvent, GRAMMAR_VERSION, PlannerDecision, PlannerResponse,
    PRIMARY_SCOPE_SEED, PRIMARY_WORLD_SEED, TRAINING_PARTITION_TAG,
};
use sparse_structural_rule_learner::experiment_b::runner::drive_learner;

const DEV_TAG: u64 = 0x4445_5600_0000_0002;
const FINAL_TAG: u64 = 0x4649_4e00_0000_0002;

struct Probe {
    state: u64,
    scope: u64,
    sequence: u64,
    available: Vec<u64>,
    development_at: Option<usize>,
}

struct ProbeFrozen {
    state: u64,
    scope: u64,
    sequence: u64,
    available: Vec<u64>,
    fingerprint: String,
}

fn decide(scope: u64, available: &[u64], sequence: u64, fingerprint: &str) -> PlannerDecision {
    let response = match available.first().copied() {
        Some(action) => PlannerResponse::Act(Act {
            action,
            procedure_refs: Vec::new(),
            plan_refs: vec![sequence],
            evidence_refs: Vec::new(),
            hypothesis_refs: Vec::new(),
            optional_prediction: None,
        }),
        None => PlannerResponse::Abstain(Abstain {
            reason: AbstainReason::InsufficientEvidence,
            hypothesis_refs: Vec::new(),
            evidence_refs: Vec::new(),
            plan_refs: Vec::new(),
        }),
    };
    PlannerDecision {
        episode_scope: scope,
        step_index: 0,
        response,
        grammar_version: GRAMMAR_VERSION.to_owned(),
        model_fingerprint: fingerprint.to_owned(),
        evidence_refs: Vec::new(),
        operation_cost: 1,
        sequence,
    }
}

impl ExperimentBLearner for Probe {
    type Frozen = ProbeFrozen;

    fn observe(&mut self, event: EpisodeEvent) -> Result<(), LearnerError> {
        match event {
            EpisodeEvent::Observation { episode_scope, state, available_actions, .. } => {
                self.scope = episode_scope;
                self.state = state;
                self.available = available_actions;
                Ok(())
            }
            EpisodeEvent::ActionResult { .. } => Ok(()),
            EpisodeEvent::End { .. } => Ok(()),
        }
    }

    fn plan(&mut self) -> Result<PlannerDecision, LearnerError> {
        self.sequence += 1;
        Ok(decide(self.scope, &self.available.clone(), self.sequence, "probe"))
    }

    fn ingest(&mut self, event: EpisodeEvent) -> Result<(), LearnerError> {
        match event {
            EpisodeEvent::ActionResult { resulting_state, .. } => {
                self.state = resulting_state;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn begin_development(&mut self) -> Result<(), LearnerError> {
        self.development_at = Some(1);
        Ok(())
    }

    fn freeze(self) -> Result<Self::Frozen, LearnerError> {
        Ok(ProbeFrozen {
            state: self.state,
            scope: self.scope,
            sequence: self.sequence,
            available: self.available,
            fingerprint: "probe-frozen".to_owned(),
        })
    }
}

impl FrozenExperimentBLearner for ProbeFrozen {
    fn observe(&mut self, event: EpisodeEvent) -> Result<(), LearnerError> {
        match event {
            EpisodeEvent::Observation { episode_scope, state, available_actions, .. } => {
                self.scope = episode_scope;
                self.state = state;
                self.available = available_actions;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn plan(&mut self) -> Result<PlannerDecision, LearnerError> {
        self.sequence += 1;
        Ok(decide(self.scope, &self.available.clone(), self.sequence, &self.fingerprint.clone()))
    }

    fn ingest(&mut self, event: EpisodeEvent) -> Result<(), LearnerError> {
        match event {
            EpisodeEvent::ActionResult { resulting_state, .. } => {
                self.state = resulting_state;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn model_fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

fn probe() -> Probe {
    Probe { state: 0, scope: 0, sequence: 0, available: Vec::new(), development_at: None }
}

fn primary_episodes() -> Vec<sparse_structural_rule_learner::experiment_b::generator::GeneratedEpisode> {
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

#[test]
fn runner_delivers_240_episodes_with_registered_phase_boundaries() {
    let episodes = primary_episodes();
    let (trace, _) = drive_learner(&episodes, probe()).unwrap();
    assert_eq!(trace.episodes.len(), 240);
    assert_eq!(trace.development_called_after, 96);
    assert_eq!(trace.frozen_after, 144);
    for (index, outcome) in trace.episodes.iter().enumerate() {
        assert_eq!(outcome.global_ordinal, (index + 1) as u64);
    }
}

#[test]
fn frozen_fingerprint_is_stable_across_finals() {
    let episodes = primary_episodes();
    let (trace, _) = drive_learner(&episodes, probe()).unwrap();
    assert_eq!(trace.fingerprint_before_final, "probe-frozen");
    assert_eq!(trace.fingerprint_before_final, trace.fingerprint_after_final);
}

#[test]
fn runner_accounts_every_observation_and_action() {
    let episodes = primary_episodes();
    let (trace, _) = drive_learner(&episodes, probe()).unwrap();
    let observations: u64 = trace.episodes.iter().map(|outcome| outcome.decisions.len() as u64 + 1).sum();
    assert_eq!(trace.received_observations_run, observations);
    let actions: u64 = trace.episodes.iter().map(|outcome| outcome.actions.len() as u64).sum();
    assert_eq!(trace.actions_issued_run, actions);
}
