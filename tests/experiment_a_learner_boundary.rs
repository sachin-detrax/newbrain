use sparse_structural_rule_learner::experiment_a::learner::{
    ExperimentALearner, FrozenExperimentALearner, LearnerError,
};
use sparse_structural_rule_learner::experiment_a::protocol::{
    AnalysisDecision, AnalysisReason, AnalysisResponse, StreamEvent, GRAMMAR_VERSION,
};

struct UnitLearner {
    scope: u64,
    sequence: u64,
}

struct UnitFrozen {
    scope: u64,
    fingerprint: String,
}

impl ExperimentALearner for UnitLearner {
    type Frozen = UnitFrozen;

    fn ingest(&mut self, event: StreamEvent) -> Result<(), LearnerError> {
        match event {
            StreamEvent::Token { stream_scope, .. } | StreamEvent::End { stream_scope, .. } => {
                if self.sequence == 0 {
                    self.scope = stream_scope;
                }
                if stream_scope != self.scope {
                    return Err(LearnerError::InvalidScope);
                }
                Ok(())
            }
        }
    }

    fn analyze(&mut self) -> Result<AnalysisDecision, LearnerError> {
        self.sequence += 1;
        Ok(AnalysisDecision {
            stream_scope: self.scope,
            response: AnalysisResponse::Abstain {
                reason: AnalysisReason::InsufficientEvidence,
                hypothesis_refs: Vec::new(),
                evidence_refs: Vec::new(),
            },
            grammar_version: GRAMMAR_VERSION.to_owned(),
            model_fingerprint: "unit-learner-v1".to_owned(),
            evidence_refs: Vec::new(),
            operation_cost: 0,
            sequence: self.sequence,
        })
    }

    fn begin_development(&mut self) -> Result<(), LearnerError> {
        Ok(())
    }

    fn freeze(self) -> Result<Self::Frozen, LearnerError> {
        Ok(UnitFrozen {
            scope: self.scope,
            fingerprint: "unit-learner-v1".to_owned(),
        })
    }
}

impl FrozenExperimentALearner for UnitFrozen {
    fn ingest(&mut self, event: StreamEvent) -> Result<(), LearnerError> {
        match event {
            StreamEvent::Token { stream_scope, .. } | StreamEvent::End { stream_scope, .. } => {
                if stream_scope != self.scope && self.scope != 0 {
                    return Err(LearnerError::InvalidScope);
                }
                self.scope = stream_scope;
                Ok(())
            }
        }
    }

    fn analyze(&mut self) -> Result<AnalysisDecision, LearnerError> {
        Ok(AnalysisDecision {
            stream_scope: self.scope,
            response: AnalysisResponse::Abstain {
                reason: AnalysisReason::InsufficientEvidence,
                hypothesis_refs: Vec::new(),
                evidence_refs: Vec::new(),
            },
            grammar_version: GRAMMAR_VERSION.to_owned(),
            model_fingerprint: self.fingerprint.clone(),
            evidence_refs: Vec::new(),
            operation_cost: 0,
            sequence: 0,
        })
    }

    fn model_fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

#[test]
fn unit_learner_speaks_only_the_opaque_boundary() {
    let mut learner = UnitLearner {
        scope: 0,
        sequence: 0,
    };
    assert!(learner.begin_development().is_ok());
    learner
        .ingest(StreamEvent::Token {
            run_scope: 7,
            stream_scope: 9,
            atom: 42,
        })
        .unwrap();
    learner
        .ingest(StreamEvent::End {
            run_scope: 7,
            stream_scope: 9,
        })
        .unwrap();
    let decision = learner.analyze().unwrap();
    assert_eq!(decision.stream_scope, 9);
    assert_eq!(decision.grammar_version, GRAMMAR_VERSION);
    assert_eq!(decision.evidence_refs, Vec::<u64>::new());
    let frozen = learner.freeze().unwrap();
    assert_eq!(frozen.model_fingerprint(), "unit-learner-v1");
}
