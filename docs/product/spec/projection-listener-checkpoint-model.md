# VIDA Projection, Listener, And Checkpoint Model

Status: draft `v1` bounded runtime artifact

Revision: `2026-03-09`

Purpose: define the smallest lawful runtime surface for derived projections, listener topics, and checkpoint hints without collapsing state, receipts, proof, and projection into one layer.

## 1. Scope

This artifact defines:

1. transition-derived projection payloads,
2. listener topic derivation,
3. checkpoint hint semantics,
4. interrupt/gateway mapping boundaries,
5. grouped projection consistency boundaries,
6. future checkpoint commit and replay boundaries.

It does not define:

1. a vendor thread/checkpointer API,
2. callback-owned canonical mutation,
3. a generic autonomous agent loop,
4. a new permanent workflow engine.

## 2. Core Boundary

### 2.1 Projection

A projection is a rebuildable derived view over canonical state, receipts, proofs, and runtime-ledger facts.

Projection examples:

1. `status`
2. `readiness`
3. `resume_hint`
4. `closure_ready`
5. `governance_wait`

### 2.2 Listener

A listener is a derived runtime subscription descriptor attached to a transition or checkpoint boundary.

Rule:

1. listeners do not own state,
2. listeners do not bypass receipts,
3. listeners may suggest downstream work such as status refresh, queue recalculation, or operator notification,
4. one listener/subscription boundary may serve multiple grouped projections when they must advance together.

### 2.3 Checkpoint

A checkpoint is a durable runtime snapshot or resumability marker written at a lawful boundary.

Rule:

1. checkpoints are runtime durability artifacts,
2. checkpoints are not canonical state by themselves,
3. if checkpoint creation is decision-relevant, it must be receipt-backed or proof-backed,
4. future checkpoint commit law may advance only a gap-less cursor,
5. future delayed checkpoint commits require idempotent downstream handlers.

### 2.4 Gateway

A gateway is an explicit interrupt/resume boundary where runtime continuation depends on human or external input.

Examples:

1. manual intervention
2. approval wait
3. human clarification hold

Rule:

1. gateways must be inspectable,
2. gateways must map to route, review, approval, or execution-plan posture,
3. gateways must not silently become ad hoc middleware state,
4. future resume targeting should prefer correlation-based handles or indexed trigger keys over broad scan.

## 3. Adopted Now

The bounded kernel adopts now:

1. transition-derived projection payloads,
2. listener topic derivation as runtime metadata,
3. checkpoint hints derived from machine/event semantics,
4. execution-time checkpoint snapshots as a lawful direction for runtime durability,
5. grouped projection boundaries under one subscription/listener surface.

## 4. Future Direction Only

Accepted for future work, not required in this pass:

1. persisted interrupt handles for human resume flows,
2. branch-aware merge strategies,
3. multi-subscriber fan-out infrastructure,
4. eventless automatic transitions triggered from checkpoint completion,
5. gap-less checkpoint commit progression,
6. replay/fork from checkpoint for proof reproduction and doctor/debug use,
7. pending checkpoint writes after partial failure.

## 5. Mapping To Existing VIDA Surfaces

1. `task_lifecycle` -> closure/status projections
2. `execution_plan` -> resume/checkpoint projections
3. `route_progression` -> lane status, assignment visibility, escalation visibility
4. `coach_lifecycle` -> formative-review wait projection
5. `verification_lifecycle` -> aggregation and proof-coverage projection
6. `approval_lifecycle` -> governance wait gateway
7. `boot_migration_gate` -> boot checkpoint and doctor visibility
8. `run_graph` -> durable resumability ledger for operator projections
9. future gateway handle index -> correlation-based resume targeting
10. future machine lint law -> static validation of checkpoint/gateway graph correctness
11. future checkpoint commit/replay lineage -> grouped checkpoint progression and replay safety

## 6. Invariants

1. root `vida/config` remains product law
2. framework/runtime helpers such as `docs/framework/**` and the active TaskFlow runtime-family implementation surfaces may implement adapters only
3. projection output must be derivable from canonical sources plus durable runtime ledgers
4. listener metadata must never replace receipts
5. checkpoint hints must never rewrite route or policy law
6. `coach` and `verification` remain separate even when both emit projections or checkpoint hints
7. grouped projections may share a listener boundary, but must not collapse distinct canonical entities into one state store

-----
artifact_path: product/spec/projection-listener-checkpoint-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: docs/product/spec/projection-listener-checkpoint-model.md
created_at: '2026-03-09T12:00:46+02:00'
updated_at: '2026-03-12T07:58:34+02:00'
changelog_ref: projection-listener-checkpoint-model.changelog.jsonl
