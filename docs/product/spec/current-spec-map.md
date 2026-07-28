# VIDA Current Spec Map

Status: active canonical map
Revision: 2026-06-25

Purpose: provide the short routing map for the active current product/spec canon after the detailed catalog was split into a companion document.

Companion rule:

1. Use this map first for product/spec routing.
2. Use [index.md](index.md) for local product/spec orientation.
3. Use [current-spec-catalog.md](current-spec-catalog.md) for the detailed active artifact catalog and config-family notes.
4. Do not expand this map back into a full catalog; register detailed entries in the catalog companion and keep the owning artifact docs authoritative.

## Canonical Entry Points

1. [docs/product/index.md](../index.md)
   - top-level product canon index for the active repository
2. [docs/product/spec/index.md](index.md)
   - spec-lane orientation and local product/spec home
3. [current-spec-catalog.md](current-spec-catalog.md)
   - detailed active product/spec artifact catalog
4. [project-documentation-law.md](project-documentation-law.md)
   - project documentation ownership and canonical state law
5. [project-document-naming-law.md](project-document-naming-law.md)
   - project-owned docs naming grammar and owner-directory terminal role rules
6. [canonical-documentation-and-inventory-layer-matrix.md](canonical-documentation-and-inventory-layer-matrix.md)
   - documentation/product alignment matrix
7. [canonical-runtime-layer-matrix.md](canonical-runtime-layer-matrix.md)
   - runtime capability layering matrix
8. [canonical-runtime-readiness-law.md](canonical-runtime-readiness-law.md)
   - runtime readiness gate law used by the project
9. [agent-role-skill-profile-flow-model.md](agent-role-skill-profile-flow-model.md)
   - role, carrier, profile, and flow model
10. [release-build-packaging-law.md](release-build-packaging-law.md)
    - release build and packaging ownership law
11. [taskflow-protocol-runtime-binding-model.md](taskflow-protocol-runtime-binding-model.md)
    - TaskFlow/runtime binding model
12. [compiled-autonomous-delivery-runtime-architecture.md](compiled-autonomous-delivery-runtime-architecture.md)
    - compiled autonomous delivery runtime architecture
13. [bootstrap-carriers-and-project-activator-model.md](bootstrap-carriers-and-project-activator-model.md)
    - bootstrap carrier and project activator model
14. [docs/process/documentation-tooling-map.md](../../process/documentation-tooling-map.md)
    - DocFlow and documentation operator-command map
15. [protocol-authoring-and-token-economy-law.md](protocol-authoring-and-token-economy-law.md)
    - protocol authoring, compression, validation, and token-budget law
16. [typed-transition-state-store-extraction-contract.md](typed-transition-state-store-extraction-contract.md)
    - typed transition/state-store extraction design contract for the active shared extraction epic
17. [ldrk-baseline/execution-preparation.md](ldrk-baseline/execution-preparation.md)
    - LDRK `ldr-001` baseline inventory execution-preparation packet
18. [ldrk-baseline/drift-map.md](ldrk-baseline/drift-map.md)
    - generated LDRK `ldr-001` runtime mutation, classifier, and host-bridge drift baseline
19. [ldrk-baseline/deletion-candidates.md](ldrk-baseline/deletion-candidates.md)
    - generated LDRK `ldr-001` command, classifier, and direct-mutation deletion candidate baseline
20. [ldrk-operation-catalog/operation-cli-map.json](ldrk-operation-catalog/operation-cli-map.json)
    - generated LDRK `ldr-003` canonical operation-to-CLI map
21. [ldrk-operation-catalog/before-after-command-tree.md](ldrk-operation-catalog/before-after-command-tree.md)
    - generated LDRK `ldr-003` current-to-target command tree
22. [ldrk-operation-catalog/top-ten-operator-workflow-walkthrough.md](ldrk-operation-catalog/top-ten-operator-workflow-walkthrough.md)
    - generated LDRK `ldr-003` top-ten operator workflow walkthrough
23. [local-durable-runtime-kernel-architecture-and-migration-law.md](local-durable-runtime-kernel-architecture-and-migration-law.md)
    - accepted LDRK `ldr-004` architecture and migration law
24. [../decisions/ldr-004-local-durable-runtime-kernel-architecture-adr.md](../decisions/ldr-004-local-durable-runtime-kernel-architecture-adr.md)
    - accepted LDRK `ldr-004` architecture decision record
25. [tower-based-canonical-command-pipeline-phase-design.md](tower-based-canonical-command-pipeline-phase-design.md)
    - proposed LDRK `ldr-040` Tower-based canonical command pipeline phase design
26. [multi-agent-stage-ensemble-contract.md](multi-agent-stage-ensemble-contract.md)
    - multi-agent stage attempt ledger and consolidation contract
27. [runworkflow-aggregate-hierarchical-statig-machin-design.md](runworkflow-aggregate-hierarchical-statig-machin-design.md)
    - proposed LDRK `ldr-020` RunWorkflow aggregate and hierarchical Statig machine design packet

## Detailed Catalog Companions

1. [current-spec-catalog.md](current-spec-catalog.md)
   - active current canon list, grouped by product/spec area
2. [docs/product/index.md](../index.md)
   - repository-wide product/process/research index
3. [docs/project-root-map.md](../../project-root-map.md)
   - active project root map that routes into this spec map
4. [AGENTS.sidecar.md](../../../AGENTS.sidecar.md)
   - bootstrap-visible project documentation map

## Routing Pointers

1. Documentation ownership, naming, inventory, protocol-authoring, and token-economy questions route to [project-documentation-law.md](project-documentation-law.md), [project-document-naming-law.md](project-document-naming-law.md), [protocol-authoring-and-token-economy-law.md](protocol-authoring-and-token-economy-law.md), [canonical-documentation-and-inventory-layer-matrix.md](canonical-documentation-and-inventory-layer-matrix.md), and [docs/process/documentation-tooling-map.md](../../process/documentation-tooling-map.md).
2. Runtime readiness, runtime layering, and operator-surface questions route to [canonical-runtime-readiness-law.md](canonical-runtime-readiness-law.md), [canonical-runtime-layer-matrix.md](canonical-runtime-layer-matrix.md), and the active runtime contract/profile specs.
3. Role, carrier, skill, profile, lane, and flow questions route to [agent-role-skill-profile-flow-model.md](agent-role-skill-profile-flow-model.md) and [docs/process/agent-extensions/index.md](../../process/agent-extensions/index.md).
4. Detailed artifact lookup routes to [current-spec-catalog.md](current-spec-catalog.md).

5. Versioned Rhai policy runtime routing uses the canonical [design](versioned-rhai-policy-runtime-design.md), its linked [authority ADR](../decisions/versioned-rhai-policy-runtime-authority-adr.md), and the [authoring runbook](../../process/rhai-policy-authoring-runbook.md); this map and its changelog are generated/documentation projections and do not replace those owners.

## Current Rule

1. The current canon is the active product/spec state for the repository.
2. Historical framework-formation evidence is provenance only unless re-promoted into a current canonical owner.
3. New current artifacts must be registered in the owning index/map and, when detailed lookup is needed, in [current-spec-catalog.md](current-spec-catalog.md).
4. Do not use extracted secondary bundles as the default current project surface unless the task explicitly targets them.

## Shared Runtime Spine Rule

1. Shared runtime helpers and executable projections live in [vida/config](../../../vida/config) and the runtime crates.
2. Product/spec docs describe the canonical contract, while executable runtime surfaces prove and enforce it.
3. When docs and executable runtime surfaces disagree, record the disagreement as a bounded runtime/documentation defect instead of silently treating either surface as disposable.

## Project Documentation Rule

1. Project-visible documentation surfaces must stay discoverable from [AGENTS.sidecar.md](../../../AGENTS.sidecar.md), [docs/project-root-map.md](../../project-root-map.md), and [docs/product/index.md](../index.md) when they become bootstrap or product-spec entrypoints.
2. Companion documents may reduce map size, but they must not hide active canonical docs from the owning maps.
3. Changelog JSONL files record map/catalog routing changes as a documentation-system mutation, not as runtime behavior proof.

## Active Canonical Spec Registration Manifest

The following active-canon spec documents are registered here exactly once for DocFlow owner-map discovery; detailed descriptions remain in [current-spec-catalog.md](current-spec-catalog.md).

1. [adr-team-flow-state-machine-owner.md](adr-team-flow-state-machine-owner.md)
2. [agent-lane-selection-and-conversation-mode-model.md](agent-lane-selection-and-conversation-mode-model.md)
3. [agent-mode-test-first-delivery-flow-model.md](agent-mode-test-first-delivery-flow-model.md)
4. [authoritative-state-access-serialization-contract.md](authoritative-state-access-serialization-contract.md)
5. [authoritative-state-lock-recovery-contract.md](authoritative-state-lock-recovery-contract.md)
6. [autonomous-report-continuation-law.md](autonomous-report-continuation-law.md)
7. [canonical-inventory-law.md](canonical-inventory-law.md)
8. [canonical-layer-documentation-template.md](canonical-layer-documentation-template.md)
9. [canonical-machine-map.md](canonical-machine-map.md)
10. [canonical-operator-command-map-export-contract.md](canonical-operator-command-map-export-contract.md)
11. [canonical-relation-law.md](canonical-relation-law.md)
12. [carrier-model-profile-selection-runtime-model.md](carrier-model-profile-selection-runtime-model.md)
13. [checkpoint-commit-and-replay-model.md](checkpoint-commit-and-replay-model.md)
14. [closure-admission-evidence-table-contract.md](closure-admission-evidence-table-contract.md)
15. [codex-app-agent-lifecycle-cleanup-contract.md](codex-app-agent-lifecycle-cleanup-contract.md)
16. [codex-host-agent-boundary-and-cli-bridge-contract.md](codex-host-agent-boundary-and-cli-bridge-contract.md)
17. [compiled-runtime-bundle-contract.md](compiled-runtime-bundle-contract.md)
18. [config-driven-host-system-runtime-contract.md](config-driven-host-system-runtime-contract.md)
19. [continuation-binding-fail-closed-contract.md](continuation-binding-fail-closed-contract.md)
20. [continuation-seeded-dispatch-bridge-contract.md](continuation-seeded-dispatch-bridge-contract.md)
21. [dead-code-removal-admission-contract.md](dead-code-removal-admission-contract.md)
22. [design-backed-implementation-routing-contract.md](design-backed-implementation-routing-contract.md)
23. [design-backed-implementation-seeding-scope-contract.md](design-backed-implementation-seeding-scope-contract.md)
24. [development-flow-catalog-schema-contract.md](development-flow-catalog-schema-contract.md)
25. [embedded-runtime-and-editable-projection-model.md](embedded-runtime-and-editable-projection-model.md)
26. [emerging-architectural-patterns-model.md](emerging-architectural-patterns-model.md)
27. [execution-preparation-and-developer-handoff-model.md](execution-preparation-and-developer-handoff-model.md)
28. [extensibility-and-output-template-model.md](extensibility-and-output-template-model.md)
29. [external-architecture-baseline.md](external-architecture-baseline.md)
30. [external-cli-carrier-hardening-contract.md](external-cli-carrier-hardening-contract.md)
31. [external-coach-retry-fallback-contract.md](external-coach-retry-fallback-contract.md)
32. [external-pattern-borrow-map.md](external-pattern-borrow-map.md)
33. [fail-closed-resume-closure-truth-contract.md](fail-closed-resume-closure-truth-contract.md)
34. [fast-high-signal-pre-commit-contract.md](fast-high-signal-pre-commit-contract.md)
35. [feature-design-and-adr-model.md](feature-design-and-adr-model.md)
36. [framework-project-documentation-layer-model.md](framework-project-documentation-layer-model.md)
37. [functional-matrix-protocol.md](functional-matrix-protocol.md)
38. [gateway-resume-handle-and-trigger-index.md](gateway-resume-handle-and-trigger-index.md)
39. [github-public-repository-law.md](github-public-repository-law.md)
40. [host-agent-bridge-adapter-contract.md](host-agent-bridge-adapter-contract.md)
41. [host-agent-layer-status-matrix.md](host-agent-layer-status-matrix.md)
42. [hybrid-host-executor-semantics-model.md](hybrid-host-executor-semantics-model.md)
43. [implementation-backend-admissibility-selection-truth-contract.md](implementation-backend-admissibility-selection-truth-contract.md)
44. [implementation-closure-write-evidence-contract.md](implementation-closure-write-evidence-contract.md)
45. [instruction-artifact-model.md](instruction-artifact-model.md)
46. [instruction-migration-map.md](instruction-migration-map.md)
47. [internal-backend-executor-route-policy-contract.md](internal-backend-executor-route-policy-contract.md)
48. [internal-codex-agent-execution-fail-closed-contract.md](internal-codex-agent-execution-fail-closed-contract.md)
49. [internal-codex-timeout-reconciliation-contract.md](internal-codex-timeout-reconciliation-contract.md)
50. [internal-dispatch-timeout-handoff-contract.md](internal-dispatch-timeout-handoff-contract.md)
51. [lane-supersede-shared-truth-envelope-contract.md](lane-supersede-shared-truth-envelope-contract.md)
52. [lawful-closure-continuation-rebinding-contract.md](lawful-closure-continuation-rebinding-contract.md)
53. [mempalace-vida-memory-implementation-model.md](mempalace-vida-memory-implementation-model.md)
54. [model-provider-price-catalog-lifecycle-contract.md](model-provider-price-catalog-lifecycle-contract.md)
55. [multi-orchestrator-session-ownership-claims-contract.md](multi-orchestrator-session-ownership-claims-contract.md)
56. [operational-state-and-synchronization-model.md](operational-state-and-synchronization-model.md)
57. [operator-output-envelope-and-bounded-rendering-contract.md](operator-output-envelope-and-bounded-rendering-contract.md)
58. [ops-state-runtime-evidence-hygiene-contract.md](ops-state-runtime-evidence-hygiene-contract.md)
59. [orchestrator-runtime-contract-hardening-contract.md](orchestrator-runtime-contract-hardening-contract.md)
60. [oversized-runtime-module-split-contract.md](oversized-runtime-module-split-contract.md)
61. [partial-development-kernel-model.md](partial-development-kernel-model.md)
62. [party-chat-v2-problem-party-model.md](party-chat-v2-problem-party-model.md)
63. [pi-primary-environment-agent-carrier-spec.md](pi-primary-environment-agent-carrier-spec.md)
64. [production-observability-and-operator-baselines-contract.md](production-observability-and-operator-baselines-contract.md)
65. [project-activation-and-configurator-model.md](project-activation-and-configurator-model.md)
66. [project-agent-first-delegation-contract.md](project-agent-first-delegation-contract.md)
67. [project-protocol-promotion-law.md](project-protocol-promotion-law.md)
68. [projection-listener-checkpoint-model.md](projection-listener-checkpoint-model.md)
69. [prompt-lifecycle-evaluation-and-safety-baseline-contract.md](prompt-lifecycle-evaluation-and-safety-baseline-contract.md)
70. [qwen-cli-reference-only-carrier-contract.md](qwen-cli-reference-only-carrier-contract.md)
71. [receipt-and-proof-law.md](receipt-and-proof-law.md)
72. [reconciled-runtime-projection-output-contract.md](reconciled-runtime-projection-output-contract.md)
73. [release-admission-evidence-detection-contract.md](release-admission-evidence-detection-contract.md)
74. [repository-two-project-surface-model.md](repository-two-project-surface-model.md)
75. [requirements-control-plane-runtime-implementation-model.md](requirements-control-plane-runtime-implementation-model.md)
76. [requirements-control-plane-state-model.md](requirements-control-plane-state-model.md)
77. [requirements-documentation-control-plane.md](requirements-documentation-control-plane.md)
78. [retrieval-identity-memory-governance-contract.md](retrieval-identity-memory-governance-contract.md)
79. [root-map-and-runtime-surface-model.md](root-map-and-runtime-surface-model.md)
80. [runtime-library-fsm-pilot-decision.md](runtime-library-fsm-pilot-decision.md)
81. [runtime-paths-and-derived-cache-model.md](runtime-paths-and-derived-cache-model.md)
82. [runtime-web-restart-current-repo-command-contract.md](runtime-web-restart-current-repo-command-contract.md)
83. [selector-precedence-bounded-repair-contract.md](selector-precedence-bounded-repair-contract.md)
84. [session-scoped-orchestrator-protocol-foundation-contract.md](session-scoped-orchestrator-protocol-foundation-contract.md)
85. [skill-management-and-activation-law.md](skill-management-and-activation-law.md)
86. [spec-compliant-exception-path-takeover-surface-contract.md](spec-compliant-exception-path-takeover-surface-contract.md)
87. [specification-lane-scope-hardening-contract.md](specification-lane-scope-hardening-contract.md)
88. [stale-blocked-dispatch-artifact-reconciliation-contract.md](stale-blocked-dispatch-artifact-reconciliation-contract.md)
89. [status-families-and-query-surface-model.md](status-families-and-query-surface-model.md)
90. [task-close-closure-truth-exception-contract.md](task-close-closure-truth-exception-contract.md)
91. [task-graph-adaptive-planner-contract.md](task-graph-adaptive-planner-contract.md)
92. [taskflow-execution-semantics-scheduler-contract.md](taskflow-execution-semantics-scheduler-contract.md)
93. [taskflow-happy-path-test-catalog-contract.md](taskflow-happy-path-test-catalog-contract.md)
94. [taskflow-task-command-parity-proxy-contract.md](taskflow-task-command-parity-proxy-contract.md)
95. [team-coordination-model.md](team-coordination-model.md)
96. [test-first-runtime-defect-remediation-model.md](test-first-runtime-defect-remediation-model.md)
97. [unified-hybrid-runtime-selection-policy-contract.md](unified-hybrid-runtime-selection-policy-contract.md)
98. [user-facing-runtime-flow-and-operating-loop-model.md](user-facing-runtime-flow-and-operating-loop-model.md)
99. [verification-merge-law.md](verification-merge-law.md)
100. [vida-coder-service-mode-executor-contract.md](vida-coder-service-mode-executor-contract.md)
101. [vida-service-tui-wizard-execution-spec.md](vida-service-tui-wizard-execution-spec.md)
102. [workflow-policy-loader-service-orchestration-contract.md](workflow-policy-loader-service-orchestration-contract.md)

-----
artifact_path: product/spec/current-spec-map
artifact_type: product_spec
artifact_version: '1'
artifact_revision: 2026-06-25
schema_version: '1'
status: canonical
source_path: docs/product/spec/current-spec-map.md
created_at: '2026-03-10T10:20:00+02:00'
updated_at: '2026-07-12T06:31:24.4228302Z'
changelog_ref: current-spec-map.changelog.jsonl
