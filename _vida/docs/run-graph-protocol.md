# Run-Graph Protocol (RGP)

Purpose: persist durable node-level orchestration state for one routed execution run beyond TODO checkpoints and task lifecycle state.

## Core Contract

1. Task lifecycle stays in `br`.
2. TODO remains execution telemetry.
3. Run-graph ledger is the canonical node-level resumability surface for one routed orchestration run.

## Canonical Artifact

1. `.vida/state/run-graphs/<task_id>.json`
2. Test or isolated runtime paths may override the state directory through framework-controlled runtime configuration such as `VIDA_RUN_GRAPH_STATE_DIR`; canonical production state remains `.vida/state/run-graphs/`.

Minimum fields:

1. `task_id`
2. `task_class`
3. `route_task_class`
4. `updated_at`
5. `nodes`

Required nodes:

1. `analysis`
2. `writer`
3. `coach`
4. `problem_party`
5. `verifier`
6. `approval`
7. `synthesis`

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
3. Use node metadata for concise resumability evidence such as manifest path, receipt path, reason, or selected route class.
4. Do not treat run-graph as a second task-state engine; it records stage status inside one task, not queue readiness.
5. Expose a compact `resume_hint` for boot/operator surfaces so compact recovery can point to the next resumable node.
6. Operator surfaces may flag suspicious or non-canonical run-graph artifacts when they pollute resumability summaries; the ledger remains canonical, but operator views should not silently treat test pollution as production state.

## Initial Local Coverage

Current minimum runtime integration:

1. `prepare-execution` must persist `analysis` node state and the next `writer` readiness decision.
2. `coach-review` must persist `coach` node state.
3. `problem-party` receipt/runtime path should persist `problem_party` completion and the next `writer` readiness decision when the debate result unblocks execution.
4. `verification`, `approval`, and `synthesis` state should also be reflected in the same ledger.

## Commands

```bash
python3 _vida/scripts/run-graph.py init <task_id> <task_class> [route_task_class]
python3 _vida/scripts/run-graph.py update <task_id> <task_class> <node> <status> [route_task_class] [meta_json]
python3 _vida/scripts/run-graph.py status <task_id>
```
