# VIDA Canonical Layer Documentation Template

Status: active product law

Purpose: define the canonical documentation template for Layers 1 through 7 so each layer can be written from the outer system frame toward the inner rule set without skipping authority, boundaries, proof, or standalone value.

## 1. Scope

This template defines:

1. the required documentation shape for Layer 1 through Layer 7 specs,
2. the outer-to-inner section order,
3. the minimum required sections,
4. layer-specific adaptation rules,
5. closure and deduplication rules for layer-law documents.

This template does not define:

1. Layer 8 runtime-consumption law,
2. the exact domain semantics of any one layer,
3. project-specific artifact-body formatting that is owned by a higher-precedence standard.

## 2. Core Template Principle

Layer specs must be written from the outside inward.

That means:

1. first define why the layer exists in the system,
2. then define what belongs to the layer and what does not,
3. then define what the layer consumes and produces,
4. then define the rules inside the layer,
5. only then define proof, value, and absorbed sources.

The template must make it possible to understand one layer without reading the implementation first.

## 3. Canonical Outer-To-Inner Section Order

Every Layer 1 through Layer 7 spec should follow this order unless a stricter promoted law says otherwise:

1. `Title`
2. `Status`
3. `Purpose`
4. `Scope`
5. `Layer Purpose`
6. `Inputs` or `Canonical Inputs`
7. `Outputs` or `Canonical Outputs`
8. `Authority and Boundaries`
9. `Core Rules`
10. `Validation or Proof Rules`
11. `Completion Proof`
12. `Standalone Value`
13. `Source Absorption`
14. machine-readable footer metadata

## 4. Canonical Layer Spec Skeleton

The canonical skeleton is:

```md
# <Layer Name>

Status: active product law

Purpose: <one sentence stating why this layer exists and what it protects or enables>

## 1. Scope

This spec defines:

1. ...

This spec does not define:

1. ...

## 2. Canonical Layer Purpose

<describe the one job this layer does in the total architecture>

## 3. Canonical Inputs

1. ...

Current canonical sources:

1. ...

## 4. Canonical Outputs

1. ...

## 5. Authority And Boundary Rule

Rules:

1. ...

## 6. Core Rule Set

Rules:

1. ...

## 7. Validation / Operational Proof Rule

Rules:

1. ...

Current bounded proof surface:

1. ...

## 8. Completion Proof For Layer <N>

Layer <N> is closed when all of the following are true:

1. ...

## 9. Standalone Value

<describe how this layer is useful even before later layers exist>

## 10. Source Absorption

This spec absorbs and concentrates law previously scattered across:

1. ...
```

## 5. Required Section Semantics

### 5.1 Title

Rules:

1. the title must name the layer capability, not the current tool that proves it,
2. use `Canonical ... Law` for promoted layer-law artifacts,
3. avoid legacy-origin titles once the layer has one promoted home.

### 5.2 Purpose

Rules:

1. the purpose must state the layer job in one sentence,
2. it must say what the layer protects, enables, or proves,
3. it must not describe implementation details first.

### 5.3 Scope

Rules:

1. scope must explicitly separate what the layer defines from what it does not define,
2. scope must push future-layer behavior out of the current layer,
3. scope must block silent authority expansion.

### 5.4 Inputs

Rules:

1. inputs must list only sources that the layer is allowed to consume,
2. inputs must distinguish canonical sources from incidental helpers,
3. if the layer depends on a registry, map, manifest, or catalog, that input must be explicit.

### 5.5 Outputs

Rules:

1. outputs must say what new capability the layer produces,
2. outputs must be inspectable without requiring later layers,
3. outputs must not smuggle runtime-consumption behavior into pre-runtime layers.

### 5.6 Authority And Boundary Rule

Rules:

1. this section must define what the layer is authoritative over,
2. it must state what the layer cannot override,
3. it must prevent duplicate or competing sources of truth.

### 5.7 Core Rule Set

Rules:

1. core rules belong here, not in the title or examples,
2. use short numbered rules, not diffuse narrative paragraphs,
3. split large mixed domains into subrules rather than into undocumented helper assumptions.

### 5.8 Validation / Operational Proof Rule

Rules:

1. every green layer must have a bounded operational proof,
2. proof commands or proof surfaces must be explicit,
3. proof must validate the layer as documented, not just tool existence.

### 5.9 Completion Proof

Rules:

1. completion proof must be stated as a closed checklist,
2. every item must be observable,
3. no completion item may require a higher unfinished layer.

### 5.10 Standalone Value

Rules:

1. this section must explain why the layer is already worth using once closed,
2. value must be stated without borrowing the promise of later layers.

### 5.11 Source Absorption

Rules:

1. this section must name the earlier scattered sources the promoted spec replaces or concentrates,
2. use it to reduce law duplication,
3. once absorbed, the promoted spec becomes the canonical home.

## 6. Layer-Specific Adaptation Rules

### Layer 1: Canonical Schema

Must emphasize:

1. vocabulary,
2. metadata contract,
3. canonical field sets,
4. status/owner/layer/type classes.

Must not depend on:

1. inventory generation,
2. mutation flows,
3. readiness or runtime behavior.

### Layer 2: Canonical Inventory

Must emphasize:

1. canonical inventory scope,
2. registry structure,
3. coverage rules,
4. canonical write path,
5. source/projection visibility.

Must not depend on:

1. relation impact logic,
2. readiness verdicts,
3. runtime consumption.

### Layer 3: Canonical Validation

Must emphasize:

1. consistency gates,
2. fail-closed validation posture,
3. warnings vs errors,
4. explicit exception policy.

Must not depend on:

1. mutation operations,
2. impact graphs,
3. runtime execution.

### Layer 4: Canonical Mutation

Must emphasize:

1. lawful edits,
2. lineage-safe changes,
3. metadata/changelog synchronization,
4. bounded migration behavior.

Must not depend on:

1. future relation analytics,
2. runtime compatibility resolution,
3. runtime authorization.

### Layer 5: Canonical Relations

Must emphasize:

1. edge taxonomy,
2. direct and reverse relations,
3. artifact impact,
4. task impact.

Must not depend on:

1. readiness verdicts,
2. bundle execution,
3. runtime progression.

### Layer 6: Canonical Operator

Must emphasize:

1. low-call workflows,
2. compact operator surfaces,
3. current-state visibility,
4. bounded closure paths.

Must not depend on:

1. runtime-owned latest resolution,
2. live boot authorization,
3. runtime consumption.

### Layer 7: Canonical Runtime Readiness

Must emphasize:

1. readiness inputs,
2. verdict classes,
3. version tuples,
4. compatibility classes,
5. bundle completeness,
6. projection parity,
7. boot-gate sufficiency,
8. fail-closed blockers.

Must not depend on:

1. actual runtime execution,
2. live route progression,
3. direct runtime consumption.

## 7. Deduplication Rule For Layer Specs

1. one layer should have one promoted law-bearing spec as its canonical home,
2. matrix docs may summarize the layer,
3. indexes and maps may point to the layer,
4. plans and research may feed the layer,
5. but the layer law itself must not be duplicated across multiple active product-law documents.

## 8. Documentation-First Rule For New Layers

1. before implementation begins for a new layer, its spec must be brought into this template shape,
2. if the layer is still exploratory, plans and research may precede the promoted spec,
3. once implementation starts against the layer, the promoted layer spec becomes mandatory,
4. if implementation reveals a gap in a green layer spec, correct the spec immediately when the correction is bounded and safe.

## 9. Skill And Project Standard Rule

1. this template governs canonical layer-law structure, not every possible artifact-body format,
2. if a higher-precedence skill-specific or project-specific artifact standard applies to a concrete document family, that standard may shape the body of the artifact,
3. canonical metadata, lineage, authority, and deduplication law remain binding around that artifact.

## 10. Optimal Use Rule

Use this template when:

1. promoting a new layer from plans/research into product law,
2. refactoring an existing layer spec that has become too implementation-shaped,
3. auditing whether a layer is actually documented from outer structure to inner rules,
4. comparing two candidate layer specs for completeness.

## 11. Current Coverage Note

This template is intended for Layers 1 through 7 because those layers can be documented and validated before direct runtime consumption is closed.

Layer 8 requires a separate runtime-consumption template once direct runtime authority exists.

-----
artifact_path: product/spec/canonical-layer-documentation-template
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: docs/product/spec/canonical-layer-documentation-template.md
created_at: '2026-03-10T04:23:08+02:00'
updated_at: '2026-03-10T04:25:26+02:00'
changelog_ref: canonical-layer-documentation-template.changelog.jsonl
