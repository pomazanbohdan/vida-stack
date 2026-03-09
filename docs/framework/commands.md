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

7. `docs/framework/thinking-protocol.md`
   - mandatory algorithm router (STC/PR-CoT/MAR/5-SOL/META).

## Core Flow

```text
/vida-research -> /vida-spec -> /vida-form-task -> /vida-implement
```

Rules:

1. `/vida-implement` starts only after explicit launch confirmation in `/vida-form-task`.
2. `br` is the only task-state source of truth.
3. TODO board is execution visibility only.
4. Project analyze/scan/test/triage behaviors are absorbed into `/vida-status`, `/vida-implement`, and `/vida-bug-fix` protocols.

## Command Layer Matrix

Use one shared command-layer taxonomy for all canonical command surfaces:

1. `CL1 Intake`
2. `CL2 Reality And Inputs`
3. `CL3 Contract And Decisions`
4. `CL4 Materialization`
5. `CL5 Gates And Handoff`

Canonical source:

1. `docs/framework/command-layer-protocol.md`

## Protocol Links

1. Thinking: `docs/framework/thinking-protocol.md`
2. Command layers: `docs/framework/command-layer-protocol.md`
3. Bug-fix: `docs/framework/bug-fix-protocol.md`
4. Web/internet validation: `docs/framework/web-validation-protocol.md`
5. Spec contract: `docs/framework/spec-contract-protocol.md`
6. Form-task bridge: `docs/framework/form-task-protocol.md`
7. Implement execution: `docs/framework/implement-execution-protocol.md`
8. Pack routing: `docs/framework/use-case-packs.md`
9. Task-state protocol: `docs/framework/beads-protocol.md`
10. Framework topology map: `docs/framework/framework-map-protocol.md`
