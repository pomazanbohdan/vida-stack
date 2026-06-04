---
name: vida-runtime-development
description: "VIDA runtime development workflow for vida-stack. Use when working on TaskFlow, DocFlow, runtime diagnostics, operator command surfaces, run-graph/lane/dispatch state, issue-driven runtime defects, command-efficiency follow-ups, or session closeout/proof planning in C:/project/vida-stack."
---

# VIDA Runtime Development

Use this skill to keep VIDA runtime work evidence-first, TaskFlow-backed, and low-friction.

## Start Sequence

1. Read `AGENTS.md` and `AGENTS.sidecar.md` after any compact or re-entry.
2. Run `vida orchestrator-init --json` before selecting work.
3. State:
   - `active_bounded_unit`
   - `why_this_unit`
   - `sequential_vs_parallel_posture`
4. If a write is needed, create a DB-backed `todo` first.
5. For runtime defects, load `docs/process/project-error-search-runtime-diagnostics-protocol.md`.
6. For command/output/timing friction, load `docs/process/command-timing-and-gate-optimization-protocol.md`.
7. For session-level runbook details, load `references/session-runtime-runbook.md`.

## Runtime Evidence Order

Prefer the smallest authoritative surface that answers the question:

1. `vida task show <task-id> --json` for known task metadata.
2. `vida task tree <task-id> --json` for parent/child closure shape.
3. `vida task validate-graph --json` after TaskFlow mutations.
4. `vida status --json` and `vida doctor --json` for projection parity.
5. `vida orchestrator-init --json` for active bounded-unit and continuation binding.
6. Run-graph, lane, recovery, and dispatch surfaces only when the defect needs that evidence.

Do not treat derived summaries, stale cache, lane preview, or advisory text as stronger than authoritative TaskFlow state, receipt-backed evidence, and current runtime status.

## Write Discipline

Before every project mutation:

1. Create a `todo` under the active task or current epic.
2. State `STEP`, `STOP`, and `IF_BLOCKED`.
3. Keep writes sequential when they mutate the same TaskFlow graph, docs map, skill folder, or runtime state.
4. Close the TODO only after validation passes.
5. If `task close` rejects a valid close because of literal words in the reason, record that as operator-surface evidence and retry with neutral wording only for the same bounded close step.

## Command Efficiency

After each coherent runtime work pool, check whether the session required avoidable operations:

- full backlog scans instead of filtered task search,
- raw reruns because compact output hid key fields,
- client-side JSON unwrapping because stable field selectors were missing,
- repeated `status`, `doctor`, `task tree`, and GitHub reads for one proof bundle,
- noisy output where a child summary would have been enough.

If yes, create or update an operator-efficiency TaskFlow item under the current runtime/quality epic.

## Proof And Closure

Minimal proof for this skill's work:

1. `vida task validate-graph --json`
2. Focused DocFlow check when docs changed.
3. Relevant runtime parity command, such as `vida status --json` plus `vida doctor --json`.
4. `git status --short` before reporting closure.
5. Any GitHub issue or PR state from live `gh` or connector evidence when GitHub surfaces changed.

Do not close GitHub issues, epics, or TaskFlow parents until the relevant children and proof targets are current.
