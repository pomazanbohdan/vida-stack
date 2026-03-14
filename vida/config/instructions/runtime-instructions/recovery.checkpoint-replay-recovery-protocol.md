# Checkpoint Replay And Recovery Protocol

Purpose: define the canonical runtime law for resumability, checkpoint ownership, replay-safe recovery, and idempotent retry behavior across VIDA routed execution.

## Core Contract

1. Durable resumability must be carried by canonical runtime artifacts, not by operator memory.
2. Checkpoint ownership, replay scope, and recovery boundaries must remain explicit.
3. Recovery must preserve fail-closed route and proof semantics.
4. Retry or replay must not silently rewrite canonical history.

## Canonical Runtime Artifacts

Primary runtime artifacts:

1. `runtime-instructions/core.run-graph-protocol`
2. `runtime-instructions/core.context-governance-protocol`
3. `docs/product/spec/checkpoint-commit-and-replay-model.md`

Current durable surfaces:

1. `.vida/state/run-graphs/<task_id>.json`
2. `.vida/state/context-governance.json`

## Ownership Rule

1. task lifecycle remains in the DB-backed task runtime,
2. TaskFlow remains execution telemetry and orchestration substrate,
3. run-graph remains the canonical node-level resumability ledger,
4. checkpoint commit and replay lineage remain runtime-owned concerns.

## Recovery Rule

On compact, restart, or interrupted routed execution:

1. recover the active task and route state from canonical runtime artifacts,
2. recover the next resumable node from the run-graph,
3. recover governed context from context governance when required,
4. re-enter the route through the smallest lawful resumable boundary,
5. do not resume from chat-memory assumptions alone.
6. if the last checkpoint shows exhausted stall/reset/budget limits, do not resume the same writer path automatically; route to replan, escalation, or explicit override.

## Replay Rule

Replay is allowed only to rebuild or prove derived/runtime surfaces.

Rules:

1. replay must keep explicit lineage distinct from the original live pass,
2. replay must not rewrite canonical receipts or canonical machine history,
3. replay scope must be bounded and named,
4. replay artifacts must remain clearly marked as replay-derived or debug-derived when applicable.

## Idempotency Rule

Where delayed checkpoint writes, retry, or duplicate delivery are possible:

1. handlers and lane-level side effects must remain idempotent or explicitly guarded,
2. proof and verification paths must tolerate safe repeated invocation,
3. recovery logic must not assume exactly-once side effects unless a stronger lower runtime guarantees it.

## Checkpoint Completeness Rule

A routed implementation checkpoint is resumability-ready only when it records the smallest lawful continuation packet.

Minimum continuation packet:

1. `task_id`
2. `delivery_task_id`
3. `execution_block_id` or equivalent bounded writer packet id
4. `owned_paths` or equivalent write boundary
5. active node and `resume_hint`
6. current verification or `review_pool` target
7. current runtime control counters:
   - `round_count`
   - `stall_count`
   - `reset_count`
   - `budget_units_consumed` when budgeted
8. explicit blocker or next-step reason when the task is not closure-ready

Rules:

1. missing continuation fields make recovery fail-closed,
2. resume must restart from the smallest lawful bounded node, not from a broader milestone reinterpretation,
3. recovery may refresh context or receipts, but it must not silently widen write scope.

## Recovery Gate

Recovery is not lawful unless all are true:

1. resumability boundary is explicit,
2. next node or next checkpoint position is known,
3. required proof/verification boundary remains inspectable,
4. widening the scope is not required.

If any item is missing:

1. fail closed,
2. escalate through the canonical runtime/debug path rather than inventing manual continuation.
3. treat missing control counters or missing `delivery_task_id` as a non-resumable checkpoint defect.

## Historical Lineage Note

Historical runtime-lineage provenance is preserved in:

1. `docs/process/framework-source-lineage-index.md`

## References

1. `runtime-instructions/core.run-graph-protocol`
2. `runtime-instructions/core.context-governance-protocol`
3. `docs/product/spec/checkpoint-commit-and-replay-model.md`
4. `docs/process/framework-source-lineage-index.md`

-----
artifact_path: config/runtime-instructions/checkpoint-replay-recovery.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/recovery.checkpoint-replay-recovery-protocol.md
created_at: '2026-03-10T15:05:00+02:00'
updated_at: '2026-03-13T07:14:58+02:00'
changelog_ref: recovery.checkpoint-replay-recovery-protocol.changelog.jsonl
