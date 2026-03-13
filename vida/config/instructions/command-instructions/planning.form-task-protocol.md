# Form-Task Protocol (FTP)

Purpose: define a single, user-confirmed bridge between approved specification and development execution.

Scope:

1. Command mode: `/vida-form-task`.
2. Import mode: callable from `work-pool-pack` and orchestration protocol.
3. Covers everything between spec completion and development start.

Epic planning is built into this protocol; no separate epic command is used.

## Command Layer Mapping

For `/vida-form-task`, FTP layers map to CLP as follows:

1. `CL1 Intake` -> `FTP-0 Intake` + `FTP-0.5 Scope Contract`
2. `CL2 Reality And Inputs` -> `FTP-1 Preflight` + `FTP-1.5 Change-Impact Reconciliation`
3. `CL3 Contract And Decisions` -> `FTP-2 Option Synthesis` + `FTP-3 User Approval Questions`
4. `CL4 Materialization` -> `FTP-4 Task Pool Build` + `FTP-5 Dependency Graph + Track Routing`
5. `CL5 Gates And Handoff` -> `FTP-6 Readiness Verdict` + `FTP-7 Launch Gate`

Canonical layer source: `vida/config/instructions/command-instructions/routing.command-layer-protocol.md`

## Core Contract

`/vida-form-task` must:

1. study approved spec inputs,
2. generate task-scope options,
3. ask structured approval questions,
4. create/update tasks and dependencies in the DB-backed `taskflow-v0 task` surface,
5. block implementation start until explicit user confirmation,
6. hand off execution only to `vida/config/instructions/command-instructions/execution.implement-execution-protocol.md`.
7. own epic-level scope boundary and ordering approval before task generation.

## Mandatory Inputs

1. Normalized `spec_intake` artifact when the upstream scope originated from mixed research, release signals, or unresolved user clarification.
1. Spec scope and decisions.
1.1. Equivalent bugfix paths may use approved `issue_contract` input from `vida/config/instructions/runtime-instructions/bridge.issue-contract-protocol.md` instead of a longer SCP artifact when the scope is already bounded.
1.2. Non-equivalent issue/release paths must carry `spec_delta` reconciliation state before task materialization continues.
1.3. SCP-driven paths must carry a compact `draft_execution_spec` artifact before task-pool build.
2. SCP readiness/confidence evidence.
3. Relevant research references.
4. Feature checklist entries in scope.
5. Architecture decisions (`docs/decisions.md`).
6. WVP evidence for external assumptions (`vida/config/instructions/runtime-instructions/work.web-validation-protocol.md`).
7. Existing scope boundaries from `docs/specs/*` when relevant.

## Epic Scope Contract (Built-in)

Before task-pool build, FTP must produce and approve scope contract:

1. `IN/OUT` scope boundary,
2. dependency ordering and phase fit,
3. explicit user approval for scope contract.

No task materialization in the DB-backed task surface before scope contract approval.
No task materialization in the DB-backed task surface from raw research/release/chat text without either normalized `spec_intake`, approved SCP artifact, or approved `issue_contract`.

## Hierarchy And Granularity Contract

FTP must decompose approved scope top-down before implementation handoff:

1. `epic`
   - user-visible outcome boundary,
   - approved by scope contract,
   - never sent directly to implementation.
2. `milestone`
   - one independently verifiable delivery slice that should complete in one implementation/review cycle.
3. `delivery_task`
   - one single-owner development contract suitable for one author lane plus downstream coach/verifier lanes.
4. `execution_block`
   - TaskFlow micro-step created downstream under `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`.

Granularity rules:

1. split until each `delivery_task` has one dominant goal, explicit non-goals, and one unambiguous done rule,
2. if a candidate task still spans multiple mutable contracts or mixed frontend/backend/schema/infra ownership without explicit isolation, split again or block it,
3. task pools may group several sibling `delivery_task` items under one `milestone`, but launch readiness is judged per leaf `delivery_task`, not per epic,
4. the review queue may batch several merge-ready leaf tasks only when they belong to the same milestone and keep disjoint writable scope.

## Question Card Protocol (Mandatory)

Use card-by-card approval with options and one recommended choice.

Card categories:

1. `Q1 Scope Boundary`: strict vs expanded scope.
2. `Q2 Delivery Cut`: MVP-only vs full slice.
3. `Q3 Dependency Strategy`: strict chain vs parallel-safe waves.
4. `Q4 Risk Policy`: conservative vs balanced vs aggressive.
5. `Q5 Launch Decision`: start `/vida-implement` now vs revise pool.
6. `Q6 Draft Execution-Spec Review`: approve the bounded execution contract vs revise assumptions/scope first.

Card rules:

1. One card at a time (or max 2 tightly coupled cards).
2. Each card includes trade-off note per option.
3. Recommendation is first option.
4. If user picks `Other`, capture exact text and re-check conflicts.

Decision outputs from cards are mandatory inputs for TaskFlow planning:

1. `scope_boundary` -> step grouping and exclusions.
2. `delivery_cut` -> MVP-first or full-slice queue ordering.
3. `dependency_strategy` -> sequential chain vs parallel-safe waves.
4. `risk_policy` -> conservative/balanced/aggressive verification depth.
5. `launch_decision` -> start dev now or keep in revision loop.
6. `draft_execution_spec_review` -> confirms the contract that task materialization may expand from.

If any required decision is missing, task-pool build is blocked.
If the draft execution-spec is not approved, task-pool build is blocked.

## Planning-to-TaskFlow Mapping Contract

After cards are approved, FTP must produce execution-ready TaskFlow plan metadata.

## Delivery-Task Card Contract

Before a task may enter the ready queue, FTP must materialize a bounded delivery-task card.

Required fields:

1. `task_id`
2. `parent_epic`
3. `milestone_id`
4. `goal`
5. `non_goals`
6. `scope_in`
7. `scope_out`
8. `owned_paths` or `owned_areas`
9. `acceptance_checks`
10. `validation_commands`
11. `definition_of_done`
12. `stop_rules`
13. `handoff_target`

Readiness rule:

1. a task without a bounded delivery-task card is not ready,
2. if `definition_of_done`, `validation_commands`, or `owned_paths` are missing, the task must remain blocked,
3. if the task still requires the worker to infer scope from repository context, the task must remain blocked.

Per planned block, minimum fields:

1. `block_id`.
2. `goal`.
3. `acceptance_check`.
4. `depends_on`.
5. `next_step` (`-` only for terminal block).
6. `track_id` (`main` by default).

Routing policy:

1. `dependency_strategy=sequential` -> single chain on `main` track.
2. `dependency_strategy=parallel-safe` -> split into non-overlapping tracks and add merge checkpoints.

Before execution handoff, run:

```bash
bash todo-plan-validate.sh <task_id> [--diff-aware]
```

`--strict` should be used when queue is ready for immediate autonomous execution.

## Gate Sequence

1. `FTP-0 Intake`:
   - gather context and select pack mode.
2. `FTP-0.5 Scope Contract`:
   - produce epic-level boundary/order contract and collect explicit approval.
3. `FTP-1 Preflight`:
   - verify spec readiness and blocker conditions.
4. `FTP-1.5 Change-Impact Reconciliation`:
   - if scope/AC/decision drift exists, route per `vida/config/instructions/runtime-instructions/work.change-impact-reconciliation-protocol.md` before task generation.
5. `FTP-2 Option Synthesis`:
   - build alternative task-scope strategies.
6. `FTP-3 User Approval Questions`:
   - run question cards, review the draft execution-spec, and resolve conflicts.
7. `FTP-4 Task Pool Build`:
   - create/update `TaskFlow tasks and metadata as bounded delivery-task cards.
8. `FTP-5 Dependency Graph + Track Routing`:
   - set `depends_on`, detect cycles;
   - decide sequential vs parallel-safe track routing;
   - materialize `next_step` chain per track;
   - declare review-pool checkpoints for merge-ready sibling tasks when lawful.
9. `FTP-6 Readiness Verdict`:
   - classify leaf tasks: `ready|blocked|deferred`.
10. `FTP-7 Launch Gate`:
   - explicit user confirmation required to start `/vida-implement`.

## Blocker Codes

1. `BLK_SPEC_MISSING`.
2. `BLK_SCP_NOT_READY`.
3. `BLK_API_REALITY_MISSING`.
4. `BLK_WVP_EVIDENCE_MISSING`.
5. `BLK_DECISION_CONFLICT`.
6. `BLK_AC_MISSING`.
7. `BLK_DEP_CYCLE`.
8. `BLK_USER_LAUNCH_PENDING`.
9. `BLK_SCOPE_CONTRACT_PENDING`.
10. `BLK_CHANGE_IMPACT_PENDING`.
11. `BLK_PLAN_DECISIONS_MISSING`.
12. `BLK_PLAN_INTEGRITY_FAILED`.
13. `BLK_TASK_TOO_LARGE`.
14. `BLK_SCOPE_OVERLAP`.
15. `BLK_VALIDATION_MISSING`.
16. `BLK_DONE_RULE_MISSING`.

`BLK_CHANGE_IMPACT_PENDING` is raised when approved spec/decisions changed after pool creation.
Resolution route is owned by `vida/config/instructions/runtime-instructions/work.change-impact-reconciliation-protocol.md`.

Task-pool rebuild obligations for this owner:

1. run `reflection-pack` for artifact sync,
2. run `/vida-spec review` for contract alignment,
3. re-run `/vida-form-task` to rebuild queue/dependencies.

## Launch Rule (Hard)

`/vida-implement` may start only when all are true:

1. `FTP-6` verdict is `READY_TO_IMPLEMENT`.
2. No unresolved blocker codes.
3. User gave explicit launch confirmation in `FTP-7`.
4. every ready leaf task satisfies the delivery-task card contract.

Execution target:

1. `/vida-implement` must run by `vida/config/instructions/command-instructions/execution.implement-execution-protocol.md` only.

Without confirmation, `/vida-form-task` ends with `WAITING_USER_CONFIRMATION` and no dev start.

## Output Schema

1. `Task Pool Summary`:
   - total, ready, blocked, deferred.
2. `Ready Queue`:
   - `id + short description + dependency state`.
3. `Blocked Queue`:
   - `id + blocker_code + required action`.
4. `Launch Decision`:
   - `approved|deferred|revise`.
5. `Next Action`:
   - exact next command (`/vida-implement ...` or revision path).
6. `Review Pools`:
   - `milestone_id + merge-ready task ids + review gate`.

## Logging Requirements

1. Log each FTP gate as TaskFlow block.
2. Store question decisions in execution artifacts/evidence.
3. Record launch confirmation text explicitly.
4. Record epic -> milestone -> leaf-task lineage and any review-pool checkpoints.
5. Run `reflect` + `verify` before reporting completion.

-----
artifact_path: config/command-instructions/form-task.protocol
artifact_type: command_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/command-instructions/planning.form-task-protocol.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-13T06:52:32+02:00'
changelog_ref: planning.form-task-protocol.changelog.jsonl
