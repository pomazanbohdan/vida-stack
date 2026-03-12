# Instruction Activation Protocol (IAP)

Purpose: define how VIDA activates instruction files and protocol surfaces during work without broad rereads, hidden heuristics, or duplicated policy bodies.

## Core Principle

Instructions must be activated by phase and trigger, not by bulk reading.

Each instruction surface must belong to exactly one of:

1. `always_on`
2. `lane_entry`
3. `triggered_domain`
4. `closure_reflection`

If a file does not have a clear activation class, the instruction layer is underspecified and should be refactored.

## Activation Stack

### 1. Bootstrap Layer

Activated always.

Canonical owner:

1. `AGENTS.md`

Allowed responsibilities:

1. identity and lane resolution,
2. a compact set of global invariants,
3. instruction precedence,
4. conflict rule,
5. pointer map to lane-entry files.

Forbidden responsibilities:

1. full orchestrator operating doctrine,
2. worker-runtime details,
3. domain-specific protocol bodies,
4. long trigger lists that belong to lane-entry or domain protocols.

Rule:

1. `AGENTS.md` should stay inspectable after compact and should not become a second full orchestrator manual.

### 2. Lane-Entry Layer

Activated immediately after bootstrap according to lane resolution.

Canonical owners:

1. `vida/config/instructions/agent-definitions/entry.orchestrator-entry.md`
2. `vida/config/instructions/agent-definitions/entry.worker-entry.md`
3. `vida/config/instructions/instruction-contracts/role.worker-thinking.md`
4. `vida/config/instructions/instruction-contracts/overlay.session-context-continuity-protocol.md` in orchestrator lane

Allowed responsibilities:

1. boot path,
2. request-intent gate,
3. tracked-flow boundary,
4. boot-profile selection,
5. next required protocol reads by trigger.

Forbidden responsibilities:

1. duplicating global invariants from `AGENTS.md`,
2. owning detailed domain policy that already has a canonical protocol,
3. embedding large runtime-law sections when a dedicated protocol/helper exists.

### 3. Triggered Domain Layer

Activated only when the task route, risk, or artifact flow requires it.

Canonical examples:

1. `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`
2. `vida/config/instructions/runtime-instructions/runtime.task-state-telemetry-protocol.md`
3. `vida/config/instructions/runtime-instructions/work.web-validation-protocol.md`
4. `vida/config/instructions/runtime-instructions/bridge.issue-contract-protocol.md`
5. `vida/config/instructions/runtime-instructions/work.spec-intake-protocol.md`
6. `vida/config/instructions/runtime-instructions/work.spec-delta-protocol.md`
7. `vida/config/instructions/command-instructions/execution.implement-execution-protocol.md`
8. `vida/config/instructions/instruction-contracts/core.agent-system-protocol.md`
9. `vida/config/instructions/runtime-instructions/work.problem-party-protocol.md`
10. `vida/config/instructions/instruction-contracts/overlay.autonomous-execution-protocol.md`
11. `vida/config/instructions/runtime-instructions/work.execution-priority-protocol.md`
12. `vida/config/instructions/instruction-contracts/work.documentation-operation-protocol.md`
13. `vida/config/instructions/instruction-contracts/work.documentation-layer7-migration-protocol.md`
14. `vida/config/instructions/runtime-instructions/lane.agent-handoff-context-protocol.md`
15. `vida/config/instructions/runtime-instructions/recovery.checkpoint-replay-recovery-protocol.md`
16. `vida/config/instructions/runtime-instructions/work.verification-lane-protocol.md`
17. `vida/config/instructions/agent-definitions/role.role-profile-contract.md`
18. `vida/config/instructions/runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`
19. `vida/config/instructions/runtime-instructions/work.verification-merge-protocol.md`
20. `vida/config/instructions/runtime-instructions/runtime.direct-runtime-consumption-protocol.md`
21. `vida/config/instructions/instruction-contracts/work.agent-system-new-protocol-development-and-update-protocol.md`
22. `vida/config/instructions/instruction-contracts/meta.protocol-naming-grammar-protocol.md`
23. `vida/config/instructions/instruction-contracts/meta.core-protocol-standard-protocol.md`

Rule:

1. Domain protocols should be loaded because a route or gate requires them, not because they exist.

### 4. Closure / Reflection Layer

Activated only near checkpoint, handoff, finish, or framework-diagnosis reflection.

Canonical examples:

1. `vida/config/instructions/runtime-instructions/work.task-state-reconciliation-protocol.md`
2. `vida/config/instructions/diagnostic-instructions/analysis.silent-framework-diagnosis-protocol.md`
3. `vida/config/instructions/diagnostic-instructions/analysis.framework-self-analysis-protocol.md`
4. `vida/config/instructions/runtime-instructions/work.human-approval-protocol.md`

Rule:

1. Closure/reflection protocols must not be treated as default boot reads unless the active mode explicitly requires them.

## Activation Matrix

| Phase | Mandatory surfaces | Trigger-only surfaces |
|---|---|---|
| bootstrap | `AGENTS.md` | none |
| lane entry | `ORCHESTRATOR-ENTRY.MD` + `SESSION-CONTEXT-CONTINUITY-PROTOCOL.MD` in orchestrator lane, or `WORKER-ENTRY.MD`; `WORKER-THINKING.MD` in worker lane | none |
| lean execution boot | `step-thinking-protocol.md`, `work.web-validation-protocol.md`, `bridge.project-overlay-protocol.md`, `vida.config.yaml` when present | `agent-system-protocol.md`, `runtime.task-state-telemetry-protocol.md`, `silent-framework-diagnosis-protocol.md` |
| standard/full execution boot | lean set plus route-required pack/TaskFlow/implementation protocols | only the protocols selected by route, pack, or risk |
| tracked execution | TaskFlow / beads / route-specific protocol | domain-specific protocols not triggered by the active path stay unread |
| closure / handoff | reconciliation, approval, diagnosis/reflection protocols as required | none |

## Trigger Matrix

| Condition | Activate |
|---|---|
| repository or runtime mutation required | `work.taskflow-protocol.md`, `runtime.task-state-telemetry-protocol.md` |
| external facts can change the decision | `work.web-validation-protocol.md` |
| issue/bug text is the primary spec input | `bridge.issue-contract-protocol.md` |
| raw inputs are mixed, scope-bearing, or negotiation-heavy | `work.spec-intake-protocol.md` |
| non-equivalent change is visible | `work.spec-delta-protocol.md` |
| implementation route selected | `implement-execution-protocol.md` |
| plan/spec/task pool is settled and the user wants continued execution to completion | `autonomous-execution-protocol.md` |
| tracked execution must choose between multiple lawful next tasks or handle reprioritization | `execution-priority-protocol.md` |
| close/handoff health-check gates are active | `execution-health-check-protocol.md` |
| bounded shell/runtime command discipline is active | `command-execution-discipline-protocol.md` |
| documentation mutation, documentation validation, canonical map work, or documentation-layer tooling work is active | `documentation-operation-protocol.md` |
| a bounded implementation step just succeeded and changed proven project run/build/install conditions | `development-evidence-sync-protocol.md`, `documentation-operation-protocol.md` |
| project documentation is being migrated toward Layer 7 closure | `documentation-layer7-migration-protocol.md` |
| worker mode active for eligible work | `agent-system-protocol.md` |
| a new agent-system protocol is being created, or an existing one is being updated, split, merged, or determinized from a command surface | `agent-system-new-protocol-development-and-update-protocol.md` |
| canonical framework naming law, category-local filename grammar, or a sequential protocol rename wave is being authored, updated, or executed | `meta.protocol-naming-grammar-protocol.md` |
| a `core` protocol is being created, materially rewritten, audited for bounded ownership, or checked for forbidden absorption | `meta.core-protocol-standard-protocol.md` |
| bounded conflict escalation is authorized | `work.problem-party-protocol.md` |
| worker packet design, next-agent prompt formation, handoff boundaries, or context shaping is active | `agent-handoff-context-protocol.md` |
| restart, resumability, checkpoint, replay, recovery, or duplicate-delivery safety is active | `recovery.checkpoint-replay-recovery-protocol.md` |
| separated authorship, coach, verifier, proving, or closure proof semantics are active | `work.verification-lane-protocol.md` |
| role identity/stance is being defined or changed independently of permissions and authority | `role.role-profile-contract.md` |
| project role/skill/profile/flow extension activation, validation, or compilation is active | `work.project-agent-extension-protocol.md` |
| auto-lane selection or conversational lane modes for scope/PBI work are active | `work.agent-lane-selection-protocol.md` |
| routed worker admissibility must be proven against typed task-class capabilities before ranking or delegation | `capability-registry-protocol.md` |
| compiled runtime bundle composition or kernel bundle readiness is active | `runtime-kernel-bundle-protocol.md` |
| multi-verifier admissibility or merged verification verdicts are active | `work.verification-merge-protocol.md` |
| final direct runtime consumption or explicit taskflow -> docflow closure evidence is active | `direct-runtime-consumption-protocol.md` |
| pack boundary admissibility or cross-pack handoff law is active | `work.pack-handoff-protocol.md` |
| a routed pack is about to be declared complete or pack-complete proof must be checked | `work.pack-completion-gate-protocol.md` |
| material scope / AC / dependency / decision drift must be reconciled across reflection, spec review, and task-pool rebuild | `work.change-impact-reconciliation-protocol.md` |
| task appears stale, done-but-open, or drifted | `work.task-state-reconciliation-protocol.md` |
| silent diagnosis mode enabled | `silent-framework-diagnosis-protocol.md` |
| explicit framework self-analysis/remediation doctrine needed | `framework-self-analysis-protocol.md` |
| one protocol family or protocol-bearing category is being audited for owner, activation, terminology, or index consistency | `protocol-consistency-audit-protocol.md` |

## Explicit Canonical Protocol Coverage

The following canonical protocol-bearing artifacts are explicitly covered by this activation law.

### Triggered-domain coverage

1. `vida/config/instructions/agent-backends/role.backend-lifecycle-protocol.md`
2. `vida/config/instructions/agent-definitions/model.agent-definitions-contract.md`
3. `vida/config/instructions/command-instructions/execution.bug-fix-protocol.md`
4. `vida/config/instructions/command-instructions/routing.command-layer-protocol.md`
5. `vida/config/instructions/command-instructions/routing.use-case-packs-protocol.md`
6. `vida/config/instructions/command-instructions/planning.form-task-protocol.md`
7. `vida/config/instructions/command-instructions/execution.project-bootstrap-protocol.md`
8. `vida/config/instructions/diagnostic-instructions/escalation.debug-escalation-protocol.md`
9. `vida/config/instructions/diagnostic-instructions/evaluation.library-evaluation-protocol.md`
10. `vida/config/instructions/diagnostic-instructions/evaluation.product-proving-pack-scaffold-contract.md`
11. `vida/config/instructions/diagnostic-instructions/analysis.protocol-self-diagnosis-protocol.md`
12. `vida/config/instructions/diagnostic-instructions/analysis.protocol-consistency-audit-protocol.md`
13. `vida/config/instructions/diagnostic-instructions/analysis.self-reflection-protocol.md`
14. `vida/config/instructions/instruction-contracts/core.orchestration-protocol.md`
15. `vida/config/instructions/instruction-contracts/overlay.step-thinking-protocol.md`
16. `vida/config/instructions/instruction-contracts/overlay.session-context-continuity-protocol.md`
17. `vida/config/instructions/instruction-contracts/lane.worker-dispatch-protocol.md`
18. `vida/config/instructions/runtime-instructions/model.boot-packet-protocol.md`
19. `vida/config/instructions/runtime-instructions/core.capability-registry-protocol.md`
20. `vida/config/instructions/runtime-instructions/core.context-governance-protocol.md`
21. `vida/config/instructions/runtime-instructions/work.document-lifecycle-protocol.md`
22. `vida/config/instructions/runtime-instructions/runtime.export-protocol.md`
23. `vida/config/instructions/runtime-instructions/runtime.framework-memory-protocol.md`
24. `vida/config/instructions/runtime-instructions/bridge.project-overlay-protocol.md`
25. `vida/config/instructions/runtime-instructions/core.run-graph-protocol.md`
26. `vida/config/instructions/runtime-instructions/work.spec-contract-protocol.md`
27. `vida/config/instructions/runtime-instructions/work.spec-freshness-protocol.md`
28. `vida/config/instructions/runtime-instructions/bridge.spec-sync-protocol.md`
29. `vida/config/instructions/runtime-instructions/bridge.task-approval-loop-protocol.md`
30. `vida/config/instructions/runtime-instructions/observability.trace-grading-protocol.md`
31. `vida/config/instructions/runtime-instructions/lane.agent-handoff-context-protocol.md`
32. `vida/config/instructions/runtime-instructions/recovery.checkpoint-replay-recovery-protocol.md`
33. `vida/config/instructions/runtime-instructions/work.verification-lane-protocol.md`
34. `vida/config/instructions/agent-definitions/role.role-profile-contract.md`
35. `vida/config/instructions/runtime-instructions/work.project-agent-extension-protocol.md`
36. `vida/config/instructions/runtime-instructions/work.agent-lane-selection-protocol.md`
37. `vida/config/instructions/runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`
38. `vida/config/instructions/runtime-instructions/work.verification-merge-protocol.md`
39. `vida/config/instructions/runtime-instructions/runtime.direct-runtime-consumption-protocol.md`
40. `vida/config/instructions/runtime-instructions/work.development-evidence-sync-protocol.md`
41. `vida/config/instructions/runtime-instructions/work.pack-handoff-protocol.md`
42. `vida/config/instructions/runtime-instructions/work.pack-completion-gate-protocol.md`
43. `vida/config/instructions/instruction-contracts/work.agent-system-new-protocol-development-and-update-protocol.md`
44. `vida/config/instructions/instruction-contracts/meta.protocol-naming-grammar-protocol.md`
45. `vida/config/instructions/instruction-contracts/meta.core-protocol-standard-protocol.md`
46. `vida/config/instructions/runtime-instructions/work.execution-health-check-protocol.md`
47. `vida/config/instructions/runtime-instructions/work.command-execution-discipline-protocol.md`
48. `vida/config/instructions/runtime-instructions/work.change-impact-reconciliation-protocol.md`

Coverage rule:

1. these artifacts are canonical protocol-bearing surfaces even when they are not named in the shorter activation examples above,
2. each of them must remain representable by this protocol's activation classes and trigger matrix,
3. `protocol-coverage-check` may treat absence of one of these artifacts from this law or from the protocol index as blocking drift.

## Decomposition Guidance

Use this protocol to decide whether an instruction file should be decomposed.

Decompose when at least one is true:

1. the file mixes bootstrap and domain-runtime policy,
2. the file duplicates rules whose canonical owner already exists elsewhere,
3. the file cannot be activated by one clear phase,
4. the file title no longer matches its actual responsibilities,
5. the file contains both trigger selection and deep domain law.

Do not decompose when all are true:

1. one owner phase is clear,
2. the file is still inspectable in one bounded read,
3. splitting it would only create pointer noise.

## Naming Guidance

Instruction file names should describe their real role.

Examples:

1. `entry` means boot and routing into deeper policy, not the full operating doctrine.
2. `protocol` means the canonical owner for a domain/runtime rule set.
3. `router` means lane or command selection, not execution law.
4. `guide` or `reference` means non-canonical support material.

Rule:

1. If a filename and its real role diverge, fix either the name or the scope.

## Wiring Rule

When introducing a new instruction surface:

1. assign one canonical owner,
2. assign one activation class from this protocol,
3. add it to `vida/config/instructions/system-maps/protocol.index.md`,
4. update only the minimal pointers in `AGENTS.md` or lane-entry files,
5. do not restate the full policy body in upper layers.

## Runtime Binding Rule

Instructions are strongest when bound to runtime surfaces.

Preferred order:

1. helper or gate,
2. receipt or state artifact,
3. tests,
4. protocol prose,
5. upper-layer pointer text.

Rule:

1. If a policy matters operationally, bind it to helper/gate/test surfaces instead of only repeating it in entry docs.

## Refactor Priority

Use this order for instruction-layer cleanup:

1. shrink the bootstrap layer,
2. keep lane-entry files focused on activation and routing,
3. move deep law into canonical domain protocols,
4. replace duplicated prose with decision tables,
5. update protocol index and change log last.

-----
artifact_path: config/instructions/instruction-contracts/bridge.instruction-activation.protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md
created_at: '2026-03-08T02:15:22+02:00'
updated_at: '2026-03-12T11:55:06+02:00'
changelog_ref: bridge.instruction-activation-protocol.changelog.jsonl
