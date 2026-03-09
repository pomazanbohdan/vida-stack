# Agentic Master Index

**Purpose:** Provide the fastest safe re-entry point after compact or agent handoff for the 2026-03-08 VIDA agentic research bundle.

**Use this file for:**
- compact recovery
- next-agent onboarding
- epic slicing preparation
- checking `current scope`, `future scope`, and `missing but referenced` artifacts

---

## 1. Canonical Status

### Current scope

The current implementation scope is:

1. local VIDA orchestration only
2. role profiles and agent definitions
3. task scoring and adaptive agent count
4. consensus and escalation
5. task packets and compaction-safe handoffs
6. verification burden and OWASP security spine
7. traces, evals, rollout, and proving
8. epic slicing and TaskFlow formation
9. `0.1 -> 1.0` binary transition planning for direct target-format implementation

### Future scope

The following domains are intentionally preserved but excluded from the current plan:

1. `MCP`
2. `A2A`
3. `A2UI`
4. remote identity / registry
5. gateways and external tool mediation
6. remote memory / content sharing

Canonical preservation artifact:

- `docs/framework/research/agentic-external-future-bundle.md`

### Missing but referenced

These artifacts are referenced by the current bundle but do not yet exist:

1. `docs/framework/research/agentic-role-profile-source-registry.md`
2. `docs/framework/research/agentic-role-profile-source-delta-log.md`
3. `docs/framework/research/agentic-role-profile-eval-plan.md`

Rule:

- do not silently ignore these references during epic slicing
- convert them into explicit early child tasks, blocker notes, or packet dependencies

---

## 2. Read This First After Compact

1. `AGENTS.md`
2. `vida/config/instructions/agent-definitions.orchestrator-entry.md`
3. `vida/config/instructions/instruction-contracts.thinking-protocol.md`
4. `vida.config.yaml`
5. this file

Then continue with the bundle reading order below.

---

## 3. Canonical Reading Order

### Tier A: Orientation backbone

1. `docs/framework/plans/vida-autonomous-role-profiles-and-adaptive-orchestration-plan.md`
2. `docs/framework/plans/vida-0.1-to-1.0-direct-binary-transition-plan.md`
3. `docs/framework/plans/vida-semantic-extraction-layer-map.md`
4. `docs/framework/plans/vida-direct-1.0-local-spec-program.md`
5. `docs/framework/research/agentic-research-architecture-map.md`
6. `docs/framework/research/agentic-parameter-registry.md`
7. `docs/framework/research/agentic-agent-definition-system.md`
8. `docs/framework/research/agentic-decision-ledger.md`
9. `docs/framework/research/agentic-proof-obligation-registry.md`
10. `docs/framework/research/agentic-escalation-policy-matrix.md`

### Tier B: Knowledge synthesis

1. `docs/framework/research/agentic-atomic-claims-registry.md`
2. `docs/framework/research/agentic-source-consensus-matrix.md`
3. `docs/framework/research/agentic-source-disagreement-matrix.md`
4. `docs/framework/research/agentic-research-invariants.md`
5. `docs/framework/research/agentic-heuristic-library.md`
6. `docs/framework/research/agentic-known-unknowns-ledger.md`
7. `docs/framework/research/agentic-research-implication-map.md`
8. `docs/framework/research/agentic-claim-to-artifact-trace-map.md`
9. `docs/framework/research/agentic-threshold-hypotheses-registry.md`

### Tier C: Execution, safety, and slicing support

1. `docs/framework/research/agentic-pattern-chooser-matrix.md`
2. `docs/framework/research/agentic-anti-pattern-catalog.md`
3. `docs/framework/research/agentic-threat-model-control-matrix.md`
4. `docs/framework/research/agentic-metric-glossary.md`
5. `docs/framework/research/agentic-terminology-glossary.md`
6. `docs/framework/research/agentic-invalidation-watchlist.md`
7. `docs/framework/research/agentic-task-archetype-library.md`
8. `docs/framework/research/agentic-source-query-log.md`
9. `docs/framework/research/agentic-cheap-worker-packet-system.md`
10. `docs/framework/research/agentic-cheap-worker-prompt-pack.md`
11. `docs/framework/research/agentic-epic-slicing-agent-instruction.md`

### Tier D: Future scope preservation

1. `docs/framework/research/agentic-external-future-bundle.md`

Read Tier D only when:

1. a task explicitly reopens external integrations
2. a future epic adopts those domains
3. you are checking scope boundaries to avoid accidental expansion

---

## 4. Reference Bundle For Child Tasks

Every child task packet cut from the master plan should carry a `Reference bundle` section.

### Mandatory current-scope references

1. `docs/framework/plans/vida-autonomous-role-profiles-and-adaptive-orchestration-plan.md`
2. `docs/framework/plans/vida-0.1-to-1.0-direct-binary-transition-plan.md`
3. `docs/framework/plans/vida-semantic-extraction-layer-map.md`
4. `docs/framework/plans/vida-direct-1.0-local-spec-program.md`
5. `docs/framework/research/agentic-master-index.md`
6. `docs/framework/research/agentic-research-architecture-map.md`
7. `docs/framework/research/agentic-parameter-registry.md`
8. `docs/framework/research/agentic-agent-definition-system.md`
9. `docs/framework/research/agentic-decision-ledger.md`
10. `docs/framework/research/agentic-proof-obligation-registry.md`
11. `docs/framework/research/agentic-escalation-policy-matrix.md`
12. `docs/framework/research/agentic-atomic-claims-registry.md`
13. `docs/framework/research/agentic-source-consensus-matrix.md`
14. `docs/framework/research/agentic-source-disagreement-matrix.md`
15. `docs/framework/research/agentic-research-invariants.md`
16. `docs/framework/research/agentic-heuristic-library.md`
17. `docs/framework/research/agentic-known-unknowns-ledger.md`
18. `docs/framework/research/agentic-research-implication-map.md`
19. `docs/framework/research/agentic-claim-to-artifact-trace-map.md`
20. `docs/framework/research/agentic-threshold-hypotheses-registry.md`
21. `docs/framework/research/agentic-pattern-chooser-matrix.md`
22. `docs/framework/research/agentic-anti-pattern-catalog.md`
23. `docs/framework/research/agentic-threat-model-control-matrix.md`
24. `docs/framework/research/agentic-metric-glossary.md`
25. `docs/framework/research/agentic-terminology-glossary.md`
26. `docs/framework/research/agentic-invalidation-watchlist.md`
27. `docs/framework/research/agentic-task-archetype-library.md`
28. `docs/framework/research/agentic-source-query-log.md`
29. `docs/framework/research/agentic-cheap-worker-packet-system.md`
30. `docs/framework/research/agentic-cheap-worker-prompt-pack.md`
31. `docs/framework/research/agentic-epic-slicing-agent-instruction.md`

### Optional future-scope reference

Use only when the task explicitly reopens excluded external domains:

1. `docs/framework/research/agentic-external-future-bundle.md`

### Planned-but-missing references

Carry as dependencies or blockers when relevant:

1. `docs/framework/research/agentic-role-profile-source-registry.md`
2. `docs/framework/research/agentic-role-profile-source-delta-log.md`
3. `docs/framework/research/agentic-role-profile-eval-plan.md`

---

## 5. Layer Roles

| Group | Documents | Why they matter |
|---|---|---|
| Backbone | plan, master index, architecture map | define scope, order, and navigation |
| Direct 1.0 planning | direct transition plan, semantic extraction layer map, local spec program | define what is preserved from 0.1 and how 1.0 is built directly |
| Parameters and language | parameter registry, agent definition system, terminology glossary | keep values, relation model, and vocabulary stable |
| Claims and synthesis | claims, consensus, disagreement, invariants, heuristics, known unknowns, implication map, trace map, threshold hypotheses | preserve research conclusions and uncertainty |
| Decisions and routing | decision ledger, pattern chooser, task archetypes | tell future tasks what was chosen and how to slice |
| Safety and proof | anti-pattern catalog, proof registry, threat/control matrix, escalation matrix | prevent unsafe or incomplete task packets |
| Measurement and refresh | metric glossary, invalidation watchlist, source query log | preserve change detection and evaluation surfaces |
| Future preservation | external future bundle | protect excluded knowledge from being lost |

---

## 6. Immediate Next Work

If epic formation starts now:

1. use the semantic extraction layer map to decide what to freeze from `0.1`
2. use the local direct-1.0 spec program as the canonical epic/program order
3. treat Plan Tasks `1-18` as the canonical child-task seed list for the behavioral platform layer
4. create early child tasks for the currently missing referenced artifacts
5. use the agent definition system doc as the canonical terminology and relation model
6. use the cheap-worker packet system and epic-slicing instruction as the execution bridge for future agents

## 6.1 Compact-Ready Next Actions

After compact, do not reopen architecture exploration first.

Resume in this exact order:

1. read `AGENTS.md`
2. read `vida/config/instructions/agent-definitions.orchestrator-entry.md`
3. read `vida/config/instructions/instruction-contracts.thinking-protocol.md`
4. read `vida.config.yaml`
5. read this master index
6. read `docs/framework/plans/vida-0.1-to-1.0-direct-binary-transition-plan.md`
7. read `docs/framework/plans/vida-semantic-extraction-layer-map.md`
8. read `docs/framework/plans/vida-0.2-semantic-freeze-spec.md`
9. read `docs/framework/plans/vida-0.2-bridge-policy.md`
10. read `docs/framework/plans/vida-direct-1.0-local-spec-program.md`
11. read `docs/framework/plans/vida-direct-1.0-compact-continuation-plan.md`
12. read `docs/framework/research/agentic-cheap-worker-packet-system.md`
13. read `docs/framework/research/agentic-cheap-worker-prompt-pack.md`
14. read `docs/framework/research/vida-direct-1.0-next-agent-compact-instruction.md`
15. read `docs/framework/plans/vida-0.3-command-tree-spec.md`
16. read `docs/framework/plans/vida-0.3-state-kernel-schema-spec.md`
17. read `docs/framework/plans/vida-0.3-instruction-kernel-spec.md`
18. read `docs/framework/plans/vida-0.3-migration-kernel-spec.md`
19. read `docs/framework/research/vida-parity-and-conformance-next-step-after-compact-instruction.md`

Then execute these next session slices in order:

1. begin post-spec implementation using `docs/framework/research/vida-binary-foundation-next-step-after-compact-instruction.md`

Session-slice rule:

1. `Part A` and `Part B` are bounded continuation scopes, not new product-law artifact families.

The spec spine is complete.
Cheap-agent implementation work may now begin only within bounded post-spec waves.
Do not start large Rust kernel implementation before the first Binary Foundation slice is bounded and started lawfully.

What is already sufficiently defined:

1. direct `0.1 -> 1.0` decision
2. semantic extraction layer model
3. semantic freeze spec
4. bridge policy
5. command tree spec
6. state kernel schema spec
7. instruction kernel spec
8. migration kernel spec
9. route-and-receipt spec `Part A` route-law boundary
10. route-and-receipt spec `Part B` receipt/proof boundary
11. direct local-first epic program
12. cheap-worker packet model
13. cheap-worker prompt model

What is still not yet fully extracted into final specs:

1. memory kernel contract
2. doctor/self-diagnosis runtime contract

---

## 7. Compact Bridge Reminder

If a future agent resumes after compact:

1. do not rely on transcript memory
2. use this file as the re-entry bridge
3. verify the missing artifacts section before creating or closing any slicing task
4. keep `future scope` excluded unless a new explicit task reopens it
-----
artifact_path: framework/research/agentic-master-index
artifact_type: framework_research_doc
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/research/agentic-master-index.md
created_at: 2026-03-09T20:28:59+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: agentic-master-index.changelog.jsonl
P26-03-09T21: 44:13Z
