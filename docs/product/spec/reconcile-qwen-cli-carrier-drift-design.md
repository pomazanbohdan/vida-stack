# Reconcile Qwen Cli Carrier Drift Design

Status: `approved`

## Summary
- Feature / change: reconcile stale `qwen_cli` drift across active docs, specs, and Rust test fixtures so the live project treats `qwen_cli` as template/reference-only and not as an active carrier.
- Owner layer: `mixed`
- Runtime surface: `taskflow | launcher | docflow`
- Status: approved

## Current Context
- Existing system overview
  - The live source-tree runtime already derives its active external carrier catalog from `vida.config.yaml`.
  - Source-tree runtime proof from `cargo run -p vida -- status --json` shows active route-primary external backends as `hermes_cli`, `kilo_cli`, `opencode_cli`, and `vibe_cli`, with no live `qwen_cli` entry.
  - Static repo search still finds `qwen_cli` in active process/spec docs and in Rust test fixtures that model carrier catalogs, routing, external preflight, and downstream execution summaries.
- Key components and relationships
  - Active runtime truth is projected through `vida.config.yaml` and runtime bundle/status surfaces.
  - Several Rust test modules still hardcode `qwen_cli` as a canonical backend in embedded YAML/config fixtures.
  - Several active docs/specs still describe `qwen_cli` as if it were part of the current live carrier catalog.
- Current pain point or gap
  - The repository currently carries drift between live runtime truth and surrounding docs/tests.
  - That drift weakens audit closure because it suggests a stale external backend is still active when the runtime has already moved on.
  - The user already fixed the policy decision: `qwen_cli` stays only in template/reference/example surfaces and must not be restored just to satisfy stale tests or stale docs.

## Goal
- What this change should achieve
  - Remove `qwen_cli` from live/current docs, current specs, and Rust test fixtures that claim to represent the active project configuration.
  - Preserve `qwen_cli` only where the surface is explicitly template/example/reference material.
  - Keep runtime/operator/test language aligned with the active carrier catalog derived from current config.
- What success looks like
  - Live/current docs no longer say `qwen_cli` is a current external carrier.
  - Runtime-facing tests stop expecting `qwen_cli` in active routing, preflight, or backend summaries.
  - Template/reference-only surfaces may still mention `qwen_cli`, but only where their non-active/reference role is explicit.
- What is explicitly out of scope
  - Reintroducing `qwen_cli` into `vida.config.yaml`.
  - Broad redesign of carrier-neutral runtime architecture beyond this drift-reconciliation slice.
  - Removing historical mention of `qwen_cli` from changelogs or intentionally archival/reference artifacts.

## Requirements

### Functional Requirements
- Must keep active runtime truth sourced from current config rather than stale carrier names.
- Must remove or rewrite active-doc statements that present `qwen_cli` as a current external carrier.
- Must update Rust test fixtures and assertions that currently require `qwen_cli` in live carrier/routing output.
- Must preserve coverage for the same runtime behaviors after replacing `qwen_cli` with current active backends or backend-neutral helpers.
- Must keep `qwen_cli` mentions only in template/example/reference surfaces where that status is explicit.

### Non-Functional Requirements
- Performance
  - No production runtime performance impact is expected; the main work is docs plus bounded test-fixture cleanup.
- Scalability
  - The chosen approach should avoid hardcoding yet another stale carrier name that will drift again after the next catalog change.
- Observability
  - Source-tree runtime/operator surfaces should continue to project the active carrier catalog clearly enough that drift is visible quickly.
- Security
  - The change must not widen active backend permissions or silently add new executors.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/reconcile-qwen-cli-carrier-drift-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/current-spec-provenance-map.md`
  - `docs/process/external-cli-carrier-operator-procedure.md`
  - `docs/process/agent-system.md`
  - `docs/product/spec/explicit-policy-selected-internal-backend-execut-design.md`
  - `docs/product/spec/hybrid-host-executor-semantics-host-environment-design.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `taskflow`
  - `launcher`
  - `docflow`
- Config / receipts / runtime surfaces affected:
  - runtime/operator carrier summaries
  - external CLI preflight summaries
  - test execution-plan fixtures
  - source-tree `status` / `orchestrator-init` proofs

## Design Decisions

### 1. Live runtime truth stays config-driven; the remediation targets stale projections around it
Will implement / choose:
- Treat the absence of `qwen_cli` from current `vida.config.yaml` and live source-tree status projections as authoritative.
- Why
  - The audit task is drift reconciliation, not a request to re-open backend selection policy.
- Trade-offs
  - Some older tests/docs lose a previously convenient example backend and need clearer fixtures or updated wording.
- Alternatives considered
  - Re-add `qwen_cli` to active config to satisfy stale parity checks.
  - Rejected because it violates the explicit user policy and reintroduces false live runtime truth.
- ADR link if this must become a durable decision record
  - none

### 2. Replace qwen-specific live-fixture assumptions with current-backend or backend-neutral fixtures
Will implement / choose:
- Update Rust tests that model active carrier/routing/preflight output so they use the current live backend set or a backend-neutral helper instead of `qwen_cli`.
- Why
  - The tests should validate behavior, not fossilize one removed backend as if it were still canonical.
- Trade-offs
  - Some assertions become a little more abstract or switch to a different active backend id.
- Alternatives considered
  - Leave qwen-only tests untouched because they are test-only.
  - Rejected because these tests currently claim to represent live routing/output assumptions.
- ADR link if needed
  - none

### 3. Active docs/specs must stop presenting qwen as current; template/reference surfaces may keep it explicitly
Will implement / choose:
- Rewrite or relocate active documentation references that describe `qwen_cli` as part of the current carrier catalog.
- Why
  - Active docs are part of operator/runtime truth, not archival notes.
- Trade-offs
  - Some examples become slightly less diverse unless a dedicated template/reference note preserves the qwen example.
- Alternatives considered
  - Keep the active docs unchanged and rely on readers to infer that qwen is historical.
  - Rejected because the current wording is not reference-marked and reads as live policy.
- ADR link if needed
  - none

## Technical Design

### Core Components
- Main components
  - active process/spec docs that still present `qwen_cli` as current
  - Rust test fixtures around status, init, routing, dispatch, consume/resume, and run-graph surfaces
  - source-tree runtime proofs confirming the active backend catalog
- Key interfaces
  - `cargo run -p vida -- status --json`
  - `cargo run -p vida -- orchestrator-init --json`
  - Rust fixture helpers and assertions under `crates/vida/src/**`
- Bounded responsibilities
  - docs/spec cleanup must align prose with current config-driven truth
  - tests must stop requiring `qwen_cli` as if it were active
  - no production runtime behavior should be widened just to satisfy stale parity debt

### Data / State Model
- Important entities
  - active external carrier ids
  - route-primary external backend lists
  - selected backend / executor backend projections
  - reference-only carrier examples
- Receipts / runtime state / config fields
  - `backend_id`
  - `executor_backend`
  - `route_primary_external_backends`
  - `selected_backend`
  - `why_this_unit`
- Migration or compatibility notes
  - The change is compatibility-neutral for live runtime state because it reconciles surrounding fixtures/docs to already-current config truth.

### Integration Points
- APIs
  - none external
- Runtime-family handoffs
  - config-driven carrier catalog -> init/status/operator summaries
  - routing/dispatch fixtures -> test assertions
  - documentation canon -> operator interpretation
- Cross-document / cross-protocol dependencies
  - active carrier catalogs are configuration-owned
  - materialized/test/example surfaces are not authority surfaces
  - template/reference retention must remain explicit

### Bounded File Set
- `docs/product/spec/reconcile-qwen-cli-carrier-drift-design.md`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/current-spec-provenance-map.md`
- `docs/process/external-cli-carrier-operator-procedure.md`
- `docs/process/agent-system.md`
- `docs/product/spec/explicit-policy-selected-internal-backend-execut-design.md`
- `docs/product/spec/hybrid-host-executor-semantics-host-environment-design.md`
- `crates/vida/src/init_surfaces.rs`
- `crates/vida/src/runtime_dispatch_execution.rs`
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/runtime_lane_summary.rs`
- `crates/vida/src/status_surface.rs`
- `crates/vida/src/status_surface_external_cli.rs`
- `crates/vida/src/status_surface_host_agents.rs`
- `crates/vida/src/taskflow_consume.rs`
- `crates/vida/src/taskflow_consume_resume.rs`
- `crates/vida/src/taskflow_routing.rs`
- `crates/vida/src/taskflow_run_graph.rs`

## Fail-Closed Constraints
- Forbidden fallback paths
  - Do not re-add `qwen_cli` to active config just to make tests/docs pass.
  - Do not leave active docs/specs ambiguously implying that qwen is still current.
- Required receipts / proofs / gates
  - Source-tree runtime proofs must continue showing the active external backend set without `qwen_cli`.
  - Targeted Rust tests must pass after replacing stale qwen-only assumptions.
- Safety boundaries that must remain true during rollout
  - Active carrier truth remains config-driven.
  - Template/example/reference mention of `qwen_cli` remains allowed only when clearly non-active.
  - No destructive migration of historical artifacts is required.

## Implementation Plan

### Phase 1
- Finalize this bounded design and register it in the current spec canon.
- Confirm the exact qwen drift split between active docs/specs and test fixtures.
- First proof target
  - `cargo run -p vida -- docflow check --root . docs/product/spec/reconcile-qwen-cli-carrier-drift-design.md`

### Phase 2
- Rewrite active docs/specs that still present `qwen_cli` as current.
- Update Rust test fixtures/assertions to use current active backends or backend-neutral helpers.
- Second proof target
  - targeted `cargo test -p vida ...` for status/init/routing/dispatch/consume-resume qwen drift cleanup

### Phase 3
- Re-run source-tree runtime/operator proofs and confirm no live `qwen_cli` expectation remains outside template/reference surfaces.
- Close the task only after docs, tests, and runtime evidence agree.
- Final proof target
  - source-tree `status` / `orchestrator-init` evidence plus green targeted tests

## Validation / Proof
- Unit tests:
  - status/external-preflight fixture tests
  - routing/backend-selection fixture tests
  - dispatch/consume/resume/run-graph fixture tests that currently require `qwen_cli`
- Integration tests:
  - none expected beyond existing targeted launcher/runtime tests
- Runtime checks:
  - `cargo run -p vida -- status --json`
  - `cargo run -p vida -- orchestrator-init --json`
- Canonical checks:
  - `cargo run -p vida -- docflow check --root . docs/product/spec/reconcile-qwen-cli-carrier-drift-design.md`
  - `cargo run -p vida -- docflow check --root . docs/product/spec/current-spec-map.md docs/product/spec/current-spec-provenance-map.md`

## Observability
- Logging points
  - none new expected
- Metrics / counters
  - none new expected
- Receipts / runtime state written
  - docflow changelog/readiness artifacts
  - runtime status/init proofs used as evidence

## Rollout Strategy
- Development rollout
  - land the docs/spec + test-fixture reconciliation in one bounded wave
- Migration / compatibility notes
  - no persisted-state migration expected
- Operator or user restart / restart-notice requirements
  - installed `vida` binaries still diverge, so source-tree `cargo run -p vida -- ...` remains the proof surface until the user explicitly requests a system-binary refresh

## Future Considerations
- Follow-up ideas
  - reduce future carrier-name drift by centralizing more test fixtures on helper-generated active backend sets
- Known limitations
  - archival/historical docs outside current canon may still mention `qwen_cli`, which is acceptable if they are clearly not current authority surfaces
- Technical debt left intentionally
  - this bounded slice does not refactor all carrier-fixture helpers into one shared generator unless needed by the concrete failing tests

## References
- `vida.config.yaml`
- `docs/process/agent-system.md`
- `docs/process/external-cli-carrier-operator-procedure.md`
- `docs/product/spec/explicit-policy-selected-internal-backend-execut-design.md`
- `docs/product/spec/hybrid-host-executor-semantics-host-environment-design.md`
- `crates/vida/src/{init_surfaces,runtime_dispatch_execution,runtime_dispatch_state,runtime_lane_summary,status_surface,status_surface_external_cli,status_surface_host_agents,taskflow_consume,taskflow_consume_resume,taskflow_routing,taskflow_run_graph}.rs`
- task `feature-reconcile-qwen-cli-carrier-drift-across-config-code`

-----
artifact_path: product/spec/reconcile-qwen-cli-carrier-drift-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-21
schema_version: 1
status: canonical
source_path: docs/product/spec/reconcile-qwen-cli-carrier-drift-design.md
created_at: 2026-04-21T18:09:02.303032316Z
updated_at: 2026-04-21T18:12:02.711563052Z
changelog_ref: reconcile-qwen-cli-carrier-drift-design.changelog.jsonl
