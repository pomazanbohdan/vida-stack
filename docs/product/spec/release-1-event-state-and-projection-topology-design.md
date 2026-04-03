# Release 1 Event-State And Projection Topology Design

Status: `proposed`

Use this template for one bounded feature/change design before implementation.

Structured-template rule:
1. Keep headings stable.
2. Replace placeholders rather than rewriting the shape.
3. Prefer explicit fields and short bullets over long free-form prose.
4. Link separate ADRs when one or more major decisions need durable decision records.

## Summary
- Feature / change: define the bounded Release-1 event-state topology that adds replayable runtime evidence where VIDA already requires lawful transitions, while preserving `SurrealDB` as the default DB-first activation/projection truth
- Owner layer: `project`
- Runtime surface: `taskflow | docflow | project activation | launcher`
- Status: `proposed`

## Current Context
- Existing system overview:
  - `TaskFlow` already owns execution substrate and closure authority in canon.
  - `DocFlow` already owns documentation, readiness, validation, and proof.
  - `vida` still holds too much runtime truth in launcher-owned modules.
  - `SurrealDB` is already the canonical DB-first operational store for activation and projections.
- Key components and relationships:
  - `crates/taskflow-state-surreal/**` defines the current default state-store target and schema bootstrap.
  - `crates/vida/src/state_store.rs` currently owns most persisted checkpoint, recovery, projection, and snapshot bridge behavior.
  - `crates/vida/src/taskflow_consume.rs`, `crates/vida/src/taskflow_run_graph.rs`, and `crates/vida/src/main.rs` currently host major execution/closure logic.
  - `release-1-canonical-artifact-schemas.md`, `checkpoint-commit-and-replay-model.md`, and `projection-listener-checkpoint-model.md` already require stronger runtime contracts than the current shared Rust layer provides.
- Current pain point or gap:
  - event/replay/checkpoint law is explicit in spec, but not yet expressed as one bounded runtime design for implementation,
  - the codebase lacks canonical shared contracts for workflow/risk/gate vocabulary and for domain-event / projection-checkpoint artifacts,
  - operator/query surfaces still read launcher-shaped runtime truth rather than clearly separated execution-event and projection domains,
  - SierraDB/event-spine ideas are relevant, but there is no bounded rule yet for where they do and do not belong in Release 1.

## Goal
- What this change should achieve:
  - define one bounded design for adding event-state where Release 1 already requires replay, checkpoint lineage, approval/tool/recovery evidence, and explicit closure receipts.
- What success looks like:
  - the design preserves `SurrealDB` as the default operational truth for activation/projections/query surfaces,
  - lawful transition domains gain one canonical event contract and projector/checkpoint contract,
  - any future SierraDB integration remains optional, feature-gated, and adapter-backed,
  - implementation can proceed crate-by-crate without re-opening the authority split question.
- What is explicitly out of scope:
  - replacing `SurrealDB` as the Release-1 default operational truth,
  - event-sourcing canonical markdown bodies or retrieval/vector payloads,
  - Release-2 reactive host-project sync implementation,
  - broad UI or external daemon design.

## Requirements

### Functional Requirements
- Must-have behavior:
  - Release 1 must keep `SurrealDB` as the default DB-first operational truth for activation entities, materialized projections, and operator query surfaces.
  - Event-state must be introduced only for lawful transition boundaries that need replay, checkpoint lineage, approval/tool/recovery evidence, or explicit closure receipts.
  - The first bounded event-state domains are:
    - TaskFlow workflow/run/lane/approval/recovery execution
    - protocol-binding and compiled-bundle revision events where admission/history matters
    - DocFlow operational verdicts for mutation/validation/readiness/proof
    - sync/reconcile decision receipts where import/export/conflict law matters
  - Operator surfaces must continue reading projections and bounded summaries rather than scanning raw event streams directly.
  - Shared contracts must add at least:
    - `domain_event`
    - `projection_checkpoint_record`
    - alignment of the existing `resumability_capsule` with replay/continuation law
  - Event storage must remain behind adapter traits; kernel/runtime law must not depend on one backend implementation.
  - If SierraDB is introduced, it must be feature-gated and optional for Release 1 rather than required for boot.
  - Filesystem and Git must remain projection and lineage surfaces, not equal runtime truth surfaces.
  - Root/operator routing must not claim top-level event-driven support until `consume`, `lane`, `approval`, and `recovery` surfaces expose stable contracts.
- Integration points:
  - `crates/taskflow-contracts/**`
  - `crates/taskflow-state/**`
  - `crates/taskflow-state-surreal/**`
  - future `crates/taskflow-state-sierra/**`
  - `crates/vida/src/release1_contracts.rs`
  - `crates/vida/src/state_store.rs`
  - `crates/vida/src/taskflow_consume.rs`
  - `crates/vida/src/taskflow_run_graph.rs`
  - `crates/vida/src/taskflow_protocol_binding.rs`
  - `crates/vida/src/taskflow_proxy.rs`
  - `docs/product/spec/release-1-canonical-artifact-schemas.md`
  - `docs/product/spec/project-activation-and-configurator-model.md`
  - `docs/product/spec/operational-state-and-synchronization-model.md`
- User-visible or operator-visible expectations:
  - `status`, `doctor`, activation status, recovery status, and closure status remain query-first and fail-closed,
  - runtime can replay supported workflows and rebuild bounded projections,
  - missing event/projection evidence blocks closure instead of silently degrading.

### Non-Functional Requirements
- Performance:
  - event append and projector updates must stay bounded enough for CLI-first Release 1 workflows,
  - projection reads remain the default hot path for operator surfaces.
- Scalability:
  - the design must support multiple storage adapters and future Release-2 sync without another authority rewrite.
- Observability:
  - every lawful transition, tool/approval/recovery decision, and replay/checkpoint boundary remains explicit and queryable.
- Security:
  - no event backend may bypass approval, policy, or identity law,
  - sensitive projection data must not leak through raw event inspection by default.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/release-1-event-state-and-projection-topology-design.md`
  - `docs/product/spec/release-1-plan.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/current-spec-provenance-map.md`
  - `docs/product/spec/release-1-canonical-artifact-schemas.md`
  - `docs/product/spec/project-activation-and-configurator-model.md`
  - `docs/product/spec/operational-state-and-synchronization-model.md`
  - `docs/product/spec/release-1-state-machine-specs.md`
- Framework protocols affected:
  - none directly; this design constrains later runtime-family implementation under the existing replay/checkpoint/runtime-consumption protocols
- Runtime families affected:
  - `TaskFlow`
  - `DocFlow`
  - launcher shell `vida`
- Config / receipts / runtime surfaces affected:
  - `.vida/data/state/**`
  - `.vida/project/**`
  - `.vida/cache/**`
  - runtime checkpoint, recovery, and closure receipts
  - future domain-event and projection-checkpoint artifacts

## Design Decisions

### 1. Keep a dual-plane authority model instead of replacing the current DB-first canon
Will implement / choose:
- `SurrealDB` remains the default operational truth for activation, projections, and operator query surfaces.
- Event-state becomes an additional runtime-evidence plane for lawful transition domains only.
- Why:
  - this matches the current activation and synchronization canon,
  - it avoids a false "replace the database" migration,
  - it preserves the existing query-first operator model.
- Trade-offs:
  - two planes must stay explicitly coordinated,
  - projector/checkpoint contracts become mandatory earlier.
- Alternatives considered:
  - replace `SurrealDB` wholesale with SierraDB; rejected because it breaks current DB-first canon and widens Release-1 risk.
- ADR link if this must become a durable decision record: optional follow-up ADR if storage-line policy needs standalone governance.

### 2. Event-source only lawful transition domains, not every mutable artifact
Will implement / choose:
- Event-state is limited to execution, approval, tool/policy, replay/recovery, bundle/protocol-binding revision, DocFlow operational verdicts, and sync/reconcile decisions.
- Canonical docs, retrieval indexes, and vector payloads remain outside the event-spine truth model.
- Why:
  - Release 1 needs replay and auditability at transition boundaries, not event copies of every document body.
- Trade-offs:
  - some surfaces still require ordinary projection/update logic,
  - boundaries must be documented carefully.
- Alternatives considered:
  - event-source the entire `.vida/**` universe; rejected as too broad and misaligned with the current canon.
- ADR link if needed: none.

### 3. Introduce adapter-backed event and projector contracts before any backend-specific implementation
Will implement / choose:
- add storage-neutral interfaces for:
  - event append/scan
  - projection checkpoint persistence
  - replay/fork lineage
  - resumability capsule loading
- keep Sierra-specific behavior behind a future dedicated adapter crate.
- Why:
  - `taskflow-v1-runtime-modernization-plan.md` already requires storage behind adapters,
  - it avoids hard-coding backend law into launcher or kernel code.
- Trade-offs:
  - first implementation wave is more contract-heavy,
  - launcher code must be carved before backend experiments produce value.
- Alternatives considered:
  - implement Sierra-specific logic directly in launcher/store code; rejected because it deepens the current ownership problem.
- ADR link if needed: none.

### 4. Keep operator surfaces projection-first and fail-closed
Will implement / choose:
- `status`, `doctor`, `consume`, `lane`, `approval`, and `recovery` surfaces must read projection/state summaries and block when checkpoint/projector lineage is incomplete.
- Why:
  - Release 1 operator contracts are query-first,
  - raw stream inspection is not the operator-facing control plane.
- Trade-offs:
  - projector freshness becomes a hard dependency for truthful status,
  - proof coverage must include projector rebuild and parity tests.
- Alternatives considered:
  - let operator surfaces read raw events directly; rejected because it weakens the status-family model and complicates fail-closed rendering.
- ADR link if needed: none.

## Technical Design

### Core Components
- Main components:
  - shared runtime contract layer for workflow/risk/gate/event vocabulary
  - event-store trait
  - projection-checkpoint trait
  - runtime handlers that convert lawful commands/transitions into appended events plus projector updates
  - projector groups for activation, execution status, approval/recovery status, and DocFlow verdict consumption
- Key interfaces:
  - `EventStore`
  - `ProjectionStore`
  - `CheckpointStore`
  - `ReplayStore`
  - canonical artifact builders for `domain_event` and `projection_checkpoint_record`
- Bounded responsibilities:
  - `vida` routes and renders only
  - `TaskFlow` owns execution-event law, replay, closure admission, and projector updates for execution domains
  - `DocFlow` emits operational verdict events and remains readiness/proof owner
  - `SurrealDB` stores the materialized projection/query truth by default
  - optional Sierra adapter stores append-only domain events when enabled

### Data / State Model
- Important entities:
  - `domain_event`
    - minimum fields: `event_id`, `event_type`, `stream_id`, `stream_kind`, `aggregate_id`, `aggregate_version`, `partition_key`, `correlation_id`, `causation_id`, `trace_id`, `workflow_class`, `risk_tier`, `actor_kind`, `actor_id`, `occurred_at`, `payload_schema_version`, `blocker_codes[]`, `related_artifact_ids[]`, `side_effect_class`, `replay_safe`, `payload`
  - `projection_checkpoint_record`
    - minimum fields: `projector_id`, `checkpoint_group`, `lineage_kind`, `origin_checkpoint_ref`, `replay_scope`, `fork_parent`, `last_gapless_position`, `updated_at`
  - `resumability_capsule`
    - remains the minimum continuation packet for runtime resume and must align with checkpoint/replay law
- Receipts / runtime state / config fields:
  - event-enabled storage line must remain explicit in runtime config and must default to disabled for Release 1 until proof exists
  - activation/projection truth continues to expose `SurrealDB` storage metadata and migration/readiness state
- Migration or compatibility notes:
  - first wave is additive: introduce contracts and traits before moving existing persisted rows
  - current `SurrealDB` tables and snapshot bridge remain valid until projector-backed replacements are proven

### Integration Points
- APIs:
  - existing CLI entrypoints continue to call bounded family-owned services
  - no new public root command is added until the operator surface contract is closed
- Runtime-family handoffs:
  - `TaskFlow` emits/consumes execution, approval, recovery, and closure artifacts
  - `DocFlow` emits mutation/validation/readiness/proof verdict events consumable by `TaskFlow`
- Cross-document / cross-protocol dependencies:
  - `release-1-canonical-artifact-schemas.md` must gain the event/projection artifacts if this design is promoted,
  - `operational-state-and-synchronization-model.md` must replace the current "next discussion item" placeholder with actual event-domain taxonomy,
  - `project-activation-and-configurator-model.md` must describe event-backed activation lifecycle without surrendering DB-first projection truth.

### Bounded File Set
- List every file expected to change
- Keep this list explicit and bounded
- Docs:
  - `docs/product/spec/release-1-event-state-and-projection-topology-design.md`
  - `docs/product/spec/release-1-plan.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/current-spec-provenance-map.md`
  - `docs/product/spec/release-1-canonical-artifact-schemas.md`
  - `docs/product/spec/project-activation-and-configurator-model.md`
  - `docs/product/spec/operational-state-and-synchronization-model.md`
  - `docs/product/spec/release-1-state-machine-specs.md`
- Runtime code:
  - `crates/taskflow-contracts/src/lib.rs`
  - `crates/taskflow-state/src/lib.rs`
  - `crates/taskflow-state-surreal/src/lib.rs`
  - `crates/vida/src/release1_contracts.rs`
  - `crates/vida/src/taskflow_proxy.rs`
  - `crates/vida/src/taskflow_consume.rs`
  - `crates/vida/src/taskflow_run_graph.rs`
  - `crates/vida/src/state_store.rs`
  - future `crates/taskflow-state-sierra/src/lib.rs`

## Fail-Closed Constraints
- Forbidden fallback paths:
  - no event-backend experiment may replace `SurrealDB` as default activation/projection truth during Release 1,
  - no operator surface may silently fall back from missing projector/checkpoint evidence to optimistic status.
- Required receipts / proofs / gates:
  - projector lineage and replay checkpoints must be explicit,
  - closure remains blocked when required event/projection evidence is missing,
  - shared workflow/risk/gate enums must be canonical before broad event-state rollout.
- Safety boundaries that must remain true during rollout:
  - storage adapters remain beneath kernel/runtime law,
  - filesystem edits remain projections/imports rather than live truth,
  - shell concentration must decrease, not deepen.

## Implementation Plan

### Phase 1
- Initial implementation tasks:
  - promote this design into the Release-1 routing set,
  - add canonical shared enum/value contracts,
  - add `domain_event` and `projection_checkpoint_record` to the artifact/schema law,
  - define adapter traits in `taskflow-state`.
- First proof target:
  - docs and shared contracts are consistent enough that conformance matrix can point to one bounded implementation queue.

### Phase 2
- Integration / refinement tasks:
  - carve checkpoint/replay/projection interfaces out of launcher-owned store code,
  - materialize event append plus projection update flow for one TaskFlow execution path,
  - keep Surreal-backed projections as the default query source.
- Second proof target:
  - one replayable TaskFlow run can rebuild the same bounded status/recovery projection.

### Phase 3
- Hardening / rollout tasks:
  - integrate DocFlow verdict events into the `TaskFlow -> DocFlow -> closure` seam,
  - add feature-gated Sierra adapter only after contract and projection parity exist,
  - stabilize operator-surface completion for `consume`, `lane`, `approval`, and `recovery`.
- Final proof target:
  - closure, replay, and operator-surface parity are receipt-backed and fail-closed.

## Validation / Proof
- Unit tests:
  - canonical enum/value normalization
  - event payload and checkpoint payload validation
  - replay/projector parity over one bounded execution path
- Integration tests:
  - TaskFlow execution -> projection rebuild -> status/recovery parity
  - TaskFlow -> DocFlow verdict -> closure admission path
  - feature-disabled boot path where no event backend is required
- Runtime checks:
  - `cargo test`
  - `vida status --json`
  - `vida doctor --json`
  - `vida taskflow recovery status <run-id> --json`
  - `vida taskflow consume final <request> --json`
- Canonical checks:
  - `activation-check`
  - `protocol-coverage-check`
  - `check`
  - `doctor`

## Observability
- Logging points:
  - event append
  - projector checkpoint commit
  - replay start/finish
  - projection parity mismatch
- Metrics / counters:
  - appended events by domain
  - projector lag / last gapless position
  - replay duration
  - projection rebuild failures
- Receipts / runtime state written:
  - `domain_event`
  - `projection_checkpoint_record`
  - existing resumability/checkpoint/recovery receipts

## Rollout Strategy
- Development rollout:
  - docs/contracts first
  - one bounded execution path second
  - optional Sierra adapter only after parity proof
- Migration / compatibility notes:
  - retain current Surreal-backed snapshots and projections until equivalent event/projector path is proven
  - do not widen root CLI contracts before family-owned services exist
- Operator or user restart / restart-notice requirements:
  - none at design stage
  - runtime packaging/build validation remains blocked until the local toolchain linker issue is fixed

## Future Considerations
- Follow-up ideas:
  - split `taskflow-contracts` into richer event/receipt/schema modules
  - add a dedicated `taskflow-runtime` crate after launcher carve-out starts
- Known limitations:
  - this design does not settle Release-2 reactive sync implementation,
  - it assumes the current DB-first projection model stays in force through Release 1
- Technical debt left intentionally:
  - current launcher-owned store code remains the temporary bridge until Phase 2

## References
- Related specs
  - `docs/product/spec/release-1-plan.md`
  - `docs/product/spec/release-1-capability-matrix.md`
  - `docs/product/spec/release-1-conformance-matrix.md`
  - `docs/product/spec/project-activation-and-configurator-model.md`
  - `docs/product/spec/operational-state-and-synchronization-model.md`
  - `docs/product/spec/projection-listener-checkpoint-model.md`
  - `docs/product/spec/checkpoint-commit-and-replay-model.md`
  - `docs/product/spec/release-1-canonical-artifact-schemas.md`
- Related protocols
  - existing TaskFlow/DocFlow replay, runtime-consumption, and protocol-binding protocol surfaces
- Related ADRs
  - none yet
- External references
  - bounded external event-state and projector-pattern research already absorbed into the active Release-1 design direction

-----
artifact_path: product/spec/release-1-event-state-and-projection-topology-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-04-03'
schema_version: '1'
status: canonical
source_path: docs/product/spec/release-1-event-state-and-projection-topology-design.md
created_at: '2026-04-03T09:54:07+03:00'
updated_at: 2026-04-03T09:54:07+03:00
changelog_ref: release-1-event-state-and-projection-topology-design.changelog.jsonl
