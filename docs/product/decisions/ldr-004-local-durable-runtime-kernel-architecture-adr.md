# LDR-004 Local Durable Runtime Kernel Architecture ADR

Status: accepted for LDRK architecture and migration law freeze.

## Context

Task `ldr-004` freezes the Local Durable Runtime Kernel architecture before implementation begins.

The preceding LDRK decisions provide the inputs:

1. `ldr-001` produced the baseline inventory: 280664 targeted production LOC, 1566 direct mutation candidates, 1596 duplicate classifier candidates, 160 canonical CLI leaf command candidates, and 527 command-specific option candidates.
2. `ldr-002` selected redb as the local `OperationalJournal` implementation for the crash-reopen/idempotency/outbox spike.
3. `ldr-003` mapped current operator commands to six canonical command families and required host bridge completion to become one structured outcome payload.

## Decision

Accept `docs/product/spec/local-durable-runtime-kernel-architecture-and-migration-law.md` as product law for LDRK implementation.

The event journal is the canonical workflow mutation truth.

`VidaCommandEnvelope` is the only semantic mutation entrypoint after operation cutover.

Task/run/lane/dispatch/continuation/status/file surfaces are projections or artifacts unless the architecture law classifies a surface as aggregate state.

Legacy writers are allowed only before the matching operation slice cuts over or during explicit shadow comparison.

After a slice cuts over, the legacy writer for that operation must not return.

## Approved Owner Boundaries

| Boundary | Owner |
|---|---|
| command semantics | `VidaCommandEnvelope` operation registry |
| command response semantics | `VidaCommandResponse` |
| event append/idempotency/outbox | `OperationalJournal` |
| task lifecycle | Task aggregate |
| run workflow | Run aggregate |
| lane lifecycle and takeover | Lane aggregate |
| dispatch packet/receipt lifecycle | Dispatch aggregate plus artifact index |
| host bridge completion | typed `CompletionOutcome` payload |
| effect execution | outbox/effect aggregate |
| repair and migration | `vida repair` operation family |
| projections | projection runners with checkpoints |

## Approved Migration Gates

1. Baseline and decision gates must close before implementation slices.
2. Each operation slice must define entry gate, exit gate, shadow-read posture, and cutover receipt.
3. A slice may not keep permanent dual authority.
4. A slice may not close on unit tests alone when it changes runtime authority.
5. A slice must include public operator proof for changed command/projection/restart/failure behavior.
6. External durable engines must implement storage-neutral `RuntimeEngine` and `OperationalJournal` ports and pass the same conformance suite as the local redb engine.

## Rejected Alternatives

| Alternative | Rejection reason |
|---|---|
| Keep TaskFlow DB rows as canonical mutation truth | Preserves duplicated direct writers and prevents deterministic rebuild from a journal. |
| Let run graph, lane packets, and dispatch receipts remain independent authorities | Keeps the current drift class where projections and receipts contradict each other. |
| Add Restate SDK types directly to domain crates | Couples domain semantics to one durable engine and blocks local/external conformance parity. |
| Keep host bridge completion as independent `decision`/`verdict`/`blocker` flags | Preserves the exact semantic duplication `ldr-003` eliminated. |
| Use redb as a task database replacement | Confuses the local operational journal with canonical TaskFlow/DocFlow/project authority during migration. |
| Allow long-lived shadow dual writes | Violates the one-way cutover rule and makes rollback/repair ownership ambiguous. |

## Consequences

1. LDRK implementation must introduce operation events and projection rebuild paths before removing legacy writers.
2. Each command family migration must name the aggregate, events, projection outputs, and proof targets.
3. Existing state files/tables must be classified as aggregate state, projection, artifact, cache, or migration source.
4. Documentation, TaskFlow proof, and release validation are closure requirements for each authority-changing slice.
5. Runtime defects that expose projection/receipt disagreement should be fixed at the shared operation/journal/projection boundary, not patched per surface.

## Proof

1. Accepted spec: `docs/product/spec/local-durable-runtime-kernel-architecture-and-migration-law.md`.
2. Accepted ADR: this file.
3. Dependency graph validation: `vida task validate-graph`.
4. DocFlow validation:
   - `vida docflow check-file --path docs/product/spec/local-durable-runtime-kernel-architecture-and-migration-law.md`
   - `vida docflow check-file --path docs/product/decisions/ldr-004-local-durable-runtime-kernel-architecture-adr.md`

-----
artifact_path: product/decisions/ldr-004-local-durable-runtime-kernel-architecture-adr
artifact_type: product_decision
artifact_version: "1"
source_path: docs/product/decisions/ldr-004-local-durable-runtime-kernel-architecture-adr.md
created_at: 2026-06-22T00:00:00+03:00
updated_at: 2026-06-22T00:00:00+03:00
changelog_ref: ldr-004-local-durable-runtime-kernel-architecture-adr.changelog.jsonl
