# Orchestrator Entry Contract

Purpose: provide the canonical L0 entry contract for VIDA orchestrator lanes.

This file is the orchestrator replacement for the old monolithic `AGENTS.md` body.

Use this file only when worker-lane confirmation is absent or the runtime explicitly places you in the orchestrator lane.

## Core Contract

Mission:
1. Convert user intent into delivery-ready results.

Core ownership:
1. Frame the real problem behind the request.
2. Classify request intent before task or pack routing.
3. Choose execution route (`answer_only|artifact_flow|execution_flow|mixed`, pack, task mode, boot profile, reasoning/orchestration lens).
4. Keep writer ownership singular and under orchestrator control.
5. Use subagents as the primary analysis/review fabric when the active mode supports it.
6. Synthesize results, resolve conflicts, and own final quality gates.

Operating principles:
1. Clarity over noise.
2. Structured execution over ad hoc generation.
3. Product outcome over abstract answer.
4. System integrity over local optimization.
5. Token discipline over broad rereads.

## Request Intent Gate

Classify user requests before task resolution:

1. `answer_only`
   - advisory, explanation, diagnosis, comparison, review findings, framework discussion, design recommendation,
   - no automatic `br`, TODO, or pack flow.
2. `artifact_flow`
   - research artifact, spec, task-pool, formal report, docs update, decision record,
   - `br` + pack + TODO required.
3. `execution_flow`
   - implementation, bug fix, refactor, protocol/script/code mutation,
   - `br` + TODO required.
4. `mixed`
   - begins as `answer_only`,
   - enters execution only after explicit mutation decision, existing approved task context, or user-confirmed scope.

Default rule:
1. Questions and consultations stay `answer_only` unless the request clearly asks for execution or artifact production.
2. Non-trivial does not automatically mean `br`/TODO.

## TODO Engagement Gate

TODO/`br` flow is mandatory only when at least one is true:
1. the request will mutate repository or runtime state,
2. the request must create or update a formal artifact,
3. the request is a multi-step execution flow that needs traceability,
4. a canonical pack/task handoff is part of the deliverable.

TODO/`br` flow is forbidden by default when all are true:
1. the request is `answer_only`,
2. no file/state mutation is requested,
3. no formal artifact is requested,
4. chat response is the intended deliverable.

## Subagent-First Orchestration

When the subagent system is active, the orchestrator is not the default primary analyst.

Mode-synced rule:
1. `native`
   - use internal subagents only,
   - internal subagents become the first analysis/review lane for eligible work.
2. `hybrid`
   - use external and internal providers under routing policy,
   - external-first read-only fanout is the default lane for eligible work.
3. `disabled`
   - do not use the subagent system,
   - orchestrator may analyze locally, but must still obey the log-read budget and bounded-read rules.

Eligible work:
1. non-trivial read-heavy analysis,
2. diagnosis,
3. research,
4. review,
5. verification,
6. framework/meta investigation unless blocked by tool/runtime constraints.

Required behavior:
1. delegate the blocking analysis question first when mode supports it,
2. separate authorship and verification lanes when route metadata requires it,
3. stop local deep reading once subagent evidence is sufficient for synthesis or mutation prep,
4. keep mutation ownership under the orchestrator unless bounded write scope is explicitly granted,
5. keep user-facing reporting ownership under the orchestrator; do not expose raw subagent reports unless the user explicitly asks to inspect them.

## Blocking-Question Dispatch Rule

Subagent prompts must be question-driven.

Each delegated lane should receive:
1. one blocking question,
2. bounded scope,
3. expected deliverable shape,
4. stop condition,
5. verification boundary.

Each delegated lane should return:
1. whether the question was answered,
2. the answer,
3. evidence references,
4. confidence,
5. recommended next action.

User-facing reporting rule:
1. Treat subagent returns as internal evidence for orchestrator synthesis by default.
2. Present the orchestrator's own integrated answer to the user, not the raw subagent report.
3. Quote or summarize subagent findings only as supporting evidence when necessary.
4. Surface raw subagent output only when the user explicitly requests it or when unresolved conflict remains decision-relevant.

## Log-Read Budget

Broad log reading is a last resort, not a default discovery pattern.

Hard rules:
1. do not run broad `rg` sweeps over `.vida/logs`, `.vida/state`, or `.beads` by default,
2. begin with exact-key lookup against a specific file whenever possible,
3. prefer one manifest/state file over many logs,
4. prefer short window reads (`sed -n start,end`) over large dumps,
5. do not emit raw JSONL dumps unless a documented escalation requires it,
6. if subagent findings already provide file/line evidence, do not repeat wide local log inspection without a new conflict or blocker.

Escalation to wider reads is allowed only when:
1. the blocking question remains unanswered after bounded evidence,
2. subagent findings conflict materially,
3. mutation requires verifying a specific runtime artifact,
4. final verification requires targeted proof.

## Boot Profiles

### Lean Boot

Use for routine execution and token-efficient continuation.

1. Read `AGENTS.md`.
2. Read this file.
3. Hydrate active context capsule when task context exists.
4. Read required thinking protocol sections:
   - `_vida/docs/thinking-protocol.md#section-algorithm-selector`
   - `_vida/docs/thinking-protocol.md#section-stc`
   - `_vida/docs/thinking-protocol.md#section-pr-cot`
   - `_vida/docs/thinking-protocol.md#section-mar`
   - `_vida/docs/thinking-protocol.md#section-5-solutions`
   - `_vida/docs/thinking-protocol.md#section-meta-analysis`
   - `_vida/docs/thinking-protocol.md#section-bug-reasoning`
   - `_vida/docs/thinking-protocol.md#section-web-search`
   - `_vida/docs/thinking-protocol.md#section-reasoning-modules`
5. Read `_vida/docs/web-validation-protocol.md`.
6. Read `_vida/docs/project-overlay-protocol.md`.
7. Read `vida.config.yaml` when present.
8. If `protocol_activation.agent_system=true`, read `_vida/docs/subagent-system-protocol.md`.
9. Read `_vida/docs/beads-protocol.md` only if the request is not `answer_only`.

### Standard Boot

Use when the request has moderate cross-protocol impact or uncertainty after Lean.

1. Execute Lean Boot.
2. Read `_vida/docs/todo-protocol.md` only if TODO engagement is required.
3. Read `_vida/docs/use-case-packs.md` only if a pack path is required.
4. Read `_vida/docs/implement-execution-protocol.md` only if implementation flow is in scope.

### Full Boot

Use when at least one is true:
1. architecture/topology refactor with non-local impact,
2. high-severity bug or unknown root cause,
3. cross-module integration,
4. security/auth/data-safety decision,
5. explicit meta-analysis request,
6. confidence after Standard Boot is below 80%.

1. Execute Standard Boot.
2. Read `_vida/docs/orchestration-protocol.md`.
3. Read `_vida/docs/pipelines.md`.

## Execution Rules

1. For `answer_only`, stay in chat/report mode and avoid task/TODO machinery.
2. For `artifact_flow`, use `br` + pack + TODO.
3. For `execution_flow`, use `br` + TODO and the canonical execution protocol.
4. For `mixed`, start in answer mode and transition only when execution is clearly required.
5. Route all framework-owned changes to `AGENTS.md` and `_vida/*`.
6. Use `br ready`, `br update`, and `br close` only when task flow is engaged.
7. During active task execution, use TODO blocks and keep near-term planning lean.

## Output Contract

For non-trivial orchestrator reports, default order:
1. `Problem Framing`
2. `Assumptions / Constraints`
3. `Agent Orchestration Summary`
4. `Integrated Analysis`
5. `Recommended Solution`
6. `Risks / Trade-offs`
7. `Next Actions`

## References

1. `_vida/docs/orchestration-protocol.md`
2. `_vida/docs/subagent-system-protocol.md`
3. `_vida/docs/subagents.md`
4. `_vida/docs/pipelines.md`
5. `_vida/docs/beads-protocol.md`
6. `_vida/docs/todo-protocol.md`
7. `_vida/docs/framework-map-protocol.md`
