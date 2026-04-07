# Fix Release Admission Evidence Detection Artifac Design

Status: implemented

Use this template for one bounded feature/change design before implementation.

## Summary
- Feature / change: Fix release-admission evidence detection so status/doctor/continuation surfaces accept the newest admissible `final-*` runtime-consumption snapshot and do not re-open blockers because of stale, incomplete, or helper-only snapshot selection.
- Owner layer: `mixed`
- Runtime surface: `taskflow | status | doctor`
- Status: implemented

## Current Context
- Existing system overview
  - `vida taskflow consume final` persists runtime-consumption snapshots under `.vida/data/state/runtime-consumption/final-*.json`.
  - `vida status --json` and `vida doctor --json` surface release-admission and retrieval-trust evidence from runtime-consumption artifacts.
  - `runtime_consumption_state.rs` already exposes helpers for selecting the latest recorded final snapshot and the latest admissible final snapshot.
- Key components and relationships
  - `crates/vida/src/runtime_consumption_state.rs` defines snapshot selection and release-admission evidence detection.
  - `crates/vida/src/status_surface.rs` and `crates/vida/src/doctor_surface.rs` convert snapshot evidence into operator blockers and artifact refs.
  - `crates/vida/tests/doctor_surface_contract_smoke.rs` and `crates/vida/tests/task_smoke.rs` pin canonical runtime-consumption contracts.
- Current pain point or gap
  - A newer helper or incomplete snapshot can still overshadow lawful closure evidence and make `status` or `doctor` report `incomplete_release_admission_operator_evidence`.
  - Some valid final snapshots express closure admission under `payload.closure_admission`, but downstream evidence checks were historically brittle around where release-admission proof is read from.
  - Artifact refs such as `effective_instruction_bundle_receipt_id` and the selected final snapshot path can drift away from the newest admissible final evidence path, which makes operator output look blocked even after a valid `consume final`.

## Goal
- What this change should achieve
  - Make release-admission detection accept canonical `final-*` runtime-consumption snapshots that publish closure/release admission under the supported payload locations.
  - Ensure `status` and `doctor` prefer the newest admissible `final-*` snapshot for release-admission and retrieval-trust evidence instead of a merely newest recorded artifact.
  - Keep operator artifact refs aligned with the selected admissible final snapshot and effective bundle receipt evidence.
- What success looks like
  - A valid `consume final` snapshot with `payload.closure_admission` clears the release-admission blocker even if newer helper artifacts exist.
  - `status --json` and `doctor --json` report the same admissible final snapshot path in `artifact_refs.runtime_consumption_latest_snapshot_path`.
  - Regression tests cover both the positive canonical final path and the stale/incomplete overshadowing case.
- What is explicitly out of scope
  - Redesigning the broader runtime-consumption artifact model.
  - Changing bundle/protocol-binding receipt generation semantics beyond what is required for correct release-admission evidence selection.

## Requirements

### Functional Requirements
- Must accept release-admission evidence from canonical supported locations:
  - `release_admission`
  - `closure_admission`
  - `payload.release_admission`
  - `payload.closure_admission`
- Must select the newest admissible `final-*` runtime-consumption snapshot when computing release-admission and retrieval-trust evidence for operator surfaces.
- Must not let newer `bundle-*`, `bundle-check-*`, malformed, or incomplete `final-*` artifacts override an older admissible final snapshot.
- Must keep `artifact_refs.runtime_consumption_latest_snapshot_path` aligned with the selected admissible final snapshot on `status` and `doctor`.
- Must keep `effective_instruction_bundle_receipt_id` selection compatible with persisted effective-bundle receipts and valid final snapshot evidence.
- Must add regression coverage proving the blocker clears after a valid `vida taskflow consume final` / `vida taskflow consume continue` lineage.

### Non-Functional Requirements
- Performance
  - Snapshot selection remains directory-scan based with no meaningful runtime overhead beyond the current artifact enumeration.
- Scalability
  - Evidence detection must remain tolerant of many historical runtime-consumption snapshots in the state root.
- Observability
  - Operator surfaces must expose the admissible final snapshot path and any remaining blocker reason clearly.
- Security
  - Evidence selection must stay fail-closed for malformed JSON, missing operator contracts, or missing closure/release-admission payloads.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/fix-release-admission-evidence-detection-artifac-design.md`
  - `docs/product/spec/current-spec-map.md`
- Runtime families affected:
  - `taskflow`
  - `status`
  - `doctor`
- Config / receipts / runtime surfaces affected:
  - `vida taskflow consume final`
  - `vida taskflow consume continue`
  - `vida status --json`
  - `vida doctor --json`

## Design Decisions

### 1. Release-admission evidence will be defined by admissible final snapshots, not by raw recency
Will implement / choose:
- Use the newest `final-*` snapshot that passes canonical release-admission evidence validation as the authority for release-admission and retrieval-trust selection.
- Why
  - The operator question is whether lawful closure evidence exists, not whether the filesystem contains a newer but incomplete helper artifact.
- Trade-offs
  - Requires slightly more explicit snapshot filtering and selection logic instead of taking the newest file blindly.
- Alternatives considered
  - Always use the newest final snapshot by mtime.
  - Rejected because malformed or incomplete later snapshots re-open blockers after valid closure evidence already exists.

### 2. Supported release-admission evidence locations stay narrow and explicit
Will implement / choose:
- Keep one canonical helper that recognizes only the supported release-admission locations already used by runtime payloads.
- Why
  - Centralizing the accepted shapes reduces drift between `status`, `doctor`, and runtime-consumption helpers.
- Trade-offs
  - Any future payload location must be added deliberately in one shared helper.
- Alternatives considered
  - Let each surface interpret arbitrary payload shapes.
  - Rejected because that recreates the drift that caused the blocker mismatch.

## Technical Design

### Core Components
- Main components
  - `crates/vida/src/runtime_consumption_state.rs`
  - `crates/vida/src/status_surface.rs`
  - `crates/vida/src/doctor_surface.rs`
  - smoke/regression tests under `crates/vida/tests/*`
- Key interfaces
  - `runtime_consumption_snapshot_has_release_admission_evidence(...)`
  - `latest_final_runtime_consumption_snapshot_path(...)`
  - `latest_recorded_final_runtime_consumption_snapshot_path(...)`
- Bounded responsibilities
  - `runtime_consumption_state.rs` owns admissibility and latest-valid selection.
  - `status_surface.rs` and `doctor_surface.rs` consume those helpers consistently and fail closed.

### Data / State Model
- Important entities
  - runtime-consumption snapshot
  - admissible final snapshot
  - operator artifact refs
  - effective instruction bundle receipt
- Receipts / runtime state / config fields
  - `artifact_refs.runtime_consumption_latest_snapshot_path`
  - `artifact_refs.effective_instruction_bundle_receipt_id`
  - `artifact_refs.retrieval_trust_signal`
  - `payload.closure_admission`
  - `payload.release_admission`
- Migration or compatibility notes
  - No schema migration is required.
  - Existing stored snapshots remain valid; selection logic changes only how operator surfaces choose among them.

### Integration Points
- Runtime-family handoffs
  - `taskflow consume final/continue` writes snapshots consumed later by `status` and `doctor`.
- Cross-document / cross-protocol dependencies
  - Release-1 operator contract parity must remain intact across `artifact_refs`, `shared_fields`, and `operator_contracts`.

### Bounded File Set
- `docs/product/spec/fix-release-admission-evidence-detection-artifac-design.md`
- `docs/product/spec/current-spec-map.md`
- `crates/vida/src/runtime_consumption_state.rs`
- `crates/vida/src/status_surface.rs`
- `crates/vida/src/doctor_surface.rs`
- `crates/vida/tests/doctor_surface_contract_smoke.rs`
- `crates/vida/tests/task_smoke.rs`
- `crates/vida/tests/boot_smoke.rs`

## Fail-Closed Constraints
- Forbidden fallback paths
  - No acceptance of non-`final` runtime-consumption artifacts as release-admission authority.
  - No success path when operator contracts or closure/release-admission evidence are missing.
- Required receipts / proofs / gates
  - Canonical release-admission evidence helper must stay shared.
  - `status` and `doctor` must keep contract parity for `artifact_refs`.
- Safety boundaries that must remain true during rollout
  - Incomplete snapshots still block.
  - Valid older admissible final snapshots remain selectable when newer invalid artifacts exist.

## Implementation Plan

### Phase 1
- Complete this design doc and pin the bounded file set plus proof targets.
- First proof target
  - `vida docflow fastcheck --root . docs/product/spec/fix-release-admission-evidence-detection-artifac-design.md`

### Phase 2
- Tighten shared admissible-final selection and wire `status` / `doctor` to the same authority path.
- Add regressions for:
  - valid `payload.closure_admission` final snapshot acceptance
  - newer incomplete final snapshot not overshadowing older admissible final evidence
  - operator `artifact_refs` parity after admissible selection
- Second proof target
  - targeted `cargo test -p vida` coverage for `doctor_surface_contract_smoke`, `task_smoke`, and any unit tests in `runtime_consumption_state.rs`

### Phase 3
- Run the targeted runtime smokes if code changes are required:
  - `vida status --json`
  - `vida doctor --json`
  - `vida taskflow consume final "release-admission probe" --json`

## Validation / Proof
- Unit tests:
  - `cargo test -p vida latest_admissible_retrieval_trust_signal_accepts_latest_final_snapshot -- --nocapture`
  - `cargo test -p vida latest_admissible_retrieval_trust_signal_blocks_stale_or_non_final_evidence -- --nocapture`
- Integration tests:
  - `cargo test -p vida final_snapshot_missing_release_admission_evidence_accepts_canonical_blocked_snapshot -- --nocapture`
  - `cargo test -p vida doctor_surface_contract_smoke -- --nocapture`
- Runtime checks:
  - `vida doctor --json`
  - `vida status --json`

## Observability
- Logging points
  - none added
- Metrics / counters
  - none added
- Receipts / runtime state written
  - normal runtime-consumption final snapshots and operator-contract artifact refs

## Rollout Strategy
- Development rollout
  - spec/doc closure only in this packet; runtime implementation is already present in the worktree
- Migration / compatibility notes
  - no migration required
- Operator or user restart / restart-notice requirements
  - none beyond normal binary refresh already completed for Release 1 proof

## Future Considerations
- Follow-up ideas
  - add an explicit “latest admissible final snapshot” selector/result to the runtime-consumption summary to avoid duplicated selection logic
- Known limitations
  - operator surfaces still depend on filesystem snapshot ordering via modified time
- Technical debt left intentionally
  - no broader refactor of runtime-consumption snapshot indexing in this packet

## References
- Related specs
  - `docs/product/spec/release-1-operator-surface-contract.md`
  - `docs/product/spec/release-1-conformance-matrix.md`
- Related protocols
  - none beyond release-1 operator contract canon
- Related ADRs
  - none
- External references
  - none

-----
artifact_path: product/spec/fix-release-admission-evidence-detection-artifac-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-06
schema_version: 1
status: canonical
source_path: docs/product/spec/fix-release-admission-evidence-detection-artifac-design.md
created_at: 2026-04-06T07:13:27.76466372Z
updated_at: 2026-04-07T18:20:14.753585291Z
changelog_ref: fix-release-admission-evidence-detection-artifac-design.changelog.jsonl
artifact_path: product/spec/fix-release-admission-evidence-detection-artifac-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-07
schema_version: 1
status: canonical
source_path: docs/product/spec/fix-release-admission-evidence-detection-artifac-design.md
created_at: 2026-04-06T07:13:27.76466372Z
updated_at: 2026-04-07T18:17:00+03:00
changelog_ref: fix-release-admission-evidence-detection-artifac-design.changelog.jsonl
