# Subagent Prompt Templates

Use these as copy-paste starting points.

For routine dispatches, prefer the render helper so `<repo_root>` and project preflight are injected automatically:

```bash
bash _vida/scripts/render-subagent-prompt.sh implementation \
  --task "Implement [feature/fix]" \
  --protocol-unit "/vida-implement#CL4" \
  --scope "[paths]" \
  --verification "[exact command]"
```

Worker-lane rule:

1. external/delegated workers should receive `_vida/docs/SUBAGENT-ENTRY.MD` semantics, not the full `AGENTS.md` orchestrator identity.
2. prompts should bias toward evidence and deliverables, not boot narration.
3. external/delegated workers should receive `_vida/docs/SUBAGENT-THINKING.MD` semantics and stay inside `STC|PR-CoT|MAR` unless explicitly instructed otherwise.
4. prompts should include an explicit runtime role packet that confirms worker-lane semantics.
5. prompts should include one blocking question and require the worker to answer it directly before optional context.
6. prompts should forbid broad `.vida/logs`, `.vida/state`, and `.beads` sweeps unless the packet explicitly escalates to them.
7. request-intent classification is orchestrator-owned; workers must not rerun it unless the packet explicitly asks for that audit.

Required runtime role packet:

```text
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: subagent
- orchestrator_entry_fallback: _vida/docs/ORCHESTRATOR-ENTRY.MD
- worker_entry: _vida/docs/SUBAGENT-ENTRY.MD
- worker_thinking: _vida/docs/SUBAGENT-THINKING.MD
```

Fallback rule:

1. If `worker_lane_confirmed: true` is absent or ambiguous, do not assume worker-lane semantics; fall back to `_vida/docs/ORCHESTRATOR-ENTRY.MD`.

## 1) Read-Only Audit (Qwen)

```text
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: subagent
- orchestrator_entry_fallback: _vida/docs/ORCHESTRATOR-ENTRY.MD
- worker_entry: _vida/docs/SUBAGENT-ENTRY.MD
- worker_thinking: _vida/docs/SUBAGENT-THINKING.MD
Worker Entry Contract:
- You are a bounded worker, not the orchestrator.
- Follow _vida/docs/SUBAGENT-ENTRY.MD.
- Follow _vida/docs/SUBAGENT-THINKING.MD and use STC by default for audits.
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
```

## 2) Implementation (Codex 5.3)

```text
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: subagent
- orchestrator_entry_fallback: _vida/docs/ORCHESTRATOR-ENTRY.MD
- worker_entry: _vida/docs/SUBAGENT-ENTRY.MD
- worker_thinking: _vida/docs/SUBAGENT-THINKING.MD
Worker Entry Contract:
- You are a bounded worker, not the orchestrator.
- Follow _vida/docs/SUBAGENT-ENTRY.MD.
- Follow _vida/docs/SUBAGENT-THINKING.MD and use PR-CoT only when trade-offs inside scope require it.
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
```

## 3) Complex Decision (Codex 5.2)

```text
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: subagent
- orchestrator_entry_fallback: _vida/docs/ORCHESTRATOR-ENTRY.MD
- worker_entry: _vida/docs/SUBAGENT-ENTRY.MD
- worker_thinking: _vida/docs/SUBAGENT-THINKING.MD
Worker Entry Contract:
- You are a bounded worker, not the orchestrator.
- Follow _vida/docs/SUBAGENT-ENTRY.MD.
- Follow _vida/docs/SUBAGENT-THINKING.MD and prefer PR-CoT or MAR depending on whether this is comparison or root-cause analysis.
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
```

## 4) Small Patch (Codex mini)

```text
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: subagent
- orchestrator_entry_fallback: _vida/docs/ORCHESTRATOR-ENTRY.MD
- worker_entry: _vida/docs/SUBAGENT-ENTRY.MD
- worker_thinking: _vida/docs/SUBAGENT-THINKING.MD
Worker Entry Contract:
- You are a bounded worker, not the orchestrator.
- Follow _vida/docs/SUBAGENT-ENTRY.MD.
- Follow _vida/docs/SUBAGENT-THINKING.MD and use STC by default for small isolated patches.
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
```
