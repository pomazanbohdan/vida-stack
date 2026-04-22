# Explicit Implementation Seed Drops Design Backed Owned Paths Design

Purpose: Bound the audit blocker where a task with an already-registered bounded design document seeds directly into `implementation` via `auto_explicit_implementation_request`, but the seeded execution plan drops the tracked design-doc context and `dispatch-init` then fails closed because the implementer `delivery_task_packet` has no derived `owned_paths`.

Status: `proposed`

## Summary
- Feature / change: preserve design-backed bounded file scope when explicit implementation seeding bypasses the old scope-discussion/work-pool override path.
- Owner layer: `mixed`
- Runtime surface: `taskflow | launcher`
- Status: `proposed`

## Current Context
- Existing system overview
  - `derive_seeded_run_graph_status(...)` in `taskflow_run_graph.rs` calls `try_existing_design_backed_implementation_override(...)` before finalizing the seeded implementation status.
  - `try_existing_design_backed_implementation_override(...)` only injects `tracked_flow_bootstrap.design_doc_path` when the pre-override selection is still `scope_discussion/spec-pack` or `pbi_discussion/work-pool-pack`.
  - `runtime_delivery_task_packet_with_scope_context(...)` in `runtime_dispatch_packets.rs` derives implementer `owned_paths` from explicit request file terms first, then from the tracked design doc bounded file set.
- Key components and relationships
  - For `feature-fix-bug-analysis-lane-can-close-implementation-without-write-evidence`, the registered design doc already exists at `docs/product/spec/analysis-lane-can-close-implementation-without-write-evidence-design.md` and includes a bounded file set.
  - Seeding that task used `reason=auto_explicit_implementation_request_override`, `tracked_flow_entry=null`, and a generic synthetic `tracked_flow_bootstrap` instead of the registered design doc.
  - `cargo run -p vida -- taskflow run-graph dispatch-init feature-fix-bug-analysis-lane-can-close-implementation-without-write-evidence --json` then failed with:
    - `Runtime dispatch packet 'delivery_task_packet' is missing required packet fields: owned_paths`
  - No dispatch receipt was recorded; run-graph status stayed at:
    - `status=ready`
    - `active_node=planning`
    - `next_node=implementer`
    - `resume_target=dispatch.implementer_lane`
- Current pain point or gap
  - Existing design-backed tasks only inherit `tracked_design_doc_path` through the old discussion/work-pool override branch.
  - Direct explicit-implementation seeding loses that context even when the task already has a lawful registered design doc.
  - The resulting dispatch-init failure is a packet-shaping regression, not a lack of runtime readiness.

## Goal
- What this change should achieve
  - Ensure explicit implementation seeding for an existing design-backed task preserves the tracked design doc and its bounded file set.
  - Allow `dispatch-init` to render a lawful implementer `delivery_task_packet` with non-empty `owned_paths` from the design-backed bounded file set when the request text itself does not name files.
  - Keep generic design-first bootstrap scaffolding out of already-designed in-progress repair tasks.
- What success looks like
  - A design-backed in-progress task seeded through explicit implementation selection still records `tracked_flow_bootstrap.design_doc_path`.
  - `dispatch-init` succeeds and renders implementer `owned_paths` from the registered design doc bounded file set.
  - Existing scope-discussion/work-pool override behavior remains green.
- What is explicitly out of scope
  - Reopening the parked MemPalace lane.
  - Broad redesign of packet validation for unrelated task classes.
  - Replacing the separate analysis-lane closure blocker; this fix is only its upstream dispatch-init prerequisite.

## Requirements

### Functional Requirements
- Existing design-backed task detection must inject `tracked_design_doc_path` for explicit implementation seeding, not only for scope/work-pool discussion overrides.
- Seeded implementation runs with registered design docs must prefer the bounded file set from that design doc when request text lacks explicit file paths.
- `dispatch-init` for those runs must not fail on missing implementer `owned_paths` if the registered design doc already supplies a bounded file set.
- Existing behavior for tasks without any registered design doc must remain unchanged and continue to fail closed when owned scope is absent.

### Non-Functional Requirements
- Observability
  - Seed payloads should expose the tracked design doc path clearly enough to explain why owned paths were derived.
- Safety
  - No heuristic widening beyond the bounded file set from the design doc.
- Compatibility
  - The earlier qwen and existing-design reseed proofs must remain green.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/explicit-implementation-seed-drops-design-backed-owned-paths-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/current-spec-provenance-map.md`
- Runtime families affected:
  - `taskflow`
  - `launcher`
- Config / receipts / runtime surfaces affected:
  - `vida taskflow run-graph seed <task-id> <request> --json`
  - `vida taskflow run-graph dispatch-init <task-id> --json`
  - implementer delivery packet shaping

## Design Decisions

### 1. Existing design-doc injection must be independent from the legacy discussion-mode branch
Will implement / choose:
- Decouple design-doc reuse from the requirement that the initial selection still be `scope_discussion` or `pbi_discussion`.
- If the task already has a registered bounded design doc and the request is an explicit implementation repair, seed should still inject `tracked_flow_bootstrap.design_doc_path`.
- Why
  - Current logic silently loses design-backed context exactly in the direct explicit-implementation path that should most need it.
- Trade-offs
  - Seed logic becomes slightly broader, but only for tasks with ready design docs.

### 2. Generic design-first bootstrap placeholders must not override existing bounded design truth
Will implement / choose:
- Preserve the already-selected implementation route while replacing synthetic placeholder bootstrap metadata with the actual registered design doc path.
- Why
  - The current seed output advertises a fake feature slug/design path that is unrelated to the real active design artifact.
- Trade-offs
  - Seed payloads become more dependent on task-to-design-doc discovery, which is acceptable because that discovery already exists and is tested.

### 3. Regression proofs must cover seed plus dispatch-init together
Will implement / choose:
- Add or update tests so a design-backed explicit implementation seed leads to implementer packet `owned_paths` at dispatch-init time, not just a prettier seed payload.
- Why
  - The actual operator failure occurs one step later at packet rendering/validation.
- Trade-offs
  - The proof surface spans both `taskflow_run_graph.rs` and runtime packet shaping helpers.

## Technical Design

### Core Components
- Main components
  - `taskflow_run_graph.rs`
  - `runtime_dispatch_packets.rs`
  - `runtime_dispatch_state.rs`
- Key interfaces
  - `existing_design_backed_task_design_doc_path(...)`
  - `try_existing_design_backed_implementation_override(...)`
  - `tracked_design_doc_path(...)`
  - `runtime_delivery_task_packet_with_scope_context(...)`
- Bounded responsibilities
  - discover the lawful registered design doc for the active task
  - inject that design doc into explicit implementation seeding
  - derive implementer `owned_paths` from the bounded file set during dispatch-init

### Bounded File Set
- `docs/product/spec/explicit-implementation-seed-drops-design-backed-owned-paths-design.md`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/current-spec-provenance-map.md`
- `crates/vida/src/taskflow_run_graph.rs`
- `crates/vida/src/runtime_dispatch_packets.rs`
- `crates/vida/src/runtime_dispatch_state.rs`

## Fail-Closed Constraints
- Forbidden fallback paths
  - no silent dispatch-init retry with generic implementer packet scope
  - no heuristic owned path widening beyond the registered bounded file set
  - no root-session local implementation as a workaround
- Required receipts / proofs / gates
  - dispatch-init must either receive non-empty design-backed `owned_paths` or fail closed for tasks without lawful design-backed scope

## Implementation Plan

### Phase 1
- Register and finalize this bounded design.
- First proof target
  - `vida docflow check --root . docs/product/spec/explicit-implementation-seed-drops-design-backed-owned-paths-design.md docs/product/spec/current-spec-map.md docs/product/spec/current-spec-provenance-map.md`

### Phase 2
- Repair explicit implementation seeding so design-backed context survives into dispatch-init.
- Second proof target
  - targeted `cargo test -p vida` for existing-design seed and implementer packet owned-path derivation

### Phase 3
- Re-run the active blocker path and confirm dispatch-init no longer fails on missing `owned_paths`.
- Final proof target
  - targeted tests plus live `cargo run -p vida -- taskflow run-graph dispatch-init feature-fix-bug-analysis-lane-can-close-implementation-without-write-evidence --json`

## Validation / Proof
- Unit tests:
  - explicit implementation seed with a registered design doc injects `tracked_flow_bootstrap.design_doc_path`
  - dispatch-init derives implementer `owned_paths` from the registered design doc bounded file set
  - tasks without registered design docs still fail closed when owned scope is absent
- Runtime checks:
  - seeded payload exposes the real design doc path, not a synthetic placeholder
  - implementer dispatch-init succeeds past packet validation for the active blocker run

## References
- `docs/product/spec/analysis-lane-can-close-implementation-without-write-evidence-design.md`
- `docs/product/spec/repair-design-backed-reseed-canonicalization-does-not-deadlock-qwen-design.md`
- `docs/product/spec/reconcile-qwen-cli-carrier-drift-design.md`
- `docs/product/spec/continuation-binding-fail-closed-hardening-design.md`

-----
artifact_path: product/spec/explicit-implementation-seed-drops-design-backed-owned-paths-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-21
schema_version: 1
status: canonical
source_path: docs/product/spec/explicit-implementation-seed-drops-design-backed-owned-paths-design.md
created_at: 2026-04-21T20:10:37.59987108Z
updated_at: 2026-04-21T20:12:59.871136281Z
changelog_ref: explicit-implementation-seed-drops-design-backed-owned-paths-design.changelog.jsonl
