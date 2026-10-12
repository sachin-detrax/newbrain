# Architecture-gap research: can the sparse structural rule learner become a broader learning framework?

**Date:** 2026-10-11  
**Repository:** `sparse-structural-rule-learner` / `newbrain`  
**Question:** Whether the current v1/v2 sparse structural rule learner can evolve into a more capable learning framework, and what established approaches reveal about the missing capabilities.

## Executive conclusion

**Yes, the current learner can evolve into a more capable framework, but the evolution is architectural rather than a matter of adding more `E0`–`E4` cases.** The repository already contains a credible, auditable kernel for a narrow capability: discovering one bounded structural invariant from opaque interactions, grounding it through returned outcomes, retaining it while learning a related rule, abstaining when evidence or grammar is insufficient, and reporting the supporting evidence and costs.

The evidence supports three conclusions:

1. **The current kernel is worth preserving.** v1 and v2 are not empty prototypes. Their recorded artifacts validate narrow claims under fixed, preregistered, deterministic protocols. The explicit evidence ledger, candidate admission, scope handling, versioning seam, abstention semantics, and leakage boundary are useful foundations for symbolic, program-synthesis, model-based, or hybrid extensions.
2. **The current kernel is not yet a general learning framework.** Its representation and search space are deliberately too small: exactly three opaque atoms, at most one repeated variable, fixed aligned positions, short episodes, deterministic transitions, no learned primitive library, no variable-length structure, no explicit goals or reward model, no multi-step planner, no stochastic belief state, no raw-stream adapter, and no durable persistence. The literature treats these as separate mechanisms, not as consequences that appear automatically once a small rule learner passes held-out transfer.
3. **The engineering MVP should proceed within its existing boundary, then architectural expansion should be preceded by decision experiments.** Do not add planning, stochasticity, raw text, or neural representations to the current MVP. Finish the specified restartable runtime and two-environment validation unchanged in scope. Before selecting the next major capability, run small architecture-decision experiments for (a) variable-length representation and boundary discovery, (b) learned transition models plus goal-directed planning, and (c) primitive/library discovery and composition. Those experiments should create new specifications and evaluation protocols rather than silently widening v2.

In short: **proceed with the MVP unchanged in scope; do not treat it as the final architecture; use decision experiments before the next expansion.** The strongest likely direction is a layered hybrid: an auditable symbolic evidence/procedure substrate, a learned representation or parser at the input boundary, a learned or symbolic transition model with uncertainty, and a bounded planner whose hypothetical traces remain explicitly distinct from observed evidence.

## 1. Scope and method

This report compares the repository with original papers, official project papers, publisher metadata, and the repository's own frozen specifications and recorded artifacts. It emphasizes mechanisms and capability boundaries rather than ranking systems by benchmark score.

For each research area, the report captures:

- **Core mechanism** — what the approach actually adds;
- **Capability achieved** — what the cited work demonstrates;
- **Limitation** — what remains assumed, bounded, or unsolved;
- **Relevance to this repository** — the architectural gap or reusable idea;
- **Primary citation** — original paper, DOI, arXiv record, or first-party specification.

The report uses “theoretically feasible but untested” for capabilities that the current design documents make plausible or explicitly schedule, but that are not implemented and validated as a reusable capability. “Open research question” is reserved for questions where a new representation, search regime, evidence policy, or evaluation design is still required.

## 2. Repository baseline

### 2.1 What v1 implemented and validated

The v1 design and artifacts define a deliberately narrow experiment:

- Each input is a fixed-length sequence of exactly three opaque atoms.
- The learner has two opaque action symbols and exact symbol equality/order, but no semantic labels.
- The primitive vocabulary includes ingest, equality, fixed-position reads, binding, copy/emit, comparison, enumeration, checking, storage, exact retrieval, and abstention.
- `G0` stores exact transition candidates as a memorization baseline.
- `G1` performs two-record anti-unification: repeated input positions may become one variable, and output positions may copy an input position or emit an observed stable symbol.
- Candidates require support, consistency, structural variation, development challenges, and budget compliance. A candidate is rejected rather than improvised when it is outside the grammar.
- Predictions and abstentions carry procedure identifiers and evidence references.
- v1 has no raw text, multimodal input, arbitrary-length sequence alignment, composition of multiple learned procedures, reward learning, or persistent archive.

The v1 artifact records `validated-v1-narrow-claim`: all primary gates passed, including 5/5 held-out R1 coverage and correctness, 5/5 R1 retention after R2, 2/2 R2 held-out coverage/correctness, no false predictions on the fixed non-applicable controls, budget honesty, and evidence traceability. The three primary reruns had matching report and ledger hashes. This validates the fixed G1 structural claim only; the artifact explicitly disclaims language learning, planning, causal reasoning, perception, general world modeling, and superiority over neural systems. [R-v1]

### 2.2 What v2 implemented and validated

v2 adds an opaque interactive boundary while retaining a very small grammar:

- The learner sees `StepInput` containing a run scope, fresh episode scope, step index, a three-atom `ObservationPacket`, two available opaque actions, remaining action budget, and previous terminal status.
- The evaluator owns hidden labels, seeds, mappings, state, transition logic, terminal consequences, and scoring. The decision is serialized before the result is available.
- The primary environment is finite and deterministic. An episode has at most two action slots. The learner is mutable through training/development and frozen before final evaluation.
- `E0` is exact replay; `E1` records aligned effects; `E2` introduces one repeated-role variable; `E3` adds an action/precondition applicability guard; `E4` promotes an admitted procedure to run scope.
- v2 uses explicit evidence, hypothesis, candidate, procedure, scope, rejection, cost, and revision records. It also includes a partial-observation seam in which unavailable evidence is not treated as a negative fact.
- Grounding can select actions through a procedure, use disagreement among hypotheses, fall back to canonical exploration, or abstain. Contradiction/revision traces preserve old procedure versions and evidence.

The v2 artifact records `validated-v2-narrow-claim`: primary gates P1–P12 and secondary gates S1–S7 passed; three-rerun logical traces were deterministic; the primary and symbol/action-permutation variants were separated; the evaluator-only controller was excluded from learner totals; and learner physical reports included measured memory and nonzero wall-clock values. This validates opaque interactive structural discovery under the frozen v2 grammar, not a learned world model or general planning. [R-v2]

### 2.3 Current architecture boundary

| Capability dimension | Current v1/v2 status | Consequence for future evolution |
|---|---|---|
| Input representation | Exactly three ordered opaque atoms per packet | There is no input adapter for raw streams, variable-length sequences, continuous values, text, images, or learned segmentation. |
| Variables and relations | v1 `G1`; v2 permits one repeated equality variable and fixed aligned positions | Relational abstraction is possible only for a tiny equality pattern. There is no learned relation vocabulary or arbitrary variable partition. |
| Outputs | Copy an input position or replay an observed stable atom | There is no arithmetic, object construction, state update graph, conditional composition, or unseen primitive output. |
| Grammar/search | Finite, deterministic, budgeted `G0/G1` or `E0`–`E4` | Search is inspectable and reproducible, but expressivity is intentionally low. |
| Memory | Append-only interaction/evidence traces and in-memory candidate/procedure/hypothesis records | Durable persistence and fresh-process restoration are specified for the MVP but not yet present. |
| Learning target | Structural prediction and action-conditioned interaction regularity | No learned reward/value function, latent state estimator, probabilistic transition model, or goal representation. |
| Environment | Tiny deterministic worlds, two actions, at most two steps per episode | No stochasticity, delayed reward, long-horizon credit assignment, or robust model error handling. |
| Planning | No long-horizon search; procedure selection is one-step/short-episode action choice | A planner needs explicit state transitions, goals, hypothetical rollouts, and a cost/risk policy. |
| Continual learning | Same-run retention, freeze boundary, contradiction/versioning seams | Lifelong cross-run reuse, persistence, task-boundary detection, and storage/revalidation policy remain untested. |
| Concept discovery | Opaque structural roles only; no semantic concepts or primitive invention | There is no mechanism for discovering reusable abstractions from repeated subprocedures or raw observations. |
| Language/multimodality | Explicitly excluded | Any text or sensory front end is a new representation-learning and grounding problem, not an input-format patch. |

## 3. What established approaches reveal

### 3.1 Symbolic AI architectures

#### Core mechanism

Classical symbolic architectures make internal state explicit as symbols, propositions, rules, goals, operators, or production memories. McCarthy's **Advice Taker** frames logical representations in memory as a way to combine stored commonsense advice with deduction and action-related reasoning. [S1] SOAR develops a general problem-solving architecture around working memory, production rules, problem spaces, subgoaling, and chunking: problem solving can create new rules that compile recurring reasoning traces into reusable knowledge. [S2]

#### Capability achieved

These architectures support inspectable symbolic state, explicit reasoning traces, goal/subgoal decomposition, production-rule execution, and some forms of learning by compilation. Their important contribution for this repository is architectural separation: a system can have a persistent symbolic memory, a control cycle, a problem-space or operator interface, and a learned procedure layer without conflating those with the environment's hidden labels.

#### Limitation

The classical systems do not solve the representation problem by themselves. They generally assume a symbolic vocabulary, operator semantics, or domain interface. Chunking can compile useful rules from problem-solving traces, but it does not by itself discover grounded perceptual concepts, learn arbitrary action dynamics, or decide which raw observations should become symbols. Production systems also face conflict-resolution and search-control issues as rule sets grow; SOAR's architecture is a framework for those mechanisms, not evidence that an arbitrary opaque stream will automatically become a useful symbolic problem space.

#### Relevance to this repository

The repository already resembles the most auditable part of this tradition: explicit working records, candidate rules, scopes, evidence, rejection reasons, and deterministic control. The missing architectural layer is a **general problem-space/state interface**. A future procedure should be able to describe preconditions, effects, intermediate state, and subprocedures rather than only match one fixed packet and emit three aligned atoms. The current evidence ledger is a good substrate for such rules, but it should not be mistaken for a symbolic world model or a goal system.

#### Primary sources

- John McCarthy, “Programs with Common Sense” (1959), first-party Stanford-hosted paper: [S1](https://www-formal.stanford.edu/jmc/mcc59/mcc59.html).
- John E. Laird, Allen Newell, and Paul S. Rosenbloom, “SOAR: An architecture for general intelligence,” *Artificial Intelligence* 33(1), 1–64 (1987), DOI [10.1016/0004-3702(87)90050-6](https://doi.org/10.1016/0004-3702(87)90050-6). Publisher metadata was verified through Crossref. [S2]

### 3.2 Inductive learning and program synthesis

#### Core mechanism

Inductive program synthesis searches for a program in a domain-specific language whose execution satisfies examples, constraints, or observations. The critical extension beyond v1/v2 is **library learning**: learned programs can be compressed into reusable abstractions and then used as primitives in later programs. DreamCoder alternates a “wake” phase that solves tasks with a “sleep” phase that compresses solutions into a growing symbolic library; neural models guide search, while the resulting programs and abstractions remain interpretable. [P1]

Probabilistic program induction takes a related route: represent concepts as generative programs with priors over program structure, then infer a compact program from sparse examples. Lake, Salakhutdinov, and Tenenbaum's Bayesian program model for handwritten characters demonstrates one-shot-like concept learning and generation from a structured program representation rather than a flat classifier. [P2]

#### Capability achieved

These systems achieve compositional generalization, reusable abstractions, program-like explanations, and transfer to tasks that share a learned library. They show a direct path from the repository's two-record anti-unification to a richer learner: candidates can be composed, factored, named by stable structural identity, and promoted when reuse across tasks justifies the abstraction. They also provide a way to treat primitive discovery as a separate, measurable operation rather than allowing arbitrary new operations to appear inside ordinary repair.

#### Limitation

Program synthesis is only as open-ended as its language, types, primitives, priors, and task interface. DreamCoder uses a typed functional language and neural search guidance; the concept-learning work uses a structured domain-specific generative language and strong inductive biases. A finite DSL can be highly capable in its domain while still failing outside its representational envelope. Library learning also creates a credit-assignment problem: a bad abstraction can shorten some programs while harming future search, and an expanding library can increase search and verification cost.

#### Relevance to this repository

This is the clearest literature-based gap. v1/v2 can discover one bounded relation, but they cannot:

- discover a new reusable primitive from a repeated subtrace;
- compose two learned procedures into a larger procedure;
- abstract over variable-length substructures or procedure arguments;
- search over typed alternatives with a principled prior/cost;
- replay imagined or stored tasks to test whether an abstraction transfers;
- maintain library provenance showing which concrete traces justify each abstraction.

The repository's existing candidate signatures, evidence expansion links, canonical ordering, and cost vectors are useful foundations for a **library-learning experiment**. The experiment should keep primitive promotion separate from ordinary repair, as the repository's own task document already proposes. The first decision is not “how many more grammar stages?” but “what is the language of procedures, and what counts as a justified new primitive?”

#### Primary sources

- Kevin Ellis et al., “DreamCoder: Growing generalizable, interpretable knowledge with wake-sleep Bayesian program learning,” arXiv:2006.08381 (2020), [arXiv record](https://arxiv.org/abs/2006.08381), DOI [10.48550/arXiv.2006.08381](https://doi.org/10.48550/arXiv.2006.08381). [P1]
- Brenden M. Lake, Ruslan Salakhutdinov, and Joshua B. Tenenbaum, “Human-level concept learning through probabilistic program induction,” *Science* 350(6266), 1332–1338 (2015), DOI [10.1126/science.aab3050](https://doi.org/10.1126/science.aab3050). Publisher metadata was verified through Crossref. [P2]
- Miroslav Kubat, “Conceptual inductive learning,” *Artificial Intelligence* 52(2), 169–182 (1991), DOI [10.1016/0004-3702(91)90041-H](https://doi.org/10.1016/0004-3702(91)90041-H). Crossref metadata verifies the title, venue, and pagination. [P3]

### 3.3 Reinforcement learning and model-free/model-based RL

#### Core mechanism

Model-free reinforcement learning learns a policy or value estimate from action, observation, and reward sequences without necessarily representing transition dynamics. Q-learning is the canonical example: it updates action values toward a one-step reward plus the best estimated continuation value, allowing learning from delayed consequences without a known transition model. [RL1]

Model-based RL adds an estimated action-transition model and uses it for planning, policy improvement, or imagined experience. Sutton's Dyna architecture explicitly integrates learning, planning, and reactive execution: experience updates a world model; planning uses the model; planning results can be compiled into behavior. The paper also explicitly allows learned models to be probabilistic and sometimes wrong, which is a useful warning for an evidence-traceable system. [RL2]

#### Capability achieved

RL adds sequential decision making, delayed credit assignment, exploration/exploitation tradeoffs, reward-directed behavior, and policies that can choose among actions based on expected future consequences. Model-based RL adds counterfactual rollouts and a way to reuse experience without physically executing every imagined trajectory.

#### Limitation

RL requires a task objective: reward, utility, a goal test, or some equivalent preference signal. The current repository has terminal statuses and evaluator checks, but it does not learn a general reward function or define a reusable objective over arbitrary environments. Model-free value estimates can be opaque and can require large experience; model-based methods inherit model bias and compounding prediction error. Neither standard Q-learning nor Dyna solves raw perceptual representation or long-term memory automatically.

#### Relevance to this repository

The current learner is closer to **evidence-backed structural prediction with action selection** than to RL. It chooses an action in v2, but the action is grounded by a fixed two-step evaluator and candidate applicability—not by a learned value function over long trajectories. To evolve toward RL, the framework would need explicit interfaces for:

1. reward/goal signals and their provenance;
2. trajectory identity and temporal credit assignment;
3. policy/value hypotheses separate from transition-rule hypotheses;
4. exploration decisions that account for information gain, task utility, and risk;
5. replay or imagination that never masquerades as observed evidence.

The existing abstention and evidence policy is valuable here: an imagined transition should be tagged as a model consequence, while only an environment return can become observed evidence.

#### Primary sources

- Christopher J. C. H. Watkins and Peter Dayan, “Q-learning,” *Machine Learning* 8, 279–292 (1992), DOI [10.1007/BF00992698](https://doi.org/10.1007/BF00992698). Crossref metadata verifies the paper and abstract-level record. [RL1]
- Richard S. Sutton, “Dyna, an integrated architecture for learning, planning, and reacting,” *ACM SIGART Bulletin* 2(4), 160–163 (1991), DOI [10.1145/122344.122377](https://doi.org/10.1145/122344.122377). Crossref metadata includes the original abstract describing model learning, planning, and reactive execution. [RL2]

### 3.4 Learned world models

#### Core mechanism

A learned world model compresses observation histories into a latent state and predicts future observations, rewards, values, or other planning-relevant quantities under actions. Ha and Schmidhuber's **World Models** learns spatial and temporal representations and trains a policy partly inside model-generated “dreams.” [WM1] PlaNet learns a latent dynamics model from images and performs online planning in latent space; it includes deterministic and stochastic transition components and a multi-step objective because planning requires useful predictions beyond the next observation. [WM2]

MuZero goes further by learning only the aspects of dynamics needed for planning—reward, policy, and value—rather than reconstructing the full environment. It combines a learned model with tree search and achieves strong results in Atari and board games without being supplied the game rules. [WM3]

#### Capability achieved

World models add compact temporal state, counterfactual action consequences, multi-step prediction, model-based exploration, and planning without a hand-authored transition equation. They can operate under partial observability and can trade full sensory reconstruction for task-relevant latent predictions.

#### Limitation

The model's latent state is not automatically interpretable or identifiable. Approximate errors compound across rollouts; stochasticity and partial observability require uncertainty or belief-state handling; a model trained for one objective can omit variables required by a later objective. MuZero's success also depends on substantial function approximation, tree search, and task-specific training machinery. A world model is not the same thing as a true causal model: predictive adequacy for a policy can be narrower than faithful state semantics.

#### Relevance to this repository

This is the main missing layer between “a reusable structural rule” and “a learner that can plan.” The current procedure predicts one aligned output packet. A world-model extension would need to represent:

- latent or symbolic state distinct from the raw packet;
- action-conditioned transition hypotheses;
- multi-step rollout traces;
- uncertainty and competing model predictions;
- observed versus imagined evidence;
- model revision when a rollout is contradicted;
- a planning objective over predicted future states.

The current evidence/procedure stores can host these records if their provenance distinguishes observation, hypothesis, simulation, and verified consequence. The current fixed three-atom packet should remain useful as a deterministic unit test, but it cannot validate the world-model capability by itself.

#### Primary sources

- David Ha and Jürgen Schmidhuber, “World Models,” arXiv:1803.10122 (2018), [arXiv record](https://arxiv.org/abs/1803.10122), project link [worldmodels.github.io](https://worldmodels.github.io/). [WM1]
- Danijar Hafner et al., “Learning Latent Dynamics for Planning from Pixels,” arXiv:1811.04551 (2019), [arXiv record](https://arxiv.org/abs/1811.04551). [WM2]
- Julian Schrittwieser et al., “Mastering Atari, Go, Chess and Shogi by Planning with a Learned Model,” arXiv:1911.08265 (2020), [arXiv record](https://arxiv.org/abs/1911.08265), related DOI [10.1038/s41586-020-03051-4](https://doi.org/10.1038/s41586-020-03051-4). [WM3]
- Danijar Hafner et al., “Mastering Diverse Domains through World Models,” arXiv:2301.04104 (DreamerV3, 2023), [arXiv record](https://arxiv.org/abs/2301.04104), first-party project link [danijar.com/dreamerv3](https://danijar.com/dreamerv3). [WM4]

### 3.5 Continual learning and durable memory

#### Core mechanism

Continual learning studies sequential tasks where the learner must acquire new knowledge without destroying prior competence. Elastic Weight Consolidation (EWC) slows learning on parameters judged important to prior tasks, preserving old skills while allowing other parameters to change. [CL1] Gradient Episodic Memory (GEM) keeps episodic examples and constrains updates so new learning does not increase loss on previous tasks; it also defines metrics for transfer and forgetting. [CL2]

The symbolic analogue is not a direct weight penalty. It is a versioned knowledge store with scopes, dependencies, immutable evidence, revalidation, selective retention, and a policy for contradiction. The repository is already closer to this form than to ordinary neural continual learning.

#### Capability achieved

These approaches provide mechanisms for sequential task retention, controlled plasticity, backward-transfer measurement, replay, and explicit forgetting/retention tradeoffs. They make clear that “learns a second rule without losing the first” is only the smallest continual-learning test; long sequences of tasks, interference, storage limits, and transfer require additional measurements.

#### Limitation

EWC and GEM are primarily parameter/update mechanisms. They do not provide semantic provenance, durable cross-process storage, explicit contradiction explanations, or a general policy for deciding when an old abstraction is stale. Replay-based methods also face finite memory and sample selection problems. Symbolic versioning avoids some destructive overwrite patterns, but it introduces dependency indexing, revalidation cost, schema migration, and retention-policy questions.

#### Relevance to this repository

The current v1/v2 retention result is meaningful but narrow: the learner preserves one admitted procedure while learning another in the same bounded run. The next required step is the repository's own MVP-4 persistence task:

- save evidence, hypotheses, procedures, dependencies, scopes, revisions, and counters;
- restore in a fresh process without evaluator labels;
- preserve frozen/mutable state correctly;
- retain original references and state fingerprints;
- distinguish stale, contradicted, suspended, revised, and still-supported procedures.

That persistence work is not a convenience feature. It is the minimum experiment that turns in-memory continual retention into durable continual learning. After persistence, a longer continual protocol should vary task order, unrelated-rule interference, changed dynamics, repeated contradictions, finite archive capacity, and cross-environment transfer.

#### Primary sources

- James Kirkpatrick et al., “Overcoming catastrophic forgetting in neural networks,” arXiv:1612.00796 (2017), [arXiv record](https://arxiv.org/abs/1612.00796), related DOI [10.1073/pnas.1611835114](https://doi.org/10.1073/pnas.1611835114). [CL1]
- David Lopez-Paz and Marc'Aurelio Ranzato, “Gradient Episodic Memory for Continual Learning,” arXiv:1706.08840 (2017), [arXiv record](https://arxiv.org/abs/1706.08840). [CL2]

### 3.6 Concept and representation discovery

#### Core mechanism

Representation discovery identifies variables or features that make the environment simpler, more invariant, or more predictive. Slow Feature Analysis learns slowly varying features from temporally ordered input, using temporal stability as an unsupervised pressure for invariant structure. [CR1] Probabilistic program induction instead discovers a structured generative explanation, as in Lake et al.'s character model. [P2] The two approaches illustrate different choices: continuous invariant features versus explicit compositional programs.

#### Capability achieved

Representation-learning approaches can compress high-dimensional or sequential input, factor nuisance variation, and provide features on which a policy or classifier generalizes. Program-based concept learning can produce interpretable generative structure and strong compositional transfer from few examples.

#### Limitation

“Useful representation” is objective-dependent. Slow variation can preserve nuisance factors and discard fast task-relevant changes. Unsupervised features do not necessarily correspond to objects, actions, causes, or concepts. Program induction requires a sufficiently expressive language and can be brittle when the intended factorization is absent. Neither approach provides a universal criterion for semantic grounding from an arbitrary raw stream.

#### Relevance to this repository

The current learner has a deliberately hand-specified representation: packet boundaries, positions, exact equality, and opaque identifiers. That is excellent for isolating structural induction, but it means the hardest representation question is outside the experiment. Before accepting raw text or sensory streams, the project must decide whether it wants:

- a symbolic segmentation/binding learner;
- a learned continuous representation feeding the symbolic stores;
- a probabilistic program/grammar learner;
- or a hybrid in which candidate structural procedures propose representations and the environment supplies discriminating evidence.

The key evaluation question is not merely prediction accuracy. It is whether the representation supports transfer, composition, explanation, revision, and evidence-linked uncertainty. The current stores should therefore be designed to reference an input representation version and its provenance, even if the MVP continues to use opaque fixed packets.

#### Primary sources

- Laurenz Wiskott and Terrence J. Sejnowski, “Slow Feature Analysis: Unsupervised Learning of Invariances,” *Neural Computation* 14(4), 715–770 (2002), DOI [10.1162/089976602317318938](https://doi.org/10.1162/089976602317318938). [CR1]
- Brenden M. Lake et al., “Human-level concept learning through probabilistic program induction,” DOI [10.1126/science.aab3050](https://doi.org/10.1126/science.aab3050). [P2]

### 3.7 Planning and goal-directed reasoning

#### Core mechanism

Classical planning represents a state, actions/operators with preconditions and effects, an initial state, and a goal. STRIPS is a foundational example: theorem-proving machinery searches over symbolic action descriptions to construct plans and then supports monitored execution. [PL1] PDDL2.1 formalizes richer planning domains with temporal and numeric constructs, but retains an explicit domain/problem representation and a planner that searches within it. [PL2]

Modern model-based RL and world-model systems add learned dynamics and learned value/reward components to the same broad loop: represent a state, predict consequences, search or imagine future action sequences, and execute a selected action.

#### Capability achieved

Planning provides multi-step goal achievement, precondition reasoning, subgoals, temporal constraints, resource-aware action sequences, and replanning when observations differ from predictions. It is the established mechanism for the capability the repository explicitly excludes as “general planning or search over long action sequences.”

#### Limitation

Classical planning normally assumes a domain model and an explicit goal. Expressive planning languages increase modeling power and can increase computational complexity. Learned planners depend on model quality, objective specification, search budget, and uncertainty handling. A rule that predicts a one-step packet effect is not yet an operator with a stable state transition, nor is a terminal success label a general goal representation.

#### Relevance to this repository

The repository needs a new **goal/state/planning layer**, not only a larger candidate grammar. A minimal planning extension would require:

1. a state abstraction and state identity across time;
2. action schemas with preconditions and effects;
3. explicit goals or constraints, with scope and provenance;
4. hypothetical branches isolated from observed ledger records;
5. bounded search and a declared cost/risk policy;
6. plan validation against the environment and revision when the model fails;
7. partial observability and stochastic outcome handling.

The current E3 applicability guard is a useful seed for preconditions, and E4 promotion is a seed for reusable operators. Neither is sufficient for long-horizon planning because there is no composition, goal condition, branching rollout, or state transition graph.

#### Primary sources

- Richard E. Fikes and Nils J. Nilsson, “STRIPS: A new approach to the application of theorem proving to problem solving,” *Artificial Intelligence* 2(3–4), 189–208 (1971), DOI [10.1016/0004-3702(71)90010-5](https://doi.org/10.1016/0004-3702(71)90010-5). Crossref metadata verifies the original paper. [PL1]
- Maria Fox and Derek Long, “PDDL2.1: An Extension to PDDL for Expressing Temporal Planning Domains,” *Journal of Artificial Intelligence Research* 20, 61–124 (2003), DOI [10.1613/jair.1129](https://doi.org/10.1613/jair.1129). The DOI citation and title were verified; direct publisher access was limited in this session. [PL2]

### 3.8 Neuro-symbolic and hybrid systems

#### Core mechanism

Hybrid systems place learned perception, representation, or probabilistic predicates beside symbolic programs, logic, constraints, or interpretable state. DeepProbLog integrates neural predicates with probabilistic logic programming and supports program induction, probabilistic inference, and learning from examples. [NS1] The Neuro-Symbolic Concept Learner (NS-CL) learns visual concepts and language-related program parsing from paired images and questions/answers, then executes symbolic programs over an object-based scene representation. [NS2]

These systems are relevant because they do not require choosing between “purely symbolic” and “fully neural” as a single global architecture. They define a seam: learned components propose or ground symbols; symbolic components perform compositional execution, constraints, and reasoning.

#### Capability achieved

DeepProbLog demonstrates end-to-end learning with symbolic probabilistic inference. NS-CL demonstrates naturally supervised concept and language learning, object-centric representations, executable programs, compositional generalization, and transfer to new compositions/scenes/questions within its task family.

#### Limitation

The seam is not free. Neural predicates and scene parsers require training data, inductive biases, uncertainty calibration, and a defined interface between continuous outputs and discrete reasoning. The cited systems operate in structured domains with particular supervision (logic examples or paired image-question-answer data), not arbitrary persistent raw experience. Symbolic execution can be exact while the learned grounding remains uncertain or wrong; a failure policy is needed.

#### Relevance to this repository

The repository's opaque protocol and evidence ledger are strong candidates for the symbolic side of a hybrid design. A future input adapter could propose:

- token/segment hypotheses;
- object or entity bindings;
- relation probabilities;
- candidate semantic parses;
- learned embeddings or latent states.

The existing learner should not silently treat those proposals as facts. They should enter as uncertain hypotheses with source/model/version references, be challenged by structural or environmental evidence, and remain distinguishable from exact symbolic observations. DeepProbLog and NS-CL suggest that a hybrid can add representation and concept discovery while retaining an executable symbolic core; they do not establish that the present fixed grammar is sufficient for the interface.

#### Primary sources

- Robin Manhaeve et al., “DeepProbLog: Neural Probabilistic Logic Programming,” arXiv:1805.10872 (2018), [arXiv record](https://arxiv.org/abs/1805.10872). [NS1]
- Jiayuan Mao et al., “The Neuro-Symbolic Concept Learner: Interpreting Scenes, Words, and Sentences From Natural Supervision,” arXiv:1904.12584 (2019), [arXiv record](https://arxiv.org/abs/1904.12584), first-party project link [nscl.csail.mit.edu](http://nscl.csail.mit.edu/). [NS2]

## 4. Gap analysis against v1/v2

### 4.1 Implemented and validated

The following claims are supported by code, tests, specifications, and recorded artifacts rather than inferred from the broader research draft:

1. **Opaque fixed-packet transport.** The learner-facing v2 protocol enforces exactly three ordered atoms, exact opaque identifiers, two available actions, explicit scopes, and decision-before-result ordering.
2. **Bounded structural induction.** v1 G1 and v2 E1/E2 can abstract repeated equality structure over multiple supports and copy/replay stable output positions/atoms.
3. **Applicability control.** v1 development gates and v2 E3 challenge the action/precondition boundary and support explicit abstention on non-applicable cases.
4. **Evidence and procedure traceability.** Candidate support, challenge, grounding, rejection, cost, procedure, hypothesis, and revision references are represented in learner-owned traces. The v2 package records JSONL traces and resource reports.
5. **Scope and freeze boundaries.** Episode/run scopes constrain reuse; final evaluation is frozen; final results cannot mutate final decision behavior.
6. **Deterministic reproducibility.** Canonical candidate order, fixed schedules, seeded mappings, three-rerun comparisons, artifact hashes, and explicit budgets are implemented and tested.
7. **Narrow continual retention.** v1 retains R1 while learning R2. v2 includes contradiction/versioning seams and tests for stale/conditional/incomplete states, while the changed-rule primary-world extension remains evaluator-side rather than a full durable learner capability.
8. **Partial-evidence honesty at the seam.** Unavailable slots are represented separately from atom values and do not become false equality or unchanged-position evidence.

These are substantial foundations for an auditable symbolic/hybrid runtime. They do not imply the capabilities in the next section.

### 4.2 Theoretically feasible but untested

The repository documents a plausible route for the following, but they are not yet validated as reusable framework capabilities:

| Candidate extension | Why the current design can support it | What must be added and measured |
|---|---|---|
| Restartable persistence | Evidence/procedure records already have identifiers, scopes, versions, and references | Canonical versioned snapshot/event format, strict load validation, atomic writes, fresh-process restore, frozen-state preservation, state fingerprints. |
| Environment-independent runtime | v2 has learner/evaluator traits and a clear hidden-world boundary | Separate reusable runtime from evaluator schedule/world; run the same controller against two materially different environments. |
| More general relational grammar | E1/E2 already record equality, changed/unchanged positions, and output operations | Multiple variables, relation predicates, typed arguments, conjunction/disjunction, negative evidence, compositional signatures, and a tractable search policy. |
| Primitive/library discovery | Candidates and expansion references are explicit | A separate library-learning phase, abstraction utility metric, replay/transfer tasks, library versioning, and safeguards against invented or over-general primitives. |
| Variable-length structure | Fixed positions and sequence equality are already explicit | Boundary/segmentation hypotheses, alignment, edit operations, nested structure, delimiter uncertainty, and tests for false merges/splits. |
| State-transition model | Interaction records contain before/action/after/terminal | Persistent state abstraction, transition hypotheses, multi-step traces, counterfactual/imagined evidence status, and model-revision logic. |
| Goal-directed planning | E3 resembles an action precondition and E4 promotes a reusable procedure | Explicit goals, operator effects, plan search, hypothetical scopes, branch pruning, risk/cost, execution monitoring, and replanning. |
| Stochastic environments | Evidence and hypothesis references can be extended | Outcome distributions, confidence/calibration, belief states, repeated trials, risk-sensitive action selection, and probabilistic contradiction semantics. |
| Raw text entry | The conceptual draft acknowledges preserving characters/boundaries/order | Segmentation, token/phrase induction, syntax/semantic hypotheses, grounding, provenance, ambiguity handling, and a new text-specific evaluation protocol. |
| Hybrid learned front end | Learner/evaluator seam can isolate proposals from exact observations | Representation API, model/version provenance, uncertainty, learned-to-symbolic conversion, calibration, and tests that prevent soft outputs becoming unearned facts. |

### 4.3 Open research questions

1. **Identifiability:** With opaque observations and sparse interventions, when can two competing structural or causal hypotheses be distinguished at all? What is the minimum discriminating evidence, and how should the learner request it?
2. **Representation selection:** Which invariances should be learned from raw streams, and which should remain exact symbolic relations? How can the system avoid discarding a fast but task-relevant change while compressing nuisance variation?
3. **Primitive invention:** What evidence warrants promoting a repeated subprocedure into a reusable primitive? How should utility, compression, transfer, and interpretability trade off against search and verification cost?
4. **Compositional search:** How can procedures compose without making candidate generation combinatorial or allowing an expressive grammar to explain every example? What type system, cost model, or Bayesian prior is appropriate?
5. **Model uncertainty:** How should a learned transition model represent epistemic uncertainty, stochasticity, partial observability, and out-of-distribution states? What does it mean for a model to be “supported” when it predicts a distribution rather than one packet?
6. **Planning objective:** Where do goals, preferences, rewards, safety constraints, and action costs come from? Can the framework remain evidence-grounded when the objective is supplied by a task rather than discovered from transitions?
7. **Hypothetical evidence:** How should imagined rollouts affect candidate ranking without being confused with observed evidence? How much model-based evidence is safe to use for planning and how should it be discounted after model error?
8. **Continual storage economics:** What is retained forever, summarized, compressed, or deleted? How are dependencies revalidated after a changed rule, and how does the system report uncertainty when revalidation is incomplete?
9. **Cross-environment transfer:** Which parts of a procedure are environment-independent, and which depend on a wire mapping, scope, representation version, or task objective? The current permutation extension tests only a narrow form of symbol/action remapping.
10. **Hybrid boundary:** Should the learned front end produce discrete hypotheses, continuous latent states, executable programs, or all three? What evidence policy makes the interface auditable without preventing useful generalization?

## 5. Architecture implications

### 5.1 What can be extended without invalidating the core

The following principles should survive future versions:

- **Observed, inferred, imagined, and evaluator-only data remain different classes.** A model rollout cannot become an observation merely because it was internally generated.
- **Procedures remain versioned and evidence-linked.** A new version narrows or revises an old procedure; it does not silently rewrite historical support.
- **Abstention remains first-class.** Unsupported grammar, insufficient evidence, conflicting hypotheses, and budget exhaustion should not be converted into confident default outputs.
- **Search remains bounded and auditable.** Every larger grammar or planner needs candidate counts, operation costs, storage costs, and explicit search-exhaustion outcomes.
- **Environment-independent runtime remains separate from benchmark/evaluator code.** Hidden labels, seeds, transition tables, and scoring must remain outside learner state.
- **Every broad claim gets a new protocol.** A variable-length, stochastic, planning, text, or hybrid result cannot be reported as a continuation of the fixed v1/v2 claim.

### 5.2 What cannot be obtained by simply widening E0–E4

Some changes are qualitatively different:

- **Variable length** needs a representation and alignment problem, not more constants in a three-slot array.
- **Primitive discovery** needs a library/abstraction mechanism, not another fixed candidate stage that knows the desired primitive form.
- **Long-horizon planning** needs states, goals, hypothetical rollouts, and search, not repeated one-step packet predictions.
- **Stochasticity** needs distributions/beliefs and calibrated uncertainty, not a larger list of deterministic evidence references.
- **Raw text** needs segmentation, parsing, grounding, and ambiguity management, not a string field attached to `ObservationPacket`.
- **General continual learning** needs durable storage and a retention/revalidation policy, not only same-run procedure retention.
- **Concept discovery** needs an objective and representation-learning bias, not only opaque equality patterns.

These capabilities can reuse the repository's stores and audit rules, but each requires a new semantic layer and new failure modes.

### 5.3 A likely layered architecture

A plausible evolution is:

```text
environment / raw stream
        |
        v
representation and segmentation layer
        |
        v
typed observations + uncertain hypotheses
        |
        v
evidence / dependency / procedure stores
        |
        +--> structural and primitive/library induction
        |
        +--> learned or symbolic transition model
        |          |
        |          v
        |      bounded planner / goal evaluator
        |
        v
decision, action, abstention, and revision policy
        |
        v
observed environment result -> append-only evidence -> revalidation
```

The current v1/v2 kernel occupies the middle and lower portions of this diagram. It does not yet occupy the representation, world-model, or planner layers. A hybrid implementation could add those layers while keeping symbolic audit and evidence semantics central.

## 6. Recommended architecture-decision experiments

These should be small, isolated experiments before committing to a broad post-MVP architecture. Each should have a new frozen protocol, a baseline without the proposed capability, and explicit negative results.

### Experiment A — Variable-length structure and boundary discovery

**Question:** Can the learner discover reusable structural relations when packet boundaries and sequence length are not supplied?

**Design:** Generate deterministic streams containing repeated motifs, distractors, optional segments, and held-out lengths. Compare:

1. fixed-position/equality baseline;
2. explicit symbolic segmentation and alignment search;
3. a learned representation or neural parser feeding symbolic candidate induction;
4. a library-learning variant that promotes recurring motifs.

**Measurements:** boundary precision/recall, false merge/split rate, structural transfer by length, candidate/search cost, evidence traceability, abstention under ambiguous segmentation, and performance after irrelevant motifs are introduced.

**Decision:** If symbolic segmentation remains tractable and transfers, deepen the grammar. If it fails on search or ambiguity, decide whether a learned front end is necessary before expanding the procedure language.

### Experiment B — Transition model and horizon extension

**Question:** What is the smallest model representation that supports reliable planning beyond two steps?

**Design:** Use a deterministic toy environment first, then add controlled stochastic outcomes. Require horizons 4, 8, and 16 with explicit goals. Compare:

1. reactive one-step procedure reuse;
2. symbolic transition hypotheses plus bounded breadth-first/A* search;
3. a learned tabular/probabilistic model plus model-predictive search;
4. a latent world-model variant only if raw observations are part of the later decision.

**Measurements:** goal success by horizon, model prediction error by rollout depth, calibration/coverage of uncertainty, plan cost, replanning frequency, counterfactual-versus-observed provenance, sample efficiency, and failure classification when the model is wrong.

**Decision:** If symbolic state is sufficient, keep the model interpretable and defer neural latents. If state aliasing or observation complexity dominates, define the hybrid representation seam before adding planning.

### Experiment C — Primitive and procedure-library discovery

**Question:** Can repeated learned procedures be compressed into reusable primitives that improve later tasks without over-generalizing?

**Design:** Create task families in which a two- or three-step subprocedure recurs under different contexts, plus adversarial tasks where superficial repetition should not become a primitive. Compare no library, hand-fixed library, and learned library promotion.

**Measurements:** library growth, compression/description length, solve time, transfer to held-out compositions, false abstraction rate, evidence expansion size, revision cost, and whether the library improves or harms search.

**Decision:** This determines whether the future system is best described as a bounded rule learner, a program-synthesis/library learner, or a hybrid of both.

### Experiment D — Persistence and lifelong revision (MVP acceptance)

**Question:** Does the learner remain evidence-consistent after process restart, environment change, and contradiction?

**Design:** Complete the already specified save/load MVP with two materially different deterministic environments. Train in process A, restore in process B, use a held-out procedure, then introduce a changed rule and inspect stale/revised/suspended dependents.

**Measurements:** fresh-process transfer, state-fingerprint equality, evidence-reference preservation, no evaluator-label leakage, load validation, revalidation work, archive size, and frozen/mutable mode correctness.

**Decision:** This is a prerequisite for calling the implementation a reusable framework. It should be completed before adding neural or planning complexity.

## 7. Recommended implementation and research order

1. **Complete the current MVP in scope.** Extract the reusable runtime, make evidence/hypothesis/procedure stores first-class, add versioned persistence, run the generic controller against two different deterministic environments, and preserve all v1/v2 artifacts.
2. **Do not fold broad capabilities into MVP.** Keep fixed three-atom packets, deterministic environments, bounded grammar, no raw text, no long-horizon planning, and no stochasticity as explicit boundary conditions.
3. **Run Experiment D as the MVP gate.** A learner that cannot survive save/load and contradiction/revision is not yet a durable framework for continual learning.
4. **Run Experiments A–C before choosing the next major architecture.** They answer representation, model/planning, and abstraction/library questions with less confounding than a single large feature build.
5. **Choose the next version based on the result.**
   - If A passes symbolically and C passes with small libraries, extend the symbolic/program-synthesis core.
   - If A fails on raw representation but B works on structured state, add a learned front end while preserving symbolic stores.
   - If B requires uncertainty and long-horizon model learning, add a model-based RL/world-model layer with an explicit imagined-evidence type.
   - If C shows strong transfer, make library learning a first-class subsystem before adding broad text.
6. **Treat raw text as a separate grounding program.** NS-CL and related hybrid work suggest a route, but the repository should not claim text learning until segmentation, parsing, grounding, ambiguity, and provenance have their own protocol.

## 8. Source-access and verification notes

- The primary-source pages used here were checked through the web fetcher on 2026-10-11. ArXiv records were available for DreamCoder, World Models, PlaNet, MuZero, DreamerV3, EWC, GEM, DeepProbLog, NS-CL, and Lake et al.'s broader architecture paper. The Stanford-hosted McCarthy paper and first-party NS-CL/World Models links were reachable; the PDDL2.1 DOI was verified, while direct publisher access was limited.
- Crossref metadata was available for the cited DOI records for SOAR, STRIPS, Q-learning, Lake et al., Kubat, Dyna, and PDDL-related papers. Some direct DOI requests returned rate-limit responses (`429`) or publisher access restrictions (`403`); where that occurred, the DOI, title, authors, venue, and citation identity were checked using Crossref metadata or ArXiv rather than treating a failed publisher fetch as evidence that the citation was invalid.
- The repository's `graphify` and `agent-reach` command shims were Windows-formatted in this Linux session and could not be executed directly. Existing `graphify-out/GRAPH_REPORT.md` and `graphify-out/graph.json` were read, and the local comparison was verified directly against the source, specifications, tests, and artifacts. No source code was modified.

## 9. References

### Repository sources

- [R-v1] [`artifacts/v1/README.md`](../../artifacts/v1/README.md) — recorded `validated-v1-narrow-claim`, gates, baselines, and limitations.
- [R-v2] [`artifacts/v2/README.md`](../../artifacts/v2/README.md) — recorded `validated-v2-narrow-claim`, protocol, gates, resources, and reruns.
- [`testable-design-v1.md`](../../testable-design-v1.md) — fixed three-atom v1 grammar and evaluation boundary.
- [`docs/specs/opaque-interactive-structural-discovery-v2.md`](../specs/opaque-interactive-structural-discovery-v2.md) — v2 learner/evaluator boundary and E0–E4 grammar.
- [`docs/protocols/opaque-interactive-structural-discovery-v2-protocol-v1.0.md`](../protocols/opaque-interactive-structural-discovery-v2-protocol-v1.0.md) — frozen deterministic protocol.
- [`docs/task.md`](../task.md) — specified restartable framework MVP and explicit exclusions.
- [`src/framework/opaque_protocol.rs`](../../src/framework/opaque_protocol.rs), [`src/framework/grammar_admission.rs`](../../src/framework/grammar_admission.rs), [`src/framework/interaction_ledger.rs`](../../src/framework/interaction_ledger.rs), [`src/framework/grounding_decision.rs`](../../src/framework/grounding_decision.rs) — implemented protocol, bounded grammar, ledger, and decision seams.

### Primary and first-party external sources

- [S1] McCarthy, J. (1959). “Programs with Common Sense.” First-party Stanford-hosted paper: <https://www-formal.stanford.edu/jmc/mcc59/mcc59.html>.
- [S2] Laird, J. E., Newell, A., & Rosenbloom, P. S. (1987). “SOAR: An architecture for general intelligence.” *Artificial Intelligence*, 33(1), 1–64. DOI: <https://doi.org/10.1016/0004-3702(87)90050-6>.
- [P1] Ellis, K., Wong, C., Nye, M., et al. (2020). “DreamCoder: Growing generalizable, interpretable knowledge with wake-sleep Bayesian program learning.” arXiv:2006.08381. <https://arxiv.org/abs/2006.08381>.
- [P2] Lake, B. M., Salakhutdinov, R., & Tenenbaum, J. B. (2015). “Human-level concept learning through probabilistic program induction.” *Science*, 350(6266), 1332–1338. DOI: <https://doi.org/10.1126/science.aab3050>.
- [P3] Kubat, M. (1991). “Conceptual inductive learning.” *Artificial Intelligence*, 52(2), 169–182. DOI: <https://doi.org/10.1016/0004-3702(91)90041-H>.
- [RL1] Watkins, C. J. C. H., & Dayan, P. (1992). “Q-learning.” *Machine Learning*, 8, 279–292. DOI: <https://doi.org/10.1007/BF00992698>.
- [RL2] Sutton, R. S. (1991). “Dyna, an integrated architecture for learning, planning, and reacting.” *ACM SIGART Bulletin*, 2(4), 160–163. DOI: <https://doi.org/10.1145/122344.122377>.
- [WM1] Ha, D., & Schmidhuber, J. (2018). “World Models.” arXiv:1803.10122. <https://arxiv.org/abs/1803.10122>; project: <https://worldmodels.github.io/>.
- [WM2] Hafner, D., Lillicrap, T., Fischer, I., et al. (2019). “Learning Latent Dynamics for Planning from Pixels.” arXiv:1811.04551. <https://arxiv.org/abs/1811.04551>.
- [WM3] Schrittwieser, J., Antonoglou, I., Hubert, T., et al. (2020). “Mastering Atari, Go, Chess and Shogi by Planning with a Learned Model.” arXiv:1911.08265. <https://arxiv.org/abs/1911.08265>; related DOI: <https://doi.org/10.1038/s41586-020-03051-4>.
- [WM4] Hafner, D., Pasukonis, J., Ba, J., & Lillicrap, T. (2023). “Mastering Diverse Domains through World Models.” arXiv:2301.04104. <https://arxiv.org/abs/2301.04104>; project: <https://danijar.com/dreamerv3>.
- [CL1] Kirkpatrick, J., Pascanu, R., Rabinowitz, N., et al. (2017). “Overcoming catastrophic forgetting in neural networks.” arXiv:1612.00796. <https://arxiv.org/abs/1612.00796>; related DOI: <https://doi.org/10.1073/pnas.1611835114>.
- [CL2] Lopez-Paz, D., & Ranzato, M. (2017). “Gradient Episodic Memory for Continual Learning.” arXiv:1706.08840. <https://arxiv.org/abs/1706.08840>.
- [CR1] Wiskott, L., & Sejnowski, T. J. (2002). “Slow Feature Analysis: Unsupervised Learning of Invariances.” *Neural Computation*, 14(4), 715–770. DOI: <https://doi.org/10.1162/089976602317318938>.
- [PL1] Fikes, R. E., & Nilsson, N. J. (1971). “STRIPS: A new approach to the application of theorem proving to problem solving.” *Artificial Intelligence*, 2(3–4), 189–208. DOI: <https://doi.org/10.1016/0004-3702(71)90010-5>.
- [PL2] Fox, M., & Long, D. (2003). “PDDL2.1: An Extension to PDDL for Expressing Temporal Planning Domains.” *Journal of Artificial Intelligence Research*, 20, 61–124. DOI: <https://doi.org/10.1613/jair.1129>.
- [NS1] Manhaeve, R., Dumančić, S., Kimmig, A., Demeester, T., & De Raedt, L. (2018). “DeepProbLog: Neural Probabilistic Logic Programming.” arXiv:1805.10872. <https://arxiv.org/abs/1805.10872>.
- [NS2] Mao, J., Gan, C., Kohli, P., Tenenbaum, J. B., & Wu, J. (2019). “The Neuro-Symbolic Concept Learner: Interpreting Scenes, Words, and Sentences From Natural Supervision.” arXiv:1904.12584. <https://arxiv.org/abs/1904.12584>; project: <http://nscl.csail.mit.edu/>.
