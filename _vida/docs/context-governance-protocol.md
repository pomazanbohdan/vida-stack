# Context Governance Protocol (CGP)

Purpose: define one canonical framework-owned ledger for context source classes, provenance, freshness, and role-scoped usage.

## Core Contract

1. Context must be classified before it becomes execution evidence.
2. Every governed context source must declare:
   - `source_class`
   - `path`
   - `freshness`
   - `provenance`
   - `role_scope`
3. The canonical source classes are:
   - `local_repo`
   - `local_runtime`
   - `overlay_declared`
   - `web_validated`
   - `external_connector`

## Canonical Artifact

1. `.vida/state/context-governance.json`

## Freshness Rules

1. `local_repo` and `local_runtime` default to `current` unless a producing runtime marks them otherwise.
2. `web_validated` must be marked `validated` or `current`.
3. `overlay_declared` is reserved for project-declared context resolved by the active overlay.
4. `external_connector` is reserved for connector-backed context outside the local repo/runtime boundary.

## Runtime Integration Rules

1. `prepare-execution` should record a context-governance summary for the input artifacts it consumes.
2. Operator surfaces may summarize counts by source class, freshness, and recent governed task usage, but the ledger remains canonical.
3. Missing context-source classification is a governance gap, not silent approval to treat all evidence equally.

## Commands

```bash
python3 _vida/scripts/context-governance.py record --task-id <task_id> --phase <phase> --sources-json '[...]'
python3 _vida/scripts/context-governance.py validate --sources-json '[...]'
python3 _vida/scripts/context-governance.py status
```
