# VIDA 0.3 Project Memory Sync Spec

Purpose: define project memory as a bidirectional sync/export surface for VIDA-managed projects rather than a boot-ingest source for the VIDA framework runtime.

Status: canonical `0.3` spec artifact for the direct `1.0` program.

Date: 2026-03-08

---

## 1. Executive Decision

`project_memory` is not a mandatory boot-ingest source for the VIDA framework runtime.

Instead it is:

1. a project-facing memory/documentation surface,
2. a bidirectional sync boundary,
3. an export/importable markdown projection for projects managed with VIDA.

Compact rule:

`project_memory syncs with VIDA-owned state; it is not a required boot substrate for the framework itself`

---

## 2. Runtime Rule

1. VIDA runtime may own project-memory state in the DB.
2. Project markdown/docs may be synchronized to or from that DB-owned state.
3. Boot for the VIDA framework must not fail because project-memory source trees are absent.

---

## 3. Export / Sync Rule

Project memory supports:

1. export from DB to markdown/docs,
2. import/sync from project markdown/docs to DB by explicit sync flow,
3. bidirectional reconciliation under explicit sync rules.

---

## 4. Non-Goal

This spec does not define the full bidirectional merge algorithm yet.
-----
artifact_path: framework/plans/vida-0.3-project-memory-sync-spec
artifact_type: plan
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/plans/vida-0.3-project-memory-sync-spec.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: vida-0.3-project-memory-sync-spec.changelog.jsonl
P26-03-09T21: 44:13Z
