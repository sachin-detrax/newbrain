# Opaque Interactive Structural Discovery v2 Evaluation Protocol

**Protocol version:** `opaque-interactive-structural-discovery-v2-protocol-v1.0`
**Frozen specification:** `opaque-interactive-structural-discovery-v2` v2.0
**Protocol status:** preregistered and frozen before v2 implementation
**Registration date:** 2026-10-11
**Purpose:** T2 evaluation protocol; no Rust implementation is authorized by this document

This document freezes every downstream choice delegated to T2. A v2 run is
valid only when it uses this protocol version byte-for-byte, the v2.0 frozen
specification, and the exact information boundary below. A change to a seed,
episode row, threshold, budget, baseline, report field, or reveal order creates
protocol version `v1.1` or later and cannot be used to reinterpret a run made
under `v1.0`.

## 1. Scope and immutability

The claim tested by this protocol is narrow: a bounded learner can discover an
action-conditioned structural procedure from opaque interaction and transfer
it from discovery contexts to held-out contexts under the v2 grammar.

This protocol does not change any v1 artifact, dataset, evaluator, gate,
grammar, or result. The following paths are read-only inputs to this work:

- `testable-design-v1.md`;
- `docs/superpowers/plans/2026-10-10-sparse-structural-rule-learner-v1.md`;
- `artifacts/v1/` and all recorded reports;
- the v1 grammar `g0-g1-v1`;
- the historical result `validated-v1-narrow-claim`.

The v2 grammar is exactly `opaque-interactive-structural-v2`, with stages
`E0`, `E1`, `E2`, `E3`, and `E4`. No v1 module, v1 report, or v1 result is
modified by a v2 implementation or run.

## 2. Fixed world generator

### 2.1 Hidden evaluator world

The evaluator uses exactly these logical values. They are evaluator notation,
not learner-facing labels:

- contexts: `c0`, `c1`, `c2`, `c3`;
- markers: `m0`, `m1`;
- actions: `a0`, `a1`;
- hidden state: `(context, marker)`;
- success condition: resulting marker is `m1`.

The observation packet is always exactly `[context, marker, context]`. Context
is preserved by every transition. The only changing transition is:

```text
if action = a0 and marker = m0:
  next marker = m1
else:
  next marker = marker
```

After an action, the evaluator returns the mapped packet and terminal status in
this order:

1. return `success` if the resulting marker is `m1`;
2. otherwise return `continue` if the action count for the episode is `1`;
3. otherwise return `failure` after the action count reaches `2` with marker
   `m0`.

An episode starts with action count `0`, has an action ceiling of `2`, and is
reset before the next episode. An abstention causes `incomplete`, performs
`0` environment actions, and ends the episode without a fabricated packet.

### 2.2 Deterministic opaque identifier generation

The learner receives wire identifiers, not the logical values in Section 2.1.
The generator uses unsigned `u64` arithmetic modulo `2^64` and this exact
SplitMix64 step:

```text
state = (state + 0x9E3779B97F4A7C15) mod 2^64
z = state
z = ((z xor (z >> 30)) * 0xBF58476D1CE4E5B9) mod 2^64
z = ((z xor (z >> 27)) * 0x94D049BB133111EB) mod 2^64
output = z xor (z >> 31)
```

For a list of logical labels, `permute(seed, labels)` performs Fisher-Yates
from index `length - 1` down to index `1`. At index `i`, it draws `1`
SplitMix64 output, computes `j = output mod (i + 1)`, and swaps indices `i`
and `j`. The final list position is the wire identifier. The atom stream uses
the world seed. The action stream uses

```text
action_seed = world_seed xor 0xD1B54A32D192ED03
```

The exact seeds are:

| Run variant | World seed | Scope seed | Random-baseline seed |
| --- | ---: | ---: | ---: |
| Primary | `2026101101` | `2026101201` | `2026101301` |
| Symbol/action permutation | `2026101102` | `2026101202` | `2026101302` |

The primary generator produces these evaluator-only wire mappings, which are
recorded for audit but never passed to the learner as logical labels:

```text
atom wire order 0..5: [c1, c3, c2, m1, c0, m0]
action wire order 0..1: [a1, a0]
```

The permutation generator produces:

```text
atom wire order 0..5: [c2, c3, c0, c1, m1, m0]
action wire order 0..1: [a0, a1]
```

These mappings are derived from the seeds and algorithm above; an
implementation must verify them in the evaluator audit. The learner sees only
wire identifiers. `available_actions` is always the `2` wire identifiers in
ascending exact identifier order, so the primary run deliberately exposes the
non-changing `a1` action before `a0` under the fixed fallback order. This is an
evaluator property, not an action label supplied to the learner.

### 2.3 Scope-token generation

For each run, initialize a second SplitMix64 stream with the variant's scope
seed. The first nonzero, unused output is `run_scope`. The next `36` nonzero,
unused outputs are `episode_scope` values in episode-table order. If an output
is `0` or repeats an already issued scope token, discard it and draw the next
output. Scope tokens are opaque exact values and are not derived from logical
context, marker, partition, or episode role.

Each matched predictor run receives the same generated scope values for its
variant. The primary and permutation variants use different scope seeds and
fresh learners.

## 3. Exact partitions and episode order

The schedule is fixed; it is not sampled or shuffled. Every row has action
budget `2`, exactly `2` available actions, and a fresh episode scope. The
logical initial state in this table is evaluator-only. A learner receives only
the resulting opaque packet and transport metadata.

### 3.1 Training partition

Training contains exactly `12` episodes and is the only partition used to
generate `E0`, `E1`, and `E2` candidates.

| Order | Episode | Initial context | Initial marker | Role |
| ---: | --- | --- | --- | --- |
| 1 | `T01` | `c0` | `m0` | discovery support |
| 2 | `T02` | `c1` | `m0` | discovery support |
| 3 | `T03` | `c0` | `m1` | precondition challenge |
| 4 | `T04` | `c1` | `m1` | precondition challenge |
| 5 | `T05` | `c0` | `m0` | repeated support |
| 6 | `T06` | `c1` | `m0` | repeated support |
| 7 | `T07` | `c0` | `m1` | repeated challenge |
| 8 | `T08` | `c1` | `m1` | repeated challenge |
| 9 | `T09` | `c0` | `m0` | discovery support |
| 10 | `T10` | `c1` | `m0` | discovery support |
| 11 | `T11` | `c0` | `m1` | repeated challenge |
| 12 | `T12` | `c1` | `m1` | repeated challenge |

The primary action mapping makes the first canonical fallback action the
non-changing action on `m0`. A learner following the frozen v2 action policy
can therefore observe the unchanged `m0` interaction and then the changing
`m0` interaction within a `2`-action episode. The protocol does not force an
action: if the learner does not obtain the required evidence, the result is
classified using Section 10 rather than repaired by the evaluator.

### 3.2 Development partition

Development contains exactly `8` episodes. It is used only for candidate
admission, fixed challenge checks, and canonical tie-breaking. It cannot add a
new grammar candidate or alter a frozen configuration.

| Order | Episode | Initial context | Initial marker | Role |
| ---: | --- | --- | --- | --- |
| 13 | `D01` | `c0` | `m0` | applicable challenge |
| 14 | `D02` | `c1` | `m0` | applicable challenge |
| 15 | `D03` | `c0` | `m1` | non-applicable challenge |
| 16 | `D04` | `c1` | `m1` | non-applicable challenge |
| 17 | `D05` | `c0` | `m0` | retention check before freeze |
| 18 | `D06` | `c1` | `m0` | retention check before freeze |
| 19 | `D07` | `c0` | `m1` | retention challenge before freeze |
| 20 | `D08` | `c1` | `m1` | retention challenge before freeze |

The learner may ingest returned packets and terminal statuses from `D01`-
`D08`, but may not use a development result to create a new `E0`, `E1`, or
`E2` candidate. Development may admit, reject, narrow, or tie-break an
existing candidate only.

### 3.3 Final partition

The learner is frozen immediately before `F01`. Final contains exactly `16`
episodes. The first `12` are the held-out transformation; the last `4` are
frozen-state retention probes.

| Order | Episode | Initial context | Initial marker | Role |
| ---: | --- | --- | --- | --- |
| 21 | `F01` | `c2` | `m0` | held-out applicable |
| 22 | `F02` | `c3` | `m0` | held-out applicable |
| 23 | `F03` | `c2` | `m1` | held-out non-applicable |
| 24 | `F04` | `c3` | `m1` | held-out non-applicable |
| 25 | `F05` | `c2` | `m0` | held-out applicable |
| 26 | `F06` | `c3` | `m0` | held-out applicable |
| 27 | `F07` | `c2` | `m1` | held-out non-applicable |
| 28 | `F08` | `c3` | `m1` | held-out non-applicable |
| 29 | `F09` | `c2` | `m0` | held-out applicable |
| 30 | `F10` | `c3` | `m0` | held-out applicable |
| 31 | `F11` | `c2` | `m1` | held-out non-applicable |
| 32 | `F12` | `c3` | `m1` | held-out non-applicable |
| 33 | `F13` | `c0` | `m0` | frozen retention applicable |
| 34 | `F14` | `c1` | `m0` | frozen retention applicable |
| 35 | `F15` | `c0` | `m1` | frozen retention non-applicable |
| 36 | `F16` | `c1` | `m1` | frozen retention non-applicable |

The primary held-out transformation is context substitution: `c0` and `c1`
are present in training/development support, while `c2` and `c3` are absent
from both mutable partitions and appear first in `F01`. The observation shape,
action set, marker values, transition rule, packet length, and action budget do
not change.

The secondary held-out transformation uses the permutation variant's complete
atom/action remapping, fresh scopes, and fresh learner. It uses the same
`36`-row logical schedule and all `16` final rows. It is a new interaction run,
not a zero-shot test and not a license to reuse primary procedures.

## 4. Evaluator leakage boundary

The learner process receives exactly the v2 `StepInput` fields:

```text
run_scope
episode_scope
step_index
observation.atoms
available_actions
remaining_action_budget
previous_terminal
```

The learner process never receives or imports:

- any logical label from Section 2.1;
- either world seed, scope seed, or random-baseline seed;
- the partition name, episode role, episode table row, or initial hidden state;
- the logical-to-wire mapping, transition table, target marker, or expected packet;
- a pre-action applicability label, score, reward, or baseline result;
- the evaluator's controller or evaluator-only module;
- final results before the corresponding decision record is serialized.

The evaluator and learner communicate through the declared input/output seam.
The evaluator may know every hidden value, but it may not write hidden values
into learner-owned state, call learner candidate generation with expected
outputs, or expose a filesystem, environment variable, shared-memory region,
network channel, or callback containing evaluator state. The learner process
has no network access and reads no protocol or evaluator files during a run.

The evaluator records a decision before applying an action. Only after that
record exists does it return `StepResult`. In mutable training/development,
the learner may ingest that result. In final mode, result ingestion can append
an immutable trace record but cannot mutate procedures, hypotheses, ordering,
configuration, or action selection. Final scores and evaluator manifests are
released only after all `16` final episodes have ended.

Any prohibited field, call, import, or result replay is `falsified-by-leakage`
and invalidates the affected run before behavioral gates are considered.

## 5. Fixed learner configuration and budgets

The learner uses the frozen action-selection order in v2 Section 10 and the
grammar version `opaque-interactive-structural-v2`. Candidate generation order,
candidate signatures, admission checks, evidence IDs, and tie-breaking are
fixed by v2 Sections 9 and 10 before `T01`.

The maximum live decision count and environment-action count for each complete
run are both `72` (`36` episodes multiplied by `2` action slots). An abstention
uses `1` decision slot and `0` environment-action slots, then ends that
episode. Baselines use the same per-episode and per-run ceilings.

A `query_call` is exactly `1` invocation of the live learner decision seam
(`Learner::step` or `FrozenLearner::step`) with `1` `StepInput`. A query call
is counted before the decision is scored. An `environment_action` is exactly
`1` evaluator application of an `ActionId`; an abstention is not an
environment action. The report field `query_calls` therefore equals the number
of decision records, and `environment_actions` equals the number of `Act`
decisions whose actions were applied.

The following logical ceilings are hard limits. A charge that would cross a
limit fails before the charge and must be recorded.

| Resource | Per decision | Per episode | Per run |
| --- | ---: | ---: | ---: |
| Decision calls | `1` | `2` | `72` |
| Environment actions | `1` | `2` | `72` |
| Candidate generation attempts | `64` | `128` | `512` |
| Structural comparisons | `256` | `512` | `2,048` |
| Active hypotheses | `16` | `16` | `64` |
| Operation steps | `1,024` | `2,048` | `8,192` |
| Retained interaction records | `32` | `32` | `128` |
| Retained candidate/procedure records | `32` | `64` | `128` |

The physical run ceilings are also fixed before execution:

- wall-clock elapsed time: at most `60,000,000,000` nanoseconds per run;
- peak process resident memory: at most `536,870,912` bytes per run;
- process count: exactly `1` learner process and `1` evaluator process;
- worker count: exactly `1` learner worker and `1` evaluator worker;
- GPU devices: `0`;
- network connections: `0`.

These physical ceilings are run-safety limits, not speed, energy, or scale
claims. A missing physical measurement is a resource-report failure, not a
zero value.

## 6. Baselines and matched execution

Each predictor is run independently with a fresh state and the same variant,
scope tokens, episode rows, terminal timing, decision budget, and final freeze
boundary. No predictor may inspect another predictor's trace.

### B0 — exact interaction retrieval

`B0` stores only exact completed learner-owned interactions keyed by
`(observation packet, action)`. It has no variable, equality-role, or
cross-context generalization. On a live input it chooses the first available
action with an exact stored key in canonical action order and predicts the
stored returned packet. If no exact key exists, it emits
`Abstain(no_applicable_procedure)`. It never uses final data before the final
decision and receives no structural candidate store.

Because `c2` and `c3` are absent from training/development, B0's held-out
applicable coverage is expected to be exactly `0/6`. A nonzero value is a
baseline-integrity or leakage failure.

### B1 — seeded random exploration

`B1` has no procedure, hypothesis, or evidence-driven action policy. At each
decision it consumes the next SplitMix64 output from the variant's random-
baseline seed, selects `output mod 2` from the ascending `available_actions`
list, emits `Act` with mode `exploration` and no prediction, and then waits for
the evaluator result. It uses the same action and query ceilings. The exact
seed is `2026101301` for primary and `2026101302` for permutation. No random
seed is selected after seeing a result.

The random baseline is reported as a null comparison. Its per-run result is
not tuned or used to alter a learner gate.

### B2 — fixed evaluator controller/reference

`B2` is evaluator-only and may inspect the hidden logical state and rule. It
selects `a0` for every `m0` state, emits the exact next packet prediction, and
selects `a0` for `m1` with the exact unchanged packet prediction. It uses the
same `2`-action episode ceiling and the same `36`-row schedule, but its hidden
lookup and costs are excluded from learner measurements.

B2 must achieve `6/6` held-out applicable task success and `6/6` exact packet
predictions. Failure means the evaluator world or scoring implementation is
invalid; it is not evidence about learner capability.

## 7. Metrics and scoring denominators

All acceptance arithmetic uses integer counts. Reports store `numerator`,
`denominator`, and `rate_micros`, where
`rate_micros = floor(1,000,000 * numerator / denominator)`. A denominator of
`0` is a protocol error; no metric is silently omitted or treated as `1.0`.

For `F01`-`F12`:

- held-out applicable episodes: exactly `6` rows with initial marker `m0`;
- held-out non-applicable episodes: exactly `6` rows with initial marker `m1`;
- episode success: terminal `success` occurs within at most `2` actions;
- prediction attempt: a `Decision` contains a packet prediction;
- exact prediction: the prediction equals the evaluator-returned packet for
  that action;
- procedure-backed coverage: an applicable episode contains at least `1`
  procedure-mode `Act` with a prediction from an admitted `E2`/`E3` procedure;
- false positive: any prediction attempt in a non-applicable episode, even if
  the predicted packet equals the returned packet;
- abstention-quality success: a non-applicable episode contains `0` packet
  predictions and no procedure-backed action;
- retention success: each of `F13`-`F16` preserves the frozen procedure
  behavior: correct procedure-backed prediction for `F13`/`F14` and no packet
  prediction for `F15`/`F16`.

An action without a prediction is a valid control attempt and contributes to
task success, but it contributes `0` to prediction coverage and `0` to packet
prediction accuracy. An `Abstain` response contributes `0` task success and
does not create a prediction attempt.

## 8. Primary acceptance gates

The primary learner passes only when every gate in this table passes. Gates are
evaluated in table order and are frozen before `T01`.

| Gate | Required result |
| --- | --- |
| P1 protocol integrity | `0` prohibited input fields, `0` evaluator calls from learner code, `0` decision-before-result violations, exactly `36/36` episode rows, and exactly `3/3` primary reruns present |
| P2 deterministic reruns | `3/3` primary reruns have byte-identical deterministic core files and identical core SHA-256 fingerprints |
| P3 structural discovery | at least `1` admitted run-scoped `E3` procedure; at least `2` support interactions; at least `2` distinct support context atoms; `0` E0-only credit for this gate |
| P4 held-out procedure coverage | at least `5/6` held-out applicable episodes have procedure-backed coverage |
| P5 held-out task success | at least `5/6` held-out applicable episodes reach terminal `success` within `2` actions |
| P6 held-out packet accuracy | `100%` exact prediction accuracy on held-out applicable prediction attempts, with at least `5` attempts and at least `5` correct predictions |
| P7 false-positive control | exactly `0/6` held-out non-applicable episodes contain a prediction attempt |
| P8 abstention quality | exactly `6/6` held-out non-applicable episodes satisfy the abstention-quality definition |
| P9 frozen retention | exactly `4/4` retention probes pass, with `0` deleted, rewritten, or lineage-free procedure records |
| P10 evidence traceability | `100%` of referenced procedure, evidence, and hypothesis IDs existed before the decision; `0` orphan references; `0` final evidence records used for final action selection |
| P11 budget honesty | `0` over-limit charges, `0` unrecorded charges, `100%` of budget exhaustion events represented as explicit `incomplete` or unsupported status, and `0` confident predictions after exhaustion |
| P12 physical resource ceiling | `100%` of the `3` primary reruns contain complete resource reports; each is at most `60,000,000,000` nanoseconds and `536,870,912` resident-memory bytes |

P6 deliberately separates prediction accuracy from P4 coverage. A learner
cannot obtain accuracy credit by abstaining on every final episode.

## 9. Secondary acceptance gates

The secondary variant is run with a fresh learner, the permutation seeds, and
the same `36` episode rows. It is not merged with primary evidence.

| Gate | Required result |
| --- | --- |
| S1 permutation structural discovery | at least `1` admitted `E3` procedure with at least `2` support interactions and at least `2` distinct support context atoms |
| S2 permutation transfer | at least `5/6` held-out applicable procedure-backed coverage and at least `5/6` held-out applicable task success |
| S3 permutation accuracy and control | `100%` packet accuracy on at least `5` applicable prediction attempts; exactly `0/6` non-applicable false-positive episodes; exactly `6/6` non-applicable abstention-quality successes |
| S4 permutation retention | exactly `4/4` frozen retention probes pass and `0` lineage breaks occur |
| S5 cross-variant parity | the absolute difference in primary and permutation held-out coverage counts is at most `1`; the absolute difference in held-out task-success counts is at most `1`; both variants meet `100%` packet accuracy on their required attempts |
| S6 baseline integrity | B0 has exactly `0/6` held-out applicable coverage in both variants; B2 has exactly `6/6` held-out applicable task success and exact packet predictions in both variants; B1 has `3/3` deterministic reruns per variant |
| S7 secondary resource completeness | `3/3` permutation reruns contain complete resource reports and each satisfies the same `60,000,000,000`-nanosecond and `536,870,912`-byte ceilings |

Secondary gates describe surface-code robustness only. Passing S1-S7 does not
expand the primary semantic or planning claim.

## 10. Failure classifications

Every failed gate and every rejected candidate receives exactly `1` primary
classification and any number of evidence references. The following event
classifications are selected without post-hoc interpretation:

| Classification | Exact condition |
| --- | --- |
| `falsified-by-leakage` | a prohibited field, hidden lookup, final replay, evaluator call, or decision-order violation is observed |
| `falsified-by-determinism` | any required `3`-rerun deterministic core comparison differs |
| `falsified-by-budget` | a logical ceiling is crossed, a charge is missing, or a confident result follows exhaustion |
| `falsified-by-resource-limit` | a physical ceiling is crossed or a required resource field is missing |
| `insufficient-evidence` | a required support or contrast is absent, no budget was exhausted, and no contradictory returned packet exists |
| `search-exhausted` | a required candidate, challenge, or decision could not be evaluated because a frozen logical budget was reached |
| `unsupported-by-grammar` | the observed relation cannot be represented by `E0`-`E4` with all relevant budgets available |
| `incorrect-hypothesis` | an admitted or attempted hypothesis predicts a known returned packet incorrectly, predicts on a required non-applicable contrast, or violates a recorded challenge |
| `falsified-by-held-out-generalization` | P3, P4, P5, or P6 fails after the evidence/search classifications above are assigned |
| `falsified-by-false-positive-control` | P7 fails after no earlier protocol, budget, or evidence classification applies |
| `falsified-by-abstention-quality` | P8 fails after no earlier classification applies |
| `falsified-by-retention` | P9 fails, including a procedure deletion, rewrite, or lineage break |
| `falsified-by-evidence-traceability` | P10 fails due to an orphan, late, or unresolvable reference |
| `secondary-robustness-failed` | a primary run passes but at least `1` secondary gate fails |

Classification precedence is exactly the table order for run-level reporting,
except that `secondary-robustness-failed` is reported only in the secondary
result. A primary result is `validated-v2-narrow-claim` if and only if P1-P12
all pass. A primary result with an earlier failure receives that earlier
classification. Secondary failure does not erase a passing primary result.

The per-candidate distinction is mandatory: missing support/challenge is
`insufficient-evidence`; a reached ceiling is `search-exhausted`; a relation
outside `E0`-`E4` is `unsupported-by-grammar`; a wrong observed consequence is
`incorrect-hypothesis`. These labels may not be collapsed into a generic
failure.

## 11. Deterministic rerun requirements

The runner performs exactly `3` primary reruns and exactly `3` permutation
reruns. Every rerun uses a fresh process and output directory but the same
variant seed, protocol bytes, source/build fingerprint, configuration, and
episode schedule for its variant. B1 uses the fixed variant seed on every
rerun; it does not receive a new random seed.

The following files are deterministic core files and must be byte-identical
within a variant's `3` reruns:

- `learner/decision-trace.jsonl`;
- `learner/interaction-ledger.jsonl`;
- `learner/candidate-events.jsonl`;
- `learner/procedure-records.jsonl`;
- `metrics.json`;
- `acceptance.json`;
- `failures.jsonl`;
- `logical-resource-report.json`.

Wall-clock samples, host strings, process IDs, output paths, and filesystem
timestamps are excluded from deterministic core files and appear only in the
physical resource report. A run fails P2 or its secondary equivalent if any
core file differs, any fingerprint differs, or any rerun omits a required file.

## 12. Artifact and resource-report format

Each rerun writes this exact top-level layout:

```text
<run-directory>/
  protocol.md
  run-manifest.json
  learner/
    decision-trace.jsonl
    interaction-ledger.jsonl
    candidate-events.jsonl
    procedure-records.jsonl
  metrics.json
  acceptance.json
  failures.jsonl
  logical-resource-report.json
  physical-resource-report.json
  baselines/
    b0/metrics.json
    b1/metrics.json
    b2/metrics.json
  evaluator-only/
    world-manifest.json
    episode-manifest.json
    score-input.json
  sha256sums.txt
```

`protocol.md` is an exact copy of this protocol. `run-manifest.json` contains
the protocol version, variant, rerun index in `1..3`, all `6` seed fields,
partition counts `12`, `8`, `16`, total episode count `36`, decision/action
ceilings `72`, grammar version, source/build fingerprints, and the final
classification.

The evaluator-only directory is not mounted in the learner process. It may be
published for audit after the run, but it is never included in a learner input
stream. `world-manifest.json` contains logical-to-wire mappings and seeds;
`episode-manifest.json` contains the `36` hidden initial states and scopes;
`score-input.json` contains returned packets, terminal statuses, and expected
metric denominators. No evaluator-only file may be copied into learner-owned
state.

### 12.1 Canonical serialization

All JSON is UTF-8 with `LF` line endings, no insignificant whitespace, sorted
object keys, and arrays in the declared event order. JSON numbers are integers;
ratios use the Section 7 fields rather than floating-point values. JSONL has
exactly `1` object per line and ends with a final `LF`. Identifiers are encoded
as unsigned decimal strings when their `u64` value is not safe in a consumer's
native integer type.

`sha256sums.txt` lists every file path in bytewise ascending order followed by
its lowercase SHA-256 digest. It excludes itself. No deterministic artifact
contains an absolute path, process ID, current time, or random UUID.

### 12.2 Required resource measurements

`logical-resource-report.json` records used and limit values for every row in
the Section 5 logical-budget table, plus `query_calls`, `environment_actions`,
`candidate_count`, `procedure_count`, `ledger_record_count`, and
`ledger_size_bytes`.

The logical report uses these fixed counting rules: `candidate_count` is the
number of candidate records generated, merged, rejected, or budget-rejected;
`procedure_count` is the number of admitted procedure versions retained;
`ledger_record_count` is the number of JSONL ledger objects, including final
immutable trace records; and `ledger_size_bytes` is the UTF-8 byte count of
`learner/interaction-ledger.jsonl` including its final `LF`. A generated
candidate that is later merged still counts as `1` generated candidate and
`0` additional retained procedure records.

`physical-resource-report.json` contains these exact integer fields:

```text
wall_clock_ns
cpu_time_ns
peak_process_memory_bytes
process_count
worker_count
gpu_count
network_connection_count
```

It also contains exactly `4` stage objects named `canonicalization`,
`retrieval`, `admission`, and `revalidation`. Each stage object contains
`wall_clock_ns`, `cpu_time_ns`, `peak_process_memory_bytes`, and
`operation_count`. Stage boundaries and operation units are fixed as follows:

| Stage | Start and end | `operation_count` unit |
| --- | --- | --- |
| `canonicalization` | from receipt of a `StepInput` or `StepResult` until its exact packet/identifier form is stored | `1` per received `StepInput`, `1` per received `StepResult`, and `1` per terminal `incomplete` record |
| `retrieval` | from the start of applicable-procedure lookup until action-selection order returns an action or abstention | `1` per admitted procedure examined for the current decision |
| `admission` | from the first candidate admission gate until the last gate result for that candidate | `1` per evaluated admission gate, including a gate that rejects the candidate |
| `revalidation` | from receipt of each final or retention probe result until its metric record is appended | `1` per final decision/result pair and `1` per frozen retention check |

The physical measurement window starts immediately before the `T01` reset and
ends after the `F16` result and final trace flush. It excludes compilation,
process startup, protocol-file hashing, and `sha256sums.txt` generation. The
stage windows are nested within that run window. `wall_clock_ns` uses a
monotonic clock, `cpu_time_ns` uses process CPU time, and
`peak_process_memory_bytes` uses the operating system's resident-memory
high-water measurement for the reported predictor process. The report records
the declared host OS, architecture, CPU model, Rust/compiler version, and
execution mode `cpu-only` as environment metadata; those strings are not used
in deterministic-core comparison.

The proposed learner and B0/B1 report their own learner-side logical and
physical measurements. B2 reports only evaluator reference measurements and
is excluded from learner resource totals.

## 13. Information-reveal order

The following order is part of the protocol and cannot be changed:

1. Before implementation and execution, the researcher receives this protocol
   and the frozen v2 specification. The learner process receives neither file
   through its runtime boundary.
2. The evaluator selects the already frozen variant seed, derives the opaque
   mappings, creates `run_scope`, and prepares the fixed `36`-row schedule.
3. For each episode, the evaluator creates its fresh `episode_scope`, resets
   the hidden state, and sends the first `StepInput` with `previous_terminal`
   equal to `none`.
4. The learner emits exactly `1` `Decision`. The evaluator serializes that
   decision before reading any expected result or applying the action.
5. For an `Act`, the evaluator applies the hidden transition and returns exactly
   `1` `StepResult`. For an `Abstain`, the evaluator returns no packet and
   closes the episode with `incomplete`.
6. Only after the decision record exists may a mutable training/development
   learner ingest the returned packet and terminal status. A frozen final
   learner may append a trace record but may not mutate any decision state.
7. If the result is `continue`, the evaluator sends the next `StepInput` with
   `step_index = 1`, `remaining_action_budget = 1`, and
   `previous_terminal = continue`. No result is sent after a terminal status.
8. After `D08`, the runner calls `freeze` exactly `1` time and verifies the
   frozen fingerprint before sending `F01`.
9. The evaluator completes `F01`-`F16`, records every final result, and only
   then releases scores, acceptance gates, and evaluator-only manifests to the
   report writer. No final score is returned to the learner.

The evaluator seed, hidden state, expected packet, partition label, terminal
score, and baseline result are never revealed before the corresponding
decision record. The order is identical for the learner and all baselines;
only B2 has the explicitly declared evaluator-only hidden-world access.

## 14. Protocol execution decision

The protocol is complete and executable without adding a label, seed, episode,
budget, threshold, baseline rule, failure category, serialization rule, or
reveal step. Any implementation choice not specified here is limited to
storage and programming-language details that cannot change the observable v2
contract. No Rust code is added as part of T2.
