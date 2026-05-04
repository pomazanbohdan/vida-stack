# Vida Stack v0.9.4

This patch release fixes installed Windows bootstrap and command resolution after the `v0.9.3` line.

## Highlights

- Fixed installed `vida orchestrator-init --json` failing before lane snapshot generation when instruction source ingest resolved `framework-source` from a CI build path instead of the active project root.
- Changed instruction and framework-memory source ingest so relative bundle roots resolve from the active VIDA project root, while absolute source roots still work for explicit proof runs.
- Updated the Windows installer to expose direct runtime binaries from `%LOCALAPPDATA%\vida-stack\current\bin` instead of generated `.cmd` launcher wrappers.
- Added regression coverage for project-root relative source ingest, absolute source ingest, idempotent instruction ingest, and boot smoke behavior.

## Validation

Observed local validation for the 2026-05-04 release wave:

1. `cargo test -p vida ingest_relative_source_root_resolves_from_active_project_root`
2. `cargo test -p vida ingest_accepts_absolute_source_root_without_project_resolution`
3. `cargo test -p vida ingest_is_idempotent_within_same_store`
4. `cargo test -p vida --test boot_smoke boot_succeeds`
5. `cargo test -p vida --test boot_smoke boot_is_idempotent_for_unchanged_source_trees`
6. Installed `vida orchestrator-init --json` resolves the active executable as `%LOCALAPPDATA%\vida-stack\current\bin\vida.exe`, reports runtime path status `pass`, and reaches `ready_enough_for_normal_work`.

## Operator Notes

1. New Windows shells should resolve `vida`, `taskflow`, and `docflow` directly to `.exe` files under `%LOCALAPPDATA%\vida-stack\current\bin`.
2. Existing shells may need to be restarted to pick up the updated User PATH.
3. Legacy generated `.cmd` wrappers are treated as cleanup targets, not as the canonical installed command surface.

-----
artifact_path: install/release-notes/v0.9.4
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-05-04'
schema_version: '1'
status: canonical
source_path: install/release-notes-v0.9.4.md
created_at: '2026-05-04T00:00:00Z'
updated_at: '2026-05-04T00:00:00Z'
changelog_ref: release-notes-v0.9.4.changelog.jsonl
