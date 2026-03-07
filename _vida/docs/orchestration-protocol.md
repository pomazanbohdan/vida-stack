# Orchestration Protocol (OP)

Purpose: define how the top-level orchestrator turns a non-trivial request into a delivery-ready result.

## Core Contract

1. Orchestration is protocol-driven and uses `br` + packs only when request intent requires task execution or artifact production.
2. The orchestrator owns problem framing, lens selection, workstream decomposition, agent routing, synthesis, and the final quality gate.
3. Packs are the runtime route; orchestration lenses are the reasoning posture. A lens never replaces a pack.
4. Task state lives only in `br`; execution visibility lives only in TODO blocks when task flow is engaged.
5. No interactive menu dependencies.

## Scope

1. Entry behavior is executed via scripts (`vida-pack-helper.sh`) and TODO blocks.
2. This protocol covers request interpretation above command-level execution details.
3. Detailed task-state, TODO, and subagent-routing rules remain in their canonical protocols.

## Inputs

1. User request text.
2. Optional explicit `task_id`.
3. Existing task, decision, or spec context when present.

## Request Intent Classes

Classify request intent before task resolution:

1. `answer_only`
   - explanation, diagnosis, comparison, review findings, framework discussion, architecture recommendation,
   - no automatic `br`, TODO, or pack flow.
2. `artifact_flow`
   - research artifact, spec, task-pool, formal report, docs update, decision record,
   - `br` + pack + TODO required.
3. `execution_flow`
   - implementation, bug fix, refactor, protocol/script/code mutation,
   - `br` + TODO required.
4. `mixed`
   - starts as `answer_only`,
   - enters task flow only after explicit mutation decision, approved task context, or user-confirmed execution scope.

## Orchestration Lenses

1. `discovery`
2. `product_strategy`
3. `business_analysis`
4. `systems_analysis`
5. `architecture`
6. `delivery_planning`
7. `implementation_support`
8. `review_audit`
9. `multi_agent_debate`
10. `recovery_debug`

Selection rule:

1. Pick the smallest sufficient lens set for the request.
2. Mixed requests may activate multiple lenses, but the orchestrator still returns one integrated outcome.

## Problem Framing Contract

Before routing work, normalize the request into:

1. target problem or desired outcome,
2. business/product/system goal,
3. explicit constraints and hidden assumptions,
4. scope boundary (`in`, `out`, dependencies),
5. symptom vs root-cause distinction when relevant,
6. readiness risks or missing evidence.

## Algorithm

1. Frame the problem.
2. Determine request intent class and active orchestration lens.
3. Apply TODO engagement gate:
   - `answer_only` -> stay outside `br`/TODO/pack flow,
   - `artifact_flow` and `execution_flow` -> task flow required,
   - `mixed` -> start answer path first and enter task flow only when execution becomes required.
4. Resolve active task only when task flow is required:
   - if `task_id` is provided, use it;
   - else prefer active `in_progress` task from `br`;
   - else pick first `ready` task or create one when the request is net-new framework/project work.
5. Detect pack only when task flow is required:
   - `bash _vida/scripts/vida-pack-helper.sh detect "<request>"`.
6. Select execution mode/profile:
   - task execution mode via `_vida/scripts/task-execution-mode.sh`,
   - boot profile via `_vida/scripts/boot-profile.sh`,
   - META / FSAP / SCP / WVP when triggers fire.
7. Select orchestration hierarchy:
   - default to free external read-only fanout for eligible non-trivial analysis/research/review/verification work,
   - use the configured bridge fallback subagent before internal escalation,
   - reserve internal subagents for senior arbitration, architecture-heavy synthesis, and mutation-owning execution.
8. Decompose the work into layers:
   - analysis,
   - design/contract,
   - implementation/materialization,
   - validation,
   - governance/documentation,
   - delivery/handoff.
9. Decide dependency order:
   - parallel only for independent read-only or isolated-scope steps,
   - otherwise keep a single writer lane on `track_id=main`.
10. Inject expert agents only when needed:
   - missing domain expertise,
   - risk cannot be assessed confidently,
   - cross-functional consequences require additional lenses,
   - conflict arbitration or critique is needed,
   - read-only ensemble fanout is available and materially reduces risk.
10.1. For eligible non-trivial work, prefer separate cli-subagent lanes for authorship and verification:
   - one cli subagent or cli-subagent ensemble produces the primary analysis/recommendation,
   - another eligible cli subagent (or verification ensemble) validates it independently when route policy requires it,
   - the orchestrator owns synthesis, escalation, and mutation-only control.
11. Start pack session only when task flow is required:
   - `bash _vida/scripts/vida-pack-helper.sh start <task_id> <pack_id> "<goal>" [constraints]`.
   - optional shortcut for standard non-dev flows:
     `bash _vida/scripts/nondev-pack-init.sh <task_id> <pack_id> "<goal>" [constraints]`.
12. Pre-register execution blocks only when task flow is required:
   - `bash _vida/scripts/vida-pack-helper.sh scaffold <task_id> <pack_id>`.
13. Execute via TODO lifecycle only when task flow is engaged:
   - `block-plan -> block-start -> block-end -> reflect -> verify`.
14. Synthesize results:
   - integrate business, product, architecture, implementation, and verification outputs,
   - resolve conflicts before reporting,
   - convert the result into an execution-ready artifact when appropriate.
15. End pack session only when task flow was engaged:
   - `bash _vida/scripts/vida-pack-helper.sh end <task_id> <pack_id> <done|partial|failed> "<summary>" [next_step]`.

## Dynamic Expert Injection

When dispatching additional agents, define all of:

1. domain role,
2. bounded scope,
3. explicit context,
4. expected output format,
5. verification boundary,
6. merge owner (always the orchestrator).

Routing rule:

1. Use `_vida/docs/subagent-system-protocol.md` + project overlay for subagent choice.
2. Use `_vida/docs/subagents.md` for dispatch contract.
3. For eligible non-trivial read-heavy work, prefer subagent-first execution whenever the active subagent mode is not `disabled`.
4. In `hybrid`, prefer external free fanout first, then the configured bridge fallback, then internal senior escalation only when route policy or evidence requires it.
5. In `native`, prefer internal subagents as the first analysis/review lane.
6. In `disabled`, keep analysis local and obey bounded-read policy.
7. Keep writer ownership singular under the orchestrator even when read-only fanout is active.
8. Prefer independent verification by a different cli subagent when route metadata marks independent verification as required and a distinct eligible verifier exists.

## Conflict Resolution

When agent or domain outputs disagree:

1. Locate whether the conflict is in assumptions, goals, constraints, evidence, or domain interpretation.
2. Prefer canonical evidence and protocol ownership over eloquence or volume.
3. Synthesize one decision or surface a bounded user decision when equivalent paths remain.
4. Do not forward contradictory raw fragments as if they were a final answer.

## User-Facing Reporting

When subagents participate in the flow:

1. treat subagent outputs as internal evidence unless the user explicitly asks to inspect them,
2. present one orchestrator-synthesized answer in chat,
3. do not stream or paste raw subagent reports into the final user response by default,
4. reference subagent findings only through synthesized conclusions, evidence refs, or clearly marked supporting summaries,
5. expose raw subagent disagreement only when it remains decision-relevant after synthesis.

## Delivery Alignment

When appropriate, convert the synthesized outcome into:

1. backlog-ready tasks,
2. implementation phases,
3. architecture decisions,
4. acceptance criteria,
5. validation or rollout checklists,
6. escalation points.

## Quality Gate

Do not finalize until the orchestrator can answer yes to all:

1. Is the real problem framed correctly?
2. Are assumptions and constraints explicit?
3. Are missing perspectives covered or consciously deferred?
4. Are internal contradictions resolved?
5. Is the result practically executable?
6. Are risks, dependencies, and trade-offs visible?
7. Is the outcome aligned with product and delivery goals?
8. Are next actions explicit when work remains?

## Decision Matrix

0. answer-only advisory/diagnosis/review request -> no pack, no automatic `br`, no TODO.
1. research request -> `research-pack`.
2. spec creation/update -> `spec-pack`.
3. scope/task pool formation -> `work-pool-pack` (`/vida-form-task`).
4. implementation -> `dev-pack` (`/vida-implement`).
5. bug investigation/fix -> `bug-pool-pack` (`/vida-bug-fix`).
6. docs/protocol synchronization or change-impact reconciliation -> `reflection-pack`.
7. explicit VIDA/framework self-analysis request -> `reflection-pack` + `_vida/docs/framework-self-analysis-protocol.md`.

Change-impact triggers routed to `reflection-pack`:

1. scope/AC/dependency drift discovered mid-flow,
2. decision changes that invalidate current task pool,
3. mismatch between approved spec and executable `br` queue.

## Output Contract

1. For task-flow outputs, include active task id + short description.
2. For task-flow outputs, include selected pack id + short goal.
3. For task-flow outputs, include planned/started blocks snapshot.
4. Include active orchestration lens or lens set when useful.
5. For non-trivial reports, default report order:
   - `Problem Framing`
   - `Assumptions / Constraints`
   - `Agent Orchestration Summary`
   - `Integrated Analysis`
   - `Recommended Solution`
   - `Risks / Trade-offs`
   - `Next Actions`
6. Completion state and next step.
7. When subagents contributed, report the orchestrator's synthesized result, not raw subagent text, unless the user explicitly asks for the raw output.

## Constraints

1. Do not mutate task state outside `br`.
2. Do not execute task-flow work outside active TODO block lifecycle.
3. Do not engage `br`/TODO/pack flow for `answer_only` requests by default.
4. Do not route through non-canonical command paths.
5. Do not use multiple writer lanes without explicit scope isolation.
6. Do not replace synthesis with unintegrated agent fragments.
7. Do not expose raw subagent reports as the default user-facing deliverable.

## Related

1. `_vida/docs/use-case-packs.md`
2. `_vida/docs/todo-protocol.md`
3. `_vida/docs/beads-protocol.md`
4. `_vida/scripts/vida-pack-helper.sh`
5. `_vida/docs/subagent-system-protocol.md`
6. `_vida/docs/subagents.md`
