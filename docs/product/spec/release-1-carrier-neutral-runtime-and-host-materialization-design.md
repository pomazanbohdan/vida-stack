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
  - runtime execution, bundle identity, and many status/help proofs still center on `codex_multi_agent`, `codex_runtime_assignment`, `.codex`, and `codex` fallback behavior.
- Key components and relationships:
  - [`crates/vida/src/project_activator_surface.rs`](/home/unnamed/project/vida-stack/crates/vida/src/project_activator_surface.rs) owns host-system registry, materialization, and template rendering, but still special-cases `codex`.
  - [`crates/vida/src/status_surface.rs`](/home/unnamed/project/vida-stack/crates/vida/src/status_surface.rs) defaults to `codex`, loads `.codex`, and mixes carrier status with launcher-local fallbacks.
  - [`crates/vida/src/main.rs`](/home/unnamed/project/vida-stack/crates/vida/src/main.rs) still owns `codex_*` runtime-assignment logic, pricing policy, scorecard state, and bundle field names.
  - [`crates/vida/src/taskflow_consume_bundle.rs`](/home/unnamed/project/vida-stack/crates/vida/src/taskflow_consume_bundle.rs) publishes bundle snapshots from `activation_bundle["codex_multi_agent"]`.
  - [`crates/vida/src/taskflow_routing.rs`](/home/unnamed/project/vida-stack/crates/vida/src/taskflow_routing.rs) derives routing from codex-shaped assignment payloads.
  - [`crates/vida/tests/boot_smoke.rs`](/home/unnamed/project/vida-stack/crates/vida/tests/boot_smoke.rs) and [`crates/vida/tests/task_smoke.rs`](/home/unnamed/project/vida-stack/crates/vida/tests/task_smoke.rs) still prove codex-era names instead of carrier-neutral contracts.
- Current pain point or gap:
  - launcher still owns carrier truth instead of activation + compiled bundle + routing contracts
  - host-system fallback/materialization remains codex-first even when another system is selected
  - proof fixtures still pin codex-era names, which blocks safe contract neutralization
  - Release-1 plan already has `r1-05-a`, `r1-05-b`, and `r1-08-*`, but this bounded implementation pack is not yet written down as one canonical design

## Goal
- What this change should achieve:
  - replace codex-era carrier/bundle field names with carrier-neutral runtime contracts
  - make host-system selection and materialization come from configured activation surfaces rather than launcher-local vendor defaults
  - keep vendor-specific carrier renderers as bounded adapters instead of canonical runtime law
- What success looks like:
  - compiled bundle exposes neutral `carrier_catalog`, `runtime_assignment`, `dispatch_aliases`, and `worker_strategy` contracts
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

### 1. Carrier-neutral bundle contracts replace codex-era canonical names
Will implement / choose:
- introduce neutral bundle sections such as `carrier_catalog`, `runtime_assignment`, `dispatch_alias_catalog`, and `carrier_strategy`
- keep codex-era field names only as bounded compatibility aliases during migration
- Why:
  - Release-1 canon already requires one carrier-neutral runtime artifact pack
  - bundle identity cannot stay vendor-branded if activation law supports multiple systems
- Trade-offs:
  - bundle consumers and tests must migrate together
  - short compatibility bridge period is required
- Alternatives considered:
  - keep codex-era names and document them as historical
  - add neutral aliases without changing canonical names
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

### Bounded File Set
- Expected code changes:
  - [`crates/vida/src/project_activator_surface.rs`](/home/unnamed/project/vida-stack/crates/vida/src/project_activator_surface.rs)
  - [`crates/vida/src/status_surface.rs`](/home/unnamed/project/vida-stack/crates/vida/src/status_surface.rs)
  - [`crates/vida/src/main.rs`](/home/unnamed/project/vida-stack/crates/vida/src/main.rs)
  - [`crates/vida/src/taskflow_consume_bundle.rs`](/home/unnamed/project/vida-stack/crates/vida/src/taskflow_consume_bundle.rs)
  - [`crates/vida/src/taskflow_routing.rs`](/home/unnamed/project/vida-stack/crates/vida/src/taskflow_routing.rs)
- Likely follow-on tests and proof assets:
  - [`crates/vida/tests/boot_smoke.rs`](/home/unnamed/project/vida-stack/crates/vida/tests/boot_smoke.rs)
  - [`crates/vida/tests/task_smoke.rs`](/home/unnamed/project/vida-stack/crates/vida/tests/task_smoke.rs)
  - canonical fixture/golden assets introduced under the Release-1 proof contract

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
- Replace launcher-local `codex_multi_agent` and `codex_runtime_assignment` canonical references with neutral contract builders.
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

## Validation / Proof
- Unit tests:
  - neutral bundle builder fields
  - host-system registry/materialization selection
  - runtime-assignment selection over neutral DTOs
- Integration tests:
  - status/consume payloads for codex and qwen compatibility paths using the same neutral contracts
  - fail-closed unknown-system and missing-carrier cases
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
artifact_revision: '2026-04-03'
schema_version: '1'
status: canonical
source_path: docs/product/spec/release-1-carrier-neutral-runtime-and-host-materialization-design.md
created_at: '2026-04-03T14:10:00+03:00'
updated_at: '2026-04-03T14:10:00+03:00'
changelog_ref: release-1-carrier-neutral-runtime-and-host-materialization-design.changelog.jsonl
