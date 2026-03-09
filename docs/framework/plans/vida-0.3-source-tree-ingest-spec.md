# VIDA 0.3 Source Tree Ingest Spec

Purpose: define the Git-resident source tree layout for the three durable data slices and the boot-time ingest path that materializes them into the DB-owned runtime.

Status: canonical `0.3` spec artifact for the direct `1.0` program.

Date: 2026-03-08

---

## 1. Executive Decision

Direct `1.0` keeps human-authored source artifacts in Git, but runtime truth still lives in the DB.

The canonical model is:

1. Git holds source trees,
2. boot/migration ingests those trees into the DB,
3. the DB becomes the authoritative runtime substrate,
4. exports are emitted from the DB.

Compact rule:

`Git is the editable source tree; DB is the authoritative runtime mirror`

---

## 2. Three Source Trees

The repository must support three top-level source trees:

1. `project slice source tree`
2. `instruction slice source tree`
3. `framework slice source tree`

These trees are separate because they have different ownership, mutability, and migration semantics.

Boot-ingest rule:

1. `instruction slice source tree` is a runtime-owned boot-ingest source,
2. `framework slice source tree` is a runtime-owned boot-ingest source,
3. `project slice source tree` is a sync/export surface and is not required for framework boot.

Runtime nuance:

1. epic/task/TaskFlow/tracing data belongs to `workflow_runtime` in the DB, not to any Git source tree,
2. worker operational data belongs to `worker_runtime` in the DB, not to the external sync trees.

---

## 3. Source Tree Requirements

Each source tree must support:

1. nested directories,
2. markdown files,
3. structured metadata,
4. stable identifiers,
5. version markers where needed,
6. change-detection hashes,
7. allowlisted file kinds where needed.

The source-tree layer must not require flat-file layouts only.

---

## 4. Instruction Tree Requirements

The instruction source tree must support:

1. `Agent Definition` sources,
2. `Instruction Contract` sources,
3. `Prompt Template Configuration` sources,
4. sidecar sources,
5. dependency declarations between instruction artifacts,
6. immutable framework-bundled sources,
7. migration/version metadata.

Nested tree rule:

1. category and subcategory directory nesting is allowed,
2. ingest must preserve logical hierarchy metadata,
3. runtime lookup must not depend on the raw path as the only identifier.

---

## 5. Boot-Time Ingest

On boot the binary must be able to:

1. scan the configured source trees,
2. read metadata and content,
3. compute stable change hashes,
4. compare source versions against DB versions,
5. ingest new or changed source artifacts,
6. supersede older immutable framework-bundled versions by append-and-replace semantics,
7. record ingest receipts,
8. fail closed on malformed, ambiguous, or incompatible source artifacts.

---

## 6. Change Detection

Minimum ingest metadata per source artifact:

1. stable artifact id,
2. source path,
3. logical slice,
4. artifact kind,
5. content hash,
6. version,
7. ingest status,
8. last ingested product version.

Rule:

1. no source artifact should be reloaded blindly on every boot when an equivalent hash/version is already present,
2. changed source artifacts must trigger deterministic ingest or migration.

---

## 7. Ordering Rule

This substrate must be implemented before deep protocol/product behavior that depends on these artifacts.

Reason:

1. protocol runtime cannot be stable if its durable source/ingest substrate is still undefined,
2. sidecar and versioning semantics depend on source-tree ingest metadata,
3. DB-first runtime ownership depends on this ingest path.

Practical sequencing rule:

1. source-tree ingest substrate before rich protocol runtime,
2. instruction-memory and framework-memory boot-ingest before advanced instruction resolver behavior,
3. project-memory sync/export after DB-owned ingest/runtime substrate exists,
4. export protocol after DB-owned ingest/runtime substrate exists.

---

## 8. Downstream Consequences

This spec requires:

1. a configurable mapping from Git source trees into DB slices,
2. boot-time ingest and migration receipts,
3. schema support for source metadata and logical hierarchy,
4. implementation tasks ahead of high-level protocol runtime completion.
-----
artifact_path: framework/plans/vida-0.3-source-tree-ingest-spec
artifact_type: plan
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/plans/vida-0.3-source-tree-ingest-spec.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: vida-0.3-source-tree-ingest-spec.changelog.jsonl
P26-03-09T21: 44:13Z
