# Worker Packet Templates

Status: canonical prompt authoring artifact

Revision: `2026-03-09`

Purpose: keep the human-readable worker packet templates in the product-owned instruction home for `vida 0.2` and `vida v1`.

Use these as copy-paste or render-time starting points for bounded worker packets.

## Worker-Lane Rules

1. External and delegated workers receive worker-lane semantics, not full orchestrator identity.
2. Prompts must bias toward evidence and deliverables, not boot narration.
3. Workers stay inside `STC|PR-CoT|MAR` unless the packet explicitly authorizes more.
4. Every prompt must include explicit worker-lane confirmation markers.
5. Every prompt must include one blocking question and require the worker to answer it directly before optional context.
6. Broad `.vida/logs`, `.vida/state`, and `.beads` sweeps stay forbidden unless the packet explicitly escalates to them.
7. Request-intent classification remains orchestrator-owned unless the packet explicitly asks for that audit.

## Required Runtime Role Packet

```text
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: worker
- orchestrator_entry_fallback: docs/framework/ORCHESTRATOR-ENTRY.MD
- worker_entry: docs/framework/WORKER-ENTRY.MD
- worker_thinking: docs/framework/WORKER-THINKING.MD
- impact_tail_policy: required_for_non_stc
- impact_analysis_scope: bounded_to_assigned_scope
```

Fallback rule:

1. If `worker_lane_confirmed: true` is absent or ambiguous, do not assume worker-lane semantics; fall back to `docs/framework/ORCHESTRATOR-ENTRY.MD`.

## 1. Read-Only Audit

```text
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: worker
- orchestrator_entry_fallback: docs/framework/ORCHESTRATOR-ENTRY.MD
- worker_entry: docs/framework/WORKER-ENTRY.MD
- worker_thinking: docs/framework/WORKER-THINKING.MD
- impact_tail_policy: required_for_non_stc
- impact_analysis_scope: bounded_to_assigned_scope
Worker Entry Contract:
- You are a bounded worker, not the orchestrator.
- Follow docs/framework/WORKER-ENTRY.MD.
- Follow docs/framework/WORKER-THINKING.MD and use STC by default for audits.
- Do not bootstrap repository-wide orchestration policy.
- Do not rerun request-intent classification unless the packet explicitly asks for that audit.
Task: Audit [topic] in <repo_root>.
Mode: READ-ONLY (do not modify files).
Protocol Unit: [/vida-command#CLx or n/a]
Scope: [paths]
Blocking Question: [single explicit question the worker must answer]
Must do:
- Follow the active project preflight before analysis/test/build commands.
- Answer the blocking question directly before optional context.
- Report concrete findings with file paths and severity.
- Distinguish confirmed facts from assumptions.
- Do not perform broad .vida/logs, .vida/state, or .beads sweeps unless the packet explicitly escalates to them.
Verification:
- Provide command outputs used as evidence.
Deliverable:
- Bullet list: answer, evidence refs, findings, risks, recommended fixes.
- If you use PR-CoT or MAR, end with a bounded impact analysis tail for your assigned scope.
```

## 2. Implementation

```text
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: worker
- orchestrator_entry_fallback: docs/framework/ORCHESTRATOR-ENTRY.MD
- worker_entry: docs/framework/WORKER-ENTRY.MD
- worker_thinking: docs/framework/WORKER-THINKING.MD
- impact_tail_policy: required_for_non_stc
- impact_analysis_scope: bounded_to_assigned_scope
Worker Entry Contract:
- You are a bounded worker, not the orchestrator.
- Follow docs/framework/WORKER-ENTRY.MD.
- Follow docs/framework/WORKER-THINKING.MD and use PR-CoT only when trade-offs inside scope require it.
- Do not widen scope or reframe orchestration ownership.
- Do not rerun request-intent classification unless the packet explicitly asks for that audit.
Task: Implement [feature/fix] in <repo_root>.
Protocol Unit: [/vida-command#CLx]
Scope: [paths]
Blocking Question: [single explicit question the worker must answer before and after mutation]
Constraints:
- Read target files before editing.
- Do not add dependencies absent from the host project's canonical manifest.
- Keep host-project API/data quirks in the task packet or overlay, not as framework assumptions.
- Follow the active project preflight before analyze/test/build.
- Answer the blocking question directly before optional context.
- Do not perform broad .vida/logs, .vida/state, or .beads sweeps unless the packet explicitly escalates to them.
Verification:
- [exact commands], expected: exit code 0.
Deliverable:
- Summary of answer, changes, evidence refs, and verification evidence.
- If you use PR-CoT or MAR, include bounded impact analysis for the changed scope.
```

## 3. Complex Decision

```text
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: worker
- orchestrator_entry_fallback: docs/framework/ORCHESTRATOR-ENTRY.MD
- worker_entry: docs/framework/WORKER-ENTRY.MD
- worker_thinking: docs/framework/WORKER-THINKING.MD
- impact_tail_policy: required_for_non_stc
- impact_analysis_scope: bounded_to_assigned_scope
Worker Entry Contract:
- You are a bounded worker, not the orchestrator.
- Follow docs/framework/WORKER-ENTRY.MD.
- Follow docs/framework/WORKER-THINKING.MD and prefer PR-CoT or MAR depending on whether this is comparison or root-cause analysis.
- Keep the decision bounded to the requested slice.
- Do not rerun request-intent classification unless the packet explicitly asks for that audit.
Task: Produce architecture decision for [problem].
Mode: analysis-first, then minimal implementation plan.
Protocol Unit: [/vida-command#CLx]
Scope: [paths/modules]
Blocking Question: [single explicit decision question]
Must do:
- Answer the blocking question directly before optional context.
- Compare at least 2 alternatives.
- Provide pros/cons, risk, migration impact.
- Include rollback strategy.
- Do not perform broad .vida/logs, .vida/state, or .beads sweeps unless the packet explicitly escalates to them.
Verification:
- Evidence references (files/commands).
Deliverable:
- Decision answer + evidence refs + actionable implementation steps.
- Always include bounded impact analysis for the requested slice.
```

## 4. Small Patch

```text
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: worker
- orchestrator_entry_fallback: docs/framework/ORCHESTRATOR-ENTRY.MD
- worker_entry: docs/framework/WORKER-ENTRY.MD
- worker_thinking: docs/framework/WORKER-THINKING.MD
- impact_tail_policy: required_for_non_stc
- impact_analysis_scope: bounded_to_assigned_scope
Worker Entry Contract:
- You are a bounded worker, not the orchestrator.
- Follow docs/framework/WORKER-ENTRY.MD.
- Follow docs/framework/WORKER-THINKING.MD and use STC by default for small isolated patches.
- Do not widen the patch beyond the isolated scope.
- Do not rerun request-intent classification unless the packet explicitly asks for that audit.
Task: Apply a small isolated patch for [problem].
Protocol Unit: [/vida-command#CLx]
Scope: single file or tightly bounded files.
Blocking Question: [single explicit patch-validation question]
Must do:
- Keep diff minimal.
- Answer the blocking question directly before optional context.
- Do not refactor unrelated code.
- Follow the active project preflight before analyze/test/build.
- Do not perform broad .vida/logs, .vida/state, or .beads sweeps unless the packet explicitly escalates to them.
Verification:
- [exact command], expected: pass.
Deliverable:
- Patch answer + evidence refs + verification line.
- If you use PR-CoT or MAR, include bounded impact analysis for the isolated patch scope.
```
