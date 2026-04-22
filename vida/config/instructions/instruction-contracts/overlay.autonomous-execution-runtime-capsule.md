# Autonomous Execution Runtime Capsule

Purpose: provide a compact runtime-facing projection of the highest-frequency autonomous follow-through law for routine continued execution.

Boundary rule:

1. this file is a compact projection, not the owner of autonomous-execution law,
2. the canonical owner remains `instruction-contracts/overlay.autonomous-execution-protocol`,
3. use the owner file when a stop condition, task-boundary doctrine, approval interaction, or overlay-governed edge case is not resolved by this capsule.

## Always-Keep-Visible

1. whether continued execution is already authorized,
2. whether a lawful next task or block is explicit,
3. whether `continue_after_reports` is active,
4. whether validation or approval still gates the next implementation step,
5. whether the current closure is `execution_block`, `delivery_task`, or true pool completion.

## High-Frequency Laws

1. autonomous follow-through continues only across already-authorized tasks or blocks; it does not authorize silent scope expansion,
2. reports, green checks, or one bounded success are not natural pause points while lawful work remains,
3. after one `execution_block` closes, reconcile through the parent `delivery_task` before any task-boundary or session-boundary behavior,
4. after one task closes, run next-task boundary analysis before entering the next task,
5. if the next lawful task or block is already known and no stop condition is active, continue automatically,
6. validation-before-implementation remains gating even when `continue_after_reports=true`,
7. spec-ready transition into downstream implementation flow and post-validation continuation are runtime-defined behaviors, not live project overlay toggles,
8. worker-first and verification law remain active during autonomous follow-through; AEP does not authorize root-session local writing by itself,
9. if another lawful ready task exists in the same authorized pool, task-local closure does not mean execution is finished.

## Stop Conditions

1. unresolved `failed` or `partial` state,
2. scope or ownership would widen beyond current authority,
3. missing or contradictory task/verification state,
4. no lawful next task or block can be selected,
5. blocker, approval gate, validation gate, or explicit user interruption is active.

## Escalate To Owner File When

1. overlay-governed next-task boundary reporting behavior is disputed,
2. approval-loop interaction with autonomous follow-through is unclear,
3. dependent coverage refresh or spec/task mutation is required at task boundary,
4. the legality of continuing after review, rework, or partial-return is unclear,
5. a framework/protocol mutation may change autonomous follow-through law itself.

-----
artifact_path: config/instructions/instruction-contracts/overlay.autonomous-execution-runtime-capsule
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-14'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/overlay.autonomous-execution-runtime-capsule.md
created_at: '2026-03-14T00:15:00+02:00'
updated_at: '2026-03-14T00:15:00+02:00'
changelog_ref: overlay.autonomous-execution-runtime-capsule.changelog.jsonl
