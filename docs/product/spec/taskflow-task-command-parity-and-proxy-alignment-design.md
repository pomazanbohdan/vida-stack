# TaskFlow Task Command Parity And Proxy Alignment Design

Status: `approved`

Use this template for one bounded feature/change design before implementation.

Structured-template rule:
1. Keep headings stable.
2. Replace placeholders rather than rewriting the shape.
3. Prefer explicit fields and short bullets over long free-form prose.
4. Link separate ADRs when one or more major decisions need durable decision records.

## Summary
- Feature / change: align `vida task` and `vida taskflow task` so backlog mutation, display-id reservation, and help/proxy behavior follow one canonical command contract
- Owner layer: `project`
- Runtime surface: `launcher | taskflow`
- Status: `approved`

## Current Context
- Existing system overview:
  - `vida task` is the root CLI home for authoritative task-store inspection over the DB-backed state store.
  - `vida taskflow task` is a launcher-owned proxy/bridge that currently exposes the richer mutation surface.
- Key components and relationships:
  - [`crates/vida/src/cli.rs`](/home/unnamed/project/vida-stack/crates/vida/src/cli.rs) defines the root `vida task` subcommands.
  - [`crates/vida/src/task_surface.rs`](/home/unnamed/project/vida-stack/crates/vida/src/task_surface.rs) executes root task commands directly against `StateStore`.
  - [`crates/vida/src/taskflow_task_bridge.rs`](/home/unnamed/project/vida-stack/crates/vida/src/taskflow_task_bridge.rs) parses `vida taskflow task ...` and contains extra mutation logic and helper rendering.
  - [`crates/vida/src/taskflow_layer4.rs`](/home/unnamed/project/vida-stack/crates/vida/src/taskflow_layer4.rs) publishes TaskFlow help text that already advertises create/update/close flows not present in root `vida task`.
- Current pain point or gap:
  - root `vida task` lacks `create`, `update`, `close`, `next-display-id`, and `export-jsonl`
  - mutation parity lives in the proxy bridge instead of the root surface
  - help/proxy text points operators toward `vida taskflow task` as the practical mutation home
  - parser and validation logic are duplicated between the direct root surface and the bridge

## Goal
- What this change should achieve:
  - make `vida task` the canonical backlog inspection and mutation surface for the authoritative task store
  - keep `vida taskflow task` as a compatibility/proxy entrypoint with the same behavior and output contract
  - eliminate hidden bridge-only mutation capability and help drift
- What success looks like:
  - operators can create, update, close, export, and reserve display ids from `vida task`
  - `vida task --help` and `vida taskflow help task` describe the same lawful capability set
  - bridge routing no longer owns distinct mutation semantics
  - all command variants return canonical status/output contracts
- What is explicitly out of scope:
  - redesign of issue types beyond existing `epic` and `task`
  - broader `consume`, `lane`, `approval`, or `recovery` root-surface work
  - TaskFlow runtime-family extraction beyond the task command slice

## Requirements

### Functional Requirements
- Must-have behavior:
  - add root `vida task` subcommands for `create`, `update`, `close`, `next-display-id`, and `export-jsonl`
  - preserve existing root query surfaces `list`, `show`, `ready`, `deps`, `blocked`, `tree`, `validate-graph`, `dep`, and `critical-path`
  - support the same mutation flags already available through `vida taskflow task`
  - keep `vida taskflow task` working as a compatibility alias/proxy over the same shared command law
- Integration points:
  - `StateStore` task mutation APIs
  - proxy routing in `run_taskflow_proxy`
  - TaskFlow help/operator recipes and bootstrap/runtime packet references
- User-visible or operator-visible expectations:
  - `vida task --help` must expose the full canonical task-store capability set
  - `vida taskflow help task` must stop implying a separate richer bridge-only task API
  - JSON outputs must stay fail-closed and canonical

### Non-Functional Requirements
- Performance:
  - command execution must stay direct over the authoritative state store without adding external process hops
- Scalability:
  - command parsing and execution must remain bounded for large task graphs
- Observability:
  - command outputs must preserve canonical status rendering and existing host-agent close telemetry hooks
- Security:
  - unsupported arguments, missing roots, invalid transitions, and bad display-id inputs must remain fail-closed

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - [`docs/product/spec/release-1-plan.md`](/home/unnamed/project/vida-stack/docs/product/spec/release-1-plan.md)
  - [`docs/product/spec/taskflow-task-command-parity-and-proxy-alignment-design.md`](/home/unnamed/project/vida-stack/docs/product/spec/taskflow-task-command-parity-and-proxy-alignment-design.md)
- Framework protocols affected:
  - command/help parity only; no protocol-law rewrite
- Runtime families affected:
  - launcher root `vida task`
  - launcher-owned `vida taskflow task` compatibility bridge
- Config / receipts / runtime surfaces affected:
  - `.vida/data/state/**`
  - task close host-agent telemetry payloads

## Design Decisions

### 1. Root `vida task` becomes the canonical mutation surface
Will implement / choose:
- root `vida task` owns both inspection and mutation for authoritative task-store commands
- `vida taskflow task` remains a compatibility entrypoint, not the richer primary home
- Why:
  - removes operator confusion and hidden bridge ownership
  - matches the existing root command family name and DB-backed authority model
- Trade-offs:
  - requires expanding clap surface and tests
  - leaves temporary dual entrypoints until the proxy can be reduced further
- Alternatives considered:
  - keep mutation only in `vida taskflow task`
  - add shell docs only without root command parity
- ADR link if this must become a durable decision record:
  - none

### 2. Shared task command law must back both root and proxy
Will implement / choose:
- extract or reuse one shared execution layer for task-store mutations and helper payload generation
- proxy parsing may stay, but it must delegate into the same shared command/executor path used by `vida task`
- Why:
  - prevents drift between two parsers and two output contracts
  - keeps help/proxy cleanup small and bounded
- Trade-offs:
  - requires small refactor around `task_surface.rs` and `taskflow_task_bridge.rs`
- Alternatives considered:
  - duplicate new root handlers independently
  - route root commands through the proxy layer instead of direct shared handlers
- ADR link if needed:
  - none

## Technical Design

### Core Components
- Main components:
  - clap root task command definitions in `cli.rs`
  - shared task command execution helpers in `task_surface.rs` or a small adjacent shared module
  - proxy/bridge delegation in `taskflow_task_bridge.rs`
  - operator/help text in `taskflow_layer4.rs`
- Key interfaces:
  - `StateStore::{create_task, update_task, close_task, export_tasks_to_jsonl, list_tasks}`
  - shared display-id helper payload generation
- Bounded responsibilities:
  - root task surface owns canonical CLI shape
  - bridge owns compatibility parsing only
  - help surfaces describe one command law

### Data / State Model
- Important entities:
  - task records
  - task dependencies
  - next display-id payloads
- Receipts / runtime state / config fields:
  - task close must keep emitting host-agent telemetry payloads
  - no new config schema is required
- Migration or compatibility notes:
  - `vida taskflow task ...` remains accepted during the migration window
  - recommended command strings in launcher/operator surfaces should progressively prefer `vida task ...`

### Integration Points
- APIs:
  - clap subcommand parsing
  - `StateStore` async task APIs
- Runtime-family handoffs:
  - `run_taskflow_proxy` to `run_taskflow_task_bridge`
- Cross-document / cross-protocol dependencies:
  - `r1-01-a` and `r1-01-d` in the Release-1 TaskFlow program

### Bounded File Set
- Expected code changes:
  - [`crates/vida/src/cli.rs`](/home/unnamed/project/vida-stack/crates/vida/src/cli.rs)
  - [`crates/vida/src/task_surface.rs`](/home/unnamed/project/vida-stack/crates/vida/src/task_surface.rs)
  - [`crates/vida/src/taskflow_task_bridge.rs`](/home/unnamed/project/vida-stack/crates/vida/src/taskflow_task_bridge.rs)
  - [`crates/vida/src/taskflow_layer4.rs`](/home/unnamed/project/vida-stack/crates/vida/src/taskflow_layer4.rs)
  - [`crates/vida/src/taskflow_proxy.rs`](/home/unnamed/project/vida-stack/crates/vida/src/taskflow_proxy.rs)
- Likely follow-on string/help alignment:
  - [`crates/vida/src/main.rs`](/home/unnamed/project/vida-stack/crates/vida/src/main.rs)
  - [`crates/vida/src/init_surfaces.rs`](/home/unnamed/project/vida-stack/crates/vida/src/init_surfaces.rs)

## Fail-Closed Constraints
- Forbidden fallback paths:
  - no bridge-only mutation capability hidden from root `vida task`
  - no silent fallback to detached JSONL snapshots as live state
- Required receipts / proofs / gates:
  - close-path host-agent telemetry must remain intact
  - JSON status fields must remain canonical
- Safety boundaries that must remain true during rollout:
  - unsupported flags must still fail closed
  - root resolution and state-store open semantics must stay unchanged

## Implementation Plan

### Phase 1
- Add missing root `vida task` clap subcommands and argument structs.
- Extract shared task-store mutation/display-id/export handlers.
- First proof target:
  - `vida task --help` exposes the full canonical command set.

### Phase 2
- Rewire `vida taskflow task` bridge onto the shared handlers instead of separate mutation semantics.
- Align `vida taskflow help task` and proxy error text with the new root canonical surface.
- Second proof target:
  - root and proxy variants return equivalent results for `create`, `update`, `close`, and `next-display-id`.

### Phase 3
- Update command-string references that still recommend bridge-first task mutation.
- Add command/help regression tests for parity and fail-closed behavior.
- Final proof target:
  - `r1-01-a` and `r1-01-d` are satisfiable without hidden bridge-only operator knowledge.

## Validation / Proof
- Unit tests:
  - clap coverage for new root task subcommands
  - handler tests for create/update/close/export/display-id payloads
- Integration tests:
  - root and proxy command equivalence for the same task mutations
  - missing-task and bad-argument fail-closed cases
- Runtime checks:
  - `vida task --help`
  - `vida task create ... --json`
  - `vida task update ... --json`
  - `vida task close ... --reason ... --json`
  - `vida taskflow task create ... --json`
  - `vida task validate-graph --json`
- Canonical checks:
  - `vida docflow activation-check --root . docs/product/spec/taskflow-task-command-parity-and-proxy-alignment-design.md docs/product/spec/current-spec-map.md`
  - `vida docflow check --root . docs/product/spec/taskflow-task-command-parity-and-proxy-alignment-design.md docs/product/spec/current-spec-map.md`
  - `vida docflow doctor --root .`

## Observability
- Logging points:
  - task close telemetry hook
  - unsupported delegated argument failures
- Metrics / counters:
  - none added in this slice
- Receipts / runtime state written:
  - normal task-store mutations only

## Rollout Strategy
- Development rollout:
  - land root parity first
  - then collapse bridge-specific mutation behavior
  - then update command-string surfaces
- Migration / compatibility notes:
  - keep `vida taskflow task` functioning during rollout
  - shift canonical examples toward `vida task`
- Operator or user restart / restart-notice requirements:
  - rebuild and reinstall `vida` after the command slice lands

## Future Considerations
- Follow-up ideas:
  - batch plan apply/import under `r1-01-b`
  - richer graph-first root views under `r1-01-c`
- Known limitations:
  - top-level `consume/lane/approval/recovery` root-surface work remains outside this slice
- Technical debt left intentionally:
  - broader launcher carve-out continues under `r1-03-*`

## References
- Related specs:
  - [`docs/product/spec/release-1-plan.md`](/home/unnamed/project/vida-stack/docs/product/spec/release-1-plan.md)
  - [`docs/product/spec/status-families-and-query-surface-model.md`](/home/unnamed/project/vida-stack/docs/product/spec/status-families-and-query-surface-model.md)
  - [`docs/product/spec/release-1-operator-surface-contract.md`](/home/unnamed/project/vida-stack/docs/product/spec/release-1-operator-surface-contract.md)
- Related protocols:
  - [`docs/process/documentation-tooling-map.md`](/home/unnamed/project/vida-stack/docs/process/documentation-tooling-map.md)
- Related ADRs:
  - none
- External references:
  - none

-----
artifact_path: product/spec/taskflow-task-command-parity-and-proxy-alignment-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-04-03'
schema_version: '1'
status: canonical
source_path: docs/product/spec/taskflow-task-command-parity-and-proxy-alignment-design.md
created_at: '2026-04-03T12:58:30+03:00'
updated_at: '2026-04-03T12:58:30+03:00'
changelog_ref: taskflow-task-command-parity-and-proxy-alignment-design.changelog.jsonl
