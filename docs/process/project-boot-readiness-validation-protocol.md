# Project Boot Readiness Validation Protocol

Status: active project process doc

Purpose: define the bounded validation sequence that proves a project development orchestrator session is boot-ready before the first write-producing dispatch.

## Scope

This protocol defines:

1. the canonical boot-readiness checks,
2. what counts as a passing session bootstrap,
3. what must fail closed before work starts,
4. the minimum proof commands for runtime and protocol visibility.

This protocol does not define:

1. one specific backlog item,
2. one specific packet,
3. product capability law,
4. framework-wide boot law.

## Core Rule

No write-producing development session is boot-ready until runtime, protocol visibility, and skill activation have all passed bounded validation.

## Boot-Readiness Checks

Run these checks in order:

1. runtime root check
2. taskflow state smoke check
3. bootstrap/protocol read-set check
4. skill initialization check
5. active queue/decomposition-depth check
6. protocol coverage check

## Required Commands

Use these commands as the minimum validation set:

```bash
vida status --json
vida orchestrator-init --json
vida docflow protocol-coverage-check --profile active-canon
```

Optional bounded follow-up when an active unit already exists:

```bash
vida taskflow task ready --json
vida taskflow task show <task-id> --json
```

## Passing Conditions

A session is boot-ready only when all are true:

1. `vida status --json` resolves to this repository root,
2. lifecycle truth is `.vida/state/taskflow-state.db`,
3. no active path depends on installed shim roots outside this repository,
4. no active path depends on `.beads/issues.jsonl` or other legacy task artifacts,
5. the required bootstrap/protocol read set is complete,
6. relevant skills are activated or `no_applicable_skill` is explicit,
7. the active queue is shaped to lawful `delivery_task` leaves,
8. deeper `execution_block` refinement is deferred until dispatch time unless already needed for the next active item,
9. `protocol-coverage-check` passes for the active canon.

## Fail-Closed Conditions

Do not begin write-producing work when any of these is true:

1. `vida status --json` points outside the repository root,
2. boot snapshot is unreadable or inconsistent with the expected queue,
3. the session cannot name the active process protocols,
4. the session cannot name the active skill set or `no_applicable_skill`,
5. the active queue is still only milestone/epic shaped,
6. the active queue is pre-split into broad speculative `execution_block` trees without a dispatch need,
7. protocol coverage fails.

## Minimum Read Set Proof

The session must be able to name these sources before first dispatch:

1. `AGENTS.md`
2. `AGENTS.sidecar.md`
3. `vida/root-map.md`
4. `docs/process/project-orchestrator-operating-protocol.md`
5. `docs/process/project-orchestrator-session-start-protocol.md`
6. `docs/process/project-orchestrator-startup-bundle.md`

## Session Output Proof

Before first dispatch, the session must be able to state:

1. request class,
2. active bounded unit,
3. next lawful leaf,
4. next lane mode,
5. proof target,
6. active relevant skills or `no_applicable_skill`.

If those outputs cannot be stated, the session must reshape or continue bootstrap instead of dispatching.

## Routing

1. for the compact project startup read set, read `docs/process/project-orchestrator-startup-bundle.md`,
2. for session-start sequencing, read `docs/process/project-orchestrator-session-start-protocol.md`,
3. for top-level operating rules, read `docs/process/project-orchestrator-operating-protocol.md`,
4. for full skill activation law, read `docs/process/project-skill-initialization-and-activation-protocol.md`.

-----
artifact_path: process/project-boot-readiness-validation-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/process/project-boot-readiness-validation-protocol.md
created_at: '2026-03-13T21:30:00+02:00'
updated_at: '2026-03-13T21:30:00+02:00'
changelog_ref: project-boot-readiness-validation-protocol.changelog.jsonl
