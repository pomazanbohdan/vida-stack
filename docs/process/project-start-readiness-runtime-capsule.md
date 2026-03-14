# Project Start Readiness Runtime Capsule

Status: active project process doc

Purpose: provide a compact runtime-facing projection of project startup readiness, combining the high-frequency skill-activation and boot-readiness rules for routine orchestrator sessions.

## Use

Use this capsule during routine startup and resume when the orchestrator needs the minimum readiness gate without rereading the full owner protocols.

Owner law remains in:

1. `docs/process/project-skill-initialization-and-activation-protocol.md`
2. `docs/process/project-boot-readiness-validation-protocol.md`
3. `docs/process/project-orchestrator-session-start-protocol.md`

Consult those owner documents when a startup edge case, validation conflict, or readiness audit is not settled by this capsule.

## Minimum Startup Checks

Before the first write-producing dispatch:

1. run `vida status --json`,
2. run `vida status --json`,
3. confirm the active read set is visible,
4. inspect the visible skill catalog,
5. activate the minimal relevant skill set or state `no_applicable_skill`,
6. run `vida docflow protocol-coverage-check --profile active-canon`.

## Boot-Ready Conditions

A session is startup-ready only when all are true:

1. TaskFlow resolves to this repository root,
2. lifecycle truth is `.vida/state/taskflow-state.db`,
3. no active path depends on installed shims or legacy task artifacts,
4. the queue is launch-ready at lawful `delivery_task` depth,
5. relevant skills are explicit or `no_applicable_skill` is explicit,
6. the session can name request class, active bounded unit, next leaf, next route, and proof target.

## Fail-Closed Summary

Do not dispatch when any of these is true:

1. runtime root/state is ambiguous,
2. startup cannot name the active bounded unit,
3. skill activation is still unknown,
4. the queue is still epic/milestone shaped or over-split speculatively,
5. protocol coverage fails.

## Routing

1. for the full startup checklist, read `docs/process/project-orchestrator-session-start-protocol.md`,
2. for full skill-activation law, read `docs/process/project-skill-initialization-and-activation-protocol.md`,
3. for full boot-readiness validation law, read `docs/process/project-boot-readiness-validation-protocol.md`.

-----
artifact_path: process/project-start-readiness-runtime-capsule
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/process/project-start-readiness-runtime-capsule.md
created_at: '2026-03-13T18:05:15+02:00'
updated_at: '2026-03-13T18:05:15+02:00'
changelog_ref: project-start-readiness-runtime-capsule.changelog.jsonl
