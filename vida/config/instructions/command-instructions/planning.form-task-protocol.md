# Form-Task Protocol (FTP)

Purpose: user-confirmed bridge from approved specification to development execution.

Scope: `/vida-form-task` command mode; import mode from `work-pool-pack` and orchestration protocol; covers spec completion through development start.

Epic planning is built in; no separate epic command.

## Command Layer Mapping

| CLP layer | FTP gates |
| --- | --- |
| `CL1 Intake` | `FTP-0 Intake` + `FTP-0.5 Scope Contract` |
| `CL2 Reality And Inputs` | `FTP-1 Preflight` + `FTP-1.5 Change-Impact Reconciliation` |
| `CL3 Contract And Decisions` | `FTP-2 Option Synthesis` + `FTP-3 User Approval Questions` |
| `CL4 Materialization` | `FTP-4 Task Pool Build` + `FTP-5 Dependency Graph + Track Routing` |
| `CL5 Gates And Handoff` | `FTP-6 Readiness Verdict` + `FTP-7 Launch Gate` |

Canonical layer: `command-instructions/routing.command-layer-protocol`

## Core Contract

`/vida-form-task` must:

1. study approved spec inputs,
2. generate scope options,
3. ask structured approval questions,
4. create/update DB-backed `vida taskflow task` tasks/dependencies,
5. block implementation until explicit user confirmation,
6. hand off execution only to `command-instructions/execution.implement-execution-protocol`.
7. own epic-level boundary and ordering approval before task generation.

## Mandatory Inputs

1. Normalized `spec_intake` artifact when the upstream scope originated from mixed research, release signals, or unresolved user clarification.
1. Spec scope/decisions.
1.1. Equivalent bugfix paths may use approved `issue_contract` input from `runtime-instructions/bridge.issue-contract-protocol` instead of a longer SCP artifact when the scope is already bounded.
1.2. Non-equivalent issue/release paths must carry `spec_delta` reconciliation state before task materialization continues.
1.3. SCP-driven paths must carry a compact `draft_execution_spec` artifact before task-pool build.
2. SCP readiness/confidence evidence.
3. Relevant research references.
4. Feature checklist entries in scope.
5. Architecture decisions (`docs/decisions.md`).
6. WVP evidence for external assumptions (`runtime-instructions/work.web-validation-protocol`).
7. Existing scope boundaries from `docs/specs/*` when relevant.

## Epic Scope Contract (Built-in)

Before task-pool build, FTP must produce and approve scope contract: `IN/OUT` boundary, dependency ordering/phase fit, user approval.

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
   - TaskFlow micro-step created downstream under `runtime-instructions/work.taskflow-protocol`.

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

Decision outputs are mandatory TaskFlow planning inputs: `scope_boundary` -> grouping/exclusions; `delivery_cut` -> MVP-first or full-slice ordering; `dependency_strategy` -> sequential chain vs parallel-safe waves; `risk_policy` -> conservative/balanced/aggressive verification depth; `launch_decision` -> start dev now or revise; `draft_execution_spec_review` -> confirms materialization contract.

If any required decision is missing, task-pool build is blocked.
If the draft execution-spec is not approved, task-pool build is blocked.

## Planning-to-TaskFlow Mapping Contract

After cards are approved, FTP must produce execution-ready TaskFlow plan metadata.

## Delivery-Task Card Contract

Before ready queue entry, FTP must materialize a bounded delivery-task card.

Required fields: `task_id`, `parent_epic`, `milestone_id`, `goal`, `non_goals`, `scope_in`, `scope_out`, `owned_paths` or `owned_areas`, `acceptance_checks`, `validation_commands`, `definition_of_done`, `stop_rules`, `handoff_target`.

Readiness rule:

1. a task without a bounded delivery-task card is not ready,
2. if `definition_of_done`, `validation_commands`, or `owned_paths` are missing, the task must remain blocked,
3. if the task still requires the worker to infer scope from repository context, the task must remain blocked.

Per planned block, minimum fields: `block_id`, `goal`, `acceptance_check`, `depends_on`, `next_step` (`-` only for terminal block), `track_id` (`main` by default).

Routing policy: `dependency_strategy=sequential` -> single chain on `main`; `dependency_strategy=parallel-safe` -> non-overlapping tracks plus merge checkpoints.

Before execution handoff, run:

```bash
bash todo-plan-validate.sh <task_id> [--diff-aware]  # legacy script name
```

Use `--strict` when queue is ready for immediate autonomous execution.

## Gate Sequence

1. `FTP-0 Intake`:
   - gather context and select pack mode.
2. `FTP-0.5 Scope Contract`:
   - produce epic-level boundary/order contract and collect explicit approval.
3. `FTP-1 Preflight`:
   - verify spec readiness and blocker conditions.
4. `FTP-1.5 Change-Impact Reconciliation`:
   - if scope/AC/decision drift exists, route per `runtime-instructions/work.change-impact-reconciliation-protocol` before task generation.
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

`BLK_SPEC_MISSING`, `BLK_SCP_NOT_READY`, `BLK_API_REALITY_MISSING`, `BLK_WVP_EVIDENCE_MISSING`, `BLK_DECISION_CONFLICT`, `BLK_AC_MISSING`, `BLK_DEP_CYCLE`, `BLK_USER_LAUNCH_PENDING`, `BLK_SCOPE_CONTRACT_PENDING`, `BLK_CHANGE_IMPACT_PENDING`, `BLK_PLAN_DECISIONS_MISSING`, `BLK_PLAN_INTEGRITY_FAILED`, `BLK_TASK_TOO_LARGE`, `BLK_SCOPE_OVERLAP`, `BLK_VALIDATION_MISSING`, `BLK_DONE_RULE_MISSING`.

`BLK_CHANGE_IMPACT_PENDING` means approved spec/decisions changed after pool creation; resolution is owned by `runtime-instructions/work.change-impact-reconciliation-protocol`.

Task-pool rebuild obligations: run `reflection-pack` for artifact sync; run `/vida-spec review` for contract alignment; re-run `/vida-form-task` to rebuild queue/dependencies.

## Launch Rule (Hard)

`/vida-implement` may start only when all are true:

1. `FTP-6` verdict is `READY_TO_IMPLEMENT`.
2. No unresolved blocker codes.
3. User gave explicit launch confirmation in `FTP-7`.
4. every ready leaf task satisfies the delivery-task card contract.

Execution target: `/vida-implement` must run by `command-instructions/execution.implement-execution-protocol` only.

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
updated_at: 2026-07-03T14:18:00+03:00
changelog_ref: planning.form-task-protocol.changelog.jsonl
protocol_authoring_gate: enforced
protocol_compression_status: audit_passed
protocol_compression_algorithm: table-normalization+registry-compaction+gate-preserve-exact
protocol_compression_baseline_ref: 0d538023e:vida/config/instructions/command-instructions/planning.form-task-protocol.md
protocol_compression_audit_at: 2026-07-03T14:18:00+03:00
protocol_compression_before_tokens: 2524
protocol_compression_after_tokens: 2517
protocol_compression_content_sha256: d5f52b1a61b86d7314366e15e735ee4fceb96fe2d8dfff8553e908c73796dfbe
