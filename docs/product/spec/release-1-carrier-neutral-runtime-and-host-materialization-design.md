# Release-1 Carrier-Neutral Runtime And Host Materialization Design

Status: `approved`

Use this template for one bounded feature/change design before implementation.

Structured-template rule:
1. Keep headings stable.
2. Replace placeholders rather than rewriting the shape.
3. Prefer explicit fields and short bullets over long free-form prose.
4. Link separate ADRs when one or more major decisions need durable decision records.

## Summary
- Feature / change: remove codex-first carrier truth from launcher/runtime contracts and move host system, carrier catalog, runtime assignment, and materialization law into carrier-neutral activation and compiled bundle surfaces
- Owner layer: `project`
- Runtime surface: `launcher | activation | taskflow`
- Status: `approved`

## Current Context
- Existing system overview:
  - `vida.config.yaml` already models `host_environment.systems.*`, configured carriers, and agent-system posture.
  - host-system execution posture and status parity already have bounded config-driven neutral paths, compiled bundles no longer emit `codex_multi_agent`, canonical write paths already center on `carrier_runtime` plus `runtime_assignment`, and launcher routing no longer reads `codex_runtime_assignment`; the remaining debt is codex-heavy adapter branches, codex-era proof fixtures, and `.codex`-shaped materialization assumptions.
- Key components and relationships:
  - [`crates/vida/src/project_activator_surface.rs`](/home/unnamed/project/vida-stack/crates/vida/src/project_activator_surface.rs) owns host-system registry, materialization, and template rendering; execution class is now config-driven there, but codex-specific adapter branches still remain.
  - [`crates/vida/src/status_surface.rs`](/home/unnamed/project/vida-stack/crates/vida/src/status_surface.rs) now resolves selected systems without codex-only fallback and reads external/internal posture from config, but still loads `.codex` and keeps compat-heavy launcher fallbacks.
  - [`crates/vida/src/main.rs`](/home/unnamed/project/vida-stack/crates/vida/src/main.rs) still owns compatibility-sensitive runtime-assignment tests, pricing policy, and bundle assembly around the now-canonical neutral fields.
  - [`crates/vida/src/taskflow_consume_bundle.rs`](/home/unnamed/project/vida-stack/crates/vida/src/taskflow_consume_bundle.rs) still carries codex-era fixture coverage, but live compiled bundles are asserted not to emit `codex_multi_agent`.
  - [`crates/vida/src/taskflow_routing.rs`](/home/unnamed/project/vida-stack/crates/vida/src/taskflow_routing.rs) now resolves only canonical runtime-assignment fields, but launcher-owned routing and backend-selection law still sits above the family boundary.
  - [`crates/vida/src/taskflow_run_graph.rs`](/home/unnamed/project/vida-stack/crates/vida/src/taskflow_run_graph.rs) still carries route/backend fallback logic that reads legacy runtime-assignment aliases directly.
  - [`crates/vida/tests/boot_smoke.rs`](/home/unnamed/project/vida-stack/crates/vida/tests/boot_smoke.rs) and bundle-oriented proof paths still prove codex-era names, while [`crates/vida/tests/task_smoke.rs`](/home/unnamed/project/vida-stack/crates/vida/tests/task_smoke.rs) now includes bounded non-codex status parity coverage.
- Current pain point or gap:
  - launcher still owns carrier truth instead of activation + compiled bundle + routing contracts
  - adapter materialization and some proof fixtures remain codex-heavy even though canonical bundle/runtime writes are already neutral
  - codex-era proof fixtures and materialization assumptions still block full contract neutralization
  - Release-1 plan already has `r1-05-a`, `r1-05-b`, and `r1-08-*`, but this bounded implementation pack is not yet written down as one canonical design

## Goal
- What this change should achieve:
  - finish the migration onto already-landed carrier-neutral runtime contracts and burn down codex-era compatibility aliases
  - make host-system selection and materialization come from configured activation surfaces rather than launcher-local vendor defaults
  - keep vendor-specific carrier renderers as bounded adapters instead of canonical runtime law
- What success looks like:
  - compiled bundle exposes neutral `carrier_runtime` and `runtime_assignment` contracts, and consume/status views expose `carriers`, `dispatch_aliases`, and `worker_strategy`
  - status/init/activation surfaces can describe configured internal and external systems without defaulting to `.codex` or `codex`
  - routing and execution selection use neutral runtime-assignment DTOs
  - smoke/golden proofs validate neutral carrier/runtime contracts first and vendor-specific compatibility second
- What is explicitly out of scope:
  - swapping storage backends or implementing SierraDB in this slice
  - widening supported tool backends beyond already configured activation law
  - redesigning the whole TaskFlow execution planner beyond contract neutralization

## Requirements

### Functional Requirements
- Must-have behavior:
  - compiled bundles must stop using `codex_multi_agent` and `codex_runtime_assignment` as canonical field names
  - host-system discovery must resolve through configured `host_environment.systems.*` entries without implicit `codex` fallback as the only lawful default
  - runtime assignment and dispatch alias selection must be emitted through neutral machine-readable fields
  - materialization mode must be selected by configured host-system adapters, not launcher-wide `if system == "codex"` branches
  - proofs and smoke fixtures must gain a neutral path that does not require codex-specific bundle names or backend literals
- Integration points:
  - project activation/configurator surfaces
  - compiled runtime bundle generation and consume/status views
  - taskflow routing/runtime assignment logic
  - host status and readiness surfaces
  - release proof fixtures
- User-visible or operator-visible expectations:
  - `vida project-activator`, `vida status`, and `vida taskflow consume` must surface selected host systems and carrier assignments generically
  - operators should still be able to use codex and qwen compatibility paths, but those must read as configured carriers rather than as canonical runtime law

### Non-Functional Requirements
- Performance:
  - neutralization must not add extra process hops to routing or bundle generation
- Scalability:
  - carrier catalogs and dispatch aliases must stay bounded and sortable regardless of vendor count
- Observability:
  - local scorecards, budget telemetry, and host-agent feedback must remain available through carrier-neutral projections
- Security:
  - unsupported or missing host systems must fail closed through activation/bundle checks rather than silently falling back to codex-era defaults

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - [`docs/product/spec/release-1-plan.md`](/home/unnamed/project/vida-stack/docs/product/spec/release-1-plan.md)
  - [`docs/product/spec/release-1-current-state.md`](/home/unnamed/project/vida-stack/docs/product/spec/release-1-current-state.md)
  - [`docs/product/spec/release-1-conformance-matrix.md`](/home/unnamed/project/vida-stack/docs/product/spec/release-1-conformance-matrix.md)
  - [`docs/product/spec/compiled-runtime-bundle-contract.md`](/home/unnamed/project/vida-stack/docs/product/spec/compiled-runtime-bundle-contract.md)
  - [`docs/product/spec/bootstrap-carriers-and-project-activator-model.md`](/home/unnamed/project/vida-stack/docs/product/spec/bootstrap-carriers-and-project-activator-model.md)
  - [`docs/product/spec/host-agent-layer-status-matrix.md`](/home/unnamed/project/vida-stack/docs/product/spec/host-agent-layer-status-matrix.md)
- Framework protocols affected:
  - carrier/runtime assignment contract law only; no new workflow or checkpoint law in this slice
- Runtime families affected:
  - launcher activation/status/bundle/routing surfaces
  - TaskFlow runtime assignment and consume surfaces
  - host-agent carrier adapters
- Config / receipts / runtime surfaces affected:
  - `vida.config.yaml`
  - compiled runtime bundle payloads
  - host-agent local state ledgers
  - proof/golden fixtures

## Design Decisions

### 1. Carrier-neutral bundle contracts become the only canonical names
Will implement / choose:
- keep `carrier_runtime` and `runtime_assignment` as the canonical runtime outputs already emitted by the launcher
- keep consume/status snapshots centered on `carriers`, `dispatch_aliases`, and `worker_strategy`
- keep codex-era field names only as bounded compatibility aliases during migration
- Why:
  - Release-1 canon already requires one carrier-neutral runtime artifact pack
  - bundle identity cannot stay vendor-branded if activation law supports multiple systems
- Trade-offs:
  - bundle consumers and tests must migrate together
  - short compatibility bridge period is required
- Alternatives considered:
  - keep codex-era names as the canonical long-term contract
  - introduce a second neutral naming set on top of the fields already shipped
- ADR link if this must become a durable decision record:
  - add ADR only if the compatibility window becomes long-lived

### 2. Host-system materialization becomes adapter-driven
Will implement / choose:
- define host-system materializers through configured entries and explicit materialization modes
- keep codex rendering as one adapter, not the owner law
- Why:
  - current activator/status code is still `if system == "codex"` heavy
  - internal and external carriers must become first-class configured systems
- Trade-offs:
  - activator/status code needs a small refactor before behavior changes
- Alternatives considered:
  - retain launcher-local vendor branches
  - move all materialization into one generic copy-only path
- ADR link if needed:
  - none

### 3. Proof fixtures must be neutralized in lockstep with code contracts
Will implement / choose:
- add neutral golden fixtures before removing codex-era names from runtime outputs
- keep vendor-specific proofs as compatibility fixtures only
- Why:
  - Release-1 current-state explicitly calls codex-era proofs a blocker
  - runtime refactors without fixture neutralization would create false regressions or false greens
- Trade-offs:
  - requires touching smoke/golden suites in the same wave
- Alternatives considered:
  - rewrite code first and fix tests later
  - delete vendor-specific proofs entirely
- ADR link if needed:
  - none

### 4. Multi-backend proof matrix stays representative and carrier-neutral
Will implement / choose:
- add one bounded proof-matrix tranche that proves the same neutral dispatch contract across representative internal and external backend combinations instead of exhaustively enumerating every configured carrier
- keep the primary full-lane chain anchored on the canonical implementation route from `vida.config.yaml`: seeded implementation dispatch, analysis on `opencode_cli`, coach on `hermes_cli`, review/approval/rework progression on the verification route, and `internal_subagents` as the lawful fallback path when admissibility blocks an external primary backend
- keep matrix expectations focused on neutral machine-readable fields such as `selected_backend`, `dispatch_kind`, `resume_target`, `handoff_state`, `downstream_dispatch_ready`, and downstream trace/receipt parity
- Why:
  - this slice needs current-tree proof that mixed-backend orchestration preserves one neutral runtime contract across seed, handoff, fallback, approval wait, and rework re-entry
  - representative cells give durable coverage for launcher/runtime law without binding canon to vendor-specific naming or requiring one test per carrier
- Trade-offs:
  - the matrix must be intentionally small, so it proves representative cells rather than exhaustive backend permutations
  - some backend-specific fanout behavior remains covered by focused routing tests instead of a single giant e2e smoke
- Alternatives considered:
  - only linear run-graph smoke without backend lineage assertions
  - one exhaustive backend-by-backend matrix that duplicates routing and smoke fixtures across every carrier
- ADR link if needed:
  - none

## Technical Design

### Core Components
- Main components:
  - host-system registry/materializer helpers in `project_activator_surface.rs`
  - status projection helpers in `status_surface.rs`
  - compiled bundle builders and runtime assignment logic in `main.rs`
  - consume snapshot rendering in `taskflow_consume_bundle.rs`
  - routing adapters in `taskflow_routing.rs`
- Key interfaces:
  - configured host-system registry from `vida.config.yaml`
  - neutral compiled-bundle DTOs
  - neutral runtime-assignment DTOs
  - compatibility alias shims for codex-era outputs where still needed
- Bounded responsibilities:
  - activation owns configured systems and materialization posture
  - compiled bundle owns carrier/runtime assignment truth
  - routing consumes neutral assignment contracts
  - tests prove neutral contracts first

### Data / State Model
- Important entities:
  - configured host systems
  - carrier catalog entries
  - dispatch alias rows
  - runtime assignment records
  - local carrier strategy and scorecard ledgers
- Receipts / runtime state / config fields:
  - status surfaces should report selected system, runtime root, carrier selection, and scorecard stores without codex-only field names
  - compatibility payloads may temporarily mirror neutral assignment into old keys until all consumers are migrated
- Migration or compatibility notes:
  - preserve current codex and qwen compatibility behavior while changing canonical names underneath
  - provide one explicit bridge window where old fields are marked compatibility-only rather than owner-law canonical

### Integration Points
- APIs:
  - activation/bundle builders
  - status JSON projections
  - routing and consume payload assembly
- Runtime-family handoffs:
  - activation/config selection -> compiled bundle
  - compiled bundle -> TaskFlow routing/consume
  - host-agent completion/feedback -> local carrier telemetry
- Cross-document / cross-protocol dependencies:
  - `r1-02-c` carrier-neutral artifact pack
  - `r1-04-a` DB-first activation and bundle authority
  - `r1-05-a` host template/materializer abstraction
  - `r1-05-b` runtime assignment neutralization
  - `r1-08-a` and `r1-08-b` fixture/proof neutralization
  - current implementation routing in `vida.config.yaml` for the representative mixed-backend lane chain and fallback cells

### Bounded File Set
- Expected code changes:
  - [`crates/vida/src/runtime_dispatch_state.rs`](/home/unnamed/project/vida-stack/crates/vida/src/runtime_dispatch_state.rs)
- Likely follow-on tests and proof assets:
  - bounded mixed-backend/full-lane-chain assertions in [`crates/vida/src/runtime_dispatch_state.rs`](/home/unnamed/project/vida-stack/crates/vida/src/runtime_dispatch_state.rs)
  - carrier-neutral proof receipts in this design doc

## Fail-Closed Constraints
- Forbidden fallback paths:
  - no launcher-local silent fallback from an unknown host system to codex canonical behavior
  - no codex-specific field names presented as the only authoritative runtime contract
- Required receipts / proofs / gates:
  - bundle/status/consume outputs must remain machine-readable during migration
  - compatibility aliases must be explicitly marked and removable
- Safety boundaries that must remain true during rollout:
  - configured codex/qwen paths keep working while neutral contracts land
  - unsupported systems still fail through activator/bundle checks before execution

## Implementation Plan

### Phase 1
- Freeze neutral DTO names and compatibility alias map for bundle, assignment, and carrier catalog payloads.
- Replace launcher-local `codex_multi_agent` and `codex_runtime_assignment` canonical assumptions with canonical `carrier_runtime` and `runtime_assignment` builders plus explicit compatibility fallbacks.
- First proof target:
  - compiled bundle and consume/status projections expose neutral carrier/runtime keys.

### Phase 2
- Refactor host-system selection/materialization/status helpers so configured systems drive runtime roots and materialization modes.
- Reduce `if system == "codex"` branches to codex adapter hooks only.
- Second proof target:
  - project activator and status surfaces behave generically across configured systems.

### Phase 3
- Rewrite smoke/golden proofs around neutral contracts, then keep vendor-specific expectations only as compatibility coverage.
- Final proof target:
  - `r1-05-b` and the carrier-neutral portion of `r1-08-*` can close without codex-era canonical names.

### Phase 4
- Add the bounded multi-backend proof matrix for representative internal/external/fallback cells and one full-lane implementation chain.
- Prove downstream receipt and trace parity for mixed backend selection, approval wait, and rework return without introducing vendor-branded canonical fields.
- Final proof target:
  - representative backend combinations stay green under one neutral contract for lane-chain execution and fallback routing.

## Validation / Proof
- Unit tests:
  - neutral bundle builder fields
  - host-system registry/materialization selection
  - runtime-assignment selection over neutral DTOs
  - representative downstream backend selection and fallback cells for implementation, coach, and verification lanes
- Integration tests:
  - status/consume payloads for codex and qwen compatibility paths using the same neutral contracts
  - fail-closed unknown-system and missing-carrier cases
  - one representative mixed-backend full-lane chain covering implementation seed, downstream receipt progression, approval wait or rework continuity, and internal fallback parity
- Runtime checks:
  - `./target/debug/vida status --json`
  - `./target/debug/vida project-activator --json`
  - `./target/debug/vida taskflow consume final "<request>" --json`
  - `./target/debug/vida taskflow protocol-binding status --json`
- Canonical checks:
  - `./target/debug/vida docflow activation-check --root . docs/product/spec/release-1-carrier-neutral-runtime-and-host-materialization-design.md docs/product/spec/current-spec-map.md`
  - `./target/debug/vida docflow check --root . docs/product/spec/release-1-carrier-neutral-runtime-and-host-materialization-design.md docs/product/spec/current-spec-map.md`
  - `./target/debug/vida docflow doctor --root .`

## Observability
- Metrics / logs / traces to add or preserve:
  - selected host system
  - carrier selection and tier assignment
  - compatibility-alias usage rate for old codex-era fields
  - fail-closed activator/status/bundle denials for unknown systems
- Operator evidence expectations:
  - status and consume projections show the neutral assignment fields and selected configured system

## Rollout / Migration
- Sequence:
  - land neutral DTOs and compatibility aliases first
  - migrate bundle/status/consume/routing builders
  - neutralize smoke/golden fixtures
  - remove codex-era canonical names only after proof parity is green
- Backward-compatibility posture:
  - codex-era keys remain compatibility-only for one bounded migration window
  - codex/qwen vendor fixtures remain as explicit compatibility tests, not canon

## Open Questions
- Should carrier-neutral DTOs live in shared Release-1 contracts first or in a new host-agent/runtime-assignment crate before extraction from `crates/vida`?
- Which compatibility fields must remain visible in Release-1 operator surfaces versus internal bundle-only mirrors?
- Do we split `r1-05-a` and `r1-05-b` into separate code commits but one shared proof wave, or land them behind one compatibility adapter layer first?

-----
artifact_path: product/spec/release-1-carrier-neutral-runtime-and-host-materialization-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-04-08'
schema_version: '1'
status: canonical
source_path: docs/product/spec/release-1-carrier-neutral-runtime-and-host-materialization-design.md
created_at: '2026-04-03T14:10:00+03:00'
updated_at: '2026-04-08T13:00:00+03:00'
changelog_ref: release-1-carrier-neutral-runtime-and-host-materialization-design.changelog.jsonl
