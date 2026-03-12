# Embedded Runtime And Editable Projection Model

Status: active product law

Purpose: define the canonical Release-1 and forward runtime model where sealed framework canon compiles at build time into embedded runtime artifacts, installed runtime execution can operate from the binary plus DB-first state alone, and editable project surfaces remain available only through lawful export/import projection flows.

## 1. Problem

The repository currently still exposes a large source tree of framework and project artifacts.

That is acceptable for source-mode development, but it is not the desired long-term runtime shape.

The target runtime should not require loose framework markdown, YAML, or helper files to remain installed beside the binary just to execute lawfully.

Without a stricter model:

1. release runtime can drift toward file-dependent execution,
2. exported editable files can be mistaken for runtime truth,
3. framework canon, compiled runtime artifacts, and mutable DB state can collapse into one ambiguous surface.

## 2. Goal

The target runtime should support all of:

1. build-time compilation of sealed framework canon into embedded runtime artifacts,
2. installed execution from the binary plus DB-first runtime state,
3. explicit project-level export for editing when editing is needed,
4. lawful re-import of edited project surfaces back into DB/runtime truth,
5. fail-closed execution when required imports or compiled artifacts are missing or invalid.

Compact rule:

1. source canon compiles,
2. runtime executes from embedded artifacts plus DB,
3. editable files are projections, not truth.

## 3. Three Operational Modes

### 3.1 Source Mode

Source mode is the repository/development mode.

In this mode:

1. canonical framework law exists in `vida/config/**` and `docs/product/spec/**`,
2. project-editable surfaces may already exist in the working tree,
3. build pipelines may read raw source artifacts directly,
4. runtime may still prove parity against those source artifacts during development.

### 3.2 Compiled Runtime Mode

Compiled runtime mode is the installed/release execution mode.

In this mode:

1. required framework runtime artifacts are embedded in the binary,
2. runtime seeds or validates DB-first state from those embedded artifacts,
3. loose framework source files are not required for lawful runtime execution,
4. the binary plus DB/state is sufficient to operate the runtime.

### 3.3 Projection Mode

Projection mode is the lawful editing loop.

In this mode:

1. the runtime exports editable project-facing files from DB or compiled runtime state,
2. humans and agents may edit those projected files,
3. edited files must be validated, compiled, and imported back into DB/runtime truth,
4. the edited files remain synchronized projections rather than an equal second truth source.

## 4. Ownership Split

### 4.1 Sealed Framework Runtime Artifacts

These compile at build time and are embedded into the binary:

1. framework protocols,
2. framework maps,
3. framework defaults,
4. framework templates,
5. framework instruction bundles,
6. compiled control-bundle schema and related framework-owned executable contracts.

Rule:

1. these artifacts are sealed for runtime execution,
2. they are not directly mutated inside installed runtime mode,
3. changes to them return through source-mode authoring and rebuild.

### 4.2 Project-Editable Surfaces

These remain editable through lawful projection:

1. project-facing configuration exports,
2. project-facing roles, skills, profiles, and flows exports,
3. project-facing protocol exports,
4. project docs and templates,
5. project-owned activation exports admitted by higher-precedence law.

Rule:

1. active runtime configuration and activation truth should live under `.vida/**`,
2. root project files are not the long-term active runtime substrate,
3. project-editable surfaces may be exported from runtime state,
4. but they become active only after validation, compilation, and DB import.

### 4.3 Operational Runtime State

These remain DB-first mutable runtime truth:

1. imported framework-state receipts,
2. imported project activation state,
3. protocol-binding state and receipts,
4. telemetry and run-graph state,
5. recovery, readiness, and health state.

Rule:

1. runtime executes against this DB-first state,
2. projections and exports are secondary to it.

## 5. Build-Time Compilation Rule

Build pipelines must be able to compile framework canon into embedded runtime artifacts.

Minimum embedded artifact families:

1. `framework_control_bundle`
2. `framework_instruction_bundle`
3. `framework_protocol_binding_registry`
4. `framework_template_bundle`
5. explicit metadata about artifact schema version, revision, and generation source

Build rule:

1. embedded artifacts must be deterministic enough for validation and repeatable rebuild,
2. build pipelines must fail closed if required framework-owned inputs are missing or invalid,
3. release runtime must not depend on ad hoc loose copies of those same framework artifacts.

## 6. Runtime Bootstrap Rule

Installed runtime bootstrap must prefer embedded artifacts over loose source files.

Minimum bootstrap path:

1. load embedded framework artifacts,
2. validate schema/version compatibility,
3. seed or migrate DB-first runtime state,
4. write import/migration receipts,
5. refuse non-bootstrap execution if required imports are missing or invalid.

Bootstrap rule:

1. the binary may materialize editable projections when explicitly asked,
2. but execution truth still comes from embedded artifacts plus DB state,
3. installed runtime must not silently downgrade into broad source-file rereads as a hidden fallback.

Placement rule:

1. the canonical installed runtime shape should converge on one `.vida/` runtime home,
2. active runtime configuration belongs under `.vida/config/**`,
3. active project activation/runtime-owned registries belong under `.vida/project/**`,
4. authoritative project-local DB state belongs under `.vida/db/**`,
5. derived serving caches belong under `.vida/cache/**`.

## 7. Editable Projection Export/Import Rule

Projection export/import is the lawful loop for runtime-editable project state.

### 7.1 Export

Runtime must support bounded export of editable project surfaces.

Exported files should carry enough metadata for safe return:

1. schema version,
2. source kind,
3. provenance/reference metadata,
4. import class or compatibility class where relevant.

### 7.2 Import

Import must do all of:

1. validate edited projection files,
2. compile them into machine-readable runtime payloads,
3. import the compiled result into DB-first runtime truth,
4. write receipts for the import outcome,
5. fail closed on invalid or conflicting input.

Import rule:

1. edited projection files do not become runtime truth merely by existing on disk,
2. they become active only after successful import.

Hidden-surface rule:

1. runtime-owned configuration, roles, skills, profiles, flows, and adjacent project activation state may remain hidden under `.vida/**` by default,
2. human editing happens through export/import loops rather than by treating root project files as the always-live runtime source.

## 8. Binary-Only Release Rule

Public installed runtime should be able to ship in a binary-first form.

That means:

1. the minimal installed runtime may consist of the binary plus runtime state directories,
2. framework canon does not need to remain unpacked as loose source files for execution,
3. project scaffolds or projections may be materialized later through runtime commands such as init/export,
4. runtime-owned templates required for first bootstrap may be embedded and expanded on demand.

Release rule:

1. loose files may still appear as exported project scaffolds,
2. but those files are runtime-produced projection surfaces, not the required execution substrate.

## 9. Fail-Closed Rule

The runtime must fail closed when:

1. embedded required artifacts are missing,
2. required DB imports are missing or invalid,
3. exported project edits fail validation or compilation,
4. runtime cannot determine whether DB state or projected files are the newer lawful source.

Failure rule:

1. runtime must expose bounded remediation instructions,
2. it must not continue by guessing which source is authoritative,
3. it must not silently treat projected files as truth.

## 10. Relationship To Other Specs

This model complements, but does not replace:

1. `compiled-runtime-bundle-contract.md`
   - defines the runtime bundle contract itself
2. `project-activation-and-configurator-model.md`
   - defines DB-first activation and lifecycle operations
3. `release-build-packaging-law.md`
   - defines public release archive and installer boundary
4. `taskflow-protocol-runtime-binding-model.md`
   - defines protocol-binding import and authority
5. `runtime-paths-and-derived-cache-model.md`
   - defines the detailed `.vida/` placement model for config, DB, framework artifacts, project activation surfaces, receipts, and derived serving caches

Interpretation rule:

1. this spec defines the higher-level runtime shape for embedded artifacts and editable projections,
2. lower specs define the individual bundle, activation, packaging, and protocol-binding details within that shape.

## 11. Completion Proof

This model is operationally closed enough when:

1. the runtime can execute from embedded framework artifacts plus DB-first state,
2. public installed runtime no longer depends on loose framework source files for lawful execution,
3. project-facing editable files can be exported on demand,
4. edited project files can be validated, compiled, and imported back into DB truth,
5. drift between embedded framework truth, projected files, and DB state fails closed.

-----
artifact_path: product/spec/embedded-runtime-and-editable-projection-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/embedded-runtime-and-editable-projection-model.md
created_at: '2026-03-12T18:46:30+02:00'
updated_at: '2026-03-12T20:10:00+02:00'
changelog_ref: embedded-runtime-and-editable-projection-model.changelog.jsonl
