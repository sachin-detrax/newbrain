# newbrain — Sparse Structural Rule Learner

A Rust-based, restartable structural-learning runtime. It learns bounded structural
rules from opaque observation/action protocols — no raw text, no neural front end.

At every step the learner receives an opaque observation and a set of available
actions, then must commit one of:

```text
Act { action, optional_prediction, procedure_refs, evidence_refs, hypothesis_refs }
Abstain { reason, procedure_refs, evidence_refs, hypothesis_refs }
```

The decision is recorded **before** the environment result is revealed. Results are
ingested into an interaction ledger, which drives evidence, hypothesis, candidate,
and procedure updates. State is saved as a versioned, loadable format, so a learner
can be frozen, checkpointed, and resumed across processes.

## Design invariants

- Decision-before-result interaction ordering
- Bounded structural induction over repeated equality roles
- Explicit abstention with audit references
- Run/episode scope confinement
- Freeze-before-final-evaluation behavior
- Environment-independent runtime proven in multiple deterministic environments

## Status (2026-10-12)

- MVP-8: bounded verification and packaging — complete
- Experiment A: variable-length boundary discovery — implemented, A1/A3 results validated
- Experiment B: transition model and goal-directed planning — in progress

## Repository layout

| Path | Contents |
|---|---|
| `src/framework/` | Environment-independent learner primitives (`Learner`, `FrozenLearner`, ledger-based learners) |
| `src/runtime.rs` | Restartable run controller, persistence, environment adapter contract |
| `src/v2/` | Benchmark-specific v2 evaluator bridge |
| `src/experiment_a/` | Variable-length boundary discovery experiment |
| `src/experiment_b/` | Transition model and goal-directed planning experiment |
| `tests/` | 233 tests across protocol, lifecycle, learner-boundary, and CLI suites |
| `docs/` | Build plan, task ledger, experiment specs and protocols |
| `src/bin/` | `run_mvp`, `run_v1`, `run_v2`, `run_a` experiment runners |

Generated experiment outputs (`artifacts/`) and the local knowledge graph
(`graphify-out/`) are git-ignored.

## Getting started

```sh
cargo build
cargo test
```

The v2 benchmark evaluator bridge is exposed as `sparse_structural_rule_learner::Evaluator`
for external benchmark harnesses.

## Key documents

- [docs/plan.md](docs/plan.md) — framework evolution build plan
- [docs/task.md](docs/task.md) — task ledger
- [docs/specs/](docs/specs/) — experiment specifications
- [docs/protocols/](docs/protocols/) — experiment protocols
- [docs/research/](docs/research/) — architecture-gap research
