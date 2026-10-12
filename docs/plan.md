# Framework Evolution Build Plan

**Status:** MVP-8 bounded verification/package complete; Experiment A specification, protocol, and implementation complete with validated A1/A3 results; Experiment B specification next  
**Date:** 2026-10-12  
**Priority:** P0  
**Depends on:** Completed v1/v2 experiments and the architecture-gap review  
**Primary references:** [`task.md`](task.md), [`research/architecture-gap-research-2026-10-11.md`](research/architecture-gap-research-2026-10-11.md), [`Experiment A specification`](specs/variable-length-boundary-discovery-experiment-a.md), [`Experiment A protocol`](protocols/variable-length-boundary-discovery-experiment-a-protocol-v1.0.md)

## 1. Decision

Preserve the current v1/v2 learner as an auditable structural-learning kernel,
but do not treat its current `G0/G1` or `E0`–`E4` grammar as the final learning
mechanism.

Build the next version in two stages:

1. **Reusable MVP:** turn the validated kernel into a restartable,
   environment-independent runtime and prove it in two different deterministic
   environments.
2. **Architecture decision experiments:** test representation discovery,
   transition/world modeling and planning, and primitive/library learning before
   choosing the larger post-MVP architecture.

The project should not add raw text, long-horizon planning, stochasticity, or a
neural front end to the MVP. Those capabilities require separate experiments
and protocols because they introduce different representations, objectives,
search spaces, and failure modes.

## 2. What this plan is trying to build

The immediate target is a **loadable structural-learning runtime**, not a
general conversational model or complete artificial brain.

At MVP completion, a program should be able to:

```text
load learner state
  -> receive an opaque observation and available actions
  -> select an action, predict a result, or abstain
  -> record the decision before the environment result is revealed
  -> ingest the observed result when mutable
  -> update evidence, hypotheses, and procedures
  -> save the new versioned state
```

The output is initially structured:

```text
Act {
  action,
  optional_prediction,
  procedure_refs,
  evidence_refs,
  hypothesis_refs
}
```

or:

```text
Abstain {
  reason,
  procedure_refs,
  evidence_refs,
  hypothesis_refs
}
```

Natural-language answers, raw-text learning, broad world knowledge, and
general-purpose planning are later capabilities, not implicit consequences of
this MVP.

## 3. Starting point: what is already validated

The current implementation and artifacts support these narrow claims:

- opaque fixed-packet input and action protocols;
- decision-before-result interaction ordering;
- bounded structural induction over repeated equality roles;
- evidence, hypothesis, candidate, procedure, and revision references;
- applicability checks and explicit abstention;
- run and episode scope confinement;
- freeze-before-final-evaluation behavior;
- changed-rule and stale-result seams;
- deterministic budgets, traces, reruns, and artifact hashes; and
- narrow held-out transfer and retention in tiny deterministic worlds.

The recorded statuses are:

```text
validated-v1-narrow-claim
validated-v2-narrow-claim
```

These results validate a bounded structural learner. They do not validate
concept discovery, a general world model, long-horizon planning, raw-text
understanding, or lifelong learning across process restarts.

## 4. Target architecture

The framework should be organized into separable layers.

```text
environment or raw stream
        |
        v
representation and segmentation layer
        |
        v
typed observations and uncertain hypotheses
        |
        +-------------------------------+
        |                               |
        v                               v
evidence/dependency stores       procedure/library induction
        |                               |
        +---------------+---------------+
                        v
              transition/world model
                        |
                        v
                bounded planner
                        |
                        v
          decision/action/abstention policy
                        |
                        v
              observed result and revision
```

The current v1/v2 implementation occupies mainly the evidence, procedure,
decision, and revision portions. The representation, library-learning,
world-model, and planner layers are future work.

### 4.1 Reusable framework layer

The reusable layer owns:

- opaque protocol types;
- learner lifecycle and freeze state;
- append-only interaction ledger;
- evidence, hypothesis, and procedure stores;
- dependency tracking and revalidation;
- bounded candidate generation and admission;
- procedure versioning;
- abstention and uncertainty reasons;
- resource accounting;
- persistence and restoration; and
- generic reports.

It must not import evaluator-only labels, hidden state, transition tables,
benchmark schedules, or benchmark scoring logic.

### 4.2 Environment layer

An environment owns:

- hidden state;
- hidden transition behavior;
- opaque wire mappings;
- available actions;
- episode initialization;
- returned observations;
- terminal outcomes; and
- evaluator-only scoring.

The environment may know the answer. It must not expose that answer before the
learner's decision is recorded.

### 4.3 Future extension layers

These layers are not part of the MVP, but their interfaces should not be made
impossible by the MVP:

- representation and segmentation;
- uncertain symbolic hypotheses;
- primitive and procedure library learning;
- symbolic or learned transition models;
- goal and preference representation;
- bounded planning and replanning; and
- hybrid learned front ends.

## 5. Invariants that must survive every version

The following are architectural requirements, not optional implementation
details:

1. **Observed, inferred, imagined, and evaluator-only information remain
   distinct.** A simulated rollout cannot become an observed fact.
2. **Procedures remain evidence-linked and versioned.** New evidence may revise,
   narrow, suspend, or replace a procedure, but cannot silently rewrite history.
3. **Abstention remains first-class.** Unsupported grammar, insufficient
   evidence, conflict, and budget exhaustion must remain distinguishable.
4. **Search remains bounded and measurable.** Candidate count, operation cost,
   storage cost, and unresolved work must be reported.
5. **Scopes remain explicit.** Episode, run, hypothetical, environment, and
   representation scopes must not be mixed accidentally.
6. **The learner/environment boundary remains auditable.** Hidden labels,
   seeds, transition equations, and final answers cannot enter learner state.
7. **Every broader capability gets a new protocol.** A planning, stochastic,
   variable-length, text, or hybrid result cannot be reported as an extension
   of the fixed v1/v2 claim without a new evaluation design.

## 6. Phase 0 — Architecture checkpoint

**Goal:** establish the exact boundary before implementation begins.

### Tasks

- [ ] Keep v1 datasets, grammar versions, artifacts, and historical reports
  immutable.
- [ ] Record that the MVP is a reusable structural-learning substrate, not the
  complete brain-machine architecture.
- [ ] Freeze the public `Environment`, mutable learner, frozen learner, and run
  controller seams.
- [ ] Decide the persistence schema version and state fingerprint strategy.
- [ ] Define which fields can be learner-visible and which remain
  evaluator-only.
- [ ] Define the second environment before implementing its adapter.
- [ ] Keep raw text, stochasticity, long-horizon planning, and neural front ends
  outside the MVP scope.

### Gate 0

Do not begin broad capability work until the architecture boundary is recorded
and the second environment has an independently specified task family.

## 7. Phase 1 — Reusable runtime MVP

### MVP-1 — Public runtime seams

**Status:** Complete (2026-10-12)

**Goal:** make the learner usable without importing a benchmark evaluator.

### Tasks

- [x] Define an environment interface for reset, observation delivery, action
  application, and terminal results.
- [x] Define mutable learner operations for `step` and result ingestion.
- [x] Define frozen learner operations that cannot mutate procedures or policy.
- [x] Define run-controller phases: discovery, development, freeze, and final
  evaluation.
- [x] Keep decision-before-result enforcement inside the ledger.
- [x] Add contract tests with a fake learner and fake environment.

### Acceptance

A caller can implement a small environment without importing
`evaluator_world`, `experiment`, hidden labels, or benchmark schedule types.

**Implementation:** [`src/runtime.rs`](../src/runtime.rs) provides the public
`Environment`, `RunController`, and lifecycle seams. Receipt-backed
`EnvironmentTurn` handling is implemented in
[`src/framework/interaction_ledger.rs`](../src/framework/interaction_ledger.rs), and the
contract coverage is in [`tests/runtime_contract.rs`](../tests/runtime_contract.rs).
The existing v2 evaluator remains compatible through the environment bridge.

**Verification:** `cargo.exe fmt --all -- --check`,
`cargo.exe check --all-targets`, `cargo.exe test --test runtime_contract`,
`cargo.exe test --all-targets`, and
`cargo.exe clippy --all-targets -- -D warnings` passed on 2026-10-12;
92 tests passed with no clippy warnings.

### MVP-2 — Extract the learner runtime

**Status:** Complete (2026-10-12)

**Goal:** separate reusable learner behavior from the v2 benchmark.

### Tasks

- [x] Reorganize `opaque_protocol`, `interaction_ledger`,
  `grammar_admission`, `grounding_decision`, and `resource_scale` behind the
  public runtime API.
- [x] Keep evaluator-world generation and frozen schedules in a benchmark
  adapter.
- [x] Preserve v2 behavior with compatibility wrappers where necessary.
- [x] Avoid duplicating the grammar or decision policy.
- [x] Add module-level documentation describing the learner/evaluator boundary.

### Acceptance

The current v2 experiment still builds and produces the same narrow result,
while the generic runtime can be called without v2 evaluator types.

**Implementation:** The reusable implementations are in
[`src/framework`](../src/framework) and are re-exported through
[`src/runtime.rs`](../src/runtime.rs). Historical `src/v2` learner module paths
remain compatibility shims. The benchmark-only `Evaluator` bridge is isolated
in [`src/v2/adapter.rs`](../src/v2/adapter.rs); the benchmark environment
adapter in [`src/v2/evaluator_world.rs`](../src/v2/evaluator_world.rs) owns
hidden world data and the frozen schedule, while
[`src/v2/experiment.rs`](../src/v2/experiment.rs) remains the runner for B2
and benchmark scoring. Focused boundary and compatibility coverage
is in [`tests/mvp2_architecture.rs`](../tests/mvp2_architecture.rs).

### MVP-3 — First-class learner stores

**Status:** Complete (2026-10-12)

**Goal:** make durable knowledge explicit instead of embedding it in the
benchmark runner.

### Stores

- `EvidenceStore`: immutable observations, decisions, results, sources, and
  provenance.
- `HypothesisStore`: active, rejected, stale, contradicted, and unresolved
  hypotheses.
- `ProcedureStore`: admitted procedures, versions, scopes, costs, and evidence
  references.
- `DependencyIndex`: links from evidence and assumptions to derived results and
  procedures.

### Required behavior

- [x] Exact lookup by identifier and structural signature.
- [x] Deterministic ordering.
- [x] Scope validation before reuse.
- [x] Immutable historical records.
- [x] Explicit stale and revalidation status.
- [x] No silent replacement of a prior procedure version.

### Acceptance

A contradiction creates a new status or version while the original evidence
and previous procedure remain readable.

**Implementation:** [`src/framework/stores.rs`](../src/framework/stores.rs)
provides `EvidenceStore`, `HypothesisStore`, `ProcedureStore`,
`DependencyIndex`, and the aggregate `LearnerStores` registration seam. The
focused coverage is in [`tests/mvp3_stores.rs`](../tests/mvp3_stores.rs).
Serialization remains intentionally deferred to MVP-4.

**Verification:** `cargo.exe fmt --all -- --check`,
`cargo.exe check --all-targets`, `cargo.exe test --test mvp3_stores`,
`cargo.exe test --all-targets`, and
`cargo.exe clippy --all-targets -- -D warnings` passed on 2026-10-12;
103 tests passed with no clippy warnings.

### MVP-4 — Persistence and restart spike

**Status:** Complete (2026-10-12)

**Goal:** prove that the learner is more than an in-memory experiment.

Implement this as an early risk gate, not as a final packaging detail.

### Tasks

- [x] Define a versioned state format for run metadata, ledger events, evidence,
  hypotheses, procedures, dependencies, and resource counters.
- [x] Write canonical JSON/JSONL using the Rust standard library.
- [x] Validate schema version, duplicate identifiers, malformed records, scope
  violations, and incomplete writes during load.
- [x] Write through a temporary file and atomically rename completed state.
- [x] Restore mutable state only when explicitly marked mutable.
- [x] Preserve frozen state as frozen.
- [x] Compute a deterministic state fingerprint.

### Acceptance

1. Process A learns a procedure and saves state.
2. Process A exits.
3. Process B loads the state without evaluator labels.
4. Process B uses the procedure on a held-out input.
5. Evidence references and procedure versions remain intact.

**Implementation:** [`src/framework/persistence.rs`](../src/framework/persistence.rs)
provides versioned canonical JSON persistence through `FrameworkState`, strict
load validation, atomic replacement, mutable/frozen access control, and
deterministic state fingerprints. The restart and tamper-resistance coverage is
in [`tests/mvp4_persistence.rs`](../tests/mvp4_persistence.rs). Process-local
wall-clock and resident-memory samples are intentionally not restored; logical
resource state is persisted.

**Verification:** `cargo.exe fmt --all -- --check`,
`cargo.exe check --all-targets`, `cargo.exe test --test mvp4_persistence`,
`cargo.exe test --all-targets`, and
`cargo.exe clippy --all-targets -- -D warnings` passed on 2026-10-12;
112 tests passed with no clippy warnings.

### MVP-5 — Generic run controller

**Status:** Complete (2026-10-12)

**Goal:** run the same lifecycle against different environments.

```text
create run
  -> discovery
  -> development
  -> admission and revision
  -> freeze
  -> held-out evaluation
  -> save report and learner state
```

### Required behavior

- [x] Record the decision before applying an action.
- [x] Close abstentions as `incomplete` without fabricating observations.
- [x] Ingest results only after ledger recording.
- [x] Prevent final results from changing frozen decisions.
- [x] Enforce action, candidate, operation, and storage budgets.
- [x] Distinguish insufficient evidence, search exhaustion, unsupported grammar,
  incorrect hypothesis, and environment failure.

### Acceptance

The controller contains no environment-specific branches and can run the v2
adapter plus an independent second adapter.

**Implementation:** [`src/framework/controller.rs`](../src/framework/controller.rs)
provides `GenericRunController`, configurable `EpisodeBudget`, typed controller
errors, episode summaries, and run summaries. Contract coverage is in
[`tests/mvp5_controller.rs`](../tests/mvp5_controller.rs). Arbitrary learner
serialization remains outside the generic controller; MVP-4 persists
framework-owned state and callers retain the learner-specific snapshot hook.

**Verification:** `cargo.exe fmt --all -- --check`,
`cargo.exe check --all-targets`, `cargo.exe test --test mvp5_controller`,
`cargo.exe test --all-targets`, and
`cargo.exe clippy --all-targets -- -D warnings` passed on 2026-10-12;
120 tests passed with no clippy warnings.

### MVP-6 — Two environment adapters

**Status:** Complete for the narrow adapter/runtime claim (2026-10-11)

#### Adapter A: existing v2 world

- [x] Wrap `EvaluatorWorld` behind the generic environment interface.
- [x] Preserve opaque mappings, scopes, terminal rules, and freeze behavior.
- [x] Keep hidden labels out of learner state and learner traces.

#### Adapter B: independent structural world

- [x] Use different hidden identifiers.
- [x] Use different transition logic.
- [x] Use a different schedule and seed.
- [x] Include held-out context or value combinations.
- [x] Require structural transfer beyond exact-example retrieval.
- [x] Ensure it is not merely a renamed copy of Adapter A.

### Acceptance

The same runtime and controller can run both adapters. The real
`GroundingLearner` now participates through an additive ledger-aware learner
path: when it records a receipt during selection, `GenericRunController` adopts
that receipt instead of recording a duplicate. The focused MVP-6 tests run that
controller against both adapters, verify opaque/deterministic behavior, and
exercise the retained E2/E3 structural relation on a held-out Adapter B
context.

**Implementation:** [`src/mvp6.rs`](../src/mvp6.rs) provides the opaque v2
adapter and an independent prime/commit structural toy environment. The
additive `LedgerLearner`/`LedgerFrozenLearner` seams in
[`src/framework/mod.rs`](../src/framework/mod.rs) preserve the MVP-5
`Learner` API while allowing [`GroundingLearner`](../src/framework/grounding_decision.rs)
to keep ledger ownership honest. [`GenericRunController`](../src/framework/controller.rs)
adopts already-recorded receipts and passes optional abstention results back to
ledger-aware learners. Coverage is in
[`tests/mvp6_adapters.rs`](../tests/mvp6_adapters.rs).

**Known boundary:** the generic controller does not invent a benchmark-neutral
candidate-generation/admission policy. The test invokes the reusable
`GrammarV2` and `GroundingLearner` induction hooks explicitly through public
ledger-aware APIs before freezing. The controller also exposes the mutable
learner/ledger pair for an application-owned hook. The next open learner/runtime
step is to define that induction hook as a first-class generic lifecycle
operation if a future MVP needs controller-owned admission. MVP-7 CLI and
architecture experiments remain open and were not implemented here.

**Verification:** `cargo.exe fmt --all -- --check`,
`cargo.exe check --all-targets`, `cargo.exe test --test mvp6_adapters`,
`cargo.exe test --all-targets`, and
`cargo.exe clippy --all-targets -- -D warnings` all passed on 2026-10-12.
The code graph was refreshed with `graphify update .` through the available
Windows batch entry point.

### MVP-7 — CLI and inspectable artifacts

**Status:** Complete for the bounded CLI/artifact claim

Add a command such as:

```text
cargo.exe run --bin run_mvp -- \
  --environment structural-a \
  --store artifacts/mvp/state \
  --report artifacts/mvp/report.json
```

The CLI must support:

- selecting an environment;
- creating or loading state;
- running discovery and development;
- freezing and evaluating held-out cases;
- saving traces and state; and
- returning a nonzero status when an MVP gate fails.

Reports must answer:

- what procedure was learned;
- which evidence supported it;
- which actions were selected;
- where the learner abstained;
- whether the procedure was reused after restart;
- whether later evidence made anything stale or contradicted; and
- whether resource limits were respected.

**Implementation:** [`src/bin/run_mvp.rs`](../src/bin/run_mvp.rs) runs both
MVP-6 adapters through the generic controller, invokes the explicitly named
application-owned induction/admission hook, persists a mutable
[`FrameworkState`](../src/framework/persistence.rs) checkpoint, reloads and
reinstalls procedure evidence, then performs frozen final evaluation. The
canonical JSON/JSONL report and learner artifacts are documented in
[`artifacts/mvp/README.md`](../artifacts/mvp/README.md), with focused coverage
in [`tests/mvp7_cli.rs`](../tests/mvp7_cli.rs). The generic controller's
arbitrary-learner resume limitation is recorded in the report and README;
MVP-8 broad verification and architecture experiments remain outside this
implementation.

**Verification:** `cargo.exe fmt --all -- --check`,
`cargo.exe check --bin run_mvp --test mvp7_cli`,
`cargo.exe test --test mvp7_cli`,
`cargo.exe clippy --bin run_mvp --test mvp7_cli -- -D warnings`,
`cargo.exe test --all-targets`, and
`cargo.exe clippy --all-targets -- -D warnings` passed on 2026-10-12.
The documented structural-a and structural-b commands both reported
`validated-mvp-narrow-claim`.

### MVP-8 — MVP verification

**Status:** Complete for the bounded MVP claim (2026-10-11)

Add tests for:

- environment and learner contracts;
- decision-before-result ordering;
- scope isolation;
- persistence round-trip and fresh-process restoration;
- frozen-state immutability;
- held-out transfer;
- justified abstention;
- contradiction and version preservation;
- deterministic fingerprints and traces; and
- evaluator-label leakage.

### MVP completion gates

All of the following must pass:

| Gate | Requirement |
| --- | --- |
| M1 | v1 artifacts and historical results are unchanged. |
| M2 | The runtime is callable without benchmark evaluator imports. |
| M3 | Every decision precedes its environment result. |
| M4 | Evidence, hypotheses, procedures, and revisions are immutable/versioned. |
| M5 | A learned procedure survives process restart. |
| M6 | Restored state transfers to a held-out input. |
| M7 | Unsupported and non-applicable inputs abstain or use explicit fallback. |
| M8 | Contradictions produce stale, revised, or suspended dependents. |
| M9 | One controller runs two environments. |
| M10 | Reports contain evidence, procedures, budgets, failures, and fingerprints. |
| M11 | Three reruns produce equivalent logical traces. |
| M12 | Formatting, compilation, tests, and clippy pass without warnings. |

The status may be reported as:

```text
validated-mvp-narrow-claim
```

**Verification package:** [`tests/mvp8_verification.rs`](../tests/mvp8_verification.rs)
aggregates M1–M12 across the public runtime/store boundary, historical v1
fingerprints, both `run_mvp` environments, persistence artifacts, reports,
leakage checks, and three deterministic fresh reruns. Exact commands and the
evidence mapping are recorded in [`artifacts/mvp/README.md`](../artifacts/mvp/README.md).
Experiments A–C remain outside this MVP.

This status means the reusable substrate passed its gates. It does not mean
that the system has learned language, general concepts, or broad intelligence.

## 8. Phase 2 — Architecture decision experiments

Do not widen the MVP to include these capabilities. Each experiment needs its
own specification, frozen protocol, baselines, acceptance gates, and negative
result policy.

### Experiment A — Variable-length structure and boundary discovery

**Specification/protocol status:** Complete and frozen (2026-10-12). See the
[`frozen specification`](specs/variable-length-boundary-discovery-experiment-a.md)
and [`preregistered protocol`](protocols/variable-length-boundary-discovery-experiment-a-protocol-v1.0.md).
**Implementation status:** Complete (2026-10-12) with `validated-a1-narrow-claim`
and `validated-a3-comparator` on the primary groups; see
[`remaining-tasks.md`](remaining-tasks.md) T9 and
[`artifacts/experiment_a/README.md`](../artifacts/experiment_a/README.md).

**Question:** Can the system discover reusable structure when sequence length
and boundaries are not supplied?

#### Environment

Generate streams containing:

- repeated motifs;
- distractors;
- optional segments;
- reordered independent segments;
- held-out lengths; and
- ambiguous boundary cases.

#### Variants

1. Fixed-position/equality baseline.
2. Explicit symbolic segmentation and alignment search.
3. Learned representation or parser feeding symbolic induction.
4. Library-learning variant that promotes recurring motifs.

#### Measurements

- boundary precision and recall;
- false merge and false split rates;
- transfer to held-out lengths;
- structural generalization;
- candidate and search cost;
- evidence traceability;
- abstention under ambiguity; and
- robustness to irrelevant motifs.

#### Decision

- If symbolic segmentation is tractable and transfers, deepen the symbolic
  representation layer.
- If symbolic search fails but a learned front end improves representation,
  define a hybrid input boundary.
- If neither approach transfers, do not proceed to raw text; revisit the
  representation objective and task design.

### Experiment B — Transition model and goal-directed planning

**Question:** What is the smallest state and transition representation that
supports reliable planning beyond two steps?

#### Environment

Use deterministic tasks first, then controlled stochastic variants. Require
horizons of 4, 8, and 16 steps with explicit goals and action costs.

#### Variants

1. Reactive one-step procedure reuse.
2. Symbolic transition hypotheses with bounded breadth-first or A* search.
3. Tabular/probabilistic model with model-predictive search.
4. Latent world model only if the later task requires raw observations.

#### Required representation

- persistent state identity;
- action schemas;
- preconditions and effects;
- goal or constraint records;
- hypothetical branches;
- model uncertainty;
- observed versus imagined evidence; and
- execution/replanning status.

#### Measurements

- goal success by horizon;
- multi-step prediction error;
- uncertainty calibration and coverage;
- plan cost;
- replanning frequency;
- counterfactual/observed provenance correctness;
- sample efficiency; and
- failure classification after model error.

#### Decision

- If symbolic state is sufficient, keep the transition model interpretable and
  defer neural latents.
- If state aliasing or observation complexity dominates, define a hybrid
  representation seam before adding a planner.
- If model error prevents safe planning, add explicit uncertainty and
  risk-sensitive policy before expanding the horizon.

### Experiment C — Primitive and procedure-library discovery

**Question:** Can repeated learned procedures become reusable primitives that
improve future tasks without over-generalizing?

#### Environment

Create task families where two- or three-step subprocedures recur under
different contexts. Add adversarial tasks where superficial repetition should
not create a primitive.

#### Variants

1. No learned library.
2. Hand-fixed library.
3. Evidence-backed learned library promotion.

#### Required behavior

- identify repeated subtraces;
- propose a typed abstraction;
- retain the expansion path to concrete evidence;
- test the abstraction on held-out compositions;
- version or reject harmful abstractions; and
- measure whether the library improves or harms search.

#### Measurements

- library growth;
- description-length or compression change;
- solve time and candidate count;
- held-out composition transfer;
- false abstraction rate;
- evidence expansion size;
- revision cost; and
- downstream search improvement.

#### Decision

- If learned libraries improve transfer at bounded cost, make library learning
  a first-class subsystem.
- If libraries improve compression but harm search, add stronger typing,
  utility thresholds, or retrieval controls.
- If libraries do not transfer, preserve procedures locally and do not claim
  autonomous concept discovery.

## 9. Phase 3 — Architecture selection

After the MVP and Experiments A–C, choose the next architecture from evidence.

### Decision matrix

| Result | Next architecture |
| --- | --- |
| A passes symbolically and C passes with small libraries | Extend the symbolic/program-synthesis core. |
| A fails on raw representation but B works on structured state | Add a learned representation front end while preserving symbolic stores. |
| B requires uncertainty and long-horizon learned dynamics | Add a model-based RL/world-model layer with explicit imagined evidence. |
| C shows strong transfer | Make primitive/library learning a first-class subsystem before raw text. |
| A, B, and C all fail | Revisit the representation, objective, and inductive bias before adding complexity. |

### Required architecture decision record

The selected next architecture must document:

- which experiment results justify it;
- which capabilities remain unsupported;
- which evidence classes are added;
- how learned representations enter the symbolic stores;
- how uncertainty is represented;
- how search and storage remain bounded;
- what new protocol freezes the claim; and
- what failure would cause the approach to be revised or abandoned.

## 10. Raw text and multimodal learning policy

Raw text must remain a separate grounding program.

Before claiming text learning, the project must specify and test:

- character and stream preservation;
- segmentation and token hypotheses;
- phrase and entity discovery;
- syntax and semantic hypotheses;
- grounding through tasks or interaction;
- ambiguity and competing parses;
- source and model provenance;
- uncertain learned-to-symbolic conversion; and
- held-out compositional transfer.

Attaching a string field to `ObservationPacket` or adding a handcrafted parser
does not count as learned language understanding.

## 11. Risks and stop conditions

### Main risks

- Persistence may expose that current records are not sufficient to reconstruct
  learner behavior.
- Runtime extraction may reveal hidden coupling to the v2 benchmark.
- The second environment may show that the grammar is overfit to one world.
- Primitive/library learning may cause combinatorial search growth.
- Variable-length segmentation may be unidentifiable from available evidence.
- World-model predictions may compound error over longer horizons.
- A learned representation may improve prediction while destroying auditability
  or transfer.
- Durable storage may grow faster than revalidation can keep up.

### Stop or revisit conditions

Pause expansion and revise the architecture if:

- save/load changes decisions or loses evidence references;
- the generic controller requires environment-specific learner logic;
- Adapter B fails because the learner depends on hidden benchmark assumptions;
- a procedure cannot distinguish observed from imagined support;
- library promotion improves training fit but reduces held-out transfer;
- planning succeeds only through evaluator leakage or final-answer reuse; or
- resource costs grow without a declared bounded-failure policy.

## 12. Verification commands

### MVP verification

```text
cargo.exe fmt --all -- --check
cargo.exe check --all-targets
cargo.exe test --all-targets
cargo.exe clippy --all-targets -- -D warnings
cargo.exe run --bin run_mvp -- --environment structural-a --store artifacts/mvp/state --report artifacts/mvp/report.json
cargo.exe run --bin run_mvp -- --environment structural-b --store artifacts/mvp/state-b --report artifacts/mvp/report-b.json
```

The final commands and artifact layout must be recorded in
`artifacts/mvp/README.md`.

### Decision-experiment verification

Each of Experiments A–C must have:

- a written specification;
- a frozen protocol before final evaluation;
- a baseline without the proposed capability;
- a deterministic or explicitly seeded generator;
- development/final separation;
- resource and failure measurements;
- leakage tests;
- raw traces and artifact hashes; and
- a decision report with positive and negative outcomes.

## 13. Completion deliverables

### Immediate MVP deliverables

- reusable framework modules;
- public environment and learner interfaces;
- first-class evidence, hypothesis, and procedure stores;
- versioned durable state;
- fresh-process restoration;
- generic run controller;
- two environment adapters;
- `run_mvp` or equivalent CLI;
- inspection-ready traces and reports;
- tests for every MVP gate; and
- `artifacts/mvp/README.md` with exact reproduction commands.

### Post-MVP decision deliverables

- Experiment A specification, implementation, and report;
- Experiment B specification, implementation, and report;
- Experiment C specification, implementation, and report;
- an architecture decision record based on their results; and
- a new protocol for the selected next capability.

## 14. Final sequence

```text
preserve v1/v2 artifacts
        ->
freeze MVP boundary and public seams
        ->
extract reusable runtime
        ->
make learner stores explicit
        ->
prove persistence and restart
        ->
run one generic controller on two environments
        ->
validate the loadable MVP
        ->
run variable-length experiment A
        ->
run world-model/planning experiment B
        ->
run primitive/library experiment C
        ->
choose the next architecture from results
        ->
write a new specification and protocol
```

The project should not skip from the current fixed-packet learner directly to
raw text or a complete brain-machine implementation. The experiments above are
the checkpoints that determine whether the next layer should remain symbolic,
become program-synthesis based, or add a learned hybrid/world-model component.
