# Silent Framework Diagnosis Protocol

Purpose: define the always-on silent VIDA framework diagnosis mode that monitors framework/runtime friction during normal work without hijacking the current user task.

Direction:

1. VIDA is an agentic engineering platform, not just a prompt wrapper.
2. The target is high-quality, token-efficient automation for agent and subagent work.
3. Quality and token efficiency are equal-weight goals in silent diagnosis mode.

## Activation

Silent diagnosis activates only when root `vida.config.yaml` declares:

1. `framework_self_diagnosis.enabled: true`
2. `framework_self_diagnosis.silent_mode: true`

When active, the orchestrator must treat silent diagnosis as a background framework guardrail, not as permission to derail the current task.

## Core Rules

1. Detect framework/runtime problems opportunistically during normal work.
2. If a framework problem is observed, create or reuse a framework bug immediately.
3. Continue the current user task with the lightest safe manual workaround when possible.
4. Do not silently patch VIDA framework code mid-task unless the user explicitly reprioritizes framework work now.
5. After the current task boundary, take the captured framework bug into the framework queue and fix it systematically.
6. Framework bug work must use WVP/web research when the fix depends on external best practice, tool behavior, or architecture claims.
7. Closure-ready state for framework bug work still requires normal delegated verification/proving rules.
8. When TODO/`br` flow is active, `docs/framework/todo-protocol.md` is the canonical execution-layer contract for deferred capture, compact-safe persistence, and post-boundary follow-up routing.
9. Durable lessons, corrections, and anomalies belong in `docs/framework/framework-memory-protocol.md`, not only in chat or transient reflection output.
10. Silent diagnosis must audit the instruction layer as well as runtime/task flow: `AGENTS.md`, lane entry contracts, and canonical framework protocols are valid diagnosis targets when they reduce quality, clarity, token efficiency, or determinism.
11. Silent diagnosis must also audit protocol execution drift using `docs/framework/protocol-self-diagnosis-protocol.md` when reporting barriers, missing task coverage, skipped catch-review, or route drift are observed.

## Bug Capture Contract

Capture records must include:

1. concise summary,
2. current task id if known,
3. current-task workaround if any,
4. enough detail to reproduce or classify the framework gap,
5. linkage to the active framework epic/wave when configured.

Canonical helper:

```bash
python3 docs/framework/history/_vida-source/scripts/vida-silent-diagnosis.py capture \
  --summary "<framework issue>" \
  --details "<what happened>" \
  --current-task "<task_id>" \
  --workaround "<temporary workaround>"
```

## Deferred Fix Rule

Silent diagnosis is intentionally deferred:

1. detect now,
2. capture now,
3. finish the current task safely,
4. return to the framework bug after the task boundary,
5. then resume project work.

This keeps product momentum while preserving framework integrity.

## Session Reflection

If `framework_self_diagnosis.session_reflection_required: true`, the orchestrator must run a self-reflection pass near task/session completion against:

1. architecture cleanliness,
2. completeness,
3. token efficiency,
4. orchestration efficiency,
5. instruction clarity and determinism,
6. instruction/runtime consistency.

If new framework gaps are found, create follow-up framework tasks/bugs and route them through the normal VIDA framework workflow.
Record durable anomalies or reusable lessons in framework memory when they should influence future framework work beyond the current session.

Instruction reflection rule:

1. Reflection must include whether `AGENTS.md`, lane entry contracts, and canonical protocols contain vague optionality, conflicting precedence, missing decision boundaries, or instruction drift relative to runtime behavior.
2. When instruction-layer friction increases iterations, context rereads, or token spend, capture it as a framework issue even if runtime code still works.
