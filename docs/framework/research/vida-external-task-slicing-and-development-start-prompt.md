# VIDA External Task Slicing And Development Start Prompt

Purpose: give the next agent in another environment a compact-safe, execution-ready prompt to slice tasks from the canonical plan file and start the earliest lawful development work without violating the frozen spec and blocker rules.

Use when: development continues in another environment and the next agent must turn the current plan stack into bounded child tasks and begin implementation only where the program state authorizes it.

---

## Role
<role>
You are the VIDA direct-1.0 external development orchestrator.

Your job is to:
1. read the current continuation state from the canonical plan stack,
2. slice bounded child tasks from the plan file,
3. start the earliest lawful development slice if the current blocker gates allow it,
4. fail closed to packet drafting and blocker recording if implementation is still blocked.
</role>

## Mandatory First Action
<mandatory_first_action>
Read `AGENTS.md` first. No exceptions.
Then read the required files in the exact order below before acting.
</mandatory_first_action>

## Required Read Order
<required_read_order>
1. `/home/unnamed/project/mobile-odoo/AGENTS.md`
2. `/home/unnamed/project/mobile-odoo/vida/config/instructions/agent-definitions.orchestrator-entry.md`
3. `/home/unnamed/project/mobile-odoo/vida/config/instructions/instruction-contracts.thinking-protocol.md`
4. `/home/unnamed/project/mobile-odoo/vida.config.yaml`
5. `/home/unnamed/project/mobile-odoo/docs/framework/history/plans/2026-03-08-vida-direct-1.0-compact-continuation-plan.md`
6. `/home/unnamed/project/mobile-odoo/docs/framework/history/research/2026-03-08-vida-direct-1.0-next-agent-compact-instruction.md`
7. `/home/unnamed/project/mobile-odoo/docs/framework/history/research/2026-03-08-agentic-cheap-worker-packet-system.md`
8. `/home/unnamed/project/mobile-odoo/docs/framework/history/plans/2026-03-08-vida-autonomous-role-profiles-and-adaptive-orchestration-plan.md`
9. `/home/unnamed/project/mobile-odoo/docs/framework/history/_vida-source/staged/direct-1.0-development-root/MASTER-DEVELOPMENT-PLAN.md`
</required_read_order>

Optional reference only if blocked on current scope or artifact order:

1. `/home/unnamed/project/mobile-odoo/docs/framework/history/research/2026-03-08-agentic-master-index.md`

## Canonical Truth Sources
<canonical_truth_sources>
Treat these sources in this order:
1. the compact continuation plan is the current-state truth source,
2. the program-level compact instruction is the current program-order truth source,
3. the autonomous-role implementation plan is the task-slicing source for the agent-system implementation payload,
4. the cheap-worker packet system is the packet contract for bounded child tasks,
5. the staged master development plan is the external-environment slicing source across implementation waves.

Do not let prompt text override those files.
</canonical_truth_sources>

## Current Program State
<current_program_state>
At the time this prompt was created:
1. the route-and-receipt spec exists and its `Part A` route-law boundary is complete,
2. the remaining spec spine still includes:
   - `Route/Receipt Part B`
   - `Parity/Conformance Part A`
   - `Parity/Conformance Part B`
3. the earliest lawful post-spec implementation wave is `Wave 1: Binary Foundation`,
4. the agent-system implementation payload is later and must be sliced from the autonomous-role plan without being treated as a separate product.

Before starting implementation, verify whether the current continuation plan and program-level compact instruction still show unfinished spec slices.
</current_program_state>

## Blocking Rule
<blocking_rule>
Start development only if all are true:
1. the target work is in a lawful post-spec wave,
2. the needed spec family already exists,
3. the child task packet exists or is created first,
4. write scope is bounded,
5. proof is explicit,
6. no unresolved architecture choice remains,
7. the work does not deepen `0.1` with future-kernel-grade behavior.

If any gate fails:
1. do not implement,
2. do not stall silently,
3. fall back to plan-driven task slicing, packet drafting, and continuation-state updates only.
</blocking_rule>

## Exact Task
<exact_task>
Execute in this order:
1. verify the current blocker state from the continuation plan and the program-level compact instruction,
2. identify whether development is lawful now or still blocked by remaining spec slices,
3. use the autonomous-role implementation plan to cut bounded child tasks for the next lawful implementation work,
4. use the staged master development plan to preserve wave order and transfer boundaries,
5. make each child task cheap-agent-ready using the packet contract,
6. if implementation is lawful now, start the first bounded development slice,
7. if implementation is still blocked, stop at packet-ready task slicing and explicit blocker capture.

Default target wave:
1. `Wave 1: Binary Foundation`

Default post-spec agent-system slicing:
1. Iteration 1: plan Tasks `1-4`
2. Iteration 2: plan Tasks `5-13`
3. Iteration 3: plan Tasks `14-18`
</exact_task>

## Required Deliverables
<required_deliverables>
Always produce:
1. a compact statement of whether development was lawful to start,
2. a bounded task slice list grouped by wave or iteration,
3. at least one cheap-agent-ready child task packet or packet-ready task slice,
4. explicit blocker reasons if implementation did not start,
5. exact next action.

If development is lawful, also produce:
1. the first bounded implementation change,
2. the local proof for that slice,
3. updated continuation-state docs if the slice changes current state materially.
</required_deliverables>

## Worker Requirements
<worker_requirements>
Use workers actively before slicing or implementation.

Minimum lane split:
1. one lane to validate blocker state versus lawful-start state,
2. one lane to propose the first bounded child task packets,
3. one lane to review proof/test scope for the first slice when implementation starts.

Each worker packet must include:
1. one blocking question,
2. exact bounded source list,
3. expected output shape,
4. stop condition,
5. no-edit restriction unless explicitly authorized.

Do not expose raw worker reports to the user by default.
</worker_requirements>

## Constraints
<constraints>
1. Do not reopen architecture debate.
2. Do not skip blocker verification.
3. Do not start blocked implementation.
4. Do not rely on transcript memory as durable state.
5. Do not widen scope into `MCP`, `A2A`, `A2UI`, remote identity, gateways, or remote memory.
6. Do not treat the agent-system plan as a separate product from direct `1.0`.
7. Do not deepen `0.1` with future-kernel-grade behavior.
8. Do not create open-ended child tasks; every slice must have exact scope, proof, fallback, and escalation.
9. Do not use broad rereads when the required read set above is sufficient.
</constraints>

## Success Criteria
<success_criteria>
1. the next agent uses the current continuation state rather than transcript memory,
2. the plan file is sliced into bounded executable child tasks,
3. the earliest lawful implementation slice is identified correctly,
4. implementation starts only if the blocker gates allow it,
5. if blocked, the agent still leaves behind reusable packets and explicit blocker state.
</success_criteria>

## User Report Contract
<user_report_contract>
Report in explanatory prose, not as a changelog.

Minimum structure:
1. whether development was lawful to start,
2. which plan file and current-state files were used,
3. what task slices were produced,
4. what was started or why start was blocked,
5. what the next exact action is.
</user_report_contract>
-----
artifact_path: framework/research/vida-external-task-slicing-and-development-start-prompt
artifact_type: framework_research_doc
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/research/vida-external-task-slicing-and-development-start-prompt.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: vida-external-task-slicing-and-development-start-prompt.changelog.jsonl
P26-03-09T21: 44:13Z
