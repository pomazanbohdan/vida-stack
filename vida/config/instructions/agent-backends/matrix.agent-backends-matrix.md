# Agent Backend Matrix

Purpose: define generic routing classes for agent backends.

This file is the canonical agent-backend matrix.

## Backend Classes

| Agent Backend Class | Best Use | Write Mode | Notes |
|---|---|---|---|
| `internal` | Default framework-native implementation backend class | ✅ | Runtime-managed inside the current platform |
| `external_cli` | Cheap or specialized CLI-driven execution backend class | ✅ with scoped ownership | Must never own workflow state |
| `external_review` | Independent review or validation backend class | Read-only by default | Prefer when separation of judgement matters |

## Routing Rules

1. Backend selection must come from active agent-system state, not from hardcoded framework docs.
2. Project overlay may map task classes to backend order and optional backend-specific model/profile policy.
3. Use stronger or promoted backends for architecture/high-risk tasks.
4. Use cheap or review backends only when bounded scope and verification contract are explicit.

Reference:

1. `instruction-contracts/lane.worker-dispatch-protocol`
2. `instruction-contracts/core.agent-system-protocol`

-----
artifact_path: config/instructions/agent-backends/matrix.agent-backends.matrix
artifact_type: agent_backend_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/agent-backends/matrix.agent-backends-matrix.md
created_at: '2026-03-09T22:51:59+02:00'
updated_at: '2026-03-11T12:33:34+02:00'
changelog_ref: matrix.agent-backends-matrix.changelog.jsonl
