# Project Orchestrator Session Start Protocol

Status: active project process doc

Purpose: define the repeatable start sequence for a project development orchestrator session so each new or resumed session can bootstrap, inspect the current state, and enter lawful packet shaping without relying on task-specific chat history.

## Scope

This protocol defines:

1. the minimum start checklist for a development orchestrator session,
2. the required runtime checks,
3. the required read path after bootstrap,
4. the minimum session outputs before packet shaping begins.

This protocol does not define:

1. one specific backlog item,
2. one specific packet,
3. lower-level worker packet semantics,
4. product capability law itself.

## Session Start Checklist

Run this checklist in order:

1. confirm project-local runtime path with `taskflow-v0 status`,
2. confirm current taskflow state with `taskflow-v0 boot snapshot --json --top-limit 5 --ready-limit 5`,
3. read framework bootstrap carriers:
   - `AGENTS.md`
   - `AGENTS.sidecar.md`
   - `vida/root-map.md`
4. read project upper-lane operating docs:
   - `docs/process/project-orchestrator-operating-protocol.md`
   - `docs/process/team-development-and-orchestration-protocol.md`
   - `docs/process/codex-agent-configuration-guide.md`
5. read and apply `docs/process/project-skill-initialization-and-activation-protocol.md`, inspect the active available skill catalog, and activate the minimal relevant skill set or state `no_applicable_skill`
6. read active product control maps needed by the current work line:
   - `docs/product/spec/release-1-program-map.md` when Release 1 is active
   - `docs/product/spec/release-1-restart-backlog.md` when restart execution is active
   - `docs/product/spec/release-1-seam-map.md` when closure, handoff, or hardening is active

## Runtime Preconditions

Before shaping work, all of the following must be true:

1. `taskflow-v0 status` resolves to this repository root,
2. TaskFlow lifecycle truth is `.vida/state/taskflow-state.db`,
3. installed shim roots are not the active `taskflow-v0` path,
4. the runtime is not relying on `.beads/issues.jsonl` or other legacy task artifacts.

If any of those fail, fix runtime bootstrap first and do not start orchestration work yet.

## Minimum Session Outputs

After the checklist, the orchestrator must be able to state explicitly:

1. whether the session is `answer_only`, `artifact_flow`, `execution_flow`, or `mixed`,
2. which backlog unit or bounded ask is active,
3. whether the next lawful leaf is `delivery_task` or `execution_block`,
4. whether the next step is local shaping, delegated implementation, proof-only verification, or escalation,
5. which proof target closes the next packet,
6. which relevant skills are active for the next bounded step, or that none apply.

If any of those are missing, the session is not yet ready for write-producing work.

Launch-readiness clarification:

1. the session is launch-ready when the active queue is lawfully shaped to `delivery_task`,
2. the session does not require the whole backlog to be pre-split into `execution_block`,
3. `execution_block` refinement is required only for the next active item or a near-critical-path item that still fails one-owner bounded closure.

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

Backlog-wide pre-splitting into `execution_block` before first dispatch is forbidden by default.

## Minimal Shell Commands

Use these commands as the canonical session-start smoke path:

```bash
taskflow-v0 status
taskflow-v0 boot snapshot --json --top-limit 5 --ready-limit 5
```

Optional next-step inspection when active work already exists:

```bash
taskflow-v0 task list --all --json
```

## Routing

1. for upper-lane logic, read `docs/process/project-orchestrator-operating-protocol.md`,
2. for packet semantics, read `docs/process/team-development-and-orchestration-protocol.md`,
3. for Codex lane/runtime posture, read `docs/process/codex-agent-configuration-guide.md`,
4. for reusable session wording, read `docs/process/project-orchestrator-reusable-prompt.md`,
5. for mandatory skill activation, read `docs/process/project-skill-initialization-and-activation-protocol.md`.

-----
artifact_path: process/project-orchestrator-session-start-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/process/project-orchestrator-session-start-protocol.md
created_at: '2026-03-13T18:55:00+02:00'
updated_at: '2026-03-13T19:11:00+02:00'
changelog_ref: project-orchestrator-session-start-protocol.changelog.jsonl
