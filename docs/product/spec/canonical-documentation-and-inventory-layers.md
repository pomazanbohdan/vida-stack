# VIDA Canonical Documentation And Inventory Layers

Status: active product law

Purpose: define the reference VIDA 1 layering model for canonical documentation, inventory, validation, mutation, relations, operator use, and runtime-readiness without coupling any layer to capabilities that belong only to a later layer.

## 0. Layer Status Matrix

Status markers:

1. `✅` completed and already available in the current transitional implementation
2. `🟡` partially implemented or planned next, but not yet closed as a full layer
3. `⚪` not yet implemented

| Category | Layer 1 | Layer 2 | Layer 3 | Layer 4 | Layer 5 | Layer 6 | Layer 7 | Layer 8 |
|---|---|---|---|---|---|---|---|---|
| Layer name | Canonical Schema | Canonical Inventory | Canonical Validation | Canonical Mutation | Canonical Relations | Canonical Operator | Canonical Runtime Readiness | Canonical Runtime Consumption |
| Status | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚪ |
| Core value | one canonical vocabulary for identity, status, compatibility, bundle/projection terms, and metadata | one authoritative inventory and canonical registry path for active canon | fail-closed consistency checks and strict quality gates with explicit bootstrap carrier rules | lawful metadata, changelog, link, and file mutation without manual drift | dependency and impact visibility over canonical artifacts | low-call operational views for state, history, and issues | explicit readiness verdict for runtime consumption | VIDA runtime directly consumes canonical inventory and readiness |
| Required implementation | schema vocabularies, metadata contract, changelog event contract | registry model, canonical registry artifact, coverage rules | check, doctor, strict profiles, consistency gates | touch, finalize, init, move, rename, link migration | deps, deps-map, artifact-impact, task-impact | overview, compact operator surfaces, low-call workflows | readiness checks for tuples, projections, bundles, compatibility | runtime consumption of registry, readiness, and canonical bundles |
| Builds on | none | Layer 1 | Layers 1-2 | Layers 1-3 | Layers 2-3 | Layers 2-5 | Layers 1-6 | Layers 1-7 |
| Must not depend on | inventory generation, mutation workflows, runtime bundle execution | impact analysis, runtime migration logic, bundle execution | graph inference, runtime startup execution, migration state machines | relation graphs, runtime compatibility resolution, migration authorization | runtime bundle materialization, live migration/boot execution | runtime latest-resolution execution, boot authorization | actual runtime execution or live route progression | none; final layer |
| Standalone value | one language for all canonical artifacts | one complete map of current canonical artifacts | one trustworthy quality gate | one lawful authoring and mutation system | one change-radius and dependency analysis system | one low-call operator surface | one runtime-readiness gate | one live runtime consumption path |
| Detail section | [§4](#4-layer-1-canonical-schema) | [§5](#5-layer-2-canonical-inventory) | [§6](#6-layer-3-canonical-validation) | [§7](#7-layer-4-canonical-mutation) | [§8](#8-layer-5-canonical-relations) | [§9](#9-layer-6-canonical-operator) | [§10](#10-layer-7-canonical-runtime-readiness) | [§11](#11-layer-8-canonical-runtime-consumption) |

Matrix reading rule:

1. read the matrix left-to-right to see the capability progression,
2. read any one column top-to-bottom to understand one layer completely,
3. use the `Detail section` row to jump into the full normative definition below,
4. treat the lower sections as the expanded law for the abbreviated matrix cells above.

## 0.1 Current Documentation Compliance Snapshot

Status markers:

1. `✅` documentation coverage is already sufficient for the layer to act as canonical law,
2. `🟡` documentation coverage exists but still depends on framework-plan detail or leaves meaningful gaps,
3. `⚪` documentation coverage is still too thin to act as canonical law by itself.

| Category | Layer 1 | Layer 2 | Layer 3 | Layer 4 | Layer 5 | Layer 6 | Layer 7 | Layer 8 |
|---|---|---|---|---|---|---|---|---|
| Documentation compliance | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | 🟡 |
| Strongest evidence | `codex-v0/docsys_schema.yaml`, `instruction-artifact-model.md`, `projection_manifest.yaml` | `canonical-inventory-law.md`, `project-documentation-system.md`, `current-spec-map.md`, `instruction_catalog.yaml`, canonical registry path | this spec, `project-documentation-system.md`, canonical `check`/`doctor` rules in bootstrap docs | this spec, `AGENTS.sidecar.md`, canonical mutation command contract | `canonical-relation-law.md`, `project-documentation-system.md`, `AGENTS.sidecar.md`, relation commands in `codex-v0/codex.py` | this spec, `framework-map-protocol.md`, `AGENTS.sidecar.md` overview/low-call contract | `canonical-runtime-readiness-law.md`, `instruction-artifact-model.md`, `docs/framework/research/canonical-runtime-readiness-external-patterns.md`, `codex-v0/codex.py` readiness-check | consumption law is defined as target architecture, but not yet expanded into a fully promoted runtime-consumption product spec |
| Main current gap | no blocking documentation gap | no blocking documentation gap | no blocking documentation gap | no blocking documentation gap | no blocking documentation gap | no blocking documentation gap | no blocking documentation gap | consumption law is defined as target architecture, but not yet expanded into a fully promoted runtime-consumption product spec |

Compliance reading rule:

1. this snapshot measures the state of the documentation itself, not the implementation maturity of the layer,
2. a layer may be well-documented even if implementation is still partial,
3. Layer 8 remains documentation-partial because its most detailed law still lives primarily in target-architecture intent rather than in a fully promoted runtime-consumption product-law form.

## 0.1 Current Documentation Alignment Snapshot

This snapshot evaluates the active documentation and instruction surfaces against the matrix above.

### Layer 1: Canonical Schema

Matrix status: `✅`  
Documentation alignment: `✅`

Strongest evidence:

1. `docs/product/spec/instruction-artifact-model.md`
2. `docs/product/spec/project-documentation-system.md`
3. `docs/product/spec/current-spec-map.md`
4. this spec

Current conclusion:

1. the documentation canon already fixes the base vocabulary for artifact identity, metadata footer, sidecar lineage, latest-only markdown authority, and active canon boundaries,
2. the remaining vocabulary depth for some machine-readable families is handled as extension of the same schema space, not as a competing authority.

### Layer 2: Canonical Inventory

Matrix status: `✅`  
Documentation alignment: `✅`

Strongest evidence:

1. `docs/product/spec/canonical-inventory-law.md`
2. `docs/product/spec/project-documentation-system.md`
3. `docs/product/spec/current-spec-map.md`
4. `vida/config/instructions/system-maps.framework-map-protocol.md`
5. `docs/product/spec/instruction-artifact-model.md`

Current conclusion:

1. the documentation canon now has one dedicated canonical inventory spec,
2. the inventory layer now explicitly defines registry structure, canonical registry path, coverage rules, source/projection linkage, and version-tuple visibility,
3. Layer 2 documentation is now sufficient to act as canonical law without relying on distributed unstated assumptions.

### Layer 3: Canonical Validation

Matrix status: `✅`  
Documentation alignment: `✅`

Strongest evidence:

1. `docs/product/spec/project-documentation-system.md`
2. this spec
3. `vida/config/instructions/system-maps.framework-map-protocol.md`

Current conclusion:

1. the documentation canon defines footer completeness, sidecar ownership, consistency checks, profile-specific validation posture, and explicit bootstrap-carrier exceptions,
2. validation behavior is described as policy-driven rather than ad hoc tool behavior.

### Layer 4: Canonical Mutation

Matrix status: `✅`  
Documentation alignment: `✅`

Strongest evidence:

1. this spec
2. `vida/config/instructions/system-maps.framework-map-protocol.md`
3. `AGENTS.sidecar.md`

Current conclusion:

1. the documentation canon already describes lawful mutation paths, one-shot finalization after multiple diff edits, exact link migration, and narrow bootstrap-carrier mutation exceptions,
2. the mutation layer is documented as operational law rather than as undocumented helper behavior.

### Layer 5: Canonical Relations

Matrix status: `✅`  
Documentation alignment: `✅`

Strongest evidence:

1. `docs/product/spec/canonical-relation-law.md`
2. `docs/product/spec/project-documentation-system.md`
3. this spec
4. `AGENTS.sidecar.md`

Current conclusion:

1. the documentation canon now has one dedicated canonical relation spec,
2. edge taxonomy, direct/reverse relations, artifact impact, task impact, and relation validation are now explicitly defined,
3. Layer 5 documentation is now sufficient to act as canonical law without relying on scattered operational descriptions.

### Layer 6: Canonical Operator

Matrix status: `✅`  
Documentation alignment: `✅`

Strongest evidence:

1. `docs/product/spec/project-documentation-system.md`
2. `vida/config/instructions/system-maps.framework-map-protocol.md`
3. `AGENTS.sidecar.md`

Current conclusion:

1. the documentation canon already requires one-command overview reads, low-call operational paths, compact history/status views, and initialization-time automatic context reads,
2. operator ergonomics are explicitly part of the architecture rather than accidental tooling convenience.

### Layer 7: Canonical Runtime Readiness

Matrix status: `✅`  
Documentation alignment: `✅`

Strongest evidence:

1. `docs/product/spec/canonical-runtime-readiness-law.md`
2. `docs/product/spec/instruction-artifact-model.md`
3. `docs/framework/research/canonical-runtime-readiness-external-patterns.md`
4. `codex-v0/codex.py`

Current conclusion:

1. the documentation canon now has one promoted readiness law,
2. source-version tuples, compatibility classes, projection parity, canonical bundles, boot-gate artifacts, verdict classes, and fail-closed blocker reasons are explicitly defined,
3. Layer 7 documentation is now sufficient to act as canonical law without relying on scattered framework-plan text.

### Layer 8: Canonical Runtime Consumption

Matrix status: `⚪`  
Documentation alignment: `⚪`

Strongest evidence:

1. this spec
2. `vida/config/instructions/system-maps.framework-map-protocol.md`
3. `docs/product/spec/current-spec-map.md`

Current conclusion:

1. the documentation canon clearly states the target end-state where runtime consumes canonical inventory, readiness, bundles, and projections directly,
2. it does not yet define a closed active law for direct runtime consumption behavior.

Primary blocker:

1. runtime-consumption law is still reference-architecture intent, not an active closed documentation layer.

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

Documentation-first development rule:

1. when a new layer is started, its canonical documentation must be brought into shape first,
2. that documentation pass must define the layer purpose, inputs, outputs, forbidden dependencies, completion proof, and standalone value before implementation begins,
3. implementation substrate work may start only after the documentation pass for that layer is explicit enough to act as the authority for the work,
4. if implementation and layer documentation diverge, the documentation must be corrected first or the implementation must be brought back into line before the layer is considered active.

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

Documentation-first rule for this layer:

1. before schema validators or schema-backed tooling evolve, the Layer 1 vocabulary and contracts must be documented first,
2. implementation may only encode vocabulary that is already documented as Layer 1 law.

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

Documentation-first rule for this layer:

1. before registry coverage or inventory tooling expands, the Layer 2 scope and coverage rules must be documented first,
2. implementation may only materialize inventory classes and registry behavior that are already documented as Layer 2 law.

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
3. exceptions are policy-defined rather than ad hoc,
4. bootstrap routing exceptions are explicit canonical carrier rules rather than hidden tool-only drift.

### 6.7 Standalone Value

Layer 3 gives VIDA a trustworthy quality gate for the documentation/inventory canon.

Documentation-first rule for this layer:

1. before new validation gates are added, the Layer 3 validation law must define allowed checks, failure posture, and exception handling first,
2. implementation may only enforce checks that are already documented as Layer 3 law.

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

### 7.8 Root Bootstrap Normalization Rule

The repository root bootstrap surface must use one explicit canonical mode rather than a mixed exception model.

Canonical root-bootstrap rule:

1. `AGENTS.md` is the bootstrap carrier and routing contract,
2. `AGENTS.sidecar.md` is the canonical metadata-bearing sidecar for bootstrap context,
3. repository root documents that are not bootstrap carriers must use the normal metadata and changelog contract.

Validation and mutation rule:

1. bootstrap carrier exceptions must be explicit, narrow, and named,
2. carrier exceptions must not silently widen to unrelated root documents,
3. canonical metadata and mutation behavior for non-carrier root documents must match the rest of the active canon.

Completion rule:

1. root bootstrap routing is handled through the carrier-plus-sidecar model,
2. non-carrier root documents are governed by the standard metadata contract,
3. no mixed root-level transitional exception mode remains.

Documentation-first rule for this layer:

1. before mutation commands or workflows expand, the Layer 4 lawful mutation contract must be documented first,
2. implementation may only automate mutation paths that are already documented as Layer 4 law.

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

Documentation-first rule for this layer:

1. before relation or impact tooling expands, the Layer 5 relation surfaces and interpretations must be documented first,
2. implementation may only expose relation outputs that are already documented as Layer 5 law.

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

Documentation-first rule for this layer:

1. before operator UX is widened, the Layer 6 operator-facing contract must be documented first,
2. implementation may only compose low-call operator views that are already documented as Layer 6 law.

## 10. Layer 7: Canonical Runtime Readiness

### 10.1 Purpose

Determine whether the canonical inventory is ready to be consumed by runtime without silent assumptions.

### 10.2 Must Validate

1. source-version tuple completeness,
2. compatibility class support,
3. bundle membership completeness,
4. projection freshness and tuple parity for explicitly bound projections,
5. required boot-gate artifact presence,
6. fail-closed readiness outcomes and explicit blocker reasons.

### 10.3 Inputs

1. Layers 1 through 6,
2. `docs/product/spec/canonical-runtime-readiness-law.md`,
3. migration/kernel requirements already fixed in the canonical specs.

### 10.4 Outputs

1. readiness verdict,
2. blocking reasons,
3. compatibility or migration-required classification,
4. bounded readiness proof through `codex-v0/codex.py readiness-check`.

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

Documentation-first rule for this layer:

1. before readiness gates are implemented, the Layer 7 readiness law must define tuples, compatibility, bundle, and projection expectations first,
2. implementation may only report readiness against rules that are already documented as Layer 7 law.

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

Documentation-first rule for this layer:

1. before runtime consumption is wired, the Layer 8 consumption contract must be documented first,
2. implementation may only consume registry, readiness, and bundle surfaces that are already documented as Layer 8 law.

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
updated_at: '2026-03-10T03:52:18+02:00'
changelog_ref: canonical-documentation-and-inventory-layers.changelog.jsonl
