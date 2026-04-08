# Ops State And Runtime Evidence Hygiene Design

Status: proposed

Use this template for one bounded feature/change design before implementation.

## Summary
- Feature / change: Define the safe operational boundary for `.vida/data/state/**`, separate durable authoritative state from generated runtime-consumption evidence, and standardize proof workflows for fresh temp-state versus long-lived project state.
- Owner layer: `mixed`
- Runtime surface: `taskflow | status | doctor | project activation | other`
- Status: proposed

## Current Context
- Existing system overview
  - `vida` uses `.vida/data/state/` as the default authoritative state root for the local project.
  - The authoritative store contains Surreal-backed state spine data and runtime-owned evidence such as `runtime-consumption/final-*.json`.
  - Release-1 closure is functionally implemented and proof-backed, but local operator proofs can be invalidated if backing-store files are manually deleted from a long-lived state root.
- Key components and relationships
  - `crates/vida/src/state_store.rs` owns default state-root selection and state-spine manifest validation.
  - `crates/vida/src/runtime_consumption_state.rs`, `crates/vida/src/status_surface.rs`, and `crates/vida/src/doctor_surface.rs` read and write runtime-consumption evidence under the state root.
  - `docs/process/project-operations.md` and `docs/process/environments.md` are the natural operator-facing homes for local-state hygiene guidance.
  - `.gitignore` currently ignores only `.vida/data/state/LOCK`, which leaves generated or broken state artifacts visible in the working tree.
- Current pain point or gap
  - The project has no explicit operator law for which parts of `.vida/data/state/**` are durable authority, which parts are disposable generated evidence, and when it is safe to clean or replace them.
  - Manual cleanup of backing-store directories can leave the local project in a non-bootable proof state even though the product logic is correct.
  - Fresh temp-state proofs and long-lived local state are both valid use cases, but the repo lacks one canonical workflow that distinguishes them clearly.
  - Some runtime surfaces require a project-bound state root, so a raw temp root that has only run `vida boot` is not sufficient for every proof path.
  - Git-visible runtime noise from `.vida/data/state/**` makes audit and review harder than necessary.

## Goal
- What this change should achieve
  - Define one canonical ops policy for `.vida/data/state/**`, including cleanup, reset, temp-state proof, and long-lived local-state handling.
  - Make it explicit which operator proofs should run against a fresh temp-state and which are expected to run against the project-local default state root.
  - Reduce accidental worktree noise and avoid manual backing-store cleanup that invalidates local `status` / `doctor` / `task` probes.
- What success looks like
  - Operators can tell, without guesswork, when to use a temp state via `VIDA_STATE_DIR` and when to keep the long-lived project state untouched.
  - The project docs explain the difference between authoritative store durability and disposable runtime evidence.
  - The repo has an explicit ignore/policy posture for generated state artifacts that should not be reviewed as product changes.
  - Follow-up implementation can add any missing guardrails without redesigning the runtime model.
- What is explicitly out of scope
  - Replacing the authoritative state store backend.
  - Redesigning Release-1 runtime-consumption semantics.
  - Reopening already-closed Release-1 feature epics.

## Requirements

### Functional Requirements
- Must define the canonical operator policy for `.vida/data/state/**`.
- Must distinguish at least these classes explicitly:
  - authoritative durable state spine and datastore backing files
  - generated runtime-consumption evidence
  - disposable temp-state proof roots created via `VIDA_STATE_DIR`
- Must define the supported cleanup/reset workflow for a broken long-lived local state root.
- Must define the preferred proof workflow for:
  - local functional scenario checks
  - release-proof / audit runs
  - project activation/bootstrap checks
- Must name the bounded code/doc surfaces that own state-root defaults, runtime-consumption snapshots, and operator guidance.
- Must define the intended git/working-tree posture for generated state artifacts.

### Non-Functional Requirements
- Performance
  - Policy changes must not add meaningful runtime overhead by default.
- Scalability
  - The same guidance should work for both routine local work and repeated temp-state proof runs.
- Observability
  - Operator docs must explain why `status` / `doctor` fail after backing-store deletion and how to recover safely.
- Security
  - The policy must preserve fail-closed behavior; broken or missing backing-store state must not silently degrade into false healthy output.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/ops-state-and-runtime-evidence-hygiene-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/process/project-operations.md`
  - `docs/process/environments.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `taskflow`
  - `status`
  - `doctor`
  - `project activation`
- Config / receipts / runtime surfaces affected:
  - `.gitignore`
  - `.vida/data/state/**`
  - `VIDA_STATE_DIR`
  - `vida status --json`
  - `vida doctor --json`
  - `vida task ...`
  - `vida taskflow consume final|continue|advance`
  - `vida project-activator`

## Design Decisions

### 1. State hygiene will be solved first as an explicit operator contract
Will implement / choose:
- Define the policy in docs/spec first, then add only the minimal code or repo guardrails required by that contract.
- Why
  - The current failure mode is mostly operational ambiguity, not missing runtime capability.
- Trade-offs
  - Documentation closure alone does not prevent every manual mistake until follow-up guardrails land.
- Alternatives considered
  - Patch `.gitignore` or runtime code ad hoc without a design packet.
  - Rejected because the repo needs a canonical distinction between durable state, generated evidence, and temp-state proofs.

### 2. Fresh temp-state proofs and long-lived project state will be treated as separate operating modes
Will implement / choose:
- Treat `VIDA_STATE_DIR=<temp>` as the canonical audit/proof sandbox and treat the default `.vida/data/state/` as the long-lived local operator state that should not be manually pruned in pieces.
- Why
  - The observed breakage came from mixing disposable-proof behavior with a long-lived authoritative store.
- Trade-offs
  - Some proof commands may need explicit env binding instead of relying on the default root.
- Alternatives considered
  - Keep using only the default project state for all proofs.
  - Rejected because it couples audit repeatability to mutable long-lived local state.

### 3. Generated state artifacts need an explicit repo-noise policy
Will implement / choose:
- Define which `.vida/data/state/**` paths should stay review-visible and which should be ignored or treated as non-product operational noise.
- Why
  - Current worktree noise obscures real code/doc changes and tempts manual cleanup in the wrong place.
- Trade-offs
  - Ignore rules must stay narrow enough not to hide canonical project assets accidentally.
- Alternatives considered
  - Leave all generated state visible and rely on operator discipline only.
  - Rejected because the recent state-store break showed that current discipline is not sufficient.

## Technical Design

### Core Components
- Main components
  - authoritative store bootstrap and manifest validation
  - runtime-consumption snapshot persistence and selection
  - operator-facing status/doctor/task probes
  - process docs and repo ignore policy
- Key interfaces
  - `resolve_state_dir(...)`
  - authoritative state-spine manifest validation in `state_store.rs`
  - runtime-consumption snapshot read/write helpers
  - `VIDA_STATE_DIR`
- Bounded responsibilities
  - `state_store.rs` owns authoritative-store boot semantics and fail-closed manifest validation.
  - `runtime_consumption_state.rs` owns generated runtime-consumption evidence under the selected state root.
  - process docs own the operator decision rule for temp-state versus long-lived state.
  - repo ignore policy owns working-tree noise boundaries for generated state.

### Data / State Model
- Important entities
  - authoritative state root
  - state-spine manifest
  - datastore backing files
  - runtime-consumption snapshots
  - temp proof state root
- Receipts / runtime state / config fields
  - `.vida/data/state/manifest/**`
  - `.vida/data/state/runtime-consumption/*.json`
  - `VIDA_STATE_DIR`
  - operator artifact refs surfaced by `status` and `doctor`
- Migration or compatibility notes
  - No schema migration is proposed in this packet.
  - Follow-up implementation may add reset/recovery guidance or guardrails, but must keep fail-closed behavior for broken backing stores.

### Integration Points
- APIs
  - none external
- Runtime-family handoffs
  - taskflow/direct-runtime-consumption outputs become status/doctor evidence under the selected state root.
  - project activation/bootstrap must remain explicit about when a state root is project-bound and reusable.
  - temp-state audits that exercise `taskflow consume bundle check` or similar project-bound runtime surfaces must include the matching activation/bootstrap path, not only `vida boot`.
- Cross-document / cross-protocol dependencies
  - Release-1 conformance and seam maps now treat functional closure as green, with datastore hygiene called out as the main operational caveat.

### Bounded File Set
- `docs/product/spec/ops-state-and-runtime-evidence-hygiene-design.md`
- `docs/product/spec/current-spec-map.md`
- `.gitignore`
- `docs/process/project-operations.md`
- `docs/process/environments.md`
- `crates/vida/src/state_store.rs`
- `crates/vida/src/runtime_consumption_state.rs`
- `crates/vida/src/status_surface.rs`
- `crates/vida/src/doctor_surface.rs`
- `crates/vida/src/taskflow_runtime_bundle.rs`
- `crates/vida/tests/boot_smoke.rs`

## Fail-Closed Constraints
- Forbidden fallback paths
  - No silent recreation of a partially deleted long-lived store as if prior proofs still existed.
  - No operator guidance that implies manual deletion of backing-store subdirectories is a normal maintenance path.
  - No broad ignore rules that hide canonical project source/doc changes.
- Required receipts / proofs / gates
  - Broken backing stores must remain visibly blocked on `status` / `doctor`.
  - Temp-state proof workflows must still produce explicit runtime-consumption evidence and operator-readable output.
- Safety boundaries that must remain true during rollout
  - Release-1 functional closure stays unchanged.
  - State hygiene improvements must remain an ops-layer hardening packet, not a stealth redesign of runtime semantics.

## Implementation Plan

### Phase 1
- Land this design doc and register it in the current spec map.
- First proof target
  - `./target/debug/vida docflow check docs/product/spec/ops-state-and-runtime-evidence-hygiene-design.md`

### Phase 2
- Update operator-facing docs with the explicit state-hygiene policy and temp-state workflow.
- Narrow the repo ignore/working-tree posture for generated state noise where justified by the policy.
- Second proof target
  - `./target/debug/vida docflow check docs/process/project-operations.md`
  - `./target/debug/vida docflow check docs/process/environments.md`

### Phase 3
- Implement any bounded runtime or test guardrails required by the approved policy.
- Validate temp-state and long-lived-state behavior with targeted runtime probes.
- Final proof target
  - targeted `cargo test -p vida` around state-store/runtime-consumption surfaces
  - fresh temp-state `vida boot`, `vida task ...`, `vida status --json`, and `vida doctor --json` checks

## Validation / Proof
- Unit tests:
  - targeted `cargo test -p vida` for state-store and runtime-consumption helpers when follow-up code changes land
- Integration tests:
  - targeted `boot_smoke` coverage for state-root selection and fail-closed operator surfaces
- Runtime checks:
  - fresh temp-state `vida boot`
  - fresh temp-state `vida task create ... --json`
  - fresh temp-state `vida status --json`
  - fresh temp-state `vida doctor --json`
- Canonical checks:
  - `./target/debug/vida docflow check docs/product/spec/ops-state-and-runtime-evidence-hygiene-design.md`
  - `./target/debug/vida docflow check docs/product/spec/current-spec-map.md`

## Observability
- Logging points
  - none proposed in this packet
- Metrics / counters
  - none proposed in this packet
- Receipts / runtime state written
  - normal runtime-consumption snapshots under the selected state root
  - no new receipt class in the design-only slice

## Rollout Strategy
- Development rollout
  - design and policy packet first
- Migration / compatibility notes
  - local broken stores may still require explicit reset/reinit once the operator policy is finalized
- Operator or user restart / restart-notice requirements
  - none in the design-only slice

## Future Considerations
- Follow-up ideas
  - add one explicit `vida` recovery/reset surface for broken local state roots
  - add machine-readable classification for disposable runtime evidence versus durable backing-store state
- Known limitations
  - this packet does not itself repair a previously broken `.vida/data/state/`
- Technical debt left intentionally
  - current worktree noise posture remains only partially specified until the follow-up doc/process patch lands

## References
- Related specs
  - `docs/product/spec/release-1-capability-matrix.md`
  - `docs/product/spec/release-1-conformance-matrix.md`
  - `docs/product/spec/release-1-seam-map.md`
  - `docs/product/spec/fix-release-admission-evidence-detection-artifac-design.md`
- Related protocols
  - none beyond current project ops/process canon
- Related ADRs
  - none
- External references
  - none

-----
artifact_path: product/spec/ops-state-and-runtime-evidence-hygiene-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-08
schema_version: 1
status: canonical
source_path: docs/product/spec/ops-state-and-runtime-evidence-hygiene-design.md
created_at: 2026-04-08T15:30:00+03:00
updated_at: 2026-04-08T06:53:51.194762855Z
changelog_ref: ops-state-and-runtime-evidence-hygiene-design.changelog.jsonl
