# Fix Status Surface External Cli Readiness Design

Purpose:
Fix the status surface readiness projection for hybrid runtime configurations where
internal host execution is enabled while external CLI backends are configured but
not required for the active route.

Status: `approved`

Problem:
`cargo test -p vida status_surface -- --nocapture` exposed two regressions:
`external_host_preserves_external_requirement_behavior` and
`internal_host_with_enabled_external_backends_is_hybrid_aware`. Both cases
expected the status surface to remain pass/admissible for the active internal
host route, while still reporting external CLI readiness as diagnostic or route
specific. The observed projection returned blocked, which over-constrained
normal runtime operation and kept TaskFlow recovery in a blocked state.

Bounded Scope:
- `crates/vida/src/status_surface_external_cli.rs`
- status-surface helpers that derive external CLI readiness and hybrid posture
- tests covering required external routes versus optional external CLI inventory

Expected Behavior:
- Required external CLI routes still fail closed when their required command is
  missing, unauthenticated, or incompatible.
- Internal host routes do not become blocked only because optional external CLI
  backends are configured and unavailable.
- Hybrid posture is visible in diagnostics without changing the active route
  admission result.
- Operator output stays compact: expose blocker codes and next actions only for
  the active blocking route, not every configured carrier.

Proof Targets:
- `cargo test -p vida status_surface -- --nocapture`
- focused tests:
  `external_host_preserves_external_requirement_behavior`
  `internal_host_with_enabled_external_backends_is_hybrid_aware`
- `vida status --json` shows external readiness diagnostics without blocking an
  internal-host active path.

Non-goals:
- Installing or authenticating optional external CLIs.
- Reworking carrier scoring or model-selection policy beyond the readiness
  projection needed for this defect.
- Expanding status output with verbose carrier dumps.

-----
artifact_path: product/spec/fix-status-surface-external-cli-readiness-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-05-16
schema_version: 1
status: canonical
source_path: docs/product/spec/fix-status-surface-external-cli-readiness-design.md
created_at: 2026-05-16T18:32:36.2352289Z
updated_at: 2026-05-16T18:34:49.7965895Z
changelog_ref: fix-status-surface-external-cli-readiness-design.changelog.jsonl
