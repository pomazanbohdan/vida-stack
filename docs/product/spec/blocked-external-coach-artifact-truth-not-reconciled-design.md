# Blocked External Coach Artifact Truth Not Reconciled Design

Purpose: Bound the current-release blocker where an external coach dispatch can remain canonically blocked by an obsolete `internal_activation_view_only` artifact even after backend-routing and timeout-return repairs land.

Status: `proposed`

## Summary
- Feature / change: reconcile blocked external coach dispatch truth so semantically obsolete blocked artifacts are surfaced as mismatched/stale and expose a lawful continuation path instead of a status-only loop.
- Owner layer: `mixed`
- Runtime surface: `taskflow | lane | status`
- Status: `proposed`

## Current Context
- Existing system overview
  - The active serialization run `feature-serialize-authoritative-state-access-lock-mitigation` is still blocked at `coach`.
  - The persisted dispatch receipt and run-graph summary carry `selected_backend = hermes_cli` and point at a blocked dispatch result artifact.
  - That dispatch result artifact still says `blocker_code = internal_activation_view_only` and `internal host carrier timed out`, which no longer matches the repaired external coach path.
- Key components and relationships
  - `crates/vida/src/taskflow_run_graph.rs` derives projection truth, stale-state suspicion, and next lawful operator action.
  - `crates/vida/src/taskflow_consume_resume.rs` decides whether blocked dispatch receipts can be retried, rebound, or must stay blocked.
  - `crates/vida/src/lane_surface.rs` exposes operator-visible lane truth and mutation affordances such as `supersede`.
  - `crates/vida/src/status_surface*.rs` project the same truth into operator envelopes.
- Current pain point or gap
  - Persisted blocked truth can become semantically obsolete without being marked stale or mismatched.
  - `next_lawful_operator_action` falls back to `vida taskflow run-graph status <run-id> --json`, creating a non-progressing polling loop.
  - The active serialization slice cannot continue lawfully because the canonical runtime still treats the obsolete blocked artifact as authoritative truth.

## Goal
- What this change should achieve
  - Detect when blocked dispatch result truth contradicts the selected backend or route semantics.
  - Surface that contradiction as operator-visible stale/mismatched state.
  - Offer a lawful continuation path that refreshes or rebinds blocked receipt truth without heuristic local takeover.
- What success looks like
  - External coach blocked artifacts no longer remain canonically classified as `internal_activation_view_only`.
  - Recovery/status surfaces stop reporting a pure status loop for this mismatch class.
  - The active serialization task can move past this blocker through canonical runtime continuation rather than notes-only diagnosis.
- What is explicitly out of scope
  - Implementing the serialization lock-mitigation feature itself.
  - Reopening MemPalace or any parked lane.
  - Broad redesign of exception takeover law.

## Requirements

### Functional Requirements
- Must treat a blocked dispatch result artifact as semantically stale or mismatched when its blocker semantics contradict the persisted selected backend or carrier class.
- Must make the operator-visible projection truth reflect that mismatch instead of claiming clean parity.
- Must provide one lawful continuation path for this mismatch class through existing runtime/operator surfaces.
- Must preserve fail-closed behavior: no hidden local-write activation, no silent artifact deletion, and no heuristic success projection.

### Non-Functional Requirements
- Observability
  - Status, recovery, or lane surfaces must explain when blocked truth is mismatched or stale.
- Safety
  - The fix must not auto-activate exception takeover or root-local write.
- Compatibility
  - Existing retry semantics for genuine `configured_backend_dispatch_failed` and `timeout_without_takeover_authority` receipts must remain intact.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/blocked-external-coach-artifact-truth-not-reconciled-design.md`
  - `docs/product/spec/current-spec-map.md`
- Runtime families affected:
  - `taskflow`
  - `lane`
  - `status`
- Config / receipts / runtime surfaces affected:
  - `vida taskflow consume continue --run-id <id> --json`
  - `vida taskflow run-graph status <id> --json`
  - `vida lane show <id> --json`

## Design Decisions

### 1. Mismatch detection belongs in projection truth, not only in ad hoc recovery notes
Will implement / choose:
- Extend run-graph projection truth to recognize blocked semantic mismatches, not only age-based stale executing artifacts.
- Why
  - The current stale-state detector only fires for `dispatch_status == executing`, so blocked semantic drift stays invisible.
- Trade-offs
  - Projection truth becomes slightly richer and must read blocked result artifacts when present.

### 2. Operator continuation must advance beyond a pure status loop
Will implement / choose:
- Introduce a bounded continuation/reconciliation path when the blocked receipt is semantically obsolete.
- Why
  - Recommending `run-graph status` forever is not a lawful continuation path for an already-diagnosed mismatch.
- Trade-offs
  - The runtime must distinguish “ordinary blocked” from “blocked but semantically obsolete”.

## Technical Design

### Core Components
- Main components
  - `taskflow_run_graph.rs`
  - `taskflow_consume_resume.rs`
  - `lane_surface.rs`
  - adjacent status/reporting helpers as needed
- Key interfaces
  - projection-truth derivation
  - blocked receipt retry/rebind preparation
  - lane/status operator envelope generation
- Bounded responsibilities
  - run-graph truth detects mismatch and exposes it
  - consume/resume uses that truth to avoid a dead status loop
  - lane/status surfaces mirror the same canonical diagnosis

### Bounded File Set
- `docs/product/spec/blocked-external-coach-artifact-truth-not-reconciled-design.md`
- `docs/product/spec/current-spec-map.md`
- `crates/vida/src/taskflow_run_graph.rs`
- `crates/vida/src/taskflow_consume_resume.rs`
- `crates/vida/src/lane_surface.rs`
- `crates/vida/src/status_surface_json_report.rs`
- `crates/vida/src/status_surface_text_report.rs`

## Fail-Closed Constraints
- Forbidden fallback paths
  - no artifact deletion
  - no automatic exception takeover activation
  - no pretending stale blocked truth is resolved without new receipt-backed evidence
- Required receipts / proofs / gates
  - operator surfaces must show mismatch/stale posture explicitly
  - recovery/continue must remain receipt-backed

## Implementation Plan

### Phase 1
- Register this bounded blocker design in the current spec map.
- First proof target
  - `vida docflow check --root . docs/product/spec/blocked-external-coach-artifact-truth-not-reconciled-design.md docs/product/spec/current-spec-map.md`

### Phase 2
- Teach projection truth and consume/resume logic to recognize and surface blocked semantic mismatch.
- Second proof target
  - targeted `cargo test -p vida` for run-graph and consume-resume mismatch handling

### Phase 3
- Align lane/status operator output with the same diagnosis and rebuild the installed binary.
- Final proof target
  - targeted tests plus `cargo build --release -p vida`

## Validation / Proof
- Unit tests:
  - new tests for blocked semantic mismatch detection
  - new tests for lawful continuation action selection under mismatched blocked truth
- Runtime checks:
  - `vida taskflow run-graph status feature-serialize-authoritative-state-access-lock-mitigation --json`
  - `vida taskflow consume continue --run-id feature-serialize-authoritative-state-access-lock-mitigation --json`
  - `vida lane show feature-serialize-authoritative-state-access-lock-mitigation --json`

## References
- `docs/product/spec/serialize-authoritative-state-access-lock-mitigation-design.md`
- `docs/product/spec/coach-lane-inherits-internal-fallback-over-explicit-review-route-design.md`
- `docs/product/spec/external-coach-timeout-truth-does-not-return-cleanly-design.md`
- `docs/product/spec/lane-supersede-and-shared-truth-envelope-design.md`

-----
artifact_path: product/spec/blocked-external-coach-artifact-truth-not-reconciled-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-21
schema_version: 1
status: canonical
source_path: docs/product/spec/blocked-external-coach-artifact-truth-not-reconciled-design.md
created_at: 2026-04-21T12:57:15.709372253Z
updated_at: 2026-04-21T12:58:45.27747414Z
changelog_ref: blocked-external-coach-artifact-truth-not-reconciled-design.changelog.jsonl
