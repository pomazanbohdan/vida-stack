# Continuation And Seeded Dispatch Bridge Design

Status: `implemented`

Use this template for one bounded feature/change design before implementation.

## Summary
- Feature / change: add first-class continuation binding and seeded run dispatch-init/packet surfaces so a seeded TaskFlow run can progress lawfully without heuristic fallback or code-level rediscovery.
- Owner layer: `mixed`
- Runtime surface: `taskflow | launcher`
- Status: `implemented`

## Current Context
- Existing system overview
  - The runtime already persists `RunGraphStatus`, `RunGraphDispatchReceipt`, and recovery/gate summaries in the authoritative state store.
  - `vida taskflow consume final` can already materialize a routed dispatch receipt, packet, and downstream preview.
  - `vida taskflow run-graph seed` and `advance` can already shape run-graph state for a bounded task.
- Key components and relationships
  - `crates/vida/src/state_store.rs` owns authoritative persisted runtime rows.
  - `crates/vida/src/taskflow_run_graph.rs` owns seeded/advanced run-graph mutation.
  - `crates/vida/src/taskflow_consume.rs` and `crates/vida/src/runtime_dispatch_state.rs` already know how to build dispatch receipts, handoff plans, and runtime packets.
  - `crates/vida/src/taskflow_runtime_bundle.rs` and `crates/vida/src/status_surface.rs` surface continuation posture to the operator.
- Current pain point or gap
  - Seeded runs do not persist the request/selection context needed to materialize the first dispatch receipt later.
  - `consume continue` fails closed when no persisted dispatch receipt exists, even if the run-graph state is otherwise lawful.
  - Continuation binding is currently summarized heuristically from latest status/receipt instead of being explicitly recordable and inspectable as first-class runtime state.

## Goal
- What this change should achieve
  - Persist explicit continuation binding for the active bounded unit and request-backed seeded dispatch context.
  - Add a `run-graph dispatch-init` bridge that materializes the first persisted dispatch receipt and packet preview for a seeded run.
  - Add packet inspection surfaces that expose lawful resume inputs directly from persisted runtime evidence.
- What success looks like
  - `vida taskflow continuation bind` records explicit `active_bounded_unit`, `why_this_unit`, and posture for one run.
  - `vida taskflow run-graph dispatch-init` succeeds for a newly seeded run without going back through `consume final`.
  - `vida taskflow packet render` shows persisted packet/receipt evidence needed for lawful continuation.
  - `vida orchestrator-init --json` and `vida status --json` prefer explicit continuation binding when available.
- What is explicitly out of scope
  - redesigning TaskFlow backlog semantics
  - changing delegated lane ownership law
  - widening packet semantics beyond the current runtime dispatch packet contract

## Requirements

### Functional Requirements
- Must persist seeded dispatch context keyed by `run_id`, including:
  - `request_text`
  - serialized `RuntimeConsumptionLaneSelection`
  - `task_id`
- Must persist explicit continuation binding keyed by `run_id`, including:
  - `active_bounded_unit`
  - `binding_source`
  - `why_this_unit`
  - `primary_path`
  - `sequential_vs_parallel_posture`
- Must add `vida taskflow continuation bind <run-id> [--why <text>] [--json]`.
- Must add `vida taskflow run-graph dispatch-init <run-id> [--json]`.
- Must add `vida taskflow packet render <run-id> [--json]`.
- Must keep `consume continue` fail-closed when no persisted dispatch receipt exists.
- Must let `dispatch-init` reuse the same receipt/packet building logic already used by `consume final`.

### Non-Functional Requirements
- Performance
  - added lookups must remain single-run keyed state-store reads/writes
- Observability
  - new rows and surfaces must be visible in JSON output
  - tests must cover seeded run -> dispatch-init -> packet render
- Security
  - no fallback to heuristic ready-task selection when explicit binding/dispatch evidence is absent

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/continuation-and-seeded-dispatch-bridge-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/current-spec-provenance-map.md`
  - `docs/product/spec/README.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `taskflow`
- Config / receipts / runtime surfaces affected:
  - `vida taskflow continuation bind`
  - `vida taskflow run-graph dispatch-init`
  - `vida taskflow packet render`
  - `vida orchestrator-init`
  - `vida status`

## Design Decisions

### 1. Seeded run context will be persisted as first-class state-store data
Will implement / choose:
- Add a persisted seeded dispatch-context row keyed by `run_id`.
- Why
  - `dispatch-init` needs the original request and lane-selection evidence, and recomputing them later from ambient state would drift.
- Trade-offs
  - Adds one more small authoritative row family.
- Alternatives considered
  - Recompute lane selection on demand from current config only.
  - Store request text only and rebuild the rest heuristically.

### 2. Explicit continuation binding will be persisted separately from heuristic summary logic
Will implement / choose:
- Add a persisted continuation-binding row keyed by `run_id`.
- Prefer that row in init/status surfaces when evidence is consistent.
- Why
  - The operator needs a first-class record of what is bound now, not only an inferred summary.
- Trade-offs
  - Requires sync on seed/advance/manual bind.
- Alternatives considered
  - Keep only the current summary logic over latest status/receipt.

### 3. `dispatch-init` will materialize receipt and packet preview but not execute the handoff
Will implement / choose:
- Reuse existing receipt/packet builders from `consume final`.
- Record the first `RunGraphDispatchReceipt` plus dispatch/downstream packet paths.
- Why
  - This closes the missing bridge without skipping the lawful `consume continue`/execution stage.
- Trade-offs
  - Adds one more operator step before continuation on seeded runs.
- Alternatives considered
  - Have `dispatch-init` execute the dispatch immediately.
  - Make `consume continue` implicitly synthesize missing receipts.

## Technical Design

### Core Components
- `state_store`
  - new persisted row families for continuation binding and seeded dispatch context
- `taskflow_run_graph`
  - seed/advance sync of those rows plus `dispatch-init`
- `taskflow_continuation`
  - explicit continuation bind surface
- `taskflow_packet`
  - packet render/inspect surface
- `continuation_binding_summary`
  - prefer explicit binding row when present and consistent

### Data / State Model
- New state row: `RunGraphDispatchContext`
  - `run_id`
  - `task_id`
  - `request_text`
  - `role_selection`
  - `recorded_at`
- New state row: `RunGraphContinuationBinding`
  - `run_id`
  - `task_id`
  - `active_bounded_unit`
  - `binding_source`
  - `why_this_unit`
  - `primary_path`
  - `sequential_vs_parallel_posture`
  - `request_text`
  - `recorded_at`
- Compatibility notes
  - additive-only state schema change
  - older seeded runs without the new context rows continue to fail closed until reseeded or rebound

### Integration Points
- `StateStore` schema bootstrap and CRUD methods
- `taskflow_proxy` routing and `taskflow_layer4` help
- `taskflow_runtime_bundle` and `status_surface`
- existing receipt/packet helpers in `taskflow_consume` and `runtime_dispatch_state`

### Bounded File Set
- `docs/product/spec/continuation-and-seeded-dispatch-bridge-design.md`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/current-spec-provenance-map.md`
- `docs/product/spec/README.md`
- `docs/product/spec/continuation-and-seeded-dispatch-bridge-design.changelog.jsonl`
- `crates/vida/src/main.rs`
- `crates/vida/src/state_store.rs`
- `crates/vida/src/continuation_binding_summary.rs`
- `crates/vida/src/taskflow_proxy.rs`
- `crates/vida/src/taskflow_layer4.rs`
- `crates/vida/src/taskflow_run_graph.rs`
- `crates/vida/src/taskflow_consume.rs`
- `crates/vida/src/taskflow_runtime_bundle.rs`
- `crates/vida/src/status_surface.rs`
- `crates/vida/src/taskflow_continuation.rs`
- `crates/vida/src/taskflow_packet.rs`

## Fail-Closed Constraints
- Do not synthesize continuation from ready backlog order when no explicit or receipt-backed binding exists.
- Do not let `dispatch-init` invent a new lane selection from ambient runtime state when seeded context is missing.
- Do not let `packet render` claim lawful resume inputs when the persisted receipt or packet path is absent.

## Implementation Plan

### Phase 1
- Register the bounded design in the current spec canon.
- First proof target
  - `vida docflow check --root . docs/product/spec/continuation-and-seeded-dispatch-bridge-design.md`

### Phase 2
- Add persisted state rows plus new `continuation`, `packet`, and `run-graph dispatch-init` surfaces.
- Second proof target
  - targeted `cargo test -p vida ...` for state-store and surface coverage

### Phase 3
- Prefer explicit binding in init/status, run live CLI proof, and rebuild the release binary.
- Final proof target
  - docflow + targeted cargo tests + live seeded-run proof + release build

## Validation / Proof
- Unit tests:
  - state-store round-trip for dispatch context and continuation binding
  - continuation summary prefers explicit binding
  - `run-graph dispatch-init` creates persisted receipt and packet paths
- Integration tests:
  - packet render returns lawful resume inputs from persisted receipt
- Runtime checks:
  - `vida taskflow continuation bind <run-id> --json`
  - `vida taskflow run-graph dispatch-init <run-id> --json`
  - `vida taskflow packet render <run-id> --json`
  - `vida status --json`
  - `vida orchestrator-init --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/continuation-and-seeded-dispatch-bridge-design.md docs/product/spec/current-spec-map.md docs/product/spec/current-spec-provenance-map.md docs/product/spec/README.md`

## Observability
- New persisted rows make explicit binding and seeded dispatch context inspectable in the authoritative DB.
- Packet render surfaces expose receipt ids, packet paths, and packet bodies directly.
- Status/init summaries surface explicit binding provenance when present.

## Rollout Strategy
- Land state-store, routing, and summary preference in one bounded change.
- Keep all changes additive so existing runtime rows remain readable.
- Rebuild and reinstall the local `vida` binary after proof passes.

## Future Considerations
- Add richer packet diff/inspect variants once more packet types are persisted.
- Surface continuation-binding lineage in recovery views as a future follow-up.

## References
- `docs/product/spec/continuation-binding-fail-closed-hardening-design.md`
- `docs/product/spec/release-1-operator-surface-contract.md`
- `AGENTS.md`

-----
artifact_path: product/spec/continuation-and-seeded-dispatch-bridge-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-04-10'
schema_version: '1'
status: canonical
source_path: docs/product/spec/continuation-and-seeded-dispatch-bridge-design.md
created_at: '2026-04-10T13:40:00+03:00'
updated_at: '2026-04-10T13:40:00+03:00'
changelog_ref: continuation-and-seeded-dispatch-bridge-design.changelog.jsonl
