# VIDA Current Spec Provenance Map

Status: active canonical companion map

Revision: `2026-04-08`

Purpose: define the detailed source lineage and absorbed historical inputs for the active product-spec canon while the short registry surface lives in `current-spec-map.md`.

Companion rule:
1. `current-spec-map.md` is the short active registry.
2. This document carries the expanded `Sources` lineage for the same canon.
3. This document is provenance/supporting context, not a separate owner-law surface.

## Current Canon Provenance

### Core

1. [partial-development-kernel-model.md](partial-development-kernel-model.md)
   Sources: promoted state/route kernel lineage preserved in `docs/process/framework-source-lineage-index.md`
2. [canonical-machine-map.md](canonical-machine-map.md)
   Sources: promoted machine/state lineage preserved in `docs/process/framework-source-lineage-index.md`
3. [receipt-and-proof-law.md](receipt-and-proof-law.md)
   Sources: `docs/process/framework-source-lineage-index.md`
4. [external-pattern-borrow-map.md](external-pattern-borrow-map.md)
   Sources: `docs/process/framework-source-lineage-index.md`, external-source synthesis
5. [projection-listener-checkpoint-model.md](projection-listener-checkpoint-model.md)
   Sources: Eventuous, Elsa, LangGraph research promoted through the borrow map
6. [gateway-resume-handle-and-trigger-index.md](gateway-resume-handle-and-trigger-index.md)
   Sources: Elsa trigger/bookmark semantics
7. [machine-definition-lint-law.md](machine-definition-lint-law.md)
   Sources: `python-statemachine` strict validation semantics
8. [checkpoint-commit-and-replay-model.md](checkpoint-commit-and-replay-model.md)
   Sources: Eventuous and LangGraph checkpoint/replay semantics
9. [verification-merge-law.md](verification-merge-law.md)
   Sources: Elsa merge regressions, verification parallelism research
10. [instruction-artifact-model.md](instruction-artifact-model.md)
    Sources: promoted instruction-kernel lineage in `docs/process/framework-source-lineage-index.md`, `vida/config/instructions/**`
11. [skill-management-and-activation-law.md](skill-management-and-activation-law.md)
    Sources: product migration decisions in this cutover
12. [instruction-migration-map.md](instruction-migration-map.md)
    Sources: `agent-definitions/model.agent-definitions-contract.md`, `vida/config/instructions/**`

### Documentation And Inventory

1. [project-documentation-law.md](project-documentation-law.md)
   Sources: current markdown-first operating model and document-sidecar migration decisions
2. [canonical-documentation-and-inventory-layer-matrix.md](canonical-documentation-and-inventory-layer-matrix.md)
   Sources: `docs/product/spec/project-documentation-law.md`, `docs/product/spec/instruction-artifact-model.md`, promoted instruction/migration lineage in `docs/process/framework-source-lineage-index.md`, `vida/config/**`
3. [canonical-inventory-law.md](canonical-inventory-law.md)
   Sources: `docs/product/spec/project-documentation-law.md`, `current-spec-map.md`, `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`, `instruction_catalog.yaml`, `projection_manifest.yaml`
4. [canonical-relation-law.md](canonical-relation-law.md)
   Sources: `docs/product/spec/project-documentation-law.md`, `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`, `docs/process/documentation-tooling-map.md`, `vida docflow`
5. [canonical-runtime-readiness-law.md](canonical-runtime-readiness-law.md)
   Sources: promoted instruction/migration/readiness lineage in `docs/process/framework-source-lineage-index.md`, `vida/config/migration/**`, `vida/config/instructions/**`, `vida docflow`
6. [canonical-layer-documentation-template.md](canonical-layer-documentation-template.md)
   Sources: `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`, promoted Layer 1 through Layer 7 specs, and current documentation operation law
7. [functional-matrix-protocol.md](functional-matrix-protocol.md)
   Sources: `docs/product/spec/canonical-runtime-layer-matrix.md`, `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`, `docs/product/spec/taskflow-protocol-runtime-binding-model.md`, and the current need to keep layered capability maps owner-linked, code-linked, seam-explicit, and proof-linked
8. [framework-project-documentation-layer-model.md](framework-project-documentation-layer-model.md)
   Sources: `AGENTS.md`, `AGENTS.sidecar.md`, `system-maps/framework.map.md`, `system-maps/framework.index.md`, and the current framework/project documentation restructuring decisions
9. [root-map-and-runtime-surface-model.md](root-map-and-runtime-surface-model.md)
   Sources: `AGENTS.md`, `AGENTS.sidecar.md`, `system-maps/framework.map.md`, `system-maps/framework.index.md`, and the current runtime-surface optimization decisions for `DocFlow`, `taskflow`, and future runtime families
10. [project-document-naming-law.md](project-document-naming-law.md)
    Sources: `docs/product/spec/project-documentation-law.md`, `docs/product/spec/framework-project-documentation-layer-model.md`, `docs/product/spec/instruction-artifact-model.md`, and the current project-doc naming standardization decisions for `docs/**`
11. [feature-design-and-adr-model.md](feature-design-and-adr-model.md)
    Sources: framework source template `docs/framework/templates/feature-design-document.template.md`, project-local materialized template `docs/product/spec/templates/feature-design-document.template.md`, official OpenAI/Anthropic prompt-structuring guidance, Microsoft/AWS ADR guidance, and the current need to keep bounded change design separate from durable decision records

### Runtime And Agent Control

1. [canonical-runtime-layer-matrix.md](canonical-runtime-layer-matrix.md)
   Sources: promoted route/runtime/readiness lineage in `docs/process/framework-source-lineage-index.md`, `docs/product/spec/partial-development-kernel-model.md`, `docs/product/spec/canonical-machine-map.md`, `docs/product/spec/projection-listener-checkpoint-model.md`, `docs/product/spec/gateway-resume-handle-and-trigger-index.md`, `docs/product/spec/verification-merge-law.md`, `docs/product/spec/canonical-runtime-readiness-law.md`, `runtime-instructions/work.taskflow-protocol.md`, `runtime-instructions/lane.agent-handoff-context-protocol.md`, and `runtime-instructions/recovery.checkpoint-replay-recovery-protocol.md`
2. [agent-role-skill-profile-flow-model.md](agent-role-skill-profile-flow-model.md)
   Sources: `agent-definitions/model.agent-definitions-contract.md`, `agent-definitions/role.role-profile-contract.md`, `instruction-contracts/core.agent-system-protocol.md`, `runtime-instructions/work.project-agent-extension-protocol.md`, `docs/product/spec/instruction-artifact-model.md`, and the current runtime/project-extension design decisions
3. [agent-lane-selection-and-conversation-mode-model.md](agent-lane-selection-and-conversation-mode-model.md)
   Sources: `agent-definitions/entry.orchestrator-entry.md`, `command-instructions/routing.use-case-packs-protocol.md`, `command-instructions/planning.form-task-protocol.md`, `runtime-instructions/work.taskflow-protocol.md`, `runtime-instructions/work.agent-lane-selection-protocol.md`, and the current lane-selection/runtime-conversation design decisions
4. [party-chat-v2-problem-party-model.md](party-chat-v2-problem-party-model.md)
   Sources: Airtable `Vida` base `Table 1` records `Party Chat v2 Spec — Part 1/4` through `Part 4/4`, `runtime-instructions/work.problem-party-protocol.md`, `docs/product/spec/agent-role-skill-profile-flow-model.md`, and the current project extension/runtime integration decisions
5. [autonomous-report-continuation-law.md](autonomous-report-continuation-law.md)
   Sources: `agent-definitions/entry.orchestrator-entry.md`, `command-instructions/routing.command-layer-protocol.md`, `command-instructions/operator.vida-research-guide.md`, `runtime-instructions/work.spec-intake-protocol.md`, `runtime-instructions/work.taskflow-protocol.md`, `runtime-instructions/bridge.project-overlay-protocol.md`, and the current overlay/routing continuation decisions
6. [taskflow-v1-runtime-modernization-plan.md](taskflow-v1-runtime-modernization-plan.md)
   Sources: `docs/product/spec/canonical-runtime-layer-matrix.md`, `docs/product/spec/party-chat-v2-problem-party-model.md`, `runtime-instructions/work.taskflow-protocol.md`, `runtime-instructions/work.problem-party-protocol.md`, the TaskFlow runtime-family implementation source tree, and the current TaskFlow v1 runtime modernization decisions
7. [docflow-v1-runtime-modernization-plan.md](docflow-v1-runtime-modernization-plan.md)
   Sources: `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`, `docs/product/spec/canonical-inventory-law.md`, `docs/product/spec/canonical-relation-law.md`, `docs/product/spec/canonical-runtime-readiness-law.md`, `docs/product/spec/root-map-and-runtime-surface-model.md`, `system-maps/runtime-family.docflow-map.md`, `runtime-instructions/runtime.direct-runtime-consumption-protocol.md`, `docs/process/documentation-tooling-map.md`, `vida docflow`, `vida/config/docflow/docsys_policy.yaml`, and the current DocFlow runtime modernization decisions
8. [compiled-autonomous-delivery-runtime-architecture.md](compiled-autonomous-delivery-runtime-architecture.md)
   Sources: `docs/product/spec/canonical-runtime-layer-matrix.md`, `docs/product/spec/root-map-and-runtime-surface-model.md`, `docs/product/spec/agent-role-skill-profile-flow-model.md`, `docs/product/spec/agent-lane-selection-and-conversation-mode-model.md`, `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`, `docs/product/spec/docflow-v1-runtime-modernization-plan.md`, `runtime-instructions/work.project-agent-extension-protocol.md`, `runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`, `runtime-instructions/runtime.direct-runtime-consumption-protocol.md`, and current OpenAI/Anthropic/Microsoft official orchestration baselines
9. [emerging-architectural-patterns-model.md](emerging-architectural-patterns-model.md)
   Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/canonical-runtime-layer-matrix.md`, `docs/product/spec/team-coordination-model.md`, `docs/product/spec/status-families-and-query-surface-model.md`, OpenAI Prompt Caching docs, Anthropic Prompt Caching docs, LiteLLM docs, Portkey AI Gateway docs, Helicone AI Gateway docs, `vllm-project/semantic-router`, and the current production-pattern synthesis around runtime-owned tool execution, graph-based multi-agent orchestration, reliability/evaluation/governance, caching, and gateway-layer improvements
10. [compiled-runtime-bundle-contract.md](compiled-runtime-bundle-contract.md)
    Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/agent-role-skill-profile-flow-model.md`, `docs/product/spec/agent-lane-selection-and-conversation-mode-model.md`, `runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`, `runtime-instructions/work.project-agent-extension-protocol.md`, and current OpenAI/Anthropic/Microsoft official orchestration baselines
11. [project-activation-and-configurator-model.md](project-activation-and-configurator-model.md)
    Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/agent-role-skill-profile-flow-model.md`, `docs/product/spec/runtime-paths-and-derived-cache-model.md`, `runtime-instructions/work.project-agent-extension-protocol.md`, transitional root `vida.config.yaml`, and the current DB-first activation decisions for Release 1
12. [team-coordination-model.md](team-coordination-model.md)
    Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/agent-role-skill-profile-flow-model.md`, `docs/product/spec/agent-lane-selection-and-conversation-mode-model.md`, and current OpenAI/Anthropic/Microsoft official multi-agent coordination baselines
13. [status-families-and-query-surface-model.md](status-families-and-query-surface-model.md)
    Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`, `docs/product/spec/docflow-v1-runtime-modernization-plan.md`, and current `vida` boot/status/doctor/memory shell surfaces
14. [project-protocol-promotion-law.md](project-protocol-promotion-law.md)
    Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/compiled-runtime-bundle-contract.md`, `runtime-instructions/work.project-agent-extension-protocol.md`, and the current project activation versus executable bundle admission decisions
15. [taskflow-protocol-runtime-binding-model.md](taskflow-protocol-runtime-binding-model.md)
    Sources: `docs/product/spec/canonical-runtime-layer-matrix.md`, `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`, `docs/product/spec/compiled-runtime-bundle-contract.md`, `runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`, `runtime-instructions/runtime.direct-runtime-consumption-protocol.md`, `instruction-contracts/bridge.instruction-activation-protocol.md`, and `diagnostic-instructions/analysis.protocol-consistency-audit-protocol.md`
16. [user-facing-runtime-flow-and-operating-loop-model.md](user-facing-runtime-flow-and-operating-loop-model.md)
    Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/release-1-plan.md`, `docs/product/spec/embedded-runtime-and-editable-projection-model.md`, `docs/product/spec/project-activation-and-configurator-model.md`, `docs/product/spec/project-protocol-promotion-law.md`, `docs/product/spec/agent-lane-selection-and-conversation-mode-model.md`, `docs/product/spec/checkpoint-commit-and-replay-model.md`, `docs/product/spec/gateway-resume-handle-and-trigger-index.md`, `docs/product/spec/status-families-and-query-surface-model.md`, `docs/product/spec/runtime-paths-and-derived-cache-model.md`, `docs/product/research/execution-approval-and-interrupt-resume-survey.md`, and the current operator-journey concretization decisions for install, init, bootstrap, project activation, intake/planning, execution, approval, interrupt/resume, readiness gating, and bounded remediation
17. [bootstrap-carriers-and-project-activator-model.md](bootstrap-carriers-and-project-activator-model.md)
    Sources: `AGENTS.md`, `AGENTS.sidecar.md`, `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`, `docs/product/spec/root-map-and-runtime-surface-model.md`, `docs/product/spec/framework-project-documentation-layer-model.md`, `docs/product/spec/project-activation-and-configurator-model.md`, and the current onboarding/init split decisions for orchestrator-init, agent-init, project-activator, and sidecar enrichment
18. [execution-preparation-and-developer-handoff-model.md](execution-preparation-and-developer-handoff-model.md)
    Sources: `docs/product/spec/agent-role-skill-profile-flow-model.md`, `docs/product/spec/team-coordination-model.md`, `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`, `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/release-1-plan.md`, and the current v1 decision to insert an explicit preparation stage between planning and code-shaped implementation
19. [operational-state-and-synchronization-model.md](operational-state-and-synchronization-model.md)
    Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/embedded-runtime-and-editable-projection-model.md`, `docs/product/spec/runtime-paths-and-derived-cache-model.md`, and the current DB-first synchronization boundary decisions
20. [host-agent-layer-status-matrix.md](host-agent-layer-status-matrix.md)
    Sources: `vida.config.yaml`, `docs/process/codex-agent-configuration-guide.md`, `docs/product/spec/agent-role-skill-profile-flow-model.md`, `docs/product/spec/bootstrap-carriers-and-project-activator-model.md`, `work.host-cli-agent-setup-protocol.md`, and current Rust launcher/runtime status surfaces

### Project And Packaging

1. [repository-two-project-surface-model.md](repository-two-project-surface-model.md)
   Sources: `AGENTS.sidecar.md`, `system-maps/framework.map.md`, root `vida.config.yaml`, and the current repository separation decisions for `vida-stack` and extracted `vida-mobile`
2. [github-public-repository-law.md](github-public-repository-law.md)
   Sources: `docs/product/spec/project-documentation-law.md`, `docs/product/spec/project-document-naming-law.md`, root repository narrative/governance files, and official GitHub public-repository/community-health documentation
3. [release-build-packaging-law.md](release-build-packaging-law.md)
   Sources: `docs/product/spec/github-public-repository-law.md`, `docs/process/release-formatting-protocol.md`, `install/install.sh`, `scripts/build-release.sh`, and the current 0.2.x release-archive minimization decisions
4. [embedded-runtime-and-editable-projection-model.md](embedded-runtime-and-editable-projection-model.md)
   Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/compiled-runtime-bundle-contract.md`, `docs/product/spec/project-activation-and-configurator-model.md`, `docs/product/spec/release-build-packaging-law.md`, `docs/product/spec/taskflow-protocol-runtime-binding-model.md`, `runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`, and the current installed/runtime bootstrap direction
5. [runtime-paths-and-derived-cache-model.md](runtime-paths-and-derived-cache-model.md)
   Sources: `docs/product/spec/embedded-runtime-and-editable-projection-model.md`, `docs/product/spec/project-activation-and-configurator-model.md`, `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/taskflow-protocol-runtime-binding-model.md`, TaskFlow runtime-family implementation surfaces, `crates/vida/**`, and the current `.vida/` path/cache unification decisions
6. [extensibility-and-output-template-model.md](extensibility-and-output-template-model.md)
   Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/project-activation-and-configurator-model.md`, `docs/product/spec/project-protocol-promotion-law.md`, and `docs/product/spec/agent-role-skill-profile-flow-model.md`
7. [external-architecture-baseline.md](external-architecture-baseline.md)
   Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, official OpenAI/Anthropic/Microsoft architecture references, and explicit vendor-alignment preservation

### Release 1

1. [release-1-plan.md](release-1-plan.md)
   Sources: top-level runtime architecture, canonical runtime layers, Release-1 capability/seam/current-state surfaces, key activation/execution/state specs, host-agent status, and the Airtable `Vida` architecture records refreshed on `2026-03-16`
2. [release-1-capability-matrix.md](release-1-capability-matrix.md)
   Sources: `docs/product/spec/release-1-plan.md`, runtime-family modernization plans, canonical runtime/documentation layer matrices, and `docs/process/vida1-development-conditions.md`
3. [release-1-seam-map.md](release-1-seam-map.md)
   Sources: `docs/product/spec/release-1-plan.md`, `docs/product/spec/release-1-capability-matrix.md`, runtime-family modernization plans, canonical runtime/documentation layer matrices, and `docs/process/vida1-development-conditions.md`
4. [release-1-current-state.md](release-1-current-state.md)
   Sources: `docs/product/spec/release-1-capability-matrix.md`, `docs/product/spec/release-1-seam-map.md`, runtime-family modernization plans, `docs/process/vida1-development-conditions.md`, and current workspace topology
5. [release-1-closure-contract.md](release-1-closure-contract.md)
   Sources: `docs/product/spec/release-1-plan.md`, `docs/product/spec/release-1-capability-matrix.md`, `docs/product/spec/release-1-seam-map.md`, and the Airtable `Vida` `Spec` records refreshed on `2026-03-16` that made closure, risk acceptance, workflow support, and production-baseline gates explicit
6. [release-1-workflow-classification-and-risk-matrix.md](release-1-workflow-classification-and-risk-matrix.md)
   Sources: Airtable `Vida` `Spec` workflow and roadmap records `900-909` and `1000-1009`, `docs/product/spec/release-1-plan.md`, `docs/product/spec/release-1-seam-map.md`, and the broader current runtime lifecycle and delegation law
7. [release-1-control-metrics-and-gates.md](release-1-control-metrics-and-gates.md)
   Sources: Airtable `Vida` `Spec` benchmark/control records `810-814` and roadmap records `1001-1009`, `docs/product/spec/release-1-plan.md`, `docs/product/spec/release-1-capability-matrix.md`, and the current need to turn production-baseline tracks into explicit release gates
8. [release-1-canonical-artifact-schemas.md](release-1-canonical-artifact-schemas.md)
   Sources: `docs/product/spec/release-1-plan.md`, `docs/product/spec/receipt-and-proof-law.md`, `docs/product/spec/compiled-runtime-bundle-contract.md`, `docs/product/spec/operational-state-and-synchronization-model.md`, and the Airtable `Vida` `Spec` records refreshed on `2026-03-16` that require stable contracts for trace, approval, evaluation, incident, and memory artifacts
9. [release-1-decision-tables.md](release-1-decision-tables.md)
   Sources: `docs/product/spec/release-1-workflow-classification-and-risk-matrix.md`, `docs/product/spec/release-1-closure-contract.md`, Airtable `Vida` `Spec` workflow/control records, and the current need to turn workflow law into explicit if-then gates
10. [release-1-state-machine-specs.md](release-1-state-machine-specs.md)
   Sources: `docs/product/spec/release-1-workflow-classification-and-risk-matrix.md`, `docs/product/spec/release-1-decision-tables.md`, `docs/product/spec/canonical-machine-map.md`, and the current need for explicit transition law across lane, approval, tool, and recovery control paths
11. [release-1-error-and-exception-taxonomy.md](release-1-error-and-exception-taxonomy.md)
   Sources: `docs/product/spec/release-1-seam-map.md`, `docs/product/spec/release-1-closure-contract.md`, and the current need to stabilize blocker/failure vocabulary across runtime, proof, and operator surfaces
12. [release-1-ownership-to-code-map.md](release-1-ownership-to-code-map.md)
   Sources: `docs/product/spec/release-1-plan.md`, `docs/product/spec/release-1-current-state.md`, current Rust workspace topology, and the current anti-drift need to bind doc ownership to crate/module boundaries
13. [release-1-proof-scenario-catalog.md](release-1-proof-scenario-catalog.md)
   Sources: `docs/product/spec/release-1-closure-contract.md`, `docs/product/spec/release-1-control-metrics-and-gates.md`, `docs/product/spec/release-1-state-machine-specs.md`, and the current requirement for explicit positive and negative closure scenarios
14. [release-1-schema-versioning-and-compatibility-law.md](release-1-schema-versioning-and-compatibility-law.md)
   Sources: `docs/product/spec/release-1-canonical-artifact-schemas.md`, `docs/product/spec/receipt-and-proof-law.md`, `docs/product/spec/compiled-runtime-bundle-contract.md`, and the current need to prevent contract drift during Release-1 refactors
15. [release-1-runtime-enum-and-code-contracts.md](release-1-runtime-enum-and-code-contracts.md)
   Sources: `docs/product/spec/release-1-workflow-classification-and-risk-matrix.md`, `docs/product/spec/release-1-state-machine-specs.md`, `docs/product/spec/release-1-error-and-exception-taxonomy.md`, and the current need to freeze Rust/runtime enum vocabulary before implementation spreads
16. [release-1-conformance-matrix.md](release-1-conformance-matrix.md)
   Sources: `docs/product/spec/release-1-ownership-to-code-map.md`, `docs/product/spec/release-1-proof-scenario-catalog.md`, `docs/product/spec/release-1-plan.md`, and the current need for one bounded doc-to-code-to-proof scoreboard
17. [release-1-operator-surface-contract.md](release-1-operator-surface-contract.md)
   Sources: `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`, `docs/product/spec/release-1-runtime-enum-and-code-contracts.md`, and the current need to stabilize operator-visible output contracts during shell refactors
18. [release-1-unsupported-surface-contract.md](release-1-unsupported-surface-contract.md)
   Sources: `docs/product/spec/release-1-workflow-classification-and-risk-matrix.md`, `docs/product/spec/release-1-closure-contract.md`, and the current need to prevent implicit support widening
19. [release-1-fixture-and-golden-data-contract.md](release-1-fixture-and-golden-data-contract.md)
   Sources: `docs/product/spec/release-1-proof-scenario-catalog.md`, `docs/product/spec/release-1-schema-versioning-and-compatibility-law.md`, and the current requirement for canonical examples and regression fixtures
20. [release-1-risk-acceptance-register.md](release-1-risk-acceptance-register.md)
   Sources: `docs/product/spec/release-1-closure-contract.md`, `docs/product/spec/release-1-control-metrics-and-gates.md`, and the current governance need to keep all open acceptances explicit rather than implicit
21. [taskflow-task-command-parity-and-proxy-alignment-design.md](taskflow-task-command-parity-and-proxy-alignment-design.md)
   Sources: `docs/product/spec/release-1-plan.md`, `docs/product/spec/status-families-and-query-surface-model.md`, `docs/product/spec/release-1-operator-surface-contract.md`, current launcher task surface code in `crates/vida/src/{cli,task_surface,taskflow_task_bridge,taskflow_layer4,taskflow_proxy}.rs`, and the active DB-backed Release-1 TaskFlow program tasks `r1-01-a` and `r1-01-d`
22. [release-1-carrier-neutral-runtime-and-host-materialization-design.md](release-1-carrier-neutral-runtime-and-host-materialization-design.md)
   Sources: `docs/product/spec/release-1-plan.md`, `docs/product/spec/release-1-current-state.md`, `docs/product/spec/release-1-conformance-matrix.md`, `docs/product/spec/compiled-runtime-bundle-contract.md`, `docs/product/spec/bootstrap-carriers-and-project-activator-model.md`, `docs/product/spec/host-agent-layer-status-matrix.md`, current launcher/runtime code in `crates/vida/src/{main,project_activator_surface,status_surface,taskflow_consume_bundle,taskflow_routing}.rs`, and the active Release-1 TaskFlow program tasks `r1-05-a`, `r1-05-b`, and `r1-08-a`
23. [launcher-decomposition-and-code-hygiene-design.md](launcher-decomposition-and-code-hygiene-design.md)
   Sources: `docs/product/spec/release-1-current-state.md`, `docs/product/spec/release-1-conformance-matrix.md`, current launcher code in `crates/vida/src/{main,state_store,project_activator_surface,runtime_dispatch_state,taskflow_routing}.rs`, extracted helper modules in `crates/vida/src/{launcher_task_commands,project_root_paths,state_store_patching,state_store_taskflow_snapshot_codec,state_store_source_scan,state_store_run_graph_summary}.rs`, format helper crates under `crates/{taskflow,docflow}-format-*`, and the current need to reduce launcher concentration without widening runtime behavior
24. [continuation-binding-fail-closed-hardening-design.md](continuation-binding-fail-closed-hardening-design.md)
   Sources: `AGENTS.md`, `docs/process/project-orchestrator-operating-protocol.md`, `docs/process/project-orchestrator-session-start-protocol.md`, `docs/process/project-packet-and-lane-runtime-capsule.md`, `docs/process/project-start-readiness-runtime-capsule.md`, `docs/product/spec/autonomous-report-continuation-law.md`, `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`, current launcher/status code in `crates/vida/src/{taskflow_runtime_bundle,status_surface,status_surface_operator_contracts,status_surface_signals,runtime_dispatch_packet_text,init_surfaces,release1_contracts}.rs`, and the current need to fail closed when continued-development intent lacks an explicitly bound bounded unit
25. [continuation-and-seeded-dispatch-bridge-design.md](continuation-and-seeded-dispatch-bridge-design.md)
   Sources: `docs/product/spec/continuation-binding-fail-closed-hardening-design.md`, `docs/product/spec/release-1-operator-surface-contract.md`, current launcher/runtime code in `crates/vida/src/{state_store,taskflow_run_graph,taskflow_consume,runtime_dispatch_state,taskflow_runtime_bundle,status_surface}.rs`, and the current need to bridge seeded run-graph state to the first persisted dispatch receipt and packet evidence without heuristic fallback
26. [authoritative-state-lock-recovery-design.md](authoritative-state-lock-recovery-design.md)
   Sources: `docs/product/spec/ops-state-and-runtime-evidence-hygiene-design.md`, `docs/process/project-operations.md`, `docs/process/environments.md`, `docs/product/spec/internal-codex-agent-execution-fail-closed-design.md`, current state-store/runtime code in `crates/vida/src/{state_store,state_store_open,taskflow_consume,taskflow_consume_resume,runtime_dispatch_state,runtime_dispatch_execution,status_surface,doctor_surface}.rs`, and the current need to shorten authoritative lock lifetime during agent-lane dispatch without introducing silent long-lived-state cleanup
27. [lawful-closure-continuation-rebinding-design.md](lawful-closure-continuation-rebinding-design.md)
   Sources: `docs/product/spec/autonomous-report-continuation-law.md`, `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`, `docs/product/spec/continuation-binding-fail-closed-hardening-design.md`, `docs/product/spec/continuation-and-seeded-dispatch-bridge-design.md`, current continuation/runtime code in `crates/vida/src/{taskflow_continuation,continuation_binding_summary,taskflow_layer4,status_surface,taskflow_runtime_bundle}.rs`, and the current need to support explicit post-closure rebinding without heuristic task picking

## Routing Pointers

Use this map only when detailed source-lineage or absorbed-history context is needed.

1. Short active registry:
   - `current-spec-map.md`
2. Active project-doc bootstrap:
   - `AGENTS.sidecar.md`
3. Current project root map:
   - `../../project-root-map.md`
4. Documentation/system/tooling follow-up:
   - `../../process/documentation-tooling-map.md`

Activation rule:

1. read this map when source-lineage, absorbed-history, or provenance questions are active,
2. prefer `current-spec-map.md` for ordinary active-canon routing,
3. prefer `../../project-root-map.md` first when the task is still choosing between product/process/project-memory lanes,
4. do not use this file as a replacement for framework root-map routing.

## Current Rule

1. `docs/product/spec/**` is the current prose canon.
2. `vida/config/**` is the executable law home.
3. this provenance map is supporting lineage context for the active canon, not a duplicate owner layer.
4. deleted framework-formation plans/research survive only as provenance in `docs/process/framework-source-lineage-index.md`, not as active product canon.

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
artifact_path: product/spec/current-spec-provenance-map
artifact_type: product_spec
artifact_version: '1'
artifact_revision: 2026-04-13
schema_version: '1'
status: canonical
source_path: docs/product/spec/current-spec-provenance-map.md
created_at: '2026-03-16T09:05:00+02:00'
updated_at: 2026-04-13T16:12:52.796643184Z
changelog_ref: current-spec-provenance-map.changelog.jsonl
