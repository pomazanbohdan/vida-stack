# Spec Freshness Protocol

Purpose: define how VIDA resolves conflicts or ambiguity between older and newer architectural/spec decisions.

## Core Contract

1. When multiple canonical artifacts govern the same behavior, prefer the fresher authoritative decision.
2. Freshness must be evaluated from explicit artifact dates and materially newer updates, not chat memory.
3. Newer artifacts may refine or replace older ones within the same ownership layer and scope.

## Resolution Rule

When two canonical artifacts overlap:

1. compare ownership layer,
2. compare scope specificity,
3. compare artifact date / revision freshness,
4. prefer the newer artifact when ownership and scope are compatible.

## Update Rule

1. When architecture changes materially, update the affected spec date-bearing artifact.
2. If an older artifact remains partially valid, narrow it explicitly instead of leaving silent contradiction.
3. If no nearby artifact can be safely updated, add a newer canonical artifact and link it in the protocol index.

## Fail-Closed Rule

1. Do not assume older specs remain controlling when a newer date-bearing artifact overrides them.
2. Do not leave materially conflicting canonical artifacts unresolved.

-----
artifact_path: config/runtime-instructions/spec-freshness.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.spec-freshness-protocol.md
created_at: '2026-03-09T12:00:46+02:00'
updated_at: '2026-03-11T13:04:01+02:00'
changelog_ref: work.spec-freshness-protocol.changelog.jsonl
