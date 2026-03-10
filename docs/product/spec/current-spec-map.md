# VIDA Current Spec Map

Status: active canonical map

Revision: `2026-03-09`

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
updated_at: '2026-03-10T03:06:28+02:00'
changelog_ref: current-spec-map.changelog.jsonl
