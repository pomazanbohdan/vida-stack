# Project Orchestrator Session Start Protocol

Status: active project process doc

Purpose: define the repeatable start sequence for a project development orchestrator session so each new or resumed session can bootstrap, inspect the current state, and enter lawful packet shaping without relying on task-specific chat history.

## Scope

This protocol defines:

1. the minimum start checklist for a development orchestrator session,
2. the required runtime checks and binding outputs before write-producing work,
3. the compact read path for project-side launch readiness.

## Session Start Checklist

Run this checklist in order:

1. confirm project-local runtime path with `vida status --json`,
2. confirm current taskflow state with `vida orchestrator-init --json`,
3. read framework bootstrap carriers:
   - `AGENTS.md`
   - `AGENTS.sidecar.md`
   - `vida/root-map.md`
4. read `docs/process/project-orchestrator-startup-bundle.md`
5. read active product control maps needed by the current work line:
   - `docs/product/spec/release-1-program-map.md` when Release 1 is active
   - `docs/product/spec/release-1-restart-backlog.md` when restart execution is active
   - `docs/product/spec/release-1-seam-map.md` when closure, handoff, or hardening is active

## Runtime Preconditions

Before shaping work, all of the following must be true:

1. `vida status --json` resolves to this repository root,
2. TaskFlow lifecycle truth is `.vida/state/taskflow-state.db`,
3. installed shim roots are not the active TaskFlow runtime path,
4. the runtime is not relying on `.beads/issues.jsonl` or other legacy task artifacts.

If any of those fail, fix runtime bootstrap first and do not start orchestration work yet.

## Minimum Session Outputs

After the checklist, the orchestrator must be able to state explicitly:

1. whether the session is `answer_only`, `artifact_flow`, `execution_flow`, or `mixed`,
2. which backlog unit or bounded ask is active,
3. whether the next lawful leaf is `delivery_task` or `execution_block`,
4. whether the next step is local shaping, delegated implementation, proof-only verification, or escalation,
5. which proof target closes the next packet,
6. which relevant skills are active for the next bounded step, or that none apply,
7. which packet template and prompt-stack layers are active for the next bounded step.
8. that the root session remains `orchestrator` unless an explicit exception path authorizes local writing.
9. which lawful next slices currently exist and whether the next move is sequential or parallel-safe.
10. when the user asked for both reporting/diagnosis and continued development, which path is primary: `diagnosis_path` or `normal_delivery_path`.
11. when continuation is requested, why this bounded unit rather than some other ready candidate is the lawful current binding.

If any of those are missing, the session is not ready for write-producing work.

Task-binding rule:

1. `continue development` must bind to an explicit active task line or bounded packet before any local repair, proof, or validation step begins,
2. the session is not launch-ready if the root session can name only a local symptom such as a failing test or compile error but cannot name the active parent bounded unit,
3. do not treat a locally visible failing test as a replacement for the active packet unless canonical task/packet evidence already makes it the lawful current leaf.
4. if the user asks to continue "the next task" without naming it, the session is not launch-ready until one of these is true:
   - one uniquely evidenced active bounded unit can be bound and stated explicitly,
   - one uniquely evidenced continuation receipt names the next lawful unit,
   - or the user confirms which bounded unit is meant.
5. do not silently bind "the next task" to `ready_head[0]` or the first canonical backlog candidate merely because TaskFlow ordering makes that choice plausible.

Launch-readiness gate:

1. the runtime-visible orchestrator control loop must be inspectable immediately after boot,
2. lawful-next selection and sequential-vs-parallel-safe posture must be explicit,
3. after bootstrap, `continue development` means resume orchestrator-led tracked execution rather than local-first implementation,
4. local root-session writing remains invalid unless an explicit exception path is active and no still-lawful open delegated cycle blocks takeover,
5. the session is launch-ready at lawful `delivery_task` depth; backlog-wide pre-splitting into `execution_block` is not required.

## First Packet Rule

The first write-producing action of a session must not be:

1. broad repository exploration,
2. ad hoc implementation,
3. milestone-level delegation,
4. local coding before one lawful packet exists.

The first write-producing action must be:

1. one lawful packet shape,
2. one chosen lane sequence,
3. one clear proof target.
4. if the root session is the writer, one explicit exception-path receipt plus proof that no still-lawful delegated cycle for the same packet remains open.

Backlog-wide pre-splitting into `execution_block` before first dispatch is forbidden by default.

## Minimal Shell Commands

Use these commands as the canonical session-start smoke path:

```bash
vida status --json
vida orchestrator-init --json
```

Optional next-step inspection when active work already exists:

```bash
vida taskflow task list --all --json
```

## Routing

1. for the compact project startup read set, read `docs/process/project-orchestrator-startup-bundle.md`,
2. for upper-lane logic, read `docs/process/project-orchestrator-operating-protocol.md`,
3. for reusable session wording, read `docs/process/project-orchestrator-reusable-prompt.md`,
4. for full packet-template law, read `docs/process/project-development-packet-template-protocol.md`,
5. for full skill-activation law, read `docs/process/project-skill-initialization-and-activation-protocol.md`,
6. for full boot-readiness validation, read `docs/process/project-boot-readiness-validation-protocol.md`,
7. for full prompt-stack law, read `docs/process/project-agent-prompt-stack-protocol.md`,
8. for full delegated-lane law and packet closure edge cases, read `docs/process/team-development-and-orchestration-protocol.md`.

-----
artifact_path: process/project-orchestrator-session-start-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/process/project-orchestrator-session-start-protocol.md
created_at: '2026-03-13T18:55:00+02:00'
updated_at: '2026-03-13T21:35:00+02:00'
changelog_ref: project-orchestrator-session-start-protocol.changelog.jsonl
