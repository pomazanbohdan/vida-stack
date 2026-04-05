# Clarify Enforce Immediate Continuation Shell Saf Design

Status: approved

Use this template for one bounded feature/change design before implementation.

## Summary
- Feature / change: clarify and enforce immediate continuation after non-gating reports, and add a shell-safe path for backlog progress recording.
- Owner layer: mixed
- Runtime surface: launcher | taskflow | project process docs
- Status: approved

## Current Context
- Existing system overview
  - Project canon already says continued-development sessions must not stop after commentary, summaries, green tests, or other non-gating intermediate reports when the next lawful item is already known.
  - Runtime packet prompts already warn that commentary is not a lawful pause boundary.
  - `vida task update` currently accepts free-text notes only through inline `--notes`, which makes shell quoting correctness an operator burden.
- Key components and relationships
  - `AGENTS.md`, `docs/process/project-orchestrator-operating-protocol.md`, `docs/process/project-packet-and-lane-runtime-capsule.md`, `docs/process/project-operations.md`, and `docs/process/codex-agent-configuration-guide.md` define continuation behavior for the active project.
  - `crates/vida/src/main.rs` emits the runtime packet prompt used by delegated lanes.
  - `crates/vida/src/init_surfaces.rs` renders scaffolded project instructions visible to host agents after activation.
  - `crates/vida/src/cli.rs`, `crates/vida/src/task_surface.rs`, and `crates/vida/src/taskflow_layer4.rs` define the task update contract and operator-facing help/query guidance.
- Current pain point or gap
  - The continuation law is strong but still too easy to violate because not every active instruction surface states the recovery action after an accidental closure-style report.
  - Shell-safe backlog progress recording is not encoded into the CLI contract, so a user or agent can corrupt notes by mixing shell metacharacters with inline `--notes`.

## Goal
- What this change should achieve
  - Make the immediate recovery rule explicit in the active project instruction surfaces and runtime prompts.
  - Add a shell-safe CLI path for task progress notes.
  - Shift active recommendations/help/query output toward the shell-safe path.
- What success looks like
  - Active process docs and runtime prompts explicitly say to re-enter commentary mode and bind the next lawful continuation item after an accidental closure-style report.
  - `vida task update` supports `--notes-file <path>`.
  - Operator help/query surfaces recommend `--notes-file` for progress recording.
  - Targeted task/proxy/query tests remain green.
- What is explicitly out of scope
  - broad redesign of TaskFlow continuation semantics
  - changing backlog state ownership
  - redesign of non-task mutation surfaces

## Requirements

### Functional Requirements
- Must preserve the existing anti-stop law.
- Must make the recovery action explicit after an accidental closure-style report during active continuation intent.
- Must add a shell-safe task-update note input path.
- Must fail closed when conflicting note sources are supplied.
- Must update active help/query/instruction surfaces to recommend the shell-safe path.

### Non-Functional Requirements
- Performance
  - negligible overhead for optional file-backed note reads
- Observability
  - targeted tests must pin the new prompt/help behavior
- Security
  - no silent fallback when both inline and file-backed notes are supplied

## Ownership And Canonical Surfaces
- Project docs / specs affected
  - `docs/process/project-orchestrator-operating-protocol.md`
  - `docs/process/project-packet-and-lane-runtime-capsule.md`
  - `docs/process/project-operations.md`
  - `docs/process/codex-agent-configuration-guide.md`
  - `docs/product/spec/clarify-enforce-immediate-continuation-shell-saf-design.md`
- Runtime/code surfaces affected
  - `crates/vida/src/cli.rs`
  - `crates/vida/src/task_surface.rs`
  - `crates/vida/src/taskflow_layer4.rs`
  - `crates/vida/src/main.rs`
  - `crates/vida/src/init_surfaces.rs`
  - `crates/vida/tests/boot_smoke.rs`
  - `crates/vida/tests/task_smoke.rs`

## Design Decisions

### 1. Continuation law will name the recovery action, not just the prohibition
Will implement / choose
- Add wording that a mistaken closure-style report must be followed immediately by commentary-mode continuation binding.
- Why
  - The observed failure was not lack of law; it was lack of a strongly repeated recovery action in the active instruction path.

### 2. Shell-safe note recording will be part of the CLI contract
Will implement / choose
- Add `vida task update --notes-file <path>` and reject simultaneous `--notes` plus `--notes-file`.
- Why
  - This removes shell command substitution and quoting complexity from the normal progress-recording path for complex text.

## Bounded File Set
- `docs/product/spec/clarify-enforce-immediate-continuation-shell-saf-design.md`
- `docs/process/project-orchestrator-operating-protocol.md`
- `docs/process/project-packet-and-lane-runtime-capsule.md`
- `docs/process/project-operations.md`
- `docs/process/codex-agent-configuration-guide.md`
- `crates/vida/src/cli.rs`
- `crates/vida/src/task_surface.rs`
- `crates/vida/src/taskflow_layer4.rs`
- `crates/vida/src/main.rs`
- `crates/vida/src/init_surfaces.rs`
- `crates/vida/tests/boot_smoke.rs`
- `crates/vida/tests/task_smoke.rs`

## Implementation Plan

### Phase 1
- Fill the bounded design document and update the active process/instruction surfaces.
- Proof target
  - `vida docflow check --root . docs/product/spec/clarify-enforce-immediate-continuation-shell-saf-design.md`

### Phase 2
- Add `--notes-file` handling and conflict validation.
- Update help/query/runtime prompts to recommend the shell-safe path and explicit continuation recovery.
- Proof target
  - targeted `cargo test -p vida ...` for task/proxy/query coverage

### Phase 3
- Run release build.
- Refresh the installed/system `vida` binary from the release artifact.
- Commit and push the bounded change set.
- Proof target
  - release build plus post-install smoke check

## Validation / Proof
- `vida docflow check --root . docs/product/spec/clarify-enforce-immediate-continuation-shell-saf-design.md`
- `cargo test -p vida taskflow_proxy_ -- --nocapture`
- `cargo test -p vida taskflow_query_ -- --nocapture`
- `cargo test -p vida taskflow_task_ -- --nocapture`
- `cargo test -p vida task_update_accepts_notes_file_for_shell_safe_progress_recording -- --nocapture`
- `cargo test -p vida task_update_rejects_notes_and_notes_file_together -- --nocapture`
- `cargo build --release -p vida`

## Rollout Strategy
- Update docs and runtime prompts first so the intended behavior is visible immediately.
- Land the shell-safe CLI path in the same bounded change.
- Reinstall the release binary after tests pass.

-----
artifact_path: product/spec/clarify-enforce-immediate-continuation-shell-saf-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-05
schema_version: 1
status: canonical
source_path: docs/product/spec/clarify-enforce-immediate-continuation-shell-saf-design.md
created_at: 2026-04-05T08:13:30.81251267Z
updated_at: 2026-04-05T08:22:09.424116052Z
changelog_ref: clarify-enforce-immediate-continuation-shell-saf-design.changelog.jsonl
