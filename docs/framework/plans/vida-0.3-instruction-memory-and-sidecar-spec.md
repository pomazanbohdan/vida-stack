# VIDA 0.3 Instruction Memory And Sidecar Spec

Purpose: define the durable storage model for instruction-system artifacts, separate instruction memory from project and framework memory, and freeze sidecar semantics for direct `1.0`.

Status: canonical `0.3` spec artifact for the direct `1.0` program.

Date: 2026-03-08

---

## 1. Executive Decision

Direct `1.0` uses a three-slice durable memory model:

1. `project_memory`
2. `instruction_memory`
3. `framework_memory`

These slices are distinct logical products even if they share one embedded `SurrealDB` engine.

Instruction-system artifacts must not be stored only as loose files, chat recall, or project-memory notes.

The instruction slice owns durable records for:

1. `Agent Definition`
2. `Instruction Contract`
3. `Prompt Template Configuration`
4. immutable framework-bundled instruction artifacts,
5. version metadata,
6. migration posture,
7. per-instruction sidecars.

Compact rule:

`keep project, instruction, and framework memory distinct; make instruction artifacts versioned, immutable where framework-owned, and sidecar-extendable`

---

## 2. Three-Slice Memory Model

### 2.1 `project_memory`

Purpose:

1. durable project/domain-specific memory,
2. project-owned facts that affect project execution,
3. memory queried through project/operator surfaces.

This slice must not become the storage home for framework-owned instruction law.

### 2.2 `instruction_memory`

Purpose:

1. durable storage for instruction-system artifacts,
2. versioned instruction assembly and lookup,
3. sidecar attachment and resolution,
4. startup compatibility and migration checks,
5. effective bundle composition from explicit records rather than transcript memory.

This slice is the canonical durable home for the instruction system.

### 2.3 `framework_memory`

Purpose:

1. durable lessons, corrections, and anomalies about framework operation,
2. framework self-improvement memory,
3. non-project, non-instruction behavioral learnings.

This slice remains distinct from instruction artifacts themselves.

---

## 3. Instruction Slice Contents

The instruction slice must persist at minimum:

1. `agent_definition`
2. `instruction_contract`
3. `prompt_template_configuration`
4. `instruction_bundle_version`
5. `instruction_schema_version`
6. `instruction_sidecar`
7. `instruction_migration_receipt`
8. `instruction_dependency_edge`
9. `instruction_projection_receipt`
10. `instruction_source_artifact`
11. `instruction_ingest_receipt`

Each instruction artifact must carry:

1. stable identifier,
2. artifact kind,
3. version,
4. ownership class,
5. mutability class,
6. source hash or equivalent integrity token,
7. activation metadata where applicable,
8. replacement/supersession metadata where applicable,
9. required follow-on instruction references where applicable.

Dependency rule:

1. instruction artifacts may declare mandatory follow-on reads,
2. those dependencies must be resolved automatically into the effective bundle,
3. optional follow-on reads remain trigger-bound and must stay separate from the mandatory chain.

Source-tree ingest rule:

1. editable instruction artifacts may originate from the Git-resident instruction source tree,
2. runtime use must happen from the ingested DB representation,
3. source path and hierarchy metadata must be preserved as ingest metadata, not as the only runtime key.

---

## 4. Framework-Bundled Immutable Instruction Artifacts

Framework-owned instruction artifacts are shipped with the product and stored in the instruction slice as immutable records.

Rules:

1. framework-bundled instruction artifacts must be loaded into the instruction slice on boot or migration,
2. framework-bundled artifacts are not user-editable in place,
3. if product version changes and bundled artifact version is newer, startup must migrate the instruction slice forward,
4. if database version is older than bundled version, the next lawful migration step must run automatically before normal execution continues,
5. if migration fails, boot must fail closed.

Immutability rule:

1. framework-owned instruction rows are append-and-supersede, not mutate-in-place,
2. superseded versions remain referenceable for migration/proof,
3. runtime uses the latest compatible active version.

---

## 5. Sidecar Semantics

Each instruction artifact may have zero or more sidecars.

Purpose of a sidecar:

1. attach bounded extra instructions,
2. clarify usage constraints,
3. add allowlisted context,
4. replace or supersede lower-priority sidecars when explicitly allowed.
5. patch immutable base instruction text without mutating the stored base artifact.

Sidecar rules:

1. sidecars are explicit first-class records, not hidden prompt text,
2. sidecars attach to one target artifact or one resolved bundle target,
3. sidecars must declare effect kind,
4. sidecars must declare precedence posture,
5. sidecars may not silently rewrite immutable framework-owned base artifacts,
6. sidecar effects must be inspectable in the effective bundle.

Canonical sidecar effect kinds:

1. `append`
2. `clarify`
3. `constrain`
4. `replace_lower_priority_sidecar`
5. `deactivate_sidecar`
6. `patch_base_projection`

Canonical storage note:

1. the runtime form of `patch_base_projection` is the structured instruction diff format,
2. raw unified diff may exist only as export/debug output, not as the canonical stored sidecar form.

### 5.1 Line-Oriented Patch Semantics

For immutable framework-bundled instruction artifacts, the canonical customization path is a sidecar patch projection rather than in-place mutation.

Patch rules:

1. the stored base artifact remains immutable,
2. sidecars target a stable artifact version or stable source hash,
3. sidecars project an effective rendered text or segment view,
4. sidecar patches must be replayable after migration or upgrade,
5. sidecar patches must fail closed when the targeted base anchor no longer matches.

Minimum sidecar patch operations:

1. `delete_range`
2. `replace_range`
3. `insert_before`
4. `insert_after`
5. `replace_with_many`

Minimum patch targeting modes:

1. absolute line span,
2. anchored line match,
3. anchored segment hash.

Recommended resolution order:

1. anchored segment hash,
2. anchored line match,
3. absolute line span.

Reason:

1. absolute line numbers alone are fragile under upstream product upgrades,
2. anchored patches are more migration-safe,
3. line numbers may remain a convenience field but must not be the only integrity check.

### 5.2 Replace/Delete Examples

Allowed examples:

1. delete one embedded line from the effective projection,
2. replace one immutable base line with one different line,
3. replace one immutable base line with multiple projected lines,
4. insert extra lines before or after one anchored line,
5. append one new instruction block without changing base lines,
6. insert one new section under an anchored point,
7. deactivate one earlier sidecar and replace it with a later higher-priority sidecar.

Forbidden examples:

1. rewriting the stored immutable framework base row itself,
2. applying a patch after anchor mismatch without explicit migration/rebind,
3. silently dropping failed patches and pretending the effective bundle is complete.

Forbidden sidecar behavior:

1. in-place mutation of immutable framework-owned instruction base rows,
2. hidden behavior changes without version or sidecar record,
3. using chat transcript as the only sidecar source,
4. silently overriding `Instruction Contract` logic through rendering-only config.

---

## 6. Versioning And Migration

Instruction memory must support version-aware startup.

Startup rules:

1. inspect stored instruction schema version,
2. inspect stored bundled artifact versions,
3. compare them to the product-bundled versions,
4. if stored version is lower, run the next required migration,
5. repeat until current product version is reached,
6. revalidate sidecar anchors against the upgraded base artifacts,
7. fail closed on any incompatible or partial migration state.

Migration rules:

1. migrations are monotonic,
2. migrations run in ordered version steps,
3. each successful migration records a receipt,
4. immutable framework-owned artifacts are superseded by new version rows rather than mutated in place,
5. user/project sidecars must be revalidated against the upgraded base version,
6. anchor mismatch requires either deterministic patch rebinding or explicit sidecar deactivation.

---

## 7. Logical Storage Layout

One embedded `SurrealDB` may contain multiple logical slices.

The minimum logical split is:

1. project-state tables/records,
2. instruction-system tables/records,
3. framework-memory tables/records.

The instruction-system slice should minimally include dedicated records or tables for:

1. base instruction artifacts,
2. sidecar patches,
3. effective bundle materialization metadata,
4. instruction version metadata,
5. instruction migration receipts.

This may be realized as:

1. separate tables,
2. separate record families,
3. separate namespaces or databases,

as long as the logical separation remains explicit and inspectable.

Current direct `1.0` preference:

1. one engine,
2. explicit logical separation inside the engine,
3. one authoritative instruction slice with its own migration/version metadata.

---

## 8. Downstream Consequences

This spec constrains implementation to:

1. keep project memory and instruction memory separate,
2. give instruction artifacts their own durable storage slice,
3. implement version-aware boot migration for instruction data,
4. expose effective bundle composition from base artifacts plus sidecars,
5. preserve immutable framework-bundled instruction artifacts while allowing attachable sidecars.

---

## 9. Immediate Next Work

The next lawful follow-through after this spec is:

1. update instruction-kernel, storage-kernel, and migration-kernel specs to reference the three-slice model,
2. add TaskFlow tasks for instruction-memory runtime and sidecar runtime,
3. implement minimal instruction-slice boot/migration scaffolding in the Rust binary.
-----
artifact_path: framework/plans/vida-0.3-instruction-memory-and-sidecar-spec
artifact_type: plan
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/plans/vida-0.3-instruction-memory-and-sidecar-spec.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: vida-0.3-instruction-memory-and-sidecar-spec.changelog.jsonl
P26-03-09T21: 44:13Z
