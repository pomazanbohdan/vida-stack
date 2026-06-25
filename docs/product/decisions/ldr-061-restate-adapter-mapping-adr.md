# LDR-061 Restate Adapter Mapping ADR

Status: accepted for prototype proof; no production cutover.

## Context

Task `ldr-061` validates that an external durable runtime can map to the
storage-neutral VIDA runtime contract without leaking engine-specific SDK types
into domain crates or command envelopes.

The accepted LDRK architecture keeps `RuntimeEngine`, `VidaCommandEnvelope`,
`VidaCommandResponse`, and the runtime schema registry as the shared contract
between local and external engines.

## Decision

Prototype the Restate mapping as an adapter boundary only.

The adapter maps a `VidaCommandEnvelope` into a small Restate-like invocation
model with service key, handler name, idempotency key, and JSON payload. The
prototype intentionally avoids a Restate SDK dependency so the proof remains
about VIDA contract shape, schema equality, and capability negotiation rather
than network execution.

The production domain crates remain Restate-free. A future production adapter
may introduce an SDK dependency only inside an external engine crate after the
same contract tests pass.

## Mapping

| VIDA contract field | Restate-like adapter field | Rule |
|---|---|---|
| `operation` | `handler` | Dot-separated operation id becomes a stable handler path. |
| `session_id` | `service_key` | Session id is the keyed serialization boundary. |
| `request_id` | `request_id` | Preserved for trace and response correlation. |
| `idempotency_key` | `idempotency_key` | Required for replay-safe mutation dispatch. |
| `payload` | `payload` | Passed as JSON without engine-specific fields. |
| `project_ref` | `project_ref` | Preserved as optional routing context. |
| `correlation` | `correlation` | Preserved as opaque JSON. |

## Gap Matrix

| Concern | Prototype posture | Production gap |
|---|---|---|
| Durable timers | Advertised as adapter-capable, not executed. | Add SDK-backed timer scheduling and replay proof. |
| Keyed serialization | Session id maps to service key. | Prove key collision and migration behavior. |
| Signals | Represented as an advertised external capability. | Add typed signal payload and failure proof. |
| Event export | Uses VIDA schema registry snapshot. | Add external event export checkpoint proof. |
| Strong reads | Uses VIDA query contract only. | Add read-after-write and replay consistency proof. |
| SDK dependency | Excluded from prototype. | Keep SDK dependency isolated from domain crates. |

## Rejected Alternatives

| Alternative | Rejection reason |
|---|---|
| Put Restate SDK types in `vida-contracts` | Violates storage-neutral runtime contracts. |
| Cut over production execution in this task | `ldr-061` is a prototype and proof task only. |
| Fork envelope schemas for Restate | Would break local/external conformance parity. |

## Proof

1. Mapping ADR and gap matrix: this file.
2. Prototype integration tests: `cargo test --manifest-path spikes/vida-runtime-restate/Cargo.toml`.
3. Schema equality test: the same spike test suite compares adapter schema export
   with `vida_contracts::runtime_envelope_schema_bundle_json()`.

-----
artifact_path: product/decisions/ldr-061-restate-adapter-mapping-adr
artifact_type: product_decision
artifact_version: "1"
source_path: docs/product/decisions/ldr-061-restate-adapter-mapping-adr.md
created_at: 2026-06-24T00:00:00+03:00
updated_at: 2026-06-24T00:00:00+03:00
changelog_ref: ldr-061-restate-adapter-mapping-adr.changelog.jsonl
