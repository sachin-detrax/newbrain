# Architecture Experiment B — Transition Model and Goal-Directed Planning

**Specification:** `architecture-experiment-b-transition-model-and-goal-directed-planning` / `1.0.0`  
**Status:** frozen before implementation; specification/protocol step complete only  
**Date:** 2026-10-12  
**Generator:** `goal-directed-planning-generator-b-v1`  
**Grammar:** `goal-directed-planning-grammar-b-v1`  
**Protocol:** [Experiment B protocol v1.0](../protocols/transition-model-and-goal-directed-planning-experiment-b-protocol-v1.0.md)

**Predecessors:** [v1 design](../../testable-design-v1.md),
[v2 specification](opaque-interactive-structural-discovery-v2.md),
[v2 protocol](../protocols/opaque-interactive-structural-discovery-v2-protocol-v1.0.md),
[architecture-gap report, Experiment B](../research/architecture-gap-research-2026-10-11.md),
[Architecture Experiment A](variable-length-boundary-discovery-experiment-a.md),
and [architecture plan](../plan.md).

## 1. Question, claim, and scope

> What is the smallest state and transition representation that supports reliable planning beyond two steps?

A passing result supports only: within the registered deterministic (and controlled-stochastic) planning task family and the explicit symbolic transition grammar, a bounded symbolic learner B1 can admit transition hypotheses, plan to a declared goal at horizons 4/8/16 within declared budgets, maintain observed-vs-imagined provenance, and transfer to held-out goal/state compositions.

The planning language and search objective are **hand-designed symbolic priors**. The learner must obtain the instantiated transition model, supports, applicability, and plans from data. This does not establish autonomous invention of its grammar, a learned perceptual representation, or superiority over neural approaches.

Experiment B is separate from completed MVP gates M1–M12 and Experiment A. Existing source, v1/v2/MVP/A specifications, datasets, protocols, artifacts, and historical results are read-only for this documentation task. Future implementation uses a separate Experiment B namespace and must preserve historical behavior.

### 1.1 Non-claims

- No raw-perception planning or learned continuous latents.
- No probabilistic belief over hidden-state cardinality.
- No causal intervention or reward/value function learning.
- No multi-agent reasoning or language-conditioned goals.
- No cross-run continual learning or scale/energy superiority.
- No neural dependency or cross-experiment transfer with A1/A3.

## 2. Opaque boundary and allowed supervision

### 2.1 Input and lifecycle

```text
EpisodeEvent = Observation { 
                 run_scope, episode_scope, step_index, 
                 observation: OpaquePacket, 
                 available_actions: [ActionId], 
                 action_costs: [u64], 
                 goal: GoalDescriptor, 
                 horizon_remaining: u16, 
                 previous_terminal: TerminalStatus 
               }
             | ActionResult { 
                 run_scope, episode_scope, step_index, 
                 resulting_observation, cost_paid, terminal 
               }
             | End { run_scope, episode_scope, terminal: TerminalStatus }
```

`OpaquePacket` is an opaque `u64` state identity. State identity is a declared visible identity under the deterministic variant; under the stochastic variant, observable identity is still deterministic given the controlled-stochastic trajectory. `GoalDescriptor` is a declared symbolic predicate class (e.g., "visible-identity = g0").

The runner supplies lifecycle controls `BeginDevelopment` after `N_train` episodes and `Freeze` after `N_dev` further episodes. These phase controls are disclosed supervision. The final `N_final` episodes are analyzed with a frozen learned model.

Allowed information is complete: observation stream, action results, goal descriptor, available-action set, action costs, episode terminal flag, lifecycle controls, the grammar and resource ceilings, and learner-owned raw/derived evidence. No latent labels or evaluator metrics are returned during training or development.

### 2.2 Output

```text
Act = { 
  action, 
  procedure_refs, 
  plan_refs, 
  evidence_refs, 
  hypothesis_refs, 
  optional_prediction 
}

Abstain = { 
  reason, 
  hypothesis_refs, 
  evidence_refs, 
  plan_refs 
}

reason = insufficient_evidence | unreachable_goal | unsupported_by_grammar
       | search_exhausted | resource_limit | conflicting_hypotheses
```

These are mutually exclusive responses. A plan must be recorded before the first action of an episode. A mid-episode replan is allowed only if triggered by an observation that falsifies a prior-plan predicted step.

### 2.3 Evaluator-only data

Seeds, row indices, generator constructors, latent transition tables, gold plans, scoring rules/results, and oracle output stay outside learner memory. The only privileged planner is the explicitly evaluator-only R0 reference.

## 3. Task family and identifiable targets

### 3.1 Deterministic task family

A bounded discrete-state controlled transition system with `k` persistent object identities, `m` opaque actions, and declared operator schemas with preconditions and effects. The symbolic operator shape is disclosed without disclosing operator semantics to the learner.

- **State identity:** Integer opaque identity revealed each step.
- **Goals:** Declared predicate class instantiated per episode; held-out goal predicates for transfer.
- **Horizons:** `H in {4, 8, 16}` steps. Termination on goal, horizon exhaustion, or declared dead-end.

### 3.2 Controlled-stochastic variant

A declared categorical action-slip model where an intended action produces an outcome drawn from a disclosed small finite set with probability `p_slip`. `p_slip`, slip-set shape, and random-draw mechanism are evaluator-only. The slip probability class is a disclosed inductive bias.

All atomic action and state identifiers are fresh across episodes under the primary variant. Under the permutation variant, a disclosed bijection is applied in first-seen order.

## 4. Grammar, search, and representation

`goal-directed-planning-grammar-b-v1` is a finite explicit symbolic grammar for state identity, action schemas, and plans.

### 4.1 Required representation

- **State identity:** Persistent `StateId` and persistent `ObjectId`s with equality and provenance (`observed` or `imagined`).
- **Action schemas:** `ActionSchema = { action_id, declared_cost, parameters, preconditions, effects }`. Preconditions and effects are symbolic predicates/updates over `StateId`.
- **Preconditions and effects:** Disjoint predicate language with equality, membership, and incremental additive updates.
- **Goal / constraint records:** `Goal = { predicate, satisfied_by: Option<StateId> }`.
- **Hypothetical branches:** `PlanBranch = { parent, action, predicted_next_state, cost, uncertainty, source: imagined }`.
- **Model uncertainty:** `TransitionHypothesis = { signature, support, challenges, confidence_class }`. Confidence classes: `deterministic`, `stochastic_declared`, `uncertain`.
- **Observed-vs-imagined provenance:** Every evidence record carries `EvidenceProvenance` in `{observed, derived-symbolic, inferred, imagined, model-proposed, evaluator-only}`.
- **Execution/replanning status:** `PlanStatus` in `{proposed, executing, replanning, completed, abandoned}`.

### 4.2 Frozen candidate stages

| Stage | Operation and eligibility |
| --- | --- |
| W0 | Exact-episode replay baseline; fit/replay only. |
| W1 | Enumerate `(state_identity, action) -> next_state_identity` observed pairs. |
| W2 | Admit deterministic transition hypotheses with support, challenge, and scope checks. |
| W3 | Bounded BFS/A* over admitted transitions, with declared heuristic class. |
| W4 | Tabular/probabilistic transition estimates with slip-mass intervals; MPC search. |
| W5 | Declared-future latent world model stage (`not-enrolled-under-v1.0`). |

## 5. Baselines and variants

| ID | Role | Permitted mechanism |
| --- | --- | --- |
| B0 | required reactive baseline | `W0`: match current state + goal to exact-episode replay. No search. |
| B1 | primary symbolic learner | `W0`-`W3`: admit deterministic transitions, run bounded BFS/A*. |
| B2 | tabular/probabilistic comparator | B1 plus `W4`: categorical transition estimates and MPC search. |
| B3 | declared future world model | `W5`; not enrolled; gated by future amendment. |
| R0 | evaluator-only reference | Read latent transition table and admissible plans; verify scoring. |

B0/B1/B2 share raw inputs, phase controls, freeze, and resource ceilings. B2 must preserve full concrete transition observation history.

### 5.1 Future B3 contract

The current `Cargo.toml` has an empty dependency section. B3 is `not-enrolled-under-v1.0`. A future amendment must specify source/license, architecture, seeds, training data, loss, optimizer budget, decoding/calibration, hardware, dependencies, and fresh final instances. Model proposals enter as `model-proposed` and must pass symbolic admission.

## 6. Partitions, horizons, and transfer

| Phase | Episodes | Allowed use |
| --- | ---: | --- |
| Training | 96 | Generate/admit `W0`-`W2` hypotheses; propose plans. |
| Development | 48 | Admit/reject/narrow hypotheses; freeze preserved. |
| Final | 96 | Frozen inference; all results sealed until complete. |

Final strata:
- `H4/H8/H16`: Horizon-specific goal success (fresh goals/identities).
- `C`: Composition transfer (composed operator schemas not jointly exercised in training).
- `S`: Stochastic transfer (B2 reports; B0/B1 abstain or report `unreachable_goal`).
- `D`: Dead-end control (provably unreachable goals; correct response is honest `unreachable_goal` abstention).
- `R`: Replanning (forced mid-episode replan).

## 7. Goal, plan, and transfer semantics

- **Predicted-step success:** `predicted_state_i == observed_state_i`.
- **Goal success:** Final observed state satisfies goal, plan recorded before first action, actions available, total cost <= budget.
- **Replan honesty:** Triggered by an observation that falsifies a prior-plan predicted step. Silent replan is a leakage event.
- **Imagined-vs-observed audit:** `imagined` branches must carry a parent chain into an `observed` record. Promotion is `falsified-by-leakage`.
- **Uncertainty calibration (B2):** Observed mismatch rate must fall within declared slip-mass bound.
- **Non-replay rule:** Exact episode replay, state-identity constants, and final-trained hypotheses do not count for transfer credit.

## 8. Evidence, abstention, and leakage

- **Provenance:** `observed`, `derived-symbolic`, `inferred`, `model-proposed`, `imagined`, `evaluator-only`.
- **Status:** `supported`, `disputed`, `ambiguous`, `suspended`, `incomplete`, `stale`.
- **Abstention reasons:** `insufficient_evidence`, `unreachable_goal`, `unsupported_by_grammar`, `search_exhausted`, `resource_limit`, `conflicting_hypotheses`.

Forbidden leakage includes generator/source imports, hidden labels/transitions, score feedback, final reuse, cross-variant state, schedule-specific branches, hardcoded transitions, and provenance laundering.

## 9. Resource and implementation boundary

The symbolic generator/search/scorer must be implementable with Rust standard library only.

| Resource | Per episode | Per run |
| --- | ---: | ---: |
| received observations | 64 | 15,360 |
| actions issued | 16 | 3,840 |
| plan nodes expanded | 1,024 | 245,760 |
| transition hypotheses generated | 512 | 122,880 |
| transition comparisons | 2,048 | 491,520 |
| retained hypotheses | 64 | 512 |
| MPC rollouts (B2) | 256 | 61,440 |
| evidence/provenance records | 256 | 61,440 |
| serialized learner artifact bytes | 2 MiB | 512 MiB |

Physical ceilings: one process, one worker, zero GPU, zero network, `120,000,000,000` ns, and `1,073,741,824` bytes per run.

## 10. Hypotheses, non-claims, and freeze

- H1: B1 reaches declared goal-success thresholds at horizons 4/8/16.
- H2: an admitted transition grammar transfers to held-out compositions `C` and controlled-stochastic variant `S`.
- H3: dead-end `D` and replan `R` episodes retain uncertainty honestly.
- H4: B2 passes its calibration + MPC comparator gates without worsening B1's primary gates.

### 10.1 Non-claims

- No autonomous invention of transition grammar or learned perceptual representation.
- No superiority over neural planners or language/text/multimodal planning.
- No causal planning or reward/value function learning.
- No unbounded horizon or stochastic belief models beyond declared slip-mass.
- No cross-run continual learning or scale/energy claim.

This specification and its companion protocol are frozen together as v1.0.0 on 2026-10-12, before any Experiment B implementation or behavioral result. Later corrections require a new version and an explicit comparability statement. The next step is Experiment B implementation; no implementation or experiment result is completed by this specification/protocol step.
