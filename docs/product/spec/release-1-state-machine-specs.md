# Release 1 State Machine Specs

Status: active Release-1 control law

Purpose: define the canonical state machines for the key `Release 1` control lifecycles so transition rules, required evidence, and forbidden shortcuts are explicit.

## 1. Scope

This document defines FSMs for:

1. lane lifecycle,
2. approval lifecycle,
3. tool execution lifecycle,
4. incident and recovery lifecycle,
5. prompt rollout lifecycle.

## 2. General Transition Rule

1. A transition is legal only when its required evidence exists.
2. Any unspecified transition is forbidden.
3. Recovery does not erase prior evidence; it appends new evidence.

## 3. Lane Lifecycle FSM

States:

1. `packet_ready`
2. `lane_open`
3. `lane_running`
4. `lane_blocked`
5. `lane_completed`
6. `lane_superseded`
7. `lane_exception_takeover`

Allowed transitions:

1. `packet_ready -> lane_open`
   - requires packet id and lane assignment
2. `lane_open -> lane_running`
   - requires carrier activation and trace root
3. `lane_running -> lane_completed`
   - requires lane execution receipt with result artifacts
4. `lane_running -> lane_blocked`
   - requires explicit blocker reason
5. `lane_running -> lane_superseded`
   - requires supersession receipt
6. `lane_blocked -> lane_running`
   - requires resumed authority and unresolved blocker cleared
7. `lane_open | lane_running | lane_blocked -> lane_exception_takeover`
   - requires explicit exception-path receipt

Forbidden transitions:

1. `lane_open -> lane_completed` without runtime evidence
2. `lane_open -> local_root_takeover` without exception receipt
3. `lane_running -> lane_running` as hidden retry without appended event

## 4. Approval Lifecycle FSM

States:

1. `approval_not_required`
2. `approval_required`
3. `waiting_for_approval`
4. `approved`
5. `denied`
6. `expired`

Allowed transitions:

1. `approval_required -> waiting_for_approval`
2. `waiting_for_approval -> approved`
3. `waiting_for_approval -> denied`
4. `approved -> expired`

Required evidence:

1. transition to `approved` requires approval record
2. transition to `denied` requires denial reason
3. transition to `expired` requires expiry timestamp or policy trigger

## 5. Tool Execution Lifecycle FSM

States:

1. `tool_ready`
2. `waiting_for_tool`
3. `tool_result_captured`
4. `tool_failed`
5. `compensating_action_required`
6. `tool_closed`

Allowed transitions:

1. `tool_ready -> waiting_for_tool`
   - requires normalized tool contract
2. `waiting_for_tool -> tool_result_captured`
   - requires trace span and raw result capture
3. `waiting_for_tool -> tool_failed`
   - requires failure taxonomy entry
4. `tool_failed -> compensating_action_required`
   - requires rollback posture
5. `tool_result_captured -> tool_closed`
6. `compensating_action_required -> tool_closed`
   - requires compensation result

## 6. Incident And Recovery Lifecycle FSM

States:

1. `incident_declared`
2. `rollback_or_restore_running`
3. `trust_reevaluation`
4. `recovered`
5. `escalated`

Allowed transitions:

1. `incident_declared -> rollback_or_restore_running`
2. `rollback_or_restore_running -> trust_reevaluation`
3. `trust_reevaluation -> recovered`
4. `incident_declared | rollback_or_restore_running | trust_reevaluation -> escalated`

Required evidence:

1. incident evidence bundle at declaration
2. rollback/restore trace during recovery
3. trust reevaluation verdict before `recovered`

## 7. Prompt Rollout Lifecycle FSM

States:

1. `draft`
2. `benchmarked`
3. `approved_for_rollout`
4. `canary`
5. `promoted`
6. `rolled_back`

Allowed transitions:

1. `draft -> benchmarked`
   - requires evaluation run
2. `benchmarked -> approved_for_rollout`
   - requires benchmark pass and approval
3. `approved_for_rollout -> canary`
4. `canary -> promoted`
   - requires regression gate pass
5. `canary | promoted -> rolled_back`
   - requires rollback target and incident or regression evidence

## 8. References

1. `docs/product/spec/release-1-decision-tables.md`
2. `docs/product/spec/release-1-control-metrics-and-gates.md`
3. `docs/product/spec/release-1-canonical-artifact-schemas.md`

-----
artifact_path: product/spec/release-1-state-machine-specs
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-16
schema_version: 1
status: canonical
source_path: docs/product/spec/release-1-state-machine-specs.md
created_at: 2026-03-16T11:35:00Z
updated_at: 2026-03-16T11:28:19.784049012Z
changelog_ref: release-1-state-machine-specs.changelog.jsonl
