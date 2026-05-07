# Continue GitHub #116 Orchestrator Session Identity Design Gate

Status: `approved`

## Summary
- Feature / change: tracked design-gate wrapper for the `github-116-orchestrator-session-identity` architecture slice.
- Owner layer: `runtime-family`
- Runtime surface: `launcher | taskflow | status | lane | project activation | docflow`
- Canonical design: `docs/product/spec/orchestrator-session-environment-identity-design.md`
- Status: `approved for work-pool and developer handoff`

## Current Context
- The active bounded request is: finalize design/spec evidence and prepare the developer handoff for first-class orchestrator session/environment identity and ownership leasing.
- GitHub issue #116 is a deeper architecture follow-up to the broader post-commit/runtime-reconciliation umbrella tracked in GitHub issue #114.
- The packet path for this lane points at this wrapper document, so the wrapper must describe the orchestrator-owner identity slice rather than the older continuation/run-graph drift handoff.
- The canonical architecture design already exists in `docs/product/spec/orchestrator-session-environment-identity-design.md`.
- The remaining gap for this lane is explicit issue-aligned handoff truth so the next developer/work-pool lane can implement the owner/lease model without spec/doc ambiguity.

## Design Gate Decision
- Use `docs/product/spec/orchestrator-session-environment-identity-design.md` as the canonical design for this bounded `#116` architecture slice.
- Treat this file as the tracked handoff wrapper expected by the active run `github-116-orchestrator-session-identity`.
- Keep scope limited to:
  - orchestrator session/environment identity derivation
  - lease-backed owner evidence on continuation-affecting runtime mutations
  - latest/recovery/continuation/status diagnostics scoped or labeled by owner session
  - explicit reclaim/transfer command and receipt path for stale/live owner conflicts
  - execution-context versus owner/publication-context separation
- Out of scope:
  - general TaskFlow scheduler redesign
  - replacing lane/carrier/runtime-role semantics with session ownership
  - host-private UI close APIs

## Developer Handoff
- Active bounded unit:
  - add first-class orchestrator session/environment identity and lease-backed owner evidence so concurrent/stale orchestrators stop sharing one ambiguous project-level latest state
- Why this unit:
  - GitHub issue #116 shows that post-commit diagnostics, recovery latest, and consume-continue can combine evidence from different orchestrators; the runtime needs owner-aware state mutation, selection, and recovery semantics before more local reconciliations are trustworthy
- Sequential vs parallel posture:
  - `sequential`
  - this slice mutates shared state-store identity, latest selection, diagnostics, and reclaim semantics and should land as one bounded implementation packet before adjacent continuation or diagnostics repairs

## Bounded Implementation Scope
- Primary code surfaces:
  - `crates/vida/src/orchestrator_session_identity.rs`
  - `crates/vida/src/state_store.rs`
  - `crates/vida/src/state_store_run_graph_state.rs`
  - `crates/vida/src/state_store_run_graph_summary.rs`
  - `crates/vida/src/init_surfaces.rs`
  - `crates/vida/src/status_surface.rs`
  - `crates/vida/src/status_surface_truth_inputs.rs`
  - `crates/vida/src/status_surface_json_report.rs`
  - `crates/vida/src/status_surface_text_report.rs`
  - `crates/vida/src/taskflow_run_graph.rs`
  - `crates/vida/src/taskflow_consume_resume.rs`
  - `crates/vida/src/taskflow_continuation.rs`
  - `crates/vida/src/lane_surface.rs`
  - `crates/vida/src/runtime_consumption_state.rs`
- Canonical docs:
  - `docs/product/spec/orchestrator-session-environment-identity-design.md`
  - `docs/product/spec/continue-github-116-orchestrator-session-environ-design.md`
  - `docs/product/spec/github-114-design-document-deterministic-post-design.md`

## Acceptance Targets
- `vida orchestrator-init --json` reports current owner identity plus active/stale sibling orchestrator sessions for the same state root.
- Continuation-affecting runtime mutations persist `RuntimeOwnerEvidence` for the owning orchestrator session/environment identity.
- Latest TaskFlow/run-graph/lane projections are either current-session scoped or explicitly labeled global/cross-session.
- Cross-session continuation ambiguity returns a specific owner-conflict blocker instead of generic ambiguity/tool-failure output.
- An explicit recovery path exists to reclaim or transfer stale orchestrator ownership with a persisted receipt.
- Post-commit and epic-closure diagnostics include a session/environment binding check.

## Proof Evidence
- Canonical design proof:
  - `vida docflow check --root . docs/product/spec/orchestrator-session-environment-identity-design.md`
- Wrapper proof:
  - `vida docflow check --root . docs/product/spec/continue-github-116-orchestrator-session-environ-design.md`
- Developer proof targets for the next lane:
  - focused regression coverage for session identity derivation, owner evidence persistence, session-aware latest gating, and reclaim/transfer receipts
  - `vida orchestrator-init --json`
  - `vida taskflow consume continue --json`
  - `vida status --json`
  - `vida taskflow recovery latest --json`
  - `vida taskflow session list --json`
  - `vida taskflow session reclaim --session-id <id> --reason <text> --json`

## Handoff Readiness
- This design gate is complete once this wrapper is docflow-valid and cites the canonical orchestrator-session-identity design above.
- The next lawful lane is the tracked developer/work-pool handoff for the bounded runtime implementation of the owner/lease model.
- Do not collapse this slice back into a generic continuation-reconciliation packet unless a separate packet explicitly supersedes the active #116 scope.

-----
artifact_path: product/spec/continue-github-116-orchestrator-session-environ-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-05-07
schema_version: 1
status: canonical
source_path: docs/product/spec/continue-github-116-orchestrator-session-environ-design.md
created_at: 2026-05-07T00:00:00+03:00
updated_at: 2026-05-07T14:14:32.7836875Z
changelog_ref: continue-github-116-orchestrator-session-environ-design.changelog.jsonl
