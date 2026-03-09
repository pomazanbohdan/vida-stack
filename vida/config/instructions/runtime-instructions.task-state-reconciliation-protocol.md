# Task State Reconciliation Protocol (TSRP)

Purpose: provide one canonical reconciliation layer that determines whether a tracked task is truly active, stale, closure-ready, or internally inconsistent.

## Core Contract

1. `br` remains SSOT for lifecycle state.
2. TaskFlow remains SSOT for execution telemetry.
3. TSRP does not introduce a third task-state engine; it classifies consistency across existing artifacts.
4. Reconciliation is mandatory before closing stale framework tasks by judgment alone.

## Canonical Inputs

TSRP must read only canonical artifacts:

1. `.beads/issues.jsonl`
2. `taskflow-tool.sh ui-json <task_id>`
3. `boot-profile.sh verify-receipt <task_id>`
4. `beads-verify-log.sh --task <task_id>`
5. `run-graph.py status_payload(<task_id>)`

## Canonical Classifications

1. `active`
   - task has an active TaskFlow block or resumable run-graph node.
2. `blocked`
   - task has a blocked TaskFlow block.
3. `done_ready_to_close`
   - task is still `in_progress`, but TaskFlow has no active backlog and verification evidence is already valid.
4. `stale_in_progress`
   - task is `in_progress`, but no block is running and execution backlog requires explicit resume or reconciliation.
5. `open_but_satisfied`
   - task remains `open`, but TaskFlow/verification evidence indicates the scoped work is already satisfied.
6. `drift_detected`
   - `br`, TaskFlow, and verification artifacts disagree in a way that requires explicit reconciliation.
7. `invalid_state`
   - task is structurally contradictory, for example `closed` with active TaskFlow backlog.
8. `closed`
   - task is closed and no contradictory execution backlog remains.

## Required Outputs

TSRP status output must include:

1. `task_id`
2. `issue_status`
3. `classification`
4. `reasons`
5. `allowed_actions`
6. `boot_receipt_ok`
7. `verify_ok`
8. `todo_counts`
9. `current_block`
10. `run_graph.resume_hint`

## Allowed Actions

1. `continue_current_block`
2. `resume_next_block`
3. `close_now`
4. `reconcile_br`
5. `verify_then_close_or_manual_review`
6. `unblock_or_escalate`
7. `manual_review`
8. `start_or_reconcile`

Rule:

1. Closing a task from `stale_in_progress`, `drift_detected`, or `invalid_state` without first resolving the mismatch is forbidden.

## TaskFlow / Beads Integration

1. Use TSRP before final closure of stale-looking framework tasks and epics.
2. `quality-health-check` should surface TSRP classification when a `task_id` is supplied.
3. Session reflection may use TSRP to detect stale-open bookkeeping drift.
4. Parent epic closure should prefer TSRP-backed evidence for child task closure readiness.

## Canonical Helper

```bash
python3 docs/framework/history/_vida-source/scripts/task-state-reconcile.py status <task_id>
```

-----
artifact_path: config/runtime-instructions/task-state-reconciliation.protocol
artifact_type: runtime_instruction
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/runtime-instructions.task-state-reconciliation-protocol.md
created_at: 2026-03-08T02:15:22+02:00
updated_at: 2026-03-10T00:55:00+02:00
changelog_ref: runtime-instructions.task-state-reconciliation-protocol.changelog.jsonl
