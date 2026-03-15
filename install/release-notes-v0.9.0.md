# 🌌 Vida Stack v0.9.0

## ✨ Release Highlights

- `0.9.0` is the active transition release toward `1.0.0` with runtime hardening and documentation-state alignment
- fresh-state `vida status` / `vida doctor` no longer fail on missing `run_graph_dispatch_receipt` table
- `StateStore::open_existing` now performs lock-aware retries plus schema reconciliation on open
- master docs are updated to current-state-first narrative (`README`, `VERSION-PLAN`, runtime/reality matrices)
- installer upgrade path is now verified end-to-end in isolated temp roots (`install -> vida upgrade -> runtime checks`)

## 📌 Release Positioning

`v0.9.0` is an intermediate stabilization and cleanup release before `1.0.0`.

It keeps the current public operator surfaces:

- `vida taskflow`
- `vida docflow`
- top-level `vida` launcher with install/bootstrap/runtime routing

## 🛠️ What Changed

- added missing canonical state table `run_graph_dispatch_receipt` in TaskFlow state schema defaults
- centralized `open_existing` reliability path:
  - retry on DB lock contention
  - enforce schema document reconciliation for existing state roots
- removed duplicated open-existing retry wrapper from launcher path and delegated to state-store authority
- updated project master documents to reflect active `0.9.0 -> 1.0` closure path instead of historical-line framing

## ✅ Practical Outcome

With `v0.9.0`, operators get:

- reliable `boot -> status/doctor` flow on clean state directories
- better resilience to lock races during status/doctor/runtime open paths
- clearer current-state documentation for release execution and 1.0 closure planning

## 🧪 Proof Snapshot

Bounded proof for this release slice is green through:

- `cargo test -p taskflow-state-surreal`
- `cargo test -p vida --test boot_smoke status_surface_reports_backend_and_bundle_receipt -- --exact --nocapture`
- fresh-state `vida boot`, `vida status --json`, and `vida doctor --json` checks on installed release binary
- isolated installer upgrade flow (`v0.2.2 -> v0.9.0`) through `vida upgrade --version v0.9.0 --archive ...`
- full command-surface smoke in isolated install root: `95/95` checks green (`28` root command-tree help + `67` runtime/help/exec checks)
- `vida docflow check-file` for updated master docs

## 🔭 Direction

`v0.9.0` closes high-risk runtime stability gaps but is not final `1.0.0` closure yet.

Remaining 1.0 blockers stay explicit:

- DB-first authority closure for activation entities (roles/skills/profiles/flows)
- full configurator lifecycle parity (`status/import/export/sync/reconcile/restore`) with receipts
- proof-surface command parity for all documented operator paths

-----
artifact_path: install/release-notes/v0.9.0
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-15'
schema_version: '1'
status: canonical
source_path: install/release-notes-v0.9.0.md
created_at: '2026-03-15T09:08:41+02:00'
updated_at: '2026-03-15T09:18:34+02:00'
changelog_ref: release-notes-v0.9.0.changelog.jsonl
