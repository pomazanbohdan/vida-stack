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

1. `vida/config/instructions/instruction-contracts.worker-dispatch-protocol.md`
2. `vida/config/instructions/instruction-contracts.agent-system-protocol.md`

-----
artifact_path: config/instructions/agent-backends/matrix
artifact_type: agent_backend_instruction
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/agent-backends.matrix.md
created_at: 2026-03-09T22:51:59+02:00
updated_at: 2026-03-10T00:55:00+02:00
changelog_ref: agent-backends.matrix.changelog.jsonl
