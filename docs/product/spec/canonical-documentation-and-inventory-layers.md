# VIDA Canonical Documentation And Inventory Layers

Status: active product law

Purpose: define the reference VIDA 1 layering model for canonical documentation, inventory, validation, mutation, relations, operator use, and runtime-readiness without coupling any layer to capabilities that belong only to a later layer.

## 0. Layer Status Table

Status markers:

1. `✅` completed and already available in the current transitional implementation
2. `🟡` partially implemented or planned next, but not yet closed as a full layer
3. `⚪` not yet implemented

| Layer | Status | Core value | Must not depend on |
|---|---|---|---|
| Canonical Schema | ✅ | one canonical vocabulary for artifact identity, status, compatibility, bundle/projection terms, and metadata | inventory generation, mutation workflows, runtime bundle execution |
| Canonical Inventory | 🟡 | one authoritative inventory and canonical registry path for active canon | impact analysis, runtime migration logic, bundle execution |
| Canonical Validation | 🟡 | fail-closed consistency checks and strict quality gates | graph inference, runtime startup execution, migration state machines |
| Canonical Mutation | ✅ | lawful metadata/changelog/link/file mutation without manual drift | relation graphs, runtime compatibility resolution, migration authorization |
| Canonical Relations | 🟡 | dependency and impact visibility over canonical artifacts | runtime bundle materialization, live migration/boot execution |
| Canonical Operator | ✅ | low-call operational views for state, history, and issues | runtime latest-resolution execution, boot authorization |
| Canonical Runtime Readiness | ⚪ | explicit readiness verdict for runtime consumption | actual runtime execution or live route progression |
| Canonical Runtime Consumption | ⚪ | VIDA runtime directly consumes canonical inventory and readiness | none; this is the final layer |

## 1. Scope

This spec defines the target capability layers for the VIDA 1 canonical documentation and inventory system.

It is not tied to the current Python implementation.

It defines:

1. layer purposes,
2. allowed inputs,
3. required outputs,
4. completion criteria,
5. forbidden forward-dependencies,
6. the rule that every layer must be independently useful and operable once completed.

## 2. Layering Rule

Each layer must satisfy all of the following:

1. it must deliver standalone operational value,
2. it must be completable using only lower layers that are already closed,
3. it must not depend on future-layer behavior,
4. it may deepen or enrich a lower layer, but it must not redefine the lower layer’s responsibility,
5. it must expose a clear proof of completion.

Compact rule:

1. each next layer deepens the system,
2. no next layer may borrow authority from an unfinished higher layer.

## 3. Canonical Slicing Categories

The canonical slicing categories are:

1. `Canonical Schema`
2. `Canonical Inventory`
3. `Canonical Validation`
4. `Canonical Mutation`
5. `Canonical Relations`
6. `Canonical Operator`
7. `Canonical Runtime Readiness`
8. `Canonical Runtime Consumption`

These categories are ordered from foundational to final.

## 4. Layer 1: Canonical Schema

### 4.1 Purpose

Freeze the canonical vocabulary and metadata contract used by all higher layers.

### 4.2 Must Define

1. artifact classes and artifact types,
2. statuses,
3. owners,
4. layers,
5. compatibility classes,
6. bundle-related terms,
7. version tuple terms,
8. footer metadata schema,
9. sidecar changelog event schema.

### 4.3 Inputs

1. product specs,
2. framework plans that are already canonical inputs,
3. existing canonical `vida/config/**` artifact families.

### 4.4 Outputs

1. one canonical schema vocabulary,
2. one canonical metadata contract,
3. one canonical changelog event contract.

### 4.5 Forbidden Dependencies

Layer 1 must not depend on:

1. inventory generation,
2. graph relations,
3. mutation workflows,
4. runtime bundle assembly,
5. runtime boot execution.

### 4.6 Completion Proof

1. the active canon has no unknown artifact/status/layer/owner/compatibility values,
2. strict schema validation succeeds against the active canon.

### 4.7 Standalone Value

Layer 1 gives VIDA one language for all canonical artifacts.

Current completion note:

1. the canonical schema vocabulary is now explicitly fixed as a dedicated schema layer,
2. it covers canonical artifact types, status terms, owners, layers, compatibility classes, and base bundle/projection/registry terms,
3. higher layers may deepen this vocabulary, but they must not invent competing schema authorities.

## 5. Layer 2: Canonical Inventory

### 5.1 Purpose

Build the authoritative inventory of canonical artifacts across markdown and machine-readable law.

### 5.2 Must Define

1. canonical registry structure,
2. registry read model,
3. canonical registry write path,
4. coverage rules for markdown authoring artifacts,
5. coverage rules for `vida/config/**` machine-readable artifacts,
6. source/projection linkage surfaces where defined canonically,
7. version tuple visibility in the inventory.

### 5.3 Inputs

1. Layer 1 schema,
2. canonical markdown artifacts,
3. canonical YAML/config artifacts.

### 5.4 Outputs

1. one registry view of the active canon,
2. one canonical registry artifact when materialized,
3. deterministic inventory coverage.

### 5.5 Forbidden Dependencies

Layer 2 must not depend on:

1. impact analysis,
2. mutation planning,
3. runtime migration logic,
4. effective bundle execution.

### 5.6 Completion Proof

1. all active canonical artifact families are present in registry output,
2. the canonical registry path can be written deterministically,
3. registry coverage is explainable from schema plus active canon only.

### 5.7 Standalone Value

Layer 2 gives VIDA a complete map of what canonical artifacts currently exist.

## 6. Layer 3: Canonical Validation

### 6.1 Purpose

Provide fail-closed consistency checking for the canonical inventory and its metadata contracts.

### 6.2 Must Validate

1. footer metadata completeness,
2. sidecar presence and ownership,
3. allowed vocabulary values,
4. source/projection consistency where required,
5. broken links,
6. orphan changelogs,
7. registry/schema consistency,
8. profile-specific warning/error posture.

### 6.3 Inputs

1. Layer 1 schema,
2. Layer 2 inventory.

### 6.4 Outputs

1. normal validation result,
2. strict validation result,
3. policy-driven warnings versus errors.

### 6.5 Forbidden Dependencies

Layer 3 must not depend on:

1. impact graph inference,
2. batch mutation planning,
3. runtime startup execution,
4. migration state machines.

### 6.6 Completion Proof

1. active-canon validation succeeds in normal mode,
2. strict mode is usable as a real release-quality gate,
3. exceptions are policy-defined rather than ad hoc.

### 6.7 Standalone Value

Layer 3 gives VIDA a trustworthy quality gate for the documentation/inventory canon.

## 7. Layer 4: Canonical Mutation

### 7.1 Purpose

Enable lawful edits to canonical artifacts without footer, sidecar, or reference drift.

### 7.2 Must Support

1. timestamp-plus-changelog updates,
2. one-shot finalization after multiple diff edits,
3. artifact initialization,
4. artifact move,
5. artifact rename,
6. exact link migration,
7. batch mutation where the operation still remains semantically bounded.

### 7.3 Inputs

1. Layer 1 schema,
2. Layer 3 validation.

### 7.4 Outputs

1. lawful artifact mutations,
2. lawful sidecar mutations,
3. quiet success and explicit failure behavior,
4. validation-backed mutation outcomes.

### 7.5 Forbidden Dependencies

Layer 4 must not depend on:

1. relation graphs,
2. runtime compatibility resolution,
3. bundle execution,
4. migration authorization.

### 7.6 Completion Proof

1. routine edits can be completed through canonical mutation paths,
2. batch edit finalization does not produce redundant changelog noise,
3. mutation commands do not leave metadata drift behind.

### 7.7 Standalone Value

Layer 4 gives VIDA an operational authoring system rather than manual footer/changelog handling.

## 8. Layer 5: Canonical Relations

### 8.1 Purpose

Expose the dependency and impact graph over the canonical artifact inventory.

### 8.2 Must Support

1. direct markdown links,
2. direct footer references,
3. dependency-edge inventory,
4. artifact impact radius,
5. task-scoped impact radius.

### 8.3 Inputs

1. Layer 2 inventory,
2. Layer 3 validated canonical artifacts.

### 8.4 Outputs

1. dependency views,
2. graph-like edge views,
3. direct and indirect impact views.

### 8.5 Forbidden Dependencies

Layer 5 must not depend on:

1. runtime bundle materialization,
2. migration boot logic,
3. parity or cutover execution.

### 8.6 Completion Proof

1. dependency views are inspectable,
2. artifact impact is traceable,
3. task impact is traceable,
4. relation tools work from canonical inventory and references only.

### 8.7 Standalone Value

Layer 5 gives VIDA a change-radius analysis system.

## 9. Layer 6: Canonical Operator

### 9.1 Purpose

Minimize the number of actions needed to understand the current state of the canonical documentation system.

### 9.2 Must Support

1. one-command overview of current state,
2. compact human-readable views,
3. low-call operational paths for state, history, validation, and impact.

### 9.3 Inputs

1. Layers 2 through 5.

### 9.4 Outputs

1. operator-ready summaries,
2. operator-ready history views,
3. operator-ready issue views.

### 9.5 Forbidden Dependencies

Layer 6 must not depend on:

1. runtime latest-resolution execution,
2. boot authorization,
3. migration transitions.

### 9.6 Completion Proof

1. normal daily orientation is possible in one or two commands,
2. operators do not need to assemble state manually from many low-level calls.

### 9.7 Standalone Value

Layer 6 gives VIDA a practical control surface for humans and transitional agents.

## 10. Layer 7: Canonical Runtime Readiness

### 10.1 Purpose

Determine whether the canonical inventory is ready to be consumed by runtime without silent assumptions.

### 10.2 Must Validate

1. source-version tuple completeness,
2. compatibility class support,
3. bundle membership completeness,
4. projection freshness and rebind requirements,
5. sidecar applicability to bundled artifacts,
6. fail-closed readiness outcomes.

### 10.3 Inputs

1. Layers 1 through 6,
2. migration/kernel requirements already fixed in the canonical specs.

### 10.4 Outputs

1. readiness verdict,
2. blocking reasons,
3. compatibility or migration-required classification.

### 10.5 Forbidden Dependencies

Layer 7 must not depend on:

1. actual runtime execution,
2. live startup mutation,
3. live route progression.

### 10.6 Completion Proof

1. the system can explain whether canonical artifacts are runtime-ready,
2. unresolved or incompatible tuples are fail-closed,
3. readiness output is explicit rather than inferred.

### 10.7 Standalone Value

Layer 7 gives VIDA a pre-runtime readiness gate.

## 11. Layer 8: Canonical Runtime Consumption

### 11.1 Purpose

Allow the VIDA 1 runtime itself to consume the canonical inventory, readiness, bundles, and projections directly.

### 11.2 Inputs

1. Layers 1 through 7.

### 11.3 Outputs

1. runtime-owned latest resolution,
2. runtime-owned bundle consumption,
3. runtime-owned compatibility enforcement.

### 11.4 Completion Proof

1. runtime no longer depends on ad hoc filesystem assumptions for canonical artifact selection,
2. runtime consumes canonical inventory and readiness surfaces directly.

### 11.5 Standalone Value

Layer 8 is the final product state where the documentation/inventory system is no longer merely transitional tooling.

## 12. Required Canonical Requirement Clusters

These requirement clusters constrain the layers above:

1. `Artifact identity and lineage`
   - footer metadata,
   - sidecar changelog,
   - latest-only canonical body,
   - canonical registry path.
2. `Instruction inventory, projection, and bundle composition`
   - artifact families,
   - projection rules,
   - bundle order,
   - immutable bundled artifacts,
   - effective bundle composition,
   - sidecar patch semantics.
3. `Compatibility, boot, and migration`
   - compatibility classes,
   - source-version tuples,
   - boot gates,
   - fail-closed migration outcomes.
4. `Receipts, proofs, checkpoints, and projections`
   - canonical evidence families,
   - receipt/proof taxonomy,
   - checkpoint/projection distinction,
   - route artifact families.
5. `Parity and cutover evidence`
   - parity inputs,
   - delta categories,
   - cutover preconditions,
   - proof-bearing readiness evidence.

## 13. Dependency Rule Between Clusters

1. `Artifact identity and lineage` is foundational for all other clusters.
2. `Instruction inventory, projection, and bundle composition` may depend on identity and lineage, but not on later runtime execution.
3. `Compatibility, boot, and migration` may depend on identity, inventory, bundles, and explicit semantic inputs, but must remain fail-closed before normal runtime execution.
4. `Receipts, proofs, checkpoints, and projections` are evidence and durability surfaces; they must not be reinterpreted as shadow state or migration authority.
5. `Parity and cutover evidence` depends on all prior clusters and must never be treated as complete when any lower cluster remains unresolved.

## 14. Reference-Architecture Rule

This layering model is a VIDA 1 reference architecture.

Rules:

1. the current transitional implementation may approximate these layers,
2. implementation convenience must not redefine the layer boundaries,
3. future implementation work should be evaluated against these layers, not the inverse.

-----
artifact_path: product/spec/canonical-documentation-and-inventory-layers
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: docs/product/spec/canonical-documentation-and-inventory-layers.md
created_at: '2026-03-10T03:25:00+02:00'
updated_at: '2026-03-10T02:37:11+02:00'
changelog_ref: canonical-documentation-and-inventory-layers.changelog.jsonl
