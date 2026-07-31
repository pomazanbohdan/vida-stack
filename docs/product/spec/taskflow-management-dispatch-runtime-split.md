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
- Execution-bound tasks are identified by explicit execution plan metadata or an active run; management-only tasks do not require dispatch evidence to close.
- When dispatch is disabled, task closure does not read, retire, reconcile, or mutate run-graph/receipt artifacts.
- When dispatch is enabled, dispatch owns execution-driven transitions for execution-bound tasks; pure management tasks remain directly manageable.

## CLI

- `vida taskflow dispatch status --json`
- `vida taskflow dispatch adopt --dry-run [--run-id <id>] [--task-id <id>] --json`
- `vida taskflow dispatch adopt --apply --run-id <id> --task-id <id> --json`
- Existing scheduler, consume, and run-graph mutation commands remain compatibility aliases and fail closed with `dispatch_runtime_disabled` when disabled.

Adoption is explicit and idempotent. Existing runs are not auto-adopted when dispatch is enabled.

## Validation

Management-only create/update/link/reparent/close paths must work without workers. Dispatch-enabled execution-bound transitions require dispatch policy and receipt evidence. Status surfaces must report management and dispatch posture separately.
