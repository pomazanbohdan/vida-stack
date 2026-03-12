# Vida Stack v0.2.2

## Highlights

- script-era `taskflow-v0` protocol-binding bridge is now the primary `0.2.2` delivery slice
- protocol-binding now materializes a deterministic compiled JSON payload before DB import
- runtime work fails closed until protocol-binding state is present in `taskflow-state.db`
- `taskflow-v0 br ...` compatibility alias is removed under LEGACY-ZERO
- release archives now carry `.codex/` plus the packaged `vida.config.yaml` template asset required for installed bootstrap

## What Changed

- `taskflow-v0 protocol-binding build [--json]` now writes `taskflow-v0/generated/protocol_binding.compiled.json`
- `taskflow-v0 protocol-binding sync [--json]` imports that compiled payload into `.vida/state/taskflow-state.db`
- `taskflow-v0 protocol-binding status [--json]` and `check [--json]` query the same authoritative DB-backed state
- installed releases now scaffold `vida.config.yaml` from `install/assets/vida.config.yaml.template` when missing
- installed releases now bundle `.codex/config.toml` and `.codex/agents/*.toml`
- installer `doctor` now fails closed if the packaged config template, `.codex/`, protocol-binding payloads, or DB state are missing

## Practical Outcome

With `v0.2.2`, operators can:

- build and inspect protocol-binding JSON deterministically before import
- install the proving runtime into a clean root and get config scaffolding automatically
- rely on installed TaskFlow bootstrap to populate protocol-binding DB state without manual repair
- run the current proving runtime without the old `br` compatibility surface

## Proof Snapshot

Bounded proof for this slice is green through:

- `nimble test` in `taskflow-v0`
- `python3 -m unittest tests/test_install_docflow_bridge.py`
- repo-local `taskflow-v0 protocol-binding build|sync|check`
- local release build via `scripts/build-release.sh v0.2.2`
- temp-install archive bootstrap with `vida doctor` and installed `taskflow-v0 protocol-binding check --json`

-----
artifact_path: install/release-notes/v0.2.2
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: install/release-notes-v0.2.2.md
created_at: '2026-03-12T18:40:00+02:00'
updated_at: '2026-03-12T18:40:00+02:00'
changelog_ref: release-notes-v0.2.2.changelog.jsonl
