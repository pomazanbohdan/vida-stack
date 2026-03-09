# VIDA 0.3 State Kernel Schema Spec

Purpose: define the authoritative task/workflow state model for direct `1.0`, freeze the state semantics that must survive the rewrite, and separate those semantics from `0.1` storage topology and later route/receipt proof law.

Status: canonical `0.3` spec artifact for the direct `1.0` program.

Date: 2026-03-08

---

## 1. Executive Decision

The direct `1.0` state kernel owns authoritative runtime facts for:

1. task/workflow lifecycle,
2. dependency and blocker posture,
3. execution-plan progress surfaces,
4. run-graph and resumability state,
5. governance review requirement state,
6. canonical reconciliation/readiness summaries.

The state kernel does **not** own:

1. route/authorization proof payloads,
2. approval/verification proof payloads,
3. exact receipt schemas,
4. `0.1` backend/layout topology,
5. provider- or script-specific transport details.

Compact rule:

`freeze authoritative task/run/governance facts; keep proofs separate; discard 0.1 topology`

---

## 2. Why This Spec Comes Next

The command-tree spec froze the future operator homes:

1. `vida task ...` mutates workflow state,
2. `vida status` summarizes it,
3. `vida boot` hydrates resumable position,
4. `vida doctor` diagnoses blocker state.

The next blocker after that operator freeze is the authoritative state model those commands will read, mutate, and summarize.

Without this artifact:

1. instruction-kernel work would bind behavior to vague or conflicting state surfaces,
2. migration work would not know what `0.1` facts to preserve,
3. route/receipt work could incorrectly absorb state facts into proof payloads,
4. parity work would not know which state semantics are canonical.

---

## 3. Source Basis

Primary local source basis:

1. `AGENTS.md`
2. `vida/config/instructions/agent-definitions.orchestrator-entry.md`
3. `vida/config/instructions/instruction-contracts.thinking-protocol.md`
4. `vida.config.yaml`
5. `docs/framework/history/plans/2026-03-08-vida-0.1-to-1.0-direct-binary-transition-plan.md`
6. `docs/framework/history/plans/2026-03-08-vida-semantic-extraction-layer-map.md`
7. `docs/framework/history/plans/2026-03-08-vida-0.2-semantic-freeze-spec.md`
8. `docs/framework/history/plans/2026-03-08-vida-0.2-bridge-policy.md`
9. `docs/framework/history/plans/2026-03-08-vida-0.3-command-tree-spec.md`
10. `docs/framework/history/plans/2026-03-08-vida-direct-1.0-local-spec-program.md`
11. `vida/config/instructions/runtime-instructions.beads-protocol.md`
12. `vida/config/instructions/runtime-instructions.taskflow-protocol.md`
13. `vida/config/instructions/runtime-instructions.task-state-reconciliation-protocol.md`
14. `vida/config/instructions/runtime-instructions.run-graph-protocol.md`
15. `vida/config/instructions/runtime-instructions.human-approval-protocol.md`
16. `vida/config/instructions/instruction-contracts.agent-system-protocol.md`
17. `vida/config/instructions/runtime-instructions.framework-memory-protocol.md`
18. `vida/config/instructions/diagnostic-instructions.silent-framework-diagnosis-protocol.md`

Bounded explorer lanes used for synthesis:

1. semantic state vocabulary and transition inventory,
2. topology-leak inventory for `br`/JSONL/file/layout surfaces,
3. state-vs-proof boundary analysis for run-graph, resumability, review, and approval facts.

Source synthesis rule:

1. `0.2` freeze artifacts define the preserved semantic vocabulary,
2. `0.1` runtime protocols provide current operational meaning,
3. command-tree law defines which future operator families must read or mutate these facts,
4. where local sources conflict, the higher-precedence frozen semantic sources win over lower-level operational drift.

---

## 4. Purpose Of The State Kernel Schema Spec

This spec answers one question:

1. what authoritative state entities, vocabularies, and mutation boundaries must the direct `1.0` binary own before instruction, migration, and route/receipt law can be finalized?

This spec defines:

1. authoritative state entities,
2. canonical vocabulary sets that belong in state,
3. mutation-law boundaries,
4. the state/proof split,
5. the state/topology split,
6. state-level invariants and non-goals.

This spec does not define:

1. exact receipt schemas,
2. exact route authorization law,
3. exact migration procedure,
4. exact Rust storage/module layout,
5. detailed memory-kernel behavior.

Storage decision note:

1. authoritative direct `1.0` state persists in embedded `SurrealDB`,
2. the canonical local backend is `kv-surrealkv`,
3. this artifact stays above exact record layout while no longer leaving the storage substrate open.

---

## 5. Authoritative State Entities

The direct `1.0` state kernel should be structured around the following entities and state surfaces.

### 5.1 `Task`

Purpose:

1. canonical workflow unit for lifecycle, dependency, blocker, and governance posture,
2. the unit mutated by `vida task ...`,
3. the unit summarized by `vida status`.

Minimum state facts:

1. `task_id`
2. `task_class`
3. `lifecycle_state`
4. dependency edges or references
5. active blocker posture
6. current review-state requirement
7. linkage to current or latest routed run

Canonical lifecycle vocabulary:

1. `open`
2. `in_progress`
3. `closed`
4. `deferred`

Lifecycle rule:

1. `blocked` is **not** a canonical task lifecycle state in the direct `1.0` freeze.
2. Blocking is represented through blocker records, reconciliation/readiness summaries, execution-step state, and run-node state.

### 5.2 `TaskDependency`

Purpose:

1. represent task-to-task or scoped workflow dependency relationships without overloading lifecycle state,
2. preserve current `deps` / `depends_on` semantics.

Minimum state facts:

1. source task
2. target task
3. dependency kind or relation label
4. active/satisfied posture

### 5.3 `TaskBlocker`

Purpose:

1. represent blocker facts as first-class state rather than implicit notes or ad hoc lifecycle aliases,
2. let `vida task`, `vida status`, `vida boot`, and `vida doctor` expose blocker posture without inventing a new lifecycle state.

Minimum state facts:

1. blocker id
2. blocking scope
3. source surface
4. reason class
5. active/resolved posture
6. evidence reference or proof reference

Freeze rule:

1. this artifact freezes the existence and required role of blocker records,
2. it does **not** yet freeze a complete blocker-code catalog.

### 5.4 `ExecutionPlanState`

Purpose:

1. preserve TaskFlow execution semantics inside the future kernel,
2. keep workflow execution progress distinct from task lifecycle state,
3. provide canonical active/next-step visibility without creating a second queue state engine.

Minimum state facts:

1. ordered or linked execution steps / blocks,
2. step status,
3. block end-result,
4. `next_step`
5. optional track ownership and dependency metadata

Canonical execution-step vocabulary:

1. `todo`
2. `doing`
3. `done`
4. `blocked`

Canonical block end-result vocabulary:

1. `done`
2. `partial`
3. `failed`

Hard boundary:

1. `ExecutionPlanState` is execution telemetry and resumability support,
2. it is not the authoritative task-lifecycle engine,
3. it must not replace dependency/blocker/review/lifecycle state on the `Task` entity.

### 5.5 `RoutedRunState`

Purpose:

1. preserve one routed execution run as durable resumability state,
2. support `vida boot`, `vida status`, and `vida doctor` summaries,
3. keep routed-stage state distinct from queue/task lifecycle state.

Minimum state facts:

1. `task_id`
2. `task_class`
3. `route_task_class`
4. `updated_at`
5. required run nodes
6. `resume_hint`

### 5.6 `RunNodeState`

Purpose:

1. persist node-level orchestration state inside one routed run,
2. preserve current run-graph semantics without freezing current file layout.

Canonical node vocabulary:

1. `analysis`
2. `writer`
3. `coach`
4. `problem_party`
5. `verifier`
6. `approval`
7. `synthesis`

Canonical node-status vocabulary:

1. `pending`
2. `ready`
3. `running`
4. `completed`
5. `blocked`
6. `failed`
7. `skipped`

### 5.7 `GovernanceState`

Purpose:

1. hold the current governance/review requirement that affects closure readiness,
2. keep governance requirement distinct from approval proof.

Canonical review-state vocabulary:

1. `review_passed`
2. `policy_gate_required`
3. `senior_review_required`
4. `human_gate_required`
5. `promotion_ready`

Boundary rule:

1. the state kernel owns the current `review_state`,
2. the proof layer owns the approval receipt and approval decision evidence that satisfies or blocks that state.

### 5.8 `ResumabilityCapsule`

Purpose:

1. preserve compact-safe continuation context,
2. keep execution resumability independent of chat memory.

Required capsule fields:

1. `epic_id`
2. `epic_goal`
3. `task_id`
4. `task_role_in_epic`
5. `done`
6. `next`
7. `constraints`
8. `open_risks`
9. `acceptance_slice`

### 5.9 `TaskReconciliationSummary`

Purpose:

1. provide a canonical readiness and drift summary across task state, execution telemetry, and routed run state,
2. preserve TSRP semantics without making reconciliation a second mutation engine.

Canonical reconciliation vocabulary:

1. `active`
2. `blocked`
3. `done_ready_to_close`
4. `stale_in_progress`
5. `open_but_satisfied`
6. `drift_detected`
7. `invalid_state`
8. `closed`

Required output posture:

1. classification
2. reasons
3. allowed actions
4. current block summary
5. resumability summary

Reconciliation rule:

1. this is a canonical derived state surface,
2. it is not a second lifecycle engine,
3. direct manual lifecycle mutation must not bypass reconciliation when drift or stale state is detected.

---

## 6. Canonical Vocabulary And Mutation Law

### 6.1 Single-Writers And Surface Separation

State mutation law for direct `1.0`:

1. there is one canonical mutation path for authoritative task/workflow state,
2. `vida task ...` is the only root command family that may mutate authoritative workflow/task state in normal operation,
3. `vida status` is read-only,
4. `vida boot` hydrates and summarizes resumability state but must not silently mutate workflow state,
5. `vida doctor` diagnoses and reports blocker/compatibility state but must not become a hidden mutation bypass,
6. task lifecycle state, execution telemetry, routed-run state, governance state, and proof receipts remain separate surfaces even when stored inside one binary-owned kernel family.

### 6.2 Task Lifecycle Mutation Law

Direct `1.0` freezes these lifecycle rules:

1. only the canonical lifecycle set `open|in_progress|closed|deferred` is part of task lifecycle law,
2. `in_progress` means active or resumable work exists,
3. `deferred` means intentionally parked by policy or sequencing, not merely blocked by missing proof,
4. `closed` is invalid while reconciliation class is `stale_in_progress`, `drift_detected`, or `invalid_state`,
5. `open_but_satisfied` and `done_ready_to_close` are reconciliation/readiness summaries, not lifecycle states,
6. unresolved blockers must not be encoded by inventing a new lifecycle alias when blocker state can represent them directly.

### 6.3 Execution-Plan Mutation Law

Execution-plan rules that survive:

1. non-trivial work is planned before execution,
2. active execution is represented through execution-step / block state, not only chat narration,
3. step status and block end-result remain separate surfaces,
4. redirects and supersession remain execution-history behavior, not task-lifecycle mutation,
5. exact storage shape for step graphs remains open, but the semantic separation between task lifecycle and execution telemetry is frozen.

### 6.4 Run-State Mutation Law

Run-state rules that survive:

1. run-graph state is the canonical node-level resumability surface for one routed run,
2. run-node state changes are explicit and durable,
3. run-graph is not a second task queue or readiness engine,
4. `resume_hint` is a first-class resumability surface,
5. hydration failure is blocking,
6. resumability state must survive compact without chat memory.

### 6.5 Governance-State Mutation Law

Governance rules that survive:

1. `policy_gate_required`, `senior_review_required`, and `human_gate_required` are blocking governance states, not advisory labels,
2. `review_passed` and `promotion_ready` are governance-state names, not proof payloads,
3. review-state changes must not silently infer missing approval proof,
4. approval satisfaction is proof-bound even when the current review requirement is state-owned.

### 6.6 Reconciliation And Close Law

Close law that survives:

1. stale, drifted, or structurally contradictory work must be reconciled before closure,
2. readiness for close is determined from state plus proof, not from lifecycle label alone,
3. closure-ready summaries belong in reconciliation/readiness surfaces, not in ad hoc lifecycle aliases.

---

## 7. Dependency, Blocker, Review, Approval, And Readiness Surfaces

### 7.1 Dependencies

The state kernel must own:

1. dependency edges,
2. whether a dependency is still blocking readiness,
3. dependency-aware reopen/resume posture.

It must not require:

1. `0.1` `br` topology,
2. JSONL adjacency or queue-specific representation,
3. `SQLite`-specific tables, row ids, or transaction choreography.

### 7.2 Blockers

The state kernel must own blocker facts as first-class records because:

1. task lifecycle does not freeze `blocked` as a canonical lifecycle enum,
2. `vida status` and `vida doctor` need a durable blocker view,
3. migration must map current `0.1` blocked behavior into first-class state instead of a shell-era alias.

This artifact freezes:

1. blocker existence,
2. blocker/source/evidence linkage,
3. active/resolved posture.

This artifact leaves open:

1. complete blocker-code vocabulary,
2. detailed blocker taxonomy and presentation rules.

### 7.3 Review And Approval Boundary

What belongs in state:

1. the current `review_state`,
2. whether governance review is required before closure can be considered,
3. route-adjacent readiness posture needed by `vida status` and `vida doctor`.

What does not belong in state as authoritative proof:

1. approval receipt payloads,
2. approver identity and notes,
3. route-hash binding,
4. stale-approval validation logic.

Approval vocabulary preserved for later proof law:

1. `approved`
2. `rejected`

Rule:

1. approval decisions remain canonical vocabulary,
2. the authoritative proof of those decisions remains in route/approval receipts, not in task lifecycle state.

### 7.4 Readiness

Readiness that belongs in state is the canonical summary posture exposed by reconciliation and resumability surfaces, including:

1. `active`
2. `blocked`
3. `done_ready_to_close`
4. `stale_in_progress`
5. `open_but_satisfied`
6. `drift_detected`
7. `invalid_state`
8. `closed`

Readiness rule:

1. these are canonical summary/read-model states,
2. they do not replace task lifecycle state,
3. they provide the state-owned side of close/readiness while proof receipts provide the evidence-bearing side.

---

## 8. Run-Graph And Resumability State

The state kernel must preserve the following run/resume semantics:

1. one routed run has canonical node-level state,
2. required nodes are frozen as `analysis|writer|coach|problem_party|verifier|approval|synthesis`,
3. required node statuses are frozen as `pending|ready|running|completed|blocked|failed|skipped`,
4. `resume_hint` is mandatory for compact-safe continuation,
5. context capsule fields are mandatory resumability state,
6. hydration failure is blocking,
7. run state remains distinct from queue/task lifecycle state.

Direct `1.0` consequence:

1. `vida boot` can restore resumable position without replaying logs,
2. `vida status` can summarize active routed stage,
3. `vida doctor` can explain why resume is blocked,
4. route/receipt work can later attach proof payloads without owning the run-state facts themselves.

---

## 9. State-Owned Facts Vs Route/Receipt-Owned Proofs

| Domain | State-owned now | Proof-owned later |
|---|---|---|
| Task identity and lifecycle | `task_id`, `task_class`, `lifecycle_state` | n/a |
| Dependencies | dependency edges and active/satisfied posture | proof receipts that justify dependency satisfaction when required |
| Blockers | blocker existence, source, active/resolved posture | exact blocker-proof artifacts and evidence payloads |
| Execution progress | execution-step state, block end-result, `next_step` | proof artifacts attached to step completion when route law requires them |
| Routed execution | `route_task_class`, run nodes, node statuses, `resume_hint` | route receipt, escalation receipt, analysis receipt |
| Governance | current `review_state` | approval receipt, approver identity, route-hash validation |
| Verification posture | readiness/reconciliation summary may reflect pending verification | verifier plan, verifier artifact, health/verification proof |
| Compact recovery | context capsule fields and hydration-required posture | boot/verify receipt proving successful hydration |

Boundary rule:

1. state owns the current durable fact,
2. route/receipt law owns the proof that the fact was authorized, verified, approved, or satisfied,
3. later receipt payloads must reference or justify state facts, not replace them as the authoritative source.

---

## 10. State Semantics Vs Discardable `0.1` Topology

### 10.1 Preserve Semantically

Direct `1.0` must preserve:

1. one SSOT for task lifecycle state,
2. one canonical mutation path,
3. separation between lifecycle state and execution telemetry,
4. dependency and blocker representation,
5. canonical reconciliation/readiness summaries,
6. run-graph/resumability semantics,
7. governance requirement state,
8. compact-safe context-capsule semantics.

### 10.2 Discard Mechanically

Direct `1.0` must **not** freeze:

1. `br` as the long-term backend,
2. `.beads/issues.jsonl`,
3. JSONL-first or `br --no-db` runtime modes,
4. `.vida/state/*` and `.vida/logs/*` path layout,
5. queue-backed shell mutator choreography as product law,
6. `docs/framework/history/_vida-source/scripts/*.sh|*.py` entrypoints,
7. shell/Python split,
8. current CLI/provider transport assumptions,
9. cache/log naming or tmp-file choreography,
10. current command/helper names used to surface the state,
11. `SQLite` as an alternative authoritative substrate.

### 10.3 Bridge Rule

Current `0.1` topology may remain bridge- or export-only when it is needed to produce:

1. frozen state vocabulary,
2. canonical transition examples,
3. run-graph examples,
4. context-capsule examples,
5. parity fixtures,
6. migration inputs.

But bridge-only topology must not become `1.0` product law.

---

## 11. Kernel Dependencies And Boundaries

### 11.1 Dependency On Command Tree Spec

This spec depends on command-tree law already frozen by:

1. `vida task ...` as the sole mutation-owning root family,
2. `vida status` as read-only,
3. `vida boot` as resumability hydration entry,
4. `vida doctor` as blocker/diagnosis surface.

### 11.2 Dependency On Instruction Kernel Spec

The instruction kernel will next depend on this state model for:

1. instruction-driven behavior that reads current lifecycle/governance/run state,
2. precise state mutation boundaries,
3. instruction/render separation from state semantics.

### 11.3 Dependency On Route And Receipt Spec

The route/receipt spec will later depend on this artifact for:

1. which facts are already authoritative state,
2. where approval/verification/authorization proofs attach,
3. how receipt law avoids swallowing state facts.

### 11.4 Dependency On Migration Kernel Spec

The migration kernel will later depend on this artifact for:

1. mapping `0.1` `blocked` and topology-heavy representations into canonical `1.0` state,
2. translating `br`/JSONL/file-layout carriers into binary-owned `SurrealDB` state entities,
3. startup fail-closed checks for incompatible legacy state.

### 11.5 Memory Boundary

Framework memory remains durable state, but not part of this task/workflow state kernel.

Rule:

1. `lesson|correction|anomaly` memory semantics remain preserved,
2. a later memory-kernel contract owns their detailed storage and operator behavior,
3. this artifact does not absorb memory-kernel design into task/workflow state.

---

## 12. State-Level Invariants

The direct `1.0` state kernel must preserve these invariants:

1. task lifecycle vocabulary is exactly `open|in_progress|closed|deferred` unless a later higher-precedence migration artifact explicitly revises the freeze,
2. `blocked` is represented by blocker/readiness/run/execution surfaces, not by silently widening lifecycle vocabulary,
3. authoritative task/workflow state has one canonical mutation path,
4. execution telemetry does not become a second lifecycle engine,
5. run-graph state does not become a second task queue,
6. governance requirement state is authoritative, but approval proof remains route-bound,
7. compact-safe resumability must work without chat memory,
8. readiness/close summaries must be derivable from authoritative state plus proofs,
9. state semantics outrank `0.1` storage topology,
10. undefined behavior remains forbidden by default.

---

## 13. State-Level Non-Goals

This spec does not:

1. define exact receipt schemas,
2. define exact route authorization rules,
3. define exact approval-validation algorithms,
4. define the full blocker-code enum catalog,
5. define exact storage tables, file layout, or Rust structs,
6. define memory-kernel behavior in full,
7. define migration procedures in full,
8. define parity fixture format in full,
9. reopen command-tree law,
10. start Rust implementation.

---

## 14. Open Ambiguities

The following remain intentionally open and must be resolved later without invalidating this artifact:

1. `0.1` operational drift still uses `blocked` as a task-level `br` status in some flows, while the frozen lifecycle set excludes it.
2. The full blocker-code vocabulary is not yet frozen beyond isolated examples such as `BLK_CONTEXT_NOT_HYDRATED` and `BLK_USER_DECISION_PENDING`.
3. The exact authoritative storage shape for execution-plan steps/blocks remains open, even though their semantic role is frozen.
4. The exact authoritative storage shape for current `review_state` remains open.
5. The exact route/receipt schema for approval, verification, analysis, escalation, and boot proof remains open.
6. Lifecycle state, review state, approval satisfaction, and closure-ready state are not yet one fully unified cross-surface state machine.
7. Run-node metadata shape remains illustrative beyond the required node identities and statuses.
8. Ownership remains unsettled for some secondary surface labels such as `approval_pending`, `superseded`, and `decision_required|autonomous`; this artifact does not freeze them as state-kernel vocabulary.

---

## 15. Downstream Contracts Unlocked By This Spec

This artifact unlocks:

1. `Instruction Kernel Spec`
   - because instruction behavior can now bind to explicit state entities and mutation boundaries.
2. `Migration Kernel Spec`
   - because `0.1` lifecycle/topology drift now has a canonical target state model to migrate into.
3. `Route And Receipt Spec`
   - because route, approval, verification, and escalation proofs now have a clear boundary against authoritative state.
4. `Parity And Conformance Spec`
   - because canonical state vocabularies, mutation boundaries, and resumability invariants are now frozen enough to test.

---

## 16. Immediate Next Artifact

The next artifact is:

1. `docs/framework/history/plans/2026-03-08-vida-0.3-instruction-kernel-spec.md`

Reason:

1. the command surface is frozen,
2. the state model it operates on is now frozen,
3. the next missing kernel law is the instruction hierarchy and effective instruction composition that will govern command behavior over this state.
-----
artifact_path: framework/plans/vida-0.3-state-kernel-schema-spec
artifact_type: plan
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/plans/vida-0.3-state-kernel-schema-spec.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: vida-0.3-state-kernel-schema-spec.changelog.jsonl
P26-03-09T21: 44:13Z
