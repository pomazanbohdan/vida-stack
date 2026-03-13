# Operational State And Synchronization Model

Status: active product law

Purpose: define the canonical operational state split, synchronization law, conflict handling, and reactive domain routing for DB-first VIDA runtime state.

## 1. Three Coordinated State Representations

The target runtime uses three coordinated state representations:

1. `operational state`
   - the live DB-first runtime truth
2. `filesystem projection`
   - synchronized editable markdown, YAML, and adjacent file artifacts
3. `Git lineage`
   - historical backup and review surface for the filesystem projection

## 2. State Rule

1. the runtime executes against database truth,
2. the filesystem is a synchronized editable projection rather than an equal second truth source,
3. changes may flow in both directions,
4. final operational convergence must still pass through the database.

## 3. Synchronization Rule

1. synchronization is bidirectional,
2. authority remains DB-first,
3. conflicts must fail closed,
4. reconciliation must be explicit and auditable,
5. silent destructive merges are forbidden.

## 4. Conflict-Resolution Rule

1. DB/filesystem conflicts must not be resolved authoritatively by freeform LLM judgment alone,
2. conflict resolution must occur through explicit point mutations using available tools in one direction or the other,
3. the model may help classify, explain, or recommend a resolution path,
4. the authoritative resolution path must remain tool-mediated, bounded, and auditable.

## 5. Reactive Synchronization And Domain Routing

Release 2 adds a reactive synchronization engine that must distinguish at least two event domains:

1. `engine-owned domain`
   - VIDA engine files, documentation, config, and synchronized internal projections
2. `host-project domain`
   - external codebase and project artifacts integrated by VIDA

### 5.1 Engine-Owned Domain

For the engine-owned domain:

1. watcher events trigger sync/reconcile flow,
2. the runtime decides whether to import, export, refresh, block, or escalate,
3. controlled tools apply the accepted change into the DB-first state model.

### 5.2 Host-Project Domain

For the host-project domain:

1. watcher events trigger indexing/memory flow rather than internal engine sync,
2. the runtime may update semantic search, graph state, code index, and memory artifacts,
3. these updates support orchestration, search, and context resolution for the host project.

## 6. Reactive-Flow Rule

1. watcher detects,
2. classifier interprets,
3. decision layer resolves,
4. apply layer mutates,
5. reconciliation verifies,
6. receipt layer records the result.

## 7. Reactive-Event Discussion Note

1. the exact canonical event taxonomy for Release 2 remains a `next discussion` item,
2. that discussion must define what an event is, why it exists, which domains emit it, and which classes are required in the first reactive engine,
3. until then, the minimal architectural meaning of the event model is:
   - detect a change
   - classify the change
   - decide whether sync, indexing, ignore, block, or escalation is required
   - execute the bounded mutation path
   - persist receipts for audit and later recovery.

## 8. Relationship To Other Specs

1. `embedded-runtime-and-editable-projection-model.md` owns the broader installed-runtime versus projection model.
2. `runtime-paths-and-derived-cache-model.md` owns the concrete `.vida/**` placement and cache topology.
3. `project-activation-and-configurator-model.md` owns project activation lifecycle over the same DB-first authority model.
4. this document owns the state-coordination and synchronization law that spans those surfaces.

## 9. Current Rule

1. VIDA runtime truth stays DB-first.
2. filesystem artifacts remain projections and exchange surfaces.
3. Git preserves historical lineage for those projections.
4. synchronization and conflict resolution remain explicit, bounded, and auditable.

-----
artifact_path: product/spec/operational-state-and-synchronization-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/product/spec/operational-state-and-synchronization-model.md
created_at: '2026-03-13T08:39:49+02:00'
updated_at: '2026-03-13T08:47:25+02:00'
changelog_ref: operational-state-and-synchronization-model.changelog.jsonl
