# Typed Transition State Store Extraction Contract

Status: canonical

## Summary
- Feature / change: extract TaskFlow and run-graph transition law out of `state_store` into typed shared/core, authority, and adapter boundaries.
- Owner layer: TaskFlow runtime family.
- Runtime surface: `vida task`, `vida taskflow`, run-graph, lane, dispatch, recovery, and continuation surfaces.
- Status: canonical design gate for `typed-transition-state-store-extraction-epic`.

## Current Context
- `crates/vida/src/state_store.rs` still owns persisted state access and mixes typed rows, transition admission, reconciliation, rollups, snapshot bridges, and tests.
- Adjacent runtime modules call state-store methods directly for lifecycle decisions and receipt reconciliation.
- The shared extraction epic must move canonical transition law below shell/operator rendering without changing persisted state compatibility or public command output.
- A paused runtime-defect task currently owns unrelated dirty hunks in `runtime_dispatch_state.rs`, `state_store.rs`, `state_store_run_graph_summary.rs`, and `taskflow_consume_resume.rs`; this design does not absorb those hunks.

## Goal
- Freeze the shared seam, file map, migration waves, proof targets, and non-goals before code extraction begins.
- Make typed transition law the canonical product contract for later implementation tasks.
- Keep the `state_store` module as a persistence adapter after extraction, not as the owner of transition decisions.

## Non-Goals
- Do not change public command names, JSON field names, default TOON/plain output, or help text in this design slice.
- Do not rewrite the state backend or migrate existing `.vida/data/state` rows.
- Do not fix receipt-suite runtime crashes in this epic unless a shared extraction task directly owns the failing invariant.
- Do not keep duplicate old and new transition paths after each bounded implementation wave.

## Ownership Model

| Boundary | Owns | Must not own |
| --- | --- | --- |
| `vida-core` shared transition model | typed task lifecycle rows, dependency graph view, transition table inputs/outputs, validation errors | Surreal queries, CLI rendering, command routing |
| TaskFlow authority module | admission decisions, lifecycle transitions, scheduler claims, continuation gates, task attempts, run-graph reconciliation verdicts | raw storage, shell output, host-tool execution |
| State-store adapter | persistence, row load/save, compatibility conversion, snapshot bridge serialization | policy decisions, duplicated lifecycle rules, public rendering |
| CLI/operator surfaces | argument parsing, rendering, envelope selection, command-specific output mode | lifecycle truth, run-graph truth, scheduler truth |
| Tests/fixtures | golden behavior, persisted-state compatibility, property/matrix coverage, CLI parity | compatibility wrappers that preserve obsolete duplicate logic |

## Canonical Data Shapes

### Task Transition Core
- `TaskLifecycleStatus`: typed projection over the current canonical TaskFlow statuses.
- `TaskLifecycleTransition`: `{ task_id, from, to, reason, actor, recorded_at }`.
- `TaskGraphView`: typed dependency graph with parent-child, blocking, and repeated-edge diagnostics.
- `TaskTransitionDecision`: `{ admitted, next_status, blocker_codes, notes, receipt_kind }`.
- `TaskTransitionError`: typed fail-closed reason used by authority and adapters.

### Run-Graph Transition Core
- `RunGraphLifecycleStatus`: typed projection over current persisted run-graph lifecycle stages.
- `RunGraphReceiptSignal`: canonical dispatch receipt signal after normalization.
- `RunGraphReconcileInput`: latest status, latest receipt, current session, continuation binding, lane evidence.
- `RunGraphReconcileDecision`: authoritative next run state, stale/drift blockers, continuation posture.
- `RunGraphEvidenceAdapter`: converts persisted rows and artifact refs into typed evidence without deciding policy.

### Authority Decisions
- Task lifecycle admission must call the shared task transition table.
- Run-graph receipt reconciliation must call the shared run-graph reconcile authority.
- Continuation, attempts, and scheduler claims must use authority-level decisions before state-store writes.
- Snapshot bridge rollups must preserve persisted compatibility while using typed row conversion.

## Module Map

| Epic task | Target canonical boundary | Primary files | Proof family |
| --- | --- | --- | --- |
| `00-design-spec` | this design contract | `docs/product/spec/typed-transition-state-store-extraction-contract.md` | `vida docflow check` |
| `01-golden-behavior` | current behavior fixtures | state-store and public CLI tests | focused golden tests |
| `02-contract-boundary` | shared core API | `crates/vida-core/**`, caller facade exports | core unit tests |
| `03-core-task-row-graph-model` | typed task row and graph view | `vida-core` task transition modules | graph validation tests |
| `04-core-task-lifecycle-table` | lifecycle transition table | `vida-core` task lifecycle modules | transition matrix tests |
| `05-authority-task-lifecycle` | TaskFlow lifecycle authority | authority module under runtime family | admission tests |
| `06-state-store-task-adapter` | persistence adapter | `state_store.rs` plus adapter child module | state-store compatibility tests |
| `07-core-run-graph-model` | typed run-graph model | `vida-core` run-graph transition modules | run-graph model tests |
| `08-authority-run-graph-reconcile` | receipt/status reconciliation authority | runtime authority module | reconciliation tests |
| `09-run-graph-evidence-adapters` | artifact and receipt evidence conversion | adapter modules near state-store/run-graph surfaces | persisted evidence tests |
| `10-continuation-gate-authority` | continuation decision authority | consume/resume and authority modules | continue/resume tests |
| `11-task-attempts-authority` | attempt rollup authority | TaskFlow authority module | attempt rollup tests |
| `12-scheduler-claim-authority` | scheduler reservation/claim admission | scheduler and authority modules | scheduler tests |
| `13-snapshot-bridge-rollup` | typed snapshot bridge and rollup conversion | state-store adapter modules | snapshot fixture tests |
| `14-public-surface-parity` | unchanged operator output contract | CLI smoke/integration tests | default and JSON parity tests |
| `15-persisted-state-fixtures` | old-state compatibility | fixture rows under test-only state roots | fixture replay tests |
| `16-property-matrix-tests` | transition property coverage | core and integration tests | property/matrix tests |
| `17-remove-legacy-duplicates` | removal of duplicate policy branches | old helpers and tests | duplicate-search proof |
| `18-validation-closure` | final closeout | graph/doc/runtime proof | validate-graph, focused tests, release install |

## Migration Waves

### Wave 0: Design And Golden Capture
- Freeze this design.
- Capture current behavior before moving logic.
- No Rust production mutation beyond tests/fixtures required for golden proof.
- Stop when public behavior coverage identifies the old state-store decisions that later waves must preserve or intentionally replace.

### Wave 1: Shared Core Contract
- Add typed task and run-graph model types in shared/core ownership.
- Add transition table and validation functions with matrix tests.
- Keep state-store writes unchanged until authority modules call the core contract.
- Stop when core tests cover success, invalid transition, missing evidence, stale evidence, and duplicate edge cases.

### Wave 2: Authority Adoption
- Move lifecycle, run-graph reconciliation, continuation, attempt, and scheduler decisions into runtime authority modules.
- State-store methods call authority decisions before persisting.
- Public command surfaces continue to read/write through existing store entrypoints.
- Stop when focused runtime tests show old surface behavior uses the shared authority path.

### Wave 3: Adapter And Compatibility Closure
- Split persistence conversion and snapshot bridge rollups into adapters.
- Replay persisted-state fixtures from current `.vida`-compatible rows.
- Remove obsolete duplicate policy helpers and tests that encode the old state-store-owned behavior.
- Stop when no duplicated transition admission remains in state-store, surfaces, or tests.

### Wave 4: Public Parity And Release Closure
- Prove default output, explicit JSON, help/option text, persisted state replay, and run-graph/TaskFlow parity.
- Run graph validation and release/install proof after the epic closes.
- Any unrelated receipt-suite or hook/runtime crash stays under the runtime defect epic.

## Public Surface Constraints
- `vida task` and `vida taskflow` outputs must remain compatible unless a later public-surface task explicitly changes them.
- Default operator output stays compact and human-facing.
- JSON output remains machine-readable and field-compatible.
- Help text must not point users to internal modules or shared-core names as operator commands.
- Failure paths must expose typed blocker codes through existing operator contract fields.

## Proof Targets
- Design slice: `vida docflow check docs/product/spec/typed-transition-state-store-extraction-contract.md --json`.
- Graph slice: `vida task validate-graph --json`.
- Core slices: focused `cargo test -p vida-core transition`.
- State-store slices: focused `cargo test -p vida state_store`.
- Run-graph slices: focused `cargo test -p vida run_graph`.
- Public parity slice: focused CLI smoke tests covering default and `--json` output.
- Epic closure: focused slice proofs, graph validation, release install, and installed `vida status`.

## Fail-Closed Rules
- If a task discovers a runtime defect that blocks proof but is outside shared extraction, create or update a runtime-defect task under the runtime epic and bypass only that proof with explicit evidence.
- If persisted compatibility requires a schema migration, stop and create a separate migration design before code mutation.
- If a shared helper would only wrap old duplicate logic, reject it and move the invariant into core or authority.
- If a public surface must change, stop and route the change through `typed-transition-state-store-14-public-surface-parity`.

-----
artifact_path: product/spec/typed-transition-state-store-extraction-contract
artifact_type: product_spec
artifact_version: '1'
artifact_revision: 2026-06-21
schema_version: '1'
status: canonical
source_path: docs/product/spec/typed-transition-state-store-extraction-contract.md
created_at: 2026-06-21T23:25:00+03:00
updated_at: 2026-06-21T23:25:00+03:00
changelog_ref: typed-transition-state-store-extraction-contract.changelog.jsonl
