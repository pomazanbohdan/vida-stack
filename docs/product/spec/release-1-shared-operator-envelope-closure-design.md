# Release 1 Shared Operator Envelope Closure Design

Status: implemented

## Summary
- Feature / change: close the remaining shared-envelope drift for root operator surfaces and aligned runtime instruction/protocol surfaces
- Owner layer: `mixed`
- Runtime surface: `launcher`
- Status: `implemented`

## Current Context
- `release1_contracts.rs` now owns canonical enums for `workflow_class`, `risk_tier`, `approval_status`, `gate_level`, `compatibility_class`, and blocker normalization.
- `vida status`, `vida doctor`, and `vida taskflow consume bundle` were still drifting from owner law in two ways:
  - emitted migration compatibility under `compatibility_classification` instead of canonical `compatibility_class`
  - accepted shape-only blocker-code arrays in `operator_contracts.rs` instead of enforcing the canonical blocker registry
- Root operator surfaces also omitted shared-envelope placeholders for `workflow_class` and `risk_tier`, which violated the owner contract that workflow-bound fields may be `null` but must not silently disappear.

## Goal
- Make root operator surfaces emit the canonical shared-envelope fields in a way that is correct immediately on the installed runtime binary.
- Keep protocol/instruction surfaces aligned with the same envelope and canonical field law.
- Leave wider root-surface closure such as promoted root `lane` and root `approval` as explicit follow-up work rather than broadening this patch.

## Requirements

### Functional Requirements
- `vida status --json` must emit canonical `compatibility_class` under migration state.
- `vida doctor --json` must emit canonical `compatibility_class` under migration preflight.
- `vida taskflow consume bundle --json` must emit canonical `compatibility_class` under migration preflight.
- Root `vida status` and `vida doctor` JSON envelopes must include `trace_id`, `workflow_class`, and `risk_tier` fields, using `null` when no bounded workflow trace/classification is available.
- `operator_contracts.rs` must reject blocker-code arrays that are lower-snake-case but not registry-backed.
- Instruction/protocol owner surfaces must describe the same field law the runtime now emits.

### Non-Functional Requirements
- Keep file scope bounded to operator-envelope surfaces, shared contract validation, and directly affected owner docs.
- Preserve fail-closed behavior for invalid blocker arrays.
- Prove the change on debug tests and on the installed release binary.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/release-1-operator-surface-contract.md`
  - `docs/product/spec/release-1-runtime-enum-and-code-contracts.md`
  - `docs/product/spec/release-1-conformance-matrix.md`
  - `docs/product/spec/current-spec-map.md`
- Framework protocols affected:
  - none expected beyond runtime-aligned instruction wording if drift is found
- Runtime families affected:
  - launcher-root `vida status`
  - launcher-root `vida doctor`
  - compatibility-path `vida taskflow consume bundle`
- Config / receipts / runtime surfaces affected:
  - `.vida/data/state/runtime-consumption/**` snapshots produced by the refreshed launcher

## Design Decisions

### 1. Emit canonical compatibility field names on live operator surfaces
Will implement / choose:
- Replace emitted `compatibility_classification` with canonical `compatibility_class` on the affected JSON surfaces.
- Keep state-store internal naming unchanged in this slice to avoid widening into storage/schema refactors.
- Why: the owner law applies to emitted machine fields, not necessarily to every internal struct name.
- Trade-offs: internal/external naming stays temporarily asymmetric.
- Alternatives considered: rename storage and projection structs in the same patch. Rejected as too wide for this bounded fix.

### 2. Fail closed on non-registry blocker codes
Will implement / choose:
- Make `operator_contracts.rs` canonicalize blocker arrays against the shared blocker registry instead of validating only lowercase shape.
- Why: lower-snake-case alone is explicitly insufficient per owner law.
- Trade-offs: tighter validation may reject previously tolerated garbage values.
- Alternatives considered: leave validation shape-only and rely on callers. Rejected because that keeps contract drift alive.

### 3. Emit null placeholders for shared-envelope trace and workflow fields
Will implement / choose:
- Add `trace_id: null`, `workflow_class: null`, and `risk_tier: null` to `vida status`, `vida doctor`, and `vida taskflow consume final` JSON envelopes and mirrored shared fields/operator contracts.
- Why: the operator-surface contract says workflow-bound fields may be `null`, but must not disappear.
- Trade-offs: this is an interim closure step, not full workflow classification.
- Alternatives considered: invent provisional workflow classifications for these surfaces. Rejected because that would encode semantics not yet fixed by runtime classification logic.

## Technical Design

### Core Components
- `crates/vida/src/status_surface.rs`
  - emit canonical envelope placeholders and canonical compatibility field names
- `crates/vida/src/doctor_surface.rs`
  - same envelope/compatibility alignment
- `crates/vida/src/taskflow_runtime_bundle.rs`
  - canonical compatibility field for bundle payloads
- `crates/vida/src/operator_contracts.rs`
  - registry-backed blocker validation
- `crates/vida/tests/boot_smoke.rs`
  - proof for envelope placeholders and canonical compatibility field names

### Data / State Model
- No storage migration.
- Runtime snapshot payload shape changes:
  - `migration_state.compatibility_class`
  - `migration_preflight.compatibility_class`
  - top-level `trace_id: null`
  - top-level `workflow_class: null`
  - top-level `risk_tier: null`
- Legacy emitted `compatibility_classification` is intentionally removed from the affected JSON surfaces.

### Integration Points
- Root operator surfaces must remain consistent with:
  - `release-1-operator-surface-contract.md`
  - `release-1-runtime-enum-and-code-contracts.md`
  - `release-1-conformance-matrix.md`
- Installed launcher refresh is required so shell-invoked `vida` reflects the patched runtime.

### Bounded File Set
- `docs/product/spec/release-1-shared-operator-envelope-closure-design.md`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/README.md`
- `docs/product/spec/release-1-operator-surface-contract.md`
- `docs/product/spec/release-1-runtime-enum-and-code-contracts.md`
- `docs/product/spec/release-1-conformance-matrix.md`
- `crates/vida/src/status_surface.rs`
- `crates/vida/src/doctor_surface.rs`
- `crates/vida/src/taskflow_runtime_bundle.rs`
- `crates/vida/src/operator_contracts.rs`
- `crates/vida/tests/boot_smoke.rs`

## Fail-Closed Constraints
- Do not invent non-canonical blocker machine values.
- Do not reintroduce `compatibility_classification` on the affected JSON surfaces.
- Do not silently omit `workflow_class` or `risk_tier` from root operator JSON once added.
- Do not claim non-null trace support in this slice without a canonical runtime source for `trace_id`.

## Implementation Plan

### Phase 1
- Update this design document and affected owner specs/maps.
- First proof target
  - `vida docflow check --root . docs/product/spec/release-1-shared-operator-envelope-closure-design.md docs/product/spec/current-spec-map.md`

### Phase 2
- Patch launcher/runtime surfaces and blocker validation.
- Add or update targeted tests for the affected envelopes.
- Second proof target
  - targeted `cargo test -p vida ...` for bundle/status/doctor/operator-contract coverage

### Phase 3
- Run release build.
- Refresh the installed/system `vida` binary from the fresh release artifact.
- Re-run live `vida` smoke checks against the installed launcher.
- Final proof target
  - release build plus installed-binary runtime checks for `status`, `doctor`, and `taskflow consume bundle`

## Validation / Proof
- Unit tests:
  - targeted `cargo test -p vida canonical_blocker_codes_must_be_registry_backed -- --nocapture`
- Integration tests:
  - targeted `cargo test -p vida taskflow_consume_bundle_renders_runtime_bundle_json -- --nocapture`
  - targeted `cargo test -p vida status_surface_supports_json_summary -- --nocapture`
  - targeted `cargo test -p vida doctor_surface_supports_json_summary -- --nocapture`
- Runtime checks:
  - `vida status --json`
  - `vida doctor --json`
  - `vida taskflow consume bundle --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/release-1-shared-operator-envelope-closure-design.md docs/product/spec/current-spec-map.md`
  - `vida docflow proofcheck --profile active-canon`

## Observability
- Logging points
  - none added
- Metrics / counters
  - none added
- Receipts / runtime state written
  - normal runtime-consumption snapshots from `vida taskflow consume final`, `vida status`, `vida doctor`, and `vida taskflow consume bundle`

## Rollout Strategy
- Development rollout
  - docs first, then bounded code/test patch, then release refresh
- Migration / compatibility notes
  - no storage migration
  - emitted JSON field rename is intentional owner-law closure on affected surfaces
- Operator or user restart / restart-notice requirements
  - refresh installed `vida` after the release build so interactive shell commands see the fixed surfaces immediately

## Future Considerations
- Follow-up ideas
  - replace `trace_id: null` placeholders with one canonical runtime trace source where a bounded workflow trace exists
  - push shared-envelope fields into remaining root/compatibility operator surfaces
- Known limitations
  - `trace_id`, `workflow_class`, and `risk_tier` are emitted as `null` placeholders in this slice rather than fully bound runtime classifications
- Technical debt left intentionally
  - internal storage structs still use `compatibility_classification`

## References
- Related specs
  - `docs/product/spec/release-1-operator-surface-contract.md`
  - `docs/product/spec/release-1-runtime-enum-and-code-contracts.md`
  - `docs/product/spec/release-1-workflow-classification-and-risk-matrix.md`
  - `docs/product/spec/release-1-conformance-matrix.md`
- Related protocols
  - `docs/process/documentation-tooling-map.md`
  - `docs/process/project-orchestrator-operating-protocol.md`

-----
artifact_path: product/spec/release-1-shared-operator-envelope-closure-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: 2026-04-05
schema_version: '1'
status: canonical
source_path: docs/product/spec/release-1-shared-operator-envelope-closure-design.md
created_at: '2026-04-05T05:00:00Z'
updated_at: 2026-04-05T05:20:01.302919223Z
changelog_ref: release-1-shared-operator-envelope-closure-design.changelog.jsonl
