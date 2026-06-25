# Session Runtime Runbook

Use this reference when a VIDA runtime session touches TaskFlow, DocFlow, GitHub issue triage, operator command shape, or proof closure.

## Session Checklist

1. Confirm active runtime:
   - `vida orchestrator-init --json`
   - `vida task validate-graph --json`
2. If no active bounded unit exists, bind the current user request to the current epic before writing.
3. Before writes, create a DB-backed `step` with owner, active form, stop criterion, and fallback.
4. Keep TaskFlow mutations sequential.
5. Use up to four parallel read-only checks only when scopes are disjoint.
6. After mutation, validate graph and inspect the affected task/tree.
7. Close the step only after validation.

## Session-Derived Operator Gaps

The 2026-06-04 session exposed these reusable improvement needs:

1. `runtime-task-search-filter-command` - avoid full backlog scans for issue/task lookup.
2. `runtime-json-field-selector-output-profiles` - avoid wrapper-shape mistakes and raw reruns.
3. `runtime-task-tree-brief-children-view` - return child summaries without oversized tree output.
4. `runtime-task-close-feedback-literal-reason-regression-20260604` - stop classifying valid close reasons by raw trigger words.
5. `runtime-session-triage-proof-bundle-command` - bundle active unit, graph validation, task tree, and status/doctor parity.

When similar friction appears, update those tasks rather than creating duplicates.

## GitHub Issue Coupling

GitHub issues are public tracking and discussion surfaces. TaskFlow remains execution authority.

For GitHub issue work, use the `vida-github-issues` skill and `docs/process/github-issues-triage-guide.md`.

## Common Proof Bundle

For closeout or issue-triage work, collect:

1. target TaskFlow task/tree,
2. graph validation,
3. open GitHub issue list or target issue state,
4. status/doctor parity when runtime state is involved,
5. clean git status,
6. step close evidence.
