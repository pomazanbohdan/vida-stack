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

## 1) Read-Only Audit (Qwen)

```text
Worker Entry Contract:
- You are a bounded worker, not the orchestrator.
- Follow _vida/docs/SUBAGENT-ENTRY.MD.
- Do not bootstrap repository-wide orchestration policy.
Task: Audit [topic] in <repo_root>.
Mode: READ-ONLY (do not modify files).
Protocol Unit: [/vida-command#CLx or n/a]
Scope: [paths]
Must do:
- Follow the active project preflight before analysis/test/build commands.
- Report concrete findings with file paths and severity.
- Distinguish confirmed facts from assumptions.
Verification:
- Provide command outputs used as evidence.
Deliverable:
- Bullet list: findings, risks, recommended fixes.
```

## 2) Implementation (Codex 5.3)

```text
Worker Entry Contract:
- You are a bounded worker, not the orchestrator.
- Follow _vida/docs/SUBAGENT-ENTRY.MD.
- Do not widen scope or reframe orchestration ownership.
Task: Implement [feature/fix] in <repo_root>.
Protocol Unit: [/vida-command#CLx]
Scope: [paths]
Constraints:
- Read target files before editing.
- Do not add dependencies absent from the host project's canonical manifest.
- Keep host-project API/data quirks in the task packet or overlay, not as framework assumptions.
- Follow the active project preflight before analyze/test/build.
Verification:
- [exact commands], expected: exit code 0.
Deliverable:
- Summary of changes + verification evidence.
```

## 3) Complex Decision (Codex 5.2)

```text
Worker Entry Contract:
- You are a bounded worker, not the orchestrator.
- Follow _vida/docs/SUBAGENT-ENTRY.MD.
- Keep the decision bounded to the requested slice.
Task: Produce architecture decision for [problem].
Mode: analysis-first, then minimal implementation plan.
Protocol Unit: [/vida-command#CLx]
Scope: [paths/modules]
Must do:
- Compare at least 2 alternatives.
- Provide pros/cons, risk, migration impact.
- Include rollback strategy.
Verification:
- Evidence references (files/commands).
Deliverable:
- Decision memo + actionable implementation steps.
```

## 4) Small Patch (Codex mini)

```text
Worker Entry Contract:
- You are a bounded worker, not the orchestrator.
- Follow _vida/docs/SUBAGENT-ENTRY.MD.
- Do not widen the patch beyond the isolated scope.
Task: Apply a small isolated patch for [problem].
Protocol Unit: [/vida-command#CLx]
Scope: single file or tightly bounded files.
Must do:
- Keep diff minimal.
- Do not refactor unrelated code.
- Follow the active project preflight before analyze/test/build.
Verification:
- [exact command], expected: pass.
Deliverable:
- Patch summary + verification line.
```
