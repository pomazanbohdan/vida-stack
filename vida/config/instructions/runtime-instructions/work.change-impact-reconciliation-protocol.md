# Change-Impact Reconciliation Protocol

Purpose: define the canonical reconciliation law when approved scope, acceptance criteria, dependencies, or governing decisions drift far enough that the current executable contract or task pool is no longer lawful.

## Scope

This protocol applies when drift is discovered:

1. after spec approval but before task materialization,
2. after task-pool creation but before implementation launch,
3. during active implementation,
4. during reflection/spec review when the current task pool is proven stale.

It owns:

1. drift classes that require reconciliation,
2. the stop-and-reconcile route,
3. resume admissibility after reconciliation,
4. the canonical bridge between reflection, spec re-baseline, and task-pool rebuild.

It does not own:

1. pack taxonomy or pack selection,
2. `/vida-spec review` internals,
3. `/vida-form-task` task-pool materialization law,
4. `/vida-implement` execution-loop law,
5. route-specific blocker catalogs outside the active owner.

## Core Contract

When material drift is detected, execution must fail closed rather than continue by inertia.

Canonical reconciliation route:

1. stop the current downstream flow at the active owner,
2. synchronize affected artifacts through `reflection-pack`,
3. re-baseline the contract through `/vida-spec review`,
4. rebuild queue/dependency state through `/vida-form-task` when the executable pool is affected,
5. resume only after the affected owner confirms the contract and executable queue are aligned again.

## Material Drift Classes

Reconciliation is mandatory when at least one of the following becomes true:

1. approved scope changed,
2. acceptance criteria changed,
3. a new dependency or ordering constraint changes execution order,
4. a decision update invalidates implementation assumptions,
5. the approved spec and executable DB-backed task queue no longer match,
6. active task boundaries no longer match the approved contract.

## Trigger Sources

The active owner may discover the trigger in different places:

1. `vida/config/instructions/instruction-contracts/core.orchestration-protocol.md`
   - routes change-impact triggers into reconciliation
2. `vida/config/instructions/command-instructions/planning.form-task-protocol.md`
   - blocks stale task-pool materialization or stale launch posture
3. `vida/config/instructions/command-instructions/execution.implement-execution-protocol.md`
   - blocks continued implementation against drifted contract or queue state
4. `vida/config/instructions/command-instructions/operator.vida-spec-guide.md`
   - re-baselines the contract after drift is confirmed

## Reconciliation Sequence

1. Raise the active owner's drift blocker when available.
2. Stop task-pool launch or implementation continuation.
3. Run `reflection-pack` so touched docs, decisions, and protocol-bearing artifacts are synchronized before contract review.
4. Run `/vida-spec review` to confirm or reject the changed contract baseline.
5. Run `/vida-form-task` when queue shape, dependency order, readiness, or launch admissibility changed.
6. Renew explicit launch confirmation before `/vida-implement` resumes when the executable pool or approved contract changed.

## Resume Admissibility

Reconciliation is complete only when all are true:

1. the changed contract is explicitly re-baselined,
2. the executable queue in the DB-backed task runtime matches the approved contract,
3. dependency ordering is rebuilt when drift changed execution order,
4. stale drift blockers are cleared by the active owner,
5. explicit launch confirmation is renewed before implementation resumes when dev execution was affected.

## Bridge Owners

This protocol is the canonical owner for generic change-impact reconciliation law.

Neighboring owners keep only their bounded responsibilities:

1. `core.orchestration-protocol.md`
   - trigger routing into `reflection-pack`
2. `command-instructions/planning.form-task-protocol.md`
   - pre-launch/task-pool blocker handling
3. `command-instructions/execution.implement-execution-protocol.md`
   - execution-time blocker handling
4. `command-instructions/operator.vida-spec-guide.md`
   - contract review and re-baseline

## Fail-Closed Rule

1. Do not continue task materialization or implementation after material drift by relying on the old pool or prior launch approval.
2. Do not treat `reflection-pack` alone as sufficient reconciliation proof.
3. Do not resume implementation until contract review, queue reconciliation, and launch renewal are complete when required.

-----
artifact_path: config/runtime-instructions/change-impact-reconciliation.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.change-impact-reconciliation-protocol.md
created_at: '2026-03-11T00:00:00+02:00'
updated_at: '2026-03-11T13:02:37+02:00'
changelog_ref: work.change-impact-reconciliation-protocol.changelog.jsonl
