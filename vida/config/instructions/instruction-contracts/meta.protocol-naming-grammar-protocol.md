# Protocol Naming Grammar Protocol

Purpose: define the canonical VIDA framework naming law for instruction artifacts under `vida/config/instructions/**`, including category-directory placement, stratum-visible filename grammar, migration safety, and fail-closed rename sequencing.

## Scope

This protocol applies when the framework:

1. names a new instruction artifact,
2. renames an existing instruction artifact,
3. restructures category-local discovery without adding deeper nested protocol directories,
4. aligns filename grammar, protocol-index wording, artifact metadata, and coverage/audit surfaces,
5. executes a framework-wide naming migration.

This protocol governs active instruction artifacts under:

1. `instruction-contracts/`
2. `runtime-instructions/`
3. `command-instructions/`
4. `diagnostic-instructions/`
5. `system-maps/`
6. `agent-definitions/`
7. `agent-backends/`
8. `prompt-templates/`
9. `references/`

## Structural Rule

The canonical filesystem model for `vida/config/instructions/**` is:

1. one category directory under `vida/config/instructions/`,
2. one artifact file directly inside that category directory,
3. no deeper protocol subdirectories below the category directory.

Hard rules:

1. category directories are the deepest stable filesystem bucket for instruction artifacts,
2. internal category structure must be expressed by filename grammar,
3. hidden semantic grouping by historical wrapper names is forbidden,
4. every canonical filename must expose enough structure to be discoverable without opening the file.

## Canonical Filename Grammar

Canonical instruction filename grammar is:

1. `<cluster>.<family>-<function>-<artifact-role>.md`

Where:

1. `cluster`
   - the primary structural cluster for the category,
2. `family`
   - the bounded domain family,
3. `function`
   - the concrete owned function,
4. `artifact-role`
   - one of:
     - `protocol`
     - `contract`
     - `map`
     - `index`
     - `guide`
     - `note`
     - `templates`
     - `reference`

Compact rule:

1. the part before the dot says what structural cluster the artifact belongs to,
2. the part after the dot says what domain family and bounded function it owns.

## Category-Local Cluster Sets

### `instruction-contracts`

Allowed clusters:

1. `core`
2. `role`
3. `overlay`
4. `bridge`
5. `work`
6. `meta`

Examples:

1. `core.agent-system-protocol.md`
2. `lane.worker-dispatch-protocol.md`
3. `overlay.step-thinking-protocol.md`
4. `bridge.instruction-activation-protocol.md`
5. `work.documentation-operation-protocol.md`
6. `meta.protocol-naming-grammar-protocol.md`

### `runtime-instructions`

Allowed clusters:

1. `core`
2. `role`
3. `bridge`
4. `work`
5. `runtime`
6. `recovery`
7. `model`
8. `observability`

Examples:

1. `core.context-governance-protocol.md`
2. `core.run-graph-protocol.md`
3. `bridge.spec-sync-protocol.md`
4. `work.spec-intake-protocol.md`
5. `runtime.direct-runtime-consumption-protocol.md`
6. `recovery.checkpoint-replay-recovery-protocol.md`
7. `observability.observability.trace-grading-protocol.md`

### `command-instructions`

Allowed clusters:

1. `routing`
2. `planning`
3. `execution`
4. `operator`
5. `migration`

Examples:

1. `routing.command-layer-protocol.md`
2. `planning.form-task-protocol.md`
3. `execution.implement-execution-protocol.md`
4. `operator.runtime-pipeline-guide.md`
5. `migration.pack-wrapper-note.md`

### `diagnostic-instructions`

Allowed clusters:

1. `escalation`
2. `analysis`
3. `evaluation`

Examples:

1. `escalation.debug-escalation-protocol.md`
2. `analysis.framework-self-analysis-protocol.md`
3. `evaluation.library-evaluation-protocol.md`

### `system-maps`

Allowed clusters:

1. `framework`
2. `protocol`
3. `runtime-family`
4. `governance`
5. `observability`
6. `template`
7. `tooling`
8. `bootstrap`
9. `migration`

Examples:

1. `framework.map.md`
2. `protocol.index.md`
3. `runtime-family.taskflow-map.md`
4. `bootstrap.orchestrator-boot-flow.md`
5. `migration.runtime-transition-map.md`

### `agent-definitions`

Allowed clusters:

1. `model`
2. `role`
3. `entry`

Examples:

1. `model.agent-definitions-contract.md`
2. `role.role-profile-contract.md`
3. `entry.orchestrator-entry.md`

### `agent-backends`

Allowed clusters:

1. `role`
2. `matrix`

Examples:

1. `role.backend-lifecycle-protocol.md`
2. `matrix.agent-backends-matrix.md`

### `prompt-templates`

Allowed clusters:

1. `worker`
2. `cheap-worker`

### `references`

Allowed clusters:

1. `protocol`
2. `algorithms`

## Category-Fit Rule

Filename grammar must not override category truth.

Rules:

1. the category directory remains the first classifier,
2. the cluster prefix refines structure inside that category only,
3. the same cluster token may exist across several categories, but it does not erase category meaning,
4. if the filename suggests a different category than the artifact's real owner domain, the artifact must be moved or renamed.

## One Canonical Name Rule

Each active instruction artifact must have exactly one canonical live name.

Rules:

1. dual-path aliases are forbidden by default,
2. temporary migration notes may point from old names to new names, but old active duplicates must not remain,
3. a rename is incomplete until:
   - the file path is moved,
   - footer `source_path` is updated,
   - `artifact_path` is updated when naming semantics changed materially,
   - `protocol-index` or other canonical maps are rewired,
   - dependent links are migrated,
   - validation passes.

## Metadata Alignment Rule

When a canonical artifact is renamed:

1. `source_path` must match the new filesystem location,
2. `artifact_path` must be normalized to the new naming model,
3. `changelog_ref` must match the new file stem,
4. row labels in `system-maps/protocol.index.md` must remain semantically aligned with the new canonical name.

Fail-closed rule:

1. a moved file with stale metadata is not naming-green,
2. a renamed file with stale index wiring is not naming-green.

## Migration Law

A framework naming migration must run in bounded waves.

Required order:

1. establish naming law,
2. define migration plan and category sequence,
3. move one bounded batch,
4. migrate links and references,
5. update protocol index and process audits,
6. validate,
7. continue to the next batch.

Required bounded waves:

1. `Wave 0`
   - establish naming law and migration plan
2. `Wave 1`
   - `instruction-contracts`, `agent-definitions`, `agent-backends`
3. `Wave 2`
   - `runtime-instructions`
4. `Wave 3`
   - `command-instructions`, `diagnostic-instructions`
5. `Wave 4`
   - `system-maps`, `prompt-templates`, `references`
6. `Wave 5`
   - final global rewiring, audit alignment, and closure validation

Batch safety rules:

1. do not rename several unrelated categories in one unvalidated batch,
2. keep old-to-new mapping explicit for each batch,
3. after each batch, search for stale old paths before starting the next batch,
4. if one batch introduces unresolved stale canonical references, stop the migration and repair before continuing.

## Validation Rule

Naming migration work is complete only when all are true:

1. changed files pass `check`,
2. canonical protocol/index changes pass `activation-check`,
3. canonical protocol-bearing/index changes pass `protocol-coverage-check`,
4. changed canonical docs/maps pass `doctor`,
5. no intended canonical old path remains as a live active owner reference.

## Forbidden Behaviors

1. Do not rename by aesthetic preference alone when the bounded owner boundary is still unclear.
2. Do not introduce deeper nested protocol directories below the category directory.
3. Do not keep both old and new canonical names active.
4. Do not update filenames without rewiring protocol-index or dependent audit/matrix docs.
5. Do not treat helper/reference artifacts as if they were protocol-bearing owners merely to fit grammar symmetry.

-----
artifact_path: config/instructions/instruction-contracts/meta.protocol-naming-grammar.protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/meta.protocol-naming-grammar-protocol.md
created_at: '2026-03-11T16:20:00+02:00'
updated_at: '2026-03-11T12:29:40+02:00'
changelog_ref: meta.protocol-naming-grammar-protocol.changelog.jsonl
