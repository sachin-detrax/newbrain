# Next Task: Build the First Working Framework MVP

**Status:** MVP-8 complete for the bounded verification/package claim; Experiment A specification, protocol, and implementation complete; Experiment B specification next  
**Date:** 2026-10-12  
**Priority:** P0  
**Depends on:** Completed v1/v2 experiments and the frozen v2 protocol  
**Related:** [`remaining-tasks.md`](remaining-tasks.md), [`plan.md`](plan.md), [`specs/opaque-interactive-structural-discovery-v2.md`](specs/opaque-interactive-structural-discovery-v2.md), [`Experiment A specification`](specs/variable-length-boundary-discovery-experiment-a.md), [`Experiment A protocol`](protocols/variable-length-boundary-discovery-experiment-a-protocol-v1.0.md)

## 1. Goal

Turn the validated v2 experiment into the smallest reusable version of the
framework.

The MVP must allow a program to:

1. receive an opaque observation from an environment;
2. choose an action, make a prediction, or abstain;
3. receive the environment's result after the decision is recorded;
4. save immutable evidence and versioned learned procedures;
5. reuse a learned procedure after restarting the program;
6. revise or suspend a procedure when later evidence contradicts it; and
7. produce an inspectable report showing what was learned and why each action
   was chosen.

In one sentence:

> Build a small, restartable, environment-independent structural-learning
> runtime, then prove that it works in two different opaque toy environments.

This is an engineering MVP. It is not a claim of semantic understanding,
general intelligence, or human-like reasoning.

## 2. What already exists

The v2 experiment already provides most of the behavior needed inside one
benchmark run:

- opaque `StepInput`, `Decision`, and `StepResult` contracts;
- decision-before-result enforcement;
- append-only interaction evidence;
- run and episode scopes;
- bounded candidate grammar and admission gates;
- grounding and disagreement decisions;
- stale-result and changed-rule handling;
- resource accounting;
- freeze-before-final-evaluation behavior; and
- deterministic experiment reports.

The current limitation is architectural: these pieces are still organized
around the frozen v2 experiment. The evaluator world and experiment runner are
not yet cleanly separated from a reusable learner runtime, and learned state
cannot yet be restored by a new process.

## 3. MVP boundary

### Included

- Rust standard library only.
- Fixed three-atom opaque observation packets.
- A finite list of opaque actions supplied by the environment.
- Deterministic environments for the first implementation.
- Discovery, development, freeze, and final-evaluation phases.
- Evidence, hypotheses, procedures, dependencies, scopes, and revisions.
- Append-only durable storage using a versioned file format.
- One adapter for the existing v2 world.
- One materially different toy environment using the same public runtime.
- A command-line demo that trains, saves, restarts, loads, and evaluates.
- JSON/JSONL traces that a person can inspect without reading Rust code.

### Explicitly excluded from this MVP

- Raw text or speech input.
- Images, audio, or continuous sensors.
- Variable-length packet segmentation.
- Natural-language concept names inside the learner.
- Neural networks, embeddings, gradient descent, or dense tensor layers.
- General planning or search over long action sequences.
- Stochastic worlds.
- GPU acceleration or distributed execution.
- Claims about speed, energy use, or scale superiority.
- A production database, hosted service, or multi-user security model.

These exclusions keep the MVP small enough to finish and preserve the meaning
of the existing v1/v2 results.

## 4. Target architecture

Separate reusable framework code from benchmark-specific code.

### Reusable framework layer

The framework layer should own:

- the opaque protocol types;
- the learner lifecycle;
- the interaction ledger;
- evidence and hypothesis storage;
- candidate generation and admission;
- procedure versioning and invalidation;
- action selection and abstention;
- resource accounting;
- persistence and recovery; and
- generic run reports.

It must not import hidden labels, seeds, transition equations, episode names,
or scoring rules from a particular benchmark.

### Environment layer

An environment implementation should own:

- hidden state and transition behavior;
- the mapping from hidden values to opaque wire identifiers;
- episode initialization and terminal outcomes;
- the available actions; and
- evaluator-only scoring.

The environment may know the answer. The learner must not receive that hidden
information before its decision is recorded.

### Runner layer

The runner should own:

- phase sequencing;
- interaction between a learner and an environment;
- the freeze boundary;
- loading and saving state;
- report generation; and
- command-line options.

The current v2 experiment runner can remain as a benchmark package, but the
new MVP runner must call the same public runtime interfaces as any future
environment.

## 5. Ordered implementation tasks

### MVP-1 — Freeze the public runtime seams

**Priority:** P0  
**Depends on:** None

- Define a public `Environment` interface for reset, action application, and
  terminal results.
- Define the public learner interface for mutable learning, freezing, and
  frozen evaluation.
- Define the public run controller interface for discovery, development,
  freeze, and final evaluation.
- Keep the decision-before-result rule enforced by the ledger rather than by
  runner convention.
- Make protocol validation errors explicit and usable by callers.
- Add contract tests using a fake environment and fake learner.

**Acceptance:** A caller can implement a small environment without importing
`evaluator_world`, `experiment`, or any hidden-world type.

### MVP-2 — Extract the reusable learner runtime

**Priority:** P0  
**Status:** Complete (2026-10-12)  
**Depends on:** MVP-1

- Move or reorganize the reusable parts of the v2 modules behind the new
  framework API.
- Keep `opaque_protocol`, `interaction_ledger`, `grammar_admission`,
  `grounding_decision`, and `resource_scale` usable by the runtime.
- Keep evaluator-world generation and frozen benchmark schedules outside the
  framework layer.
- Preserve the existing v2 behavior through compatibility wrappers or clearly
  named benchmark adapters.
- Do not duplicate the grammar or decision policy just to create the MVP API.

**Implementation:** Reusable implementations now live in
[`src/framework`](../src/framework) and are re-exported through
[`src/runtime.rs`](../src/runtime.rs). The historical `src/v2` learner paths
remain compatibility shims. The benchmark-only `Evaluator` bridge is isolated
in [`src/v2/adapter.rs`](../src/v2/adapter.rs); evaluator-world generation,
frozen schedules, the experiment runner, B2, and scoring remain in the v2
benchmark modules. Boundary and compatibility tests are in
[`tests/mvp2_architecture.rs`](../tests/mvp2_architecture.rs).

**Acceptance:** The v2 experiment still compiles and runs using the extracted
runtime, and the existing v1 artifacts remain untouched.

### MVP-3 — Make learner stores first-class

**Priority:** P0  
**Status:** Complete (2026-10-12)  
**Depends on:** MVP-2

Introduce explicit learner-owned stores, even if the first implementation uses
simple in-memory collections:

- `EvidenceStore` for immutable observations, decisions, results, and sources;
- `HypothesisStore` for active, rejected, stale, and contradicted hypotheses;
- `ProcedureStore` for admitted procedures, versions, scopes, costs, and
  evidence references; and
- a dependency index from evidence to derived records.

The stores must provide:

- exact lookup by identifier and structural signature;
- scope checks before retrieval or reuse;
- deterministic ordering;
- immutable historical records;
- explicit stale/revalidation status; and
- no silent replacement of an old procedure version.

**Acceptance:** A contradiction creates a new status or procedure version while
the original evidence and old procedure record remain readable.

**Implementation:** [`src/framework/stores.rs`](../src/framework/stores.rs)
provides deterministic in-memory evidence, hypothesis, procedure, and
dependency stores, with aggregate registration through `LearnerStores`.
Coverage is in [`tests/mvp3_stores.rs`](../tests/mvp3_stores.rs). Persistence is
deferred to MVP-4.

### MVP-4 — Add durable save and restore

**Priority:** P0  
**Status:** Complete (2026-10-12)  
**Depends on:** MVP-3

- Define a versioned persistence format for run metadata, ledger events,
  evidence, hypotheses, procedures, dependencies, and resource counters.
- Write canonical JSON/JSONL using only the Rust standard library.
- Add strict loading validation for schema version, duplicate IDs, scope
  violations, malformed records, and incomplete writes.
- Use a temporary output followed by an atomic rename for a completed snapshot
  or event-log checkpoint.
- Restore a learner into mutable mode only when the saved run is explicitly
  marked mutable; restored frozen runs must remain frozen.
- Record a state fingerprint so save/load determinism can be checked.

**Acceptance:**

1. Process A learns a procedure and saves the state.
2. Process B loads the state without access to evaluator labels.
3. Process B correctly uses the procedure on a held-out context.
4. The restored trace contains the original evidence references.

**Implementation:** [`src/framework/persistence.rs`](../src/framework/persistence.rs)
provides versioned canonical JSON save/load through `FrameworkState`, strict
validation, atomic writes, deterministic fingerprints, and frozen-state
protection. [`tests/mvp4_persistence.rs`](../tests/mvp4_persistence.rs) covers
round trips, tampering, procedure history, and subprocess restoration.
Process-local physical memory and wall-clock samples are not restored; logical
resource state is persisted.

### MVP-5 — Build the generic run controller

**Priority:** P0  
**Status:** Complete (2026-10-12)  
**Depends on:** MVP-1 through MVP-4

Implement one controller that can execute the full lifecycle:

```text
create run
  -> discovery interactions
  -> development interactions
  -> admit or reject procedures
  -> freeze learner
  -> held-out evaluation
  -> save trace and report
```

The controller must:

- ask the learner for a decision before applying an environment action;
- close abstentions as `incomplete` without fabricating an observation;
- pass a result to the mutable learner only after recording the decision;
- never pass final results into frozen decision-making;
- enforce action, operation, candidate, and storage budgets; and
- classify failures as insufficient evidence, search exhaustion, unsupported
  grammar, incorrect hypothesis, or environment failure.

**Acceptance:** The same controller can run both the existing v2 adapter and a
second adapter without benchmark-specific branches in the controller.

**Implementation:** [`src/framework/controller.rs`](../src/framework/controller.rs)
provides `GenericRunController`, configurable episode budgets, typed failure
classification, and inspectable run/episode summaries. Coverage is in
[`tests/mvp5_controller.rs`](../tests/mvp5_controller.rs). The generic
controller intentionally leaves arbitrary learner serialization to the
application-specific persistence hook.

### MVP-6 — Add two environment adapters

**Priority:** P0  
**Depends on:** MVP-5

#### Adapter A: existing opaque interactive world

- [x] Wrap the current `EvaluatorWorld` behind the generic environment interface.
- [x] Preserve the current opaque mapping, episode scopes, terminal rules, and
  freeze boundary.

#### Adapter B: independent structural toy world

Create a second deterministic world with:

- [x] different hidden identifiers and transition logic;
- [x] a different episode schedule and seed;
- [x] at least one structural relation that is not solved by exact-example
  retrieval; and
- [x] held-out context or value combinations.

The second world may reuse the current fixed packet length and bounded grammar,
but it must not be a renamed copy of Adapter A. Its purpose is to test whether
the runtime is reusable rather than accidentally wired to the first benchmark.

The learner must receive only the public opaque protocol in both adapters. This
is covered by the ledger-aware `GroundingLearner` path; it returns/adopts the
receipt it owns rather than going through a duplicate `Learner::step` recording
path.

**Acceptance:** Complete for the narrow MVP-6 claim. The same
`GenericRunController` runs Adapter A and Adapter B with the real
`GroundingLearner`, and a focused end-to-end ledger-aware test retains,
promotes, and uses a structural procedure on a held-out Adapter B context.
There is no evaluator-label access in learner state or learner traces.

**Remaining seam:** controller orchestration remains generic, but candidate
generation/admission is still an explicit application-owned hook rather than
an automatic controller phase. That limitation is recorded rather than hidden;
the next open step is to specify such a hook if it is needed. MVP-7 CLI work is
intentionally not part of this implementation.

### MVP-7 — Add the MVP command-line demo

**Status:** Complete for the bounded CLI/artifact claim

**Priority:** P0  
**Depends on:** MVP-5 and MVP-6

Add a small CLI, for example:

```text
cargo.exe run --bin run_mvp -- \
  --environment structural-a \
  --store artifacts/mvp/state \
  --report artifacts/mvp/report.json
```

The exact flag names may change, but the CLI must support:

- selecting an environment adapter;
- creating or loading a run;
- running discovery/development;
- freezing and evaluating held-out cases;
- saving state and traces; and
- returning a nonzero exit code when an MVP gate fails.

The human-readable output should answer:

- What procedure was learned?
- Which evidence supported it?
- Which actions were selected?
- Where did the learner abstain?
- Was the procedure reused after restart?
- Did any later evidence make it stale or contradicted?
- Did the run stay within its budgets?

**Implementation:** [`src/bin/run_mvp.rs`](../src/bin/run_mvp.rs) provides the
documented flags and both adapter selections. It uses the generic controller
for discovery/development, calls the application-owned induction/admission
hook explicitly, persists and reloads `FrameworkState`, reinstalls admitted
procedure evidence, and evaluates through the frozen learner. Focused CLI
coverage is in [`tests/mvp7_cli.rs`](../tests/mvp7_cli.rs), and artifact
commands/layout are in [`artifacts/mvp/README.md`](../artifacts/mvp/README.md).

### MVP-8 — Verify the end-to-end claim

**Status:** Complete for the bounded MVP claim (2026-10-11)

**Priority:** P0  
**Depends on:** MVP-1 through MVP-7

Add meaningful tests for:

- protocol and environment contract compliance;
- decision-before-result ordering;
- scope isolation;
- persistence round-trip and fresh-process restore;
- frozen-state immutability;
- held-out structural generalization;
- abstention on unsupported or non-applicable cases;
- contradiction and procedure-version preservation;
- deterministic traces and fingerprints; and
- evaluator-label leakage.

Run the complete verification suite:

```text
cargo.exe fmt --all -- --check
cargo.exe check --all-targets
cargo.exe test --all-targets
cargo.exe clippy --all-targets -- -D warnings
cargo.exe run --bin run_mvp -- --environment structural-a --store artifacts/mvp/state --report artifacts/mvp/report.json
cargo.exe run --bin run_mvp -- --environment structural-b --store artifacts/mvp/state-b --report artifacts/mvp/report-b.json
```

The exact commands may be adjusted when the CLI is implemented, but the final
commands must be documented in `artifacts/mvp/README.md`.

**Implementation:** [`tests/mvp8_verification.rs`](../tests/mvp8_verification.rs)
aggregates gates M1–M12 using the public runtime/store APIs and the built
`run_mvp` binary. It checks stable v1 fingerprints, the reusable environment
boundary, receipt ordering, immutable/versioned evidence and procedures,
round-trip and fresh-process restore, held-out reuse, justified abstention,
contradiction preservation, shared-controller execution, report budgets and
leakage fields, deterministic reruns, and artifact validity. The exact
reproduction commands and gate-to-evidence table are in
[`artifacts/mvp/README.md`](../artifacts/mvp/README.md). Architecture
experiments A–C remain deliberately unimplemented.

## 6. MVP acceptance gates

The MVP is complete only when all gates pass:

| Gate | Requirement |
| --- | --- |
| A1 | Existing v1 artifacts and the recorded v1 result are unchanged. |
| A2 | The learner can be called through the public runtime API without importing a benchmark evaluator. |
| A3 | Every decision is recorded before its environment result is available. |
| A4 | Evidence, hypotheses, procedures, and revisions are immutable/versioned and scope-safe. |
| A5 | A learned procedure survives save, process restart, and restore. |
| A6 | The restored learner performs at least one held-out structural transfer. |
| A7 | Non-applicable or unsupported cases produce a justified abstention or explicitly marked fallback, not a fabricated confident prediction. |
| A8 | A contradiction makes dependent results stale, revised, or suspended without deleting history. |
| A9 | The same generic controller runs two environment adapters. |
| A10 | Reports include evidence references, procedure versions, budgets, failures, and deterministic fingerprints. |
| A11 | Three repeated runs produce identical logical traces and equivalent reports. |
| A12 | Formatting, compilation, tests, and clippy pass with no warnings. |

The final report should use a status such as:

```text
validated-mvp-narrow-claim
```

That status means the reusable runtime passed this MVP's gates. It does not
mean that the system understands language or is generally intelligent.

## 7. Suggested implementation order

```text
MVP-1 public seams
  -> MVP-2 runtime extraction
  -> MVP-3 learner stores
  -> MVP-4 persistence
  -> MVP-5 generic controller
  -> MVP-6 two adapters
  -> MVP-7 CLI
  -> MVP-8 verification and packaging
```

Do not start raw-text ingestion, planning, neural comparisons, or large-scale
optimization until this sequence passes. Those features would make it harder
to determine whether the basic framework itself works.

## 8. Completion deliverables

The implementation task must produce:

- reusable framework modules with documented public interfaces;
- a generic run controller;
- durable save/load support;
- two environment adapters;
- `run_mvp` or an equivalently documented CLI;
- unit and integration tests for all acceptance gates;
- `artifacts/mvp/README.md` with exact reproduction commands;
- an MVP report with raw traces, state fingerprints, budgets, and failures; and
- a short update to [`remaining-tasks.md`](remaining-tasks.md) marking
  this task's status and linking the artifacts.

## 9. Next decision after the MVP

The Experiment A specification/protocol step is complete and frozen in
[`docs/specs/variable-length-boundary-discovery-experiment-a.md`](specs/variable-length-boundary-discovery-experiment-a.md)
and [`docs/protocols/variable-length-boundary-discovery-experiment-a-protocol-v1.0.md`](protocols/variable-length-boundary-discovery-experiment-a-protocol-v1.0.md).
The Experiment A implementation step is complete with
`validated-a1-narrow-claim` and `validated-a3-comparator` recorded in
[`remaining-tasks.md`](remaining-tasks.md) T9 and
[`artifacts/experiment_a/README.md`](../artifacts/experiment_a/README.md).

Only after this MVP passes should the project choose the next capability to
add. The likely candidates are:

1. variable-length observations and boundary discovery;
2. a longer-horizon planning environment; or
3. a carefully scoped text-entry experiment.

That decision now has the frozen Experiment A specification and protocol linked
above. Experiment A implementation is the next step; it must not be added
informally to this MVP or used to reinterpret the frozen v1/v2 artifacts.
