# Feature Design And ADR Model

Status: active product law

Purpose: define the canonical split between structured feature design documents and linked ADRs so major VIDA changes are described in one bounded design artifact without collapsing design, decision recording, and implementation proof into one unstructured prose body.

## 1. Core Rule

One bounded change may require both:

1. a `design document`
2. one or more `ADRs`

They are not the same artifact.

## 2. Design Document Purpose

A feature design document exists to describe one bounded change before or during implementation.

It should capture:

1. current context
2. goal and scope
3. functional and non-functional requirements
4. technical design
5. affected ownership/runtime surfaces
6. bounded file set
7. fail-closed constraints
8. implementation phases
9. proof and rollout expectations

## 3. ADR Purpose

An ADR exists to record one durable architecture or major implementation decision.

It should capture:

1. decision context
2. options considered
3. chosen outcome
4. trade-offs
5. consequences

Rule:

1. use the design document for the whole bounded change,
2. use an ADR only when one decision inside that change must remain durable and separately referenceable.

## 4. Structured Template Rule

Feature design documents must prefer:

1. stable headings
2. explicit fields
3. bounded bullet lists
4. short concrete prose
5. linked references instead of re-explaining owner law

They must not depend on:

1. giant narrative prose,
2. implicit file scope,
3. hidden assumptions about proof or rollout,
4. vague statements like "details later" for critical safety or ownership boundaries.

## 5. OpenAI And Anthropic Alignment

The framework follows the external vendor guidance direction that long-form planning artifacts work best when they are:

1. structured,
2. explicit about required fields,
3. stable in section ordering,
4. split between fixed template content and variable project-specific content.

Implication:

1. a VIDA design document should behave as a structured template plus bounded variable content,
2. not as free-form essay prose,
3. and not as a compressed DSL that hides safety semantics.

## 6. VIDA-Specific Required Sections

Every bounded feature/change design document should include at minimum:

1. `Summary`
2. `Current Context`
3. `Goal`
4. `Requirements`
5. `Ownership And Canonical Surfaces`
6. `Design Decisions`
7. `Technical Design`
8. `Bounded File Set`
9. `Fail-Closed Constraints`
10. `Implementation Plan`
11. `Validation / Proof`
12. `Observability`
13. `Rollout Strategy`
14. `References`

## 7. Ownership Rule

1. framework-owned design templates live under `docs/framework/templates/**`,
2. project-owned design documents live under the active project docs map,
3. stable promoted product-law about design/decision structure lives under `docs/product/spec/**`,
4. process/runbook guidance may point to these artifacts, but it must not become a second owner layer.

## 8. Placement Rule

Use the design document in:

1. `docs/product/spec/**`
   - when the design is committed product/runtime direction
2. `docs/product/research/**`
   - when the design is still exploratory or comparative
3. a linked ADR artifact
   - when one decision needs durable standalone recording

Do not use a feature design document as:

1. a replacement for owner protocols,
2. a replacement for packet/task state,
3. an excuse to widen implementation scope beyond the bounded file set.

## 9. Template Rule

The framework-owned reusable starting point is:

1. `docs/framework/templates/feature-design-document.template.md`

The canonical project-local materialized path is:

1. `docs/product/spec/templates/feature-design-document.template.md`

Rule:

1. use the template shape first,
2. materialize the project-local template at `docs/product/spec/templates/feature-design-document.template.md` for active project work,
3. fill bounded variable content into that shape,
4. only project-replace the template if a project-owned higher-precedence format is explicitly introduced.

## 10. Relationship To Runtime And Protocols

A design document may point to:

1. framework protocols
2. runtime families
3. project capsules
4. specs
5. validation commands

But it must not:

1. duplicate owner-law bodies unnecessarily,
2. redefine activation or safety rules that already belong to canonical protocols,
3. become a second law-bearing source when a protocol already owns the rule.

## 11. Validation Rule

When introducing or changing the design-doc model/template:

1. update template-discovery routing,
2. update relevant spec maps,
3. update relevant process/tooling entrypoints,
4. run bounded canonical validation on the changed surfaces.

## 12. Canonical Routing Surfaces

Normal discovery for this model should route through:

1. `config/system-maps/template.map`
2. `process/documentation-tooling-map`
3. `process/readme`

Rule:

1. these routing surfaces may point to the template and owner model,
2. but they must not absorb the owner-law content into duplicated prose.

## 13. Current Rule

1. use one structured design document for one bounded feature/change,
2. use linked ADRs for durable major decisions inside that design,
3. keep the template stable and the variable content explicit,
4. preserve protocol/template/implementation separation.

-----
artifact_path: product/spec/feature-design-and-adr-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-14'
schema_version: '1'
status: canonical
source_path: docs/product/spec/feature-design-and-adr-model.md
created_at: '2026-03-14T16:40:00+02:00'
updated_at: '2026-03-14T17:15:00+02:00'
changelog_ref: feature-design-and-adr-model.changelog.jsonl
