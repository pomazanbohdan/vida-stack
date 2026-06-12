# Orchestrator Runtime Contract Hardening Contract

Status: `implemented`

## Summary
- Feature / change: harden VIDA orchestrator, agent-init, lane, and carrier-selection runtime contracts for Codex App agent lifecycle operation.
- Owner layer: `mixed`
- Runtime surface: `orchestrator-init | agent-init | agent dispatch-next | lane | status`
- Status: implemented

## Current Context
- `vida orchestrator-init --json` exposes activation and dev-team readiness, but does not summarize sticky execution intent, allowed topology, or the next lawful dispatch action as top-level machine-readable truth.
- `vida agent-init --json` exposes activation semantics but can still be confused with real execution dispatch unless the operator reads nested fields.
- `vida lane exception-takeover` already records `owned_write_scope`, but status/orchestrator reports do not surface the active path-scoped write boundary beside `root_local_write_allowed`.
- `vida agent dispatch-next --json` is the existing preview authority for carrier/model/cost truth and parallel lane selection, so planner and carrier-selection API data should extend that surface.
- Locked state-store reads already have retries in some paths, but `orchestrator-init` still needs degraded lock fallback instead of hard failing during app-side contention.

## Goal
- Make init/status/dispatch surfaces explicit enough that a host app or operator can distinguish activation, execution, path-scoped exception takeover, and next lawful dispatch without inferring from prose.
- Add a lane reclaim command surface for completed/stale host-agent cleanup intent.
- Keep carrier/model/reasoning selection config-driven from `vida.config.yaml` and registries, without hardcoded model or reasoning defaults.
- Out of scope: a direct Codex App API call that forcibly closes UI-visible agents, because the host app does not expose that as a stable runtime API in this repository.

## Requirements

### Functional Requirements
- `orchestrator-init --json` must expose sticky user execution intent, allowed topology, and next lawful dispatch action.
- `status --json` and lane envelopes must expose `root_local_write_allowed_for_only_these_paths` when exception takeover metadata exists.
- `agent-init --json` must expose `dispatch_mode` that distinguishes activation/view-only from execution dispatch.
- `vida lane reclaim --completed --host-agents --json` must be a callable idempotent cleanup surface.
- `vida agent dispatch-next --json` must expose a parallelization planner and first-class carrier-selection API hints.
- Lock contention in `orchestrator-init` must return degraded lock output rather than crashing without operator-readable next actions.

### Non-Functional Requirements
- Runtime JSON must remain backward compatible by adding fields, not renaming existing ones.
- Planner output must be preview-only and fail closed when packet materialization is not explicitly requested by a later runtime.
- Release proofs must cover focused unit tests, docflow validation, release build, installed binary smoke, and agent template behavior.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/orchestrator-runtime-contract-hardening-contract.md`
  - `docs/process/codex-agent-configuration-guide.md`
- Runtime families affected:
  - bootstrap init surfaces
  - lane operator surface
  - status surface
  - agent dispatch preview surface
- Config / receipts / runtime surfaces affected:
  - `vida.config.yaml`
  - `.codex/**` generated host projections
  - `.vida/data/state/lane-exception-path-metadata/**`

## Design Decisions

### 1. Add Explicit Contract Fields
Will implement:
- Add additive JSON blocks instead of changing existing output shape.
- Derive topology and carrier selection from `dev_team_readiness`, activation bundle, and runtime assignment builders.
- Emit a hard warning when sticky agent-first intent and root-local implementation could be confused.

### 2. Keep Reclaim Preview-Truthful
Will implement:
- Add `vida lane reclaim --completed --host-agents` as an idempotent cleanup/report surface.
- Reclaim completed/stale scheduler reservations and report host-agent cleanup as operator-visible intent when no host close API exists.

## Technical Design

### Core Components
- `init_surfaces.rs`: orchestrator contract block, agent-init dispatch mode, lock fallback.
- `lane_surface.rs`: reclaim command and path-scoped takeover envelope fields.
- `agent_dispatch_surface.rs`: planner and carrier-selection API output.
- `status_surface_write_guard.rs`: path-scoped root-local write guard fields.
- `cli.rs`: first-class carrier-selection subcommand.

### Data / State Model
- Path scopes are read from exception takeover metadata `owned_write_scope`.
- Planner packet proposals are preview data only and do not mutate TaskFlow state.
- Carrier selection returns selected carrier/model/reasoning/cost truth from the existing runtime assignment builder.

### Bounded File Set
- `crates/vida/src/cli.rs`
- `crates/vida/src/init_surfaces.rs`
- `crates/vida/src/lane_surface.rs`
- `crates/vida/src/agent_dispatch_surface.rs`
- `crates/vida/src/status_surface_write_guard.rs`
- `crates/vida/src/status_surface_json_report.rs`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/orchestrator-runtime-contract-hardening-contract.md`
- `docs/process/codex-agent-configuration-guide.md`

## Fail-Closed Constraints
- `activation_view_only`, `internal_activation_view_only`, and packet activation views must not count as execution evidence.
- `root_local_write_allowed=true` must be scoped by `root_local_write_allowed_for_only_these_paths` when exception takeover metadata exists.
- Planner proposals must not launch agents or write packets by themselves.

## Implementation Plan

### Phase 1
- Add docs and top-level contract fields.
- First proof target: focused init/status unit tests.

### Phase 2
- Add lane reclaim, dispatch planner, and carrier selection API.
- Second proof target: focused lane and agent dispatch tests.

### Phase 3
- Run full focused validation, release build, install via release installer, publish GitHub release notes.
- Final proof target: installed `vida --version`, runtime smoke commands, and agent template smoke.

## Validation / Proof
- Unit tests:
  - focused init surface tests
  - focused lane surface tests
  - focused agent dispatch tests
  - focused status write guard tests
- Runtime checks:
  - `target\debug\vida.exe orchestrator-init --json`
  - `target\debug\vida.exe agent dispatch-next --json`
  - `target\debug\vida.exe lane reclaim --completed --host-agents --json`
  - installed `vida --version`
- Canonical checks:
  - `target\debug\vida.exe docflow check --root . docs/product/spec/orchestrator-runtime-contract-hardening-contract.md docs/process/codex-agent-configuration-guide.md`

## Observability
- JSON fields:
  - `orchestrator_runtime_contract`
  - `dispatch_mode`
  - `root_local_write_allowed_for_only_these_paths`
  - `parallelization_planner`
  - `carrier_selection_api`
  - `state_read.mode`

## Rollout Strategy
- Develop and prove with debug binary first.
- Build release binary.
- Install through `vida release install`.
- Publish GitHub release notes for the new tag.

## Future Considerations
- If Codex App exposes a stable agent-close API later, wire `vida lane reclaim --completed --host-agents` to close UI-visible completed/stale carriers directly.
- Future planner work can materialize packet proposals after TaskFlow owns an explicit packet-creation mutation mode.

## References
- `docs/product/spec/codex-app-agent-lifecycle-cleanup-contract.md`
- `docs/process/codex-agent-configuration-guide.md`
- `docs/product/spec/taskflow-execution-semantics-scheduler-contract.md`
- `docs/product/spec/internal-codex-agent-execution-fail-closed-contract.md`

-----
artifact_path: product/spec/orchestrator-runtime-contract-hardening-contract
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-05-04'
schema_version: '1'
status: implemented
source_path: docs/product/spec/orchestrator-runtime-contract-hardening-contract.md
created_at: '2026-05-04T00:00:00+03:00'
updated_at: '2026-05-04T00:00:00+03:00'
changelog_ref: orchestrator-runtime-contract-hardening-contract.changelog.jsonl
