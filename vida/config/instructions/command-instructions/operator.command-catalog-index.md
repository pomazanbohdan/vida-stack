# VIDA Commands (Slim Runtime)

Purpose: compact command map aligned with `br + packs + protocol` architecture.

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
   - read-only status dashboard from `br`.

Thinking is protocol-driven (no separate slash command):

7. `vida/config/instructions/instruction-contracts/overlay.step-thinking-protocol.md`
   - mandatory step-thinking router (STC/PR-CoT/MAR/5-SOL/META).
8. `vida/config/instructions/instruction-contracts/overlay.session-context-continuity-protocol.md`
   - cross-step continuity layer for invariants, scope, and session carry-forward.

## Core Flow

```text
/vida-research -> /vida-spec -> /vida-form-task -> /vida-implement
```

Rules:

1. `/vida-implement` starts only after explicit launch confirmation in `/vida-form-task`.
2. `br` is the only task-state source of truth.
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

1. `vida/config/instructions/command-instructions/routing.command-layer-protocol.md`

## Protocol Links

1. Step thinking: `vida/config/instructions/instruction-contracts/overlay.step-thinking-protocol.md`
2. Session continuity: `vida/config/instructions/instruction-contracts/overlay.session-context-continuity-protocol.md`
3. Command layers: `vida/config/instructions/command-instructions/routing.command-layer-protocol.md`
4. Bug-fix: `vida/config/instructions/command-instructions/execution.bug-fix-protocol.md`
5. Web/internet validation: `vida/config/instructions/runtime-instructions/work.web-validation-protocol.md`
6. Spec contract: `vida/config/instructions/runtime-instructions/work.spec-contract-protocol.md`
7. Form-task bridge: `vida/config/instructions/command-instructions/planning.form-task-protocol.md`
8. Implement execution: `vida/config/instructions/command-instructions/execution.implement-execution-protocol.md`
9. Pack routing: `vida/config/instructions/command-instructions/routing.use-case-packs-protocol.md`
10. Task-state protocol: `vida/config/instructions/runtime-instructions/runtime.task-state-telemetry-protocol.md`
11. Framework topology map: `vida/config/instructions/system-maps/framework.map.md`

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
