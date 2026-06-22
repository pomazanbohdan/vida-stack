# Tower-Based Canonical Command Pipeline Design

Status: `proposed`

## Summary
- Feature / change: introduce one canonical Tower-style command pipeline for VIDA command envelopes.
- Owner layer: `runtime-family`
- Runtime surface: `launcher | taskflow | service-client | runtime dispatch`
- Status: proposed for `ldr-040`

## Current Context
- `root_command_router.rs` currently routes top-level CLI commands directly to surface modules after state-dir binding and timing hooks.
- `service_client_cli.rs` already converts service/project/wizard/job/receipt commands into `VidaCommandEnvelope` and invokes a `VidaClient`.
- `vida_client_inprocess.rs` currently delegates to a fixture client instead of a production command pipeline.
- `vida-contracts` owns operation metadata such as operation posture, required claim, project-ref requirement, idempotency requirement, and apply-token requirement.
- Runtime dispatch surfaces already implement host-bridge and lane receipt semantics separately from service-client execution.

## Goal
- Make `VidaCommandEnvelope` the shared command entry contract for service-client families first, then widen to mutation-capable command families in later bounded slices.
- Fix middleware order for trace, deadline, schema/version, operation lookup, project routing, authorization/admission, idempotency, concurrency, handler execution, and response mapping.
- Preserve default human output behavior; this slice does not require a new default `--json` recommendation.
- Out of scope: replacing every existing CLI mutation in one change, adding external service networking, or introducing Restate/Effectum execution.

## Requirements

### Functional Requirements
- All service-client command envelopes must be executable through a single pipeline object rather than direct fixture dispatch.
- The pipeline must validate schema/protocol versions before operation lookup.
- Operation metadata from `vida-contracts` must drive claim, idempotency, apply-token, and project-ref checks.
- Non-idempotent apply/admin operations must fail closed when the envelope lacks an idempotency key.
- Read/plan operations may remain shared-read and must not require apply tokens.
- Response mapping must preserve `VidaCommandResponse` status, blockers, receipts, job refs, and request ids.

### Non-Functional Requirements
- Pipeline construction must be cheap enough for CLI invocation.
- Middleware order must be unit-tested as a public invariant.
- Handler adapters must contain no domain authorization logic.
- The implementation must remain in-process for now and expose an adapter boundary for later transports.

## Ownership And Canonical Surfaces
- Project docs / specs affected: this design document.
- Framework protocols affected: TaskFlow proof and closure evidence for `ldr-040`.
- Runtime families affected: command routing, service-client execution, runtime dispatch receipts.
- Config / receipts / runtime surfaces affected: command trace payloads in tests only for this slice.

## Design Decisions

### 1. Introduce A Canonical Command Pipeline Seam
Will implement / choose:
- Use `VidaCommandEnvelope -> VidaCommandResponse` as the pipeline boundary.
- Add a small internal pipeline module under `crates/vida/src`, then route `InProcessVidaClient` through it.
- Use `tower::Service` only at the pipeline seam; keep handlers ordinary Rust functions until a transport needs `BoxCloneService`.
- Why: this gives one middleware order without forcing every CLI surface to migrate at once.
- Trade-offs: service-client families move first; root CLI mutation families remain outside until follow-up slices.
- Alternatives considered: rewrite `root_command_router` first; rejected because it would mix routing, output rendering, and domain behavior in one XL change.

### 2. Fixed Middleware Order
Will implement / choose:
- Order: trace, deadline, schema/protocol, operation lookup, project routing, authorization/admission, idempotency, concurrency, handler, response mapping.
- Why: cheap syntactic rejection happens before stateful admission and handler execution.
- Trade-offs: retries are not introduced in this slice; retry semantics need durable idempotency first.
- Alternatives considered: Tower retry/load-shed immediately; rejected for apply/admin operations until idempotency proof is durable.

### 3. Service Client First Migration
Will implement / choose:
- Migrate `service_client_cli.rs` and `vida_client_inprocess.rs` to use the pipeline.
- Keep `root_command_router.rs` limited to lifecycle/timing and future architecture lint proof.
- Why: service-client surfaces already produce envelopes and have focused tests.
- Trade-offs: acceptance item "every production mutation enters through this Tower service" becomes a staged invariant: new service-client mutations enter through the pipeline now; broader mutation routing gets follow-up slices.
- Alternatives considered: mark all legacy command paths as blocked; rejected because current operators need existing TaskFlow/DocFlow commands.

## Technical Design

### Core Components
- `command_pipeline` module:
  - `VidaCommandPipeline` implementing the command service boundary.
  - `CommandPipelineLayer` enum or trace list for middleware-order proof.
  - `CommandPipelineHandler` for operation-specific execution.
- `vida_client_inprocess.rs`:
  - constructs a ready in-process pipeline.
  - implements `VidaClient` by executing the pipeline.
- `service_client_cli.rs`:
  - continues to parse CLI args and render responses.
  - contains no authorization or operation metadata logic.

### Data / State Model
- Input: `VidaCommandEnvelope`.
- Output: `VidaCommandResponse`.
- Trace evidence: ordered layer names and operation id in test-only or response metadata.
- No new persisted state in the design slice.

### Integration Points
- `vida-contracts::operation_spec` is the metadata source.
- `service_client_cli::execute_service_cli_request` is the first production caller.
- `root_command_router::run_root_command` remains the command family dispatcher.

### Bounded File Set
- `docs/product/spec/tower-based-canonical-command-pipeline-phase-design.md`
- `crates/vida/src/root_command_router.rs`
- `crates/vida/src/command_lifecycle_hooks.rs`
- `crates/vida/src/runtime_dispatch_execution.rs`
- `crates/vida/src/service_client_cli.rs`
- `crates/vida/src/agent_dispatch_surface.rs`
- Expected implementation follow-up may add `crates/vida/src/command_pipeline.rs` and update `crates/vida/src/lib.rs` or module declarations if required.
- Expected dependency follow-up may add `tower` to workspace dependencies and `crates/vida/Cargo.toml` if the implementation uses the actual crate.

## Fail-Closed Constraints
- No non-idempotent apply/admin command may be retried automatically.
- Missing/invalid schema or protocol version must block before handler execution.
- Unknown operation must block before project routing or authorization.
- Missing required project ref, idempotency key, or apply token must block before handler execution.
- Adapters must not duplicate operation metadata rules.

## Implementation Plan

### Phase 1
- Add command pipeline module and layer-order trace.
- Proof target: layer-order unit test.

### Phase 2
- Route `InProcessVidaClient` through the pipeline and keep service CLI output stable.
- Proof target: end-to-end command trace through service-client tests.

### Phase 3
- Add architecture lint/test that service-client execution uses the pipeline seam and adapters remain metadata-free.
- Proof target: mutation-entry architecture lint.

## Validation / Proof
- Unit tests:
  - `cargo test -p vida command_pipeline_layer_order`
  - `cargo test -p vida command_pipeline_blocks_non_idempotent_apply_without_key`
- Integration tests:
  - `cargo test -p vida cli_service_client_routes_all_service_first_families_through_vida_client`
- Runtime checks:
  - `vida task validate-graph`
  - `vida diagnostics post-commit`
- Canonical checks:
  - `vida orchestrator-init`
  - `vida doctor`

## Observability
- Tests must prove trace order.
- Future runtime trace receipts should include request id, operation id, layer order, verdict, and blocker codes.

## Rollout Strategy
- Implement in-process service-client pipeline first.
- Keep existing CLI default output stable.
- Do not remove current command surfaces in this slice.
- Commit and release-install after proof.

## Future Considerations
- Move TaskFlow/DocFlow mutation commands behind the pipeline in follow-up tasks.
- Add durable idempotency and retry/load-shed semantics after redb command ledger support is complete.
- Add transport-level `BoxCloneService` once IPC/server mode uses the same pipeline.

## References
- `crates/vida/src/root_command_router.rs`
- `crates/vida/src/service_client_cli.rs`
- `crates/vida/src/vida_client_inprocess.rs`
- `crates/vida-contracts/src/lib.rs`
- `todo-ldr-040-design-packet-20260622`
- `ldr-040`

-----
artifact_path: docs/product/spec/tower-based-canonical-command-pipeline-phase-design.md
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-06-22'
schema_version: '1'
status: proposed
source_path: docs/product/spec/tower-based-canonical-command-pipeline-phase-design.md
created_at: '2026-06-22T21:30:00+03:00'
updated_at: '2026-06-22T21:30:00+03:00'
changelog_ref: tower-based-canonical-command-pipeline-phase-design.changelog.jsonl
