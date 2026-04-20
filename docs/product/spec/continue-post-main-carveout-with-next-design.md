# Continue Post Main Carveout With Next Design

Status: `approved`

## Summary
- Feature / change: move the bounded owner-domain test `project_activator_command_accepts_json_output` out of `crates/vida/src/main.rs` into `crates/vida/src/project_activator_surface.rs`
- Owner layer: `runtime-family`
- Runtime surface: `project activation`
- Status: `approved for bounded implementation`

## Current Context
- `tf-post-r1-main-carveout` is the active deconcentration stream that removes owner logic and owner tests from the launcher root.
- The previous slices already moved dispatch, lane-summary, tracked-flow, and compiled-bundle tests into their owning modules.
- `project_activator_command_accepts_json_output` still lives in `crates/vida/src/main.rs` even though it exercises the `vida project-activator` surface owned by `crates/vida/src/project_activator_surface.rs`.
- Keeping this test in `main.rs` preserves unnecessary root knowledge and slows the goal of making `main.rs` a thin shell/composition surface.

## Goal
- Continue the `main.rs` carve-out with the smallest lawful next slice.
- Place the JSON-output acceptance test beside the `project_activator` owner surface it validates.
- Preserve behavior, harness semantics, and proof scope.
- Out of scope:
  - broader `project_activator` decomposition
  - moving unrelated `orchestrator_init` or `agent_init` tests
  - any `.codex` carrier schema or activation-model changes

## Requirements

### Functional Requirements
- The test `project_activator_command_accepts_json_output` must move from `crates/vida/src/main.rs` to `crates/vida/src/project_activator_surface.rs`.
- The moved test must still prove that `run(cli(&["project-activator", "--json"]))` returns `ExitCode::SUCCESS` from a bootstrap project root established by `vida init`.
- Any required test-only imports or helpers must be brought in minimally without widening the slice.
- `crates/vida/src/main.rs` must retain adjacent non-owner tests unchanged.

### Non-Functional Requirements
- No behavioral regression in the `vida project-activator` command surface.
- No change to operator-facing JSON contracts.
- Keep compile/test surface narrow and ownership explicit.
- Avoid introducing duplicate helpers across modules when existing root reexports are already sufficient.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/launcher-decomposition-and-code-hygiene-design.md`
  - `docs/product/spec/continue-post-main-carveout-with-next-design.md`
- Framework protocols affected:
  - none; this is an implementation-alignment slice under existing launcher-decomposition law
- Runtime families affected:
  - `vida project-activator`
- Config / receipts / runtime surfaces affected:
  - none expected

## Design Decisions

### 1. Keep The Slice Single-Test And Owner-Domain Local
Will implement / choose:
- Move exactly one test, because the runtime currently requires explicit bounded continuation and this is the next smallest lawful unit in the same carve-out stream.
- Why:
  - keeps the continuation unit unambiguous
  - reduces merge and proof risk
  - preserves momentum in the active `main.rs` deconcentration track
- Trade-offs:
  - smaller throughput per slice
  - more commits before the whole `project_activator` cluster is cleared
- Alternatives considered:
  - move a larger `project_activator` cluster now
  - switch to `state_store` work instead of continuing the same owner stream

### 2. Treat `project_activator_surface.rs` As The Test Owner
Will implement / choose:
- Place the test in `crates/vida/src/project_activator_surface.rs` under that module's test surface.
- Why:
  - the command contract is owned by the project activator surface
  - ownership-aligned tests make future refactors cheaper and safer
- Trade-offs:
  - the owner module may need minimal extra test imports
- Alternatives considered:
  - keep the test in `main.rs` until a larger activator sweep
  - create a separate integration test file for a single bounded command assertion

## Technical Design

### Core Components
- `crates/vida/src/main.rs`
  - source location of the legacy root-owned test
- `crates/vida/src/project_activator_surface.rs`
  - destination owner module for the command-surface test
- `crates/vida/src/test_cli_support.rs`
  - shared test-only CLI and cwd-guard helpers used by both the root test module and the moved owner-domain test

### Data / State Model
- No state model changes.
- No new receipts, snapshots, config fields, or migration concerns are introduced by this slice.

### Integration Points
- `run(cli(&["project-activator", "--json"]))`
- `run(cli(&["init"]))` as the minimal bootstrap precondition for project-root resolution
- `TempStateHarness`
- `guard_current_dir`
- a minimal test-only `VIDA_ROOT` guard so inherited host environment does not mask project-root resolution defects
- Tokio runtime initialization used by the existing command test harness pattern

### Bounded File Set
- `crates/vida/src/main.rs`
- `crates/vida/src/project_activator_surface.rs`
- `crates/vida/src/test_cli_support.rs`
- `docs/product/spec/continue-post-main-carveout-with-next-design.md`

## Fail-Closed Constraints
- Do not widen to unrelated `project_activator` tests in the same pass.
- Do not modify JSON payload semantics or activation rules.
- Do not rewrite neighboring launcher tests while touching `main.rs`.
- If the owner module requires non-minimal helper duplication to host the test, stop and reshape the slice instead of widening silently.

## Implementation Plan

### Phase 1
- Add the bounded design packet and validate it through DocFlow.
- Proof target:
  - `vida docflow check --root . docs/product/spec/continue-post-main-carveout-with-next-design.md`

### Phase 2
- Move the test into `project_activator_surface.rs`.
- Remove the original copy from `main.rs`.
- Proof target:
  - `cargo test -p vida project_activator_surface::tests::project_activator_command_accepts_json_output -- --exact --nocapture`

### Phase 3
- Review the diff for owner alignment.
- Commit, push, build release, and update the installed `vida` binary.
- Final proof target:
  - release build
  - installed binary hash parity
  - `vida --help`

## Validation / Proof
- Unit tests:
  - `cargo test -p vida project_activator_surface::tests::project_activator_command_accepts_json_output -- --exact --nocapture`
- Integration tests:
  - not required for this bounded slice
- Runtime checks:
  - release build of `vida`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/continue-post-main-carveout-with-next-design.md`

## Observability
- No new observability fields.
- Standard git commit plus release-build/install evidence are sufficient for this slice.

## Rollout Strategy
- Land as a normal bounded refactor slice in the active carve-out stream.
- No migration work.
- No operator restart notice beyond the normal binary update cycle already required by the project workflow.

## Future Considerations
- Follow-up slices should continue clearing `project_activator`-owned tests and helpers from `main.rs`.
- The remaining `main.rs` test surface should eventually contain only root-shell integration coverage.

## References
- `docs/product/spec/launcher-decomposition-and-code-hygiene-design.md`
- `tf-post-r1-main-carveout`

-----
artifact_path: product/spec/continue-post-main-carveout-with-next-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-12
schema_version: 1
status: canonical
source_path: docs/product/spec/continue-post-main-carveout-with-next-design.md
created_at: 2026-04-12T17:29:10.756612196Z
updated_at: 2026-04-12T17:30:23.47868385Z
changelog_ref: continue-post-main-carveout-with-next-design.changelog.jsonl
