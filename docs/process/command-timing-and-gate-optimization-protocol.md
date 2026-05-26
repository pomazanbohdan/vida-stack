# Command Timing And Gate Optimization Protocol

Purpose: define the project-owned operating protocol for recording timings on significant agent, shell, script, test, CI, build, release, browser, simulator, and runtime operations, then using those timings to optimize development throughput without weakening required proof.

This document is a process protocol. It does not replace product/runtime law, release admission, TaskFlow ownership, DocFlow proof law, or CI branch protection. It defines how operators and agents must collect timing evidence, diagnose slow work, and decide whether a gate should stay blocking, become faster, move to a later admission point, or become diagnostic-only for PR iteration.

## Scope

This protocol applies to:

1. `vida` and TaskFlow/DocFlow commands,
2. shell commands,
3. Git and GitHub CLI calls,
4. package managers and tool installers,
5. scripts under `scripts/**`,
6. local tests and focused regression tests,
7. workspace-wide tests,
8. build, release, install, packaging, and smoke gates,
9. browser, simulator, emulator, and mobile/web validation,
10. CI runs, CI jobs, and individual CI steps,
11. delegated agent lanes, advisory agents, and fallback manual emulation steps.

Read-only commands are not exempt when they materially affect orchestration time. A slow read-only command is still operator-friction evidence.

## Timing Envelope

Every significant operation report must include this minimum envelope:

```text
operation_id: stable short id or command family
command_or_surface: exact command, script, CI step, agent lane, or UI validation surface
cwd_or_context: repo/worktree/project/CI job/lane context
started_at: ISO-8601 timestamp when available
duration_ms: wall-clock duration in milliseconds
exit_status: pass | fail | blocked | cancelled | timed_out | unknown
blocking_scope: none | local_iteration | pr_acceptance | main_admission | release_admission | runtime_continuation
artifact_refs: log paths, CI URLs, receipt paths, screenshots, or command output paths
classification: fast | watch | slow | hard_defect | long_gate_expected
next_decision: keep_blocking | make_fast_proof | diagnostic_only_for_pr | move_to_main_or_release | remove_or_replace_stale_check | create_runtime_defect | none
```

If a tool cannot emit this envelope directly, the orchestrator must record it in the TaskFlow note, PR task, diagnostic note, or linked process artifact.

## Thresholds

1. Normal inspection, planning, routing, status, continuation, task mutation, lightweight diagnostic, and operator-query commands target `<= 2000 ms`.
2. Any normal operator command over `5000 ms` is an architectural or operator-surface defect unless the command is explicitly documented as a long-running proof gate.
3. Any local proof gate over `120000 ms` that blocks ordinary development requires a gate-optimization diagnostic.
4. Long-running commands are allowed only when their admission role is explicit: workspace proof, CI proof, build proof, release proof, install proof, simulator/browser proof, or external-provider probe.
5. A repeated slow command is stronger evidence than a single slow command. Three repeated observations in one active case require TaskFlow actualization unless the task already exists.

## Command Execution Rules

1. Prefer one bounded command per proof step when timing matters.
2. Avoid long command chains that hide which segment was slow or failed.
3. When a sequence is repeated twice, create or update a reusable script or command surface.
4. Scripts that guard PRs, release, install, packaging, runtime smoke, or diagnostics must support `--help` when practical.
5. Scripts should print a concise summary, deterministic exit code, and artifact paths for verbose logs.
6. Scripts should expose JSON or structured status when their output is consumed by agents, runtime diagnostics, CI, or TaskFlow notes.
7. If a command is expected to run longer than two minutes, state that expectation before running it and identify what smaller proof has already passed.
8. Do not repeatedly rerun a long gate to discover hidden failure details; repair output/artifact capture first.

## Gate Decision Model

When a gate is slow or repeatedly blocks iteration, classify it with exactly one decision:

| Decision | Use when | Required follow-up |
| --- | --- | --- |
| `keep_blocking` | The gate directly protects current product/runtime behavior under change. | Keep it in the current proof matrix and improve output if needed. |
| `make_fast_proof` | The same defect can be caught by a focused test, smoke script, syntax check, or targeted command. | Add or update the fast proof and run the long gate only at batch proof time. |
| `diagnostic_only_for_pr` | The gate checks release/install/package behavior outside the PR's bounded product change and focused proof is green. | Keep the signal visible in PR CI but do not block PR closure solely on this gate. |
| `move_to_main_or_release` | The gate is valid but belongs to mainline, nightly, release, or installer admission. | Move or scope the gate and create a TaskFlow note explaining the admission boundary. |
| `remove_or_replace_stale_check` | The gate asserts obsolete text, legacy paths, hidden output, or deprecated behavior. | Replace it with the current contract and prove the new assertion locally. |
| `create_runtime_defect` | The slow operation is a runtime/operator-surface defect. | Create or update the defect under the relevant runtime/operator-efficiency epic. |

## Diagnostic Update Format

Every runtime self-diagnostic, post-push diagnostic, PR CI diagnostic, long-gate diagnostic, or operator-friction audit must append or update a timing section using this format:

```text
Timing diagnostics:
- observed_operations:
  - operation_id:
    command_or_surface:
    duration_ms:
    exit_status:
    blocking_scope:
    artifact_refs:
    classification:
- slowest_operations:
  - operation_id:
    duration_ms:
    suspected_cause:
    proposed_decision:
- gate_decisions:
  - gate:
    decision:
    reason:
    taskflow_item:
    next_proof:
- optimization_backlog:
  - task_id:
    owner_scope:
    expected_gain:
```

If the diagnostic finds no slow operations, record `observed_operations: []` and `gate_decisions: []` so the absence is explicit.

## Bootstrap And TaskFlow Requirements

1. This protocol is part of the active project bootstrap read path through `AGENTS.sidecar.md`, `docs/project-root-map.md`, and `docs/process/README.md`.
2. Any bounded work item that runs commands must record timing evidence for commands that influence task selection, proof acceptance, PR closure, runtime continuation, or release admission.
3. Timing evidence belongs in the active TaskFlow task notes or linked artifact before closure.
4. A timing optimization that changes CI, scripts, command output, command options, diagnostics, or release gating must be its own TaskFlow item unless it is the direct bounded work item already in progress.
5. Timing diagnostics must optimize both wall-clock time and operator/agent iteration count. Reducing a 20-second command to 3 seconds is useful; reducing three separate reads to one structured output is also useful.

## Recommended Local Patterns

1. For local shell timing, use a wrapper that prints duration, exit code, and command id.
2. For PowerShell, prefer `Measure-Command` or a small reusable project script when the same measurement is repeated.
3. For Bash scripts, prefer `SECONDS`, `date +%s%3N`, or a shared helper that prints a final timing line.
4. For CI, prefer step-level timing from GitHub Actions plus script-level summaries inside long steps.
5. For agent lanes, record role, resolved carrier/profile when available, duration, result, rework count, and proof outcome.
6. For browser/simulator/emulator validation, record launch/setup time separately from user-flow validation time.

## Prohibited Patterns

1. Do not hide slow operations inside opaque command chains.
2. Do not make every PR wait for full release/install proof when a focused PR proof and diagnostic release signal are enough for the bounded change.
3. Do not classify a command as acceptable merely because it eventually succeeds.
4. Do not increase timeouts as the primary fix for an operator command that should be fast.
5. Do not keep stale assertions in CI because they are "only smoke"; stale smoke is still false evidence.
6. Do not leave a repeated timing finding only in chat; create or update the TaskFlow item.

## Current Known Timing Evidence

As of this protocol slice, the following observations are known from the active session and must feed follow-up optimization work:

1. `vida orchestrator-init --json` observed around `22000 ms` through the local command wrapper during re-entry.
2. `vida task next-lawful --json` observed around `17000 ms`.
3. `vida task create` and `vida task update` observed around `14000 ms`.
4. PR `validate` CI remained blocked for multiple minutes inside `cargo test --workspace --locked -- --test-threads=1`.

These observations do not prove one root cause. They prove that timing diagnostics must cover both local runtime commands and CI/test gates.

-----
artifact_path: process/command-timing-and-gate-optimization-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: 2026-05-26
schema_version: '1'
status: canonical
source_path: docs/process/command-timing-and-gate-optimization-protocol.md
created_at: 2026-05-26T00:00:00+03:00
updated_at: 2026-05-26T00:00:00+03:00
changelog_ref: command-timing-and-gate-optimization-protocol.changelog.jsonl
