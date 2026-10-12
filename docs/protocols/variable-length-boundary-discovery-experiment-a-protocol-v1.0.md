# Architecture Experiment A Evaluation Protocol

**Protocol ID:** `variable-length-boundary-discovery-experiment-a-protocol-v1.0`
**Protocol version:** `1.0.0`
**Frozen specification:** [`variable-length-boundary-discovery-experiment-a.md`](../specs/variable-length-boundary-discovery-experiment-a.md) v1.0.0
**Generator version:** `variable-stream-generator-a-v1`
**Grammar version:** `variable-stream-structure-a-v1`
**Protocol status:** preregistered and frozen before Experiment A implementation
**Registration date:** 2026-10-12
**Implementation constraint:** symbolic variants use Rust standard library only
**Predecessor:** [`architecture-gap-research-2026-10-11.md`](../research/architecture-gap-research-2026-10-11.md)

This protocol fixes the seeds, record schedule, partitions, scoring rules,
thresholds, resource limits, rerun procedure, artifact format, failure
classifications, and information-reveal order for Architecture Experiment A.
No implementation result exists at registration. A change to a seed, generator
rule, row order, threshold, baseline, resource limit, artifact field, or reveal
step creates protocol version `1.1.0` or later and cannot reinterpret a run
made under v1.0.0.

The bounded MVP, v1 artifacts, v2 artifacts, and historical result remain
read-only. This protocol authorizes only the later Experiment A implementation;
it does not authorize changes to existing source or frozen artifacts.

## 1. Protocol purpose and result vocabulary

The primary decision is whether the explicit symbolic representation and
segmentation layer in variant A1 is sufficiently accurate, auditable, and
tractable to deepen before adding a learned front end.

The required symbolic variants are:

- **A0:** fixed-position/equality baseline;
- **A1:** explicit symbolic segmentation and alignment search; and
- **A3:** A1 plus evidence-backed library promotion.

**A2**, a learned representation/parser feeding symbolic induction, is a
declared future comparator. It is not an enrolled v1.0 implementation because
this repository has no neural dependency and no separate learned-front-end
source artifact. No A2 result may be substituted, simulated, or inferred from
A1/A3. A future A2 run requires a new implementation manifest and may use this
protocol only after a versioned registration addendum.

The complete result states are:

| State | Meaning |
| --- | --- |
| `validated-a1-narrow-claim` | A1 passes all required A1 gates in a valid run set |
| `validated-a3-comparator` | A3 passes its comparator gates without invalidating A1 |
| `negative-a1-narrow-result` | A1 run set is valid but one or more behavioral/resource gates fail |
| `negative-a3-comparator-result` | A3 run set is valid but one or more comparator gates fail |
| `no-result` | The run set is not valid or is incomplete; no behavioral claim is made |
| `invalidated-by-leakage` | A prohibited evaluator fact entered a learner decision or state |

A valid negative result is not converted into a positive result by selecting a
more favorable record subset, threshold, seed, rerun, or baseline.

## 2. Exact generator and seed configuration

### 2.1 Generator algorithm

`variable-stream-generator-a-v1` uses unsigned `u64` arithmetic modulo `2^64`
and this exact SplitMix64 transition:

```text
next(state):
  state = (state + 0x9E3779B97F4A7C15) mod 2^64
  z = state
  z = ((z xor (z >> 30)) * 0xBF58476D1CE4E5B9) mod 2^64
  z = ((z xor (z >> 27)) * 0x94D049BB133111EB) mod 2^64
  return z xor (z >> 31)
```

Protocol constructor names are aliases for the specification's notation:
`Echo = E`, `Quasi = Q`, and `Noise = N`. They are evaluator-only names and
never learner-facing labels.

All additions and multiplications wrap modulo `2^64`. `record_state` is
initialized by:

```text
record_state = world_seed
                 xor partition_tag
                 xor (record_ordinal * 0xD1B54A32D192ED03 mod 2^64)
```

The partition tags and global record ordinals are fixed as follows:

| Partition | `partition_tag` | Global ordinal range |
| --- | ---: | ---: |
| training | `0x545241494E000001` | `1..64` |
| development | `0x4445560000000001` | `65..96` |
| final | `0x46494E0000000001` | `97..160` |

The generator consumes `next(record_state)` in the declared constructor order.
For an index into a finite list of length `m`, it uses `next mod m`. A fresh
atom is `next(record_state)` unless it is zero or already used by that record;
in either case it draws again. A record is regenerated only when a declared
invariant fails, using the next redraw counter in the same deterministic
substream. Redraw count is recorded in the evaluator-only manifest and is not
learner-visible.

The constructor order is:

1. choose the layout and requested payload lengths from the row schedule;
2. draw prefix noise length and allocate its atoms;
3. allocate each latent segment's fresh endpoint and payload atoms in latent
   segment order;
4. draw inter-segment noise lengths and allocate their atoms;
5. draw suffix noise length and allocate its atoms;
6. concatenate the constructed spans in the row's output order; and
7. validate the latent segmentation, equality, no-accidental-parse, and
   maximum-length invariants.

Noise lengths in every `1..3` range are `1 + (next(record_state) mod 3)`.
For the six mutable `Reordered Compose2` rows, the emitted payload-length pairs
are fixed by row: `(1,2)`, `(2,3)`, `(3,5)`, `(2,1)`, `(3,2)`, `(5,3)`.
The first three are ascending and the last three descending; the corresponding
latent slot order is retained in the evaluator manifest. For development the
pairs are `(1,2)`, `(2,3)`, `(2,1)`, `(3,2)`. For a final Reordered Compose2
row, lengths are selected by the held-out cycle and the segment permutation is
the next Fisher-Yates permutation of `[0,1]`; for final Reordered Compose3 it is
the first three Fisher-Yates draws over `[0,1,2]`. The final reorder rows contain
four Compose2 records followed by four Compose3 records; their generated
permutations must differ from canonical order in at least one record of each
subgroup, otherwise the deterministic redraw rule advances the same substream.

For `Echo`, the endpoint atom is allocated once and emitted at both endpoints.
For `Quasi`, the two endpoints are independently allocated. Payload atoms and
noise atoms are otherwise fresh. No atom is reused across records in a run.

For an `Optional` row, the slot is present on odd local row indices and absent
on even local row indices. The requested payload length is still drawn on an
absent row so later draws do not depend on the presence branch. The same rule
applies to the optional final rows `F029`-`F036`.

The learner receives only the resulting wire `AtomId` values, the two opaque
scope tokens, and the lifecycle control at the registered phase boundary.
Logical constructor labels and latent spans remain evaluator-only.

### 2.2 Seeds

These values are fixed decimal `u64` values:

| Field | Primary variant | Symbol-permutation variant |
| --- | ---: | ---: |
| `world_seed` | `2026101201` | `2026101202` |
| `scope_seed` | `2026101211` | `2026101212` |
| `baseline_seed` | `2026101221` | `2026101222` |
| `rerun_seed` | `2026101231` | `2026101232` |

`world_seed` drives record generation. `scope_seed` drives scope tokens.
`baseline_seed` is reserved for any declared baseline tie-break; A0, A1, and
A3 have no random action or search choice and must not consume it. `rerun_seed`
is recorded in manifests to prove that a rerun did not silently draw a new
configuration; it is not used to perturb a deterministic run.

The primary and symbol-permutation variants each use the same 160-record
logical schedule below, with fresh learners and fresh scope tokens. The
permutation variant applies a deterministic bijection to generated atom IDs:

```text
wire_atom = next(permutation_state)
permutation_state = world_seed xor 0x94D049BB133111EB
```

The next unused, nonzero output is assigned in first-seen logical-allocation
order. The permutation state is reset only at the start of a run. The learner
does not receive the permutation or logical allocation order.

Scope tokens use the same SplitMix64 transition initialized with `scope_seed`.
The first nonzero unused output is `run_scope`; subsequent nonzero unused
outputs are assigned as `stream_scope` in record schedule order. Scope tokens
are independent of the record's latent layout and are not passed through the
atom permutation.

### 2.3 Fixed protocol constants

```text
training_records       = 64
development_records    = 32
final_records          = 64
total_records          = 160
unambiguous_final      = 52
ambiguous_final        = 12
final_transfer_records = 28
final_negative_records = 8
final_optional_records = 8
final_reorder_records  = 8
```

The maximum generated stream length is `48`; the maximum final stream length
and every other row-specific length are generated by the fixed constructor and
validated before delivery. No stream length is sent before its last `Token`.

The exact logical ceilings, repeated from the specification, are:

| Resource | Per stream | Per run |
| --- | ---: | ---: |
| received atoms | 48 | 7,680 |
| span-boundary pairs considered | 1,024 | 163,840 |
| alignment operations | 4,096 | 655,360 |
| segmentation hypotheses retained | 64 | 512 |
| relation candidates generated | 512 | 81,920 |
| relation comparisons | 2,048 | 327,680 |
| active hypotheses | 16 | 128 |
| library entries (A3) | 8 | 32 |
| evidence/provenance records | 256 | 40,960 |
| serialized learner artifact bytes | 2 MiB | 64 MiB |

Physical ceilings are one learner process, one worker, zero GPU devices, zero
network connections, at most `120,000,000,000` elapsed nanoseconds, and at most
`1,073,741,824` resident-memory bytes. A charge that would cross a logical or
physical ceiling fails before the charge and is recorded.

## 3. Exact partition schedule

Records are delivered in table order. The runner must not shuffle, balance,
drop, duplicate, or replace a row. The learner receives no row ID, partition
name, or stratum label. The evaluator manifest records those values for audit.

### 3.1 Training rows `T001`-`T064`

The `length_cycle` is the repeating sequence `{1, 2, 3, 5}`. `fresh_pair`
means the endpoint pair is new within the row and across all earlier rows.

| Rows | Count | Layout | Lengths/order | Required role |
| --- | ---: | --- | --- | --- |
| `T001`-`T016` | 16 | one Echo | `length_cycle`, four repeats | primary relation support |
| `T017`-`T032` | 16 | one Echo with prefix/suffix/interior distractors | `length_cycle`, four repeats | boundary search and distractor support |
| `T033`-`T038` | 6 | Compose2 | segment payloads from `{1,2,3,5}` | composition support |
| `T039`-`T044` | 6 | Reordered Compose2 | `(1,2)`, `(2,3)`, `(3,5)`, `(2,1)`, `(3,2)`, `(5,3)` | reorder support |
| `T045`-`T052` | 8 | Optional | present/absent alternation, lengths `{1,2,3,5}` | optional support |
| `T053`-`T060` | 8 | Negative Quasi/Noise | same nuisance layout/lengths as `T001`-`T008`, with E replaced by Q | false-positive challenge |
| `T061`-`T064` | 4 | Ambiguous | `(1,2)`, `(2,1)`, `(1,2)`, `(2,1)` | ambiguity support |

Within each row range, the constructor consumes rows in ascending order. Echo
endpoint allocation is independent for every row, including rows with the same
length. Training includes at least four records at each primary payload length.

### 3.2 Development rows `D001`-`D032`

Development may admit, reject, narrow, or tie-break candidates generated from
the mutable evidence already available. It may not create a new grammar stage,
change a range, or alter a threshold.

| Rows | Count | Layout | Lengths/order | Required role |
| --- | ---: | --- | --- | --- |
| `D001`-`D008` | 8 | one Echo with distractors | `{1,2,3,5}` twice | boundary/admission challenges |
| `D009`-`D012` | 4 | Compose2 | lengths `{1,2,3,5}` | composition challenges |
| `D013`-`D016` | 4 | Reordered Compose2 | `(1,2)`, `(2,3)`, `(2,1)`, `(3,2)` | order and false-merge challenges |
| `D017`-`D020` | 4 | Optional | present, absent, present, absent | optional challenge |
| `D021`-`D024` | 4 | Negative Quasi/Noise | same nuisance layout/lengths as `D001`-`D004`, with E replaced by Q | false-positive challenge |
| `D025`-`D032` | 8 | Ambiguous | alternating `(1,2)` and `(2,1)` pairs | ambiguity honesty challenge |

The development records are generated with fresh atoms not used in training.

### 3.3 Final rows `F001`-`F064`

Final rows begin immediately after the learner freeze and are delivered in this
fixed order:

| Rows | Count | Stratum | Layout | Held-out property |
| --- | ---: | --- | --- | --- |
| `F001`-`F008` | 8 | `L` length transfer | one Echo with low nuisance | payload lengths `4,6,7,8`, two each |
| `F009`-`F016` | 8 | `L` length transfer | one Echo with high nuisance | payload lengths `4,6,7,8`, two each |
| `F017`-`F028` | 12 | `M` motif composition | Compose3 | triples `(4,6,7)`, `(6,7,8)`, `(7,8,4)`, `(8,4,6)`, repeated three times |
| `F029`-`F036` | 8 | `O` optional transfer | Optional with Echo lengths `4,6,7,8` | held-out lengths and presence/absence |
| `F037`-`F040` | 4 | `R` reorder transfer | Reordered Compose2 | pairs `(4,6)`, `(6,7)`, `(7,8)`, `(8,4)`; new independent order |
| `F041`-`F044` | 4 | `R` reorder transfer | Reordered Compose3 | triples `(4,6,7)`, `(6,7,8)`, `(7,8,4)`, `(8,4,6)`; new independent order |
| `F045`-`F052` | 8 | `N` negative controls | same nuisance layout/lengths as `F001`-`F008`, with E replaced by Q | no Echo relation; length/distractor matched |
| `F053`-`F064` | 12 | `A` ambiguity | Ambiguous, alternating `(1,2)` and `(2,1)` pairs | exactly two admissible boundary sets |

Final records use fresh atoms not used in either mutable partition. The length
assignment within each length-bearing row range is cyclic in the order
`4,6,7,8`, starting at the first row of the range. Composition and reorder
parameters are fixed by the row ordinal and the generator substream; the
construction trace is stored evaluator-only.

For every negative paired row, the generator reuses only the reference row's
nuisance-length vector and replaces every E endpoint pair with unequal Q
endpoints. It allocates fresh atoms and does not reuse the reference stream.
This makes the negative length/distractor control exact without exposing the
pairing to the learner.

## 4. Enrollment, variants, and matched execution

Each variant is executed independently with a fresh process, fresh learner
state, the same schedule, variant seeds, phase boundary, and resource ceilings.
No variant may inspect another variant's state, trace, metrics, or final result.

### A0 — fixed-position/equality baseline

A0 consumes the full stream and emits exactly one span `[0,n)`. It may:

- compare atoms at fixed absolute positions;
- record equality between positions in the whole stream;
- memorize a complete training/development stream; and
- emit a whole-stream equality relation only when its fixed positions match.

Operationally, A0 stores each mutable stream's exact length and equality
signature `[(i,j) where i < j and atom[i] == atom[j]]`. On a later stream it
uses a stored signature only when the length is identical, the signature is
nonempty, and the later signature is exactly equal at the same absolute
positions. It never aligns a different length, infers a span start/end, or
treats an unseen pair as an equality. This is the complete baseline rule, not a
tunable implementation choice.

A0 may not enumerate a start/end pair, split the stream, align lengths,
skip/insert/delete spans, use an optional or reorder operator, or promote a
library. Its boundary set is always empty. This exact rule makes its expected
internal-boundary recall zero while still allowing its exact/equality metrics to
be reported.

### A1 — explicit symbolic segmentation/alignment

A1 implements stages `V0`-`V4` of the frozen specification. It may enumerate
contiguous span pairs, equality positions, bounded payload lengths, optional
groups, and finite reorderings. It uses exact comparisons and deterministic
canonical order only. It has no delimiter heuristic, semantic label, neural
model, embedding, similarity function, hand-written Echo detector, or
evaluator-world import.

### A3 — symbolic segmentation plus library learning

A3 implements A1 and stage `V5`. It uses exactly the library admission rule in
the specification and must preserve every library item's concrete expansion.
Library promotion is optional for a particular record, but the A3 run must
report whether no candidate met the promotion gate. A3 does not get credit for
the presence of a library unless it also passes the utility and transfer gates.

### A2 — future learned-front-end comparator

A2 is not run under v1.0.0. A future A2 implementation must consume the same
raw token stream, not latent spans; disclose source revision, dependencies,
weights, training data/procedure, model seed, and model version; and emit
`model-proposed` hypotheses that enter the same symbolic admission and evidence
scoring path. A human-authored parser for the known generator is prohibited.
The future A2 comparator target is at least A1's boundary precision and recall
gates plus a preregistered improvement of `>= 0.05` in either boundary recall
or transfer rate without a worse-than-A1 false-merge, false-split, or negative
false-positive gate. This target is recorded now but cannot be evaluated until
A2 is separately enrolled.

### R0 — evaluator-only oracle

R0 reads the latent generator manifest and returns one admissible latent
segmentation. It validates generator/scorer consistency only. R0 is not a
learner and is excluded from learner costs, evidence, and architecture claims.

## 5. Analysis, boundary, and error scoring

### 5.1 Integer metric representation

Every metric is stored as:

```text
{
  numerator: unsigned integer,
  denominator: unsigned integer,
  rate_micros: floor(1_000_000 * numerator / denominator)
}
```

A denominator of zero is a protocol error. No metric is omitted, set to one,
or interpreted as a perfect score when its denominator is zero.

### 5.2 Boundary sets

For a stream of length `n`, only integers in `{1,...,n-1}` are internal
boundaries. Start `0` and end `n` are not scored as discovered boundaries.

For each unambiguous final record, the evaluator has one gold boundary set `G`.
`G` contains only relation-span endpoints and maximal residual-span endpoints;
it does not contain hidden constructor boundaries between adjacent nuisance
atoms. A negative `N` record has one maximal residual span and therefore no
internal gold boundaries. For the selected learner analysis, `B` is its internal
boundary set. If the learner abstains, `B` is empty for unconditional recall
arithmetic and the abstention is also counted separately; abstention is never
silently credited as a parse.

```text
true_boundary = B ∩ G
false_split   = B \ G
false_merge   = G \ B
```

For the `A` stratum, the evaluator has exactly two admissible full-coverage
gold parses and boundary sets `G1` and `G2`, as defined by the specification.
If the learner returns a selected hypothesis, the scorer chooses the `Gi`
minimizing `(false_split + false_merge, false_split, false_merge,
canonical_boundary_order)` for diagnostic scoring only. If the learner returns
both distinct gold alternatives, or abstains with `ambiguous_segmentation` and
references two distinct learner witness hypotheses, it receives ambiguity-
honesty credit but no unique-parse credit. A blanket ambiguity abstention with
no two witnesses and any third alternative are invalid for that record.

### 5.3 Boundary metrics

Boundary metrics are computed over the `52` unambiguous final records. The
ambiguous `A` records are excluded from these primary boundary denominators:

```text
boundary_precision = sum(true_boundary) /
                     (sum(true_boundary) + sum(false_split))
boundary_recall    = sum(true_boundary) /
                     (sum(true_boundary) + sum(false_merge))
false_split_rate   = records_with_false_split / 52
false_merge_rate   = records_with_false_merge / 52
```

`selected_parse_rate` is the number of unambiguous final records with a
selected analysis divided by `52`. A no-selected-analysis record contributes
false merges for all of its gold relation/residual boundaries and zero true
boundaries in unconditional recall, so abstention cannot inflate recall or
transfer. `false_merge_rate` includes these omissions; a separate
`selected_only_false_merge_rate` is reported diagnostically.

`exact_boundary_set_rate` is the number of unambiguous records with `B == G`
divided by `52`.

### 5.4 Relation and transfer metrics

The final transfer denominator is `28` (`F001`-`F028`). A record is a
`relation_transfer_success` only if the selected pre-freeze relation:

1. is a `V4` relation with mutable support and at least two fresh endpoint
   pairs and three mutable payload lengths;
2. selects all and only the relation span(s) required by the final latent
   structure;
3. has zero false merges and zero false splits for the selected relation spans;
4. recognizes endpoint equality and the bounded variable payload without a
   final atom constant; and
5. has complete evidence and provenance references.

The following are reported separately:

| Metric | Denominator |
| --- | ---: |
| `L_length_transfer_rate` | 16 |
| `M_composition_transfer_rate` | 12 |
| `OM_optional_correct_rate` | 8 |
| `R_reorder_correct_rate` | 8 |
| `N_primary_false_positive_rate` | 8 |
| `A_ambiguity_honesty_rate` | 12 |

On `N`, a false positive is a selected primary Echo relation or library
relation unsupported by the stream. A negative record with no selected
relation is correct whether it has a supported noise segmentation or an
explicit non-applicable abstention. On `O`, presence and absence must both be
handled correctly; returning an Echo relation for an absent optional segment is
a false positive for that record. On `R`, the relation is correct only when
the returned independent spans and order abstraction match the registered
reordering condition.

### 5.5 Evidence and resource metrics

The scorer reports:

- percentage of referenced IDs that existed before the decision;
- percentage of final references that resolve to learner-owned evidence;
- orphan-reference count;
- final-data-before-decision count;
- candidate records generated, merged, rejected, and admitted;
- alignment comparisons and span pairs considered;
- peak retained hypotheses, relation records, and library entries;
- logical artifact byte count; and
- physical wall-clock, CPU, resident-memory, process, worker, GPU, and network
  measurements.

All counts are integers and preserve their raw numerator/denominator.

### 5.6 Library description-cost calculation

The A3 utility metric uses a fixed integer description cost. Count one unit for
each span start, span end, `Bind`, `Repeat`, `Payload`, `Skip`, `Match`,
`InsertOptional`, `DeleteOptional`, `Optional`, `Unordered`, relation reference,
and exact observed atom literal in a canonical candidate expansion. Evidence
references, candidate IDs, and learner-assigned record IDs are not description
units.

For the development composite set `D009`-`D016`, compute:

```text
expanded_cost = sum(cost of every relation after recursively expanding
                    each library reference into its V4 definition)
library_cost  = sum(cost of each retained V4 definition once)
                + sum(1 + number of bound arguments for each library use)
utility       = (expanded_cost - library_cost) / expanded_cost
```

The scorer stores the integer numerator `expanded_cost - library_cost` and
denominator `expanded_cost`; floating-point values are not used for a gate. A
repeated relation expansion is counted as removed when a `LibraryId` replaces
one V4 relation node in at least two distinct composite analyses. A3-C2 passes
when `utility >= 20%` using integer cross-multiplication (`5 * numerator >=
expanded_cost`) or when at least two such expansions are removed.

## 6. Preregistered acceptance gates

Gates are evaluated in the order shown. The thresholds were set before any
Experiment A implementation result and cannot be altered after a run.

### 6.1 A1 primary gates

| Gate | Required result |
| --- | --- |
| `A1-P1` protocol integrity | exactly `160/160` records delivered in order; `0` prohibited fields; `0` evaluator calls/imports; `0` final-result replay events; all required files present |
| `A1-P2` deterministic reruns | `3/3` primary reruns have byte-identical deterministic core files and identical SHA-256 digests |
| `A1-P3` boundary precision | micro precision `>= 0.90` over `52` unambiguous final records |
| `A1-P4` boundary recall | micro recall `>= 0.90` over `52` unambiguous final records |
| `A1-P5` boundary error control | false-merge rate `<= 0.10` and false-split rate `<= 0.10` |
| `A1-P6` selected analysis coverage | `>= 48/52` unambiguous final records have a selected analysis |
| `A1-P7` structural transfer | `>= 24/28` total transfer successes, with `>= 13/16` in `L` and `>= 9/12` in `M` |
| `A1-P8` optional/reorder behavior | `>= 6/8` optional correct and `>= 6/8` reorder correct |
| `A1-P9` negative control | exactly `0/8` primary false positives on `N` |
| `A1-P10` ambiguity honesty | `>= 10/12` `A` records return exactly the two distinct gold alternatives or abstain with `ambiguous_segmentation` while referencing two distinct witness hypotheses; blanket ambiguity abstentions and third alternatives are failures |
| `A1-P11` evidence traceability | `100%` pre-decision references resolve; `0` orphan references; `0` final latent facts in learner state |
| `A1-P12` abstention/budget honesty | `0` silent empty-parse defaults; `0` over-limit charges; every exhaustion is explicit; no confident analysis after exhaustion |
| `A1-P13` physical resources | all `3` reruns report complete measurements; each is at most `120,000,000,000` ns and `1,073,741,824` resident-memory bytes, with one process/worker, zero GPU, zero network |

A1 passes only if every `A1-P` gate passes in every required primary rerun,
except that behavioral numerator/denominator gates use the preregistered
three-rerun aggregate defined in Section 8. A rerun with a protocol or resource
failure cannot be removed from the aggregate.

### 6.2 A0 baseline integrity and comparator report

A0 is not expected to pass A1 behavioral gates. Its required integrity gates
are:

| Gate | Required result |
| --- | --- |
| `A0-C1` matched inputs | exactly the same `160` logical records and opaque variants as A1 |
| `A0-C2` no segmentation | exactly `0` enumerated internal span-boundary candidates and empty boundary sets |
| `A0-C3` deterministic run | `3/3` reruns byte-identical in deterministic core files |
| `A0-C4` leakage/resources | same zero-leakage and physical ceilings as A1 |

A0's boundary, transfer, negative, and abstention metrics are comparison
points. They are not suppressed because they are expected to be low.

### 6.3 A3 comparator gates

A3 must first pass the A1 protocol, boundary, negative, evidence, and resource
gates with the same thresholds. It then has these additional gates:

| Gate | Required result |
| --- | --- |
| `A3-C1` library admission | at least `1` library entry with `>=6` concrete uses, `>=4` records, `>=3` payload lengths, and both one-relation and two-relation learner-parsed layouts |
| `A3-C2` library utility | at least `20%` reduction in canonical expanded relation description cost on the development composite set, or at least `2` repeated relation expansions removed |
| `A3-C3` library provenance | every library expansion resolves to its concrete inferred supports and challenges; `0` hidden labels or surface parser calls |
| `A3-C4` comparator transfer | using the three-rerun aggregate, A3 transfer is not more than `6/84` below A1 (equivalent to `2/28` per rerun average); `L` is `>=36/48`, `M` is `>=27/36`, and `N` false positives remain `0/24` |
| `A3-C5` library boundedness | at most `32` library entries and no logical budget overrun |

If A3 has no eligible library, it is a valid negative comparator result with
classification `insufficient-evidence` or `unsupported-by-grammar`, as
appropriate. It is not a no-result.

## 7. Failure classifications and precedence

Every invalid run, failed gate, candidate rejection, and unresolved decision
receives one primary classification and zero or more evidence references.
Classification precedence for run-level reporting is exactly this table order:

| Precedence | Classification | Exact condition |
| ---: | --- | --- |
| 1 | `falsified-by-leakage` | prohibited evaluator value, hidden import, final replay, or decision-order violation |
| 2 | `falsified-by-protocol` | wrong seed, row order, generator/grammar version, count, input shape, or artifact field |
| 3 | `falsified-by-determinism` | required deterministic core differs across reruns |
| 4 | `falsified-by-resource-limit` | a declared physical ceiling is crossed; missing measurements are `no-result` under Section 12 |
| 5 | `falsified-by-budget` | logical ceiling crossed, charge missing, or analysis continued after exhaustion |
| 6 | `insufficient-evidence` | support/challenge/length/layout requirement absent and no contradiction or ceiling explains it |
| 7 | `search-exhausted` | required span, alignment, candidate, or admission check was not reached because of a logical ceiling |
| 8 | `unsupported-by-grammar` | required relation cannot be represented by declared grammar with budgets available |
| 9 | `incorrect-hypothesis` | selected relation/boundary predicts an impossible equality, invalid span, wrong optional state, wrong order, or third ambiguity parse |
| 10 | `falsified-by-boundary-generalization` | A1-P3, A1-P4, A1-P5, or A1-P6 fails after earlier classifications are absent |
| 11 | `falsified-by-structural-transfer` | A1-P7 or A1-P8 fails after earlier classifications are absent |
| 12 | `falsified-by-false-positive-control` | A1-P9 fails after earlier classifications are absent |
| 13 | `falsified-by-ambiguity-honesty` | A1-P10 fails after earlier classifications are absent |
| 14 | `falsified-by-evidence-traceability` | A1-P11 fails without earlier classification |
| 15 | `falsified-by-abstention-policy` | A1-P12 fails without earlier classification |
| 16 | `comparator-library-failed` | A3-C1 through A3-C5 fail while A1 remains valid |

Per-candidate reporting must preserve the distinction between missing evidence,
search exhaustion, unsupported grammar, and an incorrect hypothesis. A generic
`failed` label is not sufficient.

## 8. Aggregate, rerun, and deterministic comparison policy

### 8.1 Required runs

For each of A0, A1, and A3, execute exactly:

- `3` primary-seed reruns; and
- `3` symbol-permutation reruns.

That is `18` learner runs, plus `6` R0 evaluator-only checks. The primary A1
claim uses the three primary-seed A1 reruns. The permutation is a secondary
robustness report and must not rescue a failed primary claim.

Every rerun starts a fresh process and output directory. It uses the same
protocol bytes, source/build fingerprint, variant seed, scope seed, schedule,
grammar, configuration, and baseline seed. No rerun samples a new seed.

### 8.2 Deterministic core files

Within each `(variant, symbol-mode)` group, these files must be byte-identical
across all three reruns:

```text
learner/stream-events.jsonl
learner/analysis-trace.jsonl
learner/candidate-events.jsonl
learner/relation-records.jsonl
learner/library-records.jsonl
metrics.json
acceptance.json
failures.jsonl
logical-resource-report.json
```

`library-records.jsonl` is present and empty for A0/A1. Host name, process ID,
absolute path, timestamps, physical samples, and output directory are excluded
from deterministic core files and appear only in physical metadata.

### 8.3 Aggregate arithmetic

For each integer metric, aggregate counts are the sums of numerators and
denominators across the three primary reruns. A gate such as `24/28` therefore
means at least `72/84` in the three-rerun aggregate, with no rerun omitted. For
the exact-zero false-positive and leakage gates, every rerun must be zero.

The result report includes each rerun separately and the aggregate. A single
passing rerun cannot mask two failures. For physical limits, every rerun must
pass individually.

### 8.4 Permutation result

The permutation group uses the same gate thresholds except that its status is
secondary. It reports parity against primary A1:

- absolute difference in boundary precision and recall is at most `0.05`;
- absolute difference in total transfer count is at most `2/28`;
- both groups retain `0/8` negative false positives; and
- both groups retain `>=10/12` ambiguity honesty.

Failure of the permutation group is `secondary-robustness-failed` and does not
erase a valid primary result.

## 9. Artifact layout and canonical serialization

Each rerun writes exactly this layout:

```text
<run-directory>/
  protocol.md
  specification.md
  run-manifest.json
  learner/
    stream-events.jsonl
    analysis-trace.jsonl
    candidate-events.jsonl
    relation-records.jsonl
    library-records.jsonl
    evidence-records.jsonl
  metrics.json
  acceptance.json
  failures.jsonl
  logical-resource-report.json
  physical-resource-report.json
  evaluator-only/
    generator-manifest.json
    latent-record-manifest.json
    score-input.json
  sha256sums.txt
```

The evaluator-only directory is not mounted into the learner process. It may
be released only after the final analysis decision of the run is serialized.
It contains the logical seed, row labels, latent spans, admissible ambiguity
sets, redraw counts, wire mapping, and scorer inputs. No evaluator-only file is
copied into `learner/`.

`protocol.md` and `specification.md` are exact copies of the frozen documents.
`run-manifest.json` contains protocol/specification/generator/grammar versions,
variant, symbol mode, rerun index, all seed fields, `64/32/64` partition
counts, `160` total records, resource limits, source/build fingerprints, and
the final classification.

Canonical serialization is UTF-8, `LF`, no insignificant whitespace, sorted
object keys, declared array/event order, and integer-only numbers. JSONL has
one object per line and a final `LF`. `u64` identifiers are decimal strings.
No deterministic artifact contains an absolute path, current timestamp,
process ID, host value, or random UUID.

`sha256sums.txt` lists every other file in bytewise ascending path order with a
lowercase SHA-256 digest. A missing or extra required artifact is a protocol
failure.

### 9.1 Logical resource report

`logical-resource-report.json` records used and limit values for every Section
9 resource in the specification, plus:

```text
record_count
atom_count
span_pair_count
alignment_operation_count
candidate_generated_count
candidate_rejected_count
relation_admitted_count
library_entry_count
evidence_record_count
serialized_learner_bytes
```

Candidate counts include generated, merged, rejected, and budget-rejected
candidate records. An admitted relation count counts retained versions, not
unique signatures. Byte counts include final `LF` bytes.

### 9.2 Physical resource report

The report contains these integer fields:

```text
wall_clock_ns
cpu_time_ns
peak_process_memory_bytes
process_count
worker_count
gpu_count
network_connection_count
```

The measurement window begins immediately before `T001` reset and ends after
`F064` analysis, final trace flush, and learner artifact close. It excludes
compilation, process startup, protocol hashing, and SHA-256 generation.

## 10. Leakage audit

Before behavioral gates, the runner verifies that the learner process received
only `run_scope`, `stream_scope`, `atom`, `End`, and the two fixed lifecycle
controls `BeginDevelopment` and `Freeze` at their registered boundaries. Final
decision records must precede any score or latent manifest access.

The following are prohibited:

- reading generator source, evaluator-only files, score input, or latent spans;
- receiving total length, partition/stratum, row ID, motif label, target
  relation, expected boundaries, or final score;
- hardcoding the generator's Echo/Quasi/Optional/Reordered constructor as an
  implementation parser;
- using another variant's state, traces, or final metrics;
- converting a `model-proposed`, `library-derived`, or imagined alignment into
  `observed` evidence;
- replaying a final stream or score before its analysis is recorded; or
- changing a configuration or gate after seeing a run result.

For A0, its fixed whole-stream rule is declared comparator behavior and must
remain exposed in its manifest. It may not be used inside A1 or A3.

Any violation is `falsified-by-leakage` and invalidates the affected variant
and run group. The runner must preserve enough process and trace evidence to
show the audit outcome.

## 11. Information-reveal order

The following sequence is frozen:

1. Before implementation, the researcher receives this protocol and the frozen
   specification. The learner runtime receives neither file through its live
   input boundary.
2. The runner chooses one of the fixed variants, seed modes, and rerun indices,
   creates a fresh learner process, derives wire atoms and scope tokens, and
   prepares the evaluator-only schedule.
3. For each `T` or `D` record, the evaluator sends one `Token` at a time in
   order. It sends `End` only after the last token. The learner does not receive
   the expected length or latent construction.
4. The learner emits exactly one `StreamAnalysis`. The runner serializes that
   decision before opening score input or exposing the evaluator's latent
   boundary for the record.
5. While a mutable stream is being delivered, the learner may retain the
   observed opaque tokens, derive exact symbolic records, and search within the
   declared budget. It may not append evaluator latent boundaries or scores.
6. After `T064` analysis, the runner sends `BeginDevelopment` exactly once.
   After `D032` analysis, it calls `freeze` exactly once and records a frozen
   learner fingerprint before delivering `F001`.
7. The frozen learner receives all final token events and `End` events. Each
   analysis is serialized before its score, latent parse, or final metric is
   made available to the report writer. Final input may be retained only as an
   immutable trace and cannot become support.
8. After `F064` analysis and final trace flush, the evaluator releases the
   evaluator-only manifest and computes metrics and gates.
9. The runner writes acceptance, failures, resource reports, and hashes. No
   final result is fed back into the learner or used to reorder a later record.

The order is identical for A0, A1, and A3. R0 alone may inspect evaluator
structure, and its output never enters learner state.

## 12. No-result and negative-result policy

### 12.1 No-result

The outcome is `no-result` when any of these occur:

- fewer than three required reruns are complete for any enrolled A0, A1, or A3
  variant/seed-mode group;
- a required source, specification, protocol, generator, or artifact file is
  missing or cannot be hashed;
- a generator version, grammar version, seed, row count, or schedule differs
  from this protocol;
- output is malformed so the declared metrics cannot be computed;
- a physical measurement required for a gate is missing; or
- a build/dependency environment cannot produce the enrolled variant and no
  valid run was made.

No-result is not evidence that symbolic segmentation succeeded or failed. The
report must state the missing stage and may not report behavioral rates from a
partial run as final results.

### 12.2 Valid negative result

A complete, leakage-free, deterministic run group that fails one or more
behavioral gates is a valid negative result. The report must include:

- all gate numerators, denominators, and thresholds;
- all primary failure classifications in precedence order;
- per-stratum and per-rerun metrics;
- the exact resource/search exhaustion evidence, where applicable;
- A0 comparison metrics; and
- a statement that the narrow claim was not validated.

The negative result remains frozen. The researcher may write a later protocol
or architecture decision record, but may not rerun v1.0 under changed gates and
call it the same experiment.

### 12.3 A2 non-enrollment

The absence of A2 in this repository is neither a negative A2 result nor a
reason to lower A1 thresholds. A2 is reported as `not-enrolled-under-v1.0`.

## 13. Decision matrix

The matrix is applied after the valid A1/A3 result is complete. It governs the
next architecture experiment only; it does not authorize implementation in this
protocol.

| Observed result | Decision | Required interpretation |
| --- | --- | --- |
| A1 passes `P1`-`P13`, A3 passes its comparator gates, and A1/A3 costs stay within limits | **Deepen symbolic segmentation and library induction** | The symbolic representation is adequate for this bounded stream family; add grammar/search depth with a new spec, not a silent v1.0 extension. Do not add A2 solely because it is available. |
| A1 passes boundary/transfer/negative/ambiguity gates but fails only search, candidate, alignment, or resource limits | **Deepen symbolic search/indexing first** | The representation has evidence of adequacy, while tractability is the bottleneck. Report `search-exhausted` or resource classification; do not call this a learned-front-end win. |
| A1 passes transfer and boundary gates but A3 fails utility or increases false positives/merges/splits | **Keep symbolic relations local; defer library learning** | A reusable representation is supported, but library promotion is not. Revisit library typing/utility in a separate protocol. |
| A1 fails boundary/coverage gates with `falsified-by-boundary-generalization`, while a separately enrolled A2 meets its pre-registered comparator target and preserves all provenance/leakage gates | **Add a learned front-end comparator seam** | Only the representation boundary is implicated. Symbolic induction remains the audit layer; A2 must remain model-proposed and uncertain. |
| A1 fails transfer but passes boundary metrics and A2 does not meet its target | **Deepen symbolic relation/alignment grammar** | Segmentation appears adequate; the failure is transfer/composition or grammar expressivity, not evidence for a neural parser. |
| A1 fails boundary and A2 also fails, or A2 is unavailable | **Revisit representation objective and task design** | Do not proceed to raw text or broader neural claims. Determine identifiability, distractor confounding, and grammar mismatch first. |
| A1 is invalid/no-result | **Do not make an architecture decision** | Repair the protocol implementation or register a new protocol version. No behavioral claim is made. |
| A1 passes but only by exact replay, atom constants, final evidence, or evaluator leakage | **Invalidate** | This is not structural discovery. Classify leakage/incorrect hypothesis and do not credit transfer. |

For the A2 row, “meets its target” means at least A1's `0.90` boundary
precision and recall, an improvement of `>=0.05` in boundary recall or total
transfer rate relative to the valid A1 aggregate, no worse than A1's false
merge/split and negative false-positive gates, and complete model provenance.
The A2 comparison must be paired on the same logical schedule and independently
rerun; an external benchmark result cannot fill this row.

## 14. Protocol completion audit

This v1.0 protocol fixes:

- exact generator and grammar version strings;
- primary and permutation seeds;
- `64/32/64` record partitions and `160` total rows;
- row order and stratum schedule;
- A0, A1, A3, future A2, and evaluator-only R0 roles;
- boundary, false-merge, false-split, transfer, negative, optional, reorder,
  ambiguity, provenance, and resource metrics;
- A1, A0, A3, and permutation thresholds;
- failure classification precedence;
- exactly three reruns per variant and seed mode;
- deterministic core artifacts and canonical serialization;
- no-result and valid-negative policies;
- the architecture decision matrix; and
- the reveal and leakage order.

The next step is Experiment A implementation. It must not modify this protocol,
the frozen specification, the bounded MVP, or existing v1/v2 artifacts.
