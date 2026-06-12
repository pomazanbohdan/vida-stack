# Project Start Readiness Runtime Capsule

Status: compatibility projection

Purpose: keep the runtime-facing startup-readiness path stable while avoiding a second checklist beside the current orchestrator startup owner documents.

## Compatibility Boundary

This file is retained because runtime and instruction indexes may still resolve this path directly. It is not a separate owner of startup, skill, boot-readiness, or dispatch law.

Canonical owner surfaces:

1. `docs/process/project-orchestrator-startup-bundle.md`
   - routine compact startup read set
2. `docs/process/project-orchestrator-session-start-protocol.md`
   - full repeatable session-start and boot-readiness checklist
3. `docs/process/project-skill-initialization-and-activation-protocol.md`
   - skill-catalog inspection and activation rule

## Runtime Projection

For routine startup, load the startup bundle first. Expand to the full session-start protocol only when the bundle does not settle active bounded unit, startup readiness, skill activation, proof target, or sequential-vs-parallel posture.

For skill-only uncertainty, load the skill-initialization protocol directly. Do not copy its rule text here.

For boot-readiness uncertainty, load the session-start protocol. Do not maintain a parallel boot-readiness checklist in this capsule.

## Fail-Closed Summary

Dispatch is not ready when any required startup field is missing, ambiguous, or stale relative to current runtime evidence:

1. active bounded unit,
2. reason this unit is current,
3. next lawful route,
4. proof target,
5. relevant skill posture,
6. sequential-vs-parallel posture.

Resolve those fields through the owner surfaces above before write-producing work.

-----
artifact_path: process/project-start-readiness-runtime-capsule
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: compatibility_projection
source_path: docs/process/project-start-readiness-runtime-capsule.md
created_at: '2026-03-13T18:05:15+02:00'
updated_at: 2026-06-13T00:00:00+03:00
changelog_ref: project-start-readiness-runtime-capsule.changelog.jsonl
