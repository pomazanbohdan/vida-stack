# Core Agent-System Runtime Capsule

Purpose: provide a compact runtime-facing projection of the highest-frequency agent-system routing and worker-first law for routine orchestrator startup and continuation.

Boundary rule:

1. this file is a compact projection, not the owner of agent-system law,
2. the canonical owner remains `instruction-contracts/core.agent-system-protocol`,
3. use the owner file when an edge case, conflict, or uncommon routing condition is not resolved by this capsule.

## Always-Keep-Visible

1. active agent-system mode,
2. whether worker-first posture is required,
3. whether the root session still remains `orchestrator`,
4. chosen orchestration pattern,
5. effective write scope and verification posture,
6. whether delegated lane / handoff / saturation state is still open.

## High-Frequency Laws

1. mode selection does not convert the root session into an implementer by default,
2. `native` and `hybrid` preserve orchestrator-first control with delegated execution as the normal posture,
3. route closure is incomplete while verification posture, route control limits, orchestration pattern, or selection basis remain implicit,
4. route closure is incomplete while the root session is being treated as the writer without explicit exception-path or supersession evidence,
5. a read-only discovery lane finding a bounded gap does not authorize root-session writing by itself,
6. an open delegated lane or unresolved handoff for the same packet blocks root-session takeover unless explicit supersession or hard-blocker evidence exists,
7. unresolved or invalid project extensions fail closed rather than silently degrading into ad hoc routing,
8. when the mode is not `disabled`, absence of an immediate worker allocation does not by itself authorize local-only continuation.

## Saturation Recovery

1. classify current delegated lanes as:
   - `active`
   - `waiting`
   - `completed_unsynthesized`
   - `superseded`
   - `blocked`
2. synthesize or supersede completed returns before declaring saturation,
3. reclaim only lanes that are completed and no longer needed for active handoff or verification state,
4. prefer lawful reuse of an existing eligible lane before any exception-path reasoning,
5. only after inventory, reclamation, and failed lawful reuse may saturation remain the active explanation for escalation or exception-path evaluation.

## Escalate To Owner File When

1. backend-class routing or mode selection is disputed,
2. route receipt fields or control limits are incomplete,
3. project extension validity changes admissibility,
4. verification-lane independence or backend fallback legality is unclear,
5. a framework/protocol mutation may change generic agent-system law itself.

-----
artifact_path: config/instructions/instruction-contracts/core.agent-system-runtime-capsule
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/core.agent-system-runtime-capsule.md
created_at: '2026-03-13T23:55:00+02:00'
updated_at: '2026-03-13T23:55:00+02:00'
changelog_ref: core.agent-system-runtime-capsule.changelog.jsonl
