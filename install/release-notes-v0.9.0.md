# 🌌 Vida Stack v0.9.0

## ✨ Release Highlights

- `v0.9.0` is a major transition milestone toward `1.0.0`
- legacy execution paths were removed from the active release surface in favor of one Rust-first runtime direction
- core operator tools, `TaskFlow` and `DocFlow`, are now positioned as the main Rust runtime surfaces
- role/skill/profile/flow mechanisms are operational in the runtime and aligned with project topology contracts
- core-system behavior and master documentation were synchronized against the active specifications

## 📌 Release Positioning

`v0.9.0` is the big stabilization jump before final `1.0.0` closure.

This release establishes one coherent operating baseline:

- `vida taskflow`
- `vida docflow`
- top-level `vida` launcher for install/bootstrap/runtime routing

## 🛠️ What Changed

- cleaned up legacy release/runtime assumptions and aligned execution around the current Rust runtime core
- strengthened runtime initialization and state consistency so operator flows are stable in clean and upgraded environments
- finalized current-state master documentation (`README`, version plan, runtime/reality status docs) to remove historical drift
- validated installer and upgrade flow as a first-class release path for practical operator use

## ✅ Practical Outcome

With `v0.9.0`, teams get:

- a clearer and more predictable VIDA runtime baseline
- practical day-to-day execution through `TaskFlow` and `DocFlow` without legacy split-brain surfaces
- a documented, validated transition platform for final `1.0.0` product closure

## 🧪 Proof Snapshot

Validation for this release is green through:

- release build and packaged installer checks
- isolated install + upgrade runtime smoke (`install -> upgrade -> boot/status/doctor`)
- full command-surface smoke in isolated install roots (`95/95` checks green)
- bounded runtime and documentation checks over the active master surfaces

## 🔭 Direction

`v0.9.0` confirms the core-system jump: legacy cleanup, Rust-first runtime baseline, and specification-aligned execution posture.

The next phase is `1.0.0` finalization on top of this stabilized core.

-----
artifact_path: install/release-notes/v0.9.0
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-15'
schema_version: '1'
status: canonical
source_path: install/release-notes-v0.9.0.md
created_at: '2026-03-15T09:08:41+02:00'
updated_at: '2026-03-15T09:25:35+02:00'
changelog_ref: release-notes-v0.9.0.changelog.jsonl
