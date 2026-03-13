# Extensibility And Output Template Model

Status: active product law

Purpose: define the canonical extensibility classes, protocol-versus-template split, and root output template system for Release 1 and later VIDA runtime rendering.

## 1. Extensibility Classes

The runtime must treat extension rights as a controlled matrix rather than a universal edit-right.

Three extensibility classes are required:

1. `sealed`
   - protected framework/core/system law
   - not directly replaceable by the project
2. `augmentable`
   - upper system surfaces that may accept sidecar-like extension without replacing the canonical owner
3. `replaceable`
   - project-facing roles, skills, profiles, flows, teams, agents, and adjacent behavior surfaces that may be disabled and replaced by project-owned alternatives

## 2. Extensibility Rule

1. the user must not directly rewrite protected core/system canon as a project customization path,
2. sidecar-style extension is the lawful way to deepen augmentable surfaces,
3. full replacement is allowed only where the architecture marks the surface as project-replaceable,
4. activation, validation, migration, and compiled runtime identity must respect these classes.

## 3. Protocol Versus Template

The runtime must distinguish clearly between protocols and templates.

1. `protocol`
   - an instruction-bearing behavioral or operational rule that governs what must happen
2. `template`
   - a rendering or structural output form used at the moment the protocol requires information, an artifact, or a contract to be produced

Template rule:

1. templates may define document shapes, screen-output shapes, packets, or external interaction contracts,
2. templates do not replace protocols,
3. templates are activated by protocol-governed moments and output classes,
4. project templates are project-replaceable surfaces unless a narrower rule explicitly protects them.

## 4. Project Replacement Rule

1. project roles, skills, profiles, flow sets, agents, project protocols, and output templates are project-replaceable surfaces,
2. a project-replaceable surface may be disabled in active configuration while still remaining preserved in the database or import state,
3. active configuration must declare which source/path is authoritative for each replaced project surface,
4. sidecars may override or deepen project-replaceable templates and project-owned behavior surfaces,
5. sealed framework/core/system protocols remain non-replaceable,
6. Release-1 augmentable-but-not-replaceable upper system surfaces are limited to:
   - output/render templates,
   - onboarding/interview templates,
   - project-facing packet or artifact formats that deepen a sealed governing protocol without replacing it.
7. routing, activation, approval, readiness, execution, and safety protocols remain sealed even when project sidecars or templates deepen the user-facing rendering around them.

## 5. Root Output Template System

The runtime requires an explicit output-rendering strategy for root operator-facing outputs.

This strategy must define:

1. which output classes exist,
2. which protocol governs each output class,
3. when a dedicated template exists versus when model rendering is allowed,
4. which data fields are mandatory for each rendered output,
5. which surfaces may produce human-facing narrative output versus structured output.

## 6. Minimum Target Output Classes

1. planning/scope snapshot,
2. specification snapshot,
3. task-graph snapshot,
4. execution status snapshot,
5. artifact surface snapshot,
6. approval/review packet,
7. verification/readiness result,
8. sync/reconcile result,
9. release/deployment result,
10. observability/audit snapshot.

## 7. Output-Rendering Rule

1. Release 1 does not require a dedicated system-owned root template registry,
2. project-owned templates may live inside skills or adjacent project extension surfaces,
3. the model may compose the final answer dynamically in Release 1 when no canonical template exists,
4. when a project-owned or later system-owned canonical template exists, rendering must follow that template rather than inventing a new output shape,
5. later releases may formalize a stronger shared template registry without making Release 1 invalid.

## 8. Format-Selection Rule

1. when multiple project-owned output formats or templates are available and no higher-precedence canonical format is active, the model may choose the best-fit format contextually for the current task,
2. format choice under those conditions is a project-level behavior concern rather than a fixed framework-level priority rule,
3. if a higher-precedence canonical format is active for the current output class, the model must follow it instead of making a free contextual choice.

## 9. Relationship To Other Specs

1. `project-activation-and-configurator-model.md` owns lifecycle and active-state control for project-replaceable surfaces.
2. `project-protocol-promotion-law.md` owns executable admission for project protocols.
3. `agent-role-skill-profile-flow-model.md` owns role/skill/profile/flow semantics.
4. this document owns the extension-rights matrix and the output-template rendering split across those surfaces.

## 10. Current Rule

1. extensibility remains controlled by explicit classes rather than by universal rewrite rights,
2. templates deepen or render protocol-governed behavior rather than replacing protocols,
3. root output rendering must remain inspectable even when Release 1 still allows dynamic model composition.

-----
artifact_path: product/spec/extensibility-and-output-template-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/product/spec/extensibility-and-output-template-model.md
created_at: '2026-03-13T08:39:49+02:00'
updated_at: '2026-03-13T08:47:25+02:00'
changelog_ref: extensibility-and-output-template-model.changelog.jsonl
