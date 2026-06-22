# Local Durable Runtime Kernel Architecture And Migration Law

Status: accepted product law for TaskFlow task `ldr-004`.

Purpose: freeze the Local Durable Runtime Kernel architecture before implementation so LDRK work does not create another partial service, state store, or state machine beside the existing VIDA runtime surfaces.

## Architecture Decision

1. The canonical workflow mutation truth is the versioned event journal.
2. Every mutation enters through `VidaCommandEnvelope` and returns `VidaCommandResponse`.
3. The local default journal is the redb `OperationalJournal` selected by `ldr-002`.
4. Task records, run graphs, lane packets, dispatch receipts, continuation bindings, status summaries, and files are projections or artifacts unless this law explicitly classifies them as aggregate state.
5. Legacy writers may run only before the operation slice cuts over or as read/compare shadow paths.
6. After an operation slice cuts over, the legacy writer for that operation must not return.
7. Domain crates must not import Restate SDK types, redb tables, SurrealDB records, Effectum tasks, or host-tool bridge artifact paths.
8. External durable engines must implement storage-neutral `RuntimeEngine` and `OperationalJournal` ports and pass the same conformance suite as the local redb engine.

## State Classification

| Surface | Classification | Owner | Migration rule |
|---|---|---|---|
| `VidaCommandEnvelope` | command input contract | command pipeline | Stable semantic entrypoint for CLI, TUI, service, and host adapters. |
| `VidaCommandResponse` | command result contract | command pipeline | Carries receipt refs, outcome, projection hints, and operator-safe messages. |
| redb operational journal | aggregate event journal | `OperationalJournal` port | Canonical local mutation log after cutover. |
| task records | aggregate state before cutover; projection after cutover | Task aggregate | Rebuilt from task events once task operations cut over. |
| run graph state | projection before cutover; projection after cutover | Run aggregate projection | No direct writer after run workflow operation cutover. |
| lane packets | artifact | lane/dispatch projection owner | Content-addressed or receipt-addressed artifact, never semantic truth. |
| dispatch receipts | artifact plus projection checkpoint | dispatch projection owner | Rebuilt/indexed from journal events; retained for operator evidence. |
| host bridge request/result/receipt files | artifact | host bridge adapter boundary | Completion outcome becomes one typed payload event. |
| continuation bindings | projection | continuation projection owner | Rebuilt from run/task/lane events. |
| status and doctor summaries | projection/cache | status projection owner | Never own mutation truth. |
| DocFlow files | artifact/source document | DocFlow owner | Remain source docs, linked by receipts and map/catalog entries. |
| release archives and checksums | artifact | release surface | Built from committed source; not runtime state truth. |
| projection checkpoints | cache/checkpoint | projection runner | Can be rebuilt from journal and artifact indexes. |
| migration snapshots | migration source | migration runner | Read-once or shadow-compare input with explicit cutover receipt. |

## Command Pipeline

1. CLI, TUI, service, and host adapters normalize requests into `VidaCommandEnvelope`.
2. The command pipeline validates identity, project context, authorization, idempotency, schema version, and operation metadata.
3. Command handlers load aggregate state through storage-neutral ports.
4. Commands append events and outbox entries atomically through `OperationalJournal`.
5. Effects are scheduled from the outbox and completed with receipt-backed events.
6. Projections consume events and update task/run/lane/status/file views.
7. Operators read projections through `vida get`, dry-run mutations through `vida plan`, mutate through `vida apply`, stream through `vida watch`, manage service lifecycle through `vida service`, and recover through `vida repair`.

## Aggregate Boundaries

| Aggregate | Owns | Does not own |
|---|---|---|
| Task aggregate | task lifecycle, dependencies, proof evidence, closure admission | lane execution artifacts, release archives |
| Run aggregate | run graph state, active node, checkpoint, resume state | task backlog truth |
| Lane aggregate | lane lifecycle, takeover/supersede state, execution receipts | host adapter implementation details |
| Dispatch aggregate | packet materialization, backend/carrier selection receipt, dispatch result | carrier-specific execution internals |
| Host bridge aggregate | typed completion outcome, host-agent provenance, artifact refs | independent decision/verdict/blocker CLI fields |
| Projection aggregate | checkpoint, failure journal, rebuild cursor | command authority |
| Effect aggregate | outbox item, effect lease, retry/failure status | domain command semantics |
| Repair aggregate | migration/cutover/rollback receipts and guarded repair actions | silent cleanup or state hacks |

## Consistency Levels

| Level | Use | Rule |
|---|---|---|
| strong command transaction | event append, idempotency, outbox write | Must be atomic in the local journal. |
| monotonic projection | task/run/lane/status views | May lag but must expose cursor/checkpoint evidence. |
| eventual artifact index | packet, receipt, browser proof, release file lookup | Must preserve content identity and source receipt refs. |
| shadow comparison | pre-cutover legacy parity | Cannot authorize permanent dual writes. |
| repair transaction | migration and emergency recovery | Requires explicit repair operation and receipt. |

## Effect Lifecycle

1. `Requested`: command accepted and event appended.
2. `OutboxPending`: side effect recorded but not executed.
3. `Leased`: executor owns the effect for a bounded lease.
4. `Completed`: receipt-backed completion event appended.
5. `FailedRetryable`: retry event appended with next attempt policy.
6. `FailedTerminal`: terminal failure event appended with operator-safe next action.
7. `Compensated`: repair event appended when a completed effect requires explicit compensation.

## Migration Phases

| Phase | Entry gate | Exit gate |
|---|---|---|
| 0 baseline and decisions | `ldr-001`, `ldr-002`, and `ldr-003` closed | This spec and the paired ADR are accepted and registered. |
| 1 host bridge completion slice | typed completion outcome schema exists | host bridge completion writes only journal events plus artifacts. |
| 2 run workflow advance slice | run aggregate command envelope exists | run graph projections rebuild from journal events. |
| 3 task/proof/closure slice | task aggregate command envelope exists | task records and proof evidence are projection-backed. |
| 4 lane/dispatch slice | dispatch aggregate and lane aggregate ports exist | lane packets and dispatch receipts are artifact/projection outputs. |
| 5 status/doctor/diagnostics slice | projection checkpoint model exists | status surfaces read projections with cursor evidence only. |
| 6 service/TUI/external engine slice | `RuntimeEngine` conformance suite exists | local and external engines pass the same command/effect tests. |

## Semantic Ownership Map

| Semantic area | Owner |
|---|---|
| command semantics | `VidaCommandEnvelope` operation registry |
| event semantics | domain event catalog behind `OperationalJournal` |
| projection semantics | projection runners and checkpoint model |
| effect semantics | outbox/effect aggregate |
| repair semantics | `vida repair` operation family and repair aggregate |
| host bridge completion semantics | `CompletionOutcome` typed payload |
| authorization semantics | Cedar policy adapter boundary |
| adapter semantics | CLI/TUI/service/host adapter normalization only |

## Quantitative Targets

| Metric | Baseline | Target |
|---|---:|---:|
| targeted production LOC | 280664 | reduce by at least 35 percent |
| duplicate classifier candidates | 1596 | reduce by at least 70 percent |
| direct surface mutation candidates | 1566 | reduce to 0 after operation cutover |
| canonical CLI leaf command candidates | 160 | reduce by at least 40 percent; `ldr-003` target is 6 families |
| command-specific option candidates | 527 | reduce by at least 50 percent; `ldr-003` target is 0 command-specific options |

## Adapter Boundary

1. CLI, TUI, service, and host adapters may parse, render, transport, and collect operator input.
2. Adapters must not decide domain state transitions after cutover.
3. Host bridge completion must submit one structured `CompletionOutcome` payload.
4. Independent `decision`, `verdict`, `blocker`, `rework_target`, and `allowed_next_node` flag families are compatibility inputs only until the host bridge slice cuts over.
5. Restate, SurrealDB, Effectum, tarpc, JSON-RPC, or filesystem-specific fields cannot leak into domain events.

## Repository Validation

This law is validated by:

1. DocFlow checks for this spec and the paired ADR.
2. Registration in `current-spec-map.md` and `current-spec-catalog.md`.
3. TaskFlow graph validation after proof evidence is attached.
4. Existing LDRK baseline, redb ADR, and operation catalog artifacts:
   - `docs/product/spec/ldrk-baseline/baseline.json`
   - `docs/product/decisions/ldr-002-redb-operational-journal-adr.md`
   - `docs/product/spec/ldrk-operation-catalog/operation-cli-map.json`

-----
artifact_path: product/spec/local-durable-runtime-kernel-architecture-and-migration-law
artifact_type: product_spec
artifact_version: "1"
source_path: docs/product/spec/local-durable-runtime-kernel-architecture-and-migration-law.md
created_at: 2026-06-22T00:00:00+03:00
updated_at: 2026-06-22T00:00:00+03:00
changelog_ref: current-spec-catalog.changelog.jsonl
