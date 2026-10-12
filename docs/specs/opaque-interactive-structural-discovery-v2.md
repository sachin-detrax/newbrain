# Opaque Interactive Structural Discovery v2

**Version:** v2.0
**Status:** T1 decision frozen; reviewable specification; implementation not authorized
**Date:** 2026-10-11
**Predecessors:** [`testable-design-v1.md`](../../testable-design-v1.md), [`brain-machine-model-draft.md`](../../brain-machine-model-draft.md)

## 1. Decision summary

The next experiment is a small interactive world. The learner receives opaque
observation packets and opaque action identifiers. It must discover a reusable
action-conditioned structural regularity from interaction and use that
regularity on a held-out context. The evaluator privately knows what the
symbols and transitions mean, but the learner never receives those meanings.

This specification freezes the research boundary for T1. It does not alter the
v1 fixture, v1 artifacts, or `g0-g1-v1` grammar. The v2 learner has a separate
grammar version and a separate evaluator boundary. Rust implementation begins
only after the v2 evaluation protocol in T2 is written and reviewed.

## 2. Research question

> Can a bounded structural learner, starting with only exact identity and
> order over opaque observation atoms, discover an action-conditioned invariant
> from interactive experience and use it to predict and control a held-out
> context, without being given semantic labels, event fields, a transition
> rule, or the hidden task condition?

The experiment separates two outcomes:

1. **Structural discovery:** the learner produces a reusable procedure whose
   support contains multiple interactions with different superficial context
   atoms and whose signature expresses the repeated structural role rather than
   an exact example.
2. **Grounding:** the learner uses that procedure to choose an action and/or
   predict the next opaque observation, and the resulting action is checked by
   the environment's returned observation and terminal outcome.

Grounding is operational in this specification. It means a tested relation
between the learner's opaque input/action history and a later observable
consequence. It does not mean that the learner has acquired the evaluator's
names for state, object, location, action, goal, or success.

A successful result supports only the narrow claim above. It does not support
semantic understanding, causal understanding, planning, broad world modeling,
raw-text learning, or general intelligence.

## 3. Frozen primary boundary

The primary experiment has these fixed properties:

- The world is finite, deterministic, and small enough for manual audit.
- A run uses one stable opaque atom/action mapping and one stable hidden
  transition rule.
- Each observation packet contains exactly three ordered opaque atoms. The
  packet boundary is a transport boundary after an environment step; it is not
  a semantic event boundary supplied by the evaluator.
- Each episode permits at most two learner actions.
- The learner chooses actions one at a time; the evaluator returns the next
  packet only after the action and prediction have been recorded.
- Training/development interaction may update the learner. The learner is
  frozen before any final held-out episode begins.
- The primary held-out transformation is substitution of a context atom that
  was not present in the learner's discovery support.
- Full atom/action symbol permutation is a pre-declared robustness extension,
  not a reason to change the primary grammar or gates after results are seen.
- No stochasticity, raw text, continuous sensor input, variable-length
  segmentation, or multi-step planning is part of the primary claim.

The fixed packet length is intentionally retained to isolate the new question:
whether interaction and terminal consequences can ground structural discovery.
Raw-stream boundary discovery and variable-length alignment remain later
experiments rather than hidden work in v2.

## 4. Exact learner input interface

The learner is called with a `StepInput` for every non-terminal observation.
Logical identifiers are exact opaque values. Their storage width and wire
encoding are implementation choices; their equality and ordering behavior are
not.

```text
AtomId       = opaque exact identifier
ActionId     = opaque exact identifier
ScopeToken   = opaque exact identifier
ProcedureId  = learner-assigned exact identifier
EvidenceId   = learner-assigned exact identifier
HypothesisId = learner-assigned exact identifier

ObservationPacket = {
  atoms: [AtomId, AtomId, AtomId]
}

TerminalStatus = continue | success | failure | incomplete
PreviousTerminal = none | continue

StepInput = {
  run_scope: ScopeToken,
  episode_scope: ScopeToken,
  step_index: 0 | 1,
  observation: ObservationPacket,
  available_actions: [ActionId, ActionId],
  remaining_action_budget: 1 | 2,
  previous_terminal: PreviousTerminal
}
```

Input rules:

1. `run_scope` is the same opaque token for all episodes in one run and is
   different for a symbol-permutation run or a changed-rule run.
2. `episode_scope` is fresh for every episode. It prevents evidence from
   being treated as an observation from the wrong episode, but it does not
   describe the episode's hidden state.
3. `available_actions` is presented in exact identifier order. `step_index`,
   packet length, action availability, and remaining budget are transport
   metadata. They do not identify an object, state, event, action meaning, or
   goal.
4. `previous_terminal` is `none` on the first input and `continue` on the
   second input. A terminal `success`, `failure`, or `incomplete` result ends
   the episode and is never followed by another `StepInput`.
5. The learner receives no input/output field names beyond this transport
   contract. In particular, the three positions are not called subject,
   object, state, source, destination, or outcome.
6. The learner receives no evaluator seed, hidden state, world-family label,
   target atom, expected next packet, or evaluator score before acting.

The environment's post-action result is:

```text
StepResult = {
  observation: ObservationPacket,
  terminal: continue | success | failure
}
```

`StepResult` is evaluator output after an action. It is not available to the
learner while constructing the action or prediction that caused it. If the
learner abstains instead of acting, the evaluator closes the episode with
`incomplete` and does not fabricate an observation.

## 5. Exact learner output/response interface

For each `StepInput`, the learner emits exactly one `Decision` before the
environment is stepped.

```text
Decision = Act {
  action: ActionId,
  prediction: optional ObservationPacket,
  mode: exploration | procedure | fallback,
  procedure_refs: [ProcedureId],
  evidence_refs: [EvidenceId],
  hypothesis_refs: [HypothesisId]
} | Abstain {
  reason: no_applicable_procedure
        | conflict
        | insufficient_evidence
        | unsupported_by_grammar
        | incomplete_budget,
  procedure_refs: [ProcedureId],
  evidence_refs: [EvidenceId],
  hypothesis_refs: [HypothesisId]
}
```

Response semantics are fixed:

- `action` must be one of `available_actions`.
- `prediction`, when present, is a prediction of the next returned opaque
  packet for the selected action. It is scored only after the evaluator returns
  `StepResult`.
- In `procedure` mode, `prediction` is required whenever the selected
  procedure has completed its output checks. In `exploration` or `fallback`
  mode it may be absent; absence is recorded as no prediction rather than
  treated as an implicit identity prediction.
- An action without a prediction is a valid control attempt but supplies no
  prediction-accuracy evidence.
- `procedure` means at least one admitted learner procedure selected the
  action; `exploration` means the action was selected to distinguish active
  hypotheses; `fallback` means no admitted procedure or disagreement rule
  determined the action and the fixed exploration tie-break was used.
- `procedure_refs`, `evidence_refs`, and `hypothesis_refs` are audit data. They
  must refer only to learner-created records available before the decision.
- `Abstain` performs no environment action and ends the episode as
  `incomplete`. It is not an identity prediction and is not scored as a
  failure to predict a packet.
- A budget exhaustion, contradiction, or unsupported grammar result cannot be
  emitted as `Act` with a confident prediction merely because a candidate was
  found before its checks completed.

The evaluator records the `Decision` before calling the hidden environment.
After the result is recorded, the learner may ingest the result in a mutable
training/development run. A frozen final learner may record the result for
trace purposes but may not change its procedures, hypotheses, or action policy.

## 6. Hidden evaluator world

The following names are evaluator notation only. They must not occur in
learner input, candidate labels, procedure names, or learner-generated
semantic explanations.

The evaluator's primary world contains:

- context values `c0`, `c1`, `c2`, and `c3`;
- state-marker values `m0` and `m1`;
- action values `a0` and `a1`;
- hidden state `(context, marker)`;
- a hidden task condition that is satisfied when the marker is `m1`.

The observation encoding is:

```text
hidden (c, m)  ->  [c, m, c]
```

The evaluator's deterministic transition is:

```text
if action = a0 and marker = m0:
  next marker = m1
else:
  next marker = marker
context is always preserved
```

Thus the evaluator returns:

```text
[c, m0, c] --a0--> [c, m1, c]
[c, m1, c] --a0--> [c, m1, c]
[c, m0, c] --a1--> [c, m0, c]
[c, m1, c] --a1--> [c, m1, c]
```

These equations define the evaluator, not a rule supplied to the learner.
The learner sees only the packets, selected actions, returned packets, and
post-action terminal status.

The evaluator emits `success` after an action whose resulting hidden marker is
`m1`. It emits `continue` after a non-success action while an action remains.
It emits `failure` when the episode ends with the hidden marker still `m0`.
The evaluator may use its hidden condition to score the environment, but the
learner is never told which packet position or symbol represents it.

The required primary episode strata are:

1. **Discovery:** interactions with `c0` and `c1` that can expose the
   `m0 -> m1` consequence under one action.
2. **Non-applicability:** interactions that expose that the same action does
   not change an already-`m1` packet and that the other action does not change
   an `m0` packet.
3. **Held-out context:** final interactions with `c2` and `c3`, withheld until
   the learner is frozen, where the repeated outer position must transfer.

T2 freezes the exact episode order, counts, generator seed(s), training and
development partitions, and acceptance thresholds within these strata. T2 may
not change the world dynamics, interface, action budget, or leakage boundary.

## 7. Scope and episode rules

### 7.1 Run scope

A run is the unit of a stable opaque alphabet and stable transition rule.
Procedures may be promoted from episode scope to run scope only after their
support and challenge evidence passes the v2 admission gates. A run-scoped
procedure may be reused in later episodes of that run.

No run-scoped procedure may be used automatically in another run. A separate
symbol-permutation run begins with a fresh learner and fresh run scope. A
changed-rule run also begins with a fresh learner unless a later extension
explicitly tests revision.

### 7.2 Episode scope

- Every episode receives a fresh `episode_scope`.
- The environment resets the hidden context and marker according to the frozen
  evaluator partition.
- The learner receives at most two `StepInput` values for one episode.
- The learner must record a decision before each environment action.
- A terminal result ends the episode; no later action may use that episode's
  scope.
- Evidence from an episode remains immutable. A later result can challenge or
  version a procedure but cannot rewrite the original observation or action.

### 7.3 Freeze boundary

The learner has three modes:

1. **Discovery mode:** training interactions may add observations, candidate
   hypotheses, rejection records, and provisionally admitted procedures.
2. **Development mode:** the fixed development interactions may be used for
   candidate admission and tie-breaking only. They are not final results.
3. **Frozen mode:** before the first final held-out episode, all learner state
   and candidate ordering are frozen. Final `StepResult` values cannot enter
   procedure generation, action selection, or configuration selection.

The final evaluator must obtain a decision record before revealing each result.
No final observation, terminal outcome, or final score may be replayed as
training input before that decision record exists.

## 8. Allowed supervision

The following are allowed and are the complete learner-facing supervision
boundary:

- exact atom identity and packet order;
- exact action identity and the list of currently available actions;
- transport packet length, step order, run scope, episode scope, and remaining
  action budget;
- the returned opaque packet after the learner's action;
- the terminal status `success`, `failure`, `continue`, or `incomplete` after
  the relevant action or abstention;
- deterministic computation, storage, exact comparison, binding, grouping,
  branching, checking, and declared budget accounting;
- a fixed canonical order for deterministic enumeration and tie-breaking.

The terminal status is outcome supervision, not a semantic world label. It may
ground the consequence of an action, but it does not identify the hidden target
or explain why the action produced the outcome.

The following are not allowed as learner input or hidden helper behavior:

- semantic names for atoms, positions, actions, states, entities, goals, or
  outcomes;
- object/location/event/action fields or pre-segmented semantic records;
- positive/negative transition labels supplied before the learner acts;
- expected next packets or hidden applicability flags before prediction;
- evaluator seed, latent state, transition table, target condition, or oracle
  controller access;
- a semantic dictionary, pretrained representation, embedding, similarity
  score, tokenizer, parser, reward model, gradient, or neural optimizer;
- a human-selected candidate, manually inserted procedure, or hand-selected
  action answer;
- evaluator scoring feedback before the decision is recorded.

If an adapter or test harness supplies a prohibited item, it is supervision and
the run is not a valid v2 run.

## 9. v2 structural grammar

The v2 grammar is a new, named grammar. It does not modify, extend, or replace
`g0-g1-v1`. Its fixed version string is:

```text
opaque-interactive-structural-v2
```

The grammar operates on learner-owned interaction traces of the form:

```text
Interaction = {
  before: ObservationPacket,
  action: ActionId,
  after: ObservationPacket,
  terminal: TerminalStatus,
  episode_scope: ScopeToken,
  evidence_id: EvidenceId
}
```

### 9.1 Candidate stages

Candidates are generated in this order:

1. **Exact replay (`E0`):** store an exact observed interaction for the
   exact-retrieval comparison. `E0` is never counted as structural discovery.
2. **Aligned effect (`E1`):** compare two completed interactions with the same
   action and packet length. Record each unchanged position, each changed
   position, and each within-packet equality relation. Emit a candidate only
   when the effect is stable across both interactions.
3. **Repeated-role schema (`E2`):** replace a context value with one scoped
   variable only when the same input equality relation occurs in every support
   interaction and the replacement transfers across at least two different
   context atoms. Output operations may copy an input position or emit an atom
   observed in every support result. No symbol may be invented.
4. **Applicability guard (`E3`):** retain the exact action condition and the
   observed precondition for the changed position. A candidate must be
   challenged by at least one interaction in which the action or precondition
   differs and the predicted change does not occur.
5. **Run-scope promotion (`E4`):** after all v2 admission gates pass, retain
   the structural procedure for later episodes in the same run. Promotion is
   an indexed version of the full candidate and must preserve expansion,
   support, challenge, and rejection references.

No v2 candidate may use arbitrary predicates, free-form semantic roles,
unbounded variable partitions, variable-length alignment, hidden state,
terminal status as a replacement for observation evidence, or a hand-written
transition rule. A candidate that cannot be generated by `E0`–`E4` is
`unsupported-by-grammar`.

### 9.2 Candidate identity and order

Every candidate has a canonical signature containing, in order:

1. grammar version and stage;
2. action identifier;
3. packet length;
4. input equality/constant/variable pattern;
5. unchanged and changed positions;
6. output copy/observed-atom operations;
7. run/episode scope;
8. sorted support evidence identifiers.

Candidates are enumerated by stage, structural edit count, first support
evidence identifier, action identifier, position order, and exact identifier
order. Duplicate signatures are merged by unioning their evidence references;
the first canonical candidate owns the procedure identifier. The ordering is a
deterministic tie-break, not a semantic prior.

### 9.3 Admission gates

An `E2`/`E3` candidate is provisionally admitted only when all of these hold:

- **Support:** at least two completed interactions support it.
- **Variation:** support interactions differ in the context value represented
  by the variable.
- **Consistency:** every support interaction has the predicted returned
  packet.
- **Structural traceability:** the candidate records exact changed,
  unchanged, and repeated-position evidence.
- **Non-applicability challenge:** the candidate abstains or predicts no
  change on the fixed contrasting interactions required by T2.
- **Grounding evidence:** at least one selected action was chosen through the
  candidate or its explicit disagreement test, and the post-action result was
  recorded after the decision.
- **Scope:** no evidence from another run or unrelated episode scope is mixed
  without the explicit run-scope promotion in `E4`.
- **Cost:** all generation, comparison, challenge, and execution work is
  within the frozen v2 budgets.

Admission is provisional. Later contradiction creates a new version,
suspension, narrowing, or unresolved status; it does not mutate the earlier
evidence ledger.

## 10. Action selection and grounding trace

The learner's action policy is fixed so exploration cannot be hidden inside an
unspecified helper:

1. Reject an action that is not in `available_actions`.
2. If one admitted procedure applies and predicts a result, select its action
   and emit that predicted packet.
3. If competing procedures predict different outcomes for the same required
   action, emit `Abstain(conflict)` unless a declared hypothesis-separating
   action is available within the remaining budget.
4. If an action can distinguish active hypotheses, select the lowest-cost
   such action and mark the decision `exploration`.
5. Otherwise select the first as-yet-untried available action in canonical
   identifier order and mark the decision `fallback`. Ties in all earlier
   steps use the same canonical order.
6. If the action or structural search budget is exhausted, emit
   `Abstain(incomplete_budget)`.

Every action record states which of these rules selected it. Random exploration
is not part of the proposed learner; it is a separately measured baseline.

The grounding trace consists of the pre-action observation, selected action,
optional prediction, post-action observation, terminal status, procedure and
evidence references, and logical cost. A correct packet prediction is one form
of grounding evidence. A successful held-out action selected by an admitted
procedure is another. Neither is a semantic label.

## 11. Primary budgets and failure statuses

The following logical ceilings are fixed for v2 and must be visible in the
ledger. T2 may not raise them after inspecting final results:

| Resource | Per decision | Per episode | Per run |
| --- | ---: | ---: | ---: |
| Environment actions | 1 | 2 | declared by T2 partition |
| Candidate generation attempts | 64 | 128 | 512 |
| Structural comparisons | 256 | 512 | 2,048 |
| Active hypotheses | 16 | 16 | 64 |
| Operation steps | 1,024 | 2,048 | 8,192 |
| Retained interaction records | 32 | 32 | 128 |
| Retained candidate/procedure records | 32 | 64 | 128 |

Wall-clock time, process memory, and hardware measurements are reported by
the implementation and acceptance protocol; they are not converted into a
claim about v1. Logical budget exhaustion returns an explicit incomplete or
unsupported status. It never becomes a negative fact or a confident
prediction.

## 12. Hypotheses and null comparisons

### H1 — structural discovery beyond exact replay

After discovery and development, the proposed learner admits at least one
procedure supported by multiple interactions with different context atoms and
applies it to a held-out context atom. Exact retrieval and exact replay alone
must not receive credit for H1.

### H2 — grounded consequence use

On held-out contexts, a procedure-backed decision produces a correct next
opaque packet prediction and/or a successful task action at a rate that meets
the preregistered T2 gate. Coverage, prediction accuracy, task success, and
abstention are reported separately.

### H3 — evidence and uncertainty honesty

The learner does not predict on the required non-applicable contrasts, does
not treat an unobserved packet as an identity result, preserves competing
hypotheses when evidence is insufficient, and reports budget exhaustion as
incomplete.

### H4 — surface-code robustness extension

When the same protocol is rerun with a bijection over atom and action IDs, the
learner's structural behavior should remain auditable and should not depend on
the spelling or numeric ordering of the v1 fixture. This is a secondary
extension because a fresh permutation run permits interaction-based discovery;
it is not a zero-shot claim about an unknown action mapping.

### Null and reference comparisons

- **B0 exact interaction retrieval:** uses only exact observed
  `(observation, action)` interactions and abstains on unseen contexts.
- **B1 random exploration:** chooses from available actions using a fixed,
  disclosed seed and the same action budget; it has no structural procedure
  store.
- **B2 fixed controller/reference:** evaluator-only controller with access to
  the hidden world rule. It is a functionality reference, not a learner and
  not part of learner resource measurements.

The proposed learner, B0, B1, and B2 use the same episode strata, action
budgets, terminal feedback timing, and final freeze boundary. T2 defines the
exact matched seeds, reports, and acceptance thresholds.

## 13. Inherited unchanged from v1

The following v1 properties are inherited as experiment invariants:

- opaque exact identity and order are legitimate primitive substrate
  operations;
- candidates and procedures are explicit, versioned, scope-aware, and linked
  to evidence;
- prediction and abstention are distinct responses;
- identity is never asserted merely because no learned procedure applies;
- final evaluator information is withheld until the learner has recorded a
  response;
- evidence, candidate costs, operation counts, budget failures, and response
  statuses are auditable;
- deterministic canonical ordering is fixed before final evaluation;
- a timeout or budget exhaustion cannot become a confident result;
- exact retrieval and a fixed evaluator-only controller remain comparison
  instruments rather than hidden dependencies;
- CPU-only reproducibility is the primary execution assumption until a later
  scale study says otherwise;
- a narrow pass or failure is reported only within the tested scope.

The following v1 artifacts are immutable for this work:

- `testable-design-v1.md`;
- `docs/superpowers/plans/2026-10-10-sparse-structural-rule-learner-v1.md`;
- `artifacts/v1/` and its recorded reports;
- the fixed v1 dataset, evaluator, and `g0-g1-v1` grammar;
- the historical v1 result `validated-v1-narrow-claim`.

The v2 implementation may share generic engineering concepts with v1, but it
must expose a separate v2 grammar/version and must not silently reinterpret a
v1 artifact as an opaque interactive result.

## 14. Explicit exclusions

The v2 primary claim excludes:

- raw text, language learning, tokenization, semantic naming, and multimodal
  input;
- images, audio, continuous sensors, learned sensory adapters, and arbitrary
  stream segmentation;
- supplied object, location, entity, event, action, state, goal, semantic
  success, or causal labels. The post-action terminal status in Section 8 is
  allowed outcome feedback and is not a supplied explanation of success;
- hidden-state access, oracle retrieval, an evaluator-selected candidate, or a
  hand-authored learner controller;
- causal identification beyond the observed action-conditioned consequence;
- multi-step planning, broad world models, open-world reasoning, and general
  intelligence;
- learned rewards, gradient descent, neural weights, embeddings, dense tensor
  foundations, or next-token objectives;
- arbitrary-length packets, variable-length alignment, primitive invention,
  unbounded candidate enumeration, or scale superiority;
- GPU speed, energy, wall-clock, RAM, or throughput claims based on v1;
- claims that a successful opaque procedure has a human semantic meaning;
- changes to v1 artifacts, v1 grammar, v1 gates, or historical v1 results.

The excluded capabilities may be considered only in a later version with its
own input contract, supervision audit, grammar, baselines, and gates.

## 15. Implementation seam and module responsibilities

The implementation seam is language-independent. The following responsibilities
must remain separate even if a later runtime places them in different files.

### 15.1 `opaque_protocol`

- Validate `StepInput`, `Decision`, and `StepResult` against the exact wire
  contract.
- Enforce action availability, packet length, step order, and terminal
  sequencing.
- Contain no hidden-world lookup and no semantic names.

### 15.2 `evaluator_world` (evaluator-only)

- Generate hidden context/marker episodes from T2's frozen partitions.
- Apply the hidden deterministic transition and terminal condition.
- Own seeds, symbol permutations, expected observations, applicability, and
  the fixed controller.
- Never be linked into learner prediction, candidate generation, or learner
  cost accounting.

### 15.3 `interaction_ledger`

- Append observations, actions, returned observations, terminal statuses,
  scopes, provenance, and logical costs.
- Preserve pre-decision and post-result ordering.
- Keep observed, inferred, simulated, disputed, and incomplete records
  distinguishable.
- Prevent final evaluator results from entering mutable learner state before
  their response record.

### 15.4 `structural_observation`

- Compare exact atoms and positions.
- Detect unchanged positions, changed positions, and repeated-position
  equality relations.
- Preserve alternative comparisons and their costs.
- Perform no semantic classification and no hidden alignment beyond the fixed
  packet positions.

### 15.5 `grammar_v2`

- Implement only `E0`–`E4` from this specification.
- Generate deterministic canonical signatures and candidate IDs.
- Record every generated, merged, rejected, and budget-exhausted candidate.
- Keep the expansion of promoted procedures available for audit.
- Have no import path that mutates or changes `g0-g1-v1`.

### 15.6 `exploration_and_decision`

- Apply the fixed action-selection order in Section 10.
- Record predictions, disagreement tests, fallback choices, and action costs.
- Use only learner-owned evidence and hypotheses.
- Never call `evaluator_world` or inspect evaluator-only expected results.

### 15.7 `admission_and_procedure_store`

- Apply the support, variation, consistency, traceability, contrast,
  grounding, scope, and cost gates.
- Store immutable procedure versions with expansion, support, challenge,
  rejection, and known-failure references.
- Promote only from episode scope to the current run scope.
- Mark contradiction and drift for revision instead of deleting history.

### 15.8 `learner_engine`

- Orchestrate discovery, development, freeze, decision, and post-action
  ingestion.
- Expose only `StepInput -> Decision` at the live boundary and a separate
  post-result ingestion call for mutable runs.
- Make frozen state non-mutating and reject final-data training calls.
- Enforce logical budgets and explicit incomplete statuses.

### 15.9 `baselines`

- Implement B0 and B1 behind the same decision interface and action budgets.
- Keep B2 in evaluator-only code.
- Disclose that random exploration and the fixed controller are comparisons,
  not parts of the proposed learner.

### 15.10 `evaluator_and_reporter`

- Record a decision before revealing its result.
- Score packet predictions, task outcomes, coverage, false predictions,
  abstentions, evidence traceability, retention, budgets, and resource use.
- Produce separate primary and extension reports.
- Never write evaluator-only hidden semantics into learner traces.

The required public seam is therefore:

```text
EvaluatorWorld::reset(run_scope, episode_scope)
  -> initial ObservationPacket

Learner::step(StepInput)
  -> Decision

EvaluatorWorld::apply(ActionId)
  -> StepResult

Learner::observe(StepResult)       # mutable discovery/development only
Learner::freeze() -> FrozenLearner
FrozenLearner::step(StepInput)     # no mutation
```

The names above are contracts, not Rust APIs. Internal storage, serialization,
and concrete language remain implementation choices only where they cannot
change this observable behavior.

## 16. T1 freeze audit

The following decisions are frozen in this document:

| Required T1 decision | Frozen answer |
| --- | --- |
| Research question | Interactive discovery and grounding of an action-conditioned opaque structural invariant |
| Learner input | Exact `StepInput` in Section 4; no semantic fields or hidden state |
| Evaluator-only information | Hidden world, dynamics, target condition, expected results, seeds, partitions, baselines, and scoring |
| Output/response | Exact `Decision` in Section 5; pre-action prediction/action or explicit abstention |
| Scope/episodes | Run scope, fresh episode scope, two actions maximum, post-action feedback, freeze before final |
| Allowed supervision | Exact identity/order, transport metadata, post-action packet, post-action terminal status, finite budgets |
| Exclusions | Section 14; no semantic/raw-text/planning/scale claims |
| Hypotheses | H1–H4 and B0–B2 in Section 12 |
| v1 inheritance | Section 13; v1 artifacts and grammar remain immutable |
| Implementation seam | Section 15; evaluator and learner boundaries are separate |
| Candidate grammar | `opaque-interactive-structural-v2`, stages `E0`–`E4` |
| Primary world boundary | Four context IDs, two marker IDs, two action IDs, fixed three-atom packet, deterministic rule, two-action episode |
| Budget behavior | Fixed logical ceilings and explicit incomplete statuses in Section 11 |

There are no unresolved implementation decisions in the primary experiment.
An implementation may choose its programming language, internal collections,
serialization, and storage layout. It may not choose a different wire
contract, hidden-world rule, action timing, scope policy, candidate grammar,
freeze boundary, budget meaning, or evaluator access path.

T2 remains responsible for protocol details that are intentionally downstream
of this decision: exact episode order and counts, generator seeds, training/
development/final partitions within the frozen strata, preregistered numerical
acceptance thresholds, baseline seed matching, and the final resource-report
format. T2 may fill in those protocol values but may not alter any T1 boundary
listed above.

## 17. Review outcome and next gate

This document completes T1 when reviewed as a separate v2 specification. The
next gate is T2: write the executable evaluation protocol before any Rust code
or v2 implementation module is created. A T2 protocol must be runnable by
another person without adding semantic labels, hidden outcome access,
post-hoc acceptance gates, or an unstated learner capability.
