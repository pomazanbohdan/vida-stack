# Execution Priority Protocol (EPP)

Purpose: define one canonical doctrine for selecting the next task/block, handling reprioritization, and preserving autonomous follow-through without silent scope drift.

Scope:

1. applies when tracked execution is active,
2. applies to TaskFlow task selection, wave ordering, and in-session reprioritization,
3. works together with `vida/config/instructions/command-instructions/planning.form-task-protocol.md`, `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`, `vida/config/instructions/command-instructions/execution.implement-execution-protocol.md`, and `vida/config/instructions/instruction-contracts/overlay.autonomous-execution-protocol.md`.

## Core Contract

Priority selection must answer two questions separately:

1. `what is lawful next?`
2. `what is highest-value among the lawful next options?`

The orchestrator must never select work only because it is attractive, nearby, or easy if that work is not lawful next according to the active contracts.

## Canonical Priority Inputs

Use these inputs in order:

1. active blocker/verification/approval receipts,
2. active TaskFlow block and `next_step`,
3. `br` lifecycle state and dependency state,
4. approved scope/order contract from `vida/config/instructions/command-instructions/planning.form-task-protocol.md`,
5. canonical wave/plan ordering,
6. route-required worker findings or verifier/coach results,
7. explicit user reprioritization,
8. local convenience heuristics.

Lower-tier inputs must not override higher-tier ones.

Autonomy default:

1. If the next lawful task is unambiguous after applying the priority inputs above, execute it without re-prompting the user.
2. Do not ask the user to choose between tasks when this protocol already determines the winner.

## Priority Selection Rule

When multiple tasks are candidate next work, select in this order:

1. continue the active `doing` block if it is still lawful,
2. else the explicit `next_step` block in the same task/track,
3. else the next `ready` task that preserves approved dependency/wave order,
4. else the highest-priority unblocked task within the same approved delivery cut,
5. else stop and surface the blocker rather than widening scope.

Worker-sensitive selection rule:

1. If a higher-priority candidate still lacks required worker analysis/review/verification state, it is not lawful next work yet.
2. In that case, prefer the next candidate whose route receipts and worker gates are already satisfiable.

## Reprioritization Rule

Reprioritization is lawful only when at least one is true:

1. the user explicitly changes priority,
2. a higher-evidence blocker invalidates the current next task,
3. a framework/runtime issue must be captured now and deferred to the next task boundary,
4. approved scope/order contract already declares a different delivery cut or risk policy.

Reprioritization must not happen because:

1. another task looks more interesting,
2. the current task is annoying,
3. the agent wants a cleaner story,
4. a later wave feels easier to implement first.

## Canonical Reprioritization Actions

When reprioritization is lawful:

1. use `beads-workflow.sh redirect` for block-level focus changes,
2. update task status only through canonical `br`/beads paths,
3. preserve the interrupted task as resumable state when possible,
4. record the reason in TaskFlow evidence or task notes,
5. if scope/order contract changes materially, route back through form-task/spec reconciliation instead of silently continuing.

Reprioritization prompt rule:

1. If reprioritization follows directly from explicit user intent or higher-evidence blocker state, apply it and continue; do not ask the user to reconfirm what the protocol already resolved.
2. Ask the user only when two or more competing priority outcomes remain lawful after applying all canonical inputs.

## Scope-Approval Contract Binding

This protocol treats the approved FTP scope contract as the authoritative scope/order decision surface.

Binding rules:

1. approved `scope_boundary` decides what work is in/out,
2. approved `delivery_cut` decides MVP-first vs full-slice order,
3. approved `dependency_strategy` decides sequential vs parallel-safe execution,
4. approved `risk_policy` decides verification depth and aggression level.

If any of these change materially, autonomous follow-through must stop and re-enter the appropriate contract path instead of improvising.

## Autonomous Execution Binding

When `vida/config/instructions/instruction-contracts/overlay.autonomous-execution-protocol.md` is active:

1. autonomous mode may keep moving forward only across tasks/blocks that remain lawful under this protocol,
2. autonomous mode must stop on ambiguous priority or unresolved reprioritization,
3. autonomous mode should prefer continuity of the active delivery cut over opportunistic side work.

## Anti-Patterns

1. treating all `br ready` tasks as equally valid next work,
2. ignoring the active TaskFlow block because another task is already open,
3. silently promoting framework fixes ahead of product work mid-block,
4. using chat memory as the only reprioritization record,
5. changing delivery cut or risk posture without re-entering the scope/approval contract.

-----
artifact_path: config/runtime-instructions/execution-priority.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.execution-priority-protocol.md
created_at: '2026-03-09T12:00:46+02:00'
updated_at: '2026-03-11T13:03:11+02:00'
changelog_ref: work.execution-priority-protocol.changelog.jsonl
