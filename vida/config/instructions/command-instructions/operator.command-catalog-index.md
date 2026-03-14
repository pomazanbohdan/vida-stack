# VIDA Commands (Slim Runtime)

Purpose: compact command map aligned with DB-backed task runtime + packs + protocol architecture.

## Canonical Entry Points

1. `/vida-bug-fix`
   - unified bug-fix flow (single or batch).
2. `/vida-research`
   - external research (BA layer).
3. `/vida-spec`
   - technical contract formation (SA layer, SCP).
4. `/vida-form-task`
   - bridge from spec to ready task pool + launch gate.
5. `/vida-implement`
   - unified development execution (IEP protocol).
6. `/vida-status`
   - read-only status dashboard from the DB-backed task runtime.

Thinking is protocol-driven (no separate slash command):

7. `instruction-contracts/overlay.step-thinking-protocol`
   - mandatory step-thinking router (STC/PR-CoT/MAR/5-SOL/META).
8. `instruction-contracts/overlay.session-context-continuity-protocol`
   - cross-step continuity layer for invariants, scope, and session carry-forward.

## Core Flow

```text
/vida-research -> /vida-spec -> /vida-form-task -> /vida-implement
```

Rules:

1. `/vida-implement` starts only after explicit launch confirmation in `/vida-form-task`.
2. `vida taskflow task` is the only task-state source of truth.
3. TaskFlow board is execution visibility only.
4. Project analyze/scan/test/triage behaviors are absorbed into `/vida-status`, `/vida-implement`, and `/vida-bug-fix` protocols.

## Command Layer Matrix

Use one shared command-layer taxonomy for all canonical command surfaces:

1. `CL1 Intake`
2. `CL2 Reality And Inputs`
3. `CL3 Contract And Decisions`
4. `CL4 Materialization`
5. `CL5 Gates And Handoff`

Canonical source:

1. `command-instructions/routing.command-layer-protocol`

## Protocol Links

1. Step thinking: `instruction-contracts/overlay.step-thinking-protocol`
2. Session continuity: `instruction-contracts/overlay.session-context-continuity-protocol`
3. Command layers: `command-instructions/routing.command-layer-protocol`
4. Bug-fix: `command-instructions/execution.bug-fix-protocol`
5. Web/internet validation: `runtime-instructions/work.web-validation-protocol`
6. Spec contract: `runtime-instructions/work.spec-contract-protocol`
7. Form-task bridge: `command-instructions/planning.form-task-protocol`
8. Implement execution: `command-instructions/execution.implement-execution-protocol`
9. Pack routing: `command-instructions/routing.use-case-packs-protocol`
10. Task-state protocol: `runtime-instructions/runtime.task-state-telemetry-protocol`
11. Framework topology map: `system-maps/framework.map`

-----
artifact_path: config/command-instructions/commands.index
artifact_type: command_instruction
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/command-instructions/operator.command-catalog-index.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-11T13:24:55+02:00'
changelog_ref: operator.command-catalog-index.changelog.jsonl
