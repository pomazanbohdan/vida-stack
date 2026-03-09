# VIDA Partial Development Kernel Scope

Status: draft `v1` root-config implementation scope

Revision: `2026-03-09`

Purpose: define the first canonical config-driven kernel for `vida`, freeze what belongs in the root config law, and separate state, proof, projection, instruction, and migration ownership before runtime implementation.

## 1. Goal

`vida v1` is a config-driven control kernel for:

1. task progression,
2. execution-plan and resumability state,
3. route progression and lawful assignment,
4. coach, verification, and approval gates,
5. receipt/proof tracking,
6. boot and migration gating,
7. deterministic operator projections,
8. listener/checkpoint kernel for resumable runtime views.

It is not the full autonomous platform.

## 2. Scope

### 2.1 In Scope

1. root `vida/config` tree,
2. canonical entity model,
3. seven machine specs,
4. route, lease, escalation, and assignment config,
5. instruction catalog and overlay composition model,
6. receipt/proof taxonomy,
7. projection/listener/checkpoint kernel semantics,
8. migration and boot-gate config,
9. schema and revision metadata,
10. parity-harness inputs and fixture expectations,
11. future strict machine-definition lint law and gateway resume targeting semantics.

### 2.2 Out Of Scope

1. free-form workflow scripting in configs,
2. arbitrary executable code embedded in YAML,
3. self-routing autonomous agents,
4. distributed orchestration topology,
5. provider-specific transport logic as product law,
6. non-deterministic transition semantics,
7. vendor-owned checkpoint threads, bookmark stores, or middleware stacks as root law.

## 3. Root Config Tree

Canonical root layout:

```text
vida/
  config/
    machines/
    routes/
    policies/
    agents/
    instructions/
    receipts/
    migration/
    schemas/
```

Rule:

1. root config artifacts are product law,
2. `docs/framework/*` and `vida-v0/*` are framework/runtime support surfaces outside product law,
3. `vida-v0` and `vida-v1` must read the same root config law.
4. concrete installed agent IDs, model IDs, and live backend inventory are runtime-owned bridge data, not canonical route or policy law.

## 4. Canonical Entities

The partial kernel freezes these entities:

1. `Task`
2. `Blocker`
3. `ExecutionPlan`
4. `RoutedRun`
5. `RunNode`
6. `CoachReview`
7. `Verification`
8. `Approval`
9. `Receipt`
10. `ProofAttachment`
11. `InstructionBinding`
12. `BootVerdict`
13. `MigrationRecord`

## 5. Source-Of-Truth Boundary

### 5.1 Canonical Truth

Only these are source of truth:

1. entity records,
2. receipts,
3. proof attachments,
4. config artifacts and their revisions.

### 5.2 Non-Canonical Surfaces

These are never source of truth:

1. CLI output,
2. cached summaries,
3. temporary route hints,
4. raw agent thoughts,
5. rendered status tables,
6. checkpoint payloads,
7. gateway resume handles,
8. shell-era helper paths or transcript memory.

## 6. State / Proof / Projection Split

### 6.1 State

State records hold current canonical posture.

Examples:

1. `Task.lifecycle_state`
2. `RunNode.status`
3. `CoachReview.state`
4. `Verification.state`
5. `Approval.state`
6. `BootVerdict.state`

### 6.2 Receipts

Receipts are append-only immutable records that a lawful transition, assignment, handoff, or gate decision occurred.

### 6.3 Proof

Proof attachments are immutable evidence artifacts referenced by guards, verification, approval, doctor, or reconciliation logic.

### 6.4 Projections

Projections are rebuildable derived views for `status`, `doctor`, queues, readiness, and reconcile surfaces.

### 6.5 Listeners

Listener hooks are runtime-derived subscription descriptors attached to lawful machine events or checkpoint boundaries.

Rules:

1. listeners are not state,
2. listeners are not receipts,
3. listeners may trigger projection refresh or operator-facing updates only through runtime-owned helpers,
4. listeners may coordinate grouped projections when those projections must advance together.

### 6.5 Checkpoints

Checkpoints are resumability artifacts derived from canonical state, receipts, and projections.

Rules:

1. checkpoints are not canonical state,
2. checkpoints are not receipts,
3. checkpoints are not proofs,
4. checkpoints may reference receipts, proofs, and projection cursors,
5. losing a checkpoint must not lose canonical truth,
6. future checkpoint commit law may advance only gap-less resumability posture.

### 6.6 Gateway Resume Targeting

Resume targeting for human or external gateways is a future kernel concern.

Rules:

1. resume targeting must prefer explicit correlation keys, resume handles, or trigger indexes,
2. broad runtime scans are not lawful as the default resume mechanism,
3. gateway handles remain runtime-owned artifacts unless promoted by explicit product-law spec.

## 7. Critical Reconciliation With Frozen Specs

The partial kernel must not silently redefine the already frozen `0.3` kernels.

### 7.1 Task Lifecycle Rule

Per `docs/framework/history/plans/2026-03-08-vida-0.3-state-kernel-schema-spec.md`, canonical `Task.lifecycle_state` remains:

1. `open`
2. `in_progress`
3. `closed`
4. `deferred`

Rule:

1. `blocked` is not a task lifecycle state,
2. `awaiting_coach`, `awaiting_verification`, `awaiting_approval`, and similar postures must live in route, governance, review, verification, approval, or projection surfaces rather than in `Task.lifecycle_state`.

### 7.2 Route Stage Rule

Per `docs/framework/history/plans/2026-03-08-vida-0.3-route-and-receipt-spec.md`, canonical route stages remain:

1. `analysis`
2. `writer`
3. `coach`
4. `verification`
5. `approval`
6. `synthesis`

Route config may add assignment, lease, fallback, and escalation metadata, but must not replace this stage law.

Implementation note for the first root-config pass:

1. `routes/route_catalog.yaml` owns task-class to lane bindings,
2. the route catalog may also carry bounded assignment hints such as `external_first_required` and generic trait preferences like `preferred_capabilities` or `preferred_write_scopes`,
3. these hints influence lawful candidate selection but do not redefine the canonical route stages.

### 7.3 Governance Rule

Governance requirement state remains state-owned; approval proof remains proof-owned.

### 7.4 Migration Rule

Boot/migration compatibility and fail-closed startup posture remain migration-owned and must not be absorbed into normal task or route state.

## 8. Seven Machines

The first runtime is built around these seven machines:

1. `task_lifecycle`
2. `execution_plan`
3. `route_progression`
4. `coach_lifecycle`
5. `verification_lifecycle`
6. `approval_lifecycle`
7. `boot_migration_gate`

## 9. Instruction Integration Model

Instructions are behavior law, not direct mutators.

Effective composition order:

1. system/base instructions,
2. machine-level bindings,
3. route-level overlay,
4. role or agent overlay,
5. task-specific overlay,
6. emergency override when explicitly authorized.

Runtime rule:

1. agents receive effective instructions,
2. agents emit outcome events, artifacts, blockers, and evidence,
3. listeners and projections may derive checkpoint payloads or operator views,
4. middleware/listener helpers may not mutate canonical state directly,
5. only runtime transitions may mutate canonical state.

## 10. Current Overlay Mapping

The current root `vida.config.yaml` is treated as a bridge source and mapped into the new root config law as follows:

1. `project_bootstrap` -> `instructions/overlays/project_overlay.yaml`
2. `language_policy` -> `instructions/overlays/project_overlay.yaml`
3. `autonomous_execution` -> `policies/closure_policy.yaml` and `instructions/overlays/project_overlay.yaml`
4. `framework_self_diagnosis` -> `policies/closure_policy.yaml` and `migration/doctor_verdicts.yaml`
5. `agent_system` -> root `agents/*.yaml` for generic role/class law, `routes/route_catalog.yaml`, `policies/assignment_policy.yaml`, and runtime-derived concrete inventory from `vida.config.yaml`
6. `pack_router_keywords` -> deferred bridge input; not part of the canonical kernel law in this pass

## 11. Command Homes

The config-driven kernel must ultimately serve these root command homes:

1. `vida task`
2. `vida run`
3. `vida coach`
4. `vida verify`
5. `vida boot`
6. `vida status`
7. `vida doctor`
8. `vida memory`

## 12. Parity Basis

Parity remains grounded in:

1. `docs/framework/history/plans/2026-03-08-vida-0.2-semantic-freeze-spec.md`
2. `docs/framework/history/plans/2026-03-08-vida-semantic-extraction-layer-map.md`
3. `docs/framework/history/plans/2026-03-08-vida-0.3-parity-and-conformance-spec.md`
4. `docs/product/spec/external-pattern-borrow-map.md` for approved external semantic borrow decisions

Rule:

1. root config law is allowed to replace legacy topology,
2. it is not allowed to silently change frozen semantics without an explicit delta record.
