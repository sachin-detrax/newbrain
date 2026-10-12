# Sparse Structural Rule Learner v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`/`- [ ]`) syntax for tracking.

**Goal:** Implement and evaluate the narrow v1 structural learner defined in `testable-design-v1.md`, proving or falsifying variable-based rule discovery, held-out symbol generalization, and retention after a second rule.

**Architecture:** Build a language-independent sparse transition engine as a small Rust library. Opaque symbols are exact IDs; candidates are explicit finite structures; learning is deterministic G0/G1 enumeration plus admission gates; prediction is partial and may abstain. The evaluator and final outputs remain outside learner state until each prediction is recorded.

**Tech Stack:** Rust stable, Cargo, Rust standard library only, CPU-only execution. No Python, PyTorch, TensorFlow, embeddings, tokenizer, gradient optimizer, neural framework, or external ML dependency.

**Spec:** `testable-design-v1.md`

## Completion status

The recorded v1 implementation and verification are complete according to the [execution progress ledger](../../../.superpowers/sdd/sparse-structural-rule-learner-v1/progress.md). The recorded result is `validated-v1-narrow-claim`; see the [v1 artifact summary](../../../artifacts/v1/README.md) and [remaining-task tracker](../../remaining-tasks.md). T0 reconciliation is complete, and the genuinely open work begins with T1 in the tracker.

## Global Constraints

- Fixed-length sequences contain exactly three opaque symbols.
- The learner receives exact identity and position, but no semantic labels or hidden evaluator state.
- The learner may generate only G0 exact candidates and G1 two-record anti-unification candidates.
- Candidates are partial predictors; no matching candidate produces explicit abstention.
- Final records are withheld until the learner is frozen and every final prediction is recorded.
- All budgets, candidate decisions, evidence references, and operation counts are logged.
- A timeout or budget exhaustion cannot be reported as a confident prediction.
- The primary experiment is CPU-only and must be deterministic across three reruns.
- Baseline B1 is an evaluator-only oracle reference and is excluded from learner measurements.
- Do not add implementation code until the user approves this plan.

## File map

The implementation should be split by responsibility:

- `Cargo.toml` — Rust package metadata and commands; no runtime dependencies.
- `src/lib.rs` — public module exports only.
- `src/types.rs` — opaque symbols, sequences, transitions, queries, responses, scopes, IDs, and evidence references.
- `src/budget.rs` — logical operation counters and hard budget exhaustion.
- `src/ledger.rs` — candidate, comparison, admission, prediction, and leakage records.
- `src/dataset.rs` — exact v1 training/development/final partitions and evaluator-only hidden outputs.
- `src/grammar.rs` — deterministic G0/G1 candidate enumeration and canonical signatures.
- `src/admission.rs` — Gates A–F and rejection reasons.
- `src/engine.rs` — learner state, Phase A/Phase B training, retrieval, and partial prediction.
- `src/baselines.rs` — B0 exact retrieval and A0 G0-only ablation behind the same predictor interface.
- `src/evaluator.rs` — final-query scoring, metrics, acceptance gates, and oracle B1 kept outside learner state.
- `src/report.rs` — deterministic human-readable and machine-readable run reports.
- `src/bin/run_v1.rs` — one command to run the frozen experiment and write artifacts.
- `tests/types_and_dataset.rs` — interface, partition, and leakage-boundary tests.
- `tests/grammar.rs` — exact G0/G1 candidate-generation tests.
- `tests/admission.rs` — Gate A–F tests and rejection evidence.
- `tests/engine.rs` — prediction, abstention, Phase B retention, and scope tests.
- `tests/budgets_and_determinism.rs` — budget enforcement and three-run reproducibility.
- `tests/evaluator.rs` — metrics, acceptance gates, and baseline comparisons.
- `artifacts/v1/.gitkeep` — output location for later raw traces and reports; no generated result is claimed before execution.

No existing source files are modified because the workspace currently contains only conceptual documents.

---

### Task 1: Create the Rust package and language-independent core types

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/types.rs`
- Test: `tests/types_and_dataset.rs`

**Interfaces:**
- Produces `Symbol`, `Action`, `Scope`, `RecordId`, `QueryId`, `ProcedureId`, `Sequence`, `Transition`, `Query`, `EvidenceRef`, `AbstentionReason`, and `Response`.
- Every type must be cloneable and comparable for deterministic tests.

- [x] **Step 1: Write failing type and interface tests**

```rust
#[test]
fn sequence_is_exactly_three_opaque_symbols() {
    let sequence = Sequence::new([7, 2, 7]);
    assert_eq!(sequence.positions(), [7, 2, 7]);
}

#[test]
fn prediction_and_abstention_are_distinct_responses() {
    let prediction = Response::prediction(
        Sequence::new([7, 3, 7]),
        ProcedureId(1),
        vec![EvidenceRef::record(RecordId(1))],
    );
    let abstention = Response::abstention(
        AbstentionReason::NoApplicableProcedure,
        vec![],
        vec![],
    );
    assert!(matches!(prediction, Response::Prediction { .. }));
    assert!(matches!(abstention, Response::Abstention { .. }));
}
```

- [x] **Step 2: Run the focused test and verify it fails because the package/types do not exist**

Run: `cargo test --test types_and_dataset sequence_is_exactly_three_opaque_symbols -- --nocapture`

Expected: FAIL with missing package or missing type definitions.

- [x] **Step 3: Implement the minimal core types**

Use exact integer IDs internally, not strings with semantic names. `Sequence::new` accepts `[Symbol; 3]`; `positions()` returns the same fixed array. `Response::Prediction` stores output, one or more procedure IDs, and evidence references. `Response::Abstention` stores a typed reason plus procedure/evidence references.

Reject no symbol value in v1; the core treats every symbol ID as opaque. Keep validation limited to the fixed sequence length so the implementation cannot accidentally add arbitrary-length behavior.

- [x] **Step 4: Run the focused test and then all current tests**

Run: `cargo test --test types_and_dataset -- --nocapture`

Expected: PASS for the new type tests; no implementation-specific semantic labels appear in the public types.

### Task 2: Encode the exact v1 partitions and evaluator boundary

**Files:**
- Modify: `src/dataset.rs`
- Modify: `tests/types_and_dataset.rs`

**Interfaces:**
- Produces `V1Dataset`, `R1Training`, `R1Development`, `R1Final`, `R2Training`, `R2Development`, and `R2Final` accessors.
- Produces `EvaluatorCase { query: Query, expected: Sequence, applicable: bool }` only to evaluator code; learner training APIs accept `Transition` without final-case access.

- [x] **Step 1: Write failing partition and leakage tests**

```rust
#[test]
fn primary_partitions_have_the_specified_records() {
    let data = V1Dataset::fixed();
    assert_eq!(data.r1_training().len(), 2);
    assert_eq!(data.r1_development().len(), 4);
    assert_eq!(data.r1_final().len(), 8);
    assert_eq!(data.r2_training().len(), 2);
    assert_eq!(data.r2_development().len(), 4);
    assert_eq!(data.r2_final().len(), 4);
}

#[test]
fn learner_inputs_do_not_contain_final_outputs() {
    let data = V1Dataset::fixed();
    let training_ids = data.all_training_and_development_ids();
    assert!(data.r1_final().iter().all(|case| !training_ids.contains(&case.query.query_id)));
}
```

- [x] **Step 2: Run the focused tests and verify they fail**

Run: `cargo test --test types_and_dataset primary_partitions_have_the_specified_records -- --nocapture`

Expected: FAIL because the dataset module is not implemented.

- [x] **Step 3: Implement the fixed dataset exactly**

Encode T1–T2, D1–D4, F1–F8, U1–U2, D5–D8, and V1–V4 from `testable-design-v1.md`. Keep evaluator expected outputs in `EvaluatorCase`; never pass those outputs to `Learner::predict` before the response is recorded. Mark applicability only in evaluator data.

Add a dataset fingerprint computed from record IDs and fields. The runner will print it so later runs cannot silently use a changed partition.

- [x] **Step 4: Add leakage-boundary assertions**

Assert that final query IDs and expected outputs never enter the learner's training input vectors. Use a test-only snapshot of learner state after each freeze; it may contain support IDs from training/development only.

- [x] **Step 5: Run all dataset tests**

Run: `cargo test --test types_and_dataset -- --nocapture`

Expected: PASS with exact counts and no final-case leakage.

### Task 3: Implement logical budgets and the execution ledger

**Files:**
- Create: `src/budget.rs`
- Create: `src/ledger.rs`
- Test: `tests/budgets_and_determinism.rs`

**Interfaces:**
- Produces `BudgetConfig`, `BudgetKind`, `BudgetState`, `BudgetError`, `CostVector`, and `Ledger`.
- `BudgetState::charge(kind, amount) -> Result<(), BudgetError>` must fail before a charge that would exceed a limit.
- `Ledger` records candidate IDs, support comparisons, development checks, operation steps, active-record peak, evidence references, response status, and rejection reasons.

- [x] **Step 1: Write failing budget tests**

```rust
#[test]
fn budget_rejects_the_charge_that_would_cross_the_limit() {
    let config = BudgetConfig::v1();
    let mut budget = BudgetState::new(config);
    budget.charge(BudgetKind::CandidatePairs, 16).unwrap();
    let error = budget.charge(BudgetKind::CandidatePairs, 1).unwrap_err();
    assert_eq!(error.kind(), BudgetKind::CandidatePairs);
}

#[test]
fn ledger_records_failed_candidates_and_costs() {
    let mut ledger = Ledger::new();
    ledger.record_rejection(ProcedureId(1), "development contradiction", CostVector::default());
    assert_eq!(ledger.rejected_candidates().len(), 1);
}
```

- [x] **Step 2: Run the focused tests and verify failure**

Run: `cargo test --test budgets_and_determinism budget_rejects_the_charge -- --nocapture`

Expected: FAIL because budget and ledger types are missing.

- [x] **Step 3: Implement the fixed v1 budget table**

Encode the exact limits from the spec:

```text
Per training update: candidate pairs 16, generated candidates 32,
support comparisons 128, development checks 32, operation steps 512,
active transitions 32, active candidates 64.

Per query: retrieved candidates 16, candidate checks 16,
operation steps 64, active records 16.

Total run: candidate records 128, event records 128,
ordinary-query full rebuilds 0, hidden-state accesses 0.
```

Make all charges explicit at call sites. No module may increment a counter directly without going through `BudgetState::charge`.

- [x] **Step 4: Implement deterministic ledger serialization**

Serialize records as stable text lines sorted by event sequence, not hash-map iteration. Include rejected candidates and budget failures. The ledger must make it possible to audit why a candidate was not admitted.

- [x] **Step 5: Run budget and ledger tests**

Run: `cargo test --test budgets_and_determinism -- --nocapture`

Expected: PASS, including tests that candidate generation stops at the exact budget boundary and reports `unsupported-by-budget` or `incomplete` rather than predicting.

### Task 4: Implement deterministic G0 and G1 candidate enumeration

**Files:**
- Create: `src/grammar.rs`
- Test: `tests/grammar.rs`

**Interfaces:**
- Produces `PatternAtom`, `InputPattern`, `OutputOp`, `CandidateKind`, `Candidate`, `CandidateSignature`, and `GrammarVersion`.
- `enumerate_g0(records, budget, ledger) -> Result<Vec<Candidate>, GenerationStatus>`.
- `enumerate_g1(records, budget, ledger) -> Result<Vec<Candidate>, GenerationStatus>`.
- Candidate signatures must implement total deterministic ordering and equality.

- [x] **Step 1: Write failing candidate tests**

```rust
#[test]
fn g1_ant_unifies_the_two_r1_training_records() {
    let data = V1Dataset::fixed();
    let candidates = enumerate_g1(&data.r1_training(), &mut test_budget(), &mut Ledger::new()).unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].signature(), "G1|a0|[X,m0,X]|[Copy(0),Emit(m1),Copy(2)]|W1");
}

#[test]
fn g0_keeps_exact_examples_separate() {
    let data = V1Dataset::fixed();
    let candidates = enumerate_g0(&data.r1_training(), &mut test_budget(), &mut Ledger::new()).unwrap();
    assert_eq!(candidates.len(), 2);
    assert!(candidates.iter().all(|candidate| candidate.is_exact()));
}
```

- [x] **Step 2: Run grammar tests and verify failure**

Run: `cargo test --test grammar -- --nocapture`

Expected: FAIL because G0/G1 are not implemented.

- [x] **Step 3: Implement G0 exact candidates**

For each record, create a fully constant input pattern and fully explicit output pattern. Charge one candidate and the required comparisons per record. Preserve the record ID as support evidence.

- [x] **Step 4: Implement G1 pair enumeration**

Enumerate ordered pairs by record ID, filter equal action and sequence length, and charge one candidate-pair budget unit before processing each pair. Apply only the spec's rules:

1. equal values across pair remain constants;
2. differing values become variables;
3. equal positions within every support record share a variable;
4. output positions become `Copy(i)` or `Emit(symbol)` only when stable;
5. reject unstable output positions.

Deduplicate by canonical signature while preserving the first support pair selected by canonical ordering. Do not inspect development or final records here.

- [x] **Step 5: Add grammar boundary tests**

Test that G1 rejects different action pairs, rejects unstable output transformations, does not create variables from one record, and stops at the candidate-pair budget. Test that no symbol category or semantic name appears in candidate data.

- [x] **Step 6: Run grammar tests**

Run: `cargo test --test grammar -- --nocapture`

Expected: PASS with exactly one G1 R1 candidate and one G1 R2 candidate for their respective training pairs.

### Task 5: Implement Gates A–F and evidence-bearing admission

**Files:**
- Create: `src/admission.rs`
- Test: `tests/admission.rs`

**Interfaces:**
- Produces `AdmissionGate`, `RejectionReason`, `AdmissionResult`, and `AdmittedProcedure`.
- `admit(candidate, training, development, budget, ledger) -> AdmissionResult`.
- `AdmittedProcedure` must expose `id()`, `scope()`, `signature()`, `support_refs()`, `challenge_refs()`, and `cost()`.

- [x] **Step 1: Write failing gate tests**

```rust
#[test]
fn r1_candidate_passes_all_gates() {
    let data = V1Dataset::fixed();
    let candidate = only_g1_r1_candidate(&data.r1_training());
    let result = admit(candidate, &data.r1_training(), &data.r1_development(), &mut test_budget(), &mut Ledger::new());
    assert!(result.is_admitted());
}

#[test]
fn candidate_without_two_variable_support_records_is_rejected() {
    let candidate = exact_candidate_from_t1();
    let result = admit(candidate, &[], &[], &mut test_budget(), &mut Ledger::new());
    assert_eq!(result.rejection_reason(), Some(RejectionReason::InsufficientSupport));
}
```

- [x] **Step 2: Run admission tests and verify failure**

Run: `cargo test --test admission -- --nocapture`

Expected: FAIL because admission types and logic are missing.

- [x] **Step 3: Implement Gates A–D**

Check support count, exact support consistency, matching-input contradictions in the supplied training set, and structural variation in a variable position. Record one ledger event per gate and preserve the first failing gate as the rejection reason.

- [x] **Step 4: Implement Gate E**

Evaluate every development record. Require prediction for applicable D1 and abstention for non-applicable D2–D4. A candidate must not be granted credit for emitting the identity output on a non-applicable record; it must not apply.

- [x] **Step 5: Implement Gate F and admission records**

Charge candidate checks and operation steps. Store the admitted procedure's exact grammar version, support references, development challenge references, tested status, scope, signature, and cost vector. Store rejected candidates with all failed gates.

- [x] **Step 6: Run admission tests**

Run: `cargo test --test admission -- --nocapture`

Expected: PASS for R1/R2 candidates and failure cases for missing support, contradictions, unstable outputs, development false positives, and budget exhaustion.

### Task 6: Implement the learner engine and continual-retention protocol

**Files:**
- Create: `src/engine.rs`
- Test: `tests/engine.rs`

**Interfaces:**
- Produces `Learner`, `LearnerSnapshot`, `Phase`, and `TrainingReport`.
- `Learner::new(config) -> Learner`.
- `Learner::train_phase_a(&mut self, r1_train, r1_dev) -> TrainingReport`.
- `Learner::train_phase_b(&mut self, r2_train, r2_dev) -> TrainingReport`.
- `Learner::freeze(&self) -> LearnerSnapshot`.
- `LearnerSnapshot::predict(&self, query) -> Response`.

- [x] **Step 1: Write failing engine tests**

```rust
#[test]
fn phase_a_predicts_unseen_r1_symbol_and_abstains_on_non_applicable_cases() {
    let data = V1Dataset::fixed();
    let mut learner = Learner::new(LearnerConfig::v1());
    learner.train_phase_a(&data.r1_training(), &data.r1_development()).unwrap();
    let snapshot = learner.freeze();
    assert!(matches!(snapshot.predict(&query_for_f1()), Response::Prediction { .. }));
    assert!(matches!(snapshot.predict(&query_for_f6()), Response::Abstention { .. }));
}

#[test]
fn phase_b_retains_r1_while_learning_r2() {
    let data = V1Dataset::fixed();
    let mut learner = Learner::new(LearnerConfig::v1());
    learner.train_phase_a(&data.r1_training(), &data.r1_development()).unwrap();
    learner.train_phase_b(&data.r2_training(), &data.r2_development()).unwrap();
    let snapshot = learner.freeze();
    assert!(matches!(snapshot.predict(&query_for_f1()), Response::Prediction { .. }));
    assert!(matches!(snapshot.predict(&query_for_v1()), Response::Prediction { .. }));
}
```

- [x] **Step 2: Run engine tests and verify failure**

Run: `cargo test --test engine -- --nocapture`

Expected: FAIL because the learner engine is not implemented.

- [x] **Step 3: Implement Phase A**

Ingest only R1 training records, enumerate G0/G1, admit candidates using the fixed R1 development set, and freeze a snapshot before final records can be referenced. Keep rejected candidates and all evidence in the ledger.

- [x] **Step 4: Implement retrieval and partial prediction**

Retrieve by exact scope, action, length, and candidate signature. Evaluate admitted procedures in canonical order. Return a prediction only for one applicable output or multiple identical applicable outputs. Return conflict abstention for differing applicable outputs and no-applicable abstention otherwise.

- [x] **Step 5: Implement Phase B without destructive updates**

Ingest U1/U2, generate and admit R2 candidates using D5–D8, and retain all R1 procedures and evidence unchanged. Use versioned procedure IDs so R1 and R2 cannot collide despite similar structure.

- [x] **Step 6: Enforce freeze and leakage boundaries**

After `freeze`, the snapshot must not expose mutating training methods or final expected outputs. The runner must require a prediction record before evaluator scoring can be called.

- [x] **Step 7: Run engine tests**

Run: `cargo test --test engine -- --nocapture`

Expected: PASS for R1 prediction, R1 abstention, R2 prediction, scope separation, conflict abstention, and frozen-state behavior.

### Task 7: Implement B0, A0, evaluator metrics, and acceptance gates

**Files:**
- Create: `src/baselines.rs`
- Create: `src/evaluator.rs`
- Test: `tests/evaluator.rs`

**Interfaces:**
- Produces `Predictor` trait, `ExactRetrievalBaseline`, `G0OnlyAblation`, `OracleReference`, `CaseResult`, `Metrics`, and `AcceptanceReport`.
- `Predictor::fit(...)` may consume only training/development partitions.
- `Predictor::predict(query) -> Response` must use the same response interface as the main learner.
- `evaluate(predictor, final_cases) -> EvaluationReport` must score only after recording each response.

- [x] **Step 1: Write failing evaluator tests**

```rust
#[test]
fn exact_retrieval_has_zero_primary_r1_coverage_on_unseen_symbols() {
    let data = V1Dataset::fixed();
    let mut baseline = ExactRetrievalBaseline::new();
    baseline.fit(&data.r1_training(), &data.r1_development()).unwrap();
    let report = evaluate(&mut baseline, &data.r1_final());
    assert_eq!(report.metrics().applicable_coverage(), 0.0);
}

#[test]
fn acceptance_report_fails_when_any_gate_fails() {
    let report = AcceptanceReport::from_metrics(metrics_with_false_positive());
    assert!(!report.passed());
    assert!(report.failed_gates().contains(&Gate::FalsePositiveControl));
}
```

- [x] **Step 2: Run evaluator tests and verify failure**

Run: `cargo test --test evaluator -- --nocapture`

Expected: FAIL because baselines, metrics, and gate evaluation are missing.

- [x] **Step 3: Implement B0 and A0**

B0 stores exact training input/action/output records and abstains on any unseen pair. A0 uses the same learner pipeline but disables G1. Both must emit evidence references and costs through the common ledger.

- [x] **Step 4: Implement B1 outside learner state**

Implement an evaluator-only fixed rule for R1/R2 to validate the dataset and metrics. Do not make it available through the `Learner` interface or include its hidden rule lookup in learner operation counts.

- [x] **Step 5: Implement scoring semantics**

For applicable cases, coverage means a `Prediction` response and accuracy means exact output match. For non-applicable cases, any `Prediction` is a false positive even if the output equals identity. Record abstention reason, procedure IDs, evidence IDs, and per-case cost.

- [x] **Step 6: Implement the ten pre-registered gates**

Check deterministic reruns, no leakage, exact 5/5 R1 coverage and accuracy, zero F6–F8 predictions, retention drop ≤5 percentage points, exact 2/2 R2 coverage and accuracy, zero V3–V4 predictions, budget compliance, and evidence traceability.

- [x] **Step 7: Run evaluator tests**

Run: `cargo test --test evaluator -- --nocapture`

Expected: PASS for B0 metrics, failed-gate reporting, and oracle reference isolation.

### Task 8: Add deterministic run reports and the v1 command

**Files:**
- Create: `src/report.rs`
- Create: `src/bin/run_v1.rs`
- Create: `artifacts/v1/.gitkeep`
- Test: `tests/budgets_and_determinism.rs`

**Interfaces:**
- Produces `RunReport` and `write_report(path, report)`.
- Binary command: `cargo run --bin run_v1 -- --out artifacts/v1/run-<id>`.
- Report must contain dataset fingerprint, grammar version, configuration fingerprint, raw ledger lines, per-case results, metrics, acceptance gates, and environment information.

- [x] **Step 1: Write failing reproducibility tests**

```rust
#[test]
fn three_runs_have_identical_candidate_and_admission_logs() {
    let runs = [run_in_memory(), run_in_memory(), run_in_memory()];
    assert_eq!(runs[0].ledger_text(), runs[1].ledger_text());
    assert_eq!(runs[1].ledger_text(), runs[2].ledger_text());
    assert_eq!(runs[0].acceptance_report(), runs[2].acceptance_report());
}
```

- [x] **Step 2: Run the reproducibility test and verify failure**

Run: `cargo test --test budgets_and_determinism three_runs_have_identical_candidate_and_admission_logs -- --nocapture`

Expected: FAIL because the runner/report path is not implemented.

- [x] **Step 3: Implement stable report serialization**

Use stable record ordering and plain text/TSV or deterministic JSON-like output produced without a serialization dependency. Include all rejected candidates, budget failures, and abstentions. Never include evaluator-only hidden rules in learner trace output.

- [x] **Step 4: Implement the v1 runner**

Run B0, A0, main learner Phase A, main learner Phase B, and B1 in that order. Freeze before final prediction. Write reports only after every final response has been recorded. Return a nonzero process status if the primary learner fails an acceptance gate, while still writing the report.

- [x] **Step 5: Run the command and inspect artifacts**

Run: `cargo run --bin run_v1 -- --out artifacts/v1/run-primary`

Expected before implementation: no claim; after implementation: deterministic report directory with raw traces, metrics, and gate outcomes. The first run's outcome must be reported as evidence, whether pass or fail.

### Task 9: Add adversarial extensions without changing the primary test

**Files:**
- Modify: `src/dataset.rs`
- Modify: `src/bin/run_v1.rs`
- Create: `tests/adversarial.rs`

**Interfaces:**
- Produces separate extension dataset constructors and `AdversarialReport`.
- Primary `V1Dataset::fixed()` and primary acceptance gates remain unchanged.

- [x] **Step 1: Write failing adversarial tests**

```rust
#[test]
fn symbol_permutation_preserves_structural_behavior() {
    let report = run_symbol_permutation_extension();
    assert!(report.main_learner_retains_rule());
}

#[test]
fn missing_development_challenge_is_reported_as_under_testing() {
    let report = run_removed_challenge_extension();
    assert!(report.contains_warning("development challenge removed"));
}
```

- [x] **Step 2: Run adversarial tests and verify failure**

Run: `cargo test --test adversarial -- --nocapture`

Expected: FAIL because extension generators and reports do not exist.

- [x] **Step 3: Implement extensions as separate runs**

Add symbol permutation, record reorder, irrelevant records, same-source duplication, separate-scope contradiction, removed development challenge, corrupted output, exhausted query budget, larger symbol set, and multiple-rule extensions. Each report must name the tested property and must not modify the primary dataset or learner grammar.

- [x] **Step 4: Run adversarial tests**

Run: `cargo test --test adversarial -- --nocapture`

Expected: PASS for extension reporting. An extension failure is reported as an extension result and does not retroactively change the primary v1 acceptance result.

### Task 10: Run the complete verification suite and package the research result

**Files:**
- Modify: `README.md` if one is created during setup
- Create: `artifacts/v1/README.md`
- Test: all test files

- [x] **Step 1: Run formatter and compilation checks**

Run:

```text
cargo fmt --all -- --check
cargo check --all-targets
```

Expected: PASS with no formatting or compilation errors.

- [x] **Step 2: Run the complete test suite**

Run: `cargo test --all-targets -- --nocapture`

Expected: PASS for unit, leakage, grammar, admission, engine, budget, evaluator, determinism, and adversarial tests.

- [x] **Step 3: Run the primary experiment three times**

Run the v1 command three times with separate output directories. Compare dataset fingerprints, candidate logs, admission logs, per-case responses, metrics, and acceptance reports byte-for-byte where deterministic.

- [x] **Step 4: Write the result README**

`artifacts/v1/README.md` must state:

- exact command used;
- Rust version and host information;
- dataset and grammar fingerprints;
- whether each acceptance gate passed;
- baseline and ablation results;
- operation counts, memory, and wall time;
- all failures and abstentions;
- what the result does not establish.

Do not write “successful brain,” “superintelligence,” “unlimited memory,” or “faster than deep learning” unless a later experiment separately proves those claims.

- [x] **Step 5: Report the research decision**

Choose exactly one status:

```text
validated-v1-narrow-claim
partially-validated-with-failures
falsified-by-held-out-generalization
falsified-by-retention
falsified-by-leakage
falsified-by-budget
unsupported-by-grammar
```

Include the evidence path to the report and the next design decision. Do not advance to text or multimodal work unless the primary result and limitations are reviewed.

## Plan self-review

### Spec coverage

- Opaque fixed-length interface: Tasks 1–2.
- Exact R1/R2 partitions and evaluator boundary: Task 2.
- G0/G1 finite grammar: Task 4.
- Gates A–F and evidence history: Task 5.
- Partial prediction and abstention: Task 6.
- Phase A/Phase B retention: Task 6.
- Fixed budgets and CPU-only execution: Tasks 3 and 8.
- B0, B1, A0, A1: Task 7.
- Metrics and ten acceptance gates: Task 7.
- Adversarial extensions: Task 9.
- Raw traces and deterministic reports: Task 8.
- Final verification and result classification: Task 10.

### Placeholder scan

The plan contains no unresolved implementation choices in the primary v1 experiment. It intentionally leaves the permanent production language, database, hardware runtime, text interface, and assistant architecture outside scope because the spec excludes them.

### Type consistency

- `Sequence`, `Transition`, `Query`, and `Response` originate in `src/types.rs` and are consumed consistently by dataset, grammar, engine, baseline, and evaluator tasks.
- `BudgetState` and `Ledger` are passed explicitly to grammar and admission functions.
- `AdmittedProcedure` is retrieved by `LearnerSnapshot::predict` and scored through the common `Response` interface.
- `Predictor` is implemented by B0, A0, and B1 and consumed by `evaluate`.
- Final expected outputs exist only in evaluator cases and are accessed after response recording.

No implementation code is included in this plan. The workspace is not currently a git repository, so no commit steps are prescribed; checkpoints should be reviewed through the plan and test artifacts instead.
