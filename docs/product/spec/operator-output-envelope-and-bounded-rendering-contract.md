# Operator Output Envelope And Bounded Rendering Contract

Status: active product contract

Use this contract as the bounded output envelope and rendering contract for operator-facing TaskFlow surfaces.

## Summary
- Contract: introduce a centralized output policy/envelope seam and make `vida task list --json` bounded by default while preserving explicit full export through `--all --json`.
- Owner layer: `runtime-family`
- Runtime surface: `taskflow`
- Status: active product contract

## Current Context
- Existing system overview
  - Many VIDA surfaces construct their own JSON payloads and then call shared rendering helpers such as `print_json_pretty`, `print_surface_json`, `print_surface_header`, and `print_surface_line`.
  - Task surfaces already use `build_pass_operator_surface_payload` in `crates/vida/src/task_cli_render.rs` for shared Release-1 envelope fields.
  - `vida task list` accepts `--summary`, `--all`, and `--json`, but the current handler passes `command.summary` directly to rendering. As a result, `vida task list --json` defaults to `view=full` unless `--summary` is also specified.
- Key components and relationships
  - `crates/vida/src/cli.rs` owns CLI flags for `TaskListArgs`.
  - `crates/vida/src/task_surface.rs` wires `TaskCommand::List` to `task_list_authoritative_first` and `print_task_list`.
  - `crates/vida/src/task_cli_render.rs` builds task list payloads and already keeps Release-1 operator contract parity.
  - `crates/vida/src/shell_runtime_helpers.rs` owns low-level `print_json_pretty`.
- Current pain point or gap
  - Operator inspection commands can accidentally dump very large JSON payloads when the operator wanted bounded status/inspection output.
  - Full dump behavior is useful for export/debug, but it should require explicit `--all`/full intent.
  - There is no first-class output policy marker that tells operators whether a payload is summary, full, or explicit full.

## Goal
- What this change should achieve
  - Make `vida task list --json` bounded and summary-shaped by default.
  - Preserve `vida task list --all --json` as the explicit full output path.
  - Add an output policy marker to task list JSON so automation can distinguish default summary from explicit full output.
  - Establish a small centralized output-policy seam that future surfaces can reuse before a broader renderer migration.
- What success looks like
  - Default task-list JSON is `view=summary` and does not serialize full `TaskRecord` rows.
  - Explicit `--all --json` remains `view=full` and returns complete task rows.
  - Both paths keep Release-1 shared envelope parity.
  - Focused unit tests prove bounded default and explicit full behavior.
- What is explicitly out of scope
  - Rewriting every VIDA surface to a new renderer in this slice.
  - Changing `vida task export-jsonl` semantics.
  - Truncating or artifact-spilling every large payload globally before the first bounded adopter proves the contract.

## Requirements

### Functional Requirements
- `vida task list --json` must emit a summary payload by default.
- `vida task list --all --json` must emit the current full payload.
- `--summary` must continue to force summary output.
- JSON payloads must include the existing Release-1 envelope fields.
- The task-list payload must expose an `output_policy` object with at least mode, explicit_full, and max_inline_items.

### Non-Functional Requirements
- Performance
  - Default list rendering should avoid serializing full task records unless `--all` is explicit.
- Scalability
  - The policy seam should be reusable by other operator surfaces later.
- Observability
  - Payload fields must make bounded-vs-full behavior visible to operators and tests.
- Security
  - Do not hide full output behind implicit heuristics; require explicit full intent.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/operator-output-envelope-and-bounded-rendering-contract.md`
  - future map/provenance registration follow-up if this design becomes canonical.
- Framework protocols affected:
  - none
- Runtime families affected:
  - `taskflow`
- Config / receipts / runtime surfaces affected:
  - `vida task list [--summary|--all] [--json]`

## Design Decisions

### 1. First adopter instead of global rewrite
Will implement / choose:
- Add a lightweight output-policy struct/helper in `task_cli_render.rs` and apply it to task list first.
- Why: task list is the observed long-output pain point and already has summary/full branches.
- Trade-offs: not every surface is fixed immediately, but the contract is proven in a small safe slice.
- Alternatives considered: rewriting all `print_json_pretty` call sites now; rejected as too broad for this recovery slice.

### 2. Explicit full output remains available
Will implement / choose:
- Default `vida task list --json` to summary unless `--all` is set.
- Why: operator inspection should be bounded by default; full dump remains useful when explicitly requested.
- Trade-offs: scripts that expected full output from `task list --json` must add `--all`.
- Alternatives considered: keep default full and add a warning; rejected because it does not restore bounded operator behavior.

## Technical Design

### Core Components
- `TaskListOutputPolicy` in `task_cli_render.rs`:
  - `mode`: `summary` or `full`
  - `explicit_full`: bool
  - `max_inline_items`: bounded default marker for summary mode
- `print_task_list` continues to own payload construction for this slice.
- `TaskCommand::List` computes `summary_only = command.summary || !command.all`.

### Data / State Model
- No TaskFlow state migration.
- No receipt changes.
- New JSON field: `output_policy`.

### Integration Points
- CLI flags stay compatible: `--all` means full; `--summary` means summary.
- Release-1 operator envelope remains unchanged.

### Bounded File Set
- `docs/product/spec/operator-output-envelope-and-bounded-rendering-contract.md`
- `crates/vida/src/task_surface.rs`
- `crates/vida/src/task_cli_render.rs`
- Optional focused tests in the same module.

## Fail-Closed Constraints
- Do not remove `--all --json` full behavior.
- Do not drop required shared envelope fields.
- Do not silently serialize full task records on default operator inspection.
- Do not use artifact spilling as a hidden replacement for explicit full/export commands in this slice.

## Implementation Plan

### Phase 1
- Add task-list output policy helper and payload field.
- First proof target: unit tests for summary/full envelope and field shape.

### Phase 2
- Change `TaskCommand::List` default JSON behavior to summary unless `--all` is explicit.
- Second proof target: `cargo test -p vida task_list`.

### Phase 3
- Run formatting, build/check, installed binary refresh, and operator smoke commands.
- Final proof target: installed `vida task list --json` shows `view=summary`; installed `vida task list --all --json` shows `view=full`.

## Validation / Proof
- Unit tests:
  - task list summary payload keeps operator parity and only summary fields.
  - task list full payload keeps operator parity and includes full record fields.
- Integration tests:
  - optional boot smoke for CLI flag wiring if needed.
- Runtime checks:
  - `vida task list --json`
  - `vida task list --all --json`
- Canonical checks:
  - `cargo fmt --all`
  - `cargo check -p vida`
  - targeted `cargo test -p vida task_list`

## Observability
- `output_policy` tells operators and automation whether output is bounded summary or explicit full.
- Existing `task_count`, `state_access`, `shared_fields`, `operator_contracts`, and `artifact_refs` remain visible.

## Rollout Strategy
- Development rollout through repo-local tests.
- Release rollout through `vida release install --target current` after proof passes.
- Compatibility note: scripts requiring full records should use `vida task list --all --json`.

## Future Considerations
- Move `TaskListOutputPolicy` into a shared `output_policy` module after two or more surfaces adopt it.
- Add artifact-spill support for very large explicit full payloads.
- Add `--limit` to task list for bounded custom windows.

## References
- current runtime contract profile
- `docs/process/documentation-tooling-map.md`
- `docs/framework/templates/feature-design-document.template.md`

-----
artifact_path: product/spec/operator-output-envelope-and-bounded-rendering-contract
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-05-20
schema_version: 1
status: canonical
source_path: docs/product/spec/operator-output-envelope-and-bounded-rendering-contract.md
created_at: 2026-05-20T13:04:57.8354806Z
updated_at: 2026-05-20T13:04:57.8354806Z
changelog_ref: operator-output-envelope-and-bounded-rendering-contract.changelog.jsonl
