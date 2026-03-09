# VIDA 0.3 Storage Kernel Spec

Purpose: freeze the authoritative embedded storage decision for direct `1.0`, define the allowed storage substrate, and prevent legacy carrier/runtime topology from reappearing as product law.

Status: canonical `0.3` spec artifact for the direct `1.0` program.

Date: 2026-03-08

---

## 1. Executive Decision

Direct `1.0` uses:

1. embedded `SurrealDB` as the product storage engine,
2. `kv-surrealkv` as the authoritative local backend,
3. one binary-owned local database opened directly by the Rust runtime,
4. fail-closed startup when the storage engine or required schema cannot be opened.

Direct `1.0` does **not** treat as product law:

1. `SQLite`,
2. `JSONL-first` task storage,
3. file-layout carriers such as `.beads/issues.jsonl`,
4. shell-era queue choreography as the long-term storage substrate,
5. alternate embedded backends unless a later higher-precedence spec explicitly replaces this decision.

Compact rule:

`store authoritative 1.0 state in embedded SurrealDB on kv-surrealkv; treat legacy carriers as migration inputs only`

---

## 2. Why This Spec Exists

The state, command, and migration specs freeze semantics and boundaries, but they intentionally left the exact product storage engine open.

That ambiguity is no longer useful because:

1. implementation planning now needs one canonical storage target,
2. migration work needs one destination substrate,
3. proof and conformance work need one real backend to validate,
4. leaving multiple backends implicit creates topology drift and reopens product law by accident.

---

## 3. Canonical Storage Decision

### 3.1 Engine

The embedded engine is `SurrealDB`.

### 3.2 Backend

The authoritative embedded backend is `kv-surrealkv`.

### 3.3 Runtime Posture

Storage is:

1. local-first,
2. embedded in the Rust binary,
3. opened without a separate daemon requirement,
4. owned by the binary runtime rather than by shell wrappers.

### 3.4 Product-Law Consequence

Any future implementation, migration, proof, or operator-visible storage behavior for direct `1.0` must assume:

1. `SurrealDB` record-oriented state,
2. `kv-surrealkv` local persistence,
3. binary-owned schema bootstrapping,
4. no product dependence on `SQLite` or JSONL carriers.

### 3.5 Runtime Hash Utility

For storage-adjacent runtime duties such as:

1. source-artifact content hashing,
2. instruction anchor hashing,
3. projection/integrity receipts,

the current canonical fast cryptographic hash choice is `BLAKE3`.

Rules:

1. `BLAKE3` is selected as the default fast hash for direct `1.0` because it is modern, cryptographic, and practical for high-frequency runtime hashing,
2. implementation may rely on its architecture-aware optimization path when building release binaries for supported targets,
3. this does not authorize generic hash-choice drift in code; replacement requires explicit library evaluation and spec update.

---

## 4. What The Storage Kernel Must Own

The storage kernel owns:

1. the authoritative embedded engine decision,
2. backend selection,
3. namespace/database boot contract,
4. schema-definition responsibility,
5. logical slice separation for project, instruction, and framework durable memory,
6. storage compatibility checks required at `vida boot`,
7. fail-closed storage-open and schema-open behavior.

The storage kernel does not yet fully define:

1. every table/record layout,
2. every index,
3. every migration routine,
4. vector-search adoption,
5. operator-facing query syntax.

---

## 5. Non-Goals

This spec does not:

1. define exact task/state record fields beyond what state/migration specs already own,
2. define exact SurrealQL for every command,
3. define vector or HNSW schemas,
4. define remote deployment or daemonized storage,
5. define alternate backend support.

---

## 6. Invariants

Direct `1.0` storage must preserve these invariants:

1. authoritative state is persisted in embedded `SurrealDB`,
2. the only canonical embedded backend is `kv-surrealkv`,
3. startup must fail closed if the backend cannot be opened or the schema contract is incompatible,
4. legacy JSONL, `.beads`, and `SQLite` surfaces are migration or diagnostic carriers only,
5. storage topology must not redefine state, command, or migration semantics,
6. no hidden multi-backend ambiguity is allowed in product law.
7. storage-adjacent runtime hashing must have one explicit canonical choice instead of ad hoc per-call crate selection.

---

## 7. Downstream Consequences

This spec now constrains:

1. the state kernel to target `SurrealDB` record/edge layout rather than file or SQLite carriers,
2. the migration kernel to translate legacy carriers into `SurrealDB` on `kv-surrealkv`,
3. the implementation roadmap to use the Rust `surrealdb` crate with `kv-surrealkv`,
4. the instruction kernel to persist its artifacts in a distinct instruction-memory slice,
5. parity and conformance proving to validate the chosen engine rather than backend-agnostic abstractions.
6. build/distribution work to support optimized binary artifacts per target architecture and not only one generic release flavor.

---

## 8. Immediate Follow-Through

The next lawful follow-through after this artifact is:

1. update active specs that still speak about storage as open or backend-agnostic,
2. remove lingering product-law ambiguity around `SQLite` and JSONL as future substrates,
3. keep legacy mentions only where they are explicitly migration-input or bridge-only references.
-----
artifact_path: framework/plans/vida-0.3-storage-kernel-spec
artifact_type: plan
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/plans/vida-0.3-storage-kernel-spec.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: vida-0.3-storage-kernel-spec.changelog.jsonl
P26-03-09T21: 44:13Z
