# 🌌 Vida Stack v0.2.1

## ✨ Hotfix Highlights

- runtime-only release archive for the active `0.2.x` proving line
- clean packaged `AGENTS.sidecar.md` scaffold for external project owners
- installer-management boundary moved out of the versioned archive
- installer Python environment now materializes directly from the shipped `codex-v0` runtime subtree
- CI package checks now fail closed if `docs/`, `crates/`, `install/`, `scripts/`, or `taskflow-v0/` source surfaces leak into the public archive

## 📌 Release Positioning

`v0.2.1` is a hotfix on the active `0.2.x` semantic-freeze and proving line.

It does not change the architectural target:

- `taskflow-v0` remains the tracked-execution proof runtime
- `codex-v0` remains the documentation and inventory proof runtime
- Rust `taskflow` / `docflow` remain active parallel implementation tracks for the next release line

## 🛠️ What Changed

- the public archive now ships only install-ready runtime surfaces and direct dependencies
- the archive no longer carries repository-development ballast such as `docs/**`, `crates/**`, `install/**`, `scripts/**`, or the `taskflow-v0/**` source subtree
- the installed `vida` launcher now routes `doctor`, `upgrade`, `install`, and `use` through an installer-management surface outside the versioned archive
- installed `vida docflow` is now documented as an explicit compatibility boundary: `help|overview only`
- the packaged `AGENTS.sidecar.md` is now a clean scaffold meant to be filled by the external project owner

## ✅ Practical Outcome

With `v0.2.1`, users can:

- install the current proving runtime into a clean project directory
- receive only the shipped runtime payload and framework substrate
- keep installer lifecycle management separate from the versioned runtime archive
- start from a blank project sidecar instead of a `vida-stack`-specific sidecar

## 🔭 Direction

`v0.2.1` tightens the public proving surface without changing the path to `VIDA 1.0`:

- the active proving line stays `0.2.x`
- the public proof runtimes stay `taskflow-v0` and `codex-v0`
- the next architectural step is still the compiled Rust runtime line, not a larger `0.2.x` payload

-----
artifact_path: install/release-notes/v0.2.1
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: install/release-notes-v0.2.1.md
created_at: '2026-03-12T14:10:00+02:00'
updated_at: '2026-03-12T16:37:07+02:00'
changelog_ref: release-notes-v0.2.1.changelog.jsonl
