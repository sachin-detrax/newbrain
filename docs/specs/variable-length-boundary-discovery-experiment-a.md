# Architecture Experiment A — Variable-Length Structure and Boundary Discovery

**Specification:** `architecture-experiment-a-variable-length-boundary-discovery` / `1.0.0`  
**Status:** frozen before implementation; specification/protocol step complete only  
**Date:** 2026-10-12  
**Generator:** `variable-stream-generator-a-v1`  
**Grammar:** `variable-stream-structure-a-v1`  
**Protocol:** [Experiment A protocol v1.0](../protocols/variable-length-boundary-discovery-experiment-a-protocol-v1.0.md)

**Predecessors:** [v1 design](../../testable-design-v1.md),
[v2 specification](opaque-interactive-structural-discovery-v2.md),
[v2 protocol](../protocols/opaque-interactive-structural-discovery-v2-protocol-v1.0.md),
[architecture-gap report, Experiment A](../research/architecture-gap-research-2026-10-11.md),
and [architecture plan](../plan.md).

## 1. Question, claim, and scope

> Can the learner discover reusable structural relations when sequence boundaries
> and lengths are not supplied?

Here, “not supplied” means **internal motif boundaries and motif/payload lengths**.
The learner receives opaque atoms individually and a transport `End` after each
stream. It can count atoms already received, buffer the stream, and analyze it
after `End`. Stream reset/end and the declared finite search ceiling are supplied
inductive biases. This is offline internal segmentation, not detection of an
unbounded stream's end or prediction of its future length.

A passing result supports only: within the registered synthetic family and
explicit equality/span grammar, a bounded symbolic learner can admit a reusable
variable-payload relation from multiple raw streams, locate its occurrences
without supplied internal boundaries, and transfer to fresh atoms, held-out
payload lengths, and held-out compositions while respecting uncertainty and cost.

The relation language and search objective are **hand-designed symbolic priors**.
The learner must obtain the instantiated relation, supports, applicability, and
boundaries from data. This does not establish autonomous invention of its grammar,
a learned perceptual representation, or superiority over neural approaches.

Experiment A is separate from completed MVP gates M1–M12 and Experiments B/C.
Existing source, v1/v2/MVP specifications, datasets, protocols, artifacts, and
historical results are read-only for this documentation task. Future implementation
uses a separate Experiment A namespace and must preserve historical behavior.

## 2. Opaque boundary and allowed supervision

### 2.1 Input and lifecycle

```text
StreamEvent = Token { run_scope, stream_scope, atom: AtomId }
            | End   { run_scope, stream_scope }
```

Identifiers are exact opaque `u64` values in distinct atom/scope domains. There
is no token offset field, announced stream length, record index, partition label,
motif count, delimiter, latent type, boundary, correctness flag, or score. The
learner may maintain its own receipt offsets. One episode is exactly one stream;
there are 160 episodes/records per run, with one analysis decision per episode.

The runner supplies lifecycle controls `BeginDevelopment` after 64 streams and
`Freeze` after 32 further streams. These two phase controls are disclosed
supervision; they carry no row role, length, target, or boundary. The final 64
streams are analyzed with a frozen learned model. There are no actions or rewards.

Allowed information is complete: exact identity/order, received tokens and `End`,
opaque scopes, phase controls, the grammar and resource ceilings, and learner-owned
raw/derived evidence. No latent labels or evaluator metrics are returned during
training or development either. No external corpus, pretrained weights, semantic
dictionary, gradients, embeddings, or tokenizer belongs to the symbolic variants.

### 2.2 Output

```text
Span = [start, end)                       # token offsets, 0 <= start < end <= n
RelationUse = { span, relation_version, bindings, evidence_refs }
Parse = {
  spans,                                 # ordered disjoint tiling of [0,n)
  relation_uses,                          # exactly the non-residual spans
  boundaries,                            # unique internal span endpoints
  group_refs, evidence_refs
}
Analysis = Selected { parse }
         | Alternatives { parses: [Parse, Parse], reason: ambiguous_segmentation }
         | Abstain { reason, hypothesis_refs, evidence_refs }
reason = insufficient_evidence | ambiguous_segmentation | unsupported_by_grammar
       | search_exhausted | resource_limit
```

These are mutually exclusive responses. Offsets and all uses must resolve against
the current received stream. An empty relation-use list is allowed in `Selected`
only after completed search, producing one explicitly residual span `[0,n)`.
It means “no admitted relation matched”, not “the environment has no structure”.
Residual spans are maximal gaps between selected relation spans; adjacent residual
spans must be coalesced. Relations cannot overlap or share tokens. Boundaries must
equal the tiling endpoints; inconsistent offsets are a malformed-output failure.

Each analysis includes grammar/representation version, run scope, immutable model
fingerprint, costs, and learner-created reference IDs. Final raw evidence may be
referenced to check a current match; it cannot become training support or influence
the next final stream. Final per-stream workspaces are discarded after analysis.

### 2.3 Evaluator-only data

Seeds, row indices, generator constructors, latent construction history, gold
relation spans, alternative gold parses, scoring rules/results, and oracle output
stay outside learner memory, imports, filesystem access, environment variables,
callbacks, and network access. The researcher knows the public protocol; the
runtime learner cannot look it up or branch on an episode count/schedule.
The only privileged parser is the explicitly evaluator-only R0 reference.

## 3. Raw stream family and identifiable target

The protocol specifies all seeds, length/order formulas, and allocation order.
This section defines its latent meaning. All names below are evaluator notation.

```text
E(k) = x + P[k] + x            # equal endpoints, nonempty fresh payload
Q(k) = x + P[k] + y            # unequal endpoints; near-miss distractor
N(g) = g fresh unequal atoms   # nuisance
```

All fresh allocations are distinct across the entire run except intentional
repetition of an E endpoint. There is no special alphabet for nuisance atoms.
Primary E payload lengths are 1–8. Prefix/suffix nuisance and gaps prevent any
fixed absolute position from identifying a motif. Q is superficially length-matched
to E but supplies no equality. Every record is finite, nonempty, and at most 48
atoms. Atom identities are fresh in every record; exact replay cannot transfer.

Layouts are:

- single E, with low or high nuisance density;
- two or three independent E segments separated by N;
- an always-present E plus an optional second E (present and absent examples);
- independent E segments reordered, without a supplied slot identity; and
- negative records made entirely of Q/N.

**Scored segmentation is not every constructor boundary.** Gold relation spans
are the E occurrences. The rest of the stream is partitioned into maximal residual
spans. Invisible boundaries between Q and N, or between two N constructors, are
coalesced and never scored. A negative stream therefore has one residual span and
zero gold internal boundaries. This avoids demanding recovery of unidentifiable
noise-generation history.

### 3.1 Ambiguity control

The ambiguous core is `x + P[a] + x + x + Q[b] + x`, with `(a,b)` equal to
`(1,2)` or `(2,1)`. P/Q here are fresh payloads, not the Q(k) constructor.
Its seven tokens admit two full-coverage, non-overlapping parses:

1. one E span with payload length 5;
2. two adjacent E spans with payload lengths 1 and 2 in the emitted order.

For core `x p x x q r x` beginning at offset `s`, these spans are respectively
`[s,s+7)` and `{[s,s+3), [s+3,s+7)}`. No token is used twice. Prefix/suffix
nuisance is residual in both parses. Other endpoint-equal subspans leave more
unexplained core tokens and are not maximal-coverage parses under the declared
objective. The learner must retain the two equally well-covered possibilities
or explicitly abstain. It cannot infer a unique construction history.

Ambiguity records use the `(1,2)` and `(2,1)` payload pairs in every partition.
They are uncertainty controls, excluded from held-out-length transfer
denominators. Their endpoint-equal spans therefore have lengths `1`, `2`, and
`5`; none reveals an equality span of held-out length 4, 6, 7, or 8.

## 4. Grammar, search, and what is learned

`variable-stream-structure-a-v1` is a finite explicit symbolic grammar:

```text
Fixed(k) = Bind(v) ; Payload(k,k) ; Repeat(v)       k in 1..8
Range    = Bind(v) ; Payload(1,8) ; Repeat(v)
Document = Residual ; (Relation ; Residual){0..3}
Group    = Optional(Relation) | Unordered(Relation{2..3})
```

`Bind` and `Repeat` require exact atom equality. `Payload` consumes contiguous
tokens without testing semantic properties. A residual consumes otherwise
unexplained atoms, is not a learned relation, and earns no transfer credit.
Empty residuals at adjacent relations are omitted. No recursion, arithmetic
predicates over atom IDs, arbitrary edit distance, latent slot access, semantic
similarity, or runtime call to the generator is permitted.

The form above is a supplied hypothesis language, not a supplied admitted parser.
In particular, a learner starting with no admitted range relation must not simply
run an E detector and claim it learned the relation. Generic span enumeration,
binding checks, cross-record alignment, challenges, and admission must appear in
the evidence trace. This is **symbolic segmentation search**, including when
implemented with human-authored code; it must never be described as A2 learning.

### 4.1 Frozen candidate stages

| Stage | Operation and eligibility |
| --- | --- |
| V0 | Exact streams and fixed absolute equality signatures; fit/replay only. |
| V1 | Enumerate every contiguous span of lengths 3–10 in `(start,end)` order; record fixed-k endpoint-equality matches and failed checks. No gold boundary filter. |
| V2 | Align V1 spans by their two binding positions, making the interior a sequence variable. Propose Range after observations cover at least three k values; the 1–8 envelope is a declared prior, not inferred knowledge of test lengths. |
| V3 | Infer optional-count and order-insensitive grouping support from learner-parsed mutable streams, using the rules below. |
| V4 | Admit the Range relation after support, variation, challenge, and scope checks. |
| V5 | A3 only: factor repeated admitted AST expansions into a versioned macro library. |

V4 requires at least four distinct training records, three distinct payload
lengths, and four distinct endpoint atoms, plus at least two **whole mutable
streams** on which exhaustive V1 search finds no equality match. These are
learner-derived contrast records, not evaluator-labeled negatives. Development
must provide at least two fresh matching records and two such no-match records.
Absence is asserted only after completed search. A contradictory exact match,
missing challenge, and incomplete search are recorded distinctly.

Training may create V0–V3 candidates and provisional V4 records. Development
can add support/challenges and admit or reject existing Range/group candidates;
it cannot create a new template/range or tune search. Raw training/development
tokens can be buffered and analyzed before that record's analysis; the analysis
still precedes any evaluator scoring. There is no label-reveal ingestion API.

V3 optional support requires at least two unambiguous parsed training streams
with one relation and two with two relations, then one of each in development.
V3 unordered support requires at least two training two-relation streams in each
of ascending/descending payload-length order (distinct lengths), then one of
each in development. These are structural count/order groups, not semantic
optional-slot or entity discovery. A three-relation document is allowed by the
supplied grammar but absent from mutable data; its first use tests composition.

### 4.2 Determinism and inference

Structural signatures use version, stage, operator tree, alpha-normalized
variables by first occurrence, length bounds, and scope. Support IDs are sorted
in immutable versions; they are not part of deduplication identity. IDs are
monotone learner counters. Sorting never uses atom numeric magnitude to select
a relation. Failed, merged, suspended, and incomplete candidates remain auditable.

The protocol fixes exact search order, ranking, group attachment, library
promotion, and resource charges. At final inference the admitted model is fixed;
only bindings, span applications, and parses are constructed for the current
stream. “No new final candidates” forbids new learned templates, not the finite
search necessary to apply an already admitted relation to new input.

## 5. Baselines and variants

| ID | Role | Permitted mechanism |
| --- | --- | --- |
| A0 | required fixed-position/equality baseline | Whole-stream length and absolute equality signatures; no segmentation/alignment. |
| A1 | primary symbolic learner | V0–V4; bounded exhaustive span, alignment, and parse search. |
| A3 | required symbolic library comparator | A1 plus V5 evidence-backed factoring of recurring relation expansions. |
| A2 | declared future learned-front-end comparator | Trainable representation/parser proposals feeding symbolic induction; not enrolled in v1.0. |
| R0 | evaluator-only reference | Read gold and verify scoring; no learner claim. |

A0/A1/A3 have the same raw inputs, phase controls, freeze, logical ceilings,
physical ceilings, and independent fresh state. A0 is deliberately weak; beating
it is not evidence against stronger fixed-window methods. A3 isolates small
evidence-backed macro reuse within Experiment A, not the general primitive or
procedure-composition claim proposed for Experiment C.

A3 promotion requires six concrete unambiguous training uses across four records,
three payload lengths, and both one-/two-relation learner-parsed layouts; all
must expand to V4 support. Development must confirm at least two uses, with the
protocol's model-plus-use description saving of at least 20%. It promotes no new
semantic operator, cannot bypass an equality check, and retains the complete
macro definition, support, challenges, invocation/expansion, cost, and lineage.

### 5.1 Future A2 contract

The current [Cargo manifest](../../Cargo.toml) has an empty dependency section.
No dependency or neural implementation is introduced here. A2 is
`not-enrolled-under-v1.0`, not a simulated or failed comparator.

A future separately sourced A2 requires a preregistered amendment specifying
source/license, model architecture, initialization, exact seeds, trainable
parameters, loss, optimizer/update budget, decoding/calibration, hardware,
dependencies, model/data hashes, and fresh final instances before evaluation.
Only mutable raw streams and declared self-supervision may train it for a matched
claim. Gold-boundary supervision, pretraining, pseudo-labels from a symbolic parser,
or extra data require separately labeled unmatched experiments.

A2 proposals carry model/version, source data, uncertainty, and exact input spans;
they enter as `model-proposed`, not observed facts. Symbolic verification remains
explicit. A handcrafted boundary finder, deterministic E detector, tokenizer,
or symbolic search wrapper does not become a learned front end by being called
a parser. Ablate the trained parameters against initialization to substantiate
learning. No future A2 result can retroactively change A1's frozen gates.

## 6. Partitions, motifs, and transfer

| Phase | Streams | Allowed use |
| --- | ---: | --- |
| Training | 64 | Generate and provisionally support V0–V4; record library proposals. |
| Development | 32 | Fixed challenges/admission/group/library confirmation; no human tuning. |
| Final | 64 | Frozen inference; all results sealed until complete. |

Mutable primary lengths are `{1,2,3,5}`. Final transfer lengths are `{4,6,7,8}`:
4 is interpolation, 6–8 are bounded extrapolation. There is no arbitrary-length
claim. Every record has fresh atoms. “Held-out motif” means a fresh surface
instance and, in M, a three-relation composition absent from mutable support;
it does **not** mean discovering a wholly unseen relation family.

Final strata are L:16 length transfer, M:12 composition, O:8 optional presence/
absence, R:8 reordering, N:8 negatives, and A:12 ambiguity. L/M are the primary
28-record transfer denominator. L contains matched low/high nuisance geometry
for a paired distractor robustness measure. Exact row rules are in the protocol.

## 7. Boundary and transfer semantics

For n tokens, internal boundaries are offsets `1..n-1`. Neither 0 nor n earns
discovery credit. A discovered boundary is a selected parse split matching a
gold internal split under the relation-plus-maximal-residual target in Section 3.
Reusable discovery additionally needs a V4-supported final relation application.

For selected boundaries B and gold boundaries G:

```text
TP = |B intersection G|
FS = |B minus G|                 # false splits inside a gold segment
FM = |G minus B|                 # false merges across a required gold boundary
boundary_precision = sum(TP) / sum(TP + FS)
boundary_recall    = sum(TP) / sum(TP + FM)
```

One selected span crossing three gold boundaries causes three FM events. One
invented split creates one FS; duplicates are malformed, not extra credit.
Offset tolerance is zero. For example, `G={2,7}`, `B={2,4}` gives TP=1, FS=1,
FM=1. A correct equality inside a falsely merged span cannot rescue its boundary
error. Relation-span exact match is also scored; boundary agreement alone could
otherwise mislabel a residual span as a relation.

Primary boundary metrics use the 52 unambiguous records. An abstention or
alternatives response on these records gets no selected-parse/transfer credit;
for unconditional recall its B is empty, so all gold boundaries are missed.
The report distinguishes these omissions from selected-parse false merges.
`false_merge_rate` and `false_split_rate` are records with at least one FM/FS
divided by 52, with omissions included in the former. Event rates and conditional
selected-only rates are secondary and explicitly denominated. Empty precision
denominators are null/undefined and cannot pass a precision gate; they do not
invalidate an abstaining baseline. An empty negative parse is not an abstention.

For ambiguity records, the two gold parses form an equivalence class. A single
selected parse is compared to the gold minimizing `(FS+FM,FS,FM,sorted boundaries)`
and reported diagnostically only. Honesty requires exactly both distinct gold
parses (order ignored, residuals coalesced) or ambiguity abstention with two
different maximal-coverage witness hypotheses. Blanket ambiguity abstention
without witnesses does not pass. Never penalize an honest ambiguity abstention
as a false merge in the primary 52-record metrics.

Transfer success requires: exact full tiling, all and only gold relation spans,
bindings satisfying the learned relation, required group evidence, immutable
pre-freeze V4 support, fresh atoms, and no budget/provenance violation. Exact
replay, atom constants, final-trained relations, and A0 signatures do not count.
O/R success likewise requires exact visible spans and applicable pre-freeze
group references. No evaluator slot identity must be recovered under reordering.

## 8. Evidence, abstention, and leakage

Primary provenance classes are `observed` raw tokens/End; `derived-symbolic`
exact checks; `inferred` relation/group hypotheses; `library-derived` expansions;
`model-proposed` future A2 proposals; `simulated` internal counterfactuals (no
admission credit); and `evaluator-only` gold/scores. Status is a separate field:
supported, disputed, ambiguous, suspended, or incomplete. A derived observation
does not change into a raw observation when admitted.

Records preserve input offsets, scope, dependencies, grammar/model version,
support/challenge references, costs, and creation order. Revisions append a new
version rather than rewriting support. All final model fingerprints must match
the freeze fingerprint. Current-input bindings are derived checks, never new
admission support; other final streams are unavailable to inference.

Abstention is neither an identity relation nor a correct parse. Insufficient
evidence, unresolved ambiguity, unsupported grammar, and exhausted search remain
distinct. A charge that would exceed a ceiling stops before the charge; unfinished
search cannot confidently return an earlier best parse. A fully searched negative
case may return a selected residual-only parse with its no-match evidence.

Forbidden leakage includes generator/source imports into learner decisions,
hidden labels or lengths, score feedback in any phase, final reuse, cross-variant
state, schedule-specific branches, hardcoded admitted relations, and provenance
laundering. A supplied hypothesis language is disclosed; a supplied target parse
is not. Audits and artifact/reveal rules are mandatory protocol gates.

## 9. Resource and implementation boundary

The symbolic generator/search/scorer/serialization must be implementable with
Rust standard library only. No neural, regex, optimization, or numerical package
is required. These resource ceilings and counting units are frozen in this
specification and repeated by the protocol; changing them requires a new
protocol, never a successful rerun under the old name. Finite search and physical
exhaustion produce explicit outcomes.

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
`1,073,741,824` resident-memory bytes per run. Missing measurements are protocol
failures, not zero values. Operation counts include rejected and budget-rejected
work; retained records are live learner state maxima unless explicitly marked
as cumulative.

```text
runner -> learner.ingest(Token/End) -> learner.analyze() -> serialized decision
runner -> learner.begin_development() / learner.freeze()
evaluator -> sealed gold -> scorer after all final decisions
```

There is no learner score-ingestion seam. Frozen model storage is read-only;
per-stream buffers are temporary. Evaluator, learner, scorer, and reporter
responsibilities remain auditable even if the later implementation shares
generic collections or serialization helpers with the existing framework.

## 10. Hypotheses, non-claims, and freeze

- H1: A1 finds the registered internal boundaries at the preregistered precision,
  recall, coverage, false-merge, and false-split gates.
- H2: a data-admitted relation transfers to held-out lengths and compositions,
  including under distractors, optionality, and reorderings.
- H3: negative/ambiguous cases retain uncertainty honestly and all evidence and
  costs remain inspectable.
- H4: A3 can factor a recurring relation with measured description savings and
  held-out reuse without a preregistered degradation relative to A1.

No claim concerns semantics, language, text tokenization, continuous perception,
causality, control, reward, planning, stochastic belief models, universal
identifiability, unrestricted library invention, cross-run continual learning,
statistical population estimates from deterministic reruns, or scale/energy
superiority. No-result and negative results are first-class outcomes; search
failure is not proof that a learned front end is necessary.

This specification and its companion protocol are frozen together as v1.0.0 on
2026-10-12, before any Experiment A implementation or behavioral result. Later
corrections require a new version and an explicit comparability statement. The
next step is Experiment A implementation; no implementation or experiment result
is completed by this specification/protocol step.
