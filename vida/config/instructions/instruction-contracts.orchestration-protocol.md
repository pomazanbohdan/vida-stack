# Orchestration Protocol (OP)

Purpose: define how the top-level orchestrator turns a non-trivial request into a delivery-ready result.

## Core Contract

1. Orchestration is protocol-driven and uses `br` + packs only when request intent requires task execution or artifact production.
2. The orchestrator owns problem framing, lens selection, workstream decomposition, agent routing, synthesis, and the final quality gate.
3. Packs are the runtime route; orchestration lenses are the reasoning posture. A lens never replaces a pack.
4. Task state lives only in `br`; execution visibility lives only in TaskFlow blocks when TaskFlow is engaged.
5. No interactive menu dependencies.

## Hard Runtime Law

1. Mandatory routing and verification requirements must be expressed as executable runtime behavior, not advisory phrasing.
2. If route metadata declares `external_first_required`, `dispatch_required`, `fanout_min_results`, or `independent_verification_required`, the orchestrator must treat violations as invalid execution states.
3. The orchestrator must not silently substitute local/manual analysis for a declared routed dispatch path.
4. When protocol text describes options, it must also define the exact condition that selects each option.

## Scope

1. Entry behavior is executed via TaskFlow blocks and, where still needed during migration, legacy wrapper surfaces such as `vida-pack-helper.sh`.
2. This protocol covers request interpretation above command-level execution details.
3. Detailed task-state, TaskFlow, and worker-routing rules remain in their canonical protocols.

## Inputs

1. User request text.
2. Optional explicit `task_id`.
3. Existing task, decision, or spec context when present.

## Request Intent Classes

Classify request intent before task resolution:

1. `answer_only`
   - explanation, diagnosis, comparison, review findings, framework discussion, architecture recommendation,
   - no automatic TaskFlow or pack flow.
2. `artifact_flow`
   - research artifact, spec, task-pool, formal report, docs update, decision record,
   - `TaskFlow` + pack required.
3. `execution_flow`
   - implementation, bug fix, refactor, protocol/script/code mutation,
   - `TaskFlow` required.
4. `mixed`
   - starts as `answer_only`,
   - enters TaskFlow only after explicit mutation decision, approved task context, or user-confirmed execution scope.

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
11. `problem_party`

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
3. For `execution_flow` and development-related `answer_only`, capture a compact boot snapshot first:
   - `taskflow-v0 boot snapshot --json`
   - prefer this snapshot over broad `br`/repo discovery for task-state questions and boot-time context
4. Apply TaskFlow engagement gate:
   - `answer_only` -> stay outside `TaskFlow/pack flow,
   - `artifact_flow` and `execution_flow` -> TaskFlow required,
   - `mixed` -> start answer path first and enter TaskFlow only when execution becomes required.
5. Resolve active task only when TaskFlow is required:
   - if `task_id` is provided, use it;
   - else prefer active `in_progress` task from `br`;
   - else pick the highest-priority `ready` task, breaking ties by most recently updated work first, or create one when the request is net-new framework/project work.
6. Detect pack only when TaskFlow is required:
   - during migration, `bash vida-pack-helper.sh detect "<request>"` remains allowed as a legacy helper
7. Select execution mode/profile:
   - task execution mode via `task-execution-mode.sh`,
   - boot profile via `taskflow-v0 boot ...`,
   - META / FSAP / SCP / WVP when triggers fire.
8. Select orchestration hierarchy:
   - default to free external read-only fanout for eligible non-trivial analysis/research/review/verification work,
   - use the configured bridge fallback worker before internal escalation,
   - reserve internal workers for senior arbitration, architecture-heavy synthesis, and mutation-owning execution.
   - if the selected route declares hard requirements, block invalid alternate paths instead of degrading to manual fallback.
9. For `execution_flow` when `protocol_activation.agent_system=true` and effective mode is not `disabled`, resolve the orchestration-first execution path before local implementation:
   - decompose the task into worker lanes first whenever route policy requires orchestration,
   - forbid local orchestrator-first development as the default path,
   - keep the orchestrator as the single writer unless bounded write scope is explicitly granted,
   - preserve mode distinction: `native` = internal-first authorized lanes, `hybrid` = external-first routing with bridge fallback and lawful internal escalation.
   - treat generic autonomous coding defaults as subordinate to VIDA route law while this protocol is active.
   - treat protocol-described behavior as an allowlist; undocumented execution shortcuts, fallbacks, or mutation paths are forbidden by default.
9.1. Run an Execution Authorization Gate before any deep local implementation prep or local test-first loop:
   - selected route receipt exists,
   - author lane exists,
   - verifier lane exists or explicit `no_eligible_verifier` blocker/receipt exists,
   - writer ownership is explicit,
   - local write path is route-authorized or escalation-authorized,
   - if any item fails, remain in orchestration mode and do not continue into local-first development.
10. Decompose the work into layers:
   - analysis,
   - design/contract,
   - implementation/materialization,
   - validation,
   - governance/documentation,
   - delivery/handoff.
11. Decide dependency order:
   - parallel only for independent read-only or isolated-scope steps,
   - otherwise keep a single writer lane on `track_id=main`.
11.1. For epics, waves, and task pools with multiple subtasks, build a top-level dependency graph before selecting the first writer task:
   - classify `ready`, `blocked`, `soft-blocked`, and `parallel_read_only`,
   - identify single-writer serialization boundaries,
   - treat missing dependency-graph analysis as a blocking orchestration gap for multi-task execution.
12. For `execution_flow` under active worker mode, treat orchestration-first dispatch as the default execution posture rather than optional expert injection:
   - dispatch analysis, review, verification, and other eligible pre-write work through the routing system first,
   - use additional expert lanes when domain specialization, conflict arbitration, or risk requires it,
   - do not bypass the routing layer into local-first development unless the active mode is `disabled` or route policy explicitly authorizes the exception.
   - user phrasing like "continue development", "fix it", or "implement now" opens execution routing, not an implicit waiver of the gate above.
12.1. For eligible non-trivial work, prefer separate cli-worker lanes for authorship, coach review, and verification when route policy requires them:
   - one cli worker or cli-worker ensemble produces the primary analysis/recommendation,
   - for write-producing routes with `coach_required=yes`, a coach lane reviews the produced implementation and may return it for rework before the final verifier runs,
   - another eligible cli worker (or verification ensemble) validates it independently when route policy requires it,
   - the orchestrator owns synthesis, escalation, and mutation-only control.
12.2. Lawful local-orchestrator mutation is an escalation path, not a default:
   - allow only when route metadata authorizes local execution, the active mode is `disabled`, or the runtime records a concise escalation receipt,
   - acceptable escalation reasons include `no_eligible_author`, `worker_exhausted`, `bridge_failure_with_time_critical_fix`, and `tiny_bounded_patch_after_completed_evidence_cycle`,
   - absent such a receipt, local orchestrator-first development is protocol-invalid.
12.3. If an action cannot be traced to an explicit protocol clause, route receipt field, or escalation receipt, the orchestrator must not perform it.
13. Start pack session only when TaskFlow is required:
   - during migration, `bash vida-pack-helper.sh start <task_id> <pack_id> "<goal>" [constraints]`.
   - optional shortcut for standard non-dev flows:
     `bash nondev-pack-init.sh <task_id> <pack_id> "<goal>" [constraints]`.
14. Pre-register execution blocks only when TaskFlow is required:
   - during migration, `bash vida-pack-helper.sh scaffold <task_id> <pack_id>`.
15. Execute via TaskFlow lifecycle only when TaskFlow is engaged:
   - `block-plan -> block-start -> block-end -> reflect -> verify`.
15.1. Treat compact/context compression as an always-possible interruption:
   - persist active task assumptions in TaskFlow evidence or context capsules before long dispatches, risky transitions, or session handoff,
   - prefer compact-resumable artifacts over chat-only state.
15.2. When the blocking problem is materially conflict-heavy but still bounded, prefer `vida/config/instructions/runtime-instructions.problem-party-protocol.md` over ad hoc open-ended debate:
   - start with the `small` board,
   - escalate to `large` only when the small board remains unresolved,
   - require a structured decision artifact before resuming the main flow.
16. Synthesize results:
   - integrate business, product, architecture, implementation, and verification outputs,
   - resolve conflicts before reporting,
   - convert the result into an execution-ready artifact when appropriate.
17. End pack session only when TaskFlow was engaged:
   - during migration, `bash vida-pack-helper.sh end <task_id> <pack_id> <done|partial|failed> "<summary>" [next_step]`.

## Dynamic Expert Injection

When dispatching additional agents, define all of:

1. domain role,
2. bounded scope,
3. explicit context,
4. expected output format,
5. verification boundary,
6. merge owner (always the orchestrator).

Routing rule:

1. Use `vida/config/instructions/instruction-contracts.agent-system-protocol.md` + project overlay for worker choice.
2. Use `vida/config/instructions/instruction-contracts.worker-dispatch-protocol.md` for dispatch contract.
3. For eligible non-trivial read-heavy work, prefer worker-first execution whenever the active worker mode is not `disabled`.
4. For `execution_flow` under active worker mode, orchestration-first routing is mandatory; do not treat workers as optional helpers around an otherwise local-first development path.
5. In `hybrid`, prefer external free fanout first, then the configured bridge fallback, then internal senior escalation only when route policy or evidence requires it.
6. In `native`, prefer internal workers as the first analysis/review lane and the first authorized development-support orchestration lane.
7. In `disabled`, keep analysis local and obey bounded-read policy.
8. Keep writer ownership singular under the orchestrator even when read-only fanout is active.
9. Prefer independent verification by a different cli worker when route metadata marks independent verification as required and a distinct eligible verifier exists.

## Conflict Resolution

When agent or domain outputs disagree:

1. Locate whether the conflict is in assumptions, goals, constraints, evidence, or domain interpretation.
2. Prefer canonical evidence and protocol ownership over eloquence or volume.
3. Synthesize one decision or surface a bounded user decision when equivalent paths remain.
4. Do not forward contradictory raw fragments as if they were a final answer.

## User-Facing Reporting

When workers participate in the flow:

1. treat worker outputs as internal evidence unless the user explicitly asks to inspect them,
2. present one orchestrator-synthesized answer in chat,
3. do not add explicit visual worker/process sections in the default user-facing report,
4. do not stream or paste raw worker reports into the final user response by default,
5. reference worker findings only through synthesized conclusions, evidence refs, or clearly marked supporting summaries,
6. expose raw worker disagreement only when it remains decision-relevant after synthesis.

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

0. answer-only advisory/diagnosis/review request -> no pack, no automatic `br`, no TaskFlow.
1. research request -> `research-pack`.
2. spec creation/update -> `spec-pack`.
3. scope/task pool formation -> `work-pool-pack` (`/vida-form-task`).
4. implementation -> `dev-pack` (`/vida-implement`).
5. bug investigation/fix -> `bug-pool-pack` (`/vida-bug-fix`).
6. docs/protocol synchronization or change-impact reconciliation -> `reflection-pack`.
7. explicit VIDA/framework self-analysis request -> run `vida/config/instructions/diagnostic-instructions.framework-self-analysis-protocol.md` directly in orchestrator chat mode by default; use `reflection-pack` only when the user explicitly requests tracked TaskFlow or a formal artifact, and require delegated verification or a structured override receipt before tracked closure-ready state.

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
   - `Integrated Analysis`
   - `Recommended Solution`
   - `Risks / Trade-offs`
   - `Next Actions`
6. Completion state and next step.
7. When workers contributed, report the orchestrator's synthesized result, not raw worker text, unless the user explicitly asks for the raw output.

## Constraints

1. Do not mutate task state outside `br`.
2. Do not execute task-flow work outside active TaskFlow block lifecycle.
3. Do not engage `TaskFlow/pack flow for `answer_only` requests by default.
4. Do not route through non-canonical command paths.
5. Do not use multiple writer lanes without explicit scope isolation.
6. Do not replace synthesis with unintegrated agent fragments.
7. Do not expose raw worker reports as the default user-facing deliverable.
8. Do not route explicit VIDA/framework self-analysis through TaskFlow/pack flow unless the user explicitly asks for tracked execution.
9. Do not use the self-diagnosis exception to justify local-only closure of tracked FSAP/remediation work; tracked closure-ready state requires delegated verification/proving or a structured override receipt.
9. Do not start dev-related boot with broad repo or `br` sweeps when the compact boot snapshot is sufficient.
10. Do not bypass a route-marked hard requirement with local/manual fallback unless the runtime also records a blocker or lawful escalation receipt.

## Related

1. `vida/config/instructions/command-instructions.use-case-packs.md`
2. `vida/config/instructions/runtime-instructions.taskflow-protocol.md`
3. `vida/config/instructions/runtime-instructions.beads-protocol.md`
4. legacy `vida-pack-helper.sh` during migration only
5. `vida/config/instructions/instruction-contracts.agent-system-protocol.md`
6. `vida/config/instructions/instruction-contracts.worker-dispatch-protocol.md`

-----
artifact_path: config/instructions/instruction-contracts/orchestration.protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts.orchestration-protocol.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-10T03:06:28+02:00'
changelog_ref: instruction-contracts.orchestration-protocol.changelog.jsonl
