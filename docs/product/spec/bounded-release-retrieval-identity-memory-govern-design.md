# Bounded Release Retrieval Identity Memory Govern Design

Status: `draft`

Use this document as the tracked spec-pack design packet for the Release-1 retrieval / identity / memory governance wave bootstrapped by `vida taskflow consume final`.

## Summary
- Feature / change: bounded Release-1 closure for retrieval trust registry semantics, principal identity/delegation modeling, and memory-governance operationalization
- Owner layer: `mixed`
- Runtime surface: `taskflow`
- Status: `draft`

## Current Context
- Retrieval trust currently exists as a fail-closed signal with the fields `source`, `citation`, `freshness`, and `acl`.
- Approval and delegation enforcement already exist through run-graph receipts and approval surfaces, but principal identity is not modeled explicitly apart from carrier/runtime-role routing identity.
- Canonical memory governance fields already exist in `CanonicalMemoryRecord`, but the runtime still enforces them only partially through narrower governance and handoff checks.

## Goal
- Close the bounded Release-1 gap recorded in the active task by:
  - extending retrieval trust with source-registry and ACL-aware evidence,
  - introducing explicit principal/delegation modeling,
  - operationalizing canonical memory-governance fields across runtime and operator surfaces.
- Keep out of scope:
  - broad external IAM integration,
  - unbounded memory product features,
  - a new storage family outside the current runtime/state model.

## Requirements

### Functional Requirements
- Retrieval trust must represent:
  - source registry reference,
  - citation linkage,
  - freshness posture,
  - ACL propagation.
- Principal / delegation data must be explicit and separate from carrier selection.
- Sensitive delegated and policy-changing workflows must point to actor identity, delegation linkage, approval evidence, and audit linkage.
- Memory governance must operationalize:
  - `memory_class`
  - `sensitivity_level`
  - `consent_basis`
  - `ttl_policy`
  - `deletion_or_correction_ref`
  - approval linkage

### Non-Functional Requirements
- All new checks remain fail-closed.
- Existing canonical blocker compatibility must be preserved where possible.
- Operator surfaces must remain machine-readable and aligned across bundle, consume, status, and doctor projections.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/release-1-plan.md`
  - `docs/product/spec/release-1-capability-matrix.md`
  - `docs/product/spec/release-1-current-state.md`
  - `docs/product/spec/release-1-decision-tables.md`
  - `docs/product/spec/release-1-workflow-classification-and-risk-matrix.md`
  - `docs/product/spec/release-1-canonical-artifact-schemas.md`
- Runtime families affected:
  - `taskflow`
- Config / receipts / runtime surfaces affected:
  - runtime-consumption retrieval-trust evidence
  - runtime-bundle admission evidence
  - run-graph governance / delegation summaries and receipts
  - approval, consume, status, and doctor operator projections

## Design Decisions

### 1. Extend Retrieval Trust In Place
Will implement / choose:
- extend the existing retrieval-trust operator contract rather than introduce a separate trust subsystem.
- Why:
  - current code already has fail-closed retrieval blocker and projection paths,
  - specs require richer evidence, not a separate architecture branch.

### 2. Separate Principal Identity From Carrier Identity
Will implement / choose:
- add explicit principal / delegation data next to existing run-graph and approval artifacts instead of inferring identity from the selected backend / carrier role.
- Why:
  - governance identity and execution routing identity are different concepts,
  - specs explicitly require actor identity and chain-of-delegation evidence.

### 3. Reuse The Canonical Memory Record Schema
Will implement / choose:
- operationalize `CanonicalMemoryRecord` rather than create a parallel memory-governance schema.
- Why:
  - the schema already contains the required Release-1 fields,
  - the real gap is runtime enforcement and projection, not missing vocabulary.

## Technical Design

### Core Components
- `release1_contracts.rs` as the canonical contract anchor
- `runtime_consumption_state.rs` for retrieval-trust evidence production
- `taskflow_runtime_bundle.rs` for bundle admission and blocker projection
- `taskflow_consume.rs` for approval / delegation / governance enforcement
- `state_store_run_graph_state.rs` and `state_store_run_graph_summary.rs` for persisted governance and delegation truth
- `approval_surface.rs`, `doctor_surface.rs`, and `status` operator surfaces for projection parity

### Data / State Model
- Retrieval trust evidence should widen additively from the current minimal signal to include registry-aware semantics.
- Principal / delegation evidence should include:
  - principal identifier
  - principal scope / kind
  - delegator linkage
  - delegatee linkage
  - approval linkage
  - audit / trace linkage
- Memory governance must project the existing canonical memory-record fields into runtime-owned enforcement and visibility paths.

### Integration Points
- `runtime_consumption_state.rs`
- `taskflow_runtime_bundle.rs`
- `taskflow_consume.rs`
- `state_store_run_graph_state.rs`
- `state_store_run_graph_summary.rs`
- `approval_surface.rs`
- `doctor_surface.rs`
- `status_surface_operator_contracts.rs`
- `consume_final_operator_surface.rs`

### Bounded File Set
- `docs/product/spec/bounded-release-retrieval-identity-memory-govern-design.md`
- `docs/product/spec/README.md`
- `crates/vida/src/release1_contracts.rs`
- `crates/vida/src/runtime_consumption_state.rs`
- `crates/vida/src/taskflow_runtime_bundle.rs`
- `crates/vida/src/taskflow_consume.rs`
- `crates/vida/src/state_store_run_graph_state.rs`
- `crates/vida/src/state_store_run_graph_summary.rs`
- `crates/vida/src/approval_surface.rs`
- `crates/vida/src/doctor_surface.rs`
- `crates/vida/src/status_surface_operator_contracts.rs`
- `crates/vida/src/consume_final_operator_surface.rs`

## Fail-Closed Constraints
- Missing retrieval-trust registry / citation / freshness / ACL evidence must continue to block.
- Principal identity must not be inferred solely from carrier/backend identity.
- Sensitive memory or identity/policy changes must not proceed without approval and audit linkage.
- Delegated implementation still routes through the tracked TaskFlow packet flow after this spec-pack closes.

## Implementation Plan

### Phase 1
- Extend canonical contracts and blocker vocabulary additively.
- First proof target:
  - contract serialization and compatibility tests.

### Phase 2
- Project those fields into runtime bundle, consume, run-graph, and operator surfaces.
- Second proof target:
  - status / doctor / consume evidence parity tests.

### Phase 3
- Harden delegated sensitive-workflow and memory-governance enforcement.
- Final proof target:
  - bounded runtime/operator proofs for retrieval trust, delegation evidence, and memory governance together.

## Validation / Proof
- Unit tests:
  - contract field coverage and blocker compatibility
  - retrieval-trust evidence generation
  - run-graph governance validation
- Integration tests:
  - runtime bundle / consume / status / doctor parity
- Runtime checks:
  - `vida status --json`
  - `vida doctor --json`
  - `vida taskflow consume bundle check --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/bounded-release-retrieval-identity-memory-govern-design.md`

## Observability
- Keep new governance/trust fields visible in operator JSON surfaces.
- Keep blocker codes canonical and machine-readable.
- Write governance/delegation linkage into runtime-owned receipts or summaries rather than prose-only notes.

## Rollout Strategy
- Close the spec-pack first.
- Shape the work-pool / dev packet from this bounded file set next.
- Keep the rollout additive where existing operator contracts are already consumed by proofs.

## Future Considerations
- Registry storage may be separated more cleanly in a later wave.
- External IAM / auth-provider integration remains out of scope here.
- Rich memory CRUD/operator tooling can follow after the bounded governance contract closes.

## References
- `docs/product/spec/release-1-plan.md`
- `docs/product/spec/release-1-capability-matrix.md`
- `docs/product/spec/release-1-current-state.md`
- `docs/product/spec/release-1-decision-tables.md`
- `docs/product/spec/release-1-workflow-classification-and-risk-matrix.md`
- `docs/product/spec/release-1-canonical-artifact-schemas.md`

-----
artifact_path: product/spec/bounded-release-retrieval-identity-memory-govern-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-20
schema_version: 1
status: canonical
source_path: docs/product/spec/bounded-release-retrieval-identity-memory-govern-design.md
created_at: 2026-04-20T10:26:05.476772467Z
updated_at: 2026-04-20T10:27:46.503375433Z
changelog_ref: bounded-release-retrieval-identity-memory-govern-design.changelog.jsonl
