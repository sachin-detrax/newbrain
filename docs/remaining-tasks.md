# Remaining Tasks

Updated: 2026-10-12

## Current status

- v1 implementation Tasks 1–10 are complete.
- The removed-development-challenge scoring correction is complete and verified.
- The recorded result is `validated-v1-narrow-claim`.
- T1 is complete; the separate v2 specification is now available for review.
- T2 is complete; the preregistered v2 protocol is now frozen before implementation.
- T3, T4, T5, and T6 are complete; T7 produced a validated narrow v2 result.
- No implementation placeholders remain in `src/` or `tests/`.
- The requested Cargo verification suite passed on 2026-10-11, including the
  v2 resource-report fix and the actual all-variant experiment.
- Experiment A implementation is complete and verified: A1 primary
  `validated-a1-narrow-claim`, A3 primary `validated-a3-comparator`, with
  3/3 byte-identical reruns per group and permutation parity held.
- The architecture review is recorded in
  [`docs/research/architecture-gap-research-2026-10-11.md`](research/architecture-gap-research-2026-10-11.md).
  It recommends preserving the validated kernel, completing the bounded MVP,
  and running separate decision experiments for representation discovery,
  transition models/planning, and primitive/library learning before widening
  the architecture.
- The combined implementation and research roadmap is in
  [`docs/plan.md`](plan.md).
- MVP-1 public runtime seams, MVP-2 runtime extraction, MVP-3 learner-owned
  stores, MVP-4 persistence/restart, MVP-5 generic run control, the narrow
  MVP-6 two-adapter claim, MVP-7 CLI/artifact work, and MVP-8 bounded
  verification/package work are complete for the bounded claim.
- Architecture Experiment A's specification and evaluation protocol are frozen
  in [`docs/specs/variable-length-boundary-discovery-experiment-a.md`](specs/variable-length-boundary-discovery-experiment-a.md)
  and [`docs/protocols/variable-length-boundary-discovery-experiment-a-protocol-v1.0.md`](protocols/variable-length-boundary-discovery-experiment-a-protocol-v1.0.md).
- The next work after this step is Experiment B specification; Experiments B
  and C remain out of scope until their own specifications and protocols
  are frozen. No implementation result beyond Experiment A is included here.

### MVP-8 — End-to-end verification and packaging

**Status:** Complete for the bounded MVP claim (2026-10-11)

- [`tests/mvp8_verification.rs`](../tests/mvp8_verification.rs) aggregates M1–M12
  without changing historical v1/v2 artifacts.
- The verifier checks the public runtime boundary, decision/result ordering,
  immutable/versioned stores, persistence and fresh-process restore, explicit
  abstention, contradiction handling, both adapters, report budgets/fingerprints/
  leakage fields, deterministic reruns, and artifact validity.
- [`artifacts/mvp/README.md`](../artifacts/mvp/README.md) records the exact
  reproduction commands and evidence for each gate.
- Experiment A's specification and protocol are approved and frozen; its
  implementation remains intentionally unimplemented. Experiments B and C
  remain out of scope until their own specifications and protocols are frozen.

### MVP-6 — Two environment adapters

**Status:** Complete for the narrow adapter/runtime claim (2026-10-11)

- Adapter A is [`V2EnvironmentAdapter`](../src/mvp6.rs), which wraps the
  existing v2 world without changing the v2 runner or artifacts.
- Adapter B is [`StructuralToyEnvironment`](../src/mvp6.rs), an independent
  deterministic prime/commit world with different opaque identifiers, scope
  stream, schedule, transition logic, a non-applicable challenge, and held-out
  contexts.
- [`GenericRunController`](../src/framework/controller.rs) now supports the
  additive ledger-aware [`LedgerLearner`](../src/framework/mod.rs) seam. The
  real [`GroundingLearner`](../src/framework/grounding_decision.rs) uses it to
  return the receipt it already recorded; no decision is silently duplicated.
- [`tests/mvp6_adapters.rs`](../tests/mvp6_adapters.rs) covers Adapter A
  compatibility, Adapter B dynamics and opacity, shared-controller execution,
  deterministic reruns, leakage, and held-out structural transfer.

**Known limitation:** candidate generation and admission remain an explicit
application-owned induction hook. MVP-6 proves that the generic runtime and
controller can carry the real structural learner and both environments; it does
not claim that the generic controller autonomously chooses a grammar or
admission policy. MVP-7 records that boundary in its CLI report rather than
hiding it. Architecture experiments remain intentionally unimplemented.

### MVP-7 — CLI and inspectable artifacts

**Status:** Complete for the bounded CLI/artifact claim

- [`src/bin/run_mvp.rs`](../src/bin/run_mvp.rs) runs both MVP-6 adapters through
  `GenericRunController`, invokes the application-owned induction/admission
  hook, saves a mutable `FrameworkState` checkpoint, reloads it, reinstalls
  procedure evidence, and performs frozen final evaluation.
- The CLI writes canonical JSON/JSONL reports, opaque learner traces, mutable
  and frozen state snapshots, candidate events, fingerprints, phase summaries,
  failures, budgets, and leakage status under the requested store path.
- [`tests/mvp7_cli.rs`](../tests/mvp7_cli.rs) covers argument failures, both
  successful environments, report/store creation, deterministic logical
  fields, gate failure, and evaluator-label absence.
- Reproduction commands and the explicit resume boundary are documented in
  [`artifacts/mvp/README.md`](../artifacts/mvp/README.md).

### MVP-2 — Extract the reusable learner runtime

**Status:** Complete (2026-10-12)

The reusable implementation now lives under [`src/framework`](../src/framework)
and is re-exported through [`src/runtime.rs`](../src/runtime.rs). It owns the
opaque protocol, receipt-backed interaction ledger, resource accounting,
bounded grammar/admission, grounding policy, and partial-observation seam.
Historical [`src/v2`](../src/v2) learner paths remain compatibility re-exports.
The v2 [`adapter`](../src/v2/adapter.rs) contains the legacy `Evaluator` bridge;
the evaluator world, frozen schedule, experiment runner, B2, and scoring remain
benchmark-specific.

Focused architecture coverage is in
[`tests/mvp2_architecture.rs`](../tests/mvp2_architecture.rs). It exercises a
custom environment and learner without v2 evaluator imports, the existing v2
runner bridge, and the reusable-source boundary.

## Ordered task list

### T0 — Reconcile implementation-plan tracking

**Priority:** P1  
**Status:** Complete (2026-10-11)  
**Depends on:** None

- [x] Mark the completed steps in `docs/superpowers/plans/2026-10-10-sparse-structural-rule-learner-v1.md`, or add a clear completion summary to that plan.
- [x] Keep `.superpowers/sdd/sparse-structural-rule-learner-v1/progress.md` as the execution record.
- [x] Link this file and the final artifacts from the plan so the plan no longer appears unfinished.

**Done:** The plan’s v1 implementation and verification steps are checked off from the progress ledger; the next open item is T1.

### T1 — Choose and freeze the next experiment

**Priority:** P0  
**Status:** Complete (2026-10-11)  
**Depends on:** T0

- [x] Decide the precise capability to test after the fixed transition benchmark.
- [x] Keep the v1 result and grammar version immutable for comparison.
- [x] State the new experiment boundary, learner inputs, evaluator-only information, output protocol, scope rules, and permitted supervision.
- [x] Decide which claims remain excluded: raw-text learning, semantic understanding, planning, causal reasoning, broad world modeling, and scale superiority.

**Done:** [`docs/specs/opaque-interactive-structural-discovery-v2.md`](specs/opaque-interactive-structural-discovery-v2.md) freezes the opaque interactive world, learner and evaluator seams, v2 grammar boundary, hypotheses, supervision, scope rules, exclusions, inherited v1 invariants, and implementation responsibilities. No Rust implementation is authorized by T1.

### T2 — Write the v2 evaluation protocol before implementation

**Priority:** P0  
**Status:** Complete (2026-10-11)  
**Depends on:** T1

- [x] Define the dataset or environment generator, training/development/final partitions, held-out transformations, and leakage boundary.
- [x] Pre-register acceptance gates for coverage, accuracy, false positives, abstention quality, retention, evidence traceability, and budget honesty.
- [x] Define the baselines: exact-example retrieval, random exploration where interaction is required, and a fixed hand-written controller/reference.
- [x] Set resource measurements and thresholds before inspecting final results.
- [x] Define failure classifications so insufficient evidence, search exhaustion, and incorrect hypotheses remain distinguishable.

**Done:** [`docs/protocols/opaque-interactive-structural-discovery-v2-protocol-v1.0.md`](protocols/opaque-interactive-structural-discovery-v2-protocol-v1.0.md) freezes the world generator, seeds, `12/8/16` partitions, held-out transformations, `72` decision/action ceilings, B0/B1/B2 baselines, primary and secondary numeric gates, failure classifications, deterministic reruns, artifact/resource reports, and reveal order. No Rust code was added.

### T3 — Add explicit resource and scale measurement

**Priority:** P1  
**Status:** Complete (2026-10-11)  
**Depends on:** T2

- [x] Add monotonic wall-clock, injectable resident-memory, candidate-count, ledger-size, and query/operation measurements to the v2 seam.
- [x] Define deterministic scale sweeps for symbol cardinality, record count, candidate count, and retained procedures.
- [x] Report the cost of canonicalization, retrieval, admission, revalidation, candidate generation, and decision/action work separately.
- [x] Do not make speed, energy, GPU, or scale claims from v1; v1 explicitly did not implement these measurements.

**Done:** [`src/framework/resource_scale.rs`](../src/framework/resource_scale.rs) provides the isolated measurement seam, fixed protocol ceilings, pre-crossing validation, explicit missing-memory state, monotonic timing, the runtime platform sampler, and canonical scale serialization. [`tests/v2_resource_measurement.rs`](../tests/v2_resource_measurement.rs) covers counter accumulation, stage separation, missing and injected memory sampling, runtime sampling, limit enforcement, and deterministic scale serialization. The requested Cargo verification suite passed on 2026-10-11.

### T4 — Build an opaque input and interaction boundary

**Priority:** P1  
**Status:** Complete (2026-10-11)  
**Depends on:** T2

- Replace the hand-authored transition fixture for the next experiment with an input adapter that exposes only the declared opaque observations and transport metadata.
- Keep event boundaries, field names, entity identities, action meanings, and outcome labels out of learner input unless the protocol explicitly counts them as supervision.
- Add held-out symbol permutations and episode/scope changes.
- Add partial observability cases where missing evidence must not become a negative fact.

**Done when:** The learner receives only the frozen protocol interface, and an audit can trace every supplied field back to an allowed source.

**Done:** The isolated learner-facing contract is implemented in [`src/framework/opaque_protocol.rs`](../src/framework/opaque_protocol.rs), with immutable scope-preserving decision/evidence records in [`src/framework/interaction_ledger.rs`](../src/framework/interaction_ledger.rs). The evaluator-only deterministic world, SplitMix64/Fisher–Yates mappings, fresh scope generation, and exact 36-row schedule are in [`src/v2/evaluator_world.rs`](../src/v2/evaluator_world.rs). Explicit partial evidence is isolated in [`src/framework/partial_observation.rs`](../src/framework/partial_observation.rs), and the small public learner/evaluator traits are exposed from [`src/framework/mod.rs`](../src/framework/mod.rs) and [`src/v2/adapter.rs`](../src/v2/adapter.rs). Coverage is exercised by [`tests/v2_opaque_interaction.rs`](../tests/v2_opaque_interaction.rs), including validation, reveal ordering, mappings, scopes, transitions, held-out rows, and unknown partial evidence. The requested verification suite and actual all-variant run passed on 2026-10-11.

### T5 — Extend structural discovery under a bounded grammar

**Priority:** P1  
**Status:** Complete (2026-10-11)  
**Depends on:** T3 and T4

- Specify the next bounded candidate-generation stages beyond the fixed G0/G1 equality-and-copy grammar.
- Represent hypotheses, competing predictions, edit locations, evidence references, and rejection reasons explicitly.
- Keep candidate generation finite, deterministic, budgeted, and inspectable.
- Test primitive or relation promotion separately from ordinary repair, preserving an expansion link to the underlying evidence.
- Preserve abstention and `incomplete` behavior when the grammar or budget cannot decide.

**Done when:** The next grammar has a written search boundary, deterministic signatures, auditable costs, and adversarial tests before it is used for a capability claim.

**Done:** [`src/framework/grammar_admission.rs`](../src/framework/grammar_admission.rs) implements the isolated opaque-only `E0`–`E4` grammar, deterministic signatures/order, explicit rejection/incomplete statuses, ResourceMeter accounting, admission gates, run-scope promotion provenance, and immutable contradiction suspension records. It is re-exported from [`src/v2/mod.rs`](../src/v2/mod.rs) for compatibility. [`tests/v2_grammar_admission.rs`](../tests/v2_grammar_admission.rs) covers stage generation, held-out opaque-ID transfer, deterministic ordering, unstable and single-support rejection, missing challenge/grounding, scope leakage, output invention, budget exhaustion, promotion provenance, and later contradiction versioning. The requested verification suite and actual all-variant run passed on 2026-10-11.

### T6 — Add discriminating evidence and revision tests

**Priority:** P1  
**Status:** Complete (2026-10-11)  
**Depends on:** T5

- Select actions or observations by disagreement among current hypotheses rather than by random exploration alone.
- Record the competing predictions, requested evidence, observed result, and budget spent.
- Test partial observability, changed transition rules, scope confinement, stale derived results, and revalidation.
- Preserve old evidence and procedure versions when a later observation invalidates or narrows a procedure.

**Done when:** The system can explain why it requested an observation, distinguish contradiction from insufficient evidence, and revise without erasing history.

**Done:** The isolated learner-facing policy and frozen boundary are implemented in [`src/framework/grounding_decision.rs`](../src/framework/grounding_decision.rs), using the existing opaque protocol, interaction ledger, resource meter, and grammar admission records. Evaluator-only changed-rule traces live in [`src/v2/changed_rule.rs`](../src/v2/changed_rule.rs), with no changes to the primary `EvaluatorWorld`. Coverage is exercised by [`tests/v2_grounding_decision.rs`](../tests/v2_grounding_decision.rs), including canonical fallback, procedure prediction, conflict abstention, lowest-cost disagreement exploration, budget exhaustion, frozen non-mutation, partial uncertainty, stale/conditional/incomplete status, and fresh-scope revision preservation. The requested verification suite and actual all-variant run passed on 2026-10-11.

### T7 — Run and package the next experiment

**Priority:** P1  
**Status:** Validated narrow claim (2026-10-11)  
**Depends on:** T2–T6

- [x] Add the isolated deterministic runner for one variant/predictor and all
  required primary, permutation, B0, B1, and evaluator-only B2 runs.
- [x] Repeat deterministic runs and compare fingerprints, traces, metrics, and
  acceptance reports through the three-rerun helper.
- [x] Package raw traces, budgets, failures, abstentions, resource measurements,
  scale points, evaluator-only manifests, and artifact hashes.
- [x] Write the decision/report serialization with explicit P1–P12 and S1–S7
  gates and frozen failure classifications.
- [x] Execute the Cargo command, inspect the generated reports, and publish
  behavioral results. `P1`–`P12` and `S1`–`S7` all passed, with measured
  learner memory and nonzero wall-clock reports.

**Done when:** The implementation and package are independently reproducible;
the exact all-variant command produces `validated-v2-narrow-claim` with all
primary and secondary gates passed.

**Implementation links:** [`src/v2/experiment.rs`](../src/v2/experiment.rs),
[`src/v2/mod.rs`](../src/v2/mod.rs),
[`src/v2/adapter.rs`](../src/v2/adapter.rs),
[`src/framework/opaque_protocol.rs`](../src/framework/opaque_protocol.rs),
[`src/v2/evaluator_world.rs`](../src/v2/evaluator_world.rs),
[`src/framework/interaction_ledger.rs`](../src/framework/interaction_ledger.rs),
[`src/framework/resource_scale.rs`](../src/framework/resource_scale.rs),
[`src/framework/grammar_admission.rs`](../src/framework/grammar_admission.rs),
[`src/framework/grounding_decision.rs`](../src/framework/grounding_decision.rs),
[`src/v2/changed_rule.rs`](../src/v2/changed_rule.rs),
[`src/framework/partial_observation.rs`](../src/framework/partial_observation.rs),
[`src/bin/run_v2.rs`](../src/bin/run_v2.rs),
[`tests/v2_experiment.rs`](../tests/v2_experiment.rs),
[`tests/v2_opaque_interaction.rs`](../tests/v2_opaque_interaction.rs),
[`tests/v2_resource_measurement.rs`](../tests/v2_resource_measurement.rs),
[`tests/v2_grammar_admission.rs`](../tests/v2_grammar_admission.rs),
[`tests/v2_grounding_decision.rs`](../tests/v2_grounding_decision.rs),
[`artifacts/v2/README.md`](../artifacts/v2/README.md), and
[`artifacts/v2/.gitkeep`](../artifacts/v2/.gitkeep).

### T8 — Freeze Architecture Experiment A specification and protocol

**Priority:** P0  
**Status:** Complete (2026-10-12)  
**Depends on:** MVP-8 and the architecture-gap review

- [x] Freeze the variable-length raw-stream boundary and evaluator-only latent
  structure.
- [x] Freeze symbolic segmentation/alignment grammar, A0/A1/A3 comparators,
  and the declared future A2 learned-front-end comparator.
- [x] Freeze training/development/final partitions, held-out lengths/motifs,
  distractors, optional/reordered/ambiguous cases, scoring, thresholds,
  leakage rules, resource limits, reruns, and reveal order.
- [x] Link the specification and preregistered protocol from the planning
  documents.

**Done:** [`docs/specs/variable-length-boundary-discovery-experiment-a.md`](specs/variable-length-boundary-discovery-experiment-a.md)
and [`docs/protocols/variable-length-boundary-discovery-experiment-a-protocol-v1.0.md`](protocols/variable-length-boundary-discovery-experiment-a-protocol-v1.0.md)
are frozen before implementation. The next task is Experiment A
implementation; no Rust code was added here.

### T9 — Implement Architecture Experiment A

**Priority:** P0  
**Status:** Complete (2026-10-12)  
**Depends on:** T8

- [x] Implement the evaluator-only `variable-stream-generator-a-v1` with the
  frozen SplitMix64 transition, seeds, 160-row schedule, and redraw rules.
- [x] Implement A0 fixed-position/equality baseline, A1 symbolic
  segmentation/alignment (V0–V4), and A3 library promotion (V5) with the
  R0 oracle for scoring only.
- [x] Run 3 primary + 3 permutation reruns per variant with byte-identical
  deterministic cores, aggregate gates, leakage audit, and artifact
  packaging through `run_a`.
- [x] Record the result without touching v1/v2/MVP artifacts or the frozen
  specification/protocol documents.

**Done:** `validated-a1-narrow-claim` on the A1 primary group and
`validated-a3-comparator` on the A3 primary group, each 3/3 byte-identical
with permutation parity held. A0 reports `no-result` as an integrity
baseline by design. A2 remains `not-enrolled-under-v1.0`.

**Implementation links:** [`src/experiment_a/`](../src/experiment_a/),
[`src/bin/run_a.rs`](../src/bin/run_a.rs),
[`tests/experiment_a_*.rs`](../tests/),
[`artifacts/experiment_a/README.md`](../artifacts/experiment_a/README.md).

## Deferred v1 cleanup

These are not blockers for the recorded v1 result, but should be completed before treating the v1 interfaces as a long-lived library:

**Status:** Complete (2026-10-11)

- [x] Strengthen `Response` construction so the non-empty procedure invariant cannot be bypassed through public enum fields. `Prediction` and `Abstention` now use opaque payload types with private fields, while `Response::output`, `reason`, `procedure_ids`, and `evidence_refs` provide stable read-only access. Existing constructors and abstention behavior are unchanged.
- [x] Add a regression test that a multi-procedure prediction preserves its output, every evidence reference, and all procedure IDs.
- [x] Make the leakage test use an independently specified expected fixture containing record IDs `1`–`6` and `15`–`20` rather than deriving both sides from the same helper.
- [x] Re-run the complete Cargo verification suite without changing v1 artifacts or experiment results.

### Verification

The requested verification commands completed successfully on 2026-10-11:

- `cargo.exe fmt --all` — passed.
- `cargo.exe test --test types_and_dataset --test engine --test adversarial --test budgets_and_determinism` — passed; 25 tests passed, 0 failed.
- `cargo.exe check --all-targets` — passed.
- `cargo.exe test --all-targets` — passed; 85 tests passed, 0 failed.
- `cargo.exe clippy --all-targets -- -D warnings` — passed with no warnings.

## Explicitly out of scope until a new specification

- Raw-text or multimodal input.
- Claims of semantic understanding, causal reasoning, planning, broad world modeling, or general intelligence.
- GPU acceleration, energy comparisons, or scale superiority.
- Replacing the fixed v1 artifacts or silently changing the v1 dataset, grammar, gates, or historical extension results.
