# Decisions

Initial activation decisions:

- project id: `vida-test`
- host CLI system: selected through `vida project-activator` (current: `codex`, internal agent-first mode)
- current Codex release posture: `v0.9.3` is published and the Windows installer has switched `%LOCALAPPDATA%\vida-stack\current` to `v0.9.3`; the observed Windows developer host disabled Smart App Control and now executes `vida 0.9.3` through installer-managed launchers
- local proof posture for blocked Windows hosts: use WSL/Linux or GitHub Actions for Rust proof and release packaging when Windows blocks Cargo build scripts or freshly installed `.exe` files
- language policy:
  - user communication: `english`
  - reasoning: `english`
  - documentation: `english`
  - todo protocol: `english`

-----
artifact_path: process/decisions
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-05-01'
schema_version: '1'
status: canonical
source_path: docs/process/decisions.md
created_at: '2026-04-04T20:24:09+03:00'
updated_at: '2026-05-01T15:20:00Z'
changelog_ref: decisions.changelog.jsonl
