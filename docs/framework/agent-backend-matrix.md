# Agent Backend Matrix

Purpose: define generic routing classes for agent backends.

This file is the canonical agent-backend matrix.

## Backend Classes

| Agent Backend Class | Best Use | Write Mode | Notes |
|---|---|---|---|
| `internal` | Default framework-native implementation lane | ✅ | Runtime-managed inside the current platform |
| `external_cli` | Cheap or specialized CLI-driven execution lane | ✅ with scoped ownership | Must never own workflow state |
| `external_review` | Independent review or validation lane | Read-only by default | Prefer when separation of judgement matters |

## Routing Rules

1. Backend selection must come from active agent-system state, not from hardcoded framework docs.
2. Project overlay may map task classes to backend order and optional backend-specific model/profile policy.
3. Use stronger or promoted backends for architecture/high-risk tasks.
4. Use cheap or review backends only when bounded scope and verification contract are explicit.

Reference:

1. `docs/framework/worker-dispatch-protocol.md`
2. `docs/framework/agent-system-protocol.md`
