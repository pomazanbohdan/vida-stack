# VIDA Checkpoint Commit And Replay Lineage

Status: draft `v1` bounded runtime artifact

Revision: `2026-03-09`

Purpose: define the future lawful shape for checkpoint commit progression, delayed checkpoint writes, replay, and fork lineage without confusing those runtime concerns with canonical state or receipt history.

## 1. Scope

This artifact defines:

1. checkpoint hint vs checkpoint commit,
2. grouped projection checkpoint advancement,
3. delayed checkpoint write handling,
4. replay and fork lineage boundaries,
5. idempotency expectations when reprocessing occurs.

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

### 2.3 Replay

Replay is a runtime action that reprocesses historical events, receipts, or proof-linked snapshots in order to rebuild projections or reproduce evidence.

Rule:

1. replay rebuilds derived surfaces,
2. replay must not rewrite canonical history,
3. replay must have explicit lineage distinct from the original live pass.

### 2.4 Fork

Fork is a replay-derived alternate runtime branch used for debugging, proving, or controlled recovery.

Rule:

1. forks create new runtime lineage,
2. forks do not replace the original live lineage,
3. fork output must be clearly marked as derived/debug unless explicitly promoted by future law.

## 3. Future Direction

The kernel accepts as future direction:

1. gap-less checkpoint commit progression,
2. grouped projection checkpoint advancement,
3. delayed checkpoint writes after successful handler execution,
4. idempotent handler requirement when duplicate delivery is possible,
5. replay from checkpoint for projection rebuild,
6. fork-from-checkpoint for doctor/debug/proof reproduction.

## 4. Candidate Laws

### 4.1 Gap-Less Commit

When events are processed out of order or grouped across handlers, the committed checkpoint should advance only to the last gap-less known safe position.

### 4.2 Grouped Projection Advancement

If multiple projections must remain consistent, their shared checkpoint group should advance together or remain uncommitted.

### 4.3 Delayed Write Safety

If handler execution succeeds but checkpoint persistence is delayed or partially fails:

1. retry must be explicit,
2. duplicate event delivery must be expected,
3. handlers must remain idempotent.

### 4.4 Replay Lineage

Replay runs should carry:

1. `lineage_kind`
2. `origin_checkpoint_ref`
3. `replay_scope`
4. `fork_parent` when applicable

## 5. Mapping To Existing VIDA Surfaces

1. `projection-listener-checkpoint-kernel` -> checkpoint hints and grouped projections
2. `receipt-proof-taxonomy` -> checkpoint receipts and replay-linked proof artifacts
3. `execution_plan` -> pending checkpoint writes and resumability posture
4. `route_progression` -> replay of route-facing projections only, not route law itself
5. `run_graph` -> practical runtime resume cursor source in `vida-v0`

## 6. Invariants

1. root `vida/config` remains product law
2. checkpoint commit remains runtime-owned until explicitly promoted
3. replay/fork never rewrites canonical receipts
4. grouped projection consistency must not collapse entity ownership
5. duplicate delivery safety is mandatory once delayed checkpoint commits are introduced

-----
artifact_path: product/spec/checkpoint-commit-and-replay-lineage
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/product/spec/checkpoint-commit-and-replay-lineage.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-09T20:28:59+02:00
changelog_ref: checkpoint-commit-and-replay-lineage.changelog.jsonl
