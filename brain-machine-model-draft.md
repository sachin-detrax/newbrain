# Brain Machine Model: Conceptual Research Draft

Status: proposed architecture, not user-approved and not implementation-ready.
Date: 2026-10-09
Scope: conceptual design only. No implementation language, AI framework, compiler, or hardware design is selected. No experiment has been run.

## 1. Requirements and boundaries

User-selected foundations:

- A fundamental neuron is a primitive operation.
- Communication uses typed event packets.
- The core excludes backpropagation, gradient descent, dense tensor foundations, and next-token prediction as its learning objective.
- Persistent knowledge must not depend on fitting history into a fixed token window.
- The eventual experimental hardware ceiling is one consumer GPU, with host CPU and RAM requirements still unspecified.
- Continual learning, generalization, reasoning/planning, and persistent world knowledge are joint goals.

These goals are not established properties. Finite machines cannot retain unbounded information at fixed storage cost, examine arbitrary history in constant time, or guarantee error-free inference from incomplete evidence. Persistent storage is memory; the intended distinction is learning reusable rules rather than merely replaying examples. Neither overfitting nor underfitting disappears when numerical weights are removed.

The architecture is designed independently of existing model frameworks. It is not claimed to have no intellectual predecessors. Event processing, symbolic computation, program synthesis, and dependency tracking have established histories. Novelty, if any, must be demonstrated in a precise mechanism and evaluated separately from usefulness.

## 2. Foundation comparison

| Foundation | Main advantage | Main risk | Proposed role |
| --- | --- | --- | --- |
| Sparse persistent event graph | Incremental updates and explicit dependencies | Graph storage alone does not learn abstractions | Storage and routing |
| Composable primitive operations | Executable, reusable transformations | Composition search can grow exponentially | Computational core |
| State-transition models | Hypothetical execution and planning | Association is not evidence of causation | Learned procedures for world changes |

Recommendation: operations are the computational primitive; the graph records their composition and dependencies. World-transition models are compositions, not a second hidden learner. This sharpens the earlier graph-first proposal using the user's chosen operation primitive.

## 3. Machine state

At any instant the machine consists of:

- Evidence store: source observations and their provenance, subject to an explicit retention policy.
- Interpretation store: assertions, alternatives, scopes, and supporting or contradicting evidence.
- Procedure store: versioned compositions and their applicability conditions.
- Dependency index: links from evidence and assumptions to derived assertions and procedures.
- Event agenda: pending work with explicit resource budgets.
- Workspace: temporary bindings and hypothetical branches for a particular task.

These stores are conceptual distinctions; no database format is prescribed. A bounded workspace can access persistent knowledge incrementally. This removes a fixed prompt-history requirement, not the physical limit on active computation.

## 4. Primitive operations and composition

Candidate primitives:

1. Distinguish: evaluate an explicit comparison, initially literal identity or structural difference, not unexplained semantic similarity.
2. Bind: associate a variable with a value within a scope.
3. Relate: construct a typed relationship between references.
4. Apply: execute a compatible procedure.
5. Check: evaluate an explicit constraint or compare a prediction with evidence.
6. Branch: create distinct hypothetical alternatives with separate scopes.

This is a candidate vocabulary, not a proved minimal or sufficient instruction set. Storage access, indexing, event delivery, budget accounting, and recording results are machine services, not learned intelligence. Arithmetic or sensory operations require separately declared capabilities; they cannot be smuggled into an undefined understanding primitive.

A procedure has input types, conditions, an operation graph, output types, dependencies, version, evidence record, and execution budget. A reusable composition can act as a higher-level operation while retaining its inspectable internal structure.

## 5. Event protocol

An event contains an identifier, kind, payload references, source, task scope, logical order, optional observed time, dependencies, and evidence status. Kinds include observation, question, hypothesis, result, contradiction, and budget exhaustion.

Evidence status distinguishes observed, inferred, simulated, disputed, and untested. A status is not a calibrated probability. Repeated copies of one source are not independent confirmation.

An operation receives compatible events and emits results. Delivery records prevent accidental repeat execution of the same operation/version/input combination within a scope. Cycles and recursion consume a finite budget. Timeouts produce incomplete results, not an assertion that no solution exists.

## 6. Structural learning loop

Learning changes executable structure rather than globally adjusting numerical weights.

1. Execute an applicable procedure and record its checks and dependencies.
2. On a failed or missing check, identify a bounded neighborhood of the execution trace. This provides possible edit locations, not guaranteed causal attribution.
3. Generate candidate edits using a declared grammar: substitute a literal with a variable; join compatible operations; add a condition; split applicability into cases; or extract a repeated subcomposition.
4. Reject ill-typed candidates, unsupported assumptions, and candidates exceeding the resource budget.
5. Challenge remaining candidates against supporting examples, relevant prior cases, and independently produced counterexamples.
6. Retain successful candidates provisionally, with their scope and evidence. Keep alternatives when the evidence does not distinguish them.
7. On later contradiction, narrow, revise, suspend, or replace a candidate without silently rewriting its evidence history.

Candidate ranking first favors lower error on development challenges, then shorter descriptions, then lower measured execution cost under explicit caps. Complexity is measured in a fixed primitive vocabulary, expanding reusable macros for comparison so that renaming a large composition cannot make it artificially simple. These preferences impose an inductive bias; they do not prove correctness or eliminate overfitting.

Each attempt has limits on candidate count, edit depth, execution steps, and retrieved evidence. Search exhaustion yields an unresolved question or a request for a discriminating observation. Bounded search controls spending but can miss the correct procedure entirely.

## 7. Worked conceptual example

Begin with structured observations, not a claim of raw-language understanding:

- An object is held by participant A.
- An observed transfer moves the object from A to B.
- A later independent observation identifies B as its holder.

A candidate procedure substitutes variables for the particular object and participants. Its applicability conditions include the initial holder and an actual completed transfer. Its predicted update changes the holder from the sender to the recipient within the relevant world scope.

Challenge cases change the names, object, chain length, and event order. Negative cases include a proposed transfer, a failed transfer, and a sender who does not possess the object. A rule that treats all mentions of a transfer as completed transfers must fail these checks.

Successful transfer to new participants demonstrates a narrow form of compositional generalization. It does not establish general intelligence, linguistic understanding, or a universal theory of ownership. Different domains may require additional conditions or different semantics.

## 8. Memory, contradictions, and change

Each assertion is scoped by source, time where relevant, and hypothetical or actual context. A changed state is not necessarily a contradiction: possession at one time can differ from possession at another. Conflicting claims about the same state remain alternatives unless evidence resolves them.

New evidence marks dependent interpretations as needing review. The dependency index finds known affected structures; revalidation can be expensive if the change has broad consequences. Undetected dependencies remain a correctness risk. If revalidation is unfinished, affected results must be marked uncertain rather than silently treated as current.

Procedure versions preserve earlier behavior for regression tests. Retention does not guarantee access: continual-learning evaluation must test actual retrieval and use, not merely confirm that an old record remains on disk.

Finite storage requires a declared retention policy. Exact deletion makes some future recovery impossible; summaries are lossy. The design must report which evidence is retained, compressed, or discarded rather than claim unlimited memory. Sensitive evidence also requires access control and deletion semantics in any later implementation; those policies are not specified by this conceptual learner.

## 9. Reasoning and planning

Reasoning composes applicable procedures and evaluates explicit checks. Each answer can expose the evidence and assumptions used to derive it. This yields an inspectable derivation, not proof that its premises are true.

Planning runs state-transition procedures in isolated hypothetical scopes. A goal is an explicit constraint on a future state. Candidate actions are expanded within a search budget, checked for violated constraints, and compared by a declared task cost. Cyclic or very deep problems may remain unresolved.

A prediction from an observational association must not be presented as an established causal law. Interactive environments can supply intervention evidence; text alone usually supplies reports and claims. Simulated outcomes never become observed evidence merely because the machine generated them.

## 10. Text acquisition is an unresolved subsystem

The entry point for raw text can preserve characters, boundaries, source, and order without importing a pretrained semantic model. Discovering meaningful segments, references, negation, and reusable grammatical structures requires further learning mechanisms and evidence.

One research possibility is to let candidate compositions propose segmentations and interpretations, then challenge those interpretations against paired observations and tasks. This remains a proposal. Ambiguous text may support multiple incompatible interpretations, and text-only evidence may not distinguish them.

Early structured-world experiments isolate the structural learner. They do not satisfy the eventual goal of learning broad knowledge from raw text. Any later demonstration must disclose hand-supplied vocabulary, schemas, grammar, rules, or supervision. A handcrafted parser cannot be counted as learned understanding.

## 11. Resource model and efficiency hypothesis

Intended savings come from selective event activation, indexing, reuse of established procedures, and localized revalidation. None guarantees efficient discovery.

For an attempted learning step, candidate evaluation work is approximately the number of candidates multiplied by challenge executions per candidate and operation steps per execution, plus retrieval, proposal generation, and bookkeeping. Persistent storage grows with retained evidence, dependency links, and procedure versions.

Worst cases include broad dependency invalidation, poor retrieval selectivity, conflicting hypotheses, and combinatorial program or plan search. Fixed budgets limit cost by allowing incompleteness, not by solving these worst cases.

Irregular sparse computation may run poorly on a consumer GPU. Parallel candidate checks might benefit, but hardware suitability requires measurement. No GPU speedup, training duration, RAM requirement, or corpus-scale throughput is claimed here.

## 12. Falsifiable evaluation plan

First evaluate a controlled state-transition domain with independently generated outcomes. Lock task generators, primitive vocabulary, resource limits, and evaluation rules before final testing. Use development challenges to revise procedures; never use the final held-out tests to guide edits. Include multiple task orders and independent random seeds.

| Goal | Experiment | Required measurements and failure evidence |
| --- | --- | --- |
| Generalization | Hold out participants, structures, and combinations; test longer compositions separately | Accuracy by split, counterexample failures, and abstentions; name substitution alone is insufficient |
| Continual learning | Interleave new tasks, then retest old tasks through the ordinary retrieval path | Before/after accuracy, forgetting, update cost, and storage growth |
| Long-history use | Increase irrelevant history while preserving distant decisive evidence | Recall, answer accuracy, retrieval work, latency, and storage; missing distant evidence is a failure |
| Reasoning/planning | Test unfamiliar goals and transitions in an independently checked environment | Goal success, invalid actions, plan length, search work, and budget exhaustion |
| Revision | Introduce changed rules, contradictory sources, and duplicate reports | Stale answers, correct scoping, duplicate-evidence handling, and recovery cost |
| Efficiency | Increase task and corpus size under fixed budgets | Wall time, peak host/device memory, operation counts, accuracy, and unresolved cases |

Comparisons should include exact-example retrieval and a nonlearning fixed-rule executor, with their different supervision disclosed. Ablations should remove composition reuse, counterexample checks, and dependency tracking individually. Comparators are evaluation instruments, not dependencies of the proposed architecture.

Numerical acceptance thresholds cannot honestly be fixed before hardware and task scales are defined. A later experiment protocol must set them in advance. Reject or materially revise the hypothesis if structural learning fails to outperform exact-example retrieval on held-out combinations at an acceptable resource cost, or if apparent improvements depend on leaked test cases or hand-encoded solutions.

## 13. Internal design review and decision status

- Constraint consistency: no dense neural learner, global gradient update, next-token objective, or fixed token-history requirement is part of the proposed core.
- Scope consistency: primitive operations are the computational units; events transport data; graph structure records composition and dependencies.
- Learning transparency: candidate edits and checks are explicit, but useful proposal generation and scalable search are unproved.
- Memory honesty: persistence is distinct from perfect recall; finite storage and revalidation costs remain.
- Grounding boundary: structured-input success does not demonstrate raw-text acquisition or causal understanding.
- Evaluation integrity: development challenges and final held-out tests are separate; implementation and numerical acceptance criteria remain future decisions.
- Evidence status: all capabilities described here are proposals. No prototype, benchmark, novelty review, or external validation has been completed.

The architecture is ready for conceptual critique, not implementation approval. The next theoretical decision is a bounded candidate-generation policy precise enough to trace by hand, including how it selects edit locations and obtains discriminating evidence without already knowing the intended rule.

## 14. Candidate-generation policy

The system must not generate arbitrary operation graphs. It begins with an observed mismatch between a procedure's predicted result and a checked result, or with a repeated pattern that the current procedure cannot express. The mismatch is converted into a **repair question**:

> Which smallest structural change would make this trace explain the result while preserving its known limits?

Candidate generation is local and staged.

### 14.1 Trace slicing

The learner first extracts the smallest dependency slice containing:

- the input events used by the procedure;
- the operation outputs that influenced the failed check;
- the assumptions and bindings on the path;
- the evidence that accepted or rejected the result.

The slice is a proposal boundary, not a proof that the cause is inside it. If no bounded slice explains the mismatch, the system records an unresolved diagnostic rather than expanding the entire history automatically.

### 14.2 Edit classes

Within the slice, the learner may propose only these initial edit classes:

1. **Parameterize:** replace a repeated literal with a scoped variable.
2. **Reorder:** change the order of independent operations when their dependencies permit it.
3. **Compose:** connect the output of one compatible operation to another.
4. **Guard:** add an explicit precondition or postcondition.
5. **Split:** create separate procedure versions for cases that have conflicting outcomes.
6. **Extract:** name a repeated subcomposition as a reusable procedure.
7. **Reject:** remove an operation or assumption shown to be unnecessary or contradicted.

Every edit must identify the exact trace locations it changes and the evidence that motivated it. New primitives are not invented during ordinary repair. Primitive invention is a separate research process because allowing it here would make the search space unbounded and make comparisons unfair.

### 14.3 Progressive widening

The learner evaluates candidates in increasing structural distance from the failed trace:

- first one edit;
- then two compatible edits;
- then extraction or case-splitting involving an existing composition;
- only later, a new composition across two previously separate traces.

The next stage is opened only if the current stage has no candidate that passes its challenge set, or if every passing candidate has a known failure on a required capability. This is a search policy, not a guarantee that the shortest path to a good rule will be found.

### 14.4 Evidence-guided proposal sources

Proposals may use only four sources:

- the failed execution trace;
- structurally similar stored traces selected through declared relation tests;
- explicit counterexamples;
- an external discriminating observation requested by the system.

“Structurally similar” must be defined through the current primitive vocabulary and event fields. It cannot mean an unexamined similarity score. Later research may introduce a learned similarity operation, but its own proposals must be audited by the same evidence rules.

When the evidence is insufficient, the system should ask for an observation that separates the leading candidates. For example, if two procedures differ only on whether a transfer was completed or merely planned, the preferred next observation is one that checks the object's resulting holder. Asking for more arbitrary data is not active learning; the requested observation must be tied to a candidate disagreement.

### 14.5 Candidate admission

A candidate enters the procedure store only if it passes all of the following gates:

1. **Type gate:** every operation connection and output is compatible.
2. **Scope gate:** it does not mix actual, hypothetical, or unrelated task scopes.
3. **Trace gate:** it explains the motivating mismatch without rewriting the evidence.
4. **Variation gate:** it survives at least one relevant change in names, values, or order.
5. **Counterexample gate:** it does not produce the same result on a deliberately distinguishing negative case.
6. **Cost gate:** its measured search and execution cost is within the declared budget.

Passing these gates makes a candidate provisionally usable; it does not make it true. Candidates with unresolved conflicts remain versioned alternatives. A candidate that only passes because the challenge set contains repeated examples is memorization, not demonstrated generalization.

## 15. Anti-memorization challenge family

Every learned procedure must be tested against transformations that preserve the intended structure while changing superficial details. The initial challenge family is:

- **Renaming:** replace entities and values while preserving relations.
- **Permutation:** change the order of independent observations.
- **Length extension:** repeat a valid composition for more steps than seen during discovery.
- **Minimal contrast:** change exactly one condition that should change the result.
- **Irrelevant insertion:** add unrelated events and verify that the result is stable.
- **Scope contrast:** place the same event in a hypothetical or historical scope.
- **Contradiction:** provide evidence that invalidates one dependency but not unrelated knowledge.

The generator must record which transformations a procedure has actually passed. A procedure that has only passed renaming cannot be described as generally compositional. Generalization claims must name their tested invariances.

## 16. What this policy does and does not solve

This policy replaces unrestricted program enumeration with a local repair grammar, progressive widening, and evidence-linked tests. It may substantially reduce search when failures are informative and the correct rule is near an existing trace.

It does not solve:

- discovering useful primitives from raw sensory or textual streams;
- locating a causal failure when the trace is incomplete;
- choosing useful abstractions when many are equally predictive;
- avoiding exponential growth across interacting rules;
- deciding which evidence sources deserve trust;
- proving that a compact procedure is semantically correct;
- guaranteeing a solution within any fixed search budget.

The architecture should therefore be judged first on domains where event structure and checks are independently available. If it cannot learn and reuse small structural rules there, adding raw text or a larger memory will not rescue the core hypothesis.

## 17. Next theoretical checkpoint

Before choosing an implementation language, the model needs a paper-trace specification for one complete learning episode. That trace must show:

1. initial event packets and their evidence statuses;
2. an attempted procedure and its execution trace;
3. the failed check;
4. the dependency slice;
5. generated one-edit candidates;
6. counterexamples and candidate rejection;
7. provisional admission of a surviving procedure;
8. later contradiction, revalidation, and versioning.

The paper trace is the next falsifiability checkpoint. If the rules cannot be applied consistently by hand, a runtime would only hide ambiguity behind code.

## 18. Paper trace: learning a guarded state transition

This trace uses a deliberately small structured domain. It is not a language-understanding demonstration. Event segmentation, event types, state semantics, and outcome checks are supplied by the domain definition so that the structural learning loop can be inspected separately.

### 18.1 Domain and available primitives

The domain contains objects, locations, and movement events. The state predicate is `at(object, location)`. A movement event may be `completed` or `proposed`.

Available primitives are:

- `Bind`: bind `object`, `from`, and `to` variables.
- `Relate`: create or inspect an `at` relation.
- `Apply`: apply a procedure to a scoped event and state.
- `Check`: compare the predicted state with an observed state.
- `Guard`: require an explicit event or state condition.
- `Branch`: keep competing interpretations separate.

The meanings of `at`, `completed`, and `proposed` are domain primitives supplied before learning. The learner is not credited with discovering them in this trace.

### 18.2 Initial evidence

All events belong to actual world scope `W1` and have distinct provenance identifiers.

| Event | Packet content | Evidence status |
| --- | --- | --- |
| `e1` | `at(ball, red)` | observed |
| `e2` | `move(ball, red, blue, completed)` | observed |
| `e3` | `at(ball, blue)` | observed |
| `e4` | `at(book, shelf)` | observed |
| `e5` | `move(book, shelf, desk, proposed)` | observed |
| `e6` | `at(book, shelf)` | observed |

The first three events support learning that a completed movement can change location. The last three events are a contrast case: a proposed movement does not change location.

### 18.3 Initial procedure and execution failure

Suppose the current procedure store contains `P0` because an earlier trace was fit too literally:

```text
P0:
  when a movement event names object X and destination D,
  infer at(X, D)
```

`P0` is an intentionally defective candidate. It has variables for the object and destination, but no condition on completion and no check that the object is currently at the named source.

The learner applies `P0` to `e5`:

1. `Bind` maps `X=book`, `from=shelf`, and `D=desk`.
2. `Apply` emits the inference `at(book, desk)` in scope `W1`.
3. `Check` compares the inference with `e6`, which reports `at(book, shelf)`.
4. The check fails. The inference is marked `disputed`, not converted into an observed fact.

The execution trace records that the failed result depended on `e5`, the current state from `e6`, the absence of a completion guard, and the destination update in `P0`.

### 18.4 Dependency slice and repair question

The smallest available slice contains:

- `e5`: the movement event and its status;
- the binding of `book`, `shelf`, and `desk`;
- `P0`'s movement-to-destination operation;
- `e6`: the checked state;
- the failed equality check between `at(book, desk)` and `at(book, shelf)`.

The repair question is:

> What smallest condition distinguishes the completed movement in `e2` from the proposed movement in `e5` without changing the evidence?

### 18.5 One-edit candidates

The initial progressive-widening stage proposes:

| Candidate | Edit | Initial prediction |
| --- | --- | --- |
| `C1` | Add `Guard(status = completed)` | A proposed movement does not update location. |
| `C2` | Add `Guard(current location = from)` | A movement from the wrong source does not update location. |
| `C3` | Split completed and proposed movements into separate procedures | The completed branch updates; the proposed branch records no update. |

`C1` is directly motivated by the failed trace. `C2` is generated because the source binding is present in the trace but unused by `P0`. `C3` is generated because the event status creates two observed behavioral cases. No new primitive is invented.

### 18.6 Counterexample challenge

The domain generator creates two minimal contrasts not used to propose the candidates:

- `e7`: `at(crate, dock)` and `move(crate, yard, warehouse, completed)`, followed by observation that `crate` remains at `dock`.
- `e8`: `at(parcel, office)` and `move(parcel, office, lobby, proposed)`, followed by observation that `parcel` remains at `office`.

Results:

- `C1` passes `e8` but incorrectly updates the crate in `e7`; it is rejected because completion alone is insufficient.
- `C2` passes `e7` but incorrectly updates the parcel in `e8`; it is rejected because source agreement alone is insufficient.
- `C3` passes both contrasts if its completed branch also requires the current location to match the source. As written, `C3` is under-specified and must be expanded into a two-edit candidate before admission.

The two-edit candidate `C4` is:

```text
C4:
  when movement status = completed
  and current location of X = from,
  infer at(X, to)
```

`C4` passes the motivating trace, the completed movement, the proposed movement, and the wrong-source movement. It remains provisional because these examples test only a narrow state-transition rule.

### 18.7 Admission record

`C4` passes the admission gates as follows:

- Type: all bindings and relations are compatible.
- Scope: every event is in actual scope `W1`.
- Trace: it explains the failed proposed movement without rewriting `e5` or `e6`.
- Variation: it works for different objects, locations, and names.
- Counterexample: it rejects both a proposed movement and a completed movement from the wrong source.
- Cost: it is within the current two-edit and finite-execution budget.

The procedure is stored as version `C4.v1`, with links to `e1` through `e8`, the challenge generator version, and the exact checks it passed. The system may use it, but should report that its tested invariances are limited to the recorded challenge family.

### 18.8 Later contradiction and versioning

Later, a new observed event arrives:

| Event | Packet content | Evidence status |
| --- | --- | --- |
| `e9` | `move(ball, red, blue, completed)` was recorded incorrectly; the movement was cancelled before completion | observed report, disputed until corroborated |
| `e10` | `at(ball, red)` after the reported cancellation | observed |

The event conflicts with the earlier interpretation of `e2` and `e3`. The system does not delete those events. It branches the interpretation:

- `I1`: `e2` was completed and `e3` reflects the resulting state.
- `I2`: `e2` was misclassified or later reversed, and `e10` reflects the current state.

Dependent uses of `C4.v1` are marked for revalidation in the relevant time scope. `C4.v1` is not globally declared false: its rule may still hold for completed movements, while the status or temporal scope of this particular ball episode is disputed. If corroboration establishes that `completed` was used incorrectly, the event interpretation is revised and the procedure's evidence ledger records the loss of support for that episode.

This trace demonstrates versioned evidence handling, not truth maintenance in general. It leaves open how corroboration is obtained, how time intervals are represented, and how many dependent results can be revalidated within a budget.

### 18.9 What the trace proves and does not prove

The trace shows that the proposed learning rules can be applied consistently to a hand-defined structural task. It demonstrates a narrow form of variable substitution, guarded state transition, counterexample rejection, and scoped contradiction handling.

It does not demonstrate raw-text parsing, primitive discovery, causal understanding, broad knowledge, efficient search at scale, or freedom from overlearning. The supplied domain vocabulary and event parser are substantial supervision. Any future claim must separate these hand-supplied components from structures learned by the system.

## 19. Paper-trace review

- Every transition has an event, procedure, check, or explicit evidence status.
- The failed result is preserved as disputed rather than silently accepted or deleted.
- One-edit candidates are generated locally before the two-edit candidate.
- Counterexamples distinguish completion status from source agreement.
- The admitted procedure is provisional and carries its evidence and challenge history.
- Later contradictory evidence creates scoped alternatives rather than rewriting the past.
- Domain semantics and parsing are explicitly declared as supplied, preventing an inflated intelligence claim.

The trace is internally consistent for the stated domain. The unresolved next question is whether the same repair grammar can acquire the domain's primitive vocabulary rather than receiving it in advance.

## 20. Primitive bootstrap boundary

“From scratch” cannot mean that the machine has no operations at all. A system with no ability to receive a distinction, preserve an observation, or compare two states has no way to learn that those abilities are useful. The honest target is therefore:

> Fix only a small substrate of non-semantic operations; require domain concepts and reusable transformations to be discovered above that substrate.

The proposed bootstrap contains:

1. **Receive:** accept an externally delimited input atom and attach source and arrival order.
2. **Preserve:** retain an observation or a reference to it until the retention policy acts.
3. **Replay:** present stored observations to a procedure or comparison.
4. **Identify:** determine whether two references are exactly the same or distinct.
5. **Order:** compare explicit sequence positions or timestamps when present.
6. **Bind:** associate a temporary role with a reference inside a scope.
7. **Group:** create a bounded collection of references without assigning semantic meaning.
8. **Branch:** keep alternative interpretations separate.
9. **Check:** compare an output against an explicit condition or observation.
10. **Budget:** stop or report incomplete work when a declared resource limit is reached.

These operations are machine axioms, not learned intelligence. They do not know what a person, object, cause, word, sentence, goal, or truth is. The bootstrap itself is a substantial design choice and must be reported in every experiment.

There is an unavoidable trade-off. Making the bootstrap smaller increases conceptual purity but makes input handling and discovery harder. Making it larger makes the system easier to use but moves more intelligence into hand-supplied machinery. The boundary should remain stable while evaluating a learning hypothesis; changing it between tasks invalidates direct comparison.

## 21. Primitive-discovery ladder

Richer operations are promoted from repeated, testable compositions rather than declared in advance.

### 21.1 Stage 0: atomic observation

The input adapter emits opaque atoms with identity, order, source, and optional physical metadata. Atoms have no semantic labels. For a text channel, these may initially be code units or externally delimited chunks; the adapter must not silently supply words, grammar, or meanings.

### 21.2 Stage 1: exact structural regularities

The machine searches for repeated sequences, repeated adjacency, recurrence after interruption, and consistent differences. It can state only observations such as:

- atom `a` is followed by atom `b` in many traces;
- a group of atoms recurs with a stable boundary;
- changing one input position changes one output position;
- two traces share an operation-shaped relation after variable positions are ignored.

These regularities use identity, order, and explicit grouping. They do not require a dense semantic space or an unexplained similarity score.

### 21.3 Stage 2: variable and role hypotheses

When a repeated structure has positions that vary while its transformation remains stable, the machine proposes scoped variables. A role hypothesis is accepted only if replacing the variable with held-out references preserves the checked behavior.

For example, repeated traces may support the structural form:

```text
input:  relation(X, source) + completed_move(X, source, destination)
result: relation(X, destination)
```

At this stage `X`, `source`, and `destination` are formal roles, not words with human meaning.

### 21.4 Stage 3: relation hypotheses

The machine proposes a relation when two or more event patterns maintain a repeatable dependency under controlled variation. A relation must record:

- the observations that support it;
- the variations that preserve it;
- the minimal contrasts that break it;
- the scope and timing in which it holds;
- competing relations that explain the same traces.

An association is not promoted to a causal relation merely because it frequently co-occurs. A causal claim requires intervention, counterfactual evidence, or an explicitly marked assumption.

### 21.5 Stage 4: executable extraction

Once a relation-shaped composition passes the anti-memorization challenge family, the machine may extract it as a reusable procedure. Extraction is a naming and indexing operation over an existing composition; it does not create new semantic power. The full composition remains available for inspection, regression, and revision.

### 21.6 Stage 5: primitive promotion

A frequently reused procedure may be promoted to a primitive-like operation for efficiency. Promotion must preserve an expansion link to the underlying composition and evidence ledger. A promoted operation can be demoted if later evidence exposes a limitation.

Promotion is therefore a cache and interface decision, not an assertion that the operation is fundamental. This prevents the system from hiding a large learned program inside an opaque “neuron.”

## 22. Discovery without dense representations

The discovery ladder needs a way to find repeated structures without assigning every input a point in a large numerical space. The provisional representation is a **canonical structural signature** made from:

- typed event kinds;
- exact or declared-equivalence references;
- ordered edges;
- scopes and time relations;
- operation names and argument positions;
- evidence statuses.

Two structures may be compared only through declared transformations: exact equality, variable renaming, permitted reordering, subgraph inclusion, or a known relation operation. Every transformation used by a comparison is recorded so that a later similarity result is auditable.

Canonical signatures do not solve scale. Indexes can reduce repeated scans, but canonicalization, graph matching, and equivalence testing can themselves be expensive. A future implementation may use compact indexes or hardware acceleration; that is an optimization question, not a new learning principle.

The discovery system should prefer transformations that explain more checked outcomes with fewer new conditions, but this is an inductive preference rather than a proof of truth. A compact false rule can still be attractive. Counterexamples and held-out variations remain mandatory.

## 23. Text as a later discovery problem

If raw text is eventually used, the input path can begin with opaque character or byte atoms. The machine can propose boundaries when recurring sequences behave as units across many contexts, then test whether the proposed unit improves prediction or transformation reuse on held-out text.

This may discover reusable strings or structural segments, but a repeated segment is not automatically a word, entity, instruction, or concept. Meaning requires relations to other observations, consequences, and tasks. The system must preserve multiple interpretations when the available text does not distinguish them.

Grammar-like structures can be proposed as transformations over sequences, but the same anti-memorization and contradiction rules apply. A segment that works only on the training corpus is a compression artifact, not learned language understanding.

The raw-text path therefore has three distinct claims that must not be conflated:

1. discovering recurring textual structure;
2. discovering reusable transformations over that structure;
3. grounding those transformations in entities, states, goals, and consequences.

The first may be possible from text alone. The second requires a task or checked transformation. The third generally requires interaction, observation, or independently grounded evidence.

## 24. Primitive-discovery falsification tests

Primitive discovery should be evaluated independently from the later reasoning system:

- **Label removal:** remove human names and semantic labels while preserving raw structure.
- **Variable holdout:** hold out combinations of references and positions.
- **Boundary holdout:** test sequences whose useful boundaries differ from those seen during discovery.
- **Negative transformation:** include repeated structures whose superficial form is shared but whose checked outcome differs.
- **Promotion audit:** compare a promoted primitive with its full expansion and verify equivalent behavior within scope.
- **Revision test:** introduce a counterexample that invalidates a promoted operation and measure demotion or versioning.
- **Resource scaling:** measure candidate count, comparison work, retained structures, latency, and unresolved cases as input grows.

Success requires more than finding compact patterns. A discovered primitive must transfer to held-out structures, preserve evidence boundaries, survive minimal contrasts, and remain inspectable after promotion. Failure to do so means the discovery mechanism is memorizing or overcompressing rather than learning a reusable operation.

## 25. Current theoretical conclusion

The architecture can begin from a small non-semantic machine substrate and attempt to grow semantic operations through repeated structural transformations. This is a coherent hypothesis, but not yet a complete theory of intelligence. The hardest remaining issue is bootstrapping useful distinctions from an unlabelled stream while avoiding both arbitrary human semantics and combinatorial search.

The next paper-level test should remove the hand-defined movement vocabulary. It should start with opaque event fields and show exactly which structure is discovered, what evidence permits the promotion, and which claims remain unknown. If the result depends on secretly supplying object boundaries, event meanings, or outcome checks, that dependency must be counted as supervision.

## 26. Opaque-world discovery protocol

The next test should use a small interactive environment whose internal semantics are known only to the evaluator. The learner receives no labels such as `object`, `location`, `move`, `completed`, or `goal`.

### 26.1 Environment boundary

The environment has hidden state and exposes only:

- an ordered stream of opaque symbols;
- a channel through which the learner may emit opaque action sequences;
- a returned stream after each action;
- a terminal success, failure, or incomplete signal for a declared task episode.

The transport layer may provide packet length, source, and episode boundaries. It must not provide semantic event boundaries, field names, entity identities, or transition descriptions. If the learner receives any of those, they are declared supervision.

The evaluator privately defines a tiny world containing hidden entities, hidden locations, and deterministic or stochastic transitions. The evaluator uses those semantics only to generate observations, tasks, and independent measurements. It does not expose the latent names to the learner.

Opaque symbols should be stable within a training family but permuted for held-out families. This tests whether the learner discovers structural roles rather than memorizing a symbol's spelling. The permutation protocol must preserve the world's structure while changing the surface code.

### 26.2 Three separate achievements

The experiment must report three levels independently:

1. **Pattern discovery:** the learner finds recurring sequences, boundaries, dependencies, and invariants in the opaque stream.
2. **Grounded operation:** the learner uses discovered structure to predict or change an observable outcome through interaction.
3. **Semantic naming:** a human or later communication protocol maps the learned roles to descriptions such as object, location, or movement.

Level 1 does not imply Level 2. A compact pattern can be useless for action. Level 2 does not require Level 3. A system can control a world using unnamed roles. Human-readable labels are an interface and evaluation aid, not evidence that the learner internally acquired the concepts.

### 26.3 Exploration and evidence

The learner begins without a task-specific transition rule. It can:

- replay observations;
- compare exact symbols and sequences;
- group recurring structures;
- choose an opaque action sequence within a budget;
- observe the resulting stream and terminal outcome;
- preserve alternative hypotheses about what changed.

An action outcome is evidence about a consequence, not a semantic explanation. For example, if action sequence `a7 a2` changes a recurring pattern, the learner may record that the sequence has an effect under the current scope. It must not immediately label that effect “movement” or “transfer.”

To distinguish hypotheses, the learner should select actions where leading procedures predict different outputs. Repeating random actions produces data but is not a discriminating experiment. An exploration request should therefore include the competing predictions it is intended to separate.

### 26.4 Hidden evaluator world

The first world should be small enough to audit manually:

- a finite set of hidden entities;
- a finite set of hidden positions or states;
- actions that sometimes alter one hidden relation;
- irrelevant symbols and distractor events;
- delayed observations;
- at least one action whose surface pattern resembles a successful action but has a different consequence.

The world should include partial observability so that the learner must preserve uncertainty rather than treat every missing symbol as a negative fact. It should also include a change in a transition rule after a declared episode boundary, testing whether old procedures are scoped and revised rather than blindly reused.

The evaluator must publish the world generator and measurement definitions after the learner protocol is frozen. Otherwise the system designer can unintentionally encode the hidden ontology into the task generator or challenge set.

## 27. Opaque-world learning episode

A conceptual episode proceeds as follows:

1. The learner receives an opaque observation stream `r0` with no semantic field names.
2. It extracts exact recurrence and order relations, retaining several possible groupings.
3. It emits an opaque action sequence `u1` under an exploration budget.
4. The environment returns `r1` and a terminal outcome for the current task.
5. The learner compares `r0` and `r1`, locating changed and unchanged structures without assuming their meanings.
6. It proposes a transition procedure whose inputs are structural roles, not human labels.
7. It selects a second action where the leading procedure and its nearest alternative predict different observations.
8. It retains only the procedure that survives the observed contrast, or keeps both if the outcome remains ambiguous.
9. It tests the procedure under symbol permutation, irrelevant insertion, delayed observation, and a held-out action combination.
10. It records whether the procedure supports prediction, control, or only compression.

This episode is intentionally weaker than natural-language learning. Its purpose is to determine whether useful operations can emerge from opaque interaction without hand-defined concepts.

## 28. Claims and failure conditions

The learner may claim **structural discovery** only if it identifies a reusable invariant without being given the hidden state fields. It may claim **grounded learning** only if that invariant improves held-out prediction or task success through interaction. It may claim **generalization** only if the result transfers across symbol permutations, entity substitutions, task orders, and relevant compositions.

The test fails the core hypothesis if:

- it succeeds only when semantic fields or event boundaries are supplied;
- it memorizes the training symbol mapping and collapses under permutation;
- it compresses observations but cannot choose actions with better-than-baseline outcomes;
- it cannot maintain competing hypotheses when observations are ambiguous;
- it uses a changed rule as if the old procedure were globally valid;
- it requires unbounded random exploration to find a useful distinction;
- its structural search grows beyond the declared resource budget before producing a reusable procedure.

Exact-example retrieval, random exploration, and a fixed hand-written controller should be evaluated as separate baselines. The proposed learner must disclose whether it receives more interaction, more feedback, or more memory than those baselines.

## 29. Limits of the opaque-world result

Even a successful opaque-world result would show only that a structural learner can discover and ground some reusable operations in a controlled environment. It would not establish natural-language understanding, human-level abstraction, open-world reasoning, or superintelligence.

Conversely, failure in a deliberately underspecified world may mean the bootstrap lacks a necessary comparison or exploration operation rather than that the entire architecture is impossible. The experiment should therefore record the exact missing operation, search failure, or evidence ambiguity instead of reducing every failure to a single accuracy number.

The value of this test is isolation: it removes semantic labels while preserving observable consequences. It tells us whether the proposed foundation can acquire any useful grounded structure before we add text, large memory, or a user-facing assistant.

## 30. Goals as explicit constraints

The machine needs a reason to spend computation. A goal should not be an unexplained scalar reward or a hidden preference embedded in weights. The provisional goal representation is an explicit constraint over a future state, an observation, or a justified information condition.

A goal packet contains:

- desired condition or set of acceptable conditions;
- scope and time horizon;
- hard constraints that may never be violated;
- allowed actions and resources;
- evidence required to declare success;
- cost or priority supplied by the goal source;
- expiry, revision, or cancellation conditions.

Examples:

```text
Goal G1:
  achieve: at(tool, desk)
  before: episode end
  prohibit: damage(tool)
  allow: at most 12 action steps
  success evidence: independently observed desk state
```

```text
Goal G2:
  distinguish: hypothesis H1 from hypothesis H2
  require: one observation whose predicted outcomes differ
  prohibit: altering protected state
  allow: 3 exploratory actions
```

The second goal is an information-seeking goal. It makes curiosity explicit as a request to separate hypotheses, not as an opaque intrinsic reward.

Goals may be nested: a broad task can contain subgoals for information gathering, safety, retrieval, and execution. Completion of a subgoal is evidence for the parent goal, not automatic proof that the parent condition holds.

The system must preserve the source of a goal. A user request, environment condition, self-maintenance rule, and speculative hypothesis are different sources and must not silently override one another. Conflicting goals remain a constraint-resolution problem, not a hidden average.

## 31. Action and attention selection

Working computation is finite even when long-term storage is persistent. The machine therefore needs an auditable event agenda rather than an unrestricted scan of every memory and hypothesis.

Each agenda item contains:

- the event, goal, hypothesis, or procedure needing work;
- dependencies that make it relevant;
- the predicted consequence of processing it;
- required operation count and memory access;
- deadline or scope;
- reasons it was placed on the agenda.

The provisional routing order is lexicographic, not a learned scalar:

1. Reject actions that violate hard safety or scope constraints.
2. Process work required to prevent a known goal violation or stale committed result.
3. Process an item that can distinguish currently competing hypotheses.
4. Process a dependency of the best currently valid plan.
5. Prefer reusable procedures over repeated exact-example work.
6. Prefer lower measured cost when two items have equivalent expected task effect.
7. Preserve unresolved alternatives when no item dominates another.

“Best currently valid plan” means a plan that has passed the available checks under its declared assumptions. It is not a claim of globally optimal planning.

Attention is therefore a routing decision over explicit relations and constraints. It is not a hidden soft weighting of all memory. A later implementation may use numeric indexes for speed, but the semantic decision must remain explainable in terms of the recorded ordering rules.

### 31.1 Retrieval path

Retrieval begins from a goal, event, failed check, or hypothesis disagreement. It follows declared indexes in stages:

1. exact references and scope;
2. known dependencies and inverse dependencies;
3. operation signatures and variable roles;
4. tested structural transformations;
5. broader alternatives only when narrower retrieval fails.

Each retrieved item carries its reason for retrieval. A memory that is merely similar but not connected by a declared transformation is a candidate, not evidence. This prevents broad associative retrieval from silently becoming fact.

If the retrieval budget is exhausted, the system reports incomplete search and the missing retrieval path. It must not answer as if unexamined memory were absent.

## 32. Active information seeking

When multiple hypotheses remain, the machine should choose an observation or action that separates them. For each candidate action, it records:

- predicted outcome under each hypothesis;
- which hypotheses would remain indistinguishable;
- action cost and risk;
- whether the action changes protected or irreversible state;
- the evidence required to interpret the result.

The preferred information action is the lowest-risk action that separates a required disagreement within budget. If no safe action separates the hypotheses, the system abstains or asks for external evidence.

This replaces a vague “curiosity score” with a testable rule. It can still fail when hypotheses are wrong, predictions are incomplete, or the environment is stochastic. The action record must preserve those limitations.

## 33. Self-evaluation and abstention

Before producing an answer or action, the machine checks:

- whether the relevant evidence was observed, inferred, simulated, disputed, or untested;
- whether the procedure applies to the current scope and time;
- whether required dependencies were retrieved and revalidated;
- whether competing hypotheses predict different outcomes;
- whether the resource budget ended before the check completed;
- whether the requested result exceeds the tested invariances of the procedure.

The result is one of:

- **assert:** evidence and checks support the result within scope;
- **conditional:** result follows only under named assumptions;
- **ambiguous:** multiple supported alternatives remain;
- **insufficient:** required evidence or computation is missing;
- **contradicted:** relevant evidence conflicts with the proposed result;
- **incomplete:** the budget ended before a reliable status could be assigned.

Abstention is not treated as success or failure by default. It is a behavior whose usefulness must be measured against incorrect confident action, unnecessary refusal, and the cost of obtaining more evidence.

## 34. Self-revision loop

When an outcome contradicts an action prediction, the system does not immediately rewrite the responsible procedure. It performs a bounded revision loop:

1. preserve the outcome and its provenance;
2. identify the action's exact dependencies and scope;
3. determine whether the environment changed, evidence was misclassified, or the procedure failed;
4. generate the smallest permitted repairs;
5. seek a discriminating observation when alternatives remain;
6. version, narrow, suspend, or replace the procedure;
7. retest affected goals and dependent procedures.

A procedure may be wrong in one scope and valid in another. A failed action may also be caused by an external change rather than a bad rule. The revision record must distinguish these possibilities.

## 35. Goal-and-attention falsification tests

The goal system should be tested separately from primitive discovery:

- **Constraint safety:** the planner must not select a high-payoff action that violates a hard constraint.
- **Information efficiency:** hypothesis-separating actions should use fewer risky steps than random exploration.
- **Distant retrieval:** a relevant old dependency must be found despite irrelevant intervening history.
- **Scope discipline:** a procedure learned in one world or time scope must not silently control another.
- **Stochastic caution:** uncertain outcomes must not be reported as certain success.
- **Revision:** a changed transition must trigger revalidation without erasing the previous evidence.
- **Budget honesty:** timeouts must produce incomplete status rather than false negative or positive conclusions.
- **Goal conflict:** incompatible goals must be exposed and resolved by explicit priority or external instruction.

Measurements include goal success, constraint violations, invalid actions, retrieval work, action count, information gained, abstention quality, revision cost, and stale-result rate. A single success percentage is insufficient because a system can achieve tasks by violating safety or by hiding uncertainty.

## 36. Current agent-level conclusion

The proposed brain now has a conceptual loop:

```text
goal or observation
        ↓
agenda and retrieval
        ↓
hypotheses and candidate procedures
        ↓
safe action, observation, or abstention
        ↓
checks and evidence update
        ↓
procedure revision and reusable structure
```

This is still not a complete intelligence theory. It defines how limited attention, explicit goals, uncertainty, and self-revision could sit above the primitive-operation substrate. It does not solve how goals originate, how values are selected, or how the system determines that an observation is trustworthy in an open world.

The next conceptual test should combine the opaque-world protocol with two competing goals and a changed transition rule. That test should reveal whether the architecture can allocate computation, protect constraints, seek discriminating evidence, and revise itself without a hidden reward optimizer.

## 37. Multi-goal arbitration

An intelligent system will receive goals that cannot all be satisfied. The architecture must expose that conflict rather than hide it inside one learned scalar.

Each goal is represented as a constraint set with a source and an authority scope. A goal relation may be:

- **requires:** one goal is a prerequisite for another;
- **prohibits:** satisfying one condition would violate another;
- **refines:** one goal adds detail without changing the parent;
- **competes:** both require resources or states that cannot coexist;
- **supersedes:** an explicit authority or newer instruction replaces an older goal;
- **unknown:** the system lacks evidence to determine compatibility.

Arbitration proceeds in stages:

1. Remove actions that violate non-negotiable safety or authorization constraints.
2. Apply explicit source authority and supersession rules.
3. Satisfy prerequisite goals required by otherwise admissible goals.
4. Find a plan satisfying all compatible goals within the available budget.
5. If goals compete, preserve the alternatives and use declared priority, deadline, reversibility, and external instruction to choose.
6. If no declared rule resolves the conflict, stop and request a decision rather than inventing a preference.

Priority is not the same as value. A deadline may determine which goal is attempted first without making the goal morally or globally more important. Reversibility matters because an action that destroys future options should require stronger authorization than a reversible information-gathering step.

The machine must record both the chosen goal set and the rejected or deferred goals. A successful action that silently abandons a higher-authority goal is a failure even if its local result looks useful.

## 38. World drift and scoped revision

The world may change after a procedure has been learned. A prediction failure therefore has at least three competing explanations:

1. the procedure was wrong;
2. the evidence or scope was misinterpreted;
3. the transition rule changed.

The system detects possible drift through repeated checked residuals: outcomes that disagree with a procedure while its input conditions and evidence interpretation remain stable. One disagreement creates an incident. Repeated independent disagreements create a drift hypothesis. The threshold and independence rule must be declared before evaluation; they must not be adjusted after seeing the result.

When drift is suspected, the machine:

- preserves the old procedure and its successful history;
- marks the affected transition as unstable in the relevant scope;
- creates a new version candidate rather than mutating the old version;
- routes new actions through a safer verification path;
- seeks a discriminating observation between old and new transition hypotheses;
- revalidates dependent plans and derived assertions;
- restores the old version only in scopes where its assumptions remain supported.

A world-level change must not corrupt unrelated knowledge. The dependency index should identify the smallest affected region, but the design must report any uncertain boundary. If the system cannot localize the change, it must quarantine a broader region and declare the resulting cost.

## 39. Computation budgets and fairness

Finite computation must be allocated explicitly. Each goal and hypothesis receives:

- a maximum work budget;
- a reserved safety and revalidation budget;
- a deadline or stopping condition;
- a record of work already spent;
- a preemption rule;
- a result status when work stops.

The scheduler reserves work for:

1. safety checks and authorization;
2. stale-result detection and world-drift handling;
3. imminent goal deadlines;
4. hypothesis-separating observations;
5. ordinary plan expansion and procedure discovery.

No goal may consume the entire machine indefinitely. A deferred goal can regain attention when a new event changes its relevance, but it cannot be silently forgotten. Fairness means every admissible goal receives either work, an explicit reason for deferral, or a request for external prioritization.

Budget allocation should be partly reversible. If a hypothesis is disproven early, its unused budget can return to the agenda. Work already spent is never treated as evidence that the hypothesis deserves to survive.

The system reports resource exhaustion separately from failure. A plan that was not found within its budget is not the same as a plan proven impossible.

## 40. Two-goal changing-world stress protocol

This protocol combines the opaque-world test with goal conflict and transition drift. The learner receives no semantic labels; the evaluator privately knows the latent world.

### 40.1 Initial conditions

The episode provides:

- an opaque observation stream;
- Goal A: achieve a protected target state before a deadline;
- Goal B: distinguish two competing transition hypotheses;
- a hard constraint prohibiting irreversible damage to a protected state;
- a finite action and reasoning budget;
- no supplied priority between Goal A and Goal B.

Initially, the safest action for Goal A depends on which transition hypothesis is true. Goal B can be pursued through an information action that separates the hypotheses but consumes time and may alter an unprotected part of the world.

The learner must identify that the goals compete. It may choose a declared priority if one is supplied by the goal source, or it must ask for a decision. It may not silently optimize a private mixture of success and information.

### 40.2 Mid-episode drift

After the learner has retained a procedure from several successful trials, the evaluator changes one transition rule in a declared hidden scope. The surface symbols remain valid, but the same action now produces a different outcome.

The learner is evaluated on whether it:

1. detects the residual;
2. avoids repeating a potentially unsafe action;
3. preserves the old procedure and evidence;
4. proposes a scoped alternative;
5. seeks a discriminating observation;
6. revalidates Goal A and Goal B;
7. reports incomplete or ambiguous status when the budget ends.

### 40.3 Required outcomes

The test should record:

- whether the hard constraint was ever violated;
- whether either goal was silently dropped;
- the number and quality of distinguishing actions;
- stale-procedure executions after drift;
- scope of the revision;
- old evidence retained and new evidence added;
- computation spent per goal and hypothesis;
- final status of every goal: satisfied, unsatisfied, deferred, ambiguous, contradicted, or incomplete.

## 41. Stress-test failure conditions

The agent-level design fails or requires revision if:

- it resolves an unstated goal conflict with an undisclosed scalar preference;
- it violates a hard constraint because another goal appears more profitable;
- it treats a changed transition as random noise and repeats an unsafe action;
- it globally invalidates valid knowledge after a local world change;
- it silently starves one goal while claiming completion;
- it reports “impossible” when it merely exhausted its budget;
- it performs random exploration despite a safer action that separates the hypotheses;
- it cannot explain which evidence caused a goal or procedure to change;
- it preserves old records but continues using them outside their supported scope.

Exact-example retrieval, a fixed-priority controller, random exploration, and an oracle that sees the hidden world should be reported as separate baselines. The oracle is an upper-bound measurement instrument, not a dependency of the proposed learner.

## 42. Values and goal origin remain open

Explicit goals improve transparency, but they do not answer where goals should come from. A user-facing Jarvis-like system will receive goals from people, applications, sensors, policies, and perhaps internal maintenance needs. The architecture must not silently convert capability into authority.

Goal origin therefore remains a separate interface and governance problem. A later system may learn preferences from demonstrations or corrections, but those preferences require a declared source, scope, revision policy, and protection against accidental generalization. The core learner cannot decide from computation alone what a person ought to want.

The proposed brain can represent and arbitrate goals; it cannot manufacture justified values merely by having memory, reasoning, or self-revision.

## 43. Current agent-level conclusion

The agent layer now has a testable response to competing goals, finite budgets, and world drift:

```text
goals and constraints
        ↓
conflict and authority analysis
        ↓
safety filter and budget allocation
        ↓
plan, investigate, defer, or ask
        ↓
observe outcome and detect drift
        ↓
scope revision and revalidation
```

This makes the architecture more honest but not yet intelligent in the broad sense. It can describe when the system should stop, ask, or revise; it still does not show that useful primitives can be acquired efficiently from unlabelled streams. The opaque-world stress protocol is the next meaningful falsification target before adding natural language or a personal-assistant interface.

## 44. Language-independent formal kernel

The architecture needs a representation that can be implemented in any suitable runtime. The following schemas are conceptual contracts, not a programming-language design.

### 44.1 Reference

```text
Reference = {
  identity,
  store,
  scope,
  version
}
```

Identity is exact within its store. Scope and version prevent a reference from silently referring to a changed or hypothetical item.

### 44.2 Event

```text
Event = {
  identity,
  kind,
  payload-references,
  source,
  scope,
  order,
  observed-time,
  dependencies,
  evidence-status
}
```

An event is append-only at the evidence layer. Later interpretation or correction creates a linked interpretation event; it does not rewrite the original observation.

### 44.3 Evidence record

```text
Evidence = {
  source,
  supports-or-challenges,
  scope,
  provenance,
  independence-basis,
  status,
  recorded-history
}
```

`independence-basis` records why two reports are treated as separate evidence. Repeated copies of one source must not automatically increase support.

### 44.4 Operation

```text
Operation = {
  name-or-identity,
  input-pattern,
  preconditions,
  transformation,
  output-pattern,
  side-effects,
  cost-bound,
  evidence-links
}
```

An operation must declare whether it reads, derives, or changes state. A procedure that changes external state requires authorization and safety checks before execution.

### 44.5 Procedure

```text
Procedure = {
  identity,
  operation-graph,
  input-pattern,
  applicability-conditions,
  output-pattern,
  scope,
  version,
  tested-invariances,
  known-failures,
  evidence-links,
  resource-bound
}
```

The operation graph remains inspectable. A promoted procedure is an indexed composition, not an opaque replacement for its expansion.

### 44.6 World state and interpretation

```text
WorldState = {
  accepted-relations,
  disputed-relations,
  alternative-interpretations,
  active-procedures,
  scope,
  as-of-time
}
```

The state is not just the currently preferred answer. It preserves relevant alternatives and the evidence that separates or fails to separate them.

### 44.7 Goal

```text
Goal = {
  desired-constraints,
  prohibited-constraints,
  source,
  authority,
  scope,
  deadline,
  action-budget,
  evidence-required,
  priority-rules,
  status
}
```

Goal status may be satisfied, unsatisfied, deferred, ambiguous, contradicted, cancelled, or incomplete. Status must not be inferred solely from whether the system stopped working.

### 44.8 Execution trace

```text
Trace = {
  procedure-version,
  input-references,
  operation-steps,
  emitted-events,
  checks,
  dependencies-used,
  resource-spent,
  termination-status
}
```

The trace is the unit used for diagnosis and candidate generation. Without it, structural learning becomes an unexplained change to stored behavior.

## 45. Explicit transition rules

The formal kernel is useful only if state changes have explicit causes.

### 45.1 Ingest

```text
ingest(raw-input, source, scope)
  → append event(s) with provenance and evidence-status = observed
```

The ingest boundary must disclose whether the input was already segmented, typed, or semantically interpreted. An adapter cannot convert hidden semantic work into an “observation” label.

### 45.2 Execute

```text
execute(procedure-version, inputs, world-state, budget)
  → trace, emitted-events, candidate-state, termination-status
```

Execution first checks scope, input compatibility, authorization, and resource bounds. A timeout emits an incomplete result. It does not emit a negative fact.

### 45.3 Check

```text
check(result, evidence, declared-condition)
  → supported | challenged | ambiguous | insufficient | incomplete
```

The check records the exact evidence and condition used. A check cannot silently use an unrecorded human judgment.

### 45.4 Learn

```text
learn(trace, check-result, candidate-grammar, challenge-budget)
  → candidate-procedures | unresolved-diagnostic
```

Learning is a bounded structural edit over the trace and its declared dependency slice. It cannot directly modify unrelated procedures or evidence.

### 45.5 Admit

```text
admit(candidate, challenge-results)
  → provisional-procedure-version | rejected-candidate
```

Admission requires the type, scope, trace, variation, counterexample, and cost gates. A candidate that fails a gate remains available as a rejected hypothesis with its failure evidence.

### 45.6 Revise

```text
revise(procedure-version, new-evidence)
  → unchanged | narrowed-version | suspended-version | replacement-version | unresolved
```

Revision appends a new version and a reason. It does not mutate the old version's evidence history.

### 45.7 Decide

```text
decide(goal, admissible-plans, evidence, budget)
  → act | retrieve | investigate | ask | abstain | report-incomplete
```

The decision must include its selected goal set, constraints checked, procedure versions used, unresolved alternatives, and remaining budget.

## 46. Core invariants

Every future implementation must preserve these properties, even if its internal data structures differ.

### 46.1 Provenance monotonicity

New information may add, challenge, or reinterpret evidence, but it may not silently erase the record of how an earlier conclusion was formed.

### 46.2 Scope confinement

A fact, procedure, goal, or hypothesis may be used only within its declared scope unless an explicit generalization procedure transfers it. Similar surface form is not permission to cross scope.

### 46.3 Hypothesis separation

Competing interpretations remain distinguishable. The system may select one provisionally, but it must preserve the alternatives and the evidence that would separate them.

### 46.4 No unsupported promotion

A procedure cannot become a primitive-like operation without an expansion link, evidence ledger, tested invariances, and known failures.

### 46.5 No timeout-as-fact

Exhausting memory, search, or action budget creates an incomplete status. It cannot create a negative conclusion, an impossibility claim, or a successful result.

### 46.6 Authorized state change

An operation that affects an external state must pass goal, scope, safety, and authorization checks. Predicting a change is not permission to cause it.

### 46.7 Evidence-status honesty

Observed, inferred, simulated, disputed, and untested items remain distinguishable in retrieval and output. A simulated result cannot silently become an observation.

### 46.8 Local revision

When a dependency changes, the system revises the affected region and reports uncertainty about the boundary. It must not globally invalidate or silently preserve unrelated knowledge.

### 46.9 Inspectable decision

Every asserted answer, action, abstention, and revision has an inspectable path containing its inputs, procedure versions, checks, evidence, and resource status.

### 46.10 Stable experimental boundary

The bootstrap operations, challenge generators, and evaluation rules remain fixed during a comparison. Adding hidden parsing or semantic labels after failure changes the experiment rather than improving the same system.

## 47. What the formal kernel guarantees—and what it cannot

The contracts can guarantee that a future implementation records provenance, separates scopes, reports incompleteness, and exposes its reasoning path if the implementation obeys them. They cannot guarantee that:

- the chosen primitive vocabulary is sufficient;
- candidate generation finds a useful procedure;
- the evidence is truthful;
- retrieval is efficient;
- goals are justified;
- a compact procedure is semantically correct;
- the system generalizes outside its tested invariances;
- the architecture is faster than an alternative;
- broad intelligence emerges.

These are empirical questions. The formal kernel prevents certain classes of hidden behavior; it does not create intelligence by definition.

## 48. Minimal language-independent machine loop

The complete conceptual loop can now be expressed without naming a programming language:

```text
receive input
  → append evidence
  → update agenda
  → retrieve scoped dependencies
  → execute admissible procedures
  → check against evidence or goal constraints
  → emit result, uncertainty, or request
  → generate bounded structural candidates
  → challenge candidates
  → admit or reject versioned procedures
  → revalidate affected goals and knowledge
```

This is the current boundary of the design. It is precise enough to write a language-independent specification and a paper-level interpreter, but not yet precise enough to claim an efficient learner. The next useful artifact is a small formal test instance that can be executed manually from these schemas, followed by an adversarial review of whether any operation secretly contains the intelligence we are trying to discover.

## 49. Minimal formal test instance

This instance tests structural discovery and grounded action without exposing the evaluator's semantic vocabulary. It is deliberately small enough to execute by hand.

### 49.1 Hidden evaluator world

The evaluator privately defines a world with a hidden state variable that can take two values. It emits opaque observation sequences and accepts opaque action symbols.

The learner does not receive the words “state,” “toggle,” “target,” or “success.” The evaluator knows that:

- observation pattern `s4 q1 s4` corresponds to hidden state `L`;
- observation pattern `s4 q2 s4` corresponds to hidden state `R`;
- action `a9` changes `L` to `R`;
- action `a9` leaves `R` unchanged;
- action `a8` has no effect;
- the task succeeds when the hidden state is `R`.

The learner sees only symbol sequences and a terminal outcome. The symbols are stable during training and permuted in a held-out family.

The transport layer exposes only episode boundaries, input/output direction, sequence order, and exact symbol identity. It does not expose the hidden state, the role of the middle symbol, or the meaning of either action.

### 49.2 Initial goal and budget

The evaluator supplies a goal packet whose semantic content is intentionally minimal:

```text
Goal G1:
  source: external-task
  scope: episode-1
  success-evidence: terminal-success signal
  action-budget: 3
  reasoning-budget: 20 operation steps
```

The goal tells the learner what evidence counts as success and how much work is available. It does not tell the learner what hidden state to change or which action is useful.

### 49.3 Initial event

The learner receives the opaque stream:

```text
r0 = [s4, q1, s4]
```

The ingest transition creates:

```text
Event E0:
  kind: opaque-observation
  payload: reference(r0)
  source: environment
  scope: episode-1
  order: 0
  evidence-status: observed
```

The learner may use exact identity and order. It may not yet claim that `q1` is a location, state, object, or value.

### 49.4 Competing structural hypotheses

From the initial observation alone, no action effect is known. The learner retains at least these hypotheses:

```text
H1: action a9 changes the middle position from q1 to q2.
H2: action a9 changes the entire sequence according to an unobserved condition.
```

Neither hypothesis is supported yet. The system records them as untested alternatives rather than selecting the simpler one as fact.

### 49.5 First action and observation

The action-selection rule chooses `a9` because it is an available action that can separate H1 from the nearest available alternative under the current exploration budget. This choice is recorded as an investigation, not as a known solution.

```text
Event A0:
  kind: opaque-action
  payload: [a9]
  source: learner
  scope: episode-1
  dependency: G1, H1, H2
  evidence-status: executed
```

The environment returns:

```text
r1 = [s4, q2, s4]
```

The learner creates observed event `E1` and compares `r0` with `r1` using exact identity and position. It can state:

```text
After action a9, the second position changed from q1 to q2;
the first and third positions remained s4.
```

It still cannot state that a location changed. That would be a semantic interpretation not yet grounded by this trace.

### 49.6 Candidate procedure

The bounded edit grammar proposes:

```text
P1:
  when the observed sequence has the form [X, q1, X]
  and action a9 is executed,
  predict [X, q2, X]
```

`P1` is a structural transformation with a variable `X`. It is not yet admitted. The trace contains:

- input reference `r0`;
- action reference `A0`;
- output reference `r1`;
- changed-position relation;
- unchanged-position relations;
- hypotheses H1 and H2;
- reasoning and action budget consumed.

### 49.7 Discriminating challenge

The learner uses its second action budget unit to request or encounter a held-out context:

```text
r2 = [s5, q1, s5]
```

It executes `a9` and receives:

```text
r3 = [s5, q2, s5]
```

This supports variable substitution for the repeated outer symbol. A second candidate, `P2`, which memorizes only `[s4, q1, s4] → [s4, q2, s4]`, is weaker because it cannot explain the held-out context.

### 49.8 Negative challenge and admission

The learner receives:

```text
r4 = [s5, q2, s5]
```

and executes `a9`. The environment returns the unchanged sequence:

```text
r5 = [s5, q2, s5]
```

The negative challenge prevents the learner from turning every `a9` action into an unconditional change to `q2`. `P1` passes the limited challenge family and is admitted provisionally as `P1.v1`.

The admission record states:

- tested transformation: repeated outer context with `q1 → q2` under `a9`;
- tested negative: already-`q2` context remains unchanged;
- untested: other outer symbols, longer sequences, delayed observations, symbol permutation, stochastic outcomes, and action `a8`;
- status: provisionally usable, not semantically named.

### 49.9 Goal result

The evaluator emits terminal success after the first action because the hidden state became `R`. The learner may record:

```text
Goal G1: satisfied within episode-1 under the observed terminal signal.
```

It may not claim that it understands the hidden state or that `a9` is universally useful. The success is scoped to the tested episode and procedure evidence.

## 50. Formal episode review

The episode exercises the formal kernel as follows:

| Kernel element | Trace use |
| --- | --- |
| Reference | Identifies `r0`, `r1`, `A0`, and later observations without copying their contents |
| Event | Records opaque observations, action, order, scope, and evidence status |
| Evidence | Links the changed and unchanged positions to the candidate procedure |
| Goal | Supplies terminal success evidence and a finite budget without supplying the solution |
| Procedure | Represents the variable-based transformation `P1` |
| World state | Preserves the learner's alternative structural hypotheses |
| Trace | Records operation steps, comparisons, candidates, and consumed budget |

The episode demonstrates only a narrow form of structural and grounded learning. It does not demonstrate that the learner discovered the evaluator's words “state,” “toggle,” or “target.” It also does not demonstrate that the candidate-generation policy will find `P1` in a large search space; the hand trace assumes the declared bounded edit grammar can enumerate the relevant comparison.

## 51. Adversarial audit of hidden intelligence

The formal kernel must be challenged by asking where the missing intelligence could be hiding.

### 51.1 Receive and packet boundaries

If the input adapter supplies semantic event boundaries, fields, entity identities, or meaningful tokens, it has already solved part of perception. The test must disclose whether boundaries came from the environment, transport protocol, or learner.

### 51.2 Identify and order

Exact identity and order are legitimate substrate operations, but they are not free in physical systems. Symbol storage, equality testing, synchronization, and time representation carry cost. A future benchmark must measure them rather than treating them as constant magical primitives.

### 51.3 Group and variable binding

The `Group` and `Bind` operations can hide a large search. If the system is allowed to try every partition, alignment, or variable mapping, the architecture may only have moved brute force into a new name. Candidate counts, edit depth, and mapping work must be exposed.

### 51.4 Check and terminal outcome

`Check` is not allowed to know the hidden semantics. It may compare a result with an explicitly supplied observation or terminal signal. If it evaluates “correctness” using the evaluator's latent world state, the experiment becomes supervised.

### 51.5 Goal packet

The goal may specify success evidence, but it must not encode the hidden solution. A goal such as “make the middle symbol q2” would leak the target. The opaque-world protocol therefore uses a terminal signal and keeps the hidden success condition in the evaluator.

### 51.6 Candidate generation

The most likely hiding place is candidate generation. A manually chosen candidate is not learned. The test must log every candidate produced by the declared edit grammar, every candidate rejected by type or scope, and the work needed to produce them.

### 51.7 Candidate ranking

Selecting the shortest or most reusable procedure is an inductive bias, not neutral reasoning. The bias must be fixed before evaluation. A compact wrong procedure must be able to lose to a more complex procedure when counterexamples require it.

### 51.8 Retrieval

“Retrieve relevant memory” cannot be an oracle. The system must show the indexes and structural transformations used, the alternatives considered, and the work spent. Similarity without an auditable transformation is a proposal, not evidence.

### 51.9 Terminal success and action selection

Terminal success is external evidence about the task, not a learned value function. It can ground consequences, but it does not tell the system why an action worked. Action selection must report whether it was random, exploratory, procedure-driven, or goal-driven.

### 51.10 Semantic naming

Mapping `q1` and `q2` to words such as “left” and “right” is not part of the episode. Any naming interface must be evaluated separately and must not be used to inflate the claim about structural learning.

## 52. Audit verdict

The formal episode is internally executable under the stated rules, but it exposes three major unresolved costs:

1. candidate generation may still contain combinatorial search;
2. grouping and variable binding may conceal perception and abstraction work;
3. terminal outcomes ground consequences but do not by themselves provide semantic understanding.

These are not reasons to discard the architecture. They are the exact proof obligations that must be measured. The next revision should define a mechanically enumerable candidate grammar for the opaque episode and count every operation needed to find `P1`, including failed candidates.

The architecture should not advance to natural-language ingestion until it can pass this audit without hidden semantic fields or an oracle candidate generator.

## 53. Mechanically enumerable candidate grammar

The opaque episode needs a grammar that can generate the same candidate list every time. The initial grammar is intentionally weak.

### 53.1 Candidate form

```text
CandidateRule = {
  action-pattern,
  input-pattern,
  output-transformation,
  applicability-conditions,
  support-traces
}
```

For a fixed-length symbol sequence, an input pattern may contain only:

- an exact symbol observed in a support trace;
- a variable copied from an identical position or an explicitly repeated position;
- an exact sequence length observed in a support trace.

An output transformation may only:

- copy an input position;
- emit a symbol observed at the corresponding output position;
- preserve a position shown to be unchanged;
- replace a position shown to change in every support trace.

The grammar cannot invent symbols, semantic names, arbitrary predicates, or hidden state fields. A new operation or richer condition belongs to a later grammar version and must be declared before evaluation.

### 53.2 Enumeration stages

Candidates are enumerated in this order:

1. **Exact replay:** one rule for each observed action/input/output transition.
2. **Aligned change:** compare same-length positive transitions and record changed and unchanged positions.
3. **Repeated-position variable:** replace positions with a variable only when the same equality pattern occurs across at least two positive traces and the output behavior remains stable.
4. **Context relaxation:** remove an equality or constant condition only after the stricter candidate has passed its current challenge set.
5. **Case split:** create separate candidates when one rule cannot explain incompatible outcomes.

The learner does not jump directly to stage 4 because it is attractive to describe a more general rule. Progressive widening makes the cost of generalization visible.

### 53.3 Candidate ordering and identity

Candidates are ordered by:

1. grammar stage;
2. number of edits;
3. first support-trace identity;
4. action identity;
5. input-position order;
6. symbol identity order defined by the transport layer.

Two candidates are identical only if their canonical structural signatures and support-trace references are identical. Renaming a candidate does not reduce its complexity.

### 53.4 Candidate set for the formal episode

The positive transitions are:

```text
T1: [s4, q1, s4] --a9--> [s4, q2, s4]
T2: [s5, q1, s5] --a9--> [s5, q2, s5]
```

The first enumeration stage produces:

```text
C1: exact T1
C2: exact T2
```

The aligned-change stage observes that the middle position changes from `q1` to `q2` while the outer positions remain unchanged in both transitions. It produces:

```text
C3: [X, q1, X] --a9--> [X, q2, X]
```

where `X` is admitted only because the first and third positions are equal in both positive traces.

The context-relaxation stage may later produce:

```text
C4: [X, q1, Y] --a9--> [X, q2, Y]
```

`C4` weakens the equality condition and therefore requires a new challenge. It is not admitted merely because it is more general.

## 54. Cost ledger for candidate generation

The architecture must count work in symbolic operation units before any hardware implementation is selected. Let:

- `N` be the number of input and output sequences;
- `m` be the maximum sequence length;
- `T` be the number of aligned positive transitions;
- `C` be the number of generated candidates;
- `H` be the number of challenge traces per candidate;
- `g` be the number of admission gates.

A first-order ledger is:

```text
ingest work       = N
alignment work    = T × m
change analysis   = T × m
candidate work    = C × grammar-cost
challenge work    = C × H × execution-cost
admission work    = C × g
```

This is not a wall-clock prediction. It identifies which quantities must be measured and prevents “candidate generation” from being treated as free.

For the formal episode before context relaxation:

```text
positive transitions T = 2
sequence length m      = 3
exact candidates       = 2
shared-variable candidates = 1
admission candidates   = 3 before rejection or promotion
```

If `C4` is opened, the ledger increments by one candidate plus its full challenge set. Failed candidates remain in the ledger; they are not discarded from the cost report.

The actual implementation must additionally measure memory reads, reference comparisons, variable mappings, branch count, and peak working memory. A sparse graph can have low logical event count but poor hardware locality. The conceptual count does not imply a GPU speedup.

## 55. Replay under the declared grammar

The opaque episode is replayed without manually selecting `P1`.

### 55.1 Ingest and action enumeration

The learner ingests `r0` and enumerates the available opaque actions `{a8, a9}`. The first action choice is recorded as exploration. If the action policy prefers the action that separates the current hypotheses, it chooses `a9`; if it has no basis to prefer either action, it may choose by a declared tie-break rule. The action-selection work and tie-break must be reported.

After `a9` returns `r1`, the learner has one positive transition. Exact replay `C1` is generated. It is not yet generalized.

### 55.2 Second positive transition

The learner receives or requests a new context and obtains `T2`. Exact replay `C2` is generated. Aligned change analysis compares `T1` and `T2` position by position:

| Position | T1 input/output | T2 input/output | Structural result |
| --- | --- | --- | --- |
| 1 | `s4 → s4` | `s5 → s5` | copied, equal within each trace |
| 2 | `q1 → q2` | `q1 → q2` | stable replacement |
| 3 | `s4 → s4` | `s5 → s5` | copied, equal within each trace |

The only repeated equality pattern is positions 1 and 3. The grammar therefore generates `C3`, not a human-selected `P1`.

### 55.3 Challenge and admission

The `q2` negative transition challenges `C1`, `C2`, and `C3`:

- `C1` fails outside its exact context and is retained only as a memorized trace.
- `C2` fails outside its exact context and is retained only as a memorized trace.
- `C3` does not apply because its input condition requires `q1`; it passes this negative challenge.

The grammar then opens context relaxation and generates `C4`. A further challenge is required:

```text
T4: [s7, q1, s8] --a9--> [s7, q1, s8]
```

`C4` predicts a change and fails. It is rejected. `C3` is admitted as `P1.v1` with the exact challenge history.

### 55.4 Replay result

The procedure emerges through the declared sequence:

```text
exact examples → aligned change → repeated-variable generalization → challenge → admission
```

No semantic label is required. However, the grammar still assumes that sequences can be aligned and that positive transitions can be identified. Those assumptions are now explicit and remain proof obligations rather than hidden achievements.

## 56. Grammar and cost audit

The mechanically enumerable replay improves the design in three ways:

- another person can reproduce the candidate list;
- failed candidates and their costs remain visible;
- generalization is opened progressively instead of being selected by intuition.

It also exposes remaining weaknesses:

- alignment is easy for equal-length toy sequences but difficult for raw streams;
- repeated-position variables are only one form of abstraction;
- action selection may still require exploration policy beyond the grammar;
- challenge generation must be independent of the candidate generator to avoid weak tests;
- the evaluator's positive and negative transitions still provide external supervision;
- a useful grammar can become expensive as sequence length and variable partitions grow.

The next implementation-independent proof obligation is therefore not “train on more data.” It is to define how raw stream segmentation and alignment are discovered while preserving the same cost ledger and audit rules.

## 57. Raw stream boundary

The learner's lowest-level input is an ordered stream of opaque atoms:

```text
Stream = [atom_0, atom_1, atom_2, ...]
```

An atom has exact identity, arrival order, source, and optional physical metadata such as duration or signal strength. It has no semantic type. A byte, character, sensor sample, or device signal may be an atom, depending on the input adapter.

The transport layer may provide only information that is physically necessary to deliver the stream:

- stream start and end;
- source channel;
- packet loss or corruption markers;
- arrival order;
- explicit external action boundaries when the environment itself defines a response cycle.

The transport layer may not provide words, entities, event types, fields, grammatical roles, or task-relevant boundaries unless those are intentionally part of the experiment and counted as supervision.

If an input adapter performs denoising, compression, tokenization, segmentation, or feature extraction, those operations must be included in the experiment's bootstrap and cost ledger. Calling the result an “observation” does not make the work disappear.

## 58. Boundary-candidate grammar

The learner does not commit to one segmentation immediately. It generates a bounded set of boundary hypotheses.

### 58.1 Candidate boundary sources

Initial boundary candidates may be proposed from:

1. **Transport edges:** stream start, stream end, and explicitly declared response boundaries.
2. **Recurrence:** repeated subsequences that occur with stable internal order.
3. **Contrast:** positions or spans that change after an action while neighboring material remains stable.
4. **Intervention:** spans whose presence predicts a different observable outcome under a controlled action.
5. **Timing:** declared gaps or duration discontinuities, treated as physical signals rather than semantic separators.
6. **Compression improvement:** a boundary that permits a shorter tested structural description.

No source is automatically correct. A repeated substring may be accidental, a timing gap may be irrelevant, and an intervention effect may be caused by a larger surrounding span.

### 58.2 Boundary candidate

```text
BoundaryCandidate = {
  start-position,
  end-position,
  proposal-source,
  supporting-traces,
  contradicting-traces,
  scope,
  cost,
  status
}
```

Candidates remain alternatives until a later procedure demonstrates that a boundary is useful and stable. The learner may operate on several segmentation hypotheses if the budget allows. It must report when it prunes alternatives.

### 58.3 Bounded segmentation stages

Segmentation proceeds in stages:

1. **Atomic view:** treat every atom as separate and preserve the original stream.
2. **Observed-span view:** propose repeated contiguous spans up to a declared maximum length.
3. **Action-contrast view:** propose spans that differ consistently before and after an action.
4. **Nested-span view:** permit a stable span to contain smaller candidate spans.
5. **Boundary relaxation:** merge or split spans only when the change improves held-out structural behavior within budget.

The system cannot enumerate every possible partition of a long stream. It must declare maximum span length, candidate count, merge/split depth, and pruning rule. A segmentation failure may be a budget failure rather than evidence that no useful boundary exists.

## 59. Alignment without semantic labels

Alignment compares candidate spans or streams across traces. It does not assume that the same position has the same meaning.

An alignment candidate contains:

```text
Alignment = {
  left-references,
  right-references,
  correspondence-type,
  unchanged-pairs,
  changed-pairs,
  insertions,
  deletions,
  supporting-traces,
  cost,
  ambiguity-set
}
```

The initial correspondence types are deliberately structural:

- exact atom identity;
- exact span identity;
- equal order;
- repeated-span identity;
- unchanged-before-and-after relation;
- changed-after-action relation.

An alignment may contain gaps or multiple alternatives. The learner cannot name an aligned span as “the object” or “the verb” until a grounded procedure gives that role evidence.

### 59.1 Alignment generation

For two candidate streams, alignment is generated in this order:

1. exact same-length positional comparison;
2. exact contiguous-span matches;
3. repeated-span matches with preserved order;
4. bounded insertion/deletion alternatives;
5. alignments justified by an action-linked change pattern.

Each alternative receives a structural cost based on the declared operation count. A lower-cost alignment is not automatically true. It is a candidate that must survive independent traces and counterexamples.

### 59.2 Ambiguous alignments

If two alignments explain the same observed traces, both remain available. The learner should seek a trace where they predict different segment boundaries or outcomes. If no affordable observation separates them, the result remains ambiguous.

Selecting one alignment solely because it produces a simpler explanation is an explicit inductive bias. The bias must be recorded and held fixed across evaluation.

## 60. Raw-stream replay of the opaque episode

The earlier opaque episode used already separated sequences such as `[s4, q1, s4]`. The raw-stream version removes that assumption.

The learner receives:

```text
raw_0 = s4 q1 s4
```

and, after action `a9`, receives:

```text
raw_1 = s4 q2 s4
```

At the atomic stage, the learner knows only that both streams contain three atoms. It may propose:

- a three-atom span;
- three one-atom spans;
- repeated outer spans if exact recurrence supports them;
- a middle span whose identity changes after `a9`.

The learner must retain the original streams and record the work used to propose the three-position alignment. It cannot simply assume that the spaces in the written notation are meaningful separators.

After a second trace:

```text
raw_2 = s5 q1 s5
raw_3 = s5 q2 s5
```

the recurrence and action-contrast stages can support a structural alignment with a repeated outer relation and a stable middle-position change. This supports the candidate pattern:

```text
[X, q1, X] --a9--> [X, q2, X]
```

The alignment remains a structural hypothesis. It becomes a procedure only after the candidate and its boundary assumptions pass independent challenges.

## 61. Segmentation and alignment cost ledger

The earlier candidate ledger is extended with raw-stream work. Let:

- `L` be total atom count;
- `B` be boundary candidates;
- `S` be retained segmentation hypotheses;
- `A` be alignment candidates;
- `D` be the maximum span length;
- `R` be the maximum insertion/deletion depth.

The first-order symbolic cost is:

```text
atom ingest       = L
span proposals    = B × D
segmentation work = S × boundary-check-cost
alignment work    = A × (D + R)
candidate work    = previous candidate ledger
```

For nested spans and variable mappings, the ledger must also include branch count and peak working memory. The system must report pruned candidates and the rule that pruned them. Otherwise a fast result could simply be a result from silently ignoring difficult alternatives.

## 62. Raw-stream falsification tests

The segmentation and alignment mechanism should be tested before semantic naming or natural-language claims.

- **Boundary holdout:** useful spans have lengths and positions not seen during discovery.
- **Delimiter removal:** remove any visually convenient separators and verify that performance does not depend on them.
- **Distractor insertion:** insert irrelevant atoms inside and around useful spans.
- **Variable length:** vary stream length while preserving the relevant transformation.
- **Ambiguous segmentation:** provide streams supporting two equally plausible partitions and require an ambiguity report.
- **Permutation:** permute symbol identities while preserving structure.
- **Delayed effect:** separate an action from its observable consequence.
- **Contrasting alignment:** create two alignments that agree on training traces but differ on a held-out trace.
- **Noise and loss:** introduce declared corruption or missing atoms and measure whether the learner preserves uncertainty.
- **Scaling:** increase atom count, maximum span length, candidate boundaries, and alignment alternatives while measuring work and memory.

Success requires a reusable grounded procedure whose boundary and alignment hypotheses transfer to held-out streams. A system that finds a shorter description only because it was given the right delimiters has not solved segmentation.

## 63. Current perception conclusion

The architecture now treats segmentation and alignment as learned, versioned structural hypotheses rather than invisible preprocessing. This is more faithful to the from-scratch goal, but it substantially increases the search problem.

The immediate consequence is important: the proposed brain is not automatically cheaper than deep learning. It may avoid dense global updates while spending significant resources on discrete boundary, alignment, hypothesis, and counterexample search. The one-consumer-GPU target remains a hypothesis to measure, not an expected result.

The next design decision is whether to keep raw stream discovery completely discrete or permit a separate low-level analog/sensory adapter. That adapter could improve input handling, but it would become another learned subsystem whose supervision and compute must be disclosed.

## 64. Perception-boundary options

There are three plausible ways to connect continuous sensors to the structural core.

### 64.1 Discrete-only core

The system accepts bytes, characters, or already quantized symbols and performs all segmentation and abstraction internally.

**Advantages:** cleanest experiment; no hidden feature extractor; easiest provenance.  
**Risks:** unrealistic for microphones, cameras, robotics, and analog sensors; low-level signal processing may dominate the budget.

### 64.2 Integrated learned sensory layer

A learned system converts images, audio, or sensor waveforms directly into event packets consumed by the core.

**Advantages:** practical input handling; potentially efficient for noisy continuous signals.  
**Risks:** the sensory layer may learn objects, words, actions, or goals before the structural core sees them. The project could quietly become a conventional learned model attached to a symbolic interface.

### 64.3 Separated sensory adapter

A separately specified adapter converts a physical signal into auditable low-level atoms, while the structural brain discovers spans, relations, and procedures above them.

**Advantages:** preserves a clear research boundary; allows different sensor technologies; makes preprocessing cost visible.  
**Risks:** the adapter may still contain substantial learned intelligence; the boundary can become bureaucratic rather than scientifically meaningful if its outputs are too rich.

### 64.4 Recommendation

Use the **separated adapter** architecture, beginning with a deterministic adapter for controlled experiments. The core should first learn from exact symbols and declared physical measurements. Later learned adapters may be added, but they must be treated as independent subsystems with their own training data, costs, benchmarks, and leakage audits.

This keeps the central hypothesis testable:

> Can the structural brain discover reusable operations above a minimally interpreted signal representation?

It also avoids pretending that converting a waveform into useful units is free.

## 65. Sensory adapter contract

An adapter receives a physical signal and emits low-level atoms plus a transformation record.

```text
AdapterOutput = {
  atom-stream,
  source-channel,
  order-and-time,
  quantization-or-loss,
  uncertainty-and-corruption,
  preprocessing-history,
  adapter-version,
  compute-and-memory-cost
}
```

The adapter may:

- sample, rescale, quantize, or packetize a physical signal;
- preserve timing, amplitude, frequency, spatial coordinate, or channel identity when those are directly measured;
- mark missing, saturated, corrupted, or uncertain measurements;
- apply a declared deterministic filter whose effect is reproducible;
- expose multiple representations while preserving links to the source signal.

The adapter may not silently emit:

- object identities;
- word or phoneme labels;
- event meanings;
- grammatical roles;
- causal relations;
- user goals;
- task success;
- semantic similarity;
- a pretrained representation whose training history is not disclosed.

If a learned adapter emits any of these, the result is not a low-level adapter for the structural experiment. It is a separate learned perception system and must be evaluated as such.

## 66. Adapter and core separation

The structural core receives adapter outputs as observed measurements, not as facts about the world. For example:

```text
Adapter output:
  channel: microphone-1
  time: 4.21 s
  atoms: [0.18, 0.22, 0.19, ...]
  uncertainty: declared measurement bounds
```

The core may discover recurring signal spans or relationships between actions and signal changes. It may not call a span “speech,” “door,” or “person” without evidence from a grounded procedure or an external label.

An adapter's preprocessing history remains attached to all downstream events. If a later core procedure depends on a filtered signal, the evidence path must show that dependency.

When multiple adapters describe the same physical episode, their outputs must retain source identities. Agreement between two outputs is not automatically independent evidence if both were produced from the same raw signal or shared preprocessing.

## 67. Adapter leakage audit

Every adapter must pass a leakage review before it is used to claim structural learning:

- **Label audit:** list all labels, annotations, pretrained data, and objectives used by the adapter.
- **Output audit:** inspect whether output fields directly encode semantic entities, actions, or goals.
- **Permutation audit:** change surface symbols or sensor channels and verify that the core is not relying on fixed semantic IDs.
- **Raw comparison:** compare the adapter output with a minimally processed signal and report which information was removed or added.
- **Ablation:** remove the adapter or replace it with a simpler deterministic version and measure which capabilities disappear.
- **Cost accounting:** include adapter training, inference, memory, energy, and latency in total resource measurements.
- **Transfer test:** evaluate new devices, noise profiles, sampling rates, and channel layouts.
- **Provenance test:** verify that every core event can be traced back to adapter output and, where retained, raw signal ranges.

An adapter that makes the core appear intelligent by supplying rich semantic features has failed the boundary audit even if the complete system performs well.

## 68. Perception experiment ladder

Perception should be introduced in increasing difficulty:

1. **Exact symbolic atoms:** test the structural core without a sensory adapter.
2. **Deterministically quantized signals:** expose raw scalar or categorical measurements with declared quantization.
3. **Noisy continuous signals:** add corruption, missing values, and timing variation.
4. **Multiple channels:** require cross-channel alignment without semantic labels.
5. **Learned adapter:** add a separately audited adapter only after the core passes earlier stages.
6. **Open-ended sensor stream:** test transfer, ambiguity, drift, and resource scaling.

Each stage must report what was supplied by the environment, adapter, core, and evaluator. A result at stage 5 cannot be compared to a stage-1 result without accounting for the extra learned subsystem.

## 69. Perception falsification criteria

The separated architecture requires revision if:

- the core succeeds only when the adapter supplies semantic labels or boundaries;
- adapter preprocessing cost exceeds the claimed savings from the structural core;
- transfer to a new sensor fails because the system memorized channel identities;
- uncertainty from corrupted input is converted into confident world facts;
- downstream procedures cannot trace their evidence through the adapter;
- two correlated adapter outputs are counted as independent confirmation;
- removing a hidden pretrained adapter causes the claimed intelligence to disappear;
- a deterministic low-level representation cannot support any reusable structural learning under controlled conditions.

## 70. Current perception decision

The project should not build a complete multimodal assistant yet. The conceptual order is:

```text
exact symbols
  → raw discrete streams
  → deterministic sensor atoms
  → noisy and multi-channel streams
  → independently audited learned adapters
  → user-facing multimodal agent
```

This preserves the from-scratch goal without denying that real perception is difficult. It also makes a negative result useful: we can determine whether the failure lies in signal conversion, segmentation, structural learning, grounding, or goal-directed action.

## 71. Memory topology without a fixed context window

The memory design must separate persistence from active computation. A system can retain information outside its working set, but it cannot inspect all retained information simultaneously at constant cost.

The proposed topology has five layers:

1. **Evidence ledger:** append-oriented records of observations, actions, sources, timestamps, and provenance.
2. **Structural index:** references to recurring spans, relations, dependencies, scopes, and tested transformations.
3. **Procedure store:** versioned executable compositions with applicability conditions and known failures.
4. **Active workspace:** the bounded set of events, hypotheses, and procedures currently being compared or executed.
5. **Archive:** compressed, cold, or externally stored material with an explicit retrieval cost and loss record.

The active workspace is bounded. The whole memory is not represented as one prompt. Long history is accessed through staged retrieval and streamed expansion of relevant evidence.

Every index entry points back to its source records. An index is a routing aid, not independent evidence. If an index is stale or incomplete, the memory system must report that limitation.

## 72. Retrieval and context expansion

Retrieval begins from a goal, event, failed check, procedure input, or explicit user reference. It expands context in bounded slices:

1. exact references and current scope;
2. direct dependencies and inverse dependencies;
3. relevant procedure versions and their tested conditions;
4. supporting and contradicting evidence;
5. related episodes through declared structural transformations;
6. archived material only when the nearer layers cannot resolve the question.

Each slice includes its source, reason for retrieval, scope, version, and cost. The system may process a sequence of slices rather than placing all evidence in one active buffer.

If a conclusion depends on a slice that was not retrieved, the result is conditional or incomplete. The system must not treat “not currently in workspace” as “does not exist.”

Retrieval can be iterative:

```text
query
  → retrieve slice
  → inspect dependencies
  → expand or narrow slice
  → check evidence
  → answer, continue, or report budget exhaustion
```

This removes the fixed prompt-history requirement but not the cost of finding relevant information. A long history can still produce latency, ambiguity, or retrieval failure.

## 73. Consolidation and abstraction

Consolidation creates a reusable procedure or structural summary from multiple records. It must preserve:

- source references;
- coverage conditions;
- tested invariances;
- known counterexamples;
- information intentionally discarded;
- scope and time validity;
- procedure version;
- consolidation cost.

A summary is not a replacement for its evidence unless the retention policy explicitly allows loss. When exact records are compressed, the system must record what cannot be reconstructed from the summary.

An abstraction is admitted only when it improves reuse or retrieval on held-out cases without hiding contradictory evidence. A compact summary that deletes inconvenient counterexamples is invalid, even if it improves average task performance.

Consolidation may run incrementally or during idle time, but its work must not block urgent safety, retrieval, or revision tasks. A procedure created during consolidation remains provisional until it passes the same challenge and admission gates as an online candidate.

## 74. Retention, deletion, and forgetting

Finite storage requires explicit retention classes:

- **protected exact:** retain the original record and provenance;
- **exact but cold:** retain externally or in an archive with retrieval cost;
- **lossy summary:** retain an abstraction and a record of information loss;
- **reconstructible index:** retain routing information that can be regenerated;
- **deletable:** remove according to policy, authorization, and legal or user instruction.

Deletion must create an auditable tombstone or deletion record without retaining a hidden usable copy. The exact treatment depends on the security and privacy requirements of a later implementation, but the conceptual design cannot claim deletion while silently preserving the content in summaries or indexes.

For this project, **catastrophic forgetting** is defined operationally:

> A previously validated procedure, fact, or constraint cannot be retrieved and used correctly under its supported scope after later learning, despite the relevant record still being retained and the normal retrieval budget being sufficient.

This definition separates forgetting from legitimate revision. If new evidence invalidates an old procedure, suspending it is not forgetting; failing to preserve its history and reason for suspension is.

The system should measure both:

- **storage retention:** whether the original or permitted summary still exists;
- **functional retention:** whether the system can retrieve, interpret, and apply it correctly.

Storage retention without functional retrieval is not success.

## 75. Memory conflict handling

Memory does not assume that one permanent fact exists for every query. It may contain:

- time-varying states;
- source disagreements;
- scoped truths;
- hypothetical branches;
- stale procedures;
- unresolved interpretations.

When a query reaches conflicting records, retrieval returns the alternatives and their evidence status. The system may prefer one according to declared scope, source authority, recency, or corroboration rules, but it must preserve the conflict and disclose the preference rule.

A new observation can update an interpretation without rewriting the old observation. Derived summaries that depend on the changed interpretation are marked for revalidation. If revalidation is incomplete, their outputs become conditional or stale rather than silently current.

## 76. Memory test suite

Memory should be evaluated as a retrieval-and-use system, not merely as storage.

- **Long-history needle:** place decisive evidence far before many irrelevant episodes; test retrieval accuracy and cost.
- **Interference:** teach many new procedures with overlapping surface forms; retest old procedures through ordinary retrieval.
- **Scope separation:** use the same symbols or roles in different worlds, users, or time periods; test for leakage.
- **Contradiction:** introduce reliable and unreliable conflicting reports; test alternative preservation and authority rules.
- **Revision:** change a transition rule; test local invalidation and unaffected knowledge retention.
- **Archive return:** move relevant evidence to cold storage; test whether the system retrieves it when required and reports cost.
- **Lossy summary:** compress evidence; test whether the system refuses claims that require discarded detail.
- **Deletion:** remove authorized information; test that active indexes, procedures, and summaries no longer use it.
- **Order robustness:** interleave learning episodes in different sequences; measure functional retention.
- **Scaling:** increase history length, index size, dependencies, and conflict count while measuring latency, memory, and unresolved cases.

Required measurements include:

- exact-record retention;
- functional retrieval accuracy;
- procedure reuse accuracy;
- stale-result rate;
- false absence rate;
- retrieval work and latency;
- archive access cost;
- revalidation work;
- storage growth;
- information loss after consolidation;
- deletion completeness.

## 77. Memory falsification criteria

The memory architecture requires revision if:

- it retains records but cannot retrieve them under the declared budget;
- new learning causes old supported procedures to become inaccessible without a recorded reason;
- an index is treated as proof rather than a routing aid;
- a summary silently deletes counterexamples or uncertainty;
- conflicts are collapsed into one answer without a declared resolution rule;
- stale results survive known dependency changes;
- archive retrieval is never attempted despite missing active evidence;
- deletion leaves usable information in summaries, indexes, or procedures;
- long-history performance depends on keeping the entire history in the active workspace.

## 78. Current memory conclusion

The architecture now has a concrete answer to “no fixed context limitation”:

> Knowledge persists in a provenance-linked memory topology, while reasoning retrieves bounded slices and expands them as needed.

This is not infinite context. It is persistent, queryable, versioned memory with finite retrieval cost. The design avoids the claim that a larger prompt alone creates memory, while accepting the physical limits of storage, indexing, bandwidth, and attention.

The next proof obligation is to run a paper-scale long-history episode where a previously learned procedure must be retrieved after interference, a contradiction must be scoped, and an archived detail must be recovered without loading the entire history into active workspace.

## 79. Paper-scale long-history episode

This episode tests memory as a persistent retrieval-and-use system. It is called “paper-scale” because the records can be generated and inspected without requiring a full runtime. The episode should later become an executable benchmark with the same scopes, retention classes, and budgets.

### 79.1 Initial procedure

In world scope `W1`, the structural learner has already admitted:

```text
P1.v1:
  when [X, q1, X] is observed
  and action a9 is executed,
  predict [X, q2, X]
```

Its evidence includes:

```text
E1: [s4, q1, s4] --a9--> [s4, q2, s4]
E2: [s5, q1, s5] --a9--> [s5, q2, s5]
E3: [s5, q2, s5] --a9--> [s5, q2, s5]
```

The procedure record includes its tested invariances, known failures, scope `W1`, version `v1`, and dependencies on `E1`, `E2`, and `E3`.

### 79.2 Consolidation and archive transition

The system consolidates the procedure into a reusable index entry:

```text
Index I1:
  key: action a9 + repeated outer context + q1 input
  points-to: P1.v1
  scope: W1
  evidence-summary: E1, E2, E3
  archive-location: cold-block-17
  information-loss: none for the tested fields
```

The exact event records `E1` through `E3` are moved to the archive. The active workspace retains only the current query, the procedure reference, the index path, and the summary. The archive retains exact records and their provenance.

### 79.3 Interference history

The system then receives a large sequence of unrelated episodes:

- procedures about different action symbols;
- the same symbols in scopes `W2` through `W100`;
- irrelevant streams with repeated outer patterns;
- failed candidates and rejected procedures;
- contradictory reports unrelated to `W1`;
- new goals and expired goals.

The evaluator records the exact number of events, procedures, scopes, index entries, and conflicts. “Large” must be defined before testing; it cannot mean whatever size makes the result look good.

The active workspace is cleared between episodes. The procedure store and structural index remain available. The full history is not loaded into the workspace.

## 80. Long-history replay

### 80.1 Ordinary reuse query

After interference, the learner receives in scope `W1`:

```text
current observation: [s8, q1, s8]
action: a9
goal: predict the resulting opaque stream
```

The retrieval path is:

1. exact current scope `W1`;
2. procedure signature for action `a9` and repeated outer context;
3. `P1.v1` and its tested conditions;
4. the current input compatibility check;
5. the procedure's stored challenge history if required.

The system retrieves `P1.v1` without scanning all distractor episodes. It predicts:

```text
[s8, q2, s8]
```

The environment returns the expected stream. The system records functional reuse, retrieval work, latency, and the fact that the exact supporting events were not needed in active workspace for this ordinary application.

### 80.2 Exact-evidence query

The user or evaluator then asks:

```text
Which observations originally supported P1.v1?
```

The procedure record points to `E1`, `E2`, and `E3`; the index points to `cold-block-17`. The memory system retrieves that archive block and streams the exact evidence into the workspace in bounded slices.

The answer includes:

- the three original observations;
- their scopes and provenance;
- the challenge result;
- the information that was not tested;
- archive access cost;
- any integrity or retrieval warnings.

The system does not load the unrelated history. If the archive block is unavailable or the budget expires, it reports incomplete evidence rather than inventing a summary.

### 80.3 Scoped contradiction

The evaluator introduces a new transition in scope `W2`:

```text
E101: [s8, q1, s8] --a9--> [s8, q1, s8]
scope: W2
status: observed
```

This challenges the general use of `P1`, but it does not directly contradict `P1.v1` in `W1`. The system creates:

```text
P1.v2:
  scope: W2
  applicability: narrower than P1.v1 or alternative transition rule
  evidence: E101 and later challenges
```

`P1.v1` remains active in `W1` if its evidence and conditions are still valid. The index must distinguish `W1` and `W2`; a broad symbol match is not enough to select a procedure.

### 80.4 Revalidation after drift

If a later event demonstrates that the transition rule changed globally, rather than only in `W2`, the system marks `P1.v1` as unstable for the affected time interval and routes new uses through a verification path. It preserves the earlier successful uses and records the reason for the version change.

The old procedure is not deleted. It remains available for historical queries and scopes where it was valid. New conclusions must not use it outside its supported region.

## 81. Memory stress variants

The same episode must be repeated with controlled failures.

### 81.1 Interference overload

Increase the number of distractor procedures until retrieval exceeds its budget. The correct result is `incomplete` or `insufficient`, not a confident statement that `P1.v1` does not exist.

Measure:

- number of index entries inspected;
- false absence rate;
- working-memory peak;
- retrieval latency;
- whether the system reports the missing path.

### 81.2 Stale index

Corrupt or remove the index entry while leaving the exact procedure and archive intact. The system should detect an index inconsistency, attempt a declared repair or slower fallback, and report the extra cost. It must not silently treat the procedure as forgotten.

### 81.3 Lossy consolidation

Replace the exact support records with a summary that omits the fact that `q2` negative cases were tested. A query requiring that detail must return conditional or insufficient status. The system must not claim that the omitted challenge was passed.

### 81.4 Authorized deletion

Delete the exact evidence `E1` through `E3` under an authorized policy while retaining a permitted tombstone and any explicitly allowed aggregate. The procedure should become unavailable for claims that require the deleted records, unless independent retained evidence supports them. The active index must not continue to expose deleted content.

### 81.5 Scope leak

Present a `W2` query using the same surface symbols as `W1`. The system fails if it applies `P1.v1` without checking scope and version.

### 81.6 Order variation

Interleave distractors before, between, and after the original episodes. Functional retention should remain stable if the relevant evidence and index are retained. A large order effect indicates hidden recency dependence or unstable consolidation.

## 82. Long-history acceptance criteria

The memory architecture passes this paper test only if it can:

1. reuse `P1.v1` in `W1` after interference;
2. retrieve exact supporting evidence from the archive on demand;
3. avoid scanning all history for ordinary reuse;
4. preserve the `W1`/`W2` distinction;
5. version or suspend procedures after a genuine drift event;
6. report incomplete retrieval when budgets are exhausted;
7. expose information loss after lossy consolidation;
8. remove authorized content from active use after deletion;
9. preserve old evidence history without treating it as current truth;
10. provide measured storage, retrieval, and revalidation costs.

It fails if it stores everything but cannot use it, uses a summary as if it were exact evidence, silently loses old procedures, globally invalidates unrelated knowledge, or claims infinite context because the archive exists.

## 83. Memory conclusion and next proof obligation

The long-history episode turns the memory idea into a measurable claim:

> Persistent, provenance-linked storage can support functional reuse through bounded retrieval without requiring the entire history in active context.

That claim is narrower and more testable than “no context limitation.” It still depends on index quality, scope discipline, archive availability, and retrieval budgets.

The next design question is how to make the structural index self-maintaining: how entries are created, invalidated, repaired, and checked without turning the index into an untrusted second memory or requiring a full scan after every update.

## 84. Structural-index model

An index is a routing structure over memory. It is not a second source of truth and must never be treated as evidence merely because it returns a result quickly.

An index entry contains:

```text
IndexEntry = {
  key-signature,
  target-references,
  source-references,
  scope,
  version,
  dependency-references,
  creation-reason,
  validation-status,
  last-validated,
  known-failures,
  cost-to-rebuild
}
```

`key-signature` describes the declared structural transformation used for routing. It cannot be an unexplained semantic similarity value. If a learned similarity operation is introduced later, the entry must point to that operation and its evidence.

An index entry may be:

- **validated:** its routing result was checked against current source references;
- **stale:** a dependency changed and validation is pending;
- **invalid:** a check demonstrated that the entry routes incorrectly;
- **partial:** only a bounded region was checked;
- **unavailable:** its source or archive cannot currently be reached;
- **deleted:** its targets were removed under policy.

The status is metadata about routing. It does not change the evidence status of the target records.

## 85. Index creation

Index entries may be created by four declared causes:

1. **Direct reference:** a procedure, event, or goal explicitly points to a record.
2. **Structural extraction:** a validated transformation yields a reusable signature.
3. **Dependency registration:** a derived result records which inputs it depends on.
4. **Archive placement:** moving data creates a location route for exact retrieval.

Every creation event records the input records, operation or procedure that produced the entry, scope, version, and work spent. The index is not allowed to infer that two records are equivalent merely because their surface keys match.

Creation may be synchronous for safety-critical references and asynchronous for optional optimization. Until an asynchronous entry is validated, retrieval must treat it as a candidate route and retain a fallback or report the missing validation.

## 86. Incremental index maintenance

New evidence should update indexes through dependency links rather than a global scan.

When event `Enew` arrives:

1. append `Enew` to the evidence ledger;
2. identify direct keys and declared scope;
3. follow inverse dependency links to affected entries;
4. mark those entries stale or invalid according to the evidence;
5. enqueue local validation or repair work;
6. publish only entries whose status permits the requested use;
7. preserve an unresolved status if the budget ends.

An entry that has no dependency link cannot be assumed unaffected. The system must record the blind spot and either run a bounded fallback scan or report that it cannot guarantee freshness.

The update path must be idempotent: processing the same evidence event twice should not duplicate dependencies, erase a prior invalidation, or create different index identities.

## 87. Invalidation and revalidation

An index entry is marked stale when a dependency may have changed. It is marked invalid only after a check demonstrates that its route or target no longer satisfies its declared conditions.

Revalidation proceeds from narrow to broad:

1. verify source references still exist and are within scope;
2. verify versions and deletion status;
3. recheck the key transformation on the affected records;
4. recheck dependent procedure applicability;
5. expand to neighboring entries only if a dependency boundary is uncertain.

A stale entry may support low-risk candidate retrieval if its uncertainty is exposed, but it must not silently support a committed answer or unsafe action. Safety-critical routes require validated status.

If a changed fact invalidates a procedure but not the underlying observations, the procedure is versioned or suspended while the evidence remains available. If an archive block is unavailable, the route is unavailable; the index cannot manufacture the missing data.

## 88. Index repair and fallback

Repair is itself an explicit procedure:

```text
repair(index-entry, source-references, budget)
  → validated-entry | replacement-entry | stale-entry | unavailable
```

Repair first uses recorded source references and local dependencies. If those are insufficient, it may inspect a bounded neighboring region. A full scan is a last-resort diagnostic, not the normal update path.

When an index is corrupt or missing, the system may:

- rebuild from direct references;
- use a slower alternative index;
- search a declared bounded region;
- request external assistance;
- report incomplete retrieval.

It must not report that no memory exists merely because the preferred index failed. Conversely, a fallback scan must not be hidden from the cost report.

## 89. Index trust and evidence separation

Index correctness has two independent dimensions:

- **routing correctness:** the index points to the records it claims to locate;
- **evidence correctness:** those records actually support the conclusion being considered.

A perfect index can route to false or contradictory evidence. Reliable evidence can become inaccessible through a corrupt index. The system must test and report these dimensions separately.

An index may cache a derived structural signature, but the signature must retain a link to the operation and source records that produced it. Rebuilding a signature after source deletion must not recreate deleted content from a hidden cache.

## 90. Structural-index stress tests

The index should be tested independently from the reasoning procedures it serves:

- **Incremental update:** add one event and measure the affected-entry set without scanning unrelated history.
- **Dependency fan-out:** change a highly referenced event and measure local invalidation, revalidation, and uncertainty propagation.
- **Missing dependency:** remove a link and verify that the system reports a freshness blind spot rather than claiming safety.
- **Corruption:** alter or delete an index entry while preserving source records; test fallback repair and cost reporting.
- **Archive movement:** move targets between active and cold storage; test route updates and exact retrieval.
- **Scope collision:** create equal surface signatures in different scopes; test that routes remain separated.
- **Procedure revision:** create a new procedure version; test that old and new targets are distinguishable.
- **Deletion:** delete a target and verify that no active route returns usable deleted content.
- **Duplicate event:** replay the same event; test idempotence and stable index identity.
- **Scale:** increase entry count, dependency fan-out, update rate, and archive depth while measuring latency and memory.

Required measurements include:

- entries inspected per update;
- stale and invalid entry counts;
- false-route rate;
- false-absence rate;
- revalidation latency;
- fallback frequency;
- archive access cost;
- dependency fan-out;
- peak index memory;
- full-scan frequency;
- unresolved freshness warnings.

## 91. Index falsification criteria

The index architecture requires revision if:

- an update requires a full history scan in ordinary cases;
- stale entries are used as current facts without an exposed warning;
- a missing dependency causes silent false confidence;
- a corrupt index is reported as missing knowledge rather than routing failure;
- a procedure revision leaves old and new routes indistinguishable;
- deleted records remain reachable through derived signatures or caches;
- duplicate events produce duplicate or inconsistent routes;
- local changes invalidate unrelated memory without an explainable dependency path;
- index routing is fast only because it hides large unmeasured preprocessing or repair work.

## 92. Current index conclusion

The structural index is now defined as a versioned, provenance-linked, fallible routing layer. It can reduce retrieval work by following explicit dependencies, but it cannot guarantee constant-time access or eliminate the cost of broad changes.

The correct claim is:

> A self-maintaining index may avoid repeated full-history scans when dependency coverage is good; when coverage is incomplete, the system must expose uncertainty, repair cost, or incomplete status.

The next proof obligation is a dependency-fan-out episode: one old event should affect many procedures, while an unrelated event should affect none. The system must show which entries were invalidated, why, how much work was performed, and whether any stale result escaped detection.

## 93. Dependency-fan-out test model

This episode tests whether index maintenance propagates change through declared dependencies instead of either missing affected knowledge or invalidating everything.

### 93.1 Dependency graph

The evaluator constructs two scopes, `W1` and `W2`, and an unrelated branch `U`. The graph contains:

```text
E_shared(W1)
  ├──> P_transfer(W1)
  ├──> P_route(W1)
  ├──> G_delivery(W1)
  └──> S_summary(W1)

E_independent(U)
  └──> P_unrelated(U)

E_cross(W1)
  └──> P_cross(W2)     [explicit cross-scope dependency]
```

Where:

- `E_shared` is an observation used by several procedures and a goal;
- `E_independent` belongs only to an unrelated branch;
- `E_cross` is scoped to `W1` but is explicitly declared as an input to a procedure in `W2`;
- `S_summary` is a derived summary depending on the shared event and the procedures above it.

The graph is deliberately small enough to inspect, but the episode should later scale the fan-out and depth.

### 93.2 Entry records

Each dependency edge has a source reference, target reference, scope relation, creation reason, and version. For example:

```text
Dependency D1:
  source: E_shared.v1
  target: P_transfer.v1
  relation: procedure-input
  scope: W1 → W1
  reason: execution trace T_transfer_01

Dependency D2:
  source: E_cross.v1
  target: P_cross.v1
  relation: explicit cross-scope input
  scope: W1 → W2
  reason: declared world-link L1
```

An undeclared relation must not be inferred from matching symbols during the normal update path. If later evidence suggests that an undeclared dependency exists, that is a dependency-discovery event and its uncertainty must be recorded.

## 94. Shared-event change replay

The evaluator appends a correction or new observation that changes the interpretation of `E_shared`:

```text
E_shared.v2:
  status: observed correction
  scope: W1
  effect: the source condition used by the original procedures is no longer supported
```

The maintenance transition is:

1. Append `E_shared.v2`; preserve `E_shared.v1` and its history.
2. Follow inverse dependency links from `E_shared.v1`.
3. Mark `P_transfer`, `P_route`, `G_delivery`, and `S_summary` stale.
4. Do not mark `P_unrelated` stale because no dependency path reaches it.
5. Follow `E_cross` only if the changed event is on its declared path; otherwise preserve `P_cross`.
6. Queue narrow revalidation in dependency order: direct procedures, then goal, then summary.
7. Prevent stale entries from supporting committed output or unsafe action.

The event correction does not automatically invalidate every procedure in `W1`. Scope and dependency edges determine the affected set.

### 94.1 Revalidation outcomes

The evaluator supplies three outcomes:

- `P_transfer` fails its applicability check and receives `P_transfer.v2` or suspended status;
- `P_route` remains valid after checking that it did not rely on the changed source condition;
- `S_summary` becomes stale until it is rebuilt from the current procedure versions.

The goal `G_delivery` cannot be declared satisfied using the old summary while revalidation is pending. It becomes conditional, stale, or incomplete according to the available evidence and budget.

The system must preserve a causal explanation:

```text
E_shared.v2
  → D1, D2, D3, D4
  → stale P_transfer, P_route, G_delivery, S_summary
  → P_transfer suspended
  → P_route revalidated
  → S_summary rebuilt or marked incomplete
```

## 95. Unrelated-event replay

The evaluator then appends:

```text
E_independent.v2:
  scope: U
  effect: correction within unrelated branch
```

The maintenance path follows only dependencies from `E_independent`. `P_unrelated` becomes stale or invalid according to its own checks. The `W1` branch must remain unchanged.

If any `W1` entry is marked stale, the system must provide an explicit dependency path. If it cannot, the result is an invalidation-precision failure or an unreported dependency leak.

## 96. Missing-link replay

For the third variant, the evaluator removes the dependency edge from `E_shared` to `P_route` while leaving the procedure's behavior dependent on that event. The next correction to `E_shared` creates an incomplete dependency graph.

The system must not claim that `P_route` is unaffected. It must either:

- detect a freshness blind spot from an audit or failed check;
- run a declared bounded fallback search;
- quarantine the uncertain branch;
- or report that it cannot guarantee freshness.

If `P_route` produces a stale answer before the missing link is detected, the result is a stale-escape failure. The failure must include the missing edge and the first output that used it.

## 97. Fan-out metrics

Let:

- `A` be the evaluator-defined affected set;
- `M` be the system-marked stale or invalid set;
- `U` be the set of entries unnecessarily marked stale;
- `E` be the set of stale outputs that escaped before detection;
- `W` be the update and revalidation work;
- `F` be the work of a full scan over all entries.

The episode records:

```text
invalidation recall    = |A ∩ M| / |A|
invalidation precision = |A ∩ M| / |M|
unnecessary invalidation = |U|
stale escape count      = |E|
locality ratio          = W / F
```

These measurements require the evaluator's hidden graph to be available for scoring, but not to the learner during execution. If `A` is ambiguous because the dependency graph is incomplete by design, the evaluator must score the known-dependency set separately from the hidden true-dependency set.

The system must report the absolute counts along with ratios. A high precision on a tiny graph can be misleading, and a low work ratio with poor recall is unsafe.

## 98. Fan-out acceptance criteria

The index-maintenance design passes the controlled episode only if it:

1. marks all known affected entries after `E_shared.v2`;
2. does not invalidate unrelated entries without an explainable path;
3. follows explicit cross-scope dependencies when present;
4. prevents stale goals and summaries from being treated as current;
5. preserves old versions and evidence history;
6. detects or reports the missing dependency variant;
7. records update and revalidation work;
8. identifies any stale escape and its first cause;
9. avoids a full scan in the covered-dependency case;
10. reports broader quarantine or fallback cost when dependency coverage is incomplete.

It fails if it updates only the directly changed procedure while leaving dependent goals stale, invalidates the entire memory graph without a path, silently ignores cross-scope edges, or claims freshness after a missing-link failure.

## 99. Index conclusion

The dependency-fan-out episode sharpens the meaning of “local revision.” Local does not mean “change only the directly connected record.” It means:

> Propagate change through every known dependent path, preserve uncertainty where coverage is incomplete, and avoid unrelated work when the dependency graph proves independence.

This is a measurable systems property, not an automatic consequence of using a graph. The next unresolved issue is dependency discovery itself: how the system finds a missing edge without scanning all history or assuming that matching surface structure implies a causal dependency.

## 100. Dependency-discovery model

An explicit dependency edge means that changing or withdrawing one record can change the validity, output, scope, or interpretation of another record. The system may know such an edge because it was recorded during execution, or it may suspect one after a later mismatch. These are different statuses.

Dependency candidates have the form:

```text
DependencyCandidate = {
  source-reference,
  target-reference,
  relation-kind,
  proposal-source,
  supporting-traces,
  contradicting-traces,
  scope-relation,
  validation-status,
  audit-cost
}
```

The relation kind should be explicit:

- **input-use:** a procedure read the source as an input;
- **derived-value:** the target was computed from the source;
- **scope-condition:** the target's applicability depends on the source scope;
- **authorization:** the source permitted or restricted the target action;
- **summary-support:** a summary includes the source's information;
- **cross-scope-link:** the source is intentionally used in another scope;
- **suspected-influence:** evidence suggests a dependency but has not established one.

The last relation is deliberately weaker. A suspected influence may trigger an audit or quarantine, but it cannot be used as if it were a validated causal edge.

## 101. Sources of dependency evidence

Dependency candidates can be proposed from several sources, ordered from strongest to weakest:

1. **Execution trace:** an operation explicitly read or transformed the source reference while producing the target.
2. **Declared interface:** a procedure's input pattern, goal constraint, or scope declaration names the source.
3. **Version relation:** a target version was created because a source version changed.
4. **Counterfactual replay:** removing or substituting the source changes the target result while other inputs remain fixed.
5. **Intervention:** an external action changes the source and produces a predictable target difference.
6. **Mismatch correlation:** a target fails when a source changes, but the causal path is not yet isolated.
7. **Structural similarity:** source and target share a pattern without a checked effect.

The first three can create validated edges when their records are complete. Sources four and five can support validation through controlled tests. Source six creates a suspected-influence edge. Source seven creates at most a retrieval candidate and must never create a dependency by itself.

This ordering prevents matching symbols, shared names, or frequent co-occurrence from being mistaken for causality.

## 102. Dependency recording at execution time

Every operation execution should record the references it actually inspected, transformed, or used in a guard. The trace distinguishes:

- read and used;
- read and rejected;
- available but not read;
- inferred through another procedure;
- unavailable because of budget or retrieval failure.

This distinction matters. A procedure that happened to have access to `E_shared` but did not inspect it is not automatically dependent on `E_shared`. A procedure that used a summary is dependent on the summary and, transitively, on the summary's declared support records.

Execution-time capture is the primary defense against missing direct edges. It does not catch dependencies hidden inside an incomplete operation, an undeclared external condition, or a procedure that was not executed during the relevant case.

## 103. Bounded missing-edge discovery

The system cannot prove that no missing dependency exists without examining all relevant alternatives. Instead, it uses bounded discovery modes.

### 103.1 Shadow replay

When a target is important or a result is surprising, the system replays the target procedure with one candidate source removed, replaced, or version-shifted. It compares the resulting trace and output with the original.

If the result changes, the source becomes a dependency candidate. If the result does not change under the tested variation, the system records only a limited negative result; it does not prove independence under all future states.

### 103.2 Dependency sampling

For low-risk entries, the system samples target procedures and candidate sources according to a declared schedule. Samples must cover:

- high fan-out sources;
- recently changed sources;
- cross-scope relationships;
- summaries and promoted procedures;
- entries with incomplete provenance;
- entries whose outputs have recently failed.

Sampling reduces routine cost but creates coverage uncertainty. The sample schedule and untested region must be reported.

### 103.3 Canary revalidation

Before using a high-impact procedure after a relevant change, the system runs a small canary check against current evidence or a safe hypothetical branch. A canary does not prove the procedure globally valid; it is an early warning against stale use.

### 103.4 Dependency quarantine

If a procedure has a plausible but unvalidated dependency on changed evidence, the system may quarantine it. Quarantine prevents committed or unsafe use while allowing low-risk analysis to continue with an explicit uncertain status.

Quarantine is preferable to silently treating the procedure as independent, but over-quarantine harms availability. Its rate and causes must be measured.

### 103.5 External discriminating evidence

If replay cannot distinguish independence from dependence, the system may request an observation or intervention that makes the target predictions diverge. The request must describe the competing hypotheses and the expected outcomes. “Collect more data” is not a dependency test.

## 104. Dependency-discovery transition

The transition for a suspected missing edge is:

```text
suspect(source, target, reason)
  → candidate edge
  → shadow replay or safe audit
  → validated edge | bounded independence result | unresolved
```

The resulting status has limits:

- **validated edge:** use for ordinary invalidation and revalidation;
- **bounded independence result:** no dependency observed under named variations;
- **unresolved:** do not claim freshness for high-impact use;
- **rejected candidate:** preserve the audit evidence and the tested scope.

An independence result is always scoped to the source versions, target versions, action path, and variations tested. It is not a universal proof of non-dependence.

## 105. Dependency-discovery example

Return to the fan-out graph. Suppose the edge `E_shared → P_route` was missing. A later correction to `E_shared` causes `P_route` to produce an unexpected result.

The system records:

```text
Mismatch M1:
  source candidate: E_shared.v2
  target: P_route.v1
  observed effect: predicted result differs after E_shared correction
  status: suspected-influence
```

It performs a shadow replay:

1. execute `P_route.v1` with `E_shared.v1`;
2. execute the same trace with `E_shared.v2`;
3. hold all other retrieved references and procedure versions fixed;
4. compare emitted events, checks, and output;
5. record whether the changed source is the only changed input.

If the output changes only under `E_shared.v2`, the system validates the missing edge, marks `P_route.v1` stale, and revalidates its dependents. If the output does not change, the edge remains unresolved if another hidden external condition could explain the original mismatch.

The system must not rewrite the original trace to make the dependency appear as if it had always been recorded. The new edge has its own creation reason, version, and audit cost.

## 106. Dependency metrics

Let:

- `T` be the evaluator-defined true dependencies in the tested scope;
- `V` be validated edges recorded by the system;
- `S` be suspected but unresolved edges;
- `Q` be quarantined targets;
- `X` be stale outputs that escaped before a dependency was detected;
- `C` be dependency-discovery work;
- `F` be full-scan work over candidate source-target pairs.

The evaluation records:

```text
validated-edge precision = |T ∩ V| / |V|
validated-edge recall    = |T ∩ V| / |T|
unresolved suspicion rate = |S| / candidate-suspicions
quarantine rate           = |Q| / high-impact targets
stale escape count        = |X|
discovery locality ratio  = C / F
```

The evaluator must separate known dependencies from hidden true dependencies. A system should not be penalized for failing to record an edge that the test intentionally makes unobservable, but it must not claim that the edge does not exist.

Absolute counts, audit samples, tested variations, and untested regions must accompany every ratio.

## 107. Dependency-discovery falsification tests

The discovery mechanism requires revision if:

- it treats shared symbols or frequent co-occurrence as validated causality;
- it cannot recover a direct dependency from an execution trace;
- it marks a procedure fresh after a tested source substitution changes its output;
- it silently ignores a suspected dependency after a mismatch;
- it claims independence from one unchanged replay without stating the tested scope;
- it performs unbounded shadow replay or sampling;
- it quarantines nearly everything instead of improving dependency coverage;
- it fails to distinguish cross-scope links from accidental surface matches;
- it rewrites historical traces to hide that a dependency was discovered late;
- its reported locality comes from excluding failed audits or repair work.

## 108. Dependency conclusion

The architecture cannot guarantee perfect dependency knowledge cheaply. It can define a disciplined hierarchy:

```text
direct trace use
  → declared relation
  → controlled replay/intervention
  → suspected influence
  → structural similarity only
```

Only the stronger levels support ordinary invalidation. Weaker levels trigger audit, quarantine, or uncertainty. This avoids both dangerous optimism and the claim that every possible causal relationship can be known without cost.

The next proof obligation is to test dependency discovery under hidden external conditions: a procedure should appear independent under ordinary replay but fail after an environmental change. The system must detect the limitation, avoid overclaiming independence, and preserve an honest uncertainty boundary.

## 109. Hidden-condition dependency test

This test separates two questions:

1. **Internal dependency:** which stored records and procedure inputs did the machine use?
2. **Environmental dependency:** which unobserved external conditions affect whether the procedure remains valid?

The execution trace can answer the first question only for the information it actually receives. It cannot prove that the outside world has no hidden variable. A failed prediction after environmental change is evidence of a limitation or drift, not immediate identification of the hidden cause.

### 109.1 Environment

The evaluator defines a hidden mode `M` with two values, `M0` and `M1`. The learner does not receive the mode directly.

The observable interface is:

```text
observation: [s8, q1, s8]
action: a9
expected under M0: [s8, q2, s8]
expected under M1: [s8, q1, s8]
```

The learner initially operates only in `M0` and admits `P1.v1` from earlier evidence. Ordinary shadow replay substitutes stored observations and procedure inputs while the environment remains in `M0`; the replay shows no difference attributable to the hidden mode because the mode has not changed.

At a declared but unobserved point, the evaluator changes the environment to `M1`. The surface observation and action symbols remain identical. The next execution returns `[s8, q1, s8]` instead of the predicted `[s8, q2, s8]`.

The learner can observe a residual:

```text
predicted: [s8, q2, s8]
observed:  [s8, q1, s8]
```

It cannot honestly identify `M1` unless it has an observation, intervention, or prior procedure that makes the hidden mode distinguishable.

## 110. Hidden-condition replay

The learner processes the residual through the following steps:

1. preserve the failed output and all prior evidence;
2. verify that the input stream, action, scope, and procedure version match `P1.v1`;
3. check for known source changes or missing dependencies;
4. attempt bounded shadow replay using available stored inputs;
5. compare the old and current outcomes;
6. create a drift or hidden-condition hypothesis;
7. quarantine `P1.v1` for the affected environment scope;
8. seek a safe discriminating observation or intervention;
9. version the procedure only when the new evidence supports a specific change;
10. otherwise report an unresolved environmental dependency.

The initial status must be:

```text
P1.v1 applicability: unresolved after environmental residual
possible causes: hidden condition, world drift, corrupted observation,
                 unrecorded dependency, or procedure failure
confidence: not assigned as a universal probability
```

The system must not invent a hidden variable merely because the evaluator knows one exists. The evaluator's latent explanation is for scoring, not learner evidence.

## 111. Safe discrimination of the hidden condition

Suppose the evaluator provides a safe diagnostic action `a_diag` whose outcomes differ by mode but does not change protected state:

```text
under M0: [d0]
under M1: [d1]
```

The learner may propose an information goal:

```text
Goal G_diag:
  distinguish current environmental condition H0 from H1
  action: a_diag
  prohibit: protected-state change
  evidence: returned diagnostic observation
```

If `a_diag` returns `d1`, the learner has evidence that the current condition differs from the training condition. It may create a scoped version:

```text
P1.v2:
  applies only when diagnostic condition = d0
P1.v3:
  alternate transition under diagnostic condition = d1
```

If no diagnostic action exists, or if all available actions have ambiguous outcomes, the system remains uncertain. It may refuse the unsafe action, request external confirmation, or operate only in a low-impact hypothetical branch.

## 112. Hidden-condition variants

The evaluator repeats the test with controlled variants:

### 112.1 Observed mode change

The environment emits an explicit mode-change event. A correct system uses that event as a dependency and scopes the revision. This tests whether it records direct environmental signals.

### 112.2 Unobserved mode change

The mode changes without an observable indicator. The system can detect the residual but should not claim to know its cause. This tests uncertainty honesty.

### 112.3 Confounded change

The mode remains fixed, but the observation adapter introduces a corruption. The system must preserve both hidden-condition and measurement-corruption hypotheses until evidence separates them.

### 112.4 Reversible intervention

An action changes the hidden mode and a safe diagnostic reveals the result. The system must record the intervention and avoid generalizing the result beyond the tested mode and scope.

### 112.5 Unrecoverable ambiguity

Two hidden modes generate identical observations and consequences for every action within budget. The correct status is unresolved equivalence, not an invented distinction.

## 113. Hidden-condition acceptance criteria

The system passes the conceptual test only if it:

1. detects the prediction residual;
2. preserves the old procedure and successful history;
3. does not claim that ordinary replay proved universal independence;
4. distinguishes internal recorded dependencies from unknown environmental causes;
5. quarantines or limits unsafe reuse after unexplained drift;
6. proposes a discriminating observation when one exists;
7. creates a scoped revision only after relevant evidence arrives;
8. preserves multiple explanations when measurement corruption and world drift are confounded;
9. reports unresolved equivalence when no affordable action can distinguish causes;
10. includes the failed audit and quarantine work in resource accounting.

It fails if it silently continues using `P1.v1`, labels the hidden mode without evidence, treats one successful replay as proof of independence, globally rewrites old history, or reports environmental certainty from evaluator-only metadata.

## 114. What this test establishes

The hidden-condition test establishes a boundary of the architecture:

> A provenance-aware structural system can detect some unmodeled change through prediction residuals, but it cannot identify an unobserved environmental cause without discriminating evidence.

This is not a defect unique to the proposed system. It is a basic limitation of inference from incomplete observations. The architectural requirement is therefore not omniscience; it is correct uncertainty handling, safe containment, and targeted evidence seeking.

The test also clarifies that dependency discovery has two layers:

- **recorded dependency discovery:** recoverable through traces, declarations, and replay;
- **world dependency discovery:** requires observation or intervention that exposes the external condition.

The next unresolved issue is the bridge from controlled opaque streams to language: how textual sequences become grounded observations without importing a hidden semantic parser or treating next-symbol prediction as understanding.

## 115. Text as an opaque input channel

The first text interface treats text as an ordered stream of externally observable atoms. For a controlled experiment, an atom may be a byte or code point. The choice must be declared because it affects segmentation cost and information loss.

```text
TextStream = [atom_0, atom_1, atom_2, ...]
```

The text adapter may preserve:

- exact atom identity;
- order;
- source document and location;
- explicit transmission boundaries;
- encoding errors or missing data;
- physical layout metadata if the source exposes it.

It may not silently provide:

- words;
- sentences;
- parts of speech;
- named entities;
- coreference links;
- semantic vectors;
- sentiment, intent, or topic labels;
- factual truth labels;
- a pretrained language representation with undisclosed training history.

The structural core may discover repeated spans and transformations over the stream. A repeated span is not automatically a word, and a stable sequence is not automatically a fact.

## 116. Text-structure hypothesis ladder

Textual interpretation should develop through separate hypotheses rather than one hidden “understanding” operation.

### 116.1 Span hypotheses

The system proposes a span when a contiguous atom sequence recurs, participates in stable transformations, or improves a checked task. Each span records its boundaries, source occurrences, alternatives, and cost.

The same atoms may have multiple segmentations. The system retains alternatives when their evidence is tied or when they predict different downstream outcomes.

### 116.2 Sequence and boundary hypotheses

The system proposes ordering and boundary relations from recurrence, interruption, action-linked change, document structure, or explicit external effects. It must distinguish physical line breaks and punctuation from semantic sentence boundaries; either may be useful, neither is automatically meaningful.

### 116.3 Role hypotheses

When a span occupies a stable structural role across varied contexts, the system may propose a formal role. A role is initially unnamed:

```text
role R1 = span that is repeatedly supplied to transformation T2
```

It becomes a candidate entity, action, property, or relation only when grounded consequences support that interpretation. Human naming is an interface operation and must not be confused with role discovery.

### 116.4 Reference hypotheses

The system may propose that two spans refer to the same underlying item when substituting one for the other preserves checked consequences across contexts. It must retain alternatives for ambiguous references such as repeated names, pronouns, descriptions, or ellipsis.

Reference is not identity. Two texts may refer to one entity, or one text may use the same form for different entities. The evidence ledger must preserve the source and scope of every proposed link.

### 116.5 Relation and order hypotheses

The system may propose relations such as:

- before or after;
- same or different;
- causes or enables;
- owns or contains;
- asks or asserts;
- prohibits or permits.

These names are evaluator or interface labels. Internally, the learner first has structural relation candidates. A causal relation requires intervention or independently checked consequences; textual co-occurrence alone is insufficient.

### 116.6 Negation and modality hypotheses

The system must distinguish the structure of a claim from its status. A text span may report:

- an observed event;
- a denial;
- a question;
- a possibility;
- a plan;
- a hypothetical;
- a quoted claim;
- an instruction.

The learner may propose these distinctions from contrastive use and downstream outcomes, but it must not treat every sentence as an assertion about the world. Quoted, hypothetical, and negated content require separate scopes.

## 117. Language grounding path

Text becomes grounded only when a textual hypothesis connects to something independently checkable. Possible grounding paths are:

1. **Co-observation:** text occurs alongside a sensor or environment event.
2. **Action consequence:** interpreting a text span predicts the result of an action.
3. **Task constraint:** a text instruction can be executed and checked against an external state.
4. **Cross-source agreement:** independent sources support a relation, with source independence recorded.
5. **Correction:** a user or environment provides a discriminating correction.
6. **Self-consistency:** a textual transformation preserves checked structural relations, which supports form but not external truth.

The last path can support grammar-like structure but cannot establish that the text is true. A document may be internally consistent and factually wrong.

Every grounded textual interpretation should preserve:

- the original text span;
- the proposed structural role;
- the external evidence or consequence;
- the scope and time;
- competing interpretations;
- the cost and action history that produced the grounding;
- what remains untested.

## 118. Text is not a next-symbol objective

The system may observe that one atom often follows another, but next-symbol frequency is only one structural statistic. It does not supply a sufficient objective for:

- entity identity;
- reference across long spans;
- negation;
- truth;
- causality;
- goals;
- action consequences;
- contradiction handling.

The proposed text learner instead asks whether a candidate segmentation, relation, or transformation supports checked reuse beyond its source examples. It may use compression or prediction as diagnostics, but a lower description length or lower prediction error is not by itself a semantic admission criterion.

This distinction does not claim that sequence prediction is useless. It says that sequence prediction must not be allowed to stand in for the full learning problem.

## 119. Paper-scale textual episode

The evaluator provides three text streams as raw atoms, together with an external world in which the described state can be checked. The learner receives no word boundaries or semantic labels.

The streams are conceptually written here with spaces for readability only; the learner receives the raw atom sequences:

```text
T1: mira places tokenA on shelfB
T2: mira removes tokenA from shelfB
T3: tokenA is on shelfB
```

The evaluator privately knows that `T1` should produce a state change, `T2` should reverse it, and `T3` describes a state. The learner does not receive those meanings.

The episode proceeds:

1. discover candidate spans and boundaries from raw atoms;
2. compare the streams and preserve alternative alignments;
3. associate recurring spans with structural roles;
4. use a controlled environment or simulator to test an interpretation;
5. emit an action or prediction only within an explicit goal scope;
6. compare the outcome against an independent observation;
7. revise the interpretation if the consequence fails;
8. test the learned transformation with renamed participants and reversed order;
9. retain unresolved readings when the text alone cannot distinguish them.

A candidate structural procedure might be:

```text
form A: [role_person, role_action, role_item, role_relation, role_location]
effect hypothesis: relation(role_item, role_location) becomes active
```

This is not admitted because a human can read the sentence. It is admitted only if the candidate-generation grammar produces it and its predicted consequence survives independent tests.

## 120. Text grounding variants

The episode should be repeated with controlled variants:

- **Renaming:** replace all surface names while preserving roles and consequences.
- **Word-boundary removal:** remove spaces and punctuation from the input stream.
- **Negation:** add a structurally similar denial and verify that no state change is inferred.
- **Modality:** describe a plan or possibility and verify that it is not treated as completed action.
- **Quotation:** place a claim inside quoted text and verify that the claim is scoped to the speaker or document.
- **Reference ambiguity:** use a repeated name or pronoun with two possible antecedents.
- **Temporal reversal:** present events in a different order and test state reconstruction.
- **Contradictory sources:** provide conflicting descriptions with different provenance.
- **Grounding gap:** provide text with no external consequence and require an ungrounded status.
- **Long-distance reference:** place the decisive antecedent far outside the active workspace and test staged retrieval.

## 121. Text evaluation criteria

The text subsystem may claim:

- **span discovery** when it finds reusable boundaries without supplied tokenization;
- **structural role discovery** when roles transfer across varied surface forms;
- **grounded interpretation** when a role hypothesis improves held-out prediction or action success against an external check;
- **reference resolution** when alternative links are selected correctly on held-out consequences;
- **truth-sensitive reporting** only when source, evidence, scope, and contradiction status are preserved.

It may not claim language understanding from:

- memorizing frequent strings;
- reproducing a corpus;
- predicting the next atom;
- producing fluent text through an external generator;
- matching human labels without testing consequences;
- selecting one ambiguous interpretation without disclosing alternatives.

Required measurements include:

- boundary precision and recall against evaluator annotations, reported separately from learner input;
- held-out role transfer;
- grounded task success;
- negation and modality error rates;
- reference ambiguity handling;
- contradiction and provenance accuracy;
- long-distance retrieval work;
- abstention quality;
- candidate count, alignment work, and memory cost.

## 122. Text falsification criteria

The language bridge requires revision if:

- it depends on hidden tokenization or a pretrained semantic representation;
- recurring strings are treated as meanings without grounded consequences;
- a described, planned, quoted, or hypothetical event is treated as observed fact;
- negation is lost during structural compression;
- the system collapses source disagreement without a declared rule;
- it cannot preserve ambiguity when text alone is insufficient;
- next-symbol accuracy improves while grounded task performance does not;
- long-distance references require the whole document in active workspace;
- fluent output hides missing evidence or unsupported claims;
- text grounding fails when surface names and order are varied.

## 123. Current language conclusion

The proposed bridge from text to the structural brain is:

```text
raw text atoms
  → candidate spans and alignments
  → unnamed structural roles
  → reference and relation hypotheses
  → grounded consequence tests
  → versioned procedures and world-linked interpretations
```

This keeps text separate from truth and language form separate from grounded meaning. It does not promise that raw text alone can produce a complete world model. Text can report, describe, deny, imagine, or mislead; grounding requires evidence outside repetition.

The next proof obligation is a paper-scale text episode with negation, quotation, modality, ambiguity, and an external action check. The learner must show which structure it discovered, which meaning was grounded, and which interpretations remain unresolved.

## 124. Prior toy-episode consistency corrections

Before promoting any earlier example into an executable specification, the following corrections apply:

1. A candidate procedure is a **partial predictor** unless its applicability conditions explicitly cover a case. If `P1` applies only to `[X, q1, X]`, failure to apply it to `[X, q2, X]` is an abstention, not a prediction that the state will remain unchanged.
2. A negative transition may reject a candidate only when the environment supplies an observed output for that input and action. The learner must compare the candidate's predicted output, or its declared abstention, with that observation.
3. The earlier action-selection claim that `a9` separates hypotheses was under-specified. In the first experiment, action selection must use a declared deterministic exploration schedule or enumerate all available actions within budget. It cannot use a hidden “nearest alternative” oracle.
4. Terminal success is an evaluation signal, not a semantic explanation. The learner may use it to evaluate the task goal, but it must not infer the evaluator's hidden state names from it.
5. A hand-written evaluator can generate hidden outcomes, but the experiment must disclose which observations and action results are exposed to the learner. Hidden evaluator metadata is scoring information, not learner input.

The standalone Testable Design v1 below adopts these corrections and intentionally narrows the first experiment to structural rule discovery, held-out generalization, and continual retention. It does not claim to test language, multimodal perception, planning, or superintelligence.
