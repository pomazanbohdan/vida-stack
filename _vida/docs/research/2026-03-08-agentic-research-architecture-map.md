# Agentic Research Architecture Map

**Purpose:** Provide one map of all research-derived documentation layers created for VIDA autonomous orchestration work so future protocol, config, plan, and test changes can reuse the same architecture instead of rediscovering it from chat history.

**Primary inputs:**
- `_vida/docs/plans/2026-03-08-vida-autonomous-role-profiles-and-adaptive-orchestration-plan.md`
- `_vida/docs/research/2026-03-08-agentic-parameter-registry.md`

**Primary outputs:**
- a stable documentation graph
- promotion rules from doc-layer to protocol/config/test-layer
- traceability between research, decisions, patterns, proofs, security, and rollout

---

## Layer Graph

| Layer | File | Purpose | Promotes into |
|---|---|---|---|
| Source registry | `_vida/docs/research/2026-03-08-agentic-role-profile-source-registry.md` | current external sources and trust basis | refresh protocol, delta log |
| Source delta log | `_vida/docs/research/2026-03-08-agentic-role-profile-source-delta-log.md` | material changes in external guidance | spec deltas, task refresh gates |
| Parameter registry | `_vida/docs/research/2026-03-08-agentic-parameter-registry.md` | reusable enumerations and value families | templates, protocols, config, tests |
| Agent definition system | `_vida/docs/research/2026-03-08-agentic-agent-definition-system.md` | canonical terminology and relation model for agent definition, instruction contract, and prompt template configuration | agent-definition protocol, instruction schemas, templates, evals |
| Cheap worker packet system | `_vida/docs/research/2026-03-08-agentic-cheap-worker-packet-system.md` | minimum packet/prompt system for low-cost bounded worker execution | packet templates, worker prompts, readiness gates, conformance tests |
| Cheap worker prompt pack | `_vida/docs/research/2026-03-08-agentic-cheap-worker-prompt-pack.md` | concrete reusable prompts for cheap schema/test/kernel/review worker lanes | rendered worker packets, dispatch templates, prompt libraries |
| Atomic claims registry | `_vida/docs/research/2026-03-08-agentic-atomic-claims-registry.md` | fine-grained research claims with confidence and affected surfaces | consensus/disagreement layers, decisions, protocols |
| Source consensus matrix | `_vida/docs/research/2026-03-08-agentic-source-consensus-matrix.md` | strongest cross-source agreements | invariants, heuristics, protocols |
| Source disagreement matrix | `_vida/docs/research/2026-03-08-agentic-source-disagreement-matrix.md` | unresolved or conditional differences between source families | local decision rules, experiments, thresholds |
| Decision ledger | `_vida/docs/research/2026-03-08-agentic-decision-ledger.md` | explicit architecture decisions derived from research | protocols, plans, config |
| Research implication map | `_vida/docs/research/2026-03-08-agentic-research-implication-map.md` | what research means for routing, profiles, proof, security, and rollout | plans, protocols, promotion backlog |
| Claim-to-artifact trace map | `_vida/docs/research/2026-03-08-agentic-claim-to-artifact-trace-map.md` | which claims are already captured in which artifacts | coverage review, compaction resilience |
| Threshold hypotheses registry | `_vida/docs/research/2026-03-08-agentic-threshold-hypotheses-registry.md` | candidate thresholds not yet promoted to runtime law | pilots, evals, route tuning |
| Research invariants | `_vida/docs/research/2026-03-08-agentic-research-invariants.md` | stable cross-source truths safe to treat as design laws | protocols, reviews, templates |
| Heuristic library | `_vida/docs/research/2026-03-08-agentic-heuristic-library.md` | conditional design heuristics distilled from research | routing, review, task packets |
| Known unknowns ledger | `_vida/docs/research/2026-03-08-agentic-known-unknowns-ledger.md` | unresolved questions and threshold gaps | experiments, pilots, future tasks |
| Pattern chooser | `_vida/docs/research/2026-03-08-agentic-pattern-chooser-matrix.md` | route selection by task shape | routing protocol, task scoring |
| Anti-pattern catalog | `_vida/docs/research/2026-03-08-agentic-anti-pattern-catalog.md` | failure and drift patterns to avoid | verification, review checklists |
| Proof registry | `_vida/docs/research/2026-03-08-agentic-proof-obligation-registry.md` | required proof bundles by task family | verification burden matrix, health checks |
| Threat/control matrix | `_vida/docs/research/2026-03-08-agentic-threat-model-control-matrix.md` | risks, attacks, and controls | OWASP/security protocol, approval gates |
| Escalation matrix | `_vida/docs/research/2026-03-08-agentic-escalation-policy-matrix.md` | when work must reopen or escalate | routing law, approval policy |
| Metric glossary | `_vida/docs/research/2026-03-08-agentic-metric-glossary.md` | metrics and formulas | eval protocol, operator surfaces |
| Terminology glossary | `_vida/docs/research/2026-03-08-agentic-terminology-glossary.md` | normalized vocabulary | all docs, config, tests |
| Invalidation watchlist | `_vida/docs/research/2026-03-08-agentic-invalidation-watchlist.md` | external changes that force document review | refresh protocol, delta log |
| Task archetype library | `_vida/docs/research/2026-03-08-agentic-task-archetype-library.md` | reusable child-task shapes | epic slicing, task packets |
| Source query log | `_vida/docs/research/2026-03-08-agentic-source-query-log.md` | repeatable refresh queries | source refresh work |
| Master index | `_vida/docs/research/2026-03-08-agentic-master-index.md` | compact-safe entry point and reading order | handoff, compact recovery, epic slicing |
| Epic slicing instruction | `_vida/docs/research/2026-03-08-agentic-epic-slicing-agent-instruction.md` | prompt-ready execution bridge for next-agent epic formation | epic/task formation, packet generation |
| External future bundle | `_vida/docs/research/2026-03-08-agentic-external-future-bundle.md` | preserved future-facing bundle for excluded external integration topics | future external epic/spec work |

---

## Information Flow

1. `External sources`
   - OpenAI, Anthropic, Google, Microsoft, OWASP
2. `Source layers`
   - source registry
   - source delta log
   - source query log
3. `Normalization layers`
   - parameter registry
   - agent definition system
   - terminology glossary
   - atomic claims registry
4. `Knowledge synthesis layers`
   - source consensus matrix
   - source disagreement matrix
   - research implication map
   - claim-to-artifact trace map
   - threshold hypotheses registry
   - research invariants
   - heuristic library
   - known unknowns ledger
5. `Choice layers`
   - decision ledger
   - pattern chooser matrix
   - task archetype library
6. `Safety and proof layers`
   - anti-pattern catalog
   - proof obligation registry
   - threat/control matrix
   - escalation matrix
7. `Measurement layers`
   - metric glossary
   - eval protocol and traces
8. `Execution layers`
   - plans
   - task packets
   - protocols
   - config
   - tests
9. `Navigation and handoff bridge`
   - master index
   - epic slicing instruction
10. `Future external bundle`
   - external future bundle

---

## Promotion Rules

Use this order when moving a documented concept into runtime law:

1. `Research doc`
   - captures current understanding
2. `Plan / archetype / task packet`
   - applies the idea to concrete execution
3. `Protocol`
   - turns the idea into canonical rule text
4. `Config / template`
   - makes the rule selectable or repeatable
5. `Test / health gate`
   - proves the rule is enforced

Rule:
- if a value or rule matters operationally, it should eventually leave doc-only form and become protocol/config/test-backed

---

## Minimum Use Per Future Task

Every future child task cut from the master plan should reference:

1. source registry
2. source delta log
3. parameter registry
4. agent definition system
5. atomic claims registry
6. decision ledger
7. proof obligation registry
8. escalation matrix
9. master index
10. epic slicing instruction

And use, when relevant:

1. pattern chooser matrix
2. anti-pattern catalog
3. threat/control matrix
4. task archetype library
5. invalidation watchlist
6. source consensus matrix
7. source disagreement matrix
8. research invariants
9. heuristic library
10. known unknowns ledger
11. research implication map
12. claim-to-artifact trace map
13. threshold hypotheses registry
14. external future bundle

---

## Document Health Rules

1. Keep names stable once cited by protocols or tests.
2. Prefer appending dated artifacts instead of silently overwriting history-heavy files.
3. If a document becomes the basis of runtime law, add a pointer to the stronger source.
4. If a document is superseded, keep a short deprecation note rather than deleting it silently.

---

## Immediate Consumers

- role-profile protocol work
- task classification and scoring work
- adaptive agent-count work
- consensus and escalation work
- verification burden and OWASP mapping work
- proving-wave and rollout work
- research-validation and threshold-setting work
- future external integration planning work
