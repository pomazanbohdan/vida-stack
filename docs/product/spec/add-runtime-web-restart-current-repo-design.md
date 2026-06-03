# Add Runtime Web Restart Current Repo Command Design

Status: `approved`

## Summary
- Feature / change: add a canonical `vida runtime web restart --scope current-repo --include-edge-proxy` operator command.
- Owner layer: `runtime-family`
- Runtime surface: `runtime web`
- Status: `approved`

## Current Context
- VIDA downstream projects currently restart local Flutter/Odoo/proxy/browser-proof stacks with project-local scripts.
- Browser proof failures can be caused by stale listeners from another worktree, stale edge proxy processes, or mismatched current-repo ports.
- Existing runtime web diagnostics are split across scripts and ad hoc status checks; there is no single TaskFlow-friendly restart receipt.

## Goal
- Provide one command that restarts the web proof process group for the current repository.
- Keep restart scope explicit and fail closed before terminating unrelated processes.
- Emit compact JSON evidence suitable for task proof and closeout.
- Out of scope: implementing project-specific Flutter/Odoo startup logic for every downstream project in this slice.

## Requirements

### Functional Requirements
- Add `vida runtime web restart --scope current-repo --include-edge-proxy --json`.
- Support a dry run path or preview fields in the JSON output before process mutation is performed.
- Detect current repository root, expected local web ports, edge proxy inclusion, and stale listener ownership.
- Restart only processes that can be attributed to the current repository or to the configured current-repo edge proxy.
- Return per-component actions: `stopped`, `started`, `skipped`, or `blocked`.

### Non-Functional Requirements
- Fail closed if a listener cannot be attributed to the current repository.
- Keep output compact enough for agent/operator consumption.
- Preserve Windows PowerShell compatibility.
- Avoid introducing product-specific paths into framework owner law.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/add-runtime-web-restart-current-repo-design.md`
- Runtime families affected:
  - runtime web diagnostics and restart operator surfaces
- Config / receipts / runtime surfaces affected:
  - `vida runtime web restart`
  - future pairing with `vida runtime web status`

## Design Decisions

### 1. Current-Repo Scoped Restart
Will implement / choose:
- `--scope current-repo` as the only supported restart scope in this slice.
- It prevents accidental cross-worktree process mutation.
- Broader scopes can be added after status diagnostics prove process ownership reliably.

### 2. Edge Proxy Is Explicit
Will implement / choose:
- `--include-edge-proxy` is required to restart edge proxy listeners.
- Without the flag, edge proxy state is reported but not mutated.
- This preserves a safe default for local app-only restarts.

### 3. JSON Receipt First
Will implement / choose:
- The command returns a structured receipt with `components`, `actions`, `blocker_codes`, and `next_actions`.
- Human output can be layered later; TaskFlow proof needs the machine-readable shape first.

## Technical Design

### Core Components
- CLI routing in the VIDA runtime command surface.
- Runtime web restart planner:
  - discovers expected ports and process ownership,
  - builds restart actions,
  - blocks on ambiguous ownership.
- Restart executor:
  - stops current-repo listeners,
  - optionally restarts edge proxy,
  - reports receipt fields.

### Data / State Model
- Receipt fields:
  - `scope`
  - `include_edge_proxy`
  - `components`
  - `actions`
  - `blocked_components`
  - `blocker_codes`
  - `next_actions`
- No persistent migration is required in this slice.

### Integration Points
- `vida runtime web status` should later share discovery logic with restart.
- Browser proof commands can consume restart receipt artifacts in follow-up work.

### Bounded File Set
- `crates/vida/src`
- `docs/process`
- `docs/product/spec/add-runtime-web-restart-current-repo-design.md`

## Fail-Closed Constraints
- Do not stop a process when repository ownership is unknown.
- Do not restart edge proxy unless `--include-edge-proxy` is present.
- Do not treat a successful stop as a successful restart unless start evidence exists.
- Do not write project-specific downstream assumptions into generic runtime law.

## Implementation Plan

### Phase 1
- Add CLI shape and JSON receipt model.
- Add dry planning logic for current-repo scope.

### Phase 2
- Add process restart executor for owned listeners.
- Wire `--include-edge-proxy` into explicit edge proxy action planning.

### Phase 3
- Add tests for command parsing, fail-closed ownership, JSON receipt shape, and edge proxy opt-in.

## Validation / Proof
- Unit tests:
  - runtime web restart parser and receipt shape
  - fail-closed ambiguous ownership
  - edge proxy requires explicit inclusion
- Integration tests:
  - focused CLI smoke using dry/plan mode when available
- Runtime checks:
  - `vida runtime web restart --scope current-repo --include-edge-proxy --json`
  - `cargo test -p vida runtime_web`
  - `cargo build -p vida`
- Canonical checks:
  - `docflow check-file --path docs/product/spec/add-runtime-web-restart-current-repo-design.md`

## Observability
- Emit blocker codes for ambiguous ownership, missing restart adapter, and start failure.
- Include component-level action results in the JSON receipt.

## Rollout Strategy
- Land command behind explicit `runtime web restart` invocation.
- Keep edge proxy restart opt-in.
- Reuse the same discovery model in the future `runtime web status` task.

## Future Considerations
- Add `vida runtime web status --json`.
- Add proof attachment integration for browser screenshots and DOM evidence.
- Add downstream project adapters through config-owned process groups.

## References
- `docs/product/spec/fix-status-surface-external-cli-readiness-design.md`
- `docs/process/project-error-search-runtime-diagnostics-protocol.md`
- `docs/process/command-timing-and-gate-optimization-protocol.md`

-----
artifact_path: product/spec/add-runtime-web-restart-current-repo-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-06-03
schema_version: 1
status: canonical
source_path: docs/product/spec/add-runtime-web-restart-current-repo-design.md
created_at: 2026-06-03T22:22:00+03:00
updated_at: 2026-06-03T22:22:00+03:00
changelog_ref: add-runtime-web-restart-current-repo-design.changelog.jsonl
