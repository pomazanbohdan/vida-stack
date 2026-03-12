# Human Approval Protocol (HAP)

Purpose: define one canonical approval-receipt contract for review states that require policy, senior, or human approval before closure-ready state.

## Core Contract

1. Review states `policy_gate_required`, `senior_review_required`, and `human_gate_required` are blocking governance states, not advisory labels.
2. Closure-ready state is invalid until a matching approval receipt exists or an explicit rejection receipt blocks advancement.
3. Approval is route-bound:
   - the receipt must match the active `route_receipt_hash`,
   - stale approval receipts are invalid,
   - approval must be re-recorded after route-changing drift.
4. Rejection is also first-class:
   - rejected receipts must block synthesis-ready state,
   - the rejection notes become follow-up guidance for rework or escalation.

Vocabulary rule:

1. `policy_gate_required`, `senior_review_required`, and `human_gate_required` are the canonical runtime review-state names.
2. Older aliases such as `policy_check_pending`, `senior_review_pending`, and `requires_human` are legacy/reference terminology only and must not be emitted by new runtime artifacts, receipts, or protocol examples.
3. When an external roadmap or release document still uses the older names, treat this protocol and the active runtime protocols as authoritative for execution behavior.

## Receipt Artifact

Canonical artifact path:

1. `.vida/logs/route-receipts/<task_id>.<task_class>.approval.json`

Minimum fields:

1. `ts`
2. `task_id`
3. `task_class`
4. `review_state`
5. `decision` (`approved|rejected`)
6. `approver_id`
7. `notes`
8. `route_receipt_hash`
9. `route_receipt`

## Validation Rules

1. `promotion_ready` and `review_passed` do not require approval receipts.
2. `policy_gate_required`, `senior_review_required`, and `human_gate_required` require matching approval receipts.
3. Missing receipt -> `approval_pending`
4. Rejected receipt -> `blocked`
5. Stale receipt -> invalid until re-recorded

## Runtime Integration

1. Worker/verification manifests may reach a governance review target before final closure.
2. Runtime must apply the approval gate after technical verification and before closure-ready synthesis.
3. Approval gating must not silently downgrade the review state.

## Commands

```bash
python3 human-approval-gate.py record <task_id> <task_class> <review_state> <approved|rejected> --approver-id "<id>" --notes "<notes>"
python3 human-approval-gate.py status <task_id> <task_class>
python3 human-approval-gate.py validate <task_id> <task_class> <review_state>
```

## Fail-Closed Rule

1. Do not infer approval from task status, chat text, or generic “looks good”.
2. Do not reuse approval across route drift, issue-contract drift, or review-state escalation.

-----
artifact_path: config/runtime-instructions/human-approval.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.human-approval-protocol.md
created_at: '2026-03-07T22:08:06+02:00'
updated_at: '2026-03-11T13:03:17+02:00'
changelog_ref: work.human-approval-protocol.changelog.jsonl
