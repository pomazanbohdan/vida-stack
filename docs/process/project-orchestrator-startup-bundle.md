# Project Orchestrator Startup Bundle

Status: active project process doc

Purpose: provide one compact project-side startup bundle for routine orchestrator sessions, aggregating the current always-read project control surfaces without replaying every owner document separately.

## Use

Use this bundle after framework bootstrap when the project orchestrator needs the minimum project read set for routine startup, resume, or cheaper orchestration.

This bundle is a routing and compression surface only.
It does not own protocol law.

Owner law remains in:

1. `docs/process/project-orchestrator-operating-protocol.md`
2. `docs/process/project-orchestrator-session-start-protocol.md`
3. `docs/process/project-packet-and-lane-runtime-capsule.md`
4. `docs/process/project-start-readiness-runtime-capsule.md`
5. `docs/process/project-packet-rendering-runtime-capsule.md`

Consult those owner surfaces when an edge case, launch-readiness conflict, or routing ambiguity is not settled by this bundle.

## Bundle Contents

Treat this bundle as the compact project `always_on_core` startup set for routine development orchestration:

1. top-level project routing and anti-stop narrowing from `project-orchestrator-operating-protocol.md`,
2. packet and delegated-lane defaults from `project-packet-and-lane-runtime-capsule.md`,
3. startup readiness and skill gating from `project-start-readiness-runtime-capsule.md`,
4. packet rendering and prompt-stack interpretation from `project-packet-rendering-runtime-capsule.md`.

## Runtime Summary

After reading this bundle, the orchestrator should be able to answer:

1. which bounded unit is active or why it is still ambiguous,
2. whether the next leaf is `delivery_task` or `execution_block`,
3. whether the next move is shape, delegate, verify, or escalate,
4. which proof target closes the next packet,
5. whether startup readiness and skill activation are already explicit,
6. whether a full owner protocol read is required for an edge case.

## Expansion Rule

Use the bundle by default for routine startup.

Expand beyond it only when:

1. the session-start checklist itself is being audited or changed,
2. launch readiness is blocked on an owner-level validation conflict,
3. delegated-lane closure or exception-path law is ambiguous,
4. packet-template or prompt-stack edge cases are not settled by the rendering capsule,
5. the user explicitly asks for the deeper owner protocol.

## Routing

1. for the full session-start checklist, read `docs/process/project-orchestrator-session-start-protocol.md`,
2. for top-level routing and project anti-stop narrowing, read `docs/process/project-orchestrator-operating-protocol.md`,
3. for packet/lane defaults, read `docs/process/project-packet-and-lane-runtime-capsule.md`,
4. for startup readiness and skill gating, read `docs/process/project-start-readiness-runtime-capsule.md`,
5. for packet rendering and prompt-stack law, read `docs/process/project-packet-rendering-runtime-capsule.md`.

-----
artifact_path: process/project-orchestrator-startup-bundle
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/process/project-orchestrator-startup-bundle.md
created_at: '2026-03-13T18:05:15+02:00'
updated_at: '2026-03-13T18:05:15+02:00'
changelog_ref: project-orchestrator-startup-bundle.changelog.jsonl
