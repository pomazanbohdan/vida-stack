# Vida Stack v0.9.3

This release hardens the Codex App and internal-agent configuration path for the 0.9 transition line.

## Changes

1. Aligns the main VIDA configuration with Codex App multi-agent materialization.
2. Adds explicit legacy Codex CLI `--enable multi_agent` launch metadata.
3. Completes the internal GPT-5.5 model-profile ladder for low, medium, high, and xhigh reasoning.
4. Adds rendered Codex App internal dispatch-alias templates for specification, implementation, coaching, verification, escalation, and execution preparation.
5. Documents the observed Codex App host-agent launch behavior and the VIDA activation-view boundary.
6. Fixes the TaskFlow runtime-bundle retrieval-trust evidence cycle by allowing bundle trust evidence to cite the latest recorded final snapshot when an admissible final snapshot does not exist yet.

## Validation

This release is valid when release packaging passes, the bounded Codex App/agent-system smoke checks pass, and the target host can execute the installed binaries.

Observed validation for the 2026-05-01 release wave:

1. GitHub `Publish Release` passed and published Linux, macOS, and Windows assets.
2. GitHub Windows installer smoke passed on `windows-latest`.
3. Local WSL proof passed:
   - `cargo test -p vida runtime_bundle_retrieval_trust_evidence -- --test-threads=1`
   - `cargo run -p vida -- taskflow consume bundle check --json`
   - `bash scripts/build-release.sh v0.9.3`
4. The local Windows host installed the `v0.9.3` release archive and an explicit installer `use -Version v0.9.3` switched `%LOCALAPPDATA%\vida-stack\current` to that release.
5. The stale `.bun\bin\vida.exe` that shadowed `vida.cmd` was moved into a backup file, so Windows command resolution now reaches the installer-managed launcher.
6. Smart App Control was disabled on the local developer host and Code Integrity policy refresh was applied; `vida --version` now reports `vida 0.9.3`, while `taskflow --help` and `docflow --help` execute.
7. GitHub `CI` still has a broad Ubuntu unit-test failure set unrelated to release asset publication; platform builds and Windows installer validation passed.

Operational note:

1. On Windows hosts with Device Guard or Smart App Control, prefer trusted code signing or a managed App Control allow policy for the `v0.9.3` binaries.
2. On developer hosts, disabling Smart App Control can unblock unsigned local binaries, but it is a host security posture change.
3. Do not treat a blocked local Windows binary as evidence that the published release archive is invalid; confirm with release workflow status and checksum first.

-----
artifact_path: install/release-notes/v0.9.3
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-05-01'
schema_version: '1'
status: canonical
source_path: install/release-notes-v0.9.3.md
created_at: '2026-05-01T00:00:00Z'
updated_at: '2026-05-01T15:20:00Z'
changelog_ref: release-notes-v0.9.3.changelog.jsonl
