# Split Oversized Runtime Modules By Ownership Design

Status: `execution-preparation`

## Summary
- Feature / change: split oversized runtime modules by runtime ownership while preserving current behavior.
- Owner layer: `runtime-family`
- Runtime surface: `taskflow`, launcher shell `vida`
- Status: `execution-preparation baseline ready`

## Current Context
- The runtime request targets compatibility-preserving splits for oversized Rust files named in the packet: `runtime_dispatch_state`, `taskflow_consume_resume`, `taskflow_run_graph`, `taskflow_proxy`, `task_surface`, and `init_surfaces`.
- Current product law says `crates/vida/**` should remain shell-only for argument parsing, subcommand routing, and text/json rendering, while lane lifecycle, closure law, state transitions, run graph, execution receipts, and enforcement logic move below shell ownership.
- The active packet is a `specification` lane with doc-only owned scope. It must produce a bounded design gate, not implementation.

## Goal
- Define ownership-based split boundaries for the oversized runtime modules.
- Preserve operator-visible command behavior, JSON envelopes, receipt semantics, and runtime state compatibility during extraction.
- Produce proof targets for a later execution-preparation and implementation lane.
- Out of scope: moving code in this specification lane, changing runtime behavior, renaming public commands, or broad repository cleanup.

## Requirements

### Functional Requirements
- Keep launcher modules responsible only for CLI routing, shell composition, and rendering.
- Move TaskFlow lifecycle ownership below the shell:
  - dispatch state and validation,
  - consume/resume continuation,
  - run-graph status and recovery,
  - taskflow proxy/service boundaries,
  - task/surface operator contracts.
- Move bootstrap and init surface internals out of monolithic surface files into ownership-specific modules without changing the public command contract.
- Preserve all existing runtime packet, receipt, taskflow, docflow, and host bridge JSON fields unless a later design explicitly authorizes a schema change.
- Require an execution-preparation lane before implementation because this is architecture-sensitive, multi-module work.

### Non-Functional Requirements
- Compatibility: split modules by re-export/adapter seams first so downstream callers can migrate without a big-bang rewrite.
- Maintainability: every extracted module must have one owner responsibility and a small public API.
- Observability: command outputs, status surfaces, run-graph evidence, and receipt paths must remain inspectable before and after the split.
- Safety: no specification, coach, or verifier lane may gain code ownership from this design artifact.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/split-oversized-runtime-modules-ownership-execut-design.md`
  - `docs/product/spec/release-1-ownership-to-code-map.md`
  - `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`
  - `docs/product/spec/execution-preparation-and-developer-handoff-model.md`
  - `docs/product/spec/specification-lane-scope-hardening-design.md`
- Framework protocols affected:
  - none directly; execution must follow existing TaskFlow and lane handoff law.
- Runtime families affected:
  - `TaskFlow`
  - launcher shell `vida`
  - `DocFlow` only for documentation validation/proof.
- Config / receipts / runtime surfaces affected:
  - `.vida/data/state/runtime-consumption/**`
  - run-graph status and recovery status
  - dispatch result and receipt artifacts
  - taskflow/docflow operator JSON outputs

## Design Decisions

### 1. Split By Ownership, Not By Line Count
Will implement / choose:
- Extract ownership domains first, using size reduction as the secondary success signal.
- Why: oversized modules are risky because they mix runtime authorities, not only because they are long.
- Trade-offs: some files may remain moderately large until all callers migrate, but behavior stays compatible.
- Alternatives considered: mechanical file slicing by function groups; rejected because it can preserve the same owner confusion under new filenames.
- ADR link if needed: none for this bounded split.

### 2. Preserve Compatibility Through Facade Modules
Will implement / choose:
- Keep existing public module names as facades during the first implementation phase where callers are numerous.
- Move internals into owner-specific child modules and re-export stable entrypoints.
- Why: this lets implementation prove behavior before removing transitional facades.
- Trade-offs: temporary facade layers add one level of indirection.
- Alternatives considered: direct rename-only extraction; rejected because it increases call-site churn and proof blast radius.

### 3. Execution Preparation Is Mandatory Before Code Mutation
Will implement / choose:
- Route the next lane through `execution_preparation` before implementer work.
- Required outputs: architecture preparation report, developer handoff packet, change boundary, dependency impact summary, and spec alignment summary.
- Why: the packet targets multiple runtime modules and must reconcile code dependencies before safe implementation.
- Trade-offs: adds one lane before code movement, but prevents raw spec-to-worker drift.
- Alternatives considered: direct implementation from this design; rejected by the execution-preparation model for architecture-sensitive work.

## Technical Design

### Core Components
- `runtime_dispatch_state`
  - Target owner: dispatch packet state, validation, result/receipt state, host bridge dispatch truth.
  - Split direction: move validators, path policy, result/receipt helpers, and projection adapters into child modules.
- `taskflow_consume_resume`
  - Target owner: continuation consumption, resume reconciliation, legacy packet normalization, next-action binding.
  - Split direction: separate consume command orchestration from reconciliation policy and continuation projection.
- `taskflow_run_graph`
  - Target owner: run graph model, lane state, recovery/status queries, graph evidence.
  - Split direction: separate graph domain types, status rendering inputs, recovery classification, and persistence access.
- `taskflow_proxy`
  - Target owner: service/client boundary and taskflow command proxying.
  - Split direction: isolate transport/service-client concerns from taskflow domain semantics.
- `task_surface`
  - Target owner: task operator surface composition and user-facing command envelopes.
  - Split direction: keep CLI rendering at shell edge and move task lifecycle/domain logic below the shell.
- `init_surfaces`
  - Target owner: project/runtime initialization command surfaces and bootstrap projections.
  - Split direction: separate bootstrap discovery, project activation, template/materialization, and output rendering.

### Data / State Model
- Existing persisted state and JSON fields are compatibility contracts for this split.
- Any new internal module boundary must consume existing typed structs or introduce private helpers only.
- Schema additions require a separate design gate; this split should not require a migration.

### Integration Points
- CLI command routing in `crates/vida/**`
- TaskFlow runtime-family state and dispatch packet surfaces
- DocFlow validation for the design artifact
- Host-tool bridge request/result/receipt paths

### Bounded File Set
Expected implementation candidates for the next execution-preparation lane:
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/taskflow_consume_resume.rs`
- `crates/vida/src/taskflow_run_graph.rs`
- `crates/vida/src/taskflow_proxy.rs`
- `crates/vida/src/task_surface.rs`
- `crates/vida/src/init_surfaces.rs`
- new child modules under the same runtime-family ownership areas, named only after execution-preparation dependency mapping.

Specification-lane files changed:
- `docs/product/spec/split-oversized-runtime-modules-ownership-execut-design.md`
- `.vida/data/state/host-tool-bridge/results/architecture-refactor-oversized-module-split-specification-architecture-refactor-oversized-module-split-2026-06-03T03-55-38.7902773Z-host-tool-bridge.json`
- `.vida/data/state/host-tool-bridge/receipts/architecture-refactor-oversized-module-split-specification-architecture-refactor-oversized-module-split-2026-06-03T03-55-38.7902773Z-host-tool-bridge.json`

### Pre-Split Module Map

Baseline command: `Get-ChildItem crates/vida/src -Filter *.rs | Sort-Object Length -Descending | Select-Object -First 12 Name,KB`.

| Module | Baseline size | Current mixed ownership | First split boundary | Compatibility plan | Target proof |
| --- | ---: | --- | --- | --- | --- |
| `runtime_dispatch_state.rs` | 997.4 KB | dispatch packet paths, dispatch result persistence, receipt reconciliation, host bridge truth, projection helpers | extract path/result/receipt policy behind facade re-exports | keep `runtime_dispatch_state` as the public facade and move private helpers into child modules first | dispatch result and host bridge receipt tests |
| `taskflow_consume_resume.rs` | 809.4 KB | consume command orchestration, resume input resolution, packet normalization, reconciliation policy, recovery shaping | extract resume input resolution and reconciliation policy from command orchestration | preserve existing public consume/resume functions, migrate callers only after child-module tests pass | consume final/continue/resume tests and packet repair tests |
| `taskflow_run_graph.rs` | 599.3 KB | run graph status model, recovery summaries, closure projection, stale/terminal run classification | extract recovery classification and terminal-closure predicates | keep existing status builders as facade functions until projections prove identical | run-graph status, reconcile, closure projection tests |
| `task_surface.rs` | 511.9 KB | task lifecycle mutations, operator envelopes, progress/closure semantics, import/export surfaces | extract task mutation receipts and closure/progress policy from CLI surface routing | preserve command JSON shape and keep render-only code at shell edge | task close/progress/closure-ready/import tests |
| `taskflow_proxy.rs` | 486.5 KB | service/client proxying, scheduling projection, continuation binding, next-lawful decision policy | extract scheduling/continuation decision policy from transport proxy helpers | keep proxy command entrypoints stable, re-export typed decision builders | graph-summary, next-lawful, scheduler dispatch tests |
| `init_surfaces.rs` | 359.3 KB | orchestrator init, agent init, project activator bootstrap discovery, template/materialization, init JSON output | extract bootstrap discovery/materialization policy before output rendering | keep `vida orchestrator-init`, `vida agent-init`, and `vida project-activator` payload fields unchanged | init surface smoke tests and project activation tests |

Execution order:
1. Start with the smallest behavior-preserving facade extraction that has focused tests and low command-surface risk.
2. Prefer private-helper moves before public API moves.
3. After each extraction, record a post-split size and owner-boundary delta in this map.
4. Do not remove transitional facades until all callers and tests prove the new ownership path.

## Fail-Closed Constraints
- Do not begin implementation from this lane.
- Do not widen beyond the six named oversized module areas without a new packet.
- Do not move runtime truth into launcher rendering modules.
- Do not change public command names, JSON envelope shape, or receipt semantics as part of the split.
- Do not close implementation without pre-split and post-split module maps plus runtime/surface/targeted tests.
- If execution-preparation cannot produce a dependency impact summary, block implementation rather than guessing split order.

## Implementation Plan

### Phase 1
- Produce execution-preparation artifacts for the six named module areas.
- First proof target: pre-split module map with current owners, public entrypoints, dependencies, and known tests.

### Phase 2
- Implement compatibility-preserving facade splits for one ownership area at a time.
- Second proof target: targeted tests after each owner-area split plus unchanged operator JSON samples where available.

### Phase 3
- Remove only proven-dead transitional glue and finalize post-split ownership map.
- Final proof target: full runtime/surface proof ladder and release build/install smoke.

## Validation / Proof
- Unit tests:
  - targeted `cargo test` coverage for each moved owner-area behavior.
- Integration tests:
  - runtime dispatch, taskflow consume/resume, run graph, proxy/service, task surface, and init surface scenarios.
- Runtime checks:
  - pre-split module map,
  - post-split module map,
  - dispatch result artifact,
  - updated dispatch receipt,
  - `vida taskflow consume continue --run-id architecture-refactor-oversized-module-split --json`.
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/split-oversized-runtime-modules-ownership-execut-design.md`
  - `cargo test runtime`
  - `cargo test surfaces`
  - targeted `cargo test` for moved modules
  - release build and install smoke proof

## Observability
- Keep pre/post module maps as lane evidence.
- Keep bridge result and receipt artifacts tied to the request id.
- Preserve operator-visible JSON fields so run graph, continuation, and bridge status remain comparable before and after splitting.

## Rollout Strategy
- Start with documentation and execution-preparation evidence.
- Execute one ownership area at a time.
- Prefer facade-preserving moves before call-site cleanup.
- Gate each split on focused tests before moving to the next ownership area.
- Run the release build/install smoke only after all owner-area splits pass targeted proof.

## Future Considerations
- Promote a reusable oversized-module split checklist if this pattern repeats in `DocFlow` or other runtime-family crates.
- Consider adding automated module-size and owner-boundary diagnostics after the manual split is proven.

## References
- `docs/product/spec/release-1-ownership-to-code-map.md`
- `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`
- `docs/product/spec/execution-preparation-and-developer-handoff-model.md`
- `docs/product/spec/specification-lane-scope-hardening-design.md`
- `.vida/data/state/runtime-consumption/dispatch-packets/architecture-refactor-oversized-module-split-2026-06-03T03-55-38.7902773Z.json`

-----
artifact_path: product/spec/split-oversized-runtime-modules-ownership-execut-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-06-04
schema_version: 1
status: execution-preparation
source_path: docs/product/spec/split-oversized-runtime-modules-ownership-execut-design.md
created_at: 2026-06-03T03:55:38.7902773Z
updated_at: 2026-06-04T00:00:00Z
changelog_ref: split-oversized-runtime-modules-ownership-execut-design.changelog.jsonl
