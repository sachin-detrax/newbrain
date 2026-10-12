# Testable Design v1: Sparse Structural Rule Learner

Status: design specification only. Not implemented, not benchmarked, and not approved for implementation yet.
Date: 2026-10-10

## 1. Purpose

This specification defines the first narrow experiment for the proposed Brain Machine Model. It tests one claim:

> Can a learner discover and reuse a variable-based structural rule from sparse event transitions, transfer that rule to held-out symbols, and retain it after learning an unrelated rule—without backpropagation, gradient descent, dense tensor layers, or next-token training?

This is not a test of language, vision, general planning, world-scale memory, consciousness, or superintelligence. A success would establish only a small structural-learning result. A failure would falsify or require revision of the current candidate-generation mechanism, not prove that every from-scratch architecture is impossible.

## 2. Scope decisions

### Included

- Fixed-length opaque sequences of exactly three atoms.
- Two opaque action symbols.
- Sparse symbolic event records.
- A finite candidate grammar.
- Variable discovery through repeated-position anti-unification.
- Explicit candidate evidence and challenge history.
- Held-out symbol generalization.
- Retention after an unrelated rule is learned.
- Exact operation, candidate, comparison, and workspace accounting.

### Excluded

- Raw language and text semantics.
- Images, audio, continuous sensors, and sensory adapters.
- Human-readable concept names.
- Causal claims about the external world.
- Arbitrary-length sequences.
- Neural weights, embeddings, gradient updates, or dense matrix computation.
- A learned reward function.
- Natural-language generation.
- A persistent archive or distributed runtime.

The excluded capabilities remain later research targets. They must not be smuggled into this experiment through the input generator, evaluator, or candidate generator.

## 3. Formal task interface

The learner receives a stream of transition records. Each record has exactly:

```text
Transition = {
  input:  [symbol_0, symbol_1, symbol_2],
  action: action_symbol,
  output: [symbol_0, symbol_1, symbol_2],   # training/development only
  scope:  scope_symbol,
  record_id
}
```

On a test query, the learner receives only:

```text
Query = {
  input:  [symbol_0, symbol_1, symbol_2],
  action: action_symbol,
  scope:  scope_symbol,
  query_id
}
```

It must return exactly one of:

```text
Prediction(output_sequence, procedure_version, evidence_refs)
Abstention(reason, procedure_refs, evidence_refs)
```

The expected output is hidden from the learner until the evaluator scores the response. The evaluator may reveal the output after the query for continual learning, but it must record whether the learner had access before prediction.

The learner may not receive semantic labels such as object, location, movement, state, action meaning, positive, negative, or success. Symbol identity and sequence position are available as exact structural properties.

## 4. Fixed alphabet and ordering

The experiment uses the following opaque symbols:

```text
Outer symbols:  x0, x1, x2, x3, x4, x5, x6, x7
Marker symbols: m0, m1, n0, n1
Actions:        a0, a1
Scopes:         W1, W2
```

The learner receives symbol identities, not their semantic categories. The category names “outer” and “marker” are evaluator-only terminology and must not appear in the learner input.

Canonical ordering for deterministic enumeration is:

```text
x0 < x1 < x2 < x3 < x4 < x5 < x6 < x7
m0 < m1 < n0 < n1
a0 < a1
W1 < W2
```

The ordering is an implementation-independent tie-break only. It is not a learned relation and cannot be used as evidence that symbols have meaning.

## 5. Hidden transition generator

The evaluator generates outputs using two hidden rules. The learner does not receive these rules.

### Rule R1: structural transformation

For any `X` in `{x0, x1, x2, x3, x4, x5, x6, x7}`:

```text
input  = [X, m0, X]
action = a0
output = [X, m1, X]
```

### Rule R2: independent structural transformation

For any `X` in `{x0, x1, x2, x3, x4, x5, x6, x7}`:

```text
input  = [X, n0, X]
action = a1
output = [X, n1, X]
```

### Identity cases

All queries not matching R1 or R2 produce identity output:

```text
output = input
```

Identity cases are evaluator outcomes. They are not automatically training targets for the learner's partial procedure: if no admitted procedure applies, the correct response is abstention rather than an unsupported claim that the input will remain unchanged.

## 6. Dataset partitions

The partitions are fixed before implementation and must not be changed after inspecting final results.

### 6.1 R1 training records

```text
T1: [x0, m0, x0] --a0--> [x0, m1, x0]
T2: [x1, m0, x1] --a0--> [x1, m1, x1]
```

### 6.2 R1 development records

Development records are a fixed admission gate and may break ties between candidates with identical training ranking. They are not used to invent new grammar operations.

```text
D1: [x2, m0, x2] --a0--> [x2, m1, x2]
D2: [x2, m0, x3] --a0--> [x2, m0, x3]
D3: [x2, m1, x2] --a0--> [x2, m1, x2]
D4: [x2, m0, x2] --a1--> [x2, m0, x2]
```

D1 is applicable-positive. D2, D3, and D4 are non-applicable cases.

### 6.3 R1 final records

The final evaluator uses these records, hidden until the learner is frozen:

```text
F1:  [x3, m0, x3] --a0--> [x3, m1, x3]
F2:  [x4, m0, x4] --a0--> [x4, m1, x4]
F3:  [x5, m0, x5] --a0--> [x5, m1, x5]
F4:  [x6, m0, x6] --a0--> [x6, m1, x6]
F5:  [x7, m0, x7] --a0--> [x7, m1, x7]
F6:  [x3, m0, x4] --a0--> [x3, m0, x4]
F7:  [x4, m1, x4] --a0--> [x4, m1, x4]
F8:  [x5, m0, x5] --a1--> [x5, m0, x5]
```

F1–F5 are applicable-positive. F6–F8 are non-applicable.

### 6.4 R2 training and retention records

After R1 admission, the learner receives:

```text
U1: [x0, n0, x0] --a1--> [x0, n1, x0]
U2: [x1, n0, x1] --a1--> [x1, n1, x1]
```

R2 is intentionally independent of R1 except for sharing the same structural form. The learner must retain R1 while learning R2.

### 6.5 R2 development records

The R2 development gate mirrors the R1 gate with the second action and marker family:

```text
D5: [x2, n0, x2] --a1--> [x2, n1, x2]
D6: [x2, n0, x3] --a1--> [x2, n0, x3]
D7: [x2, n1, x2] --a1--> [x2, n1, x2]
D8: [x2, n0, x2] --a0--> [x2, n0, x2]
```

D5 is applicable-positive. D6, D7, and D8 are non-applicable cases. These records are visible to the learner during Phase B admission and are not final evaluation cases.

### 6.6 R2 final records

```text
V1: [x2, n0, x2] --a1--> [x2, n1, x2]
V2: [x3, n0, x3] --a1--> [x3, n1, x3]
V3: [x4, n0, x5] --a1--> [x4, n0, x5]
V4: [x5, n1, x5] --a1--> [x5, n1, x5]
```

V1–V2 are applicable-positive. V3–V4 are non-applicable.

No final record is reused during candidate generation or configuration selection.

## 7. Learner primitive vocabulary

The v1 learner may use only these non-learned operations:

1. `Ingest(record)` — append a transition and provenance.
2. `Equal(a, b)` — exact symbol equality.
3. `Read(position)` — read one of the three fixed positions.
4. `Bind(variable, value)` — bind a variable inside one candidate scope.
5. `Emit(value)` — construct an output symbol already present in a support transition.
6. `Copy(position)` — copy an input position to the output.
7. `Compare(sequence_a, sequence_b)` — compare fixed positions.
8. `Enumerate()` — generate candidates in canonical order.
9. `Check(candidate, record)` — evaluate candidate applicability and prediction.
10. `Store(candidate)` — persist a candidate with evidence references.
11. `Retrieve(signature, scope)` — retrieve stored candidates by exact structural signature.
12. `Abstain(reason)` — produce an explicit non-prediction.

The primitive vocabulary cannot inspect hidden evaluator state, use a semantic dictionary, calculate a similarity score, or access a gradient. Any later addition requires a new experiment version.

## 8. Finite candidate grammar

The learner generates only the following candidate families.

### G0: exact transition

For every training record, generate one exact candidate:

```text
if input = observed_input and action = observed_action:
  predict observed_output
```

G0 candidates are memorization baselines stored for comparison. They do not generalize to unseen outer symbols.

### G1: two-record anti-unification

For every ordered pair of distinct positive training records with the same action and sequence length:

1. Compare each input position.
2. If a position has the same symbol in both records, retain that symbol as a constant.
3. If a position differs, introduce one variable for that position.
4. If two positions are equal within every support record, they must share one variable.
5. If two positions are not equal in any support record, they may not be forced to share a variable.
6. Compare each output position.
7. Represent an output position as `Copy(i)` if it equals input position `i` in every support record.
8. Represent it as `Emit(symbol)` if it has the same output symbol in every support record.
9. Reject the candidate if neither representation is stable across support records.

For T1 and T2, G1 produces exactly:

```text
C_R1:
  if input = [X, m0, X] and action = a0:
    predict [Copy(0), Emit(m1), Copy(2)]
```

For U1 and U2, it produces exactly:

```text
C_R2:
  if input = [X, n0, X] and action = a1:
    predict [Copy(0), Emit(n1), Copy(2)]
```

### No other grammar in v1

V1 does not permit:

- arbitrary predicates;
- unrestricted variable partitions;
- variable-length alignment;
- composition of more than one learned procedure;
- context relaxation beyond the equality pattern in G1;
- candidate generation from final records;
- semantic names;
- a manually inserted rule.

This makes v1 intentionally weak. If the target rule is not expressible by G0 or G1, the correct result is `unsupported-by-grammar`, not an improvised extension during the run.

## 9. Candidate admission

A candidate is admitted only after all gates pass.

### Gate A: support

The candidate must have at least two distinct support records with the same action and exact sequence length.

### Gate B: consistency

The candidate must produce the recorded output for every support record.

### Gate C: no training contradiction

The candidate must not predict an output for a training record whose input/action matches its applicability pattern but whose observed output differs.

### Gate D: structural variation

The two support records must differ in at least one input symbol that is represented by a variable. This prevents two identical examples from being called generalization.

### Gate E: development challenge

The candidate is checked against D1–D4. A matching candidate must predict D1 and must abstain on D2–D4. An abstention is correct only when the candidate's applicability pattern does not match; the learner may not claim the identity output without a matching procedure.

### Gate F: cost

The candidate must remain within the per-update candidate, comparison, and workspace budgets.

Admission stores the candidate, exact support references, all checks, all abstentions, the grammar version, and the candidate cost. Admission is provisional; final performance is evaluated only on F and V.

## 10. Exact execution semantics

Candidates are partial predictors.

For query `Q`:

1. Retrieve candidates whose scope, action, length, and structural signature may match.
2. Evaluate candidates in canonical order.
3. If exactly one admitted candidate applies, return its predicted output.
4. If multiple candidates apply and predict the same output, return that output with all procedure references.
5. If multiple candidates apply and predict different outputs, return `Abstention(conflict)`.
6. If no candidate applies, return `Abstention(no-applicable-procedure)`.

The learner is never required to predict identity for an unknown case. This prevents a default no-op from masquerading as a learned world model.

If the learner has an exact G0 candidate and a generalized G1 candidate that both apply, G1 is preferred because it has the lower description length under the fixed grammar. The preference is fixed before testing and is reported.

## 11. Training protocol

The training run has two phases.

### Phase A: learn R1

1. Ingest T1 and T2 in the declared order.
2. Generate all G0 candidates.
3. Generate all G1 candidates from ordered pairs.
4. Deduplicate candidates by canonical signature.
5. Apply Gates A–F using only T1, T2, and D1–D4.
6. Store admitted R1 candidate and rejected candidates with reasons.
7. Freeze the R1 learner state before exposing final records F1–F8.

### Phase B: continual update with R2

1. Ingest U1 and U2 after Phase A.
2. Generate G0 and G1 candidates from U1 and U2.
3. Preserve the R1 candidate and its evidence.
4. Apply Gates A–F using the fixed R2 development set:

```text
R2 development:
D5: [x2, n0, x2] --a1--> [x2, n1, x2]
D6: [x2, n0, x3] --a1--> [x2, n0, x3]
D7: [x2, n1, x2] --a1--> [x2, n1, x2]
D8: [x2, n0, x2] --a0--> [x2, n0, x2]
```

5. Freeze the state before exposing V1–V4.
6. Retest F1–F8 through ordinary retrieval, then test V1–V4.

No replay of final outputs is permitted before a prediction is recorded. The evaluator may then reveal the output for reporting, but that revelation belongs to evaluation or a later explicitly logged update.

## 12. Resource budgets

Budgets are logical operation units and must be counted in every implementation.

### Per training update

- Maximum candidate pairs: 16
- Maximum generated candidates: 32
- Maximum support comparisons: 128
- Maximum development checks: 32
- Maximum operation steps: 512
- Maximum active transition records: 32
- Maximum active candidate records: 64

### Per query

- Maximum retrieved candidates: 16
- Maximum candidate checks: 16
- Maximum operation steps: 64
- Maximum active records: 16

### Total run

- Maximum candidate records created: 128
- Maximum event records retained: 128
- Maximum full-state rebuilds: 0 during ordinary query evaluation
- Maximum hidden-state accesses: 0

If any budget is exhausted, the learner returns an explicit `incomplete` or `unsupported-by-budget` status. It may not return a confident prediction as if the check completed.

The first implementation must run CPU-only for reproducibility. A consumer GPU may be used in a later scale study, but GPU use is not part of the v1 claim.

## 13. Baselines and ablations

### Baseline B0: exact retrieval

Stores every training transition and predicts only when the exact input/action pair has been observed. Otherwise it abstains. B0 tests whether generalized learning adds value over memorization.

### Baseline B1: fixed oracle rule

A hand-written implementation of R1 and R2. B1 is an upper-bound functionality reference, not a learning system. Its code and hidden rule access must be excluded from the learner's measurements.

### Ablation A0: no variable generation

Uses G0 only. It should demonstrate the cost of memorization and establish the held-out generalization gap.

### Ablation A1: no development gate

Generates G1 but admits candidates without D1–D4. This measures the value of counterexample and non-applicability checks. It is expected to produce more false predictions in adversarial variants.

All baselines receive exactly the same transition records, query order, budgets, and scope information. None receives final outputs before prediction.

## 14. Metrics

For each query, record:

- response status: prediction, abstention, conflict, incomplete, or unsupported;
- exact output correctness when a prediction is made;
- applicability coverage: fraction of evaluator-applicable cases predicted;
- applicable accuracy: exact correctness among applicable predictions;
- false-prediction rate on non-applicable cases;
- abstention rate and reason;
- procedure version and evidence references;
- candidates generated, rejected, and admitted;
- symbol comparisons and support comparisons;
- operation steps;
- peak active records;
- wall-clock time and host memory in the eventual implementation.

Primary metrics are:

```text
applicable coverage = applicable cases receiving Prediction / applicable test cases
applicable accuracy = correct applicable predictions / all applicable predictions
false prediction rate = non-applicable cases receiving Prediction / non-applicable cases
retention drop = Phase-A R1 coverage − post-R2 R1 coverage
```

Coverage and accuracy must be reported separately. A system that abstains on everything has undefined applicable accuracy but zero useful coverage. A prediction on a non-applicable case is false for v1 even if its emitted sequence happens to equal the evaluator's identity output; the learner was not entitled to predict there.

## 15. Pre-registered acceptance gates

The learner passes the v1 structural-learning claim only if all gates pass:

1. **Reproducibility:** candidate order and admission decisions are identical across three deterministic reruns.
2. **No leakage:** no final record, hidden rule, semantic label, or evaluator-only state is read before prediction.
3. **Generalization:** R1 applicable coverage on F1–F5 is 5/5.
4. **Prediction correctness:** applicable accuracy on F1–F5 is 5/5.
5. **False-positive control:** false prediction rate on F6–F8 is 0%.
6. **Continual retention:** after learning R2, R1 coverage and applicable accuracy drop by no more than 5 percentage points.
7. **R2 generalization:** applicable coverage on V1–V2 is 2/2, with 2/2 applicable accuracy.
8. **R2 non-applicability:** false prediction rate on V3–V4 is 0%.
9. **Budget compliance:** no declared budget is exceeded and no incomplete query is reported as a prediction.
10. **Evidence traceability:** every admitted candidate has exact support and challenge references.

If any gate fails, report the failing gate and do not describe the run as successful. A pass demonstrates only v1's narrow structural claim; it does not validate the larger Brain Machine Model.

## 16. Required adversarial extensions

After the fixed primary test, run separate extensions without changing the primary result:

- permute all symbol identities while preserving the hidden rule;
- reorder independent training records;
- add irrelevant transition records;
- duplicate records from the same source;
- introduce a contradictory record in a separate scope;
- remove a development challenge to measure under-testing;
- corrupt one transition output;
- exhaust the query budget;
- increase outer-symbol cardinality;
- increase the number of unrelated rules.

Each extension must state whether it is testing symbol invariance, order robustness, evidence independence, scope control, corruption handling, budget honesty, or scaling. Extensions cannot be used to tune the primary grammar after final results.

## 17. What counts as learning in v1

The learner counts as having learned a structural rule only if:

- the rule was generated by G1 from at least two distinct support records;
- the rule applies to an outer symbol absent from its support records;
- its output is correct on a hidden applicable query;
- it abstains on hidden non-applicable queries;
- its evidence and candidate-generation cost are recorded;
- it remains functionally available after R2 is learned.

Exact replay alone is memorization. A human-supplied rule is supervision. A rule that passes only because all final cases were shown during training is leakage. A rule that predicts correctly but cannot identify its support and challenge history is not an admissible v1 result.

## 18. Explicit limitations

This experiment cannot establish:

- semantic understanding;
- raw-text learning;
- causal reasoning;
- arbitrary-length context;
- broad world modeling;
- planning;
- sensory perception;
- superiority over neural models;
- lower energy or faster execution at scale;
- absence of overfitting in other domains;
- unlimited memory;
- superintelligence.

It tests a small, discrete structural induction mechanism. Its value is that it can fail cleanly and reveal whether the proposed primitive operation and candidate grammar do anything beyond exact-example retrieval.

## 19. Implementation boundary

An eventual implementation may choose any language or storage technology, provided it exposes:

- the exact transition interface;
- candidate-generation logs;
- operation and memory counters;
- evidence and scope references;
- deterministic candidate ordering;
- budget exhaustion statuses;
- pre-prediction input/output separation.

The implementation must not add embeddings, pretrained tokenizers, gradient optimization, hidden semantic labels, or a default language model while still calling the result v1. Any such addition creates a new experiment version.

## 20. Decision status

This specification is complete enough for an implementation plan, but implementation has not begun. The next gate is human review of this file. After approval, the work should proceed in this order:

1. implement the language-independent transition and candidate ledger;
2. implement the fixed dataset and evaluator;
3. run B0 and A0 first;
4. run the main learner under the pre-registered gates;
5. run B1 only as a functionality reference;
6. publish raw traces, budgets, failures, and metrics;
7. decide whether the architecture deserves a larger experiment.

No result should be described as evidence for a general artificial brain until this v1 experiment and its limitations are reported honestly.

## 21. Design self-review

### Placeholder check

No unresolved placeholder or unspecified acceptance threshold remains in the primary experiment. Later extensions are intentionally outside the v1 decision boundary.

### Leakage check

- The evaluator's R1 and R2 rules generate outputs but are not learner inputs.
- Final records are withheld until the learner is frozen.
- Symbol category names are evaluator-only.
- No tokenizer, embedding, semantic dictionary, or pretrained component is allowed.
- Baseline B1 is isolated as an oracle reference.

### Metric check

- Applicability coverage measures whether the learner predicts when it should.
- Applicable accuracy measures correctness when it predicts on an applicable case.
- Non-applicable prediction is always a v1 false positive, even if the emitted output matches identity.
- Retention measures post-update functional use, not merely record existence.

### Contradiction check

- Candidates are partial predictors and may abstain.
- A negative example checks non-applicability when its output is supplied during development or evaluation; it does not force the learner to infer identity.
- The primary experiment uses a deterministic fixed dataset, not a hidden action-selection oracle.
- The v1 grammar does not claim arbitrary sequence alignment, language understanding, causal discovery, or unlimited memory.

### Scope check

The specification is one experiment, not the full assistant. It is small enough to implement and run without choosing a permanent language, database, or hardware architecture. Any broader capability requires a new design version with new baselines and acceptance gates.
