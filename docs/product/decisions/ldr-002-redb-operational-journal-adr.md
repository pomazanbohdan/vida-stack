# LDR-002 redb Operational Journal ADR

Status: accepted for spike-backed local runtime journal adoption.

Date: 2026-06-22

## Decision

Use `redb` as the embedded local operational journal for VIDA runtime operation history, replay, idempotency records, retryable effect outbox records, projection cursors, and artifact indexes after an operation-specific cutover.

`redb` is not the canonical TaskFlow, DocFlow, or project-state authority. SurrealDB and SurrealKV remain canonical for those stores until a separate migration explicitly moves an operation boundary.

## Dependency Evidence

`cargo info redb` on 2026-06-22 reported:

- version: `4.1.0`
- license: `MIT OR Apache-2.0`
- rust-version: `1.89`
- repository: `https://github.com/cberner/redb`
- features used by this spike: default only

This fits the current Rust toolchain direction for VIDA runtime spike work. The dependency is added to workspace dependencies so the intended production boundary is explicit, while the executable proof remains isolated under `spikes/local-durable-runtime`.

## Ownership Boundary

redb owns:

- append-only local operation event journal
- operation expected-version checks
- operation replay streams
- idempotency key to response records
- retryable local effect outbox
- projection cursor and artifact-index records

SurrealDB and SurrealKV own:

- canonical TaskFlow state
- canonical DocFlow state
- project activation and framework state
- existing state-store projections until a named operation cutover is completed

Large artifact bodies are not stored in redb. redb may store content-addressed references and small indexes.

## Rejected Alternatives

- `sqlite-es`: rejected because it overlaps with the existing SurrealDB/SurrealKV state authority while adding a second SQL/event-store concern.
- ad hoc SQLite: rejected because it creates another broad embedded state owner without a narrow journal boundary.
- direct SurrealKV bypass: rejected because it would mix runtime operation replay with canonical project-state persistence.
- full SurrealDB replacement: rejected as out of scope for local operational journaling.
- large artifact bodies inside redb: rejected to preserve small journal files and content-addressed artifact ownership.

## Acceptance Evidence

Spike crate: `spikes/local-durable-runtime`

Proof commands:

```powershell
cargo test --manifest-path spikes/local-durable-runtime/Cargo.toml --locked
```

Covered behavior:

- event append with expected version
- expected-version mismatch fails closed
- global replay rebuilds ordered events
- process restart after append before effect execution preserves journal and pending outbox
- duplicate idempotency key with same payload returns previous response
- duplicate idempotency key with different payload fails closed
- retryable effect remains pending after reopen and can be marked complete

## Follow-Up Cutover Criteria

Before production cutover, create a runtime task that:

- maps each operation command to a versioned envelope
- defines redb table names and key encoding as stable contracts
- proves Windows, Linux, and macOS file locking behavior in CI
- documents backup, repair, and projection rebuild operations
- adds public CLI proof for default compact output and explicit JSON output

-----
artifact_path: product/decisions/ldr-002-redb-operational-journal-adr
artifact_type: architecture_decision_record
artifact_version: '1'
artifact_revision: 2026-06-22
schema_version: '1'
status: canonical
source_path: docs/product/decisions/ldr-002-redb-operational-journal-adr.md
created_at: 2026-06-22T00:00:00+03:00
updated_at: 2026-06-22T00:00:00+03:00
changelog_ref: ldr-002-redb-operational-journal-adr.changelog.jsonl
