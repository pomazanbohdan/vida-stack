# /vida-form-task — Spec-to-Dev Task Bridge

Purpose: single command that absorbs all work between completed specification and development execution.

## Protocol Layers

This command maps layers as:

1. `CL1 Intake` -> pool target resolution, scope draft, and epic-level entry context.
2. `CL2 Reality And Inputs` -> spec/SCP/WVP preflight plus blocker validation.
3. `CL3 Contract And Decisions` -> task-scope options, question cards, and planning contract.
4. `CL4 Materialization` -> `TaskFlow task-pool build plus dependency/track routing.
5. `CL5 Gates And Handoff` -> readiness verdict and explicit launch gate for `/vida-implement`.

Canonical source: `command-layer-protocol.md`

Handoff boundary:

1. `/vida-form-task` owns task-pool materialization and launch approval state.
2. `/vida-implement` starts only after `CL5` confirms launch.

## Runtime Position

1. `/vida-research` -> business evidence.
2. `/vida-spec` -> technical contract (SCP).
3. `/vida-form-task` -> task scope, dependencies, readiness, launch approval.
4. `/vida-implement` -> development execution (only after launch confirmation).

This command also absorbs epic-planning responsibilities (scope boundary, dependency ordering, and scope approval gate).

## Mode

1. Command mode:
   - `/vida-form-task <scope>`
2. Import mode:
   - called by orchestration protocol or `work-pool-pack` as a protocol module.

## Mandatory Reads Before Execution

1. `form-task-protocol.md`.
2. `spec-contract-protocol.md`.
3. `web-validation-protocol.md`.
4. `runtime.task-state-telemetry-protocol.md`.
5. `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`.
6. `implement-execution-protocol.md`.
7. `docs/decisions.md`.
8. `docs/feature-checklist.md`.
9. `docs/specs/*` (if scope touches existing boundaries).

## Inputs

1. User request and scope target.
2. Approved spec contract and SCP confidence.
3. Relevant research evidence and feature-list entries.
4. Current `TaskFlow` task context.
5. WVP evidence for external assumptions.
6. Existing epic boundaries/dependencies (if any) to avoid scope drift.

## FTP Pipeline (Canonical)

1. `FT-00 Intake`:
   - resolve task context and targeted scope.
   - align or create epic-level scope contract inside this flow.
2. `FT-01 Preflight`:
   - verify spec readiness and blocker conditions.
3. `FT-01.5 Change-Impact Reconciliation`:
   - absorb scope/AC/dependency drift before pool generation.
4. `FT-02 Option Synthesis`:
   - produce task-scope strategy options.
5. `FT-03 User Approval Questions`:
   - ask structured cards with recommended option and trade-offs.
6. `FT-03.5 Planning Contract`:
   - map approved answers into planning fields (`scope_boundary`, `delivery_cut`, `dependency_strategy`, `risk_policy`).
   - stop on unresolved conflicts.
7. `FT-04 Task Pool Build`:
   - create/update tasks in `br` with clear descriptions.
8. `FT-05 Dependency Graph + Track Routing`:
   - set dependencies, detect cycles, build sequential/parallel-safe routing.
   - materialize `next_step` chain and validate with `todo-plan-validate.sh`.
9. `FT-06 Readiness Verdict`:
   - classify tasks as `ready|blocked|deferred`.
10. `FT-07 Launch Gate`:
   - require explicit user confirmation before `/vida-implement`.

## Epic Scope Contract (Built-in)

Epic planning is internal to `/vida-form-task` and not a separate command.

1. Build scope boundary (`IN/OUT`) from spec + user constraints.
2. Validate dependency order and phase fit.
3. Require explicit scope approval before task pool materialization.
4. Record approved scope snapshot in task evidence/logs.

## Questioning Contract (Mandatory)

Use `form-task-protocol.md` question-card rules.

Required cards:

1. scope boundary,
2. delivery cut,
3. dependency strategy,
4. risk policy,
5. launch decision.

Rules:

1. one card at a time (or max two tightly coupled cards),
2. include trade-off sentence per option,
3. recommended option first,
4. run conflict check after each answer.

## Blockers (Hard Stop)

1. `BLK_SPEC_MISSING`
2. `BLK_SCP_NOT_READY`
3. `BLK_API_REALITY_MISSING`
4. `BLK_WVP_EVIDENCE_MISSING`
5. `BLK_DECISION_CONFLICT`
6. `BLK_AC_MISSING`
7. `BLK_DEP_CYCLE`
8. `BLK_CHANGE_IMPACT_PENDING`
9. `BLK_PLAN_DECISIONS_MISSING`
10. `BLK_PLAN_INTEGRITY_FAILED`

If any blocker remains unresolved, command outcome must be `BLOCKED` and `/vida-implement` must not start.

## Launch Rule (Hard)

`/vida-implement` start is allowed only when all are true:

1. readiness verdict is `READY_TO_IMPLEMENT`,
2. no unresolved blockers,
3. user explicitly confirms launch at `FT-07`.

If confirmation is not received, end state is `WAITING_USER_CONFIRMATION`.

## Output Schema

1. `Task Pool Summary`: total/ready/blocked/deferred.
2. `Ready Queue`: `id + short description + depends_on`.
3. `Blocked Queue`: `id + blocker + required action`.
4. `Launch Decision`: `approved|deferred|revise`.
5. `Next Action`: exact command to run next.
6. `Execution Plan Snapshot`: ordered block chain (`block_id`, `depends_on`, `next_step`, `track_id`).

## Minimal Execution Algorithm

```yaml
VIDA_FORM_TASK:
  step_1: load_spec_context
  step_2: run_preflight_gates
  step_3: synthesize_options
  step_4: run_user_question_cards
  step_5: map_answers_to_planning_contract
  step_6: build_or_update_br_tasks
  step_7: create_dependency_graph_and_track_routing
  step_8: validate_plan_integrity
  step_9: compute_readiness_verdict
  step_10: ask_launch_confirmation
  step_11:
    if launch_confirmed:
      next_command: "/vida-implement <ready_task_or_pool>"
    else:
      next_command: "revise scope/questions"
```

## Logging Contract

1. Run via TaskFlow blocks and pack events.
2. Capture decisions from question cards as evidence.
3. Record launch confirmation verbatim.
4. Finish with `reflect` and `verify` before reporting done.

## Related

1. `form-task-protocol.md`
2. `use-case-packs.md`
3. `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`
4. `vida/config/instructions/command-instructions/operator.vida-spec-guide.md`
5. `vida/config/instructions/command-instructions/operator.vida-implement-guide.md`

-----
artifact_path: config/command-instructions/vida.form-task
artifact_type: command_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/command-instructions/operator.vida-form-task-guide.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-11T13:25:46+02:00'
changelog_ref: operator.vida-form-task-guide.changelog.jsonl
