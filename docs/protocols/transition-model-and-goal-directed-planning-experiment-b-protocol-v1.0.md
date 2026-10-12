# Architecture Experiment B Evaluation Protocol

**Protocol ID:** `transition-model-and-goal-directed-planning-experiment-b-protocol-v1.0`
**Protocol version:** `1.0.0`
**Frozen specification:** [`transition-model-and-goal-directed-planning-experiment-b.md`](../specs/transition-model-and-goal-directed-planning-experiment-b.md) v1.0.0
**Generator version:** `goal-directed-planning-generator-b-v1`
**Grammar version:** `goal-directed-planning-grammar-b-v1`
**Protocol status:** preregistered and frozen before Experiment B implementation
**Registration date:** 2026-10-12
**Implementation constraint:** symbolic variants use Rust standard library only
**Predecessor:** [`variable-length-boundary-discovery-experiment-a-protocol-v1.0.md`](variable-length-boundary-discovery-experiment-a-protocol-v1.0.md)

This protocol fixes the seeds, episode schedule, partitions, scoring rules,
thresholds, resource limits, rerun procedure, artifact format, failure
classifications, and information-reveal order for Architecture Experiment B.
No implementation result exists at registration. A change to a seed, generator
rule, episode order, threshold, baseline, resource limit, artifact field, or reveal
step creates protocol version `1.1.0` or later and cannot reinterpret a run
made under v1.0.0.

The frozen claims validated-v1, validated-v2, validated-mvp, validated-a1, and
validated-a3 are read-only predecessors. This protocol authorizes only the
later Experiment B implementation; it does not authorize changes to existing
source or frozen artifacts.

## 1. Protocol purpose and result vocabulary

The primary decision is whether the explicit symbolic transition representation
and bounded planner in variant B1 are sufficiently accurate, auditable, and
tractable at horizons 4, 8, and 16 before deepening or adding a learned front
end or world model.

The required symbolic variants are:

- **B0:** reactive one-step reuse baseline;
- **B1:** primary symbolic learner with bounded BFS/A* search; and
- **B2:** tabular/probabilistic comparator with MPC search.

**B3**, a learned latent world model, is a declared future comparator. It is
not an enrolled v1.0 implementation because this repository has no neural
dependency. No B3 result may be substituted, simulated, or inferred from B1/B2.
A future B3 run requires a new implementation manifest and may use this
protocol only after a versioned registration addendum.

The complete result states are:

| State | Meaning |
| --- | --- |
| `validated-b1-narrow-claim` | B1 passes all required B1 gates in a valid run set |
| `validated-b2-comparator` | B2 passes its comparator gates without invalidating B1 |
| `negative-b1-narrow-result` | B1 run set is valid but one or more behavioral/resource gates fail |
| `negative-b2-comparator-result` | B2 run set is valid but one or more comparator gates fail |
| `no-result` | The run set is not valid or is incomplete; no behavioral claim is made |
| `invalidated-by-leakage` | A prohibited evaluator fact entered a learner decision or state |

A valid negative result is not converted into a positive result by selecting a
more favorable episode subset, threshold, seed, rerun, or baseline.

## 2. Exact generator and seed configuration

### 2.1 Generator algorithm

`goal-directed-planning-generator-b-v1` uses unsigned `u64` arithmetic modulo
`2^64` and the exact SplitMix64 transition:

```text
next(state):
  state = (state + 0x9E3779B97F4A7C15) mod 2^64
  z = state
  z = ((z xor (z >> 30)) * 0xBF58476D1CE4E5B9) mod 2^64
  z = ((z xor (z >> 27)) * 0x94D049BB133111EB) mod 2^64
  return z xor (z >> 31)
```

All additions and multiplications wrap modulo `2^64`. `record_state` is
initialized by:

```text
record_state = world_seed
                 xor partition_tag
                 xor (episode_ordinal * 0xD1B54A32D192ED03 mod 2^64)
```

The partition tags and global episode ordinals are fixed as follows:

| Partition | `partition_tag` | Global ordinal range |
| --- | ---: | ---: |
| training | `0x545241494E000002` | `1..96` |
| development | `0x4445560000000002` | `97..144` |
| final | `0x46494E0000000002` | `145..240` |

The generator consumes `next(record_state)` in the declared constructor order.
For an index into a finite list of length `m`, it uses `next mod m`. A fresh
identity is `next(record_state)` unless it is zero or already used by that
episode; in either case it draws again.

The per-episode 8-step constructor order is:

1. choose the horizon class from the row schedule;
2. choose the task configuration (number of objects, operator variants);
3. allocate fresh state identities;
4. select the start state and goal predicate for the row's declared layout;
5. generate the latent transition trajectory under the operator semantics;
6. (stochastic variant only) draw per-step slip events from the categorical model;
7. serialize wire observations and action-result packets; and
8. validate reachability, dead-end, and replan invariants before delivery.

The learner receives only the resulting wire `StateId` and `ActionId` values,
the opaque scope tokens, and the lifecycle controls. Latent operator semantics
and slip draws remain evaluator-only.

### 2.2 Seeds

These values are fixed decimal `u64` values:

| Field | Primary variant | Symbol-permutation variant |
| --- | ---: | ---: |
| `world_seed` | `2026101203` | `2026101204` |
| `scope_seed` | `2026101213` | `2026101214` |
| `baseline_seed` | `2026101223` | `2026101224` |
| `rerun_seed` | `2026101233` | `2026101234` |

`world_seed` drives episode generation. `scope_seed` drives scope tokens.
`baseline_seed` is reserved for any declared baseline tie-break. `rerun_seed`
is recorded in manifests to prove that a rerun did not silently draw a new
configuration.

The permutation variant applies a deterministic bijection to generated state
and action IDs in first-seen logical-allocation order. The permutation state
is reset only at the start of a run.

Scope tokens use the same SplitMix64 transition initialized with `scope_seed`.
The first nonzero unused output is `run_scope`; subsequent nonzero unused
outputs are assigned as `episode_scope` in schedule order.

## 3. Exact partition schedule

Episodes are delivered in table order. The runner must not shuffle, balance,
drop, duplicate, or replace a row.

### 3.1 Training episodes `T001`–`T096`

| Episodes | Count | Horizon | Layout | Required role |
| --- | ---: | --- | --- | --- |
| `T001`–`T032` | 32 | 4 | simple chains | primary relation support |
| `T033`–`T064` | 32 | 8 | branching paths | composition support |
| `T065`–`T096` | 32 | 16 | complex graphs | search and budget support |

Training includes at least four episodes for each primary operator schema.

### 3.2 Development episodes `D001`–`D048`

| Episodes | Count | Horizon | Layout | Required role |
| --- | ---: | --- | --- | --- |
| `D001`–`D016` | 16 | 4 | admission challenges | hypothesis narrowing |
| `D017`–`D032` | 16 | 8 | rejection challenges | challenge support |
| `D033`–`D048` | 16 | 16 | budget challenges | search optimization |

Development episodes use fresh identities not used in training.

### 3.3 Final episodes `F001`–`F096`

Final episodes begin immediately after the learner freeze:

| Episodes | Count | Stratum | Horizon | Held-out property |
| --- | ---: | --- | --- | --- |
| `F001`–`F016` | 16 | `H4` | 4 | fresh goals and identities |
| `F017`–`F032` | 16 | `H8` | 8 | fresh goals and identities |
| `F033`–`F048` | 16 | `H16` | 16 | fresh goals and identities |
| `F049`–`F060` | 12 | `C` | 8/16 | composition transfer |
| `F061`–`F072` | 12 | `S` | 8/16 | stochastic transfer |
| `F073`–`F084` | 12 | `D` | 8/16 | dead-end control |
| `F085`–`F096` | 12 | `R` | 8/16 | forced replanning |

Final episodes use fresh identities not used in mutable partitions.

## 4. Enrollment, variants, and matched execution

Each variant is executed independently with a fresh process and fresh state.

### B0 — reactive baseline

B0 memorizes `(state_identity, goal_descriptor) -> action` tuples from training.
On a later episode, it replays the single matching action for an identical
visible identity and goal; otherwise it abstains. It may not plan more than
one step ahead and may not query any transition hypothesis.

### B1 — primary symbolic learner

B1 implements stages `W0`–`W3` of the specification. It admits deterministic
transition hypotheses and runs bounded BFS/A* search with a declared integer
heuristic class. It must abstain honestly on `S` stochastic episodes.

### B2 — tabular/probabilistic comparator

B2 implements B1 plus stage `W4`. It maintains integer-count-based categorical
transition estimates with declared slip-mass intervals and runs MPC search
with uncertainty-aware action selection.

### B3 — future world model

B3 is not run under v1.0.0. It is reported as `not-enrolled-under-v1.0`.

### R0 — evaluator-only oracle

R0 reads the latent generator manifest and returns admissible plans. It
validates generator/scorer consistency only and is not a learner.

## 5. Analysis, goal, plan, and error scoring

### 5.1 Integer metric representation

Every metric is stored as:

```text
{
  numerator: unsigned integer,
  denominator: unsigned integer,
  rate_micros: floor(1_000_000 * numerator / denominator)
}
```

A denominator of zero is a protocol error.

### 5.2 Primary metrics

- **Goal success:** Final observed state satisfies goal, plan recorded before
  first action, actions available, total cost <= budget.
- **Multi-step prediction accuracy:** `sum(predicted_step_success) / sum(predicted_steps)`.
- **Plan-cost efficiency:** `sum(plan_cost) / sum(oracle_min_cost)` for reachable goals.
- **Replan honesty:** `replan_triggered_by_falsification / total_replan_events`.
- **Imagined-promotion zero:** `imagined_nodes_promoted_to_observed / total_episodes`.
- **Dead-end honesty:** `honest_abstentions / total_D_episodes`.
- **Composition transfer:** `goal_successes / total_C_episodes`.
- **Stochastic transfer:** `goal_successes / total_S_episodes` (B2 primary).
- **Uncertainty calibration:** `episodes_within_slip_bound / episodes_with_stochastic_prediction`.

### 5.3 Evidence and resource metrics

The scorer reports pre-decision reference resolution, orphan references,
candidates generated/merged/rejected/admitted, retained hypotheses, logical
artifact byte count, and physical wall-clock/memory measurements.

## 6. Preregistered acceptance gates

### 6.1 B1 primary gates

| Gate | Required result |
| --- | --- |
| `B1-P1` protocol integrity | exactly `240/240` episodes delivered; `0` prohibited fields; `0` evaluator calls; `0` final replays |
| `B1-P2` deterministic reruns | `3/3` primary reruns have byte-identical core files and identical SHA-256 digests |
| `B1-P3` horizon-4 success | micro success rate `>= 14/16` (87.5%) over `F001`–`F016` |
| `B1-P4` horizon-8 success | micro success rate `>= 12/16` (75.0%) over `F017`–`F032` |
| `B1-P5` horizon-16 success | micro success rate `>= 8/16` (50.0%) over `F033`–`F048` |
| `B1-P6` prediction accuracy | micro accuracy `>= 0.85` over all executed plans |
| `B1-P7` plan efficiency | aggregate efficiency `<= 1.50` (plan cost <= 1.5x oracle minimum) |
| `B1-P8` dead-end honesty | `>= 10/12` honest abstentions on `D` with two witness alternatives |
| `B1-P9` replan honesty | imagined-promotion exactly `0`; replan-honesty rate `>= 10/12` |
| `B1-P10` composition transfer | micro success rate `>= 9/12` (75.0%) over `C` |
| `B1-P11` stochastic honesty | `>= 10/12` honest abstentions on `S` |
| `B1-P12` evidence traceability | `100%` pre-decision references resolve; `0` orphan references; `0` final latent facts in state |
| `B1-P13` budget honesty | `0` silent empty plans; `0` over-limit charges; every exhaustion is explicit |
| `B1-P14` physical resources | all `3` reruns within `120,000,000,000` ns and `1,073,741,824` bytes |

### 6.2 B0 baseline integrity gates

| Gate | Required result |
| --- | --- |
| `B0-C1` matched inputs | exactly the same `240` episodes and opaque variants as B1 |
| `B0-C2` no planning | exactly `0` plan nodes expanded |
| `B0-C3` deterministic run | `3/3` reruns byte-identical in deterministic core files |
| `B0-C4` leakage/resources | same zero-leakage and physical ceilings as B1 |

### 6.3 B2 comparator gates

| Gate | Required result |
| --- | --- |
| `B2-C1` B1 parity | pass B1 protocol, boundary, negative, evidence, and resource gates |
| `B2-C2` calibration | micro calibration rate `>= 0.80` |
| `B2-C3` MPC utility | on `S`, B2 goal success `>= B1` success + `2/12` (16.6%) |
| `B2-C4` comparator transfer | B2 transfer on `C` and horizons not worse than B1 by more than `3/96` aggregate |
| `B2-C5` boundedness | at most `64` retained hypotheses and no logical budget overrun |

## 7. Failure classifications and precedence

1. `falsified-by-leakage`
2. `falsified-by-protocol`
3. `falsified-by-determinism`
4. `falsified-by-resource-limit`
5. `falsified-by-budget`
6. `insufficient-evidence`
7. `search-exhausted`
8. `unsupported-by-grammar`
9. `incorrect-hypothesis`
10. `falsified-by-goal-horizon-generalization`
11. `falsified-by-structural-transfer`
12. `falsified-by-dead-end-control`
13. `falsified-by-replan-honesty`
14. `falsified-by-evidence-traceability`
15. `falsified-by-abstention-policy`
16. `comparator-mpc-failed`

## 8. Aggregate, rerun, and deterministic comparison policy

### 8.1 Required runs

For each of B0, B1, and B2, execute exactly `3` primary-seed reruns and `3`
symbol-permutation reruns (18 learner runs + 6 R0 checks). The primary B1
claim uses the three primary-seed B1 reruns.

### 8.2 Deterministic core files

These files must be byte-identical across all three reruns:

```text
learner/episode-events.jsonl
learner/plan-trace.jsonl
learner/action-events.jsonl
learner/transition-records.jsonl
learner/model-records.jsonl
metrics.json
acceptance.json
failures.jsonl
logical-resource-report.json
```

`model-records.jsonl` is present and empty for B0/B1.

### 8.3 Aggregate arithmetic

Aggregate counts are the sums of numerators and denominators across the three
primary reruns. Exact-zero gates must be zero on every rerun.

## 9. Artifact layout and canonical serialization

Each rerun writes exactly this layout:

```text
<run-directory>/
  protocol.md
  specification.md
  run-manifest.json
  learner/
    episode-events.jsonl
    plan-trace.jsonl
    action-events.jsonl
    transition-records.jsonl
    model-records.jsonl
    evidence-records.jsonl
  metrics.json
  acceptance.json
  failures.jsonl
  logical-resource-report.json
  physical-resource-report.json
  evaluator-only/
    generator-manifest.json
    latent-trajectory-manifest.json
    score-input.json
  sha256sums.txt
```

The evaluator-only directory is released only after the final analysis decision.
Canonical serialization is UTF-8, LF, sorted keys, and integer-only numbers.

## 10. Leakage audit

The runner verifies that the learner process received only `run_scope`,
`episode_scope`, `step_index`, `observation`, `available_actions`, `action_costs`,
`goal`, `horizon_remaining`, `previous_terminal`, `BeginDevelopment`, and `Freeze`.

Prohibited:
- reading generator source, evaluator-only files, or hidden transition tables;
- receiving ground-truth plans or operator semantics;
- silent replan without a triggering observation;
- promoting `imagined` or `model-proposed` nodes to `observed` evidence;
- replaying a final episode before its plan is recorded.

Any violation is `falsified-by-leakage`.

## 11. Information-reveal order

1. Researcher receives protocol and spec; learner does not.
2. Runner selects variant, mode, rerun; derives wire identities.
3. Per episode: deliver `Observation`, learner emits `Act` or `Abstain`, runner
   serializes decision, runner applies action and delivers `ActionResult`.
4. After `T096`, runner issues `BeginDevelopment` once.
5. After `D048`, runner calls `freeze` once and records fingerprint.
6. Frozen learner receives final episodes; each `Act` serialized before the
   corresponding `ActionResult`.
7. After final flush, runner releases evaluator-only manifest and runs scorer.
8. Writes acceptance, failures, resource reports, and hashes.

## 12. No-result and negative-result policy

### 12.1 No-result

The outcome is `no-result` when fewer than three reruns are complete, a
required file is missing, a version/seed/schedule mismatch occurs, or a
physical measurement is missing.

### 12.2 Valid negative result

A complete, leakage-free, deterministic run group that fails one or more
behavioral gates is a valid negative result. The report must include all
numerators, denominators, thresholds, and failure classifications.

## 13. Decision matrix

| Observed result | Decision | Required interpretation |
| --- | --- | --- |
| B1 passes `P1`–`P14`, B2 passes comparator gates | **Deepen symbolic transition grammar and bounded planning** | Representation adequate; add grammar/search depth with new spec. |
| B1 passes behavioral but fails only search/budget limits | **Deepen symbolic search/indexing first** | Representation adequate; tractability is the bottleneck. |
| B1 passes horizon-4/8 but fails horizon-16 or composition `C` | **Add hybrid state representation seam before adding planner depth** | State aliasing or combinatorial state space dominates. |
| B1 fails stochastic `S` honesty but B2 passes `S` calibration | **Keep symbolic planner; promote tabular/probabilistic layer** | Uncertainty representation is the gap. |
| B1 fails horizon generalization and B3 meets its target | **Add learned world-model comparator seam** | Only representation boundary implicated; symbolic planner remains audit layer. |
| B1 fails horizon generalization and B3 unavailable or fails | **Revisit representation objective and task design** | Identify state-aliasing or operator mismatch first. |
| B1 is invalid/no-result | **Do not make an architecture decision** | Repair implementation or register new protocol version. |
| B1 passes but only via exact replay or leakage | **Invalidate** | Classify leakage; do not credit transfer. |

## 14. Protocol completion audit

This v1.0 protocol fixes:
- generator ID `goal-directed-planning-generator-b-v1`;
- primary and permutation seeds;
- `96/48/96` episode partitions and `240` total rows;
- row order and stratum schedule;
- B0, B1, B2, future B3, and evaluator-only R0 roles;
- goal success, prediction accuracy, efficiency, honesty, and resource metrics;
- B1, B0, and B2 thresholds;
- failure classification precedence;
- exactly three reruns per variant and seed mode;
- deterministic core artifacts and canonical serialization;
- no-result and valid-negative policies;
- the architecture decision matrix; and
- the reveal and leakage order.

The next step is Experiment B implementation. It must not modify this protocol,
the frozen specification, the bounded MVP, or existing v1/v2/Experiment-A artifacts.
