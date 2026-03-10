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

1. `vida/config/instructions/agent-definitions.orchestrator-entry.md`
2. `vida/config/instructions/agent-definitions.worker-entry.md`
3. `vida/config/instructions/instruction-contracts.worker-thinking.md`

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

1. `vida/config/instructions/runtime-instructions.taskflow-protocol.md`
2. `vida/config/instructions/runtime-instructions.beads-protocol.md`
3. `vida/config/instructions/runtime-instructions.web-validation-protocol.md`
4. `vida/config/instructions/runtime-instructions.issue-contract-protocol.md`
5. `vida/config/instructions/runtime-instructions.spec-intake-protocol.md`
6. `vida/config/instructions/runtime-instructions.spec-delta-protocol.md`
7. `vida/config/instructions/command-instructions.implement-execution-protocol.md`
8. `vida/config/instructions/instruction-contracts.agent-system-protocol.md`
9. `vida/config/instructions/runtime-instructions.problem-party-protocol.md`
10. `vida/config/instructions/instruction-contracts.autonomous-execution-protocol.md`
11. `vida/config/instructions/runtime-instructions.execution-priority-protocol.md`
12. `vida/config/instructions/instruction-contracts.documentation-operation-protocol.md`

Rule:

1. Domain protocols should be loaded because a route or gate requires them, not because they exist.

### 4. Closure / Reflection Layer

Activated only near checkpoint, handoff, finish, or framework-diagnosis reflection.

Canonical examples:

1. `vida/config/instructions/runtime-instructions.task-state-reconciliation-protocol.md`
2. `vida/config/instructions/diagnostic-instructions.silent-framework-diagnosis-protocol.md`
3. `vida/config/instructions/diagnostic-instructions.framework-self-analysis-protocol.md`
4. `vida/config/instructions/runtime-instructions.human-approval-protocol.md`

Rule:

1. Closure/reflection protocols must not be treated as default boot reads unless the active mode explicitly requires them.

## Activation Matrix

| Phase | Mandatory surfaces | Trigger-only surfaces |
|---|---|---|
| bootstrap | `AGENTS.md` | none |
| lane entry | `ORCHESTRATOR-ENTRY.MD` or `WORKER-ENTRY.MD`; `WORKER-THINKING.MD` in worker lane | none |
| lean execution boot | `thinking-protocol.md`, `web-validation-protocol.md`, `project-overlay-protocol.md`, `vida.config.yaml` when present | `agent-system-protocol.md`, `beads-protocol.md`, `silent-framework-diagnosis-protocol.md` |
| standard/full execution boot | lean set plus route-required pack/TaskFlow/implementation protocols | only the protocols selected by route, pack, or risk |
| tracked execution | TaskFlow / beads / route-specific protocol | domain-specific protocols not triggered by the active path stay unread |
| closure / handoff | reconciliation, approval, diagnosis/reflection protocols as required | none |

## Trigger Matrix

| Condition | Activate |
|---|---|
| repository or runtime mutation required | `taskflow-protocol.md`, `beads-protocol.md` |
| external facts can change the decision | `web-validation-protocol.md` |
| issue/bug text is the primary spec input | `issue-contract-protocol.md` |
| raw inputs are mixed, scope-bearing, or negotiation-heavy | `spec-intake-protocol.md` |
| non-equivalent change is visible | `spec-delta-protocol.md` |
| implementation route selected | `implement-execution-protocol.md` |
| plan/spec/task pool is settled and the user wants continued execution to completion | `autonomous-execution-protocol.md` |
| tracked execution must choose between multiple lawful next tasks or handle reprioritization | `execution-priority-protocol.md` |
| documentation mutation, documentation validation, canonical map work, or documentation-layer tooling work is active | `documentation-operation-protocol.md` |
| worker mode active for eligible work | `agent-system-protocol.md` |
| bounded conflict escalation is authorized | `problem-party-protocol.md` |
| task appears stale, done-but-open, or drifted | `task-state-reconciliation-protocol.md` |
| silent diagnosis mode enabled | `silent-framework-diagnosis-protocol.md` |
| explicit framework self-analysis/remediation doctrine needed | `framework-self-analysis-protocol.md` |

## Explicit Canonical Protocol Coverage

The following canonical protocol-bearing artifacts are explicitly covered by this activation law.

### Triggered-domain coverage

1. `vida/config/instructions/agent-backends.lifecycle-protocol.md`
2. `vida/config/instructions/agent-definitions.protocol.md`
3. `vida/config/instructions/command-instructions.bug-fix-protocol.md`
4. `vida/config/instructions/command-instructions.command-layer-protocol.md`
5. `vida/config/instructions/command-instructions.form-task-protocol.md`
6. `vida/config/instructions/command-instructions.project-bootstrap-protocol.md`
7. `vida/config/instructions/diagnostic-instructions.debug-escalation-protocol.md`
8. `vida/config/instructions/diagnostic-instructions.library-evaluation-protocol.md`
9. `vida/config/instructions/diagnostic-instructions.product-proving-packs-protocol.md`
10. `vida/config/instructions/diagnostic-instructions.protocol-self-diagnosis-protocol.md`
11. `vida/config/instructions/diagnostic-instructions.self-reflection-protocol.md`
12. `vida/config/instructions/instruction-contracts.orchestration-protocol.md`
13. `vida/config/instructions/instruction-contracts.thinking-protocol.md`
14. `vida/config/instructions/instruction-contracts.worker-dispatch-protocol.md`
15. `vida/config/instructions/runtime-instructions.boot-packet-protocol.md`
16. `vida/config/instructions/runtime-instructions.capability-registry-protocol.md`
17. `vida/config/instructions/runtime-instructions.context-governance-protocol.md`
18. `vida/config/instructions/runtime-instructions.document-lifecycle-protocol.md`
19. `vida/config/instructions/runtime-instructions.export-protocol.md`
20. `vida/config/instructions/runtime-instructions.framework-memory-protocol.md`
21. `vida/config/instructions/runtime-instructions.project-overlay-protocol.md`
22. `vida/config/instructions/runtime-instructions.run-graph-protocol.md`
23. `vida/config/instructions/runtime-instructions.spec-contract-protocol.md`
24. `vida/config/instructions/runtime-instructions.spec-freshness-protocol.md`
25. `vida/config/instructions/runtime-instructions.spec-sync-protocol.md`
26. `vida/config/instructions/runtime-instructions.task-approval-loop-protocol.md`
27. `vida/config/instructions/runtime-instructions.trace-eval-protocol.md`

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
3. add it to `vida/config/instructions/system-maps.protocol-index.md`,
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
artifact_path: config/instructions/instruction-contracts/instruction-activation.protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts.instruction-activation-protocol.md
created_at: '2026-03-08T02:15:22+02:00'
updated_at: '2026-03-10T04:07:10+02:00'
changelog_ref: instruction-contracts.instruction-activation-protocol.changelog.jsonl
