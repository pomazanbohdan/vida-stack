# Internal Codex activation-view timeout holder release design

Purpose: Bound the current-release fix where stale read/status/resume reconciliation rewrites a legitimately executing internal-host delegated dispatch to terminal `internal_activation_view_only` after a fixed `10s`, even though live dispatch execution is still inside its canonical handoff timeout window.

Status: `proposed`

## Summary
- Feature / change: make stale in-flight reconciliation derive timeout truth from the same canonical handoff timeout contract used by live internal dispatch execution.
- Owner layer: `mixed`
- Runtime surface: `taskflow | launcher | recovery`
- Status: `proposed`

## Current Context
- `execute_and_record_dispatch_receipt(...)` records an in-flight runtime dispatch result with `execution_state = "executing"` and `stale_after_seconds = handoff_timeout_seconds`.
- That `stale_after_seconds` value is already computed from the canonical internal/external handoff timeout contract in `runtime_dispatch_state.rs`.
- Stale reconciliation in both `runtime_dispatch_state.rs` and `taskflow_consume_resume.rs` still compares result age against a separate fixed `10s` constant and then rewrites the receipt/result into terminal blocked truth.
- This causes read/status/resume surfaces to report `internal_activation_view_only` too early, even while the delegated internal-host dispatch is still legitimately executing.

## Goal
- Preserve `executing` state until the canonical handoff window recorded by live dispatch has actually elapsed.
- Normalize to blocked truth only after that same canonical window expires.
- Keep continuation and recovery prompts truthful by avoiding premature blocked/internal-activation classification.

## Requirements
- Must reuse the canonical timeout window already emitted by live dispatch execution instead of an independent stale heuristic.
- Must keep legacy artifacts readable by falling back safely when `stale_after_seconds` is absent.
- Must apply the same timeout value when stale normalization rewrites the result artifact into blocked truth so operator evidence stays consistent.
- Must cover both reconciliation entry points:
  - `runtime_dispatch_state.rs`
  - `taskflow_consume_resume.rs`

## Bounded File Set
- `docs/product/spec/internal-codex-activation-view-timeout-holder-release-design.md`
- `docs/product/spec/current-spec-map.md`
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/taskflow_consume_resume.rs`

## Technical Design
- Add one shared helper in `runtime_dispatch_state.rs` that reads `stale_after_seconds` from the runtime dispatch result artifact and falls back to the legacy `10s` default for older artifacts.
- Use that helper in stale reconciliation instead of the fixed constant.
- When stale reconciliation rewrites an artifact to blocked truth, pass through the same derived timeout seconds so the normalized `provider_error` and operator evidence remain truthful.
- Leave already-correct projection logic alone because `taskflow_run_graph.rs` already respects `stale_after_seconds` when deciding whether a dispatch looks stale.

## Validation / Proof
- Unit test: reconciliation does not rewrite a still-executing internal dispatch when `age_seconds < stale_after_seconds`.
- Unit test: reconciliation rewrites to terminal blocked truth once `age_seconds > stale_after_seconds`, and the normalized artifact reports that same timeout value.
- Compatibility test: artifacts without `stale_after_seconds` still use the legacy fallback window.

-----
artifact_path: product/spec/internal-codex-activation-view-timeout-holder-release-design
artifact_type: design_doc
artifact_version: '1'
artifact_revision: '2026-04-21'
schema_version: '1'
status: proposed
source_path: docs/product/spec/internal-codex-activation-view-timeout-holder-release-design.md
created_at: '2026-04-21T00:00:00Z'
updated_at: 2026-04-22T12:59:31.418999703Z
changelog_ref: internal-codex-activation-view-timeout-holder-release-design.changelog.jsonl
