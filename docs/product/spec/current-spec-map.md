# VIDA Current Spec Map

Status: active canonical map

Revision: `2026-03-16`

Purpose: define the current product-spec home, provide a short active registry of product-law artifacts, and route detailed source lineage to the provenance companion map.

Companion rule:
1. `docs/product/spec/current-spec-map.md` is the short active registry.
2. `docs/product/spec/current-spec-provenance-map.md` carries detailed `Sources` lineage for the same canon.

## Current Canon

### Core

1. [partial-development-kernel-model.md](partial-development-kernel-model.md)
   Config families: `vida/config/machines/**`, `vida/config/routes/**`, `vida/config/policies/**`
2. [canonical-machine-map.md](canonical-machine-map.md)
   Config families: `vida/config/machines/**`
3. [receipt-and-proof-law.md](receipt-and-proof-law.md)
   Config families: `vida/config/receipts/**`, `vida/config/policies/**`
4. [external-pattern-borrow-map.md](external-pattern-borrow-map.md)
   Config families: cross-cutting product law only
5. [projection-listener-checkpoint-model.md](projection-listener-checkpoint-model.md)
   Config families: `vida/config/machines/**`, runtime consumption by the TaskFlow runtime family
6. [gateway-resume-handle-and-trigger-index.md](gateway-resume-handle-and-trigger-index.md)
   Config families: future route/gateway law
7. [machine-definition-lint-law.md](machine-definition-lint-law.md)
   Config families: future machine lint
8. [checkpoint-commit-and-replay-model.md](checkpoint-commit-and-replay-model.md)
   Config families: runtime-derived checkpoint law
9. [verification-merge-law.md](verification-merge-law.md)
   Config families: future verification routing law
10. [instruction-artifact-model.md](instruction-artifact-model.md)
    Config families: `vida/config/instructions/**`
11. [skill-management-and-activation-law.md](skill-management-and-activation-law.md)
    Config families: `skills/**`, `activation/**`
12. [instruction-migration-map.md](instruction-migration-map.md)
    Config families: `vida/config/instructions/**`

### Documentation And Inventory

1. [project-documentation-law.md](project-documentation-law.md)
   Config families: project documentation governance only
2. [canonical-documentation-and-inventory-layer-matrix.md](canonical-documentation-and-inventory-layer-matrix.md)
   Config families: canonical inventory, validation, mutation, relation, readiness, and runtime-consumption architecture across `vida/config/**`
3. [canonical-inventory-law.md](canonical-inventory-law.md)
   Config families: canonical inventory, registry structure, coverage, source/projection linkage, and version-tuple visibility across active canon
4. [canonical-relation-law.md](canonical-relation-law.md)
   Config families: canonical dependencies, direct/reverse references, artifact impact, task impact, and relation validation across active canon
5. [canonical-runtime-readiness-law.md](canonical-runtime-readiness-law.md)
   Config families: source-version tuples, compatibility classes, projection parity, canonical bundles, boot-gate artifacts, and fail-closed readiness verdicts across active canon
6. [canonical-layer-documentation-template.md](canonical-layer-documentation-template.md)
   Config families: canonical layer-law authoring shape for Layers 1 through 7
7. [functional-matrix-protocol.md](functional-matrix-protocol.md)
   Config families: canonical functional/capability matrix design, row schema, law-versus-implementation-versus-proof status split, seam protocol, bridge posture, and update/review rules for matrix-bearing specs
8. [framework-project-documentation-layer-model.md](framework-project-documentation-layer-model.md)
   Config families: framework canon vs role/bootstrap/governance/project documentation layering, derivation boundaries, two-map bootstrap, and root-map requirements
9. [root-map-and-runtime-surface-model.md](root-map-and-runtime-surface-model.md)
   Config families: framework root map, project root map, runtime-family submaps, template maps, and activation-trigger discoverability across active canon
10. [project-document-naming-law.md](project-document-naming-law.md)
    Config families: `docs/product/spec/**`, `docs/process/**`, `docs/product/research/**`, `docs/project-memory/**`, lane-root naming, reserved filename handling, and bounded rename-wave law for project-owned documentation
11. [feature-design-and-adr-model.md](feature-design-and-adr-model.md)
    Config families: structured feature/change design artifacts, linked ADR split, framework design-template routing, and bounded proof/rollout authoring for project and framework changes

### Runtime And Agent Control

1. [canonical-runtime-layer-matrix.md](canonical-runtime-layer-matrix.md)
   Config families: layered runtime capability progression across `vida/config/**`, TaskFlow runtime-family implementation surfaces, runtime ledgers, readiness gates, and future direct runtime consumption
2. [agent-role-skill-profile-flow-model.md](agent-role-skill-profile-flow-model.md)
   Config families: framework role law, project role/skill/profile/flow activation through `vida.config.yaml`, project-owned agent-extension registries, and runtime validation for the TaskFlow runtime family
3. [agent-lane-selection-and-conversation-mode-model.md](agent-lane-selection-and-conversation-mode-model.md)
   Config families: overlay-driven auto-lane selection, bounded conversational modes, one-task scope/PBI discussion, and lawful handoff into pack/taskflow routing
4. [party-chat-v2-problem-party-model.md](party-chat-v2-problem-party-model.md)
   Config families: `docs/process/agent-extensions/**`, `vida.config.yaml`, `.vida/logs/problem-party/**`, single-agent or multi-agent Party Chat execution plans, and runtime consumption by the TaskFlow runtime family
5. [autonomous-report-continuation-law.md](autonomous-report-continuation-law.md)
   Config families: `vida.config.yaml`, `vida/config/instructions/**`, TaskFlow routing and autonomous execution behavior
6. [taskflow-v1-runtime-modernization-plan.md](taskflow-v1-runtime-modernization-plan.md)
   Config families: TaskFlow runtime-family implementation surfaces, `vida/config/instructions/**`, runtime feature registration, shared runtime kernel, provider registry, modular config validation, and the active TaskFlow modernization backlog
7. [docflow-v1-runtime-modernization-plan.md](docflow-v1-runtime-modernization-plan.md)
   Config families: DocFlow runtime-family implementation surfaces, canonical `vida/config/docflow-*.current.jsonl` artifacts, `vida/config/instructions/**`, documentation tooling operator surfaces, runtime-family migration, and explicit final `taskflow -> docflow` consumption seams
8. [compiled-autonomous-delivery-runtime-architecture.md](compiled-autonomous-delivery-runtime-architecture.md)
   Config families: `vida/config/instructions/**`, `.vida/config/**`, `.vida/project/**`, `.vida/cache/**`, transitional source-mode bridge surfaces such as root `vida.config.yaml` and `docs/process/agent-extensions/**`, TaskFlow runtime-family implementation surfaces, DocFlow runtime-family implementation surfaces, and future compiled orchestration bundle surfaces
9. [emerging-architectural-patterns-model.md](emerging-architectural-patterns-model.md)
   Config families: runtime loop ownership, specialist-agent topology, routing, verifier aggregation, persistent workflow state, production observability, evaluation posture, governance/security expectations, caching strategy, and gateway/proxy control surfaces across `vida/config/instructions/**`, TaskFlow runtime-family implementation surfaces, and future compiled runtime surfaces
10. [compiled-runtime-bundle-contract.md](compiled-runtime-bundle-contract.md)
    Config families: compiled control bundles with `control_core`, `activation_bundle`, `protocol_binding_registry`, and `cache_delivery_contract`, `.vida/config/**`, `.vida/project/**`, `.vida/db/**`, `.vida/cache/**`, runtime init/boot activation, bundle validation, and future machine-readable orchestration bundle surfaces
11. [project-activation-and-configurator-model.md](project-activation-and-configurator-model.md)
    Config families: DB-first project activation, `.vida/config/**`, `.vida/project/**`, roles/skills/profiles/flows/agents/teams/model/backend policy, sync/reconcile surfaces, and project lifecycle control
12. [team-coordination-model.md](team-coordination-model.md)
    Config families: team composition, coordination pattern, activation, shared policy, handoff/context posture, and closure semantics
13. [status-families-and-query-surface-model.md](status-families-and-query-surface-model.md)
    Config families: CLI query/status families, operator-facing render surfaces, bounded runtime snapshots, and status-family routing
14. [project-protocol-promotion-law.md](project-protocol-promotion-law.md)
    Config families: known versus compiled project protocol admission, project discovery/mapping, executable bundle promotion, and fail-closed protocol binding
15. [taskflow-protocol-runtime-binding-model.md](taskflow-protocol-runtime-binding-model.md)
    Config families: script-era protocol binding bridge, Rust-native protocol runtime crate, activation resolution, gate enforcement, protocol receipts, binding matrices, and the dedicated TaskFlow protocol-binding subrelease
16. [user-facing-runtime-flow-and-operating-loop-model.md](user-facing-runtime-flow-and-operating-loop-model.md)
    Config families: operator-facing install/init/bootstrap flow, project-local runtime onboarding, project activation/config sequencing, intake/planning sequencing, execution/approval/resume sequencing, bounded pre-readiness allowlists, runtime bootstrap posture, and the staged user-facing operating loop across `.vida/**`, installed runtime assets, and DB-first readiness state
17. [bootstrap-carriers-and-project-activator-model.md](bootstrap-carriers-and-project-activator-model.md)
    Config families: bootstrap carriers, runtime init command split, project activator pipeline, sidecar/project-map enrichment, host-template onboarding, and bounded protocol-load separation between orchestrator and agent lanes
18. [execution-preparation-and-developer-handoff-model.md](execution-preparation-and-developer-handoff-model.md)
    Config families: `solution_architect`, execution preparation, architecture-preparation reports, developer handoff packets, change-boundary shaping, dependency-impact summaries, and fail-closed pre-execution gating for code-shaped work
19. [operational-state-and-synchronization-model.md](operational-state-and-synchronization-model.md)
    Config families: DB-first operational state, filesystem projection, Git lineage, synchronization law, conflict handling, and reactive domain routing
20. [host-agent-layer-status-matrix.md](host-agent-layer-status-matrix.md)
    Config families: host-agent activation layers, overlay-owned tier ladders, tier selection economics, local score/state surfaces, task-close feedback ingestion, and status/budget observability over `.vida/state/**`

### Project And Packaging

1. [repository-two-project-surface-model.md](repository-two-project-surface-model.md)
   Config families: active current-project routing, extracted second-project bundle boundaries, root config continuity, and two-project repository map discipline
2. [github-public-repository-law.md](github-public-repository-law.md)
   Config families: root repository entrypoints, `.github/**`, public-repository community surfaces, code ownership, issue/PR templates, security disclosure, and release/tag publication posture
3. [release-build-packaging-law.md](release-build-packaging-law.md)
   Config families: public release archive composition, installer/archive boundary, runtime-only package contents, sidecar scaffold packaging, and public release-page formatting alignment
4. [embedded-runtime-and-editable-projection-model.md](embedded-runtime-and-editable-projection-model.md)
   Config families: embedded framework artifacts, binary-only runtime execution, project projection export/import loops, hidden runtime-owned config/activation surfaces under `.vida/**`, DB-first runtime truth, and release/runtime separation between sealed framework state and editable project surfaces
5. [runtime-paths-and-derived-cache-model.md](runtime-paths-and-derived-cache-model.md)
   Config families: `.vida/config/**`, `.vida/db/**`, `.vida/cache/**`, `.vida/framework/**`, `.vida/project/**`, derived serving cache invalidation, hidden runtime-owned config and activation surfaces, and bridge migration away from root runtime files
6. [extensibility-and-output-template-model.md](extensibility-and-output-template-model.md)
   Config families: sealed/augmentable/replaceable surfaces, protocol-versus-template distinction, root output rendering, and project-replaceable template boundaries
7. [external-architecture-baseline.md](external-architecture-baseline.md)
   Config families: external orchestration baseline, guardrail boundary alignment, subagent specialization alignment, and runtime-state ownership references

### Release 1

1. [release-1-plan.md](release-1-plan.md)
   Config families: Release-1 execution ownership, mandatory capability closure, V1 target architecture, crate/file decomposition, stateful agent-lane governance, phase ordering, and platform-shape preservation
   Registered path: `docs/product/spec/release-1-plan.md`
2. [release-1-capability-matrix.md](release-1-capability-matrix.md)
   Config families: Release-1 capability ladder, cross-track closure, slice mapping, proof surfaces, and fail-closed seam ownership
   Registered path: `docs/product/spec/release-1-capability-matrix.md`
3. [release-1-seam-map.md](release-1-seam-map.md)
   Config families: Release-1 closure seam, TaskFlow-to-DocFlow activation/proof return, blocker classes, and final hardening admission
   Registered path: `docs/product/spec/release-1-seam-map.md`
4. [release-1-current-state.md](release-1-current-state.md)
   Config families: Release-1 readiness by slice/layer/seam, keep-versus-refactor posture, launcher concentration risk, and current state inputs
   Registered path: `docs/product/spec/release-1-current-state.md`
5. [release-1-closure-contract.md](release-1-closure-contract.md)
   Config families: Release-1 definition of done, non-waivable blockers, risk-acceptance law, and closure evidence bundle
   Registered path: `docs/product/spec/release-1-closure-contract.md`
6. [release-1-workflow-classification-and-risk-matrix.md](release-1-workflow-classification-and-risk-matrix.md)
   Config families: Release-1 workflow classes, risk tiers, approval posture, lifecycle variants, and supported workflow surface
   Registered path: `docs/product/spec/release-1-workflow-classification-and-risk-matrix.md`
7. [release-1-control-metrics-and-gates.md](release-1-control-metrics-and-gates.md)
   Config families: Release-1 control metrics, gate thresholds, release-candidate evidence windows, and workflow-tier gate binding
   Registered path: `docs/product/spec/release-1-control-metrics-and-gates.md`
8. [release-1-canonical-artifact-schemas.md](release-1-canonical-artifact-schemas.md)
   Config families: Release-1 minimum machine-readable contracts for traces, approvals, tool contracts, evaluation runs, incidents, memory records, and closure admission
   Registered path: `docs/product/spec/release-1-canonical-artifact-schemas.md`
9. [release-1-decision-tables.md](release-1-decision-tables.md)
   Config families: Release-1 executable control rules for approval, delegation, tool use, retrieval trust, memory writes, and rollback gates
   Registered path: `docs/product/spec/release-1-decision-tables.md`
10. [release-1-state-machine-specs.md](release-1-state-machine-specs.md)
   Config families: Release-1 canonical FSMs for lanes, approvals, tools, incidents, and prompt rollout
   Registered path: `docs/product/spec/release-1-state-machine-specs.md`
11. [release-1-error-and-exception-taxonomy.md](release-1-error-and-exception-taxonomy.md)
   Config families: Release-1 blocker codes, failure vocabulary, and exception-path taxonomy
   Registered path: `docs/product/spec/release-1-error-and-exception-taxonomy.md`
12. [release-1-ownership-to-code-map.md](release-1-ownership-to-code-map.md)
   Config families: Release-1 owner-doc to crate/module placement, shell boundary discipline, and anti-drift ownership mapping
   Registered path: `docs/product/spec/release-1-ownership-to-code-map.md`
13. [release-1-proof-scenario-catalog.md](release-1-proof-scenario-catalog.md)
   Config families: Release-1 minimum proof scenarios, negative-control scenarios, and scenario evidence requirements
   Registered path: `docs/product/spec/release-1-proof-scenario-catalog.md`
14. [release-1-schema-versioning-and-compatibility-law.md](release-1-schema-versioning-and-compatibility-law.md)
   Config families: Release-1 artifact schema evolution, compatibility classes, mixed-version rules, and migration law
   Registered path: `docs/product/spec/release-1-schema-versioning-and-compatibility-law.md`
15. [release-1-runtime-enum-and-code-contracts.md](release-1-runtime-enum-and-code-contracts.md)
   Config families: Release-1 canonical enum/value contracts for workflow classes, risk tiers, statuses, gate levels, blocker codes, and compatibility classes
   Registered path: `docs/product/spec/release-1-runtime-enum-and-code-contracts.md`
16. [release-1-conformance-matrix.md](release-1-conformance-matrix.md)
   Config families: Release-1 doc-to-code-to-proof mapping, conformance posture, and bounded implementation targets
   Registered path: `docs/product/spec/release-1-conformance-matrix.md`
17. [release-1-operator-surface-contract.md](release-1-operator-surface-contract.md)
   Config families: Release-1 stable operator output contracts for status, doctor, consume, lane, approval, and recovery surfaces
   Registered path: `docs/product/spec/release-1-operator-surface-contract.md`
18. [release-1-unsupported-surface-contract.md](release-1-unsupported-surface-contract.md)
   Config families: Release-1 unsupported and architecture-reserved surface boundaries and denial posture
   Registered path: `docs/product/spec/release-1-unsupported-surface-contract.md`
19. [release-1-fixture-and-golden-data-contract.md](release-1-fixture-and-golden-data-contract.md)
   Config families: Release-1 canonical fixtures, golden scenarios, and compatibility-proof sample contracts
   Registered path: `docs/product/spec/release-1-fixture-and-golden-data-contract.md`
20. [release-1-risk-acceptance-register.md](release-1-risk-acceptance-register.md)
   Config families: Release-1 explicit open-risk tracking, bounded acceptances, and closure-governance visibility
   Registered path: `docs/product/spec/release-1-risk-acceptance-register.md`

### Support

1. [current-spec-provenance-map.md](current-spec-provenance-map.md)
   Config families: detailed source lineage, absorbed historical inputs, and provenance routing for the active product-spec canon
   Registered path: `docs/product/spec/current-spec-provenance-map.md`

## Routing Pointers

Use this map through the project-doc route rather than as a standalone bootstrap carrier.

1. Active project-doc bootstrap:
   - `AGENTS.sidecar.md`
2. Current project root map:
   - `../../project-root-map.md`
3. Documentation/system/tooling follow-up:
   - `../../process/documentation-tooling-map.md`
4. Detailed source-lineage follow-up:
   - `current-spec-provenance-map.md`

Activation rule:

1. read this spec map when active product/spec canon questions are active,
2. prefer `../../project-root-map.md` first when the task is still choosing between product/process/project-memory lanes,
3. use the provenance companion only when detailed source lineage or absorbed-history questions are active,
4. do not use this file as a replacement for framework root-map routing.

## Current Rule

1. `docs/product/spec/**` is the current prose canon.
2. `vida/config/**` is the executable law home.
3. deleted framework-formation plans/research survive only as provenance in `docs/process/framework-source-lineage-index.md`, not as active product canon.

## Shared Runtime Spine Rule

1. The active `TaskFlow v1` modernization program and `VIDA 1.0` share one semantic runtime-spec spine.
2. Stable product-law portions of that spine are promoted here into `docs/product/spec/**`.
3. Historical formation inputs for that spine are preserved only in `docs/process/framework-source-lineage-index.md`.
4. The TaskFlow runtime-family implementation surfaces are the current transitional implementation substrate and donor bridge for the `TaskFlow v1` line, not a separate semantic canon.

## Project Documentation Rule

1. Root repository docs, `docs/product/**`, `docs/process/**`, and `docs/project-memory/**` are part of the active project documentation surface.
2. Active canonical markdown documents in those lanes must carry machine-readable footer metadata and a sibling `*.changelog.jsonl`.
3. During the pre-runtime phase, only the latest markdown revision is kept as the active body; historical lineage stays in sidecars and git history.

-----
artifact_path: product/spec/current-spec-map
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-16'
schema_version: '1'
status: canonical
source_path: docs/product/spec/current-spec-map.md
created_at: '2026-03-09T20:28:59+02:00'
updated_at: 2026-03-16T11:34:41.626135419Z
changelog_ref: current-spec-map.changelog.jsonl
