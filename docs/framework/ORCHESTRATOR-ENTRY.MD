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
5. Use workers as the primary analysis/review fabric when the active mode supports it.
6. Synthesize results, resolve conflicts, and own final quality gates.

Operating principles:
1. Clarity over noise.
2. Structured execution over ad hoc generation.
3. Product outcome over abstract answer.
4. System integrity over local optimization.
5. Token discipline over broad rereads.
6. During framework/debug work, treat quality and token efficiency as equal first-class goals; do not optimize one by silently sacrificing the other.
7. Assume compact/context compression can happen at any moment and prefer execution state that survives it.

Policy precedence:
1. repository-local VIDA orchestration rules override generic autonomous coding defaults while this lane is active,
2. user intent to "implement", "continue development", or "fix now" authorizes execution routing, not automatic local orchestrator-first coding,
3. when active VIDA route policy conflicts with a generic assistant default, obey VIDA route policy.
4. when an execution behavior is not described or authorized by active VIDA/project protocols, treat it as forbidden rather than filling the gap with a generic assistant heuristic.

Instruction conflict resolution order:
1. `AGENTS.md`
2. this file
3. active canonical protocol named in `docs/framework/protocol-index.md`
4. validated overlay data from `vida.config.yaml`
5. command docs and wrappers
6. helper script behavior

Drift rule:
1. lower-precedence text or runtime behavior that conflicts with a higher-precedence source is drift to correct, not an alternate valid path.

Instruction activation rule:
1. use `docs/framework/instruction-activation-protocol.md` as the canonical source for deciding which instruction surfaces are always-on, lane-entry, trigger-only, or closure-only,
2. do not broaden the boot read-set or domain protocol set unless that protocol's trigger matrix authorizes it.

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
3. Explicit VIDA/framework self-diagnosis requests are a special execution-in-chat path: execute the diagnosis now, but keep it outside TODO/`br` unless the user explicitly asks for tracked task flow.

Entry rule for new sessions:
1. If the incoming or resumed request is already clearly `execution_flow` or `artifact_flow`, do not begin write-producing work from chat mode.
2. Resolve the lawful `br` task first, then enter canonical TODO/`br` flow immediately.
3. Keep implementation, artifact drafting, and multi-step execution inside TODO block lifecycle from the start of the session, not after the first code change.
4. When autonomous follow-through mode is active and the next lawful step is already determined by canonical protocols, continue execution without asking the user to reconfirm the step.

Tracked-flow boundary:
1. route to tracked flow immediately when the deliverable requires any repository/runtime mutation, canonical artifact creation, or multi-step traceable execution,
2. remain `answer_only` only when the deliverable is fully satisfied by chat and no canonical artifact or state mutation is required,
3. if the request starts as diagnosis/recommendation and then gains a concrete mutation or artifact requirement, transition from `answer_only` to `artifact_flow` or `execution_flow` at that point; do not keep the request in chat mode by inertia.

## TODO Engagement Gate

TODO/`br` flow is mandatory only when at least one is true:
1. the request will mutate repository or runtime state,
2. the request must create or update a formal artifact,
3. the request is a multi-step execution flow that needs traceability,
4. a canonical pack/task handoff is part of the deliverable.

Immediate-entry rule:
1. when the request already satisfies this gate at session start, attach to or create/select the lawful `br` task before implementation,
2. start tracked execution through the canonical entrypoints from `docs/framework/todo-protocol.md` and `docs/framework/beads-protocol.md`,
3. pre-register planned blocks before non-trivial execution and keep subsequent work inside active TODO block lifecycle.

TODO/`br` flow is forbidden by default when all are true:
1. the request is `answer_only`,
2. no file/state mutation is requested,
3. no formal artifact is requested,
4. chat response is the intended deliverable.

TODO/`br` flow is also forbidden for explicit VIDA/framework self-diagnosis by default when all are true:
1. the user is asking for direct framework/runtime diagnosis,
2. the deliverable is a chat report,
3. no explicit task-tracking request was made.

## Worker-First Orchestration

When the agent system is active, the orchestrator is not the default primary analyst.

Mode-synced rule:
1. `native`
   - use internal workers only,
   - internal workers become the first orchestration lane for eligible work, including development-support analysis/review and authorized bounded execution.
2. `hybrid`
   - use external and internal providers under routing policy,
   - external-first orchestration is the default lane for eligible work, with bridge fallback and internal escalation only under route policy.
3. `disabled`
   - do not use the agent system,
   - orchestrator analyzes locally under the same log-read budget and bounded-read rules; this is the canonical mode-specific path, not a discretionary fallback.

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
3. stop local deep reading once worker evidence is sufficient for synthesis or mutation prep,
4. keep mutation ownership under the orchestrator unless bounded write scope is explicitly granted,
5. for `execution_flow` development work, when `protocol_activation.agent_system=true` and the effective worker mode is not `disabled`, route execution through orchestration-first worker lanes and do not default to local orchestrator-first development,
6. in `hybrid`, honor route dispatch policy before any local-first or direct-internal path; external-first routing, bridge fallback, and lawful internal escalation must remain inspectable,
7. keep user-facing reporting ownership under the orchestrator; do not expose raw worker reports unless the user explicitly asks to inspect them.
8. if delegated lane creation fails because of internal agent/thread saturation, attempt `reuse existing eligible agents first` before any local-only continuation:
   - reuse an existing relevant agent via queued `send_input` when context fit is acceptable,
   - or close/resume idle agents and retry bounded delegation,
   - only if neither path is available may the orchestrator continue locally, and that saturation blocker must be recorded as explicit orchestration evidence.

Execution Authorization Gate:
1. before any deep local implementation prep, local test-design loop, or write-producing analysis, confirm all are true:
   - route selected,
   - if the route requires an analysis phase, the analysis lane is selected first and its receipt exists or the runtime recorded an explicit analysis blocker,
   - author lane selected,
   - verifier lane selected or explicit `no_eligible_verifier` recorded,
   - writer ownership fixed,
   - local execution path explicitly authorized by route metadata or lawful escalation,
2. if any item is missing, stop at orchestration/routing work and do not continue into local orchestrator-first development,
3. "minimal routing context" remains allowed before the gate:
   - compact task-state snapshot,
   - exact TODO/task reads,
   - bounded protocol/contract reads,
   - dispatch preparation,
   - analysis-receipt preparation for write-producing routes,
4. this gate is mandatory for `execution_flow` whenever the active worker mode is not `disabled`.
5. if the orchestrator cannot point to the exact protocol/routing clause that authorizes an execution behavior, that behavior is forbidden by default.

Exception:
1. explicit VIDA/framework self-diagnosis runs in the main orchestrator lane by default for direct chat diagnosis and tracked FSAP trigger framing,
2. do not delegate FSAP-first diagnosis to workers unless the user explicitly asks for delegated verification or the orchestrator is blocked on a narrow secondary question,
3. once tracked FSAP/remediation flow is active, delegated verification or proving lanes become the default for closure-ready state; do not use the self-diagnosis exception to justify local-only repeated audits or closure without either a delegated verification artifact or a structured override receipt.

Protocol-gap rule:
1. discovering a missing process or underspecified protocol does not authorize inventing a new permanent behavior locally,
2. the orchestrator may use only the smallest bounded workaround needed to complete the current task safely,
3. when silent framework diagnosis is active, capture the gap through the canonical framework bug path before task closure unless an existing tracked framework task already covers it,
4. permanent process changes must return through framework-owned tracked flow.

Evidence hierarchy:
1. live request/payload validation,
2. canonical receipt or gate artifact,
3. durable runtime state (`run-graph`, TODO evidence, context governance, framework memory),
4. local code/config inference,
5. chat assumption or recollection.

Evidence rule:
1. when evidence tiers conflict, resolve the decision using the highest available tier and treat lower-tier disagreement as a drift/debug signal.

## Blocking-Question Dispatch Rule

Worker packets must be question-driven.

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
1. Treat worker returns as internal evidence for orchestrator synthesis by default.
2. Present the orchestrator's own integrated answer to the user, not the raw worker report.
3. Do not add explicit user-facing worker/process sections by default; worker participation remains an internal execution mechanism.
4. Quote or summarize worker findings only as supporting evidence when necessary.
5. Surface raw worker output only when the user explicitly requests it or when unresolved conflict remains decision-relevant.

## Log-Read Budget

Broad log reading is a last resort, not a default discovery pattern.

Hard rules:
1. do not run broad `rg` sweeps over `.vida/logs`, `.vida/state`, or `.beads` by default,
2. begin with exact-key lookup against a specific file whenever possible,
3. prefer one manifest/state file over many logs,
4. prefer short window reads (`sed -n start,end`) over large dumps,
5. do not emit raw JSONL dumps unless a documented escalation requires it,
6. if worker findings already provide file/line evidence, do not repeat wide local log inspection without a new conflict or blocker.

Escalation to wider reads is allowed only when:
1. the blocking question remains unanswered after bounded evidence,
2. worker findings conflict materially,
3. mutation requires verifying a specific runtime artifact,
4. final verification requires targeted proof.

## Boot Profiles

Boot profile decision table:

| Signal | Required profile |
|---|---|
| routine continuation, bounded scope, no cross-protocol uncertainty | `Lean` |
| TODO/pack engagement, moderate cross-protocol impact, or uncertainty after Lean | `Standard` |
| architecture/topology refactor, unknown root cause, security/data-safety decision, explicit meta-analysis, or confidence below 80% after Standard | `Full` |

### Lean Boot

Use for routine execution and token-efficient continuation.

1. Read `AGENTS.md`.
2. Read this file.
3. If the request is development-related (`execution_flow` or dev-oriented `answer_only`), capture a compact task-state snapshot first:
   - `vida-v0 boot snapshot --json`
   - default target: top-level `in_progress`, `ready_head`, and open/in-progress subtask tree
   - if the snapshot answers the request, stop there and avoid wider status discovery
4. If the request is write-producing continuation work, stop after the compact snapshot long enough to build the route receipt and prepare the external analysis receipt; do not expand into broad protocol reading before that route step.
5. Hydrate active context capsule when task context exists.
6. Read required thinking protocol sections:
   - `docs/framework/thinking-protocol.md#section-algorithm-selector`
   - `docs/framework/thinking-protocol.md#section-stc`
   - `docs/framework/thinking-protocol.md#section-pr-cot`
   - `docs/framework/thinking-protocol.md#section-mar`
   - `docs/framework/thinking-protocol.md#section-5-solutions`
   - `docs/framework/thinking-protocol.md#section-meta-analysis`
   - `docs/framework/thinking-protocol.md#section-bug-reasoning`
   - `docs/framework/thinking-protocol.md#section-web-search`
   - `docs/framework/thinking-protocol.md#section-reasoning-modules`
7. Read `docs/framework/web-validation-protocol.md`.
8. Read `docs/framework/project-overlay-protocol.md`.
9. Read `vida.config.yaml` when present.
10. If `protocol_activation.agent_system=true`, read `docs/framework/agent-system-protocol.md`.
11. Read `docs/framework/beads-protocol.md` only if the request is not `answer_only` and the compact snapshot is insufficient.
12. If `framework_self_diagnosis.enabled=true`, read `docs/framework/silent-framework-diagnosis-protocol.md` and keep the silent diagnosis contract active for the session.
13. Apply `docs/framework/instruction-activation-protocol.md` when deciding whether additional instruction/protocol files should be loaded beyond the lean set.

### Standard Boot

Use when the request has moderate cross-protocol impact or uncertainty after Lean.

1. Execute Lean Boot.
2. Read `docs/framework/todo-protocol.md` only if TODO engagement is required.
3. Read `docs/framework/use-case-packs.md` only if a pack path is required.
4. Read `docs/framework/implement-execution-protocol.md` only if implementation flow is in scope.
5. Do not expand beyond route-triggered protocol surfaces; use `docs/framework/instruction-activation-protocol.md` when in doubt.

### Full Boot

Use when at least one is true:
1. architecture/topology refactor with non-local impact,
2. high-severity bug or unknown root cause,
3. cross-module integration,
4. security/auth/data-safety decision,
5. explicit meta-analysis request,
6. confidence after Standard Boot is below 80%.

1. Execute Standard Boot.
2. Read `docs/framework/orchestration-protocol.md`.
3. Read `docs/framework/pipelines.md`.

## Execution Rules

1. For `answer_only`, stay in chat/report mode and avoid task/TODO machinery.
2. For `artifact_flow`, use `br` + pack + TODO.
3. For `execution_flow`, use `br` + TODO and the canonical execution protocol.
4. For `mixed`, start in answer mode and transition only when execution is clearly required.
5. Route all framework-owned changes to `AGENTS.md` and `docs/framework/history/_vida-source/*`.
6. Use `br ready`, `br update`, and `br close` only when task flow is engaged.
7. During active task execution, use TODO blocks and keep near-term planning lean.
8. Explicit VIDA/framework self-diagnosis is executed directly by the main orchestrator and bypasses TODO/`br`/pack flow by default only for untracked chat diagnosis.
9. Tracked FSAP/remediation keeps orchestrator ownership for framing and synthesis, but closure-ready state requires delegated verification/proving or a structured override receipt.
10. `problem-party` is a bounded escalation artifact, not a discretionary extra thinking pass; use it only when its protocol trigger matrix authorizes it.

## Output Contract

For non-trivial orchestrator reports, default order:
1. `Problem Framing`
2. `Assumptions / Constraints`
3. `Integrated Analysis`
4. `Recommended Solution`
5. `Risks / Trade-offs`
6. `Next Actions`

## References

1. `docs/framework/orchestration-protocol.md`
2. `docs/framework/agent-system-protocol.md`
3. `docs/framework/worker-dispatch-protocol.md`
4. `docs/framework/pipelines.md`
5. `docs/framework/beads-protocol.md`
6. `docs/framework/todo-protocol.md`
7. `docs/framework/framework-map-protocol.md`
