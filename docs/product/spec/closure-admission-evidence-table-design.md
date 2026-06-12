# Closure Admission Evidence Table Design

Status: `proposed`

Use this template for one bounded feature/change design before implementation.

## Summary
- Feature / change: define one explicit closure-admission evidence table so runtime-consumption, status, doctor, and closure gating surfaces consume the same minimum evidence set instead of relying on scattered prose.
- Owner layer: `mixed`
- Runtime surface: `taskflow | status | doctor | runtime-consumption`
- Status: `proposed`

## Current Context
- Existing system overview
  - `docs/product/spec/release-1-closure-contract.md` already requires explicit, receipt-backed closure admission.
  - `docs/product/spec/release-1-canonical-artifact-schemas.md` already defines the minimal `closure_admission_record` schema.
  - `docs/product/spec/fix-release-admission-evidence-detection-artifac-design.md` already narrows how admissible final snapshots are selected for operator surfaces.
- Key components and relationships
  - `release-1-closure-contract.md` defines what Release-1 closure must prove.
  - `release-1-canonical-artifact-schemas.md` defines the machine-readable record shapes.
  - `runtime_consumption_state.rs`, `status_surface.rs`, `doctor_surface.rs`, and `release1_contracts.rs` are the current runtime owner surfaces that interpret closure-admission evidence.
- Current pain point or gap
  - The canon names the required evidence families, but it does not yet provide one compact table that binds evidence class, canonical source artifact, minimum acceptable signal, and fail-closed blocker semantics in one place.
  - Packet `github-123-closure-admission-evidence-table` has no existing canonical spec landing zone in the repo, so delegated specification work cannot point to a bounded owner doc yet.
  - Without one explicit table, nearby specs can remain individually correct while runtime and operator surfaces drift on which artifacts are mandatory versus merely helpful.

## Goal
- What this change should achieve
  - Create one bounded design doc that states the closure-admission evidence table explicitly.
  - Bind closure-admission evidence classes to canonical artifact families, consuming surfaces, and blocker semantics.
  - Give future implementation or proof packets one canonical specification target instead of relying on cross-document inference.
- What success looks like
  - Closure-admission evidence can be reviewed from one table without reconstructing it from multiple specs.
  - Runtime work can cite a canonical minimum evidence table when deciding whether closure admission is lawful.
  - The active spec map and provenance map register this document as the packet landing artifact.
- What is explicitly out of scope
  - Reworking release-admission snapshot selection logic.
  - Redesigning the `closure_admission_record` schema beyond clarifying how it participates in the evidence table.
  - Implementing runtime changes in this delegated specification packet.

## Requirements

### Functional Requirements
- Must define one explicit closure-admission evidence table for the current Release-1 closure path.
- Must distinguish required evidence from supporting evidence.
- Must identify the canonical artifact or receipt family for each required evidence class.
- Must state the minimum acceptable signal for each evidence class.
- Must state the fail-closed blocker when an evidence class is absent, stale, or contradictory.
- Must keep `release-1-closure-contract.md`, `release-1-canonical-artifact-schemas.md`, and runtime-consumption/operator evidence semantics aligned.
- Must provide a bounded specification target that future runtime packets can cite directly.

### Non-Functional Requirements
- Performance
  - Documentation-only in this packet; future runtime consumers should be able to evaluate the table without widening artifact scans beyond current families.
- Scalability
  - The table must remain additive so later evidence families can be appended without redefining the closure contract.
- Observability
  - Operator surfaces should be able to explain closure blockers in terms that map directly to the table rows.
- Security
  - Missing or contradictory evidence must remain fail-closed; the table must not create any heuristic success path.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/closure-admission-evidence-table-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `active spec/catalog maps and Git history`
- Runtime families affected:
  - `taskflow`
  - `status`
  - `doctor`
  - `runtime-consumption`
- Config / receipts / runtime surfaces affected:
  - `vida taskflow consume final`
  - `vida status --json`
  - `vida doctor --json`
  - closure/readiness proof receipts under the current runtime-consumption path

## Design Decisions

### 1. Closure admission will be specified as a table, not only as prose
Will implement / choose:
- Add one compact evidence table that names every required closure-admission evidence family and its blocker semantics.
- Why
  - Closure law, schema law, and runtime law are already present, but the missing crosswalk makes review and implementation drift too easy.
- Trade-offs
  - Adds one more bounded spec artifact to maintain.
- Alternatives considered
  - Continue relying on the closure contract plus schema docs only.
  - Rejected because the current packet exists precisely because that split leaves no single bounded handoff target.

### 2. The table will treat runtime-consumption final snapshots as derived evidence, not the sole closure authority
Will implement / choose:
- Treat the final runtime-consumption snapshot as an admissibility and operator-consumption surface that must align with stronger owner artifacts such as closure records, readiness/proof receipts, and lineage artifacts.
- Why
  - Snapshot selection logic already matters, but Release-1 closure law explicitly requires more than “latest final snapshot exists.”
- Trade-offs
  - Closure evaluation remains multi-artifact instead of collapsing into one helper artifact.
- Alternatives considered
  - Let the latest admissible final snapshot stand in for the whole closure bundle.
  - Rejected because it weakens the explicit closure bundle and replay/lineage requirements in the active canon.

## Technical Design

### Closure Admission Evidence Table

| Evidence class | Canonical source artifact | Minimum acceptable signal | Main consuming surfaces | Fail-closed blocker when absent or contradictory |
| --- | --- | --- | --- | --- |
| Closure decision record | `closure_admission_record` from `release-1-canonical-artifact-schemas.md` | `closure_decision=closed`, named `decision_owner`, `decision_at`, and non-empty `evidence_bundle_refs` | `release1_contracts.rs`, final closure checks, future closure summaries | no explicit closure admission verdict or verdict not backed by bundle refs |
| Runtime-consumption final snapshot | canonical admissible `final-*` runtime-consumption snapshot | supported closure or release admission payload present and selected as the newest admissible final snapshot | `runtime_consumption_state.rs`, `status_surface.rs`, `doctor_surface.rs` | operator surfaces report incomplete or stale release-admission evidence |
| DocFlow readiness and proof receipts | readiness/proof receipts required by `release-1-closure-contract.md` | explicit green-enough readiness/proof receipts for the claimed closure scope | `taskflow consume final`, closure gating, seam proof review | seam proof relies only on protocol binding or prose without DocFlow receipts |
| Lane execution and handoff receipts | `lane_execution_receipt` plus bounded delegated execution receipts | every closure-relevant lane has receipt-backed execution evidence; activation view alone is insufficient | run-graph reconciliation, closure gating, delegated audit chain | closure path depends on activation-only or non-executing delegated artifacts |
| Replay/checkpoint lineage artifacts | replay/checkpoint lineage artifacts required by the closure contract | recovery or rollback claims point to explicit lineage artifacts, not latest summaries only | recovery validation, closure admission, doctor/status blocker projection | recovery closure claimed without replay/checkpoint lineage |
| Risk acceptance artifacts | bounded risk-acceptance artifact matching `release-1-closure-contract.md` | any open non-terminal gap is explicitly bounded, owned, and still lawful for supported scope | closure readiness review, operator blockers, release decision | open gap exists but no valid bounded risk acceptance artifact explains it |
| Evidence bundle linkage | closure bundle refs across plan, seam, capability, proof, control, and artifact-schema surfaces | referenced artifacts resolve to the active canonical docs/artifacts for the same release scope | final closure review, conformance review, future self-diagnostic summaries | closure claim cannot be reconstructed from canonical artifacts alone |

### Core Components
- Main components
  - `docs/product/spec/release-1-closure-contract.md`
  - `docs/product/spec/release-1-canonical-artifact-schemas.md`
  - `docs/product/spec/fix-release-admission-evidence-detection-artifac-design.md`
  - `crates/vida/src/{runtime_consumption_state,status_surface,doctor_surface,release1_contracts}.rs`
- Key interfaces
  - release-admission evidence selection helpers
  - closure-admission record production/consumption
  - operator blocker rendering for missing evidence families
- Bounded responsibilities
  - This design doc owns the crosswalk table.
  - Existing closure/schema docs remain the source of truth for detailed law and field definitions.

### Data / State Model
- Important entities
  - `closure_admission_record`
  - admissible final runtime-consumption snapshot
  - `lane_execution_receipt`
  - DocFlow readiness/proof receipt
  - replay/checkpoint lineage artifact
  - bounded risk-acceptance artifact
- Migration or compatibility notes
  - No schema migration is required for this specification packet.
  - Future runtime implementation should consume the table additively and preserve current fail-closed behavior.

### Integration Points
- Runtime-family handoffs
  - `taskflow` produces and reconciles the closure-facing evidence families.
  - `status` and `doctor` surface blockers when required rows are missing or contradictory.
- Cross-document / cross-protocol dependencies
  - `release-1-closure-contract.md`
  - `release-1-canonical-artifact-schemas.md`
  - `release-1-conformance-matrix.md`
  - `fix-release-admission-evidence-detection-artifac-design.md`

### Bounded File Set
- `docs/product/spec/closure-admission-evidence-table-design.md`
- `docs/product/spec/current-spec-map.md`
- `active spec/catalog maps and Git history`
- `docs/product/spec/release-1-closure-contract.md`
- `docs/product/spec/release-1-canonical-artifact-schemas.md`
- `docs/product/spec/release-1-conformance-matrix.md`
- `crates/vida/src/runtime_consumption_state.rs`
- `crates/vida/src/status_surface.rs`
- `crates/vida/src/doctor_surface.rs`
- `crates/vida/src/release1_contracts.rs`

## Fail-Closed Constraints
- Forbidden fallback paths
  - No closure success path based only on commentary, status text, or activation-view artifacts.
  - No collapse of the full closure bundle into one final snapshot heuristic.
- Required receipts / proofs / gates
  - Closure decision must remain explicit and receipt-backed.
  - DocFlow proof/readiness receipts remain mandatory where the closure contract requires them.
  - Recovery claims remain invalid without lineage artifacts.
- Safety boundaries that must remain true during rollout
  - Operator surfaces may summarize, but they must not silently widen what counts as closure evidence.
  - Missing rows in the evidence table remain blockers, not warnings.

## Implementation Plan

### Phase 1
- Land this design doc and register it in the active spec/provenance maps.
- First proof target
  - `vida docflow fastcheck --root . docs/product/spec/closure-admission-evidence-table-design.md docs/product/spec/current-spec-map.md active spec/catalog maps and Git history`

### Phase 2
- Update closure-facing runtime logic to consume or cite the table explicitly where blocker classification or evidence summaries currently rely on scattered checks.
- Second proof target
  - targeted `cargo test -p vida` around release-admission, closure gating, and operator blocker classification

### Phase 3
- Re-run runtime/operator proof for a closure path and a blocked path to confirm table-row parity between artifacts and operator output.
- Third proof target
  - `vida status --json`
  - `vida doctor --json`
  - bounded `vida taskflow consume final ... --json` closure proof replay

## Validation / Proof
- Unit tests
  - future targeted tests in `release1_contracts.rs` and runtime-consumption helpers for missing-row blocker parity
- Integration tests
  - future `cargo test -p vida` coverage for status/doctor closure-admission blocker rendering
- Runtime checks
  - `vida docflow check --root . docs/product/spec/closure-admission-evidence-table-design.md docs/product/spec/current-spec-map.md active spec/catalog maps and Git history`
  - `vida status --json`
  - `vida doctor --json`

## Observability
- Logging points
  - none in this specification packet
- Metrics / counters
  - none in this specification packet
- Receipts / runtime state written
  - none in this specification packet beyond normal documentation metadata/changelog tracking

## Rollout Strategy
- Development rollout
  - spec-only landing artifact for packet `github-123-closure-admission-evidence-table`
- Migration / compatibility notes
  - additive documentation change only
- Operator or user restart / restart-notice requirements
  - none

## Future Considerations
- Follow-up ideas
  - expose table-row names or codes directly in closure blockers so operator output cites the same vocabulary as the spec
  - add a machine-readable closure-evidence bundle schema if future runtime work needs stricter bundle validation
- Known limitations
  - this packet does not yet enforce the table in runtime code
- Technical debt left intentionally
  - existing closure evidence logic remains distributed across current runtime owner files until a later implementation packet consumes this table directly

## References
- Related specs
  - `docs/product/spec/release-1-closure-contract.md`
  - `docs/product/spec/release-1-canonical-artifact-schemas.md`
  - `docs/product/spec/release-1-conformance-matrix.md`
  - `docs/product/spec/fix-release-admission-evidence-detection-artifac-design.md`
- Related protocols
  - none beyond the active Release-1 closure canon
- Related ADRs
  - none
- External references
  - none

-----
artifact_path: product/spec/closure-admission-evidence-table-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-05-13
schema_version: 1
status: canonical
source_path: docs/product/spec/closure-admission-evidence-table-design.md
created_at: 2026-05-13T00:00:00Z
updated_at: 2026-05-13T00:00:00Z
changelog_ref: closure-admission-evidence-table-design.changelog.jsonl
