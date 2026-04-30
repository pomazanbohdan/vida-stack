# Vida Stack v0.9.1

## Release Highlights

- `v0.9.1` hardens the transition line established in `v0.9.0`
- standalone `taskflow` and `docflow` binaries are now release-packaged beside the top-level `vida` launcher
- Linux, macOS, and Windows release assets are built through the GitHub release pipeline
- Windows installer parity is now first-class, including `vida`, `taskflow`, and `docflow` launcher shims
- runtime dispatch, lane recovery, scheduler, continuation, and state-store surfaces received release-line hardening

## Release Positioning

`v0.9.1` is a packaging and operator-readiness release on the path to final `1.0.0` closure.

It keeps the same Rust-first runtime direction as `v0.9.0`, while making the installable release more practical across supported operator environments.

## What Changed

- added standalone `TaskFlow` and `DocFlow` CLI packaging with release-manifest coverage
- added Windows PowerShell installer support and Windows installer smoke coverage
- expanded release publication automation for Linux, macOS arm64, and Windows x86_64 assets
- strengthened installer behavior around packaged templates, launcher wrappers, checksums, and local archive installs
- hardened runtime surfaces around delegated dispatch, continuation binding, exception takeover, recovery, scheduler execution, and command-surface truth
- moved more runtime logic out of the monolithic launcher path into focused owner modules with broader smoke and contract coverage
- aligned role, carrier, model-profile, and development-team runtime projections with config-owned authority

## Practical Outcome

With `v0.9.1`, operators can install a release archive and get:

- `vida`
- `taskflow`
- `docflow`

as executable command surfaces from the packaged release, without compiling Rust locally.

The release also gives Windows operators the same installer path expected on the rest of the supported release line.

## Proof Snapshot

Validation for this release is expected through:

- full workspace tests with locked dependencies
- release build and runtime-only package checks
- Linux installer smoke from packaged archive
- Windows installer smoke from packaged zip, including `vida`, `taskflow`, and `docflow`
- GitHub Actions release asset publication for `v0.9.1`

## Direction

`v0.9.1` closes the release-packaging and cross-platform installer gap after the `v0.9.0` transition baseline.

The next phase remains final `1.0.0` closure on top of the Rust-first runtime, standalone TaskFlow/DocFlow surfaces, and verified installer path.

-----
artifact_path: install/release-notes/v0.9.1
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-04-30'
schema_version: '1'
status: canonical
source_path: install/release-notes-v0.9.1.md
created_at: '2026-04-30T12:00:00+02:00'
updated_at: '2026-04-30T12:00:00+02:00'
changelog_ref: release-notes-v0.9.1.changelog.jsonl
