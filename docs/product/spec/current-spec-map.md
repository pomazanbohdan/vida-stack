# VIDA Current Spec Map

Status: active canonical map

Revision: `2026-03-10`

Purpose: define the current product-spec home, show absorbed historical sources, and anchor each current artifact to executable product law.

## Current Canon

1. [partial-development-kernel-scope.md](/home/unnamed/project/vida-stack/docs/product/spec/partial-development-kernel-scope.md)
   Sources: `docs/framework/plans/vida-0.3-state-kernel-schema-spec.md`, `docs/framework/plans/vida-0.3-route-and-receipt-spec.md`
   Config families: `vida/config/machines/**`, `vida/config/routes/**`, `vida/config/policies/**`
2. [canonical-machine-map.md](/home/unnamed/project/vida-stack/docs/product/spec/canonical-machine-map.md)
   Sources: `docs/framework/plans/vida-0.3-state-kernel-schema-spec.md`, `docs/framework/plans/vida-0.3-route-and-receipt-spec.md`
   Config families: `vida/config/machines/**`
3. [receipt-proof-taxonomy.md](/home/unnamed/project/vida-stack/docs/product/spec/receipt-proof-taxonomy.md)
   Sources: `docs/framework/plans/vida-0.3-route-and-receipt-spec.md`
   Config families: `vida/config/receipts/**`, `vida/config/policies/**`
4. [external-pattern-borrow-map.md](/home/unnamed/project/vida-stack/docs/product/spec/external-pattern-borrow-map.md)
   Sources: `docs/framework/research/**`, external-source synthesis
   Config families: cross-cutting product law only
5. [projection-listener-checkpoint-kernel.md](/home/unnamed/project/vida-stack/docs/product/spec/projection-listener-checkpoint-kernel.md)
   Sources: Eventuous, Elsa, LangGraph research promoted through the borrow map
   Config families: `vida/config/machines/**`, runtime consumption by `taskflow-v0`
6. [gateway-resume-handle-and-trigger-index.md](/home/unnamed/project/vida-stack/docs/product/spec/gateway-resume-handle-and-trigger-index.md)
   Sources: Elsa trigger/bookmark semantics
   Config families: future route/gateway law
7. [machine-definition-lint-law.md](/home/unnamed/project/vida-stack/docs/product/spec/machine-definition-lint-law.md)
   Sources: `python-statemachine` strict validation semantics
   Config families: future machine lint
8. [checkpoint-commit-and-replay-lineage.md](/home/unnamed/project/vida-stack/docs/product/spec/checkpoint-commit-and-replay-lineage.md)
   Sources: Eventuous and LangGraph checkpoint/replay semantics
   Config families: runtime-derived checkpoint law
9. [verification-merge-law.md](/home/unnamed/project/vida-stack/docs/product/spec/verification-merge-law.md)
   Sources: Elsa merge regressions, verification parallelism research
   Config families: future verification routing law
10. [instruction-artifact-model.md](/home/unnamed/project/vida-stack/docs/product/spec/instruction-artifact-model.md)
   Sources: `docs/framework/plans/vida-0.3-instruction-kernel-spec.md`, `vida/config/instructions/**`
   Config families: `vida/config/instructions/**`
11. [skill-management-and-activation.md](/home/unnamed/project/vida-stack/docs/product/spec/skill-management-and-activation.md)
   Sources: product migration decisions in this cutover
   Config families: `vida/config/instructions/skills/**`, `vida/config/instructions/activation/**`
12. [instruction-migration-crosswalk.md](/home/unnamed/project/vida-stack/docs/product/spec/instruction-migration-crosswalk.md)
   Sources: `vida/config/instructions/agent-definitions.protocol.md`, `vida/config/instructions/**`
   Config families: `vida/config/instructions/**`
13. [project-documentation-system.md](/home/unnamed/project/vida-stack/docs/product/spec/project-documentation-system.md)
   Sources: current markdown-first operating model and document-sidecar migration decisions
   Config families: project documentation governance only
14. [canonical-documentation-and-inventory-layers.md](/home/unnamed/project/vida-stack/docs/product/spec/canonical-documentation-and-inventory-layers.md)
   Sources: `docs/product/spec/project-documentation-system.md`, `docs/product/spec/instruction-artifact-model.md`, `docs/framework/plans/vida-0.3-instruction-kernel-spec.md`, `docs/framework/plans/vida-0.3-migration-kernel-spec.md`, `vida/config/**`
   Config families: canonical inventory, validation, mutation, relation, readiness, and runtime-consumption architecture across `vida/config/**`
15. [canonical-inventory-law.md](/home/unnamed/project/vida-stack/docs/product/spec/canonical-inventory-law.md)
   Sources: `docs/product/spec/project-documentation-system.md`, `docs/product/spec/current-spec-map.md`, `docs/product/spec/canonical-documentation-and-inventory-layers.md`, `vida/config/instructions/instruction_catalog.yaml`, `vida/config/instructions/projection_manifest.yaml`
   Config families: canonical inventory, registry structure, coverage, source/projection linkage, and version-tuple visibility across active canon
16. [canonical-relation-law.md](/home/unnamed/project/vida-stack/docs/product/spec/canonical-relation-law.md)
   Sources: `docs/product/spec/project-documentation-system.md`, `docs/product/spec/canonical-documentation-and-inventory-layers.md`, `docs/process/documentation-tooling-map.md`, `codex-v0/codex.py`
   Config families: canonical dependencies, direct/reverse references, artifact impact, task impact, and relation validation across active canon
17. [canonical-runtime-readiness-law.md](/home/unnamed/project/vida-stack/docs/product/spec/canonical-runtime-readiness-law.md)
   Sources: `docs/framework/plans/vida-0.3-instruction-kernel-spec.md`, `docs/framework/plans/vida-0.3-migration-kernel-spec.md`, `docs/framework/research/canonical-runtime-readiness-external-patterns.md`, `vida/config/migration/**`, `vida/config/instructions/**`, `codex-v0/codex.py`
   Config families: source-version tuples, compatibility classes, projection parity, canonical bundles, boot-gate artifacts, and fail-closed readiness verdicts across active canon
18. [canonical-layer-documentation-template.md](/home/unnamed/project/vida-stack/docs/product/spec/canonical-layer-documentation-template.md)
   Sources: `docs/product/spec/canonical-documentation-and-inventory-layers.md`, promoted Layer 1 through Layer 7 specs, and current documentation operation law
   Config families: canonical layer-law authoring shape for Layers 1 through 7
19. [framework-project-documentation-layering.md](/home/unnamed/project/vida-stack/docs/product/spec/framework-project-documentation-layering.md)
   Sources: `AGENTS.md`, `AGENTS.sidecar.md`, `vida/config/instructions/system-maps.framework-map-protocol.md`, `vida/config/instructions/system-maps.framework-index.md`, and the current framework/project documentation restructuring decisions
   Config families: framework canon vs role/bootstrap/governance/project documentation layering, derivation boundaries, two-map bootstrap, and root-map requirements
20. [root-map-and-runtime-surface-model.md](/home/unnamed/project/vida-stack/docs/product/spec/root-map-and-runtime-surface-model.md)
   Sources: `AGENTS.md`, `AGENTS.sidecar.md`, `vida/config/instructions/system-maps.framework-map-protocol.md`, `vida/config/instructions/system-maps.framework-index.md`, and the current runtime-surface optimization decisions for `codex`, `taskflow`, and future runtime families
   Config families: framework root map, project root map, runtime-family submaps, template maps, and activation-trigger discoverability across active canon
21. [canonical-runtime-layer-matrix.md](/home/unnamed/project/vida-stack/docs/product/spec/canonical-runtime-layer-matrix.md)
   Sources: `docs/framework/plans/vida-0.3-route-and-receipt-spec.md`, `docs/framework/plans/vida-0.3-db-first-runtime-spec.md`, `docs/product/spec/partial-development-kernel-scope.md`, `docs/product/spec/canonical-machine-map.md`, `docs/product/spec/projection-listener-checkpoint-kernel.md`, `docs/product/spec/gateway-resume-handle-and-trigger-index.md`, `docs/product/spec/verification-merge-law.md`, `docs/product/spec/canonical-runtime-readiness-law.md`, `vida/config/instructions/runtime-instructions.taskflow-protocol.md`, `vida/config/instructions/runtime-instructions.agent-handoff-context-protocol.md`, `vida/config/instructions/runtime-instructions.checkpoint-replay-recovery-protocol.md`, and `docs/framework/research/vida-1.0-agent-runtime-external-alignment.md`
   Config families: layered runtime capability progression across `vida/config/**`, `taskflow-v0/**`, runtime ledgers, readiness gates, and future direct runtime consumption
22. [agent-role-skill-profile-flow-model.md](/home/unnamed/project/vida-stack/docs/product/spec/agent-role-skill-profile-flow-model.md)
   Sources: `vida/config/instructions/agent-definitions.protocol.md`, `vida/config/instructions/agent-definitions.role-profile-protocol.md`, `vida/config/instructions/instruction-contracts.agent-system-protocol.md`, `vida/config/instructions/runtime-instructions.project-agent-extension-protocol.md`, `docs/product/spec/instruction-artifact-model.md`, and the current runtime/project-extension design decisions
   Config families: framework role law, project role/skill/profile/flow activation through `vida.config.yaml`, project-owned agent-extension registries, and runtime validation for `taskflow-v0`
23. [agent-role-selection-and-conversation-mode-model.md](/home/unnamed/project/vida-stack/docs/product/spec/agent-role-selection-and-conversation-mode-model.md)
   Sources: `vida/config/instructions/agent-definitions.orchestrator-entry.md`, `vida/config/instructions/command-instructions.use-case-packs.md`, `vida/config/instructions/command-instructions.form-task-protocol.md`, `vida/config/instructions/runtime-instructions.taskflow-protocol.md`, `vida/config/instructions/runtime-instructions.agent-role-selection-protocol.md`, and the current role-selection/runtime-conversation design decisions
   Config families: overlay-driven auto-role selection, bounded conversational modes, one-task scope/PBI discussion, and lawful handoff into pack/taskflow routing
24. [repository-two-project-surface-model.md](/home/unnamed/project/vida-stack/docs/product/spec/repository-two-project-surface-model.md)
   Sources: `AGENTS.sidecar.md`, `vida/config/instructions/system-maps.framework-map-protocol.md`, root `vida.config.yaml`, and the current repository separation decisions for `vida-stack` and extracted `vida-mobile`
   Config families: active current-project routing, extracted second-project bundle boundaries, root config continuity, and two-project repository map discipline
25. [party-chat-v2-problem-party-integration.md](/home/unnamed/project/vida-stack/docs/product/spec/party-chat-v2-problem-party-integration.md)
   Sources: Airtable `Vida` base `Table 1` records `Party Chat v2 Spec — Part 1/4` through `Part 4/4`, `vida/config/instructions/runtime-instructions.problem-party-protocol.md`, `docs/product/spec/agent-role-skill-profile-flow-model.md`, and the current project extension/runtime integration decisions
   Config families: `docs/process/agent-extensions/**`, `vida.config.yaml`, `.vida/logs/problem-party/**`, single-agent or multi-agent Party Chat execution plans, and runtime consumption by `taskflow-v0`
26. [autonomous-report-continuation.md](/home/unnamed/project/vida-stack/docs/product/spec/autonomous-report-continuation.md)
   Sources: `vida/config/instructions/agent-definitions.orchestrator-entry.md`, `vida/config/instructions/command-instructions.command-layer-protocol.md`, `vida/config/instructions/command-instructions.vida-research.md`, `vida/config/instructions/runtime-instructions.spec-intake-protocol.md`, `vida/config/instructions/runtime-instructions.taskflow-protocol.md`, `vida/config/instructions/runtime-instructions.project-overlay-protocol.md`, and the current overlay/routing continuation decisions
   Config families: `vida.config.yaml`, `vida/config/instructions/**`, TaskFlow routing and autonomous execution behavior
27. [taskflow-v0.2.0-modular-runtime-refactor-plan.md](/home/unnamed/project/vida-stack/docs/product/spec/taskflow-v0.2.0-modular-runtime-refactor-plan.md)
   Sources: `docs/product/spec/canonical-runtime-layer-matrix.md`, `docs/product/spec/party-chat-v2-problem-party-integration.md`, `vida/config/instructions/runtime-instructions.taskflow-protocol.md`, `vida/config/instructions/runtime-instructions.problem-party-protocol.md`, `taskflow-v0/src/vida.nim`, `taskflow-v0/src/core/config.nim`, `taskflow-v0/src/state/problem_party.nim`, and the current modular runtime refactor decisions
   Config families: `taskflow-v0/**`, `vida/config/instructions/**`, runtime feature registration, shared runtime kernel, provider registry, modular config validation, and final `0.2.0` refactor backlog

## Routing Pointers

Use this map through the project-doc route rather than as a standalone bootstrap carrier.

1. Active project-doc bootstrap:
   - `AGENTS.sidecar.md`
2. Current project root map:
   - `docs/project-root-map.md`
3. Documentation/system/tooling follow-up:
   - `docs/process/documentation-tooling-map.md`

Activation rule:

1. read this spec map when product/spec canon questions are active,
2. prefer `docs/project-root-map.md` first when the task is still choosing between product/process/project-memory lanes,
3. do not use this file as a replacement for framework root-map routing.

## Current Rule

1. `docs/product/spec/**` is the current prose canon.
2. `vida/config/**` is the executable law home.
3. `docs/framework/plans/**` and `docs/framework/research/**` are active framework inputs, not product canon.

## Shared Runtime Spine Rule

1. VIDA `0.2.0` and VIDA `1.0` share one semantic runtime-spec spine.
2. `docs/framework/plans/**` remains the active strategic and execution-spec program layer for that shared runtime spine.
3. Stable product-law portions of that spine are promoted here into `docs/product/spec/**`.
4. `taskflow-v0/**` is the current transitional implementation substrate for the `0.2.0` line, not a separate semantic canon.

## Project Documentation Rule

1. Root repository docs, `docs/product/**`, `docs/process/**`, and `docs/project-memory/**` are part of the active project documentation surface.
2. Active canonical markdown documents in those lanes must carry machine-readable footer metadata and a sibling `*.changelog.jsonl`.
3. During the pre-runtime phase, only the latest markdown revision is kept as the active body; historical lineage stays in sidecars and git history.

-----
artifact_path: product/spec/current-spec-map
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: docs/product/spec/current-spec-map.md
created_at: '2026-03-09T20:28:59+02:00'
updated_at: '2026-03-10T18:05:00+02:00'
changelog_ref: current-spec-map.changelog.jsonl
