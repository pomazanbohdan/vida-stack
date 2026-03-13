# Silent Framework Diagnosis Protocol

Purpose: define the always-on silent VIDA framework diagnosis mode that monitors framework/runtime friction during normal work without hijacking the current user task.

Direction:

1. VIDA is an agentic engineering platform, not just a prompt wrapper.
2. The target is high-quality, token-efficient automation for agent and worker work.
3. Quality and token efficiency are equal-weight goals in silent diagnosis mode.

## Activation

Silent diagnosis activates only when root `vida.config.yaml` declares:

1. `framework_self_diagnosis.enabled: true`
2. `framework_self_diagnosis.silent_mode: true`

When active, the orchestrator must treat silent diagnosis as a background framework guardrail, not as permission to derail the current task.

## Core Rules

1. Detect framework/runtime problems opportunistically during normal work.
2. If a framework problem is observed, create or reuse a framework bug immediately.
3. Continue the current user task with the lightest safe lawful workaround when possible.
4. Do not silently patch VIDA framework code mid-task unless the user explicitly reprioritizes framework work now.
5. After the current task boundary, take the captured framework bug into the framework queue and fix it systematically.
6. Framework bug work must use WVP/web research when the fix depends on external best practice, tool behavior, or architecture claims.
7. Closure-ready state for framework bug work still requires normal delegated verification/proving rules.
8. When TaskFlow is active, `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md` is the canonical execution-layer contract for deferred capture, compact-safe persistence, and post-boundary follow-up routing.
9. Durable lessons, corrections, and anomalies belong in `vida/config/instructions/runtime-instructions/runtime.framework-memory-protocol.md`, not only in chat or transient reflection output.
10. Silent diagnosis must audit the instruction layer as well as runtime/TaskFlow: `AGENTS.md`, lane entry contracts, and canonical framework protocols are valid diagnosis targets when they reduce quality, clarity, token efficiency, or determinism.
11. Silent diagnosis must also audit protocol execution drift using `vida/config/instructions/diagnostic-instructions/analysis.protocol-self-diagnosis-protocol.md` when reporting barriers, missing task coverage, skipped catch-review, or route drift are observed.
12. Timeout-driven pauses, report-after-timeout stopping, or other generic-assistant waiting behavior during `in_work=1` must be captured as framework drift rather than accepted as normal session pacing.
13. Silent diagnosis does not choose between `diagnosis_path` and `normal_delivery_path` implicitly; when a turn mixes report/diagnosis intent with continued development, path selection must be made explicitly before any write-producing action.
14. Silent diagnosis pressure, discovered defect visibility, or “scope is already clear” do not authorize local product patching until that path selection and downstream route law are explicit.
15. If live runtime evidence reports `delegated_cycle_open=true` together with `local_exception_takeover_gate=blocked_open_delegated_cycle`, silent diagnosis must stay in orchestrator/diagnosis posture and treat the state as an open process-conflict boundary rather than as permission for a manual local workaround.
16. In that state, lawful continuation is limited to bounded delegated-lane inspection, lawful waiting/polling, capture/reuse of the framework bug, explicit blocker or escalation routing, or a user-facing process-conflict report that does not perform local write work.
17. Implementer delay, hanging subordinate lanes, or partially recovered delivery context do not relax the `blocked_open_delegated_cycle` gate; local write work remains forbidden until canonical supersession, hard blocker evidence, or higher-precedence route law is recorded.
18. Silent diagnosis must treat "I repaired one failing test and it turned green" as bounded evidence only; it does not prove that the active development context is exhausted unless the parent task/packet was rebuilt and found fully closed.

## Bug Capture Contract

Capture records must include:

1. concise summary,
2. current task id if known,
3. current-task workaround if any,
4. enough detail to reproduce or classify the framework gap,
5. linkage to the active framework epic/wave when configured.

Canonical helper:

```bash
python3 vida-silent-diagnosis.py capture \
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

-----
artifact_path: config/diagnostic-instructions/silent-framework-diagnosis.protocol
artifact_type: diagnostic_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/diagnostic-instructions/analysis.silent-framework-diagnosis-protocol.md
created_at: '2026-03-07T20:46:00+02:00'
updated_at: '2026-03-11T13:34:34+02:00'
changelog_ref: analysis.silent-framework-diagnosis-protocol.changelog.jsonl
