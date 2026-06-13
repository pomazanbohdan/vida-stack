# Codex App Agent Lifecycle Cleanup Contract

Status: active product contract

## Summary

- Contract: make Codex App agent lanes orderly by proving the configured carrier/profile ladder, keeping `agent-init` smoke runnable in debug builds, and documenting the host-agent cleanup expectation.
- Owner layer: mixed project/runtime
- Runtime surface: launcher, TaskFlow, project activation, Codex App config projection
- Status: active product contract

## Current Context

- `vida.config.yaml` is the authority for Codex carrier tiers, model profiles, reasoning effort, and runtime-role/task-class coverage.
- `.codex/config.toml` and `.codex/agents/*.toml` are materialized projections for Codex App, not authority surfaces.
- On Windows, the debug `target/debug/vida.exe agent-init --role worker --json` path can overflow the default main thread stack while rendering the large runtime bundle, while release builds already pass.
- Codex App host agents spawned for validation must be explicitly closed by the orchestrator after evidence is collected; leaving idle agents open makes the UI show stale background lanes.
- On Windows Codex App, child-agent shells may start without the same PATH/core environment as a normal terminal. Current OpenAI Codex issue evidence points to Windows process-environment inheritance, so VIDA command bootstrap must be platform-scoped instead of global.
- Codex custom-agent discovery requires each standalone `.codex/agents/*.toml` file to declare `name`, `description`, and `developer_instructions`; `[agents.<id>]` entries in `.codex/config.toml` are not enough for the spawn API to recognize a custom `agent_type`.

## Goal

- `vida agent-init --role worker --json` must not crash in debug or release builds.
- Tests must prove the configured Codex App projection exposes the model and reasoning-effort values declared in `vida.config.yaml`.
- Runtime smoke should verify explicit role activation for worker, business analyst/coach, verifier, and solution architect roles.
- Host-agent validation in this session must close every spawned agent after collecting the evidence.

Out of scope:

- Changing Codex App internals or adding a new host-agent API.
- Replacing TaskFlow/VIDA `agent-init` with host-tool-specific subagent dispatch.

## Requirements

### Functional Requirements

- Preserve `vida.config.yaml` as carrier/profile authority.
- Preserve `.codex/**` as generated/executor projection.
- Keep `agent-init` activation-view semantics non-executing unless `--execute-dispatch` is explicitly supplied.
- Ensure the launcher can render large JSON startup views without stack overflow in debug builds.

### Non-Functional Requirements

- Keep the fix localized to launcher/runtime projection code and focused tests.
- Avoid adding long-lived background agents during validation; any spawned host agents must be closed.
- Keep release behavior unchanged except for improved debug/smoke stability.
- During an active development session, validate projection and launcher fixes through `target/debug/vida.exe` from this repository and do not update the system-installed VIDA binary unless a separate release/update step is explicitly in scope.

## Ownership And Canonical Surfaces

- Project docs / specs affected: this contract, current spec map.
- Framework protocols affected: none.
- Runtime families affected: launcher `vida`, TaskFlow bundle projection.
- Config / receipts / runtime surfaces affected: `vida.config.yaml`, `.codex/config.toml`, `.codex/agents/*.toml`, `vida agent-init`, `vida taskflow consume agent-system`.

## Design Decisions

### 1. Launcher Stack Stabilization

Will implement / choose:

- Replace the `#[tokio::main]` entrypoint with a small synchronous `main` that runs the async root command inside a named thread with an explicit larger stack.
- This keeps debug and release behavior aligned without weakening runtime bundle rendering.
- Alternative considered: shrink the agent-init JSON payload. That is larger scope and risks losing operator evidence.

### 2. Config Projection Proof

Will implement / choose:

- Add host-runtime materialization tests that read `vida.config.yaml` and verify rendered `.toml` files contain the configured model and reasoning-effort values.
- This tests the projection layer directly while the runtime smoke tests prove command-level activation.
- Add platform-scoped Codex App host bootstrap config under `host_environment.systems.codex.app.platform_overrides.windows` so Windows-specific PATH/`LOCALAPPDATA` recovery does not leak into Linux or macOS projections.
- Materialization must be idempotent: repeated `target/debug/vida.exe project-activator --repair --json` runs must not duplicate bootstrap text, shell-environment tables, or alias agent files.
- Render custom-agent schema fields `name`, `description`, and `developer_instructions` into every `.codex/agents/*.toml` projection so Codex can identify custom agents by the configured name.

### 3. Host Agent Cleanup Discipline

Will implement / choose:

- Treat host-agent spawning as a validation-only action in this session and close each spawned agent immediately after result collection.
- Keep this as orchestration discipline, because Codex App agent lifecycle APIs live outside this repository.

## Technical Design

### Core Components

- `crates/vida/src/main.rs`
  - owns CLI process entry and Tokio runtime creation.
- `crates/vida/src/host_runtime_materialization.rs`
  - owns `.codex/config.toml` and `.codex/agents/*.toml` projection from configured carrier catalogs without introducing built-in model or reasoning-effort defaults.
- `vida.config.yaml`
  - owns platform-scoped Codex App environment/bootstrap overrides; Windows overrides are active only when the materializer is running on Windows.

### Data / State Model

- No new persisted runtime state.
- Existing model-profile fields remain:
  - `model`
  - `model_reasoning_effort`
  - `default_model_profile`
  - `model_profiles`

### Integration Points

- `vida agent-init --role <runtime-role> --json`
- `vida taskflow consume agent-system --json`
- Codex App `.codex/config.toml` and `.codex/agents/*.toml`
- Windows Codex App platform override:
  - `vida.config.yaml -> host_environment.systems.codex.app.platform_overrides.windows`

### Bounded File Set

- `docs/product/spec/codex-app-agent-lifecycle-cleanup-contract.md`
- `docs/product/spec/current-spec-map.md`
- `crates/vida/src/main.rs`
- `crates/vida/src/host_runtime_materialization.rs`
- `vida.config.yaml`
- `.codex/config.toml`
- `.codex/agents/*.toml`

## Fail-Closed Constraints

- Do not treat `agent-init` activation views as execution evidence.
- Do not leave spawned host agents open after validation.
- Do not make `.codex/**` the source of truth for carrier policy.
- Do not bypass TaskFlow/VIDA agent routing for normal write-producing work.
- Do not apply Windows Codex App command-resolution fallback to non-Windows host projections unless that platform later gains its own explicit override.
- Do not refresh `%LOCALAPPDATA%\vida-stack\current` or installed `v0.9.5` from a development-session patch; use the repo-local debug binary until an explicit release step.

## Implementation Plan

### Phase 1

- Keep this contract and current spec map registration aligned.
- Proof target: `vida docflow check --root . docs/product/spec/codex-app-agent-lifecycle-cleanup-contract.md`.

### Phase 2

- Stabilize debug CLI process entry with explicit stack.
- Add host-runtime projection tests for configured model/reasoning-effort parity.
- Add platform-scoped Windows Codex App command bootstrap projection and idempotency proof.
- Add custom-agent schema projection for `name` and `description`.
- Proof target: focused `cargo test -p vida ...`.

### Phase 3

- Run debug/release smoke for `agent-init` roles and `taskflow consume agent-system`.
- Spawn host agents for configured-profile validation only if needed, then close all spawned agents.

## Validation / Proof

- Unit tests:
  - `cargo test -p vida host_runtime_agent_toml`
  - `cargo test -p vida host_runtime_materialization_renders_configured_reasoning_profiles`
  - `cargo test -p vida host_runtime_materialization_renders_configured_agent_command_bootstrap`
- Runtime checks:
  - `cargo build -p vida`
  - `target/debug/vida.exe agent-init --role worker --json`
  - `target/debug/vida.exe agent-init --role business_analyst --json`
  - `target/debug/vida.exe agent-init --role verifier --json`
  - `target/debug/vida.exe agent-init --role solution_architect --json`
  - `target/debug/vida.exe taskflow consume agent-system --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/codex-app-agent-lifecycle-cleanup-contract.md`

## Observability

- Existing JSON output exposes:
  - `backend_truth.runtime_assignment.model_reasoning_effort`
  - `backend_truth.runtime_assignment.selected_model_profile_id`
  - `runtime_bundle_summary.launcher_runtime_paths`
  - `snapshot.carriers[*].model_profiles[*].reasoning_effort`

## Rollout Strategy

- Development rollout: debug build and repo-local projection proof first; system-installed production update is a separate release step and must not be performed during a debug-only development session.
- Migration / compatibility notes: no state migration required.
- Operator or user restart / restart-notice requirements: after `.codex/**` materialization changes, restart Codex App so projected agents reload.

## Future Considerations

- Add a repository-owned smoke wrapper that validates Codex App projection without spawning long-lived host agents.
- Add richer lifecycle telemetry if Codex App exposes close/reclaim events to the repository runtime.

## References

- `docs/process/codex-agent-configuration-guide.md`
- `docs/product/spec/internal-codex-agent-execution-fail-closed-contract.md`
- `docs/product/spec/carrier-model-profile-selection-runtime-model.md`

-----
artifact_path: product/spec/codex-app-agent-lifecycle-cleanup-contract
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-05-04
schema_version: 1
status: canonical
source_path: docs/product/spec/codex-app-agent-lifecycle-cleanup-contract.md
created_at: 2026-05-04T10:35:07.4486631Z
updated_at: 2026-05-04T16:05:39.092982Z
changelog_ref: codex-app-agent-lifecycle-cleanup-contract.changelog.jsonl
