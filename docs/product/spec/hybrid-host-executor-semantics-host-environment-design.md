# Hybrid Host Executor Semantics Host Environment Design

Status: implemented

Use this design to record the hybrid runtime contract now implemented in bounded dispatch and status surfaces.

## Summary
- Feature / change: keep `host_environment.cli_system` as the primary runtime posture, but allow route policy to lawfully dispatch both internal and external executor backends.
- Core rule: host selection is a posture and materialization choice, not a hard gate on executor backend class.
- Compatibility rule: `agent_system.subagents` remains the canonical executor registry; legacy route hints stay as compatibility aliases only.
- Safety rule: `internal_subagents` stays internal-only and does not acquire an external CLI dispatch contract.

## Current Context
- Host selection is already config-driven under `host_environment.cli_system` and `host_environment.systems`.
- Executor selection already has canonical registry data under `agent_system.subagents`.
- Route policy already exposes explicit backend fields such as `executor_backend`, `fanout_executor_backends`, and `fallback_executor_backend`.
- The current runtime gap is that internal host posture still acts like a hard gate in dispatch and preflight logic, so an internal host cannot yet lawfully dispatch an external backend when route policy selects one.

## Goal
- What this change should achieve
  - Make host posture and executor selection separate concerns.
  - Allow hybrid execution paths where internal and external backends are both admissible in the same runtime.
  - Keep status and preflight surfaces aligned with the actual hybrid reality.
- What success looks like
  - `host_environment.cli_system` still reports the active host posture.
  - Route policy can select internal or external backends without being blocked only by host class.
  - External backend dispatch can occur from an internal host when policy selects it.
  - `internal_subagents` remains internal-only.
- What is explicitly out of scope
  - Reworking scoring or carrier pricing policy.
  - Adding a new external dispatch adapter for internal backends.
  - Removing the existing canonical host-system registry.

## Requirements

### Functional Requirements
- `agent_system.subagents` must remain the canonical registry of executable backends.
- Host posture must be allowed to coexist with both internal and external executor paths.
- Route policy must continue to use explicit backend fields:
  - `executor_backend`
  - `fanout_executor_backends`
  - `fallback_executor_backend`
- Internal backends must remain internal-only.
- Status/preflight must report when the runtime is hybrid-aware rather than pretending external execution is impossible from an internal host.

### Non-Functional Requirements
- Clarity
  - The docs must distinguish host posture from executor selection.
- Compatibility
  - Legacy hints may remain for now, but they are not canonical.
- Safety
  - Internal backend execution must remain internal-only and fail closed when policy or config is incomplete.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/process/agent-system.md`
  - `docs/process/environments.md`
  - `docs/product/spec/hybrid-host-executor-semantics-host-environment-design.md`
- Config surfaces affected:
  - `vida.config.yaml`
- Runtime families affected:
  - `crates/vida/src/runtime_dispatch_state.rs`
  - `crates/vida/src/status_surface_external_cli.rs`
  - `crates/vida/src/status_surface_host_agents.rs`
  - `crates/vida/src/status_surface.rs`

## Design Decisions

### 1. Host posture is not a dispatch gate
Will implement / choose:
- Treat `host_environment.cli_system` as the primary runtime posture and materialization selector.
- Do not use host execution class alone as a universal blocker for executor backend dispatch.
- Why
  - Hybrid systems need the host to be allowed to host both kinds of execution rather than forcing the host to define the whole routing universe.
- Trade-offs
  - Dispatch and status logic must become backend-aware instead of relying on one host-level class check.

### 2. Route policy chooses the executor class
Will implement / choose:
- Let the route policy decide whether the active backend is internal, external, or mixed/fanout.
- Why
  - This keeps executor choice local to the route and avoids hidden host-level coupling.
- Trade-offs
  - More status/proof surfaces need to surface the effective execution class explicitly.

### 3. Internal backends stay internal-only
Will implement / choose:
- Keep `internal_subagents` internal-only with no external dispatch contract.
- Why
  - Internal execution should stay on the internal utility path.
- Trade-offs
  - Hybrid routing must still check policy and backend class carefully to avoid accidental external dispatch fallthrough.

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
- Host posture fields
  - `host_environment.cli_system`
  - `host_environment.systems.<system>.execution_class`
- Legacy compatibility fields
  - `subagents`
  - `fanout_subagents`
  - `bridge_fallback_subagent`

### Route Semantics
- Primary routes choose one explicit backend id from `agent_system.subagents`.
- Fanout routes may list both internal and external backends explicitly.
- Fallback routes may use an alternate backend class when policy escalation is required.
- Host posture influences materialization and default execution environment, but it must not veto an admissible backend selected by policy.

### Data / State Model
- Important entities
  - host posture
  - agent registry entry
  - route policy entry
  - internal executor backend
  - external executor backend
- Compatibility notes
  - `subagents` is a compatibility alias, not the source of truth.
  - `host_environment.systems.<system>.execution_class` remains a host-level posture signal, not a universal dispatch blocker.

## Bounded File Set
- `docs/product/spec/hybrid-host-executor-semantics-host-environment-design.md`
- `docs/process/agent-system.md`
- `docs/process/environments.md`

## Fail-Closed Constraints
- Forbidden fallback paths
  - No route should imply internal backend execution through external CLI dispatch.
  - No host-level posture check should silently erase an explicit backend selection from route policy.
- Required proofs
  - The docs must distinguish host posture from executor selection.
  - The runtime proofs must show internal-host-to-external-backend and external-host-to-internal-backend dispatch are both admissible when policy allows them.

## Implementation Plan

### Phase 1
- Update the design/spec and process docs to make hybrid semantics explicit.
- Keep canonical registry and route policy terminology stable.

### Phase 2
- Teach runtime dispatch and preflight/status surfaces to use hybrid-aware semantics instead of a hard host-class gate.

## Implementation Outcome
- Runtime dispatch now allows internal-host-to-external-backend execution when route policy explicitly selects an enabled external backend.
- Internal hosts still default to `vida agent-init` when no explicit external backend is selected.
- External hosts still keep policy-selected internal backends on `vida agent-init`.
- Status/preflight now surface `hybrid_external_cli_relevant` and treat enabled external backends as operationally relevant even when the selected host posture is internal.

## Validation / Proof
- `vida docflow check --root . docs/product/spec/hybrid-host-executor-semantics-host-environment-design.md`
- Runtime proof targets:
  - internal host dispatching an external backend when route policy selects it
  - external host dispatching `internal_subagents` when route policy selects it
  - status/preflight reporting hybrid-aware execution posture

## Rollout Strategy
- Development rollout
  - land the docs/spec clarification first, then apply runtime dispatch and status changes under the new packet
- Compatibility notes
  - keep legacy route hints during transition
  - do not remove internal-only guarantees for `internal_subagents`

## References
- Related specs
  - `docs/product/spec/config-driven-host-system-runtime-keep-design.md`
  - `docs/product/spec/explicit-policy-selected-internal-backend-execut-design.md`
- Related protocols
  - `docs/process/agent-system.md`
  - `docs/process/environments.md`
- Related ADRs
  - none
- External references
  - none

-----
artifact_path: product/spec/hybrid-host-executor-semantics-host-environment-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-04-09'
schema_version: '1'
status: canonical
source_path: docs/product/spec/hybrid-host-executor-semantics-host-environment-design.md
created_at: '2026-04-09T05:42:06.076300966Z'
updated_at: 2026-04-09T05:49:49.39642382Z
changelog_ref: hybrid-host-executor-semantics-host-environment-design.changelog.jsonl
