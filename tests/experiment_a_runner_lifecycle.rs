use sparse_structural_rule_learner::experiment_a::generator::{generate_run, SymbolMode};
use sparse_structural_rule_learner::experiment_a::learner::{
    ExperimentALearner, FrozenExperimentALearner, LearnerError,
};
use sparse_structural_rule_learner::experiment_a::protocol::{
    AnalysisDecision, AnalysisReason, AnalysisResponse, StreamEvent, GRAMMAR_VERSION,
    PRIMARY_SCOPE_SEED, PRIMARY_WORLD_SEED, TRAINING_PARTITION_TAG,
};
use sparse_structural_rule_learner::experiment_a::runner::drive_learner;

const DEV_TAG: u64 = 0x4445_5600_0000_0001;
const FINAL_TAG: u64 = 0x4649_4e00_0000_0001;

struct LifecycleProbe {
    scope: u64,
    analyses: usize,
    development_at: Option<usize>,
}

struct ProbeFrozen {
    scope: u64,
    fingerprint: String,
    analyses: usize,
}

impl ExperimentALearner for LifecycleProbe {
    type Frozen = ProbeFrozen;

    fn ingest(&mut self, event: StreamEvent) -> Result<(), LearnerError> {
        match event {
            StreamEvent::Token { stream_scope, .. } | StreamEvent::End { stream_scope, .. } => {
                self.scope = stream_scope;
                Ok(())
            }
        }
    }

    fn analyze(&mut self) -> Result<AnalysisDecision, LearnerError> {
        self.analyses += 1;
        Ok(AnalysisDecision {
            stream_scope: self.scope,
            response: AnalysisResponse::Abstain {
                reason: AnalysisReason::InsufficientEvidence,
                hypothesis_refs: Vec::new(),
                evidence_refs: Vec::new(),
            },
            grammar_version: GRAMMAR_VERSION.to_owned(),
            model_fingerprint: "probe".to_owned(),
            evidence_refs: Vec::new(),
            operation_cost: 1,
            sequence: self.analyses as u64,
        })
    }

    fn begin_development(&mut self) -> Result<(), LearnerError> {
        self.development_at = Some(self.analyses);
        Ok(())
    }

    fn freeze(self) -> Result<Self::Frozen, LearnerError> {
        Ok(ProbeFrozen {
            scope: self.scope,
            fingerprint: "probe-frozen".to_owned(),
            analyses: self.analyses,
        })
    }
}

impl FrozenExperimentALearner for ProbeFrozen {
    fn ingest(&mut self, event: StreamEvent) -> Result<(), LearnerError> {
        match event {
            StreamEvent::Token { stream_scope, .. } | StreamEvent::End { stream_scope, .. } => {
                self.scope = stream_scope;
                Ok(())
            }
        }
    }

    fn analyze(&mut self) -> Result<AnalysisDecision, LearnerError> {
        self.analyses += 1;
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
            operation_cost: 1,
            sequence: self.analyses as u64,
        })
    }

    fn model_fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

#[test]
fn runner_delivers_160_analyses_with_registered_phase_boundaries() {
    let run = generate_run(
        PRIMARY_WORLD_SEED,
        PRIMARY_SCOPE_SEED,
        TRAINING_PARTITION_TAG,
        DEV_TAG,
        FINAL_TAG,
        SymbolMode::Primary,
    );
    let learner = LifecycleProbe {
        scope: 0,
        analyses: 0,
        development_at: None,
    };
    let (trace, _) = drive_learner(&run.records, learner).unwrap();
    assert_eq!(trace.outcomes.len(), 160);
    assert_eq!(trace.development_called_after, 64);
    assert_eq!(trace.frozen_after, 96);
    for (index, outcome) in trace.outcomes.iter().enumerate() {
        assert_eq!(outcome.global_ordinal, (index + 1) as u64);
        assert_eq!(outcome.decision.sequence, (index + 1) as u64);
    }
}

#[test]
fn frozen_fingerprint_is_stable_across_final() {
    let run = generate_run(
        PRIMARY_WORLD_SEED,
        PRIMARY_SCOPE_SEED,
        TRAINING_PARTITION_TAG,
        DEV_TAG,
        FINAL_TAG,
        SymbolMode::Primary,
    );
    let learner = LifecycleProbe {
        scope: 0,
        analyses: 0,
        development_at: None,
    };
    let (trace, _) = drive_learner(&run.records, learner).unwrap();
    assert_eq!(trace.fingerprint_before_final, "probe-frozen");
    assert_eq!(
        trace.fingerprint_before_final,
        trace.fingerprint_after_final
    );
}

#[test]
fn runner_counts_every_received_atom() {
    let run = generate_run(
        PRIMARY_WORLD_SEED,
        PRIMARY_SCOPE_SEED,
        TRAINING_PARTITION_TAG,
        DEV_TAG,
        FINAL_TAG,
        SymbolMode::Primary,
    );
    let expected: usize = run.records.iter().map(|r| r.wire_tokens.len()).sum();
    let learner = LifecycleProbe {
        scope: 0,
        analyses: 0,
        development_at: None,
    };
    let (trace, _) = drive_learner(&run.records, learner).unwrap();
    assert_eq!(trace.received_atoms_run, expected as u64);
    assert_eq!(trace.analysis_operation_cost, 160);
}
