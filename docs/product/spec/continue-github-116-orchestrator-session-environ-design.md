# Continue GitHub #116 Orchestrator Session Environment Design Gate

Status: `approved`

## Summary
- Feature / change: tracked TaskFlow design-gate wrapper for GitHub #116 orchestrator session/environment identity work.
- Owner layer: `runtime-family`
- Runtime surface: `taskflow | docflow | orchestrator-init | status`
- Canonical design: `docs/product/spec/orchestrator-session-environment-identity-design.md`
- Status: `approved for work-pool handoff`

## Design Gate Decision
- Use `docs/product/spec/orchestrator-session-environment-identity-design.md` as the canonical design for the bounded #116 implementation wave.
- Treat this file as the tracked bootstrap design gate expected by the run-graph lifecycle for `github-116-orchestrator-session-identity`.
- Keep Phase 1 limited to additive session/lease identity reporting, heartbeat persistence, status/init projection, legacy-owner defaults, and bounded local proof.

## Proof Evidence
- `cargo fmt -p vida -- --check`
- `vida docflow check-file --path docs/product/spec/orchestrator-session-environment-identity-design.md`
- `cargo test -p vida orchestrator_session_identity -- --nocapture`
- `cargo test -p vida orchestrator_init_json_payload_exposes_session_identity -- --nocapture`
- `cargo test -p vida orchestrator_session_heartbeat_record_round_trips_from_state_store -- --nocapture`
- `cargo test -p vida orchestrator_session_records_default_empty_for_legacy_state_without_table -- --nocapture`
- `cargo test -p vida runtime_owner_evidence_defaults_legacy_ownerless_rows -- --nocapture`
- `target\debug\vida.exe orchestrator-init --json`
- `target\debug\vida.exe status --json`
- `cargo build -p vida --release`
- Installed `vida.exe` updated at `C:\Users\pomaz\AppData\Local\vida-stack\current\bin\vida.exe`

## Handoff
- Close the tracked spec task after this design gate is docflow-checked.
- Continue with the tracked work-pool handoff only after `pending_design_finalize` and `pending_spec_task_close` are clear.

-----
artifact_path: product/spec/continue-github-116-orchestrator-session-environ-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-05-07
schema_version: 1
status: canonical
source_path: docs/product/spec/continue-github-116-orchestrator-session-environ-design.md
created_at: 2026-05-07T00:00:00+03:00
updated_at: 2026-05-07T00:00:00+03:00
changelog_ref: continue-github-116-orchestrator-session-environ-design.changelog.jsonl
