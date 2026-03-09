# VIDA Canonical Machine Map

Status: draft `v1` machine law

Revision: `2026-03-09`

Purpose: map the canonical runtime machines, their owned entities, states, events, key guards, and cross-machine dependencies for the partial development kernel.

## 1. Machine Set

The canonical machine set is:

1. `task_lifecycle`
2. `execution_plan`
3. `route_progression`
4. `coach_lifecycle`
5. `verification_lifecycle`
6. `approval_lifecycle`
7. `boot_migration_gate`

## 2. Shared Rules

1. machines must not duplicate ownership,
2. `Task.lifecycle_state` keeps the frozen `open|in_progress|closed|deferred` vocabulary,
3. route stages keep the frozen `analysis|writer|coach|verification|approval|synthesis` vocabulary,
4. back-edges are lawful only through explicit events and receipts,
5. projections may summarize multiple machines but own no canonical state,
6. checkpoints may snapshot resumability posture but own no canonical state,
7. gateways such as `awaiting_coach`, `awaiting_verification`, `awaiting_approval`, or manual intervention must remain projection-derived postures unless an owned machine state explicitly carries them,
8. listeners are derived runtime hooks over events and checkpoint boundaries, not machine-owned state.

## 2.1 Derived Runtime Surfaces

Each machine may emit, through runtime helpers:

1. projection topics,
2. listener topics,
3. checkpoint hints.

Rule:

1. these surfaces are derived from state, receipts, proofs, config law, or durable runtime ledgers,
2. they do not redefine machine ownership,
3. gateways for human interruption must map back to route, review, approval, or execution-plan semantics,
4. future replay/fork must derive from checkpoints without rewriting canonical history.

## 3. task_lifecycle

Entity: `Task`

Owned state:

1. `open`
2. `in_progress`
3. `closed`
4. `deferred`

Primary events:

1. `task_created`
2. `task_started`
3. `task_deferred`
4. `task_resumed`
5. `task_closed`
6. `task_reopened`

Key guards:

1. `execution_plan_present`
2. `route_authorized_for_start`
3. `deferral_reason_present`
4. `closure_ready_projection`
5. `approval_state_satisfied`
6. `verification_state_satisfied`
7. `no_reconciliation_blocker`

Back-edges:

1. `deferred -> open`
2. `closed -> open`

Boundary:

1. detailed waiting states do not belong here,
2. blocker posture is separate,
3. closure reads downstream machine state, not vice versa.

## 4. execution_plan

Entity: `ExecutionPlan`

Owned step status:

1. `todo`
2. `doing`
3. `done`
4. `blocked`

Owned block end-result:

1. `done`
2. `partial`
3. `failed`

Primary events:

1. `plan_created`
2. `step_started`
3. `step_completed`
4. `step_blocked`
5. `step_unblocked`
6. `block_completed`
7. `block_superseded`

Key guards:

1. `dependencies_satisfied`
2. `next_step_known`
3. `blocker_absent`
4. `resume_hint_present_when_required`

Back-edges:

1. `blocked -> todo`

Boundary:

1. execution telemetry is not task lifecycle,
2. execution completion does not close the task directly,
3. resumability checkpoints belong here before they appear in task closure projections,
4. future pending checkpoint writes belong here, not in task state.

## 5. route_progression

Entity: `RoutedRun` plus attached `RunNode`

Canonical stage axis:

1. `analysis`
2. `writer`
3. `coach`
4. `verification`
5. `approval`
6. `synthesis`

Canonical node status axis:

1. `pending`
2. `ready`
3. `running`
4. `completed`
5. `blocked`
6. `failed`
7. `skipped`

Primary events:

1. `route_resolved`
2. `assignment_requested`
3. `agent_assigned`
4. `lease_issued`
5. `stage_started`
6. `stage_blocked`
7. `stage_completed`
8. `fallback_selected`
9. `escalation_triggered`
10. `route_completed`
11. `route_failed`

Key guards:

1. `route_metadata_present`
2. `required_role_available`
3. `instruction_bundle_composed`
4. `independence_satisfied`
5. `lease_valid`
6. `fallback_allowed`
7. `escalation_lawful`

Back-edges:

1. `coach rework -> writer stage ready`
2. `verification fail -> writer stage ready`
3. `approval rejected -> writer or analysis stage ready`

Boundary:

1. route progression authorizes movement,
2. it does not replace task lifecycle,
3. it emits receipts and stage/node facts that downstream closure reads,
4. route projections may expose gateway posture and resume cursors without redefining route-stage law,
5. future gateway resume handles and trigger indexes map here before they are surfaced through operator projections.

## 6. coach_lifecycle

Entity: `CoachReview`

States:

1. `not_requested`
2. `requested`
3. `assigned`
4. `in_review`
5. `feedback_issued`
6. `rework_required`
7. `accepted`
8. `dismissed`
9. `closed`

Primary events:

1. `coach_requested`
2. `coach_assigned`
3. `coach_started`
4. `coach_feedback_issued`
5. `coach_rework_required`
6. `coach_feedback_accepted`
7. `coach_feedback_dismissed`
8. `coach_closed`

Key guards:

1. `coach_required_or_explicitly_not_required`
2. `coach_independent_from_writer`
3. `coach_feedback_present`
4. `rework_payload_present`

Boundary:

1. `coach` is formative review,
2. `coach` is not verification,
3. rework must produce both receipt and payload linkage back to task/run.

## 7. verification_lifecycle

Entity: `Verification`

States:

1. `not_requested`
2. `requested`
3. `assigned`
4. `in_verification`
5. `partial_results_received`
6. `aggregation_pending`
7. `passed`
8. `failed`
9. `inconclusive`
10. `superseded`
11. `closed`

Primary events:

1. `verification_requested`
2. `verifier_assigned`
3. `verification_started`
4. `verification_partial_received`
5. `verification_aggregated`
6. `verification_passed`
7. `verification_failed`
8. `verification_inconclusive`
9. `verification_superseded`
10. `verification_closed`

Key guards:

1. `verifier_independent`
2. `minimum_verifier_count_satisfied`
3. `proof_category_coverage_satisfied`
4. `aggregation_policy_satisfied`

Boundary:

1. verification is independent validation,
2. aggregation policy is explicit,
3. passed or failed verification influences closure guards but does not itself close the task,
4. future merge/aggregation strategies must remain explicit if parallel verification branches are introduced,
5. grouped verification projections may advance together, but verifier independence remains mandatory,
6. future merged verification verdicts must preserve partial-verifier receipts and explicit fallback to manual reconcile.

## 8. approval_lifecycle

Entity: `Approval`

States:

1. `not_required`
2. `requested`
3. `pending`
4. `approved`
5. `rejected`
6. `expired`
7. `escalated`
8. `withdrawn`
9. `closed`

Primary events:

1. `approval_required_determined`
2. `approval_requested`
3. `approval_received`
4. `approval_rejected`
5. `approval_expired`
6. `approval_escalated`
7. `approval_withdrawn`
8. `approval_closed`

Key guards:

1. `approval_required_by_policy`
2. `approver_class_allowed`
3. `approval_not_expired`
4. `approval_receipt_valid`

Boundary:

1. approval is governance-state plus approval proof,
2. approval does not replace verification.

## 9. boot_migration_gate

Entity: `BootVerdict` plus `MigrationRecord`

States:

1. `boot_unchecked`
2. `compat_checked`
3. `migration_required`
4. `migration_in_progress`
5. `doctor_required`
6. `ready_to_boot`
7. `boot_blocked`
8. `booted`
9. `boot_failed`

Verdict classes:

1. `compatible`
2. `compatible_with_warnings`
3. `migration_required`
4. `doctor_blocking`
5. `unsupported_revision`
6. `unsafe_to_boot`

Primary events:

1. `compatibility_checked`
2. `migration_required_detected`
3. `migration_started`
4. `migration_applied`
5. `doctor_verdict_issued`
6. `boot_allowed`
7. `boot_blocked`
8. `boot_started`
9. `boot_failed`

Key guards:

1. `compatibility_class_supported`
2. `migration_proof_satisfied`
3. `doctor_verdict_non_blocking`
4. `instruction_revision_supported`

Boundary:

1. boot/migration is startup-domain only,
2. it is not part of normal task flow,
3. it must fail closed.

## 10. Dependency Map

```text
boot_migration_gate
  -> enables runtime startup

execution_plan
  -> feeds route_progression and task start readiness

route_progression
  -> dispatches lanes, leases work, emits route receipts

coach_lifecycle
  -> can send work back to writer via rework

verification_lifecycle
  -> emits pass/fail/inconclusive input for closure guards

approval_lifecycle
  -> emits governance satisfaction input for closure guards

task_lifecycle
  -> closes only when downstream route/verification/approval/reconciliation inputs satisfy closure law
```

-----
artifact_path: product/spec/canonical-machine-map
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/product/spec/canonical-machine-map.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-09T20:28:59+02:00
changelog_ref: canonical-machine-map.changelog.jsonl
