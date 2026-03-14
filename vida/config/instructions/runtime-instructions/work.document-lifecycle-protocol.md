# Document Lifecycle Protocol (DLP)

Purpose: define canonical lifecycle states and freshness validation for framework-owned VIDA documents.

## Core Contract

1. Framework-owned documentation should carry explicit lifecycle state, not implicit “probably current” status.
2. The canonical lifecycle states are:
   - `proposed`
   - `current`
   - `superseded`
   - `deprecated`
   - `stale`
3. Freshness is separate from lifecycle:
   - a document may still be `current` but become stale if it has not been re-reviewed within the expected window.

## Canonical Artifact

1. `.vida/state/doc-lifecycle.json`

Minimum entry fields:

1. `path`
2. `state`
3. `owner`
4. `notes`
5. `last_reviewed_at`

## Commands

```bash
python3 doc-lifecycle.py record <path> <proposed|current|superseded|deprecated|stale> --owner "<owner>" --notes "<notes>"
python3 doc-lifecycle.py validate <path> [--max-age-days N]
python3 doc-lifecycle.py status
```

## Validation Rules

1. Missing document-state entry is a lifecycle gap.
2. Missing `last_reviewed_at` is invalid.
3. `current` documents older than the accepted freshness window should be treated as `stale_document` until re-reviewed.

## Scope Rule

1. This protocol governs framework-owned documents first (`AGENTS.md`, `vida/config/instructions/**`, synced framework templates/overlays when applicable).
2. Project-owned `docs/*` may adopt the same model later, but that does not change framework ownership boundaries.

-----
artifact_path: config/runtime-instructions/document-lifecycle.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.document-lifecycle-protocol.md
created_at: '2026-03-07T22:08:06+02:00'
updated_at: '2026-03-11T13:02:59+02:00'
changelog_ref: work.document-lifecycle-protocol.changelog.jsonl
