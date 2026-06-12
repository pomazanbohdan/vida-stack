# Product Canon Index

Purpose: make current VIDA product law discoverable without reading historical framework evidence by guesswork.

Top-level execution model:

1. `agent` is the execution carrier (model/tier/cost/effectiveness), not runtime lane identity.
2. `role` is a separate runtime activation state.
3. Runtime selection order is fixed: capability/admissibility -> local score/telemetry guard -> cheapest eligible carrier.

Canonical split:

1. `docs/product/spec/` is the current product spec home.
2. [vida/config](../../vida/config) is the executable law home.
3. `docs/product/research/` is the promoted product research lane.
4. `docs/process/` is the project process lane.
5. `docs/project-memory/` is the project-memory source lane.
6. Historical framework-formation source lineage is retained in Git history and active artifact sidecars after plan/research promotion cleanup.

Current entrypoints:

1. [project-root-map.md](../project-root-map.md)
2. [current-spec-map.md](spec/current-spec-map.md)
3. [current-spec-catalog.md](spec/current-spec-catalog.md)
4. [docs/product/spec/README.md](spec/README.md)
6. [instruction-artifact-model.md](spec/instruction-artifact-model.md)
7. [skill-management-and-activation-law.md](spec/skill-management-and-activation-law.md)
8. [instruction-migration-map.md](spec/instruction-migration-map.md)
9. [project-documentation-law.md](spec/project-documentation-law.md)
10. [canonical-documentation-and-inventory-layer-matrix.md](spec/canonical-documentation-and-inventory-layer-matrix.md)
11. [docs/process/README.md](../process/README.md)
12. [docs/process/documentation-tooling-map.md](../process/documentation-tooling-map.md)
13. [docs/process/github-issues-triage-guide.md](../process/github-issues-triage-guide.md)
14. [docs/project-memory/README.md](../project-memory/README.md)
15. [repository-two-project-surface-model.md](spec/repository-two-project-surface-model.md)
16. [framework-project-documentation-layer-model.md](spec/framework-project-documentation-layer-model.md)
17. [root-map-and-runtime-surface-model.md](spec/root-map-and-runtime-surface-model.md)
18. [canonical-runtime-layer-matrix.md](spec/canonical-runtime-layer-matrix.md)
19. [functional-matrix-protocol.md](spec/functional-matrix-protocol.md)
20. [agent-role-skill-profile-flow-model.md](spec/agent-role-skill-profile-flow-model.md)
21. [agent-lane-selection-and-conversation-mode-model.md](spec/agent-lane-selection-and-conversation-mode-model.md)
22. [compiled-autonomous-delivery-runtime-architecture.md](spec/compiled-autonomous-delivery-runtime-architecture.md)
23. [emerging-architectural-patterns-model.md](spec/emerging-architectural-patterns-model.md)
41. [compiled-runtime-bundle-contract.md](spec/compiled-runtime-bundle-contract.md)
42. [project-activation-and-configurator-model.md](spec/project-activation-and-configurator-model.md)
43. [team-coordination-model.md](spec/team-coordination-model.md)
44. [status-families-and-query-surface-model.md](spec/status-families-and-query-surface-model.md)
45. [project-protocol-promotion-law.md](spec/project-protocol-promotion-law.md)
46. [project-document-naming-law.md](spec/project-document-naming-law.md)
47. [github-public-repository-law.md](spec/github-public-repository-law.md)
48. [release-build-packaging-law.md](spec/release-build-packaging-law.md)
49. [taskflow-protocol-runtime-binding-model.md](spec/taskflow-protocol-runtime-binding-model.md)
50. [embedded-runtime-and-editable-projection-model.md](spec/embedded-runtime-and-editable-projection-model.md)
51. [runtime-paths-and-derived-cache-model.md](spec/runtime-paths-and-derived-cache-model.md)
52. [user-facing-runtime-flow-and-operating-loop-model.md](spec/user-facing-runtime-flow-and-operating-loop-model.md)
53. [bootstrap-carriers-and-project-activator-model.md](spec/bootstrap-carriers-and-project-activator-model.md)
54. [execution-preparation-and-developer-handoff-model.md](spec/execution-preparation-and-developer-handoff-model.md)
55. [taskflow-execution-semantics-and-scheduler-design.md](spec/taskflow-execution-semantics-and-scheduler-design.md)
56. [instruction-packing-and-caching-survey.md](research/instruction-packing-and-caching-survey.md)
57. [agent-governance-and-policy-hardening-survey.md](research/agent-governance-and-policy-hardening-survey.md)
58. [langgraph-runtime-patterns-survey.md](research/langgraph-runtime-patterns-survey.md)
59. [execution-approval-and-interrupt-resume-survey.md](research/execution-approval-and-interrupt-resume-survey.md)
60. [runtime-framework-open-questions-and-external-patterns-survey.md](research/runtime-framework-open-questions-and-external-patterns-survey.md)
61. [compiled-control-bundle-contract-research.md](research/compiled-control-bundle-contract-research.md)
62. [runtime-memory-state-and-retrieval-research.md](research/runtime-memory-state-and-retrieval-research.md)
63. [db-authority-and-migration-runtime-research.md](research/db-authority-and-migration-runtime-research.md)
64. [runtime-home-and-surface-migration-research.md](research/runtime-home-and-surface-migration-research.md)
65. [derived-cache-delivery-and-invalidation-research.md](research/derived-cache-delivery-and-invalidation-research.md)
66. [embedded-runtime-bootstrap-and-projection-research.md](research/embedded-runtime-bootstrap-and-projection-research.md)
67. [execution-preparation-and-developer-handoff-survey.md](research/execution-preparation-and-developer-handoff-survey.md)
68. [docs/product/research/vida-service-tui-wizard-architecture-research.md](research/vida-service-tui-wizard-architecture-research.md)

Repository project split:

1. active current project surface:
   - `vida-stack`
   - mapped through [AGENTS.sidecar.md](../../AGENTS.sidecar.md) -> [project-root-map.md](../project-root-map.md) -> current docs under [docs](..)
2. extracted secondary bundle:
   - `vida-mobile`
   - preserved locally under `projects/vida-mobile` when that bundle is materialized in the repository
   - not part of the default bootstrap/project-doc path

Repository narrative entrypoints:

1. [README.md](../../README.md)
2. [VERSION-PLAN.md](../../VERSION-PLAN.md)
3. [CONTRIBUTING.md](../../CONTRIBUTING.md)

-----
artifact_path: product/index
artifact_type: product_index
artifact_version: '1'
artifact_revision: 2026-06-12
schema_version: '1'
status: canonical
source_path: docs/product/index.md
created_at: '2026-03-09T20:28:59+02:00'
updated_at: 2026-06-12T00:00:00+03:00
changelog_ref: index.changelog.jsonl
