# Explicit Policy Selected Internal Backend Execution Design

Status: implemented

Use this design to record the canonical agent registry and explicit route policy contract that now back the runtime selection logic.

## Summary
- Feature / change: keep `host_environment.cli_system` independent from `agent_system.subagents`, treat `agent_system.subagents` as the canonical registry for all executor backends, and make routes point to explicit executor backend fields rather than vague `subagents` hints.
- Core rule: `internal_subagents` stays internal-only and is selected by policy, not by changing the active host system.
- Compatibility rule: legacy `subagents` / `fanout_subagents` / `bridge_fallback_subagent` entries may remain as shims until runtime code is updated, but they are not the canonical contract.

## Current Context
- Host system selection is already config-driven under `host_environment.cli_system` and `host_environment.systems`.
- The current config still mixes canonical registry data with route hints.
- Route blocks currently use `subagents`, `fanout_subagents`, and `bridge_fallback_subagent` as execution hints.
- `internal_subagents` already exists in `agent_system.subagents` with `subagent_backend_class: internal`.
- External CLI subagents remain separate backend entries with explicit dispatch contracts.

## Goal
- What this change should achieve
  - Make `agent_system.subagents` the only canonical registry for executor backends.
  - Make route-level execution policy explicit and self-describing.
  - Keep internal backend execution available without turning the active host system into an internal host.
- What success looks like
  - Every route can state its primary executor backend explicitly.
  - Fanout routes can state their backend set explicitly.
  - Fallback/escalation routes can state their fallback backend explicitly.
  - `internal_subagents` remains internal-only and does not acquire an external CLI dispatch contract.
- What is explicitly out of scope
  - Changing the active host system from its current external CLI selection.
  - Adding a new external dispatch adapter for internal subagents.
  - Reworking runtime scoring or carrier pricing policy.

## Requirements

### Functional Requirements
- `agent_system.subagents` must remain the canonical registry of executable backends.
- Routes must point to explicit executor backend fields:
  - `executor_backend`
  - `fanout_executor_backends`
  - `fallback_executor_backend`
- Internal backend selection must be policy-based and route-specific.
- Legacy route hint fields may remain during transition, but they must be treated as compatibility aliases.
- Route policy must continue to support profiles such as `profiles.internal_subagents: internal_fast`.

### Non-Functional Requirements
- Clarity
  - Config should make backend intent obvious without requiring runtime inference from vague names.
- Compatibility
  - Existing route consumers must remain readable while the runtime code catches up.
- Safety
  - Internal executor selection must not imply external CLI execution.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/process/agent-system.md`
  - `docs/process/environments.md`
  - `docs/product/spec/explicit-policy-selected-internal-backend-execut-design.md`
- Config surfaces affected:
  - `vida.config.yaml`
- Runtime families affected:
  - `crates/vida/src/taskflow_routing.rs`
  - `crates/vida/src/runtime_lane_summary.rs`
  - `crates/vida/src/runtime_dispatch_state.rs`
  - `crates/vida/src/main.rs`

## Design Decisions

### 1. `agent_system.subagents` is the canonical executor registry
Will implement / choose:
- Keep all executors, including `internal_subagents`, in `agent_system.subagents`.
- Why
  - This avoids splitting the registry between host selection and route policy.
- Trade-offs
  - Route policy must refer to backend ids explicitly instead of relying on ambiguous hints.

### 2. Routes declare executor intent explicitly
Will implement / choose:
- Add explicit route fields:
  - `executor_backend` for the primary backend.
  - `fanout_executor_backends` for multi-backend fanout.
  - `fallback_executor_backend` for policy fallback.
- Why
  - Route intent becomes readable and machine-parseable without overloading `subagents`.
- Trade-offs
  - Legacy fields stay temporarily to avoid breaking current runtime consumers.

### 3. Internal backend stays internal-only
Will implement / choose:
- Keep `internal_subagents` with `subagent_backend_class: internal`.
- Do not add a `dispatch` contract for internal backends.
- Why
  - Internal backends should be executed by the internal utility path, not an external CLI bridge.
- Trade-offs
  - Runtime selection has to preserve legacy hints during the transition window.

## Technical Design

### Config Shape
- Canonical registry
  - `agent_system.subagents.internal_subagents`
  - `agent_system.subagents.qwen_cli`
  - `agent_system.subagents.hermes_cli`
  - `agent_system.subagents.opencode_cli`
- Canonical route fields
  - `executor_backend: <backend_id>`
  - `fanout_executor_backends: [<backend_id>, ...]`
  - `fallback_executor_backend: <backend_id>`
- Legacy compatibility fields
  - `subagents`
  - `fanout_subagents`
  - `bridge_fallback_subagent`

### Route Semantics
- Primary routes choose one explicit backend id from `agent_system.subagents`.
- Fanout routes list all eligible backend ids explicitly.
- Fallback routes list the backend id to use when policy escalation is required.
- Profiles stay orthogonal and can continue to map a backend to a profile name such as `internal_fast`.

### Data / State Model
- Important entities
  - agent registry entry
  - route policy entry
  - internal executor backend
  - external CLI executor backend
- Compatibility notes
  - `subagents` is a compatibility alias, not the source of truth for new documentation.
  - Existing consumers may still read legacy fields, but runtime selection now prefers explicit executor backend fields first.

## Bounded File Set
- `docs/product/spec/explicit-policy-selected-internal-backend-execut-design.md`
- `vida.config.yaml`
- `docs/process/agent-system.md`
- `docs/process/environments.md`
- `crates/vida/src/taskflow_routing.rs`
- `crates/vida/src/runtime_lane_summary.rs`
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/main.rs`

## Fail-Closed Constraints
- Forbidden fallback paths
  - No route should imply internal backend execution through external CLI dispatch.
  - No active host switch should be required to use `internal_subagents`.
- Required proofs
  - The config must show explicit executor backend fields in route blocks.
  - The docs must describe `agent_system.subagents` as the canonical registry.

## Implementation Plan

### Phase 1
- Update the design/spec and process docs to make the registry/policy split explicit.
- Add explicit executor backend fields to the config routes while keeping compatibility aliases.

### Phase 2
- Teach routing to consume the explicit executor fields first and keep legacy route hints as compatibility fallback.
- Keep policy-selected `internal_subagents` on `vida agent-init` even when the active host system remains external.

## Validation / Proof
- `vida docflow check --root . docs/product/spec/explicit-policy-selected-internal-backend-execut-design.md`
- Config review:
  - confirm route blocks carry `executor_backend` / `fanout_executor_backends` / `fallback_executor_backend`
  - confirm internal backend remains under `agent_system.subagents`
- Rust proof:
  - `cargo test -p vida explicit_executor_backend -- --nocapture`
  - `cargo test -p vida runtime_agent_lane_dispatch_keeps_policy_selected_internal_backend_on_agent_init -- --nocapture`

## Rollout Strategy
- Development rollout
  - land explicit config terminology and runtime consumption in one bounded packet
- Compatibility notes
  - keep legacy route fields during the transition window
  - avoid changing the active host system for this policy change

## References
- Related specs
  - `docs/product/spec/config-driven-host-system-runtime-keep-design.md`
- Related protocols
  - `docs/process/agent-system.md`
  - `docs/process/environments.md`
- Related ADRs
  - none
- External references
  - none

-----
artifact_path: product/spec/explicit-policy-selected-internal-backend-execut-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-04-09'
schema_version: '1'
status: canonical
source_path: docs/product/spec/explicit-policy-selected-internal-backend-execut-design.md
created_at: '2026-04-08T21:55:36.892136577Z'
updated_at: 2026-04-08T22:06:57.906378163Z
changelog_ref: explicit-policy-selected-internal-backend-execut-design.changelog.jsonl
