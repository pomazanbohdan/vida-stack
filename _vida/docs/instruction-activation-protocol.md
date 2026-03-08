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

1. `_vida/docs/ORCHESTRATOR-ENTRY.MD`
2. `_vida/docs/SUBAGENT-ENTRY.MD`
3. `_vida/docs/SUBAGENT-THINKING.MD`

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

1. `_vida/docs/todo-protocol.md`
2. `_vida/docs/beads-protocol.md`
3. `_vida/docs/web-validation-protocol.md`
4. `_vida/docs/issue-contract-protocol.md`
5. `_vida/docs/spec-intake-protocol.md`
6. `_vida/docs/spec-delta-protocol.md`
7. `_vida/docs/implement-execution-protocol.md`
8. `_vida/docs/subagent-system-protocol.md`
9. `_vida/docs/problem-party-protocol.md`

Rule:

1. Domain protocols should be loaded because a route or gate requires them, not because they exist.

### 4. Closure / Reflection Layer

Activated only near checkpoint, handoff, finish, or framework-diagnosis reflection.

Canonical examples:

1. `_vida/docs/task-state-reconciliation-protocol.md`
2. `_vida/docs/silent-framework-diagnosis-protocol.md`
3. `_vida/docs/framework-self-analysis-protocol.md`
4. `_vida/docs/human-approval-protocol.md`

Rule:

1. Closure/reflection protocols must not be treated as default boot reads unless the active mode explicitly requires them.

## Activation Matrix

| Phase | Mandatory surfaces | Trigger-only surfaces |
|---|---|---|
| bootstrap | `AGENTS.md` | none |
| lane entry | `ORCHESTRATOR-ENTRY.MD` or `SUBAGENT-ENTRY.MD`; `SUBAGENT-THINKING.MD` in worker lane | none |
| lean execution boot | `thinking-protocol.md`, `web-validation-protocol.md`, `project-overlay-protocol.md`, `vida.config.yaml` when present | `subagent-system-protocol.md`, `beads-protocol.md`, `silent-framework-diagnosis-protocol.md` |
| standard/full execution boot | lean set plus route-required pack/TODO/implementation protocols | only the protocols selected by route, pack, or risk |
| tracked execution | TODO / beads / route-specific protocol | domain-specific protocols not triggered by the active path stay unread |
| closure / handoff | reconciliation, approval, diagnosis/reflection protocols as required | none |

## Trigger Matrix

| Condition | Activate |
|---|---|
| repository or runtime mutation required | `todo-protocol.md`, `beads-protocol.md` |
| external facts can change the decision | `web-validation-protocol.md` |
| issue/bug text is the primary spec input | `issue-contract-protocol.md` |
| raw inputs are mixed, scope-bearing, or negotiation-heavy | `spec-intake-protocol.md` |
| non-equivalent change is visible | `spec-delta-protocol.md` |
| implementation route selected | `implement-execution-protocol.md` |
| subagent mode active for eligible work | `subagent-system-protocol.md` |
| bounded conflict escalation is authorized | `problem-party-protocol.md` |
| task appears stale, done-but-open, or drifted | `task-state-reconciliation-protocol.md` |
| silent diagnosis mode enabled | `silent-framework-diagnosis-protocol.md` |
| explicit framework self-analysis/remediation doctrine needed | `framework-self-analysis-protocol.md` |

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
3. add it to `_vida/docs/protocol-index.md`,
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
