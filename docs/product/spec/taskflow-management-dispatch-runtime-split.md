# TaskFlow Management and Dispatch Runtime Split

Status: canonical

## Summary

TaskFlow is split into two logical runtimes over one authoritative state store:

1. `Task Management Runtime` is always available through `vida task` and owns task rows, graph mutations, lifecycle validation, and management closure.
2. `Task Dispatch Runtime` is opt-in through `taskflow.dispatch.enabled` and owns worker, scheduler, run-graph, packet, receipt, and execution-driven lifecycle transitions.

## Configuration

```yaml
taskflow:
  dispatch:
    enabled: false
```

The default is management-only. Host execution class and `dev_team.enabled` do not enable dispatch.

## Authority and boundaries

- Management and dispatch share one canonical task store and one lifecycle API.
- `TaskLifecycleMutationSource::Management` is used for ordinary task operations.
- `TaskLifecycleMutationSource::DispatchReceipt` requires a matching run and receipt identity.
- Execution-bound tasks are identified deterministically: canonical `execution_semantics` must be non-empty, or an active run-graph/task binding is present while dispatch is enabled. `owned_paths`, `acceptance_targets`, and `proof_targets` do not bind a task.
- `ManagementOnly + Management` may close a task even when proof targets exist; graph, child, and reopen guards remain mandatory. `DispatchEnabled + ExecutionBound` rejects management lifecycle mutation, while a dispatch source must carry validated `{run_id, receipt_id}` and admitted proof.
- When dispatch is disabled, task closure does not read, retire, reconcile, or mutate run-graph/receipt artifacts.
- When dispatch is enabled, dispatch owns execution-driven transitions for execution-bound tasks; pure management tasks remain directly manageable.
- `dev_team.enabled: false` disables TeamFlow catalog materialization without blocking management status/doctor/init; worker execution surfaces return `team_flow_disabled`. `enabled: true` retains strict authority-catalog validation.

## CLI

- `vida taskflow dispatch status --json`
- `vida taskflow dispatch adopt --dry-run [--run-id <id>] [--task-id <id>] --json`
- `vida taskflow dispatch adopt --apply --run-id <id> --task-id <id> --json`
- Existing scheduler, consume, and run-graph mutation commands remain compatibility aliases and fail closed with `dispatch_runtime_disabled` when disabled.

Adoption is explicit and idempotent. Existing runs are not auto-adopted when dispatch is enabled. `adopt` validates the actual task/run binding and a non-empty dispatch receipt identity; mismatched pairs and missing receipts fail closed.

## Validation

Management-only create/update/link/reparent/close paths must work without workers. Dispatch-enabled execution-bound transitions require dispatch policy and receipt evidence. Status surfaces must report management and dispatch posture separately.

-----
artifact_path: product/spec/taskflow-management-dispatch-runtime-split
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-07-31'
schema_version: '1'
status: canonical
source_path: docs/product/spec/taskflow-management-dispatch-runtime-split.md
created_at: '2026-07-31T00:00:00+03:00'
updated_at: '2026-07-31T00:00:00+03:00'
changelog_ref: taskflow-management-dispatch-runtime-split.changelog.jsonl
