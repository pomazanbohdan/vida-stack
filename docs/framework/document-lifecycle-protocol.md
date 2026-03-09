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
python3 docs/framework/history/_vida-source/scripts/doc-lifecycle.py record <path> <proposed|current|superseded|deprecated|stale> --owner "<owner>" --notes "<notes>"
python3 docs/framework/history/_vida-source/scripts/doc-lifecycle.py validate <path> [--max-age-days N]
python3 docs/framework/history/_vida-source/scripts/doc-lifecycle.py status
```

## Validation Rules

1. Missing document-state entry is a lifecycle gap.
2. Missing `last_reviewed_at` is invalid.
3. `current` documents older than the accepted freshness window should be treated as `stale_document` until re-reviewed.

## Scope Rule

1. This protocol governs framework-owned documents first (`AGENTS.md`, `docs/framework/*`, synced framework templates/overlays when applicable).
2. Project-owned `docs/*` may adopt the same model later, but that does not change framework ownership boundaries.
