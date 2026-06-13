# Vida Stack v0.9.7

This patch release hardens Codex App agent lifecycle and VIDA runtime contract visibility after the `v0.9.6` cross-platform installer diagnostics wave.

## Highlights

- Added top-level `orchestrator_runtime_contract` output to `vida orchestrator-init --json`, including sticky user execution intent, allowed topology, next lawful dispatch action, and hard agent-first warnings.
- Added `dispatch_mode` to `vida agent-init --json` and `--execute-dispatch` results so activation/view-only handoffs are visibly distinct from receipt-backed execution dispatch.
- Added path-scoped exception takeover truth through `root_local_write_allowed_for_only_these_paths` in status and lane envelopes.
- Added `vida lane reclaim --completed --host-agents --json` as an idempotent cleanup surface for completed/stale VIDA-owned lane state.
- Added preview-only `parallelization_planner` and `carrier_selection_api` output to `vida agent dispatch-next --json`.
- Added first-class `vida agent select --runtime-role <role> --task-class <class> --json` carrier/model/reasoning selection from config and registries.
- Hardened `orchestrator-init` lock behavior with degraded lock-contention output instead of an opaque state-store failure.
- Fixed Windows integration-test bounded command helpers so tests do not accidentally invoke the Windows `timeout.exe` syntax.
- Kept Codex App agent model/reasoning materialization config-driven from `vida.config.yaml`; `.codex/**` remains a projection, not source of truth.

## Validation

Observed local validation for the 2026-05-04 release wave:

1. `cargo check -p vida`
2. `cargo fmt --check`
3. `cargo test -p vida host_runtime_agent_toml -- --test-threads=1`
4. `cargo test -p vida agent_init_surface_payload -- --test-threads=1`
5. `cargo test -p vida agent_dispatch_next_preview -- --test-threads=1`
6. `cargo test -p vida lane_surface -- --test-threads=1`
7. `cargo test -p vida root_session_write_guard -- --test-threads=1`
8. `cargo test -p vida --test boot_smoke agent_dispatch_next_preview_aligns_scheduler_preview_selected_lanes_and_unsafe_rejections -- --test-threads=1`
9. `target\debug\vida.exe docflow check --root . docs/product/spec/orchestrator-runtime-contract-hardening-contract.md docs/product/spec/codex-app-agent-lifecycle-cleanup-contract.md docs/process/codex-agent-configuration-guide.md`
10. `target\debug\vida.exe orchestrator-init --json`
11. `target\debug\vida.exe agent dispatch-next --json`
12. `target\debug\vida.exe agent select --runtime-role verifier --task-class verification --json`
13. `target\debug\vida.exe lane reclaim --completed --host-agents --json`
14. `target\debug\vida.exe status --summary --json`
15. `cargo build --release -p vida -p taskflow-cli -p docflow-cli`
16. `VIDA_RELEASE_SUFFIX=windows-x86_64 scripts/build-release.sh v0.9.7`
17. `pwsh -ExecutionPolicy Bypass -File dist\vida-install.ps1 install -Archive dist\vida-stack-v0.9.7-windows-x86_64.zip -Force`
18. `vida --version`, `taskflow --version`, and `docflow --version` from `C:\Users\pomaz\AppData\Local\vida-stack\current\bin`
19. Installed `vida orchestrator-init --json` verified `state_read.lock_resilient=true` and top-level `orchestrator_runtime_contract`.
20. Installed `vida agent select --runtime-role verifier --task-class verification --json` selected the config-driven `senior` carrier with `high` reasoning.
21. Installed `vida lane reclaim --completed --host-agents --json` returned `status=pass`.

## Operator Notes

1. `vida agent-init` without `--execute-dispatch` is activation/view-only context and does not prove an agent completed work.
2. Exception takeover is path-scoped. `root_local_write_allowed=true` must be read together with `root_local_write_allowed_for_only_these_paths`.
3. `vida lane reclaim --completed --host-agents` reclaims VIDA-owned runtime state. Codex App UI handles still need a host-app close API before VIDA can forcibly close them.
4. After `.codex/agents/*.toml` schema changes, restart Codex App so custom agent types are reloaded.

-----
artifact_path: install/release-notes/v0.9.7
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-05-04'
schema_version: '1'
status: canonical
source_path: install/release-notes-v0.9.7.md
created_at: '2026-05-04T00:00:00Z'
updated_at: '2026-05-04T00:00:00Z'
changelog_ref: release-notes-v0.9.7.changelog.jsonl
