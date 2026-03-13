# Project Overlay Runtime Capsule

Purpose: provide a compact runtime-facing projection of project overlay behavior for routine bootstrap and execution.

Boundary rule:

1. this file is a compact projection, not the owner of project-overlay law,
2. the canonical owner remains `vida/config/instructions/runtime-instructions/bridge.project-overlay-protocol.md`,
3. consult the owner file for schema details, unsupported-key checks, validation edge cases, or overlay-governance changes.

## Runtime Role

1. `vida.config.yaml` is optional project-owned root data,
2. overlay may activate framework bundles and project routing posture,
3. overlay must not weaken framework invariants,
4. framework-owned behavior remains in `AGENTS.md`, `vida/config/instructions/*`, and `taskflow-v0/*`.

## Routine Boot Rules

1. detect `vida.config.yaml` after core bootstrap is available,
2. parse and schema-validate before binding runtime behavior,
3. activate only overlay-approved protocol domains admitted by activation law,
4. if overlay is absent, continue with framework-only behavior,
5. if overlay is invalid, fail closed before write-producing execution.

## High-Frequency Runtime Fields

For routine orchestration, the main overlay questions are:

1. language policy affecting user/project communication,
2. whether `protocol_activation.agent_system=true`,
3. whether `framework_self_diagnosis` is enabled,
4. whether `autonomous_execution` changes reporting/continuation behavior,
5. whether `agent_extensions` are enabled and require registry-aware routing.

## Runtime Consequences

1. no overlay:
   - use framework-owned commands and wrappers only,
   - do not assume project operations docs are canonical.
2. valid overlay:
   - apply only the activated runtime posture,
   - keep project-specific docs/ops as project-owned narrowing,
   - do not treat overlay data as permission to bypass verification, DB-backed task truth, or framework safety rules.

## Escalate To Owner Protocol When

1. schema or key support is in question,
2. worker/route validation semantics are disputed,
3. a new overlay section or key is being introduced,
4. project activation, agent-system validation, or overlay portability law is being changed.

-----
artifact_path: config/runtime-instructions/project-overlay-runtime-capsule
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/bridge.project-overlay-runtime-capsule.md
created_at: '2026-03-13T22:40:00+02:00'
updated_at: '2026-03-13T22:40:00+02:00'
changelog_ref: bridge.project-overlay-runtime-capsule.changelog.jsonl
