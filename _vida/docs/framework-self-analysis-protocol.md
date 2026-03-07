# VIDA Framework Self-Analysis Protocol (FSAP)

Purpose: run a bounded meta-diagnostic of the VIDA framework itself when the user explicitly asks to inspect protocol friction, instruction conflicts, token overhead, runtime ergonomics, or framework/process efficiency.

## Trigger

Run FSAP only on explicit user request, for example:

1. "diagnose VIDA/framework"
2. "analyze what should be improved in the framework"
3. "run VIDA self-analysis"
4. "check instruction or script conflicts"
5. "find what reduces iterations, token cost, or context rereads"

Do not use FSAP for product/codebase diagnosis unless the user explicitly asks about the framework/runtime itself.

## Routing

1. Default execution lane: main orchestrator only.
2. Default task-flow policy: bypass TODO/`br`/pack flow and execute in chat mode unless the user explicitly requests tracked execution or a formal artifact.
3. Delegation policy:
   - do not delegate the primary FSAP analysis to subagents by default;
   - use delegated lanes only for narrow secondary verification when the orchestrator is blocked or the user explicitly requests delegation.
4. Thinking mode:
   - default `META` for explicit self-analysis requests;
   - downgrade to `MAR` only for narrow single-script questions with low blast radius.
5. Scope: `AGENTS.md`, `_vida/docs/*`, `_vida/scripts/*`, runtime logs, and only the project evidence that proves a framework-level friction point.

When the user explicitly requests tracked execution, use `reflection-pack` and the dedicated FSAP chain:

1. `FSAP01`: `FSAP-0_2_Trigger_Runtime_Snapshot_and_Evidence_Scope`
2. `FSAP02`: `FSAP-3_5_Friction_Classification_Ownership_Split_and_Improvement_Decision`
3. `FSAP03`: `FSAP-6_8_Canonical_Update_Verification_and_Report`

## Core Boundary

FSAP must separate findings into two ownership buckets:

1. `framework-owned`
   - VIDA runtime protocols
   - AGENTS rules
   - `_vida/docs/*`
   - `_vida/scripts/*`
2. `project-owned`
   - app-specific runbooks
   - `docs/*`
   - `scripts/*`
   - codebase/tooling issues that only expose a framework gap

Rule:

1. Do not "fix project pain" inside `_vida/*`.
2. Do not store framework policy in `docs/*`.
3. If one symptom spans both layers, produce split actions per ownership layer.

## FSAP Workflow

1. `FSAP-0 Trigger Confirmation`
   - confirm the request is about VIDA/framework behavior, not product behavior.
2. `FSAP-1 Runtime State Snapshot`
   - capture the current orchestrator/runtime state and relevant health/status views.
   - if tracked execution is active, also capture current task id, active TODO block, and pack state.
   - preferred shortcuts:
     - dev/task-state visibility: `python3 _vida/scripts/vida-boot-snapshot.py`
     - untracked mode: `python3 _vida/scripts/subagent-system.py status` plus bounded queue/status reads as needed,
     - tracked mode: `bash _vida/scripts/framework-self-check.sh <task_id>`.
3. `FSAP-2 Evidence Collection`
   - inspect only the protocols/scripts actually involved in the observed friction.
   - prefer direct script/doc reads over broad repo sweeps.
4. `FSAP-3 Friction Classification`
   - classify each issue as:
     - protocol gap
     - script/runtime bug
     - instruction conflict
     - ergonomics/observability gap
     - project issue mislocated in framework
5. `FSAP-4 Ownership Split`
   - mark every finding `framework-owned` or `project-owned`.
6. `FSAP-5 Improvement Decision`
   - choose fixes that reduce:
     - iteration count
     - repeated rereads
     - stale state/conflicting status
     - unnecessary token spend
     - ambiguous ownership
7. `FSAP-6 Canonical Update`
   - update framework files in `_vida/*`.
   - if project fixes are in scope, update `docs/*` / `scripts/*` separately in the same request.
8. `FSAP-7 Verification`
   - run the lightest proof that the framework fix changed behavior.
9. `FSAP-8 Report`
   - report findings in chat, grouped by ownership.

## Required Evidence

Every FSAP report must include:

1. active execution context:
   - untracked mode: direct orchestrator FSAP run,
   - tracked mode: active `br` task id + short description
2. active TODO block(s) when tracked mode is active
3. concrete file/script references for each finding
4. why the issue increases iterations/context/tokens
5. whether the fix belongs to framework or project layer

## Preferred Verification

Use the smallest proof that demonstrates the framework change:

1. `bash -n` for shell scripts
2. `todo-tool current|compact` for TODO/runtime state fixes in tracked mode
3. `quality-health-check.sh --mode quick <task_id>` for protocol sanity in tracked mode
4. a focused smoke command that reproduces the improved behavior

Avoid full project build/test loops unless the framework change directly affects them.

## Optional Tracked Escalation

If the user explicitly requests task tracking, formal artifact production, or deferred multi-step follow-through, FSAP may escalate into tracked mode:

```bash
bash _vida/scripts/framework-wave-start.sh <task_id> <reflection-pack|dev-pack|work-pool-pack> "<goal>" [constraints]
```

Use this only when at least one is true:

1. the user explicitly asks for tracked execution,
2. the diagnosis must create or update formal artifacts,
3. the follow-up work is too large for a direct orchestrator chat run.

## Output Contract

Report in this structure:

1. `Framework-owned findings`
2. `Project-owned findings`
3. `Implemented framework improvements`
4. `Implemented project improvements` (only if in scope)
5. `Residual risks / next best improvements`

## Anti-Patterns

1. Mixing project bugs into framework conclusions without ownership split.
2. Broad rereads of unrelated protocols that do not change the decision.
3. Reporting "framework is better now" without a concrete behavioral proof.
4. Leaving framework/project ownership ambiguous after the analysis.
5. Auto-routing explicit VIDA self-diagnosis into TODO/`br` flow when the user asked for direct diagnosis only.
6. Delegating the primary FSAP analysis away from the main orchestrator without an explicit reason.
7. Starting token-cost diagnosis with broad queue discovery before trying the compact boot snapshot or another exact-key status source.
