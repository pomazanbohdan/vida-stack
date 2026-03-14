# Run-Graph Protocol (RGP)

Purpose: persist durable node-level orchestration state for one routed execution run beyond execution-telemetry checkpoints and task lifecycle state.

## Core Contract

1. Task lifecycle truth remains in the canonical task-state surface.
2. Execution telemetry remains in the canonical execution-telemetry surface.
3. Run-graph ledger is the canonical node-level resumability surface for one routed orchestration run.

## Canonical Artifact

1. the canonical routed-run resumability ledger artifact resolved by the active framework runtime
2. runtime configuration may relocate the physical storage path without changing this protocol's ownership or ledger semantics

## Activation Surface

Activate this protocol when at least one is true:

1. one routed execution run needs node-level resumability beyond task lifecycle truth and execution telemetry,
2. recovery or replay must identify the next resumable routed node,
3. execution-stage continuity must survive interruption, compact, or retry,
4. observability or recovery work needs the canonical routed-run ledger rather than ad hoc stage notes.

Primary activating companions:

1. `instruction-contracts/core.orchestration-protocol`
2. `runtime-instructions/core.context-governance-protocol`
3. `runtime-instructions/recovery.checkpoint-replay-recovery-protocol`
4. `instruction-contracts/bridge.instruction-activation-protocol`

Minimum fields:

1. `task_id`
2. `task_class`
3. `route_task_class`
4. `updated_at`
5. `nodes`
6. `control_limits`
7. `control_counters`
8. `resume_hint`

Required nodes:

1. `analysis`
2. `writer`
3. `coach`
4. `problem_party`
5. `verifier`
6. `approval`
7. `synthesis`

Minimum `control_limits` fields when declared by route or overlay:

1. `max_rounds`
2. `max_stalls`
3. `max_resets`
4. `max_budget_units`
5. `max_total_runtime_seconds`

Minimum `control_counters` fields:

1. `round_count`
2. `stall_count`
3. `reset_count`
4. `budget_units_consumed`
5. `runtime_seconds_consumed`

`resume_hint` minimum fields:

1. `next_node`
2. `delivery_task_id`
3. `execution_block_id`
4. `verification_target`
5. `reason`

Allowed node statuses:

1. `pending`
2. `ready`
3. `running`
4. `completed`
5. `blocked`
6. `failed`
7. `skipped`

## Usage Rule

1. Initialize or refresh the run-graph before routed execution stages start.
2. Update node status when a routed stage enters `running`, reaches `completed`, or stops in `blocked|failed`.
3. Use node metadata for concise resumability evidence such as evidence pointers, reason, or selected route class.
4. Do not treat run-graph as a second task-lifecycle state engine; it records stage status inside one task, not queue readiness.
5. Expose a compact `resume_hint` for boot/operator surfaces so compact recovery can point to the next resumable node.
6. Operator surfaces may flag suspicious or non-canonical run-graph artifacts when they pollute resumability summaries; the ledger remains canonical, but operator views should not silently treat test pollution as production state.
7. Refresh `control_counters` at each writer, coach, verifier, and reset boundary.
8. When a no-progress or budget/stall rule fires, persist that event in the run-graph before recovery or escalation routing.
9. If a review pool is active, `verification_target` should identify the current single-task verifier path or the declared `review_pool` checkpoint.

## Boundary Rule

1. this protocol owns node-level routed-run resumability only,
2. it does not own task lifecycle truth, which stays in the canonical task-state surface,
3. it does not own execution telemetry, which stays in the canonical execution-telemetry surface,
4. it does not own governed evidence classification, which stays in `core.context-governance`,
5. it must not become a generic runtime command catalog.

## Required Core Linkages

1. `core.orchestration` depends on this protocol when routed execution stages need node-level resumability and compact recovery hints,
2. `core.context-governance` remains a peer continuity source and must not be replaced by run-graph state,
3. recovery and replay owners may project this ledger, but they do not replace it as the canonical node-level resumability source.

## Initial Local Coverage

Current minimum runtime integration:

1. the active pre-execution owner must persist `analysis` node state and the next `writer` readiness decision.
2. the active coach owner should persist `coach` node state.
3. the active structured-conflict owner should persist `problem_party` completion and the next `writer` readiness decision when the result unblocks execution.
4. `verification`, `approval`, and `synthesis` state should also be reflected in the same ledger.
5. the active execution owner should persist control-limit binding and control-counter updates before and after every resumable writer pass.

## Operational Proof And Closure

1. a routed run is resumability-ready only when the active node state can be read from the canonical ledger with a compact next-step hint,
2. run-graph state must remain coherent with orchestration stage transitions and must not become a second task queue,
3. recovery or replay surfaces may project this ledger, but they must not replace it as the canonical node-level state source.

## Runtime Surface Note

1. concrete operator commands and runtime entrypoints for initializing, updating, or reading run-graph state stay in runtime-family and migration surfaces,
2. this protocol owns the resumability law and minimum ledger semantics, not the concrete command syntax.

-----
artifact_path: config/runtime-instructions/run-graph.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/core.run-graph-protocol.md
created_at: '2026-03-08T02:15:22+02:00'
updated_at: '2026-03-13T07:14:58+02:00'
changelog_ref: core.run-graph-protocol.changelog.jsonl
