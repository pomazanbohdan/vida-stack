# VIDA Current Spec Map

Status: active canonical map

Revision: `2026-03-09`

Purpose: define the current product-spec home, show absorbed historical sources, and anchor each current artifact to executable product law.

## Current Canon

1. [partial-development-kernel-scope.md](/home/unnamed/project/vida-stack/docs/product/spec/partial-development-kernel-scope.md)
   Sources: `docs/framework/history/plans/2026-03-08-vida-0.3-state-kernel-schema-spec.md`, `docs/framework/history/plans/2026-03-08-vida-0.3-route-and-receipt-spec.md`
   Config families: `vida/config/machines/**`, `vida/config/routes/**`, `vida/config/policies/**`
2. [canonical-machine-map.md](/home/unnamed/project/vida-stack/docs/product/spec/canonical-machine-map.md)
   Sources: `docs/framework/history/plans/2026-03-08-vida-0.3-state-kernel-schema-spec.md`, `docs/framework/history/plans/2026-03-08-vida-0.3-route-and-receipt-spec.md`
   Config families: `vida/config/machines/**`
3. [receipt-proof-taxonomy.md](/home/unnamed/project/vida-stack/docs/product/spec/receipt-proof-taxonomy.md)
   Sources: `docs/framework/history/plans/2026-03-08-vida-0.3-route-and-receipt-spec.md`
   Config families: `vida/config/receipts/**`, `vida/config/policies/**`
4. [external-pattern-borrow-map.md](/home/unnamed/project/vida-stack/docs/product/spec/external-pattern-borrow-map.md)
   Sources: `docs/framework/history/research/**`, external-source synthesis
   Config families: cross-cutting product law only
5. [projection-listener-checkpoint-kernel.md](/home/unnamed/project/vida-stack/docs/product/spec/projection-listener-checkpoint-kernel.md)
   Sources: Eventuous, Elsa, LangGraph research promoted through the borrow map
   Config families: `vida/config/machines/**`, runtime consumption by `vida-v0`
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
   Sources: `docs/framework/history/plans/2026-03-08-vida-0.3-instruction-kernel-spec.md`, `docs/framework/history/_vida-source/instructions/framework/**`
   Config families: `vida/config/instructions/**`
11. [skill-management-and-activation.md](/home/unnamed/project/vida-stack/docs/product/spec/skill-management-and-activation.md)
   Sources: product migration decisions in this cutover
   Config families: `vida/config/instructions/skills/**`, `vida/config/instructions/activation/**`
12. [instruction-migration-crosswalk.md](/home/unnamed/project/vida-stack/docs/product/spec/instruction-migration-crosswalk.md)
   Sources: `docs/framework/history/_vida-source/docs/agent-definition-protocol.md`, `docs/framework/history/_vida-source/templates/**`, `docs/framework/history/_vida-source/instructions/framework/**`
   Config families: `vida/config/instructions/**`

## Current Rule

1. `docs/product/spec/**` is the current prose canon.
2. `vida/config/**` is the executable law home.
3. `docs/framework/history/**` is evidence/history, not active canon, unless cited here.

## Shared Runtime Spine Rule

1. VIDA `0.2.0` and VIDA `1.0` share one semantic runtime-spec spine.
2. `docs/framework/plans/**` remains the active strategic and execution-spec program layer for that shared runtime spine.
3. Stable product-law portions of that spine are promoted here into `docs/product/spec/**`.
4. `vida-v0/**` is the current transitional implementation substrate for the `0.2.0` line, not a separate semantic canon.
