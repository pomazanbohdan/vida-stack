# Prompt Lifecycle Evaluation And Safety Baseline Design

Status: `approved`

## Summary
- Feature / change: add a bounded prompt lifecycle, evaluation loop, and safety baseline for Release-1 runtime workflows.
- Owner layer: `runtime-family`
- Runtime surface: `taskflow`
- Status: `approved`

## Current Context
- Release-1 control law already requires:
  - prompt lifecycle and controlled rollout
  - process evaluation and feedback loop
  - safety defect gating and adversarial baseline coverage
- The current workspace has partial raw ingredients but no owned subsystem:
  - `crates/vida/src/agent_feedback_surface.rs` records host-agent feedback into local state and observability stores
  - `crates/vida/src/release1_contracts.rs` defines canonical artifact kinds for `evaluation_run` and `feedback_event`, but only `evaluation_run` has a concrete typed struct today
  - no prompt rollout registry or canary/promotion/rollback artifact is projected on an operator/runtime path
- The current gap is therefore not “no telemetry at all”, but that prompt-change safety and evaluation truth remains implicit and fragmented instead of being expressed through bounded machine-readable contracts.

## Goal
- Introduce the minimum Release-1 prompt/evaluation/safety baseline without inventing a full prompt-management platform.
- Make prompt rollout, evaluation, and feedback evidence machine-readable and operator-visible.
- Establish a bounded adversarial/safety baseline that later waves can deepen.
- Out of scope:
  - provider-specific prompt hosting or vendor prompt registries
  - full benchmark farm or large dataset orchestration
  - broad UI/dashboard work beyond bounded runtime/status surfaces

## Requirements

### Functional Requirements
- The runtime must support a bounded prompt rollout lifecycle with at least:
  - `draft`
  - `benchmarked`
  - `approved_for_rollout`
  - `canary`
  - `promoted`
  - `rolled_back`
- Prompt or policy changes must be representable through explicit runtime artifacts rather than free-form notes only.
- Evaluation and feedback ingestion must emit canonical machine-readable artifacts linked to workflow class and sample window.
- A first safety baseline must exist for runtime workflows using explicit adversarial/safety verdict fields instead of prose-only status.
- Operator-visible runtime surfaces must be able to project the bounded prompt/evaluation/safety posture.

### Non-Functional Requirements
- Keep the slice bounded to canonical contracts and directly affected runtime surfaces.
- Preserve compatibility with existing feedback state under `.vida/state/**`.
- Do not require network access, external benchmark vendors, or large migrations.
- Keep the first adversarial/safety baseline contract-first and fail-closed.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/prompt-lifecycle-evaluation-and-safety-baseline-design.md`
  - `docs/product/spec/README.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `release1_contracts`
  - host-agent feedback ingestion
  - status / operator summary projection
- Config / receipts / runtime surfaces affected:
  - `.vida/state/worker-scorecards.json`
  - `.vida/state/host-agent-observability.json`
  - bounded prompt/evaluation/safety runtime artifacts
  - status-family prompt/evaluation/safety summaries

## Design Decisions

### 1. Prompt Control Starts As Contracted Runtime Artifacts
Will implement / choose:
- represent the first prompt lifecycle through bounded canonical artifacts and JSON projections before wider rollout machinery
- Why:
  - Release-1 state-machine law already defines the lifecycle; the immediate gap is missing runtime ownership, not missing future platform ambition
- Trade-offs:
  - the first slice will be intentionally small and not yet a complete prompt deployment service
- Alternatives considered:
  - delay prompt lifecycle until post-release maturity; rejected because Release-1 closure law already names prompt rollout and regression gating

### 2. Feedback And Evaluation Must Share One Canonical Artifact Layer
Will implement / choose:
- close the gap between current `agent_feedback_surface` state writes and Release-1 artifact law by adding typed `feedback_event` and bounded evaluation projections
- Why:
  - current feedback exists, but it is not yet a first-class artifact contract that operator surfaces can reason about consistently
- Trade-offs:
  - some duplication may remain temporarily between local scorecard state and the new canonical artifacts
- Alternatives considered:
  - leave feedback only in local state and treat evaluation as later work; rejected because the task explicitly requires an evaluation loop around agent feedback

### 3. Safety Baseline Must Be Explicit Even If Minimal
Will implement / choose:
- add a bounded adversarial/safety summary that can fail closed on missing critical evidence instead of claiming “safe enough” through narrative notes
- Why:
  - Release-1 control metrics require explicit `safety_defect_rate` and prompt regression posture
- Trade-offs:
  - the first baseline will likely summarize safety posture at a coarse grain rather than providing a large scenario catalog
- Alternatives considered:
  - postpone safety until full benchmark tooling exists; rejected because the release gate is already part of active spec law

## Technical Design

### Core Components
- `crates/vida/src/release1_contracts.rs`
  - add typed canonical `feedback_event` support
  - add bounded helper summaries for prompt rollout, evaluation, and safety posture
- `crates/vida/src/agent_feedback_surface.rs`
  - project feedback ingestion through canonical feedback/evaluation-compatible artifacts
- directly affected status/operator surfaces
  - expose bounded prompt/evaluation/safety summaries alongside existing Release-1 control contracts

### Data / State Model
- Prompt rollout baseline fields:
  - lifecycle state
  - target workflow class
  - benchmark/evaluation artifact id
  - canary status
  - rollback target or rollback posture
- Feedback baseline fields:
  - feedback id
  - workflow class
  - source
  - score
  - outcome
  - task linkage
- Safety/evaluation baseline fields:
  - evaluation profile
  - dataset or sample window
  - regression summary
  - safety defect summary
  - promotion / rollback decision
- Compatibility note:
  - existing scorecard and observability state remains the durable source for local history while the new artifact layer provides machine-readable projection truth

### Integration Points
- `vida agent-feedback`
- automatic task-close feedback recording
- status/operator summary surfaces that already project Release-1 trust/control contracts
- Release-1 control metrics and prompt rollout FSM specs

### Bounded File Set
- `docs/product/spec/prompt-lifecycle-evaluation-and-safety-baseline-design.md`
- `docs/product/spec/README.md`
- `crates/vida/src/release1_contracts.rs`
- `crates/vida/src/agent_feedback_surface.rs`
- directly affected status / operator summary files

## Fail-Closed Constraints
- Do not treat prompt rollout as promoted when no evaluation artifact exists.
- Do not claim a safety baseline through prose-only notes or local score state alone.
- Do not silently infer promotion/canary/rollback semantics from arbitrary free-text close reasons.
- Do not break existing host-agent feedback recording while adding the canonical artifact layer.

## Implementation Plan

### Phase 1
- Finalize the bounded design and identify the smallest feedback/evaluation contract gap in code.
- First proof target:
  - canonical design document plus bounded file set

### Phase 2
- Add typed `feedback_event` support and bounded prompt/evaluation/safety summary helpers.
- Second proof target:
  - focused unit tests in `release1_contracts` and the first affected runtime surface

### Phase 3
- Project the new bounded summaries through feedback/status/operator surfaces and re-run targeted proofs.
- Final proof target:
  - green focused `cargo test -p vida` filters plus runtime JSON proof for the bounded prompt/evaluation/safety baseline

## Validation / Proof
- Unit tests:
  - focused `cargo test -p vida` filters around `release1_contracts` and `agent_feedback_surface`
- Integration tests:
  - bounded status/operator JSON checks for prompt/evaluation/safety projection
- Runtime checks:
  - inspect `vida agent-feedback --json` and affected status-family JSON after the bounded change
- Canonical checks:
  - `vida docflow finalize-edit docs/product/spec/prompt-lifecycle-evaluation-and-safety-baseline-design.md "record bounded prompt lifecycle, evaluation, and safety baseline design"`
  - `vida docflow check --root . docs/product/spec/prompt-lifecycle-evaluation-and-safety-baseline-design.md`

## Observability
- Feedback ingestion remains visible through `.vida/state/host-agent-observability.json`.
- The new artifact layer must keep evaluation, promotion, rollback, and safety posture queryable.
- Status/operator summaries must expose bounded prompt/evaluation/safety truth in machine-readable form.

## Rollout Strategy
- Land as a contract-first Release-1 control-maturity slice.
- Preserve current feedback storage and augment it with canonical projections rather than replacing it immediately.
- No restart requirement is expected for the bounded initial slice.

## Future Considerations
- Add richer canary cohort controls and benchmark-set management after the baseline contract is stable.
- Deepen adversarial suite coverage and defect clustering beyond the first operator/runtime baseline.
- Consolidate local scorecard history and canonical feedback artifacts if later waves need one storage truth.

## References
- `docs/product/spec/release-1-plan.md`
- `docs/product/spec/release-1-current-state.md`
- `docs/product/spec/release-1-control-metrics-and-gates.md`
- `docs/product/spec/release-1-state-machine-specs.md`
- `docs/product/spec/host-agent-layer-status-matrix.md`
- `crates/vida/src/agent_feedback_surface.rs`
- `crates/vida/src/release1_contracts.rs`

-----
artifact_path: product/spec/prompt-lifecycle-evaluation-and-safety-baseline-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-20
schema_version: 1
status: canonical
source_path: docs/product/spec/prompt-lifecycle-evaluation-and-safety-baseline-design.md
created_at: 2026-04-20T13:05:00+03:00
updated_at: 2026-04-20T10:01:28.4521634Z
changelog_ref: prompt-lifecycle-evaluation-and-safety-baseline-design.changelog.jsonl
