# VIDA Runtime Hardening Release Readiness Guide

Status: active project process doc

Purpose: provide the operator guide for closing the `VIDA-RUNTIME-HARDENING` epic without weakening TaskFlow, DocFlow, release-install, or proof evidence authority.

## Scope

This guide applies to the final runtime hardening closure wave. It covers:

1. live TaskFlow readiness checks for the epic and remaining descendants,
2. proof evidence required before closing release-readiness tasks,
3. local quality gates that must run before release closure,
4. the system runtime release-install proof for the current revision,
5. the operator sequence for committing, pushing, and continuing.

It does not replace the authoritative TaskFlow database, root `AGENTS.md`, or framework runtime law. If this document and a live `vida` surface disagree, inspect the live surface first and update this guide only after the runtime evidence is understood.

## Closure Authority

Use these surfaces as the closure authority:

```powershell
$env:VIDA_STATE_DIR='C:\project\vida-stack\.vida\data\state'
vida orchestrator-init --json
vida task progress VIDA-RUNTIME-HARDENING --json
vida task ready --scope VIDA-RUNTIME-HARDENING --json
vida task blocked --json
vida task validate-graph --json
```

The epic is closeable only when `vida task progress VIDA-RUNTIME-HARDENING --json` reports no open or in-progress descendants and the epic close command accepts the accumulated proof evidence. Do not close from a stale percentage, a cached projection, or a human summary.

## Runtime Hardening Invariants

The final wave must preserve these invariants:

1. TaskFlow remains the execution authority; repository files are projections or implementation artifacts.
2. Every write-producing mutation has a DB-backed `step` under the active task unless the runtime is explicitly bypassed for a documented VIDA runtime defect.
3. Root-local writes require either delegated execution evidence or an active exception takeover with the exact owned path scope.
4. Structured proof evidence must be attached for every configured proof target before closing a non-container task.
5. `vida task validate-graph --json` must pass after TaskFlow mutations and before treating a task closure as stable.
6. Release install must build the current revision's release `vida.exe`, install that exact binary into the system `current\bin`, and verify installed `vida status`.
7. When a clean worktree lacks the authoritative state spine, run release-install and installed-status proof with `VIDA_STATE_DIR=C:\project\vida-stack\.vida\data\state`.
8. Commit and push after each closed task slice before selecting unrelated remaining work.

## Quality Gate

Before `VH-64` or the epic can close, the current proof bundle should include:

```powershell
cargo test -p taskflow-core
cargo test -p vida task_smoke
cargo test -p vida --test large_backlog_graph_performance -- --nocapture
vida docflow check-file docs/process/vida-runtime-hardening-release-readiness-guide.md --json
vida task validate-graph --json
pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode release-install -Json
```

If a full `task_smoke` run is too slow for the current slice, attach the focused proof already run for the changed surface and keep `VH-64` open until the broad gate is run or an explicit blocker is recorded.

## Release Install Proof

The canonical local install proof is:

```powershell
$env:VIDA_STATE_DIR='C:\project\vida-stack\.vida\data\state'
pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode release-install -Json
```

The gate is valid only when all three stages pass:

1. `cargo build --locked -p vida --release`,
2. `vida release install --skip-build --source-binary <release vida.exe> --json`,
3. installed `current\bin\vida.exe status --json`.

After the gate, compare hashes when the task requires explicit binary proof:

```powershell
Get-FileHash .vida\cargo-target\release\vida.exe -Algorithm SHA256
Get-FileHash "$env:LOCALAPPDATA\vida-stack\current\bin\vida.exe" -Algorithm SHA256
vida --version
```

The release hash and installed hash must match. The installed binary should resolve from `%LOCALAPPDATA%\vida-stack\current\bin` before any older PATH entry.

## Operator Close Sequence

Use this sequence for remaining hardening work:

1. Run `vida orchestrator-init --json` and record `active_bounded_unit`, `why_this_unit`, and `sequential_vs_parallel_posture`.
2. If there is no active unit, use `vida task ready --scope VIDA-RUNTIME-HARDENING --json` plus blockers to select the next highest-priority ready descendant.
3. Create a bounded TODO under that task before writes.
4. Run the focused implementation or documentation proof for the owned paths.
5. Attach structured proof evidence to the TODO and parent task.
6. Close the TODO, then close the parent only when `vida task proof status <task-id> --json` has no missing target.
7. Run `vida task validate-graph --json`.
8. Commit and push the scoped repository changes.
9. Run canonical release-install when the closed slice changes runtime code, tests, proof gates, or release-facing docs.
10. Re-read epic progress and continue until no open descendants remain.

## Current Final-Wave Evidence

As of the `2026-06-19` VH-63 closure slice, the large-backlog proof gate includes:

1. reusable deterministic JSONL fixture support in `vida-test-support`,
2. `large_backlog_graph_performance` integration coverage for import, progress, tree, ready, blocked, graph-summary, scheduler dispatch, and status `taskflow_counts`,
3. diagnostic timings recorded as evidence only, not as flaky pass/fail thresholds,
4. release install proof from the current pushed revision with matching release and installed binary hashes.

Do not treat this evidence as completing later open tasks. It is closure evidence for the already closed large-backlog slice and a prerequisite for final release-readiness consolidation.

-----
artifact_path: process/vida-runtime-hardening-release-readiness-guide
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-06-19'
schema_version: '1'
status: canonical
source_path: docs/process/vida-runtime-hardening-release-readiness-guide.md
created_at: 2026-06-19T17:35:00+03:00
updated_at: 2026-06-19T17:35:00+03:00
changelog_ref: vida-runtime-hardening-release-readiness-guide.changelog.jsonl
