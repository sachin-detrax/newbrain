use sparse_structural_rule_learner::experiment_b::learner::{
    ExperimentBLearner, FrozenExperimentBLearner, LearnerError,
};
use sparse_structural_rule_learner::experiment_b::protocol::{
    Abstain, AbstainReason, EpisodeEvent, GoalDescriptor, PlannerDecision, PlannerResponse,
    TerminalStatus, GRAMMAR_VERSION,
};

struct UnitLearner {
    episode_scope: u64,
    sequence: u64,
    last_step: u32,
}

struct UnitFrozen {
    episode_scope: u64,
    fingerprint: String,
    last_step: u32,
}

fn event_scope(event: &EpisodeEvent) -> u64 {
    match *event {
        EpisodeEvent::Observation { episode_scope, .. }
        | EpisodeEvent::ActionResult { episode_scope, .. }
        | EpisodeEvent::End { episode_scope, .. } => episode_scope,
    }
}

fn event_step(event: &EpisodeEvent) -> Option<u32> {
    match *event {
        EpisodeEvent::Observation { step_index, .. }
        | EpisodeEvent::ActionResult { step_index, .. } => Some(step_index),
        EpisodeEvent::End { .. } => None,
    }
}

impl ExperimentBLearner for UnitLearner {
    type Frozen = UnitFrozen;

    fn observe(&mut self, event: EpisodeEvent) -> Result<(), LearnerError> {
        let scope = event_scope(&event);
        if self.sequence == 0 && self.episode_scope == 0 {
            self.episode_scope = scope;
        }
        if scope != self.episode_scope {
            return Err(LearnerError::InvalidScope);
        }
        if let Some(step) = event_step(&event) {
            self.last_step = step;
        }
        Ok(())
    }

    fn plan(&mut self) -> Result<PlannerDecision, LearnerError> {
        self.sequence += 1;
        Ok(PlannerDecision {
            episode_scope: self.episode_scope,
            step_index: self.last_step,
            response: PlannerResponse::Abstain(Abstain {
                reason: AbstainReason::InsufficientEvidence,
                hypothesis_refs: Vec::new(),
                evidence_refs: Vec::new(),
                plan_refs: Vec::new(),
            }),
            grammar_version: GRAMMAR_VERSION.to_owned(),
            model_fingerprint: "unit-learner-b-v1".to_owned(),
            evidence_refs: Vec::new(),
            operation_cost: 0,
            sequence: self.sequence,
        })
    }

    fn ingest(&mut self, event: EpisodeEvent) -> Result<(), LearnerError> {
        self.observe(event)
    }

    fn begin_development(&mut self) -> Result<(), LearnerError> {
        Ok(())
    }

    fn freeze(self) -> Result<Self::Frozen, LearnerError> {
        Ok(UnitFrozen {
            episode_scope: self.episode_scope,
            fingerprint: "unit-learner-b-v1".to_owned(),
            last_step: self.last_step,
        })
    }
}

impl FrozenExperimentBLearner for UnitFrozen {
    fn observe(&mut self, event: EpisodeEvent) -> Result<(), LearnerError> {
        let scope = event_scope(&event);
        if scope != self.episode_scope && self.episode_scope != 0 {
            return Err(LearnerError::InvalidScope);
        }
        self.episode_scope = scope;
        if let Some(step) = event_step(&event) {
            self.last_step = step;
        }
        Ok(())
    }

    fn plan(&mut self) -> Result<PlannerDecision, LearnerError> {
        Ok(PlannerDecision {
            episode_scope: self.episode_scope,
            step_index: self.last_step,
            response: PlannerResponse::Abstain(Abstain {
                reason: AbstainReason::InsufficientEvidence,
                hypothesis_refs: Vec::new(),
                evidence_refs: Vec::new(),
                plan_refs: Vec::new(),
            }),
            grammar_version: GRAMMAR_VERSION.to_owned(),
            model_fingerprint: self.fingerprint.clone(),
            evidence_refs: Vec::new(),
            operation_cost: 0,
            sequence: 0,
        })
    }

    fn ingest(&mut self, event: EpisodeEvent) -> Result<(), LearnerError> {
        self.observe(event)
    }

    fn model_fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

#[test]
fn unit_learner_speaks_only_the_opaque_boundary() {
    let mut learner = UnitLearner {
        episode_scope: 0,
        sequence: 0,
        last_step: 0,
    };
    assert!(learner.begin_development().is_ok());
    let goal = GoalDescriptor::new(0xA11CE, 42, 10);
    learner
        .observe(EpisodeEvent::Observation {
            run_scope: 7,
            episode_scope: 9,
            step_index: 0,
            state: 100,
            available_actions: vec![1, 2, 3],
            action_costs: vec![1, 1, 1],
            goal: goal.clone(),
            horizon_remaining: 4,
            previous_terminal: TerminalStatus::NotTerminated,
        })
        .unwrap();
    let decision = learner.plan().unwrap();
    assert_eq!(decision.episode_scope, 9);
    assert_eq!(decision.step_index, 0);
    assert!(matches!(
        decision.response,
        PlannerResponse::Abstain(Abstain {
            reason: AbstainReason::InsufficientEvidence,
            ..
        })
    ));
    assert_eq!(decision.grammar_version, GRAMMAR_VERSION);
    assert_eq!(decision.evidence_refs, Vec::<u64>::new());
    learner
        .ingest(EpisodeEvent::ActionResult {
            run_scope: 7,
            episode_scope: 9,
            step_index: 0,
            resulting_state: 101,
            cost_paid: 1,
            terminal: TerminalStatus::NotTerminated,
        })
        .unwrap();
    learner
        .ingest(EpisodeEvent::End {
            run_scope: 7,
            episode_scope: 9,
            terminal: TerminalStatus::GoalSatisfied,
        })
        .unwrap();
    let frozen = learner.freeze().unwrap();
    assert_eq!(frozen.model_fingerprint(), "unit-learner-b-v1");
}

#[test]
fn unit_learner_rejects_cross_episode_scope() {
    let mut learner = UnitLearner {
        episode_scope: 0,
        sequence: 0,
        last_step: 0,
    };
    let goal = GoalDescriptor::new(1, 2, 3);
    learner
        .observe(EpisodeEvent::Observation {
            run_scope: 1,
            episode_scope: 10,
            step_index: 0,
            state: 100,
            available_actions: vec![1],
            action_costs: vec![1],
            goal: goal.clone(),
            horizon_remaining: 4,
            previous_terminal: TerminalStatus::NotTerminated,
        })
        .unwrap();
    let error = learner.observe(EpisodeEvent::Observation {
        run_scope: 1,
        episode_scope: 11,
        step_index: 1,
        state: 101,
        available_actions: vec![1],
        action_costs: vec![1],
        goal,
        horizon_remaining: 3,
        previous_terminal: TerminalStatus::NotTerminated,
    });
    assert!(matches!(error, Err(LearnerError::InvalidScope)));
}
