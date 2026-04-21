# VIDA Checkpoint Commit And Replay Model

Status: active Release-1 implementation law

Revision: `2026-04-03`

Purpose: define the lawful shape for checkpoint commit progression, resumability capsules, delayed checkpoint writes, replay, and fork lineage without confusing those runtime concerns with canonical state or receipt history.

## 1. Scope

This artifact defines:

1. checkpoint hint vs checkpoint commit,
2. grouped projection checkpoint advancement,
3. delayed checkpoint write handling,
4. resumability capsule boundaries,
5. replay and fork lineage boundaries,
6. idempotency expectations when reprocessing occurs,
7. bounded continuation packet expectations for resumable execution.

It does not define:

1. a storage-specific checkpoint backend,
2. vendor thread identity,
3. mutation of canonical state during replay,
4. silent rewriting of receipt history.

## 2. Core Distinction

### 2.1 Checkpoint Hint

A checkpoint hint is a derived runtime suggestion that a resumability boundary should exist.

### 2.2 Checkpoint Commit

A checkpoint commit is the durable write of a cursor or resumability position for a listener, projection group, or gateway boundary.

Rule:

1. checkpoint commit is runtime-owned,
2. checkpoint commit may be receipt-backed when decision-relevant,
3. checkpoint commit does not itself mutate canonical machine state.

### 2.3 Resumability Capsule

A resumability capsule is the smallest bounded continuation summary needed to resume one active runtime path.

Rule:

1. a resumability capsule is not a substitute for checkpoint-commit lineage,
2. latest resumability state is a derived runtime surface, not the full replay history,
3. recovery summaries may read from resumability state, but they must not claim checkpoint/replay closure without persisted checkpoint lineage.

### 2.4 Replay

Replay is a runtime action that reprocesses historical events, receipts, or proof-linked snapshots in order to rebuild projections or reproduce evidence.

Rule:

1. replay rebuilds derived surfaces,
2. replay must not rewrite canonical history,
3. replay must have explicit lineage distinct from the original live pass.

### 2.5 Fork

Fork is a replay-derived alternate runtime branch used for debugging, proving, or controlled recovery.

Rule:

1. forks create new runtime lineage,
2. forks do not replace the original live lineage,
3. fork output must be clearly marked as derived/debug unless explicitly promoted by future law.

## 3. Release-1 Required Shape

Release 1 requires all of:

1. gap-less checkpoint commit progression,
2. grouped projection checkpoint advancement,
3. delayed checkpoint writes after successful handler execution,
4. idempotent handler requirement when duplicate delivery is possible,
5. replay from checkpoint for projection rebuild,
6. fork-from-checkpoint for doctor/debug/proof reproduction.
7. durable execution semantics that distinguish replay-safe from non-replay-safe side effects.
8. explicit distinction between checkpoint-commit artifacts and latest resumability summaries.

## 4. Required Laws

### 4.1 Gap-Less Commit

When events are processed out of order or grouped across handlers, the committed checkpoint must advance only to the last gap-less known safe position.

### 4.2 Grouped Projection Advancement

If multiple projections must remain consistent, their shared checkpoint group must advance together or remain uncommitted.

Minimum persisted checkpoint-commit fields:

1. `projector_id`
2. `checkpoint_group`
3. `last_gapless_position`
4. `updated_at`

Replay-required lineage fields:

1. `lineage_kind`
2. `origin_checkpoint_ref`
3. `replay_scope`
4. `fork_parent` when applicable

Artifact rule:

1. the checkpoint-commit artifact must be distinct from the resumability capsule,
2. projection/read-model summaries must be derived from persisted checkpoint records rather than replacing them.

Current bounded TaskFlow mapping for the active `run_graph` family:

1. the first distinct checkpoint artifact is an append-only `projection_checkpoint_record` emitted from grouped `run_graph` status persistence rather than from the resumability capsule,
2. `projector_id` is the stable family-owned projector id `taskflow.run_graph.status_projection`,
3. `checkpoint_group` is the per-run grouped projection boundary `run_graph_status:{run_id}`,
4. `last_gapless_position` is the shared grouped-write commit token for that status projection pass and currently reuses the persisted `updated_at` value rather than the resumability `resume_target`,
5. `origin_checkpoint_ref` for this bounded slice is `{run_id}:{checkpoint_kind}:{resume_target}`,
6. `replay_scope` is `live_pass` and `fork_parent` remains empty unless an explicit replay/fork surface is activated,
7. when `checkpoint_kind = none`, no projection-checkpoint record is emitted because placeholder resumability state is not lawful checkpoint lineage.

### 4.3 Delayed Write Safety

If handler execution succeeds but checkpoint persistence is delayed or partially fails:

1. retry must be explicit,
2. duplicate event delivery must be expected,
3. handlers must remain idempotent.

### 4.4 Replay Lineage

Replay runs must carry:

1. `lineage_kind`
2. `origin_checkpoint_ref`
3. `replay_scope`
4. `fork_parent` when applicable

### 4.5 Bounded Continuation Packet

A resumable implementation checkpoint should carry the smallest lawful execution continuation packet rather than relying on transcript reconstruction.

Minimum continuation expectations:

1. `delivery_task_id`
2. `execution_block_id`
3. `resume_hint`
4. current verification or `review_pool` target
5. current control counters and effective route control limits when the route is budgeted or stall-limited

Rule:

1. recovery should resume from the smallest lawful bounded node,
2. exhausted control limits should force replan/escalation rather than blind resume,
3. replay/fork may reuse the continuation packet, but must preserve derived lineage.
4. a resumability capsule without checkpoint lineage is insufficient proof for replay closure.

### 4.6 Append-Evidence Transition Rule

1. latest status summaries must not replace append-evidence transition history,
2. checkpoint/recovery law must be able to point to persisted receipt or transition records for the active run,
3. upserted latest-row summaries are allowed as query surfaces, but not as the only durable lineage.

## 5. Mapping To Existing VIDA Surfaces

1. `projection-listener-checkpoint-kernel` -> checkpoint hints and grouped projections
2. `receipt-proof-taxonomy` -> checkpoint receipts and replay-linked proof artifacts
3. `execution_plan` -> pending checkpoint writes and resumability posture
4. `route_progression` -> replay of route-facing projections only, not route law itself
5. `run_graph` -> practical runtime resume cursor source in the active TaskFlow runtime family
6. `task_state_telemetry` -> checkpoint-visible continuation packet and compact hydration contract

Mapping rule:

1. `resumability_capsule` is one runtime continuation surface and must not be treated as the checkpoint-commit record itself,
2. latest `run_graph` recovery/checkpoint summaries are query surfaces and must not be treated as full replay lineage,
3. proof closure requires distinct checkpoint/replay artifacts when replay lineage is part of the release claim,
4. the current bounded `run_graph` implementation therefore keeps append-only `projection_checkpoint_record` and replay-lineage receipts as durable artifacts while leaving latest recovery/checkpoint summaries as derived read surfaces only.

## 6. Invariants

1. root `vida/config` remains product law
2. checkpoint commit remains runtime-owned until explicitly promoted
3. replay/fork never rewrites canonical receipts
4. grouped projection consistency must not collapse entity ownership
5. duplicate delivery safety is mandatory once delayed checkpoint commits are introduced
6. resumability must not require re-deriving bounded execution intent from chat history.
7. replay/fork lineage must remain visible and queryable,
8. checkpoint closure must not be claimed from latest resumability summaries alone.

LangGraph alignment note:

1. this model aligns with LangGraph-style durable execution and replay lineage,
2. VIDA should borrow the resumability and replay discipline, not LangGraph-owned thread identity or framework APIs.

-----
artifact_path: product/spec/checkpoint-commit-and-replay-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-04-21'
schema_version: '1'
status: canonical
source_path: docs/product/spec/checkpoint-commit-and-replay-model.md
created_at: '2026-03-09T12:00:46+02:00'
updated_at: '2026-04-21T18:30:00+03:00'
changelog_ref: checkpoint-commit-and-replay-model.changelog.jsonl
