# Vida Stack v0.9.2

Purpose: ship the migration-gap repair wave for project sidecar semantics, activation repair, help stability, and DocFlow footer repair.

## Highlights

- `AGENTS.sidecar.md` is now the project agent-instructions overlay. The project docs map remains mandatory, but it is a section inside the sidecar rather than the sidecar's only purpose.
- Nested help paths are hardened so help output exits successfully and does not require project-root resolution.
- `vida init` and `vida project-activator --repair` now materialize ready-enough project docs/config/runtime projections with safe defaults where available.
- DocFlow adds an explicit footer repair path for legacy markdown files.
- Runtime startup bundle/capsule projection status now accepts canonical footer status without reporting the projection as blocked.
- Operator blocker output includes more concrete remediation commands for retrieval-trust, cache-key, and carrier-catalog defects.

## Verification Target

This release is valid when the release build passes, the installed system `vida` reports `0.9.2`, and the bounded help/repair smoke checks pass.

-----
artifact_path: install/release-notes/v0.9.2
artifact_type: process_doc
artifact_version: 1
artifact_revision: 2026-04-30
schema_version: 1
status: canonical
source_path: install/release-notes-v0.9.2.md
created_at: 2026-04-30T21:35:33.1587477Z
updated_at: 2026-04-30T22:15:50.7121375Z
changelog_ref: release-notes-v0.9.2.changelog.jsonl
