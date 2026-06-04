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

## Agent Research Ring

For complex runtime defects, use an adaptive agent research ring instead of a fixed number of research loops.

1. Agent research is the default for multi-surface runtime defects, external-project live defect reports, PR/security findings, and any blocker that repeats after a local patch. Start agents before local implementation unless the next local command is the only critical-path proof.
2. Start with 2-3 parallel `middle`/medium read-only agents when the defect spans multiple runtime surfaces. Split the prompts by evidence owner:
   - owner/code path and task mapping,
   - state/config/runtime boundary,
   - failing proof and closure tests.
3. If thread limits, provider limits, or host-tool capability gaps prevent the full ring, launch the maximum available agents immediately, state the reduced lane count, and continue with non-overlapping local validation only. Do not silently collapse the ring into root-only investigation.
4. Follow with one inverse coach/reviewer agent whose job is to challenge the findings, look for false-green fixes, missing dependencies, and scope creep.
5. Use an architect/synthesis agent only when the first reports conflict or the defect crosses architectural ownership boundaries.
6. Stop research when the ring proves:
   - root cause or a bounded root-cause tree,
   - owning files/functions,
   - fix-now versus follow-up boundary,
   - failing proof and closure proof suite,
   - inverse coach found no blocking gap.
7. Prefer 1 explorer + 1 coach for simple defects, 3 explorers + 1 coach for runtime surface mismatches, and 3 explorers + 1 inverse coach + optional architect for architectural defects.
8. Use large fixed loops, such as 10 rounds, only when the defect decomposes into multiple independent sub-defects or the agent reports materially disagree.
9. Keep prompts narrow. Ask each agent for concrete file/function references, defect ownership, proof gaps, and explicit non-goals. Do not ask every agent to rediscover the entire codebase.
10. Root/orchestrator remains the consolidator. Do not apply the first minimal patch proposal if it only fixes a fixture or local symptom while preserving a broader runtime mismatch.
11. Before closing a high-risk runtime defect, run at least one coach/verifier pass over the implemented patch and proof plan. If no coach lane is available, record that capacity blocker in the TaskFlow note and keep closure conservative.

## Proof And Closure

Minimal proof for this skill's work:

1. `vida task validate-graph --json`
2. Focused DocFlow check when docs changed.
3. Relevant runtime parity command, such as `vida status --json` plus `vida doctor --json`.
4. `git status --short` before reporting closure.
5. Any GitHub issue or PR state from live `gh` or connector evidence when GitHub surfaces changed.

## Test Coverage Standard

1. Runtime defects that block another project, affect operator JSON, change dispatch/run-graph/lane/TaskFlow state, or repair a security/actionability bug require integration or smoke coverage through the public CLI surface. Unit tests alone are not sufficient.
2. Target 100% coverage of the changed behavior path: success path, fail-closed blocker, machine-readable `blocker_codes`, `next_actions`, and `artifact_refs` needed for the operator to act.
3. For parity defects, test every affected public surface together, such as `status`, `doctor`, `consume continue`, `dispatch-next`, and `run-graph status`, instead of testing only the helper that computes the flag.
4. For packet/state repair defects, add at least one fixture that uses persisted runtime state and invokes the same command family an operator or agent would run.
5. If an integration test is impossible in the current slice, record the exact blocker in the TaskFlow note, keep the task open unless the user explicitly accepts the risk, and create a follow-up before closing related parent work.
6. Keep unit tests for small classifiers/helpers, but treat them as supporting proof below the integration/smoke test.

Do not close GitHub issues, epics, or TaskFlow parents until the relevant children and proof targets are current.
