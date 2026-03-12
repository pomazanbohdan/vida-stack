# VIDA Current Spec Map

Status: active canonical map

Revision: `2026-03-12`

Purpose: define the current product-spec home, show absorbed historical sources, and anchor each current artifact to executable product law.

## Current Canon

1. [partial-development-kernel-model.md](/home/unnamed/project/vida-stack/docs/product/spec/partial-development-kernel-model.md)
   Sources: promoted state/route kernel lineage preserved in `docs/process/framework-source-lineage-index.md`
   Config families: `vida/config/machines/**`, `vida/config/routes/**`, `vida/config/policies/**`
2. [canonical-machine-map.md](/home/unnamed/project/vida-stack/docs/product/spec/canonical-machine-map.md)
   Sources: promoted machine/state lineage preserved in `docs/process/framework-source-lineage-index.md`
   Config families: `vida/config/machines/**`
3. [receipt-and-proof-law.md](/home/unnamed/project/vida-stack/docs/product/spec/receipt-and-proof-law.md)
   Sources: `docs/process/framework-source-lineage-index.md`
   Config families: `vida/config/receipts/**`, `vida/config/policies/**`
4. [external-pattern-borrow-map.md](/home/unnamed/project/vida-stack/docs/product/spec/external-pattern-borrow-map.md)
   Sources: `docs/process/framework-source-lineage-index.md`, external-source synthesis
   Config families: cross-cutting product law only
5. [projection-listener-checkpoint-model.md](/home/unnamed/project/vida-stack/docs/product/spec/projection-listener-checkpoint-model.md)
   Sources: Eventuous, Elsa, LangGraph research promoted through the borrow map
   Config families: `vida/config/machines/**`, runtime consumption by `taskflow-v0`
6. [gateway-resume-handle-and-trigger-index.md](/home/unnamed/project/vida-stack/docs/product/spec/gateway-resume-handle-and-trigger-index.md)
   Sources: Elsa trigger/bookmark semantics
   Config families: future route/gateway law
7. [machine-definition-lint-law.md](/home/unnamed/project/vida-stack/docs/product/spec/machine-definition-lint-law.md)
   Sources: `python-statemachine` strict validation semantics
   Config families: future machine lint
8. [checkpoint-commit-and-replay-model.md](/home/unnamed/project/vida-stack/docs/product/spec/checkpoint-commit-and-replay-model.md)
   Sources: Eventuous and LangGraph checkpoint/replay semantics
   Config families: runtime-derived checkpoint law
9. [verification-merge-law.md](/home/unnamed/project/vida-stack/docs/product/spec/verification-merge-law.md)
   Sources: Elsa merge regressions, verification parallelism research
   Config families: future verification routing law
10. [instruction-artifact-model.md](/home/unnamed/project/vida-stack/docs/product/spec/instruction-artifact-model.md)
   Sources: promoted instruction-kernel lineage in `docs/process/framework-source-lineage-index.md`, `vida/config/instructions/**`
   Config families: `vida/config/instructions/**`
11. [skill-management-and-activation-law.md](/home/unnamed/project/vida-stack/docs/product/spec/skill-management-and-activation-law.md)
   Sources: product migration decisions in this cutover
   Config families: `vida/config/instructions/skills/**`, `vida/config/instructions/activation/**`
12. [instruction-migration-map.md](/home/unnamed/project/vida-stack/docs/product/spec/instruction-migration-map.md)
   Sources: `vida/config/instructions/agent-definitions/model.agent-definitions-contract.md`, `vida/config/instructions/**`
   Config families: `vida/config/instructions/**`
13. [project-documentation-law.md](/home/unnamed/project/vida-stack/docs/product/spec/project-documentation-law.md)
   Sources: current markdown-first operating model and document-sidecar migration decisions
   Config families: project documentation governance only
14. [canonical-documentation-and-inventory-layer-matrix.md](/home/unnamed/project/vida-stack/docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md)
   Sources: `docs/product/spec/project-documentation-law.md`, `docs/product/spec/instruction-artifact-model.md`, promoted instruction/migration lineage in `docs/process/framework-source-lineage-index.md`, `vida/config/**`
   Config families: canonical inventory, validation, mutation, relation, readiness, and runtime-consumption architecture across `vida/config/**`
15. [canonical-inventory-law.md](/home/unnamed/project/vida-stack/docs/product/spec/canonical-inventory-law.md)
   Sources: `docs/product/spec/project-documentation-law.md`, `docs/product/spec/current-spec-map.md`, `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`, `vida/config/instructions/instruction_catalog.yaml`, `vida/config/instructions/projection_manifest.yaml`
   Config families: canonical inventory, registry structure, coverage, source/projection linkage, and version-tuple visibility across active canon
16. [canonical-relation-law.md](/home/unnamed/project/vida-stack/docs/product/spec/canonical-relation-law.md)
   Sources: `docs/product/spec/project-documentation-law.md`, `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`, `docs/process/documentation-tooling-map.md`, `codex-v0/codex.py`
   Config families: canonical dependencies, direct/reverse references, artifact impact, task impact, and relation validation across active canon
17. [canonical-runtime-readiness-law.md](/home/unnamed/project/vida-stack/docs/product/spec/canonical-runtime-readiness-law.md)
   Sources: promoted instruction/migration/readiness lineage in `docs/process/framework-source-lineage-index.md`, `vida/config/migration/**`, `vida/config/instructions/**`, `codex-v0/codex.py`
   Config families: source-version tuples, compatibility classes, projection parity, canonical bundles, boot-gate artifacts, and fail-closed readiness verdicts across active canon
18. [canonical-layer-documentation-template.md](/home/unnamed/project/vida-stack/docs/product/spec/canonical-layer-documentation-template.md)
   Sources: `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`, promoted Layer 1 through Layer 7 specs, and current documentation operation law
   Config families: canonical layer-law authoring shape for Layers 1 through 7
19. [framework-project-documentation-layer-model.md](/home/unnamed/project/vida-stack/docs/product/spec/framework-project-documentation-layer-model.md)
   Sources: `AGENTS.md`, `AGENTS.sidecar.md`, `vida/config/instructions/system-maps/framework.map.md`, `vida/config/instructions/system-maps/framework.index.md`, and the current framework/project documentation restructuring decisions
   Config families: framework canon vs role/bootstrap/governance/project documentation layering, derivation boundaries, two-map bootstrap, and root-map requirements
20. [root-map-and-runtime-surface-model.md](/home/unnamed/project/vida-stack/docs/product/spec/root-map-and-runtime-surface-model.md)
   Sources: `AGENTS.md`, `AGENTS.sidecar.md`, `vida/config/instructions/system-maps/framework.map.md`, `vida/config/instructions/system-maps/framework.index.md`, and the current runtime-surface optimization decisions for `DocFlow`, `taskflow`, and future runtime families
   Config families: framework root map, project root map, runtime-family submaps, template maps, and activation-trigger discoverability across active canon
21. [canonical-runtime-layer-matrix.md](/home/unnamed/project/vida-stack/docs/product/spec/canonical-runtime-layer-matrix.md)
   Sources: promoted route/runtime/readiness lineage in `docs/process/framework-source-lineage-index.md`, `docs/product/spec/partial-development-kernel-model.md`, `docs/product/spec/canonical-machine-map.md`, `docs/product/spec/projection-listener-checkpoint-model.md`, `docs/product/spec/gateway-resume-handle-and-trigger-index.md`, `docs/product/spec/verification-merge-law.md`, `docs/product/spec/canonical-runtime-readiness-law.md`, `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`, `vida/config/instructions/runtime-instructions/lane.agent-handoff-context-protocol.md`, and `vida/config/instructions/runtime-instructions/recovery.checkpoint-replay-recovery-protocol.md`
   Config families: layered runtime capability progression across `vida/config/**`, `taskflow-v0/**`, runtime ledgers, readiness gates, and future direct runtime consumption
22. [agent-role-skill-profile-flow-model.md](/home/unnamed/project/vida-stack/docs/product/spec/agent-role-skill-profile-flow-model.md)
   Sources: `vida/config/instructions/agent-definitions/model.agent-definitions-contract.md`, `vida/config/instructions/agent-definitions/role.role-profile-contract.md`, `vida/config/instructions/instruction-contracts/core.agent-system-protocol.md`, `vida/config/instructions/runtime-instructions/work.project-agent-extension-protocol.md`, `docs/product/spec/instruction-artifact-model.md`, and the current runtime/project-extension design decisions
   Config families: framework role law, project role/skill/profile/flow activation through `vida.config.yaml`, project-owned agent-extension registries, and runtime validation for `taskflow-v0`
23. [agent-lane-selection-and-conversation-mode-model.md](/home/unnamed/project/vida-stack/docs/product/spec/agent-lane-selection-and-conversation-mode-model.md)
   Sources: `vida/config/instructions/agent-definitions/entry.orchestrator-entry.md`, `vida/config/instructions/command-instructions/routing.use-case-packs-protocol.md`, `vida/config/instructions/command-instructions/planning.form-task-protocol.md`, `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`, `vida/config/instructions/runtime-instructions/work.agent-lane-selection-protocol.md`, and the current lane-selection/runtime-conversation design decisions
   Config families: overlay-driven auto-lane selection, bounded conversational modes, one-task scope/PBI discussion, and lawful handoff into pack/taskflow routing
24. [repository-two-project-surface-model.md](/home/unnamed/project/vida-stack/docs/product/spec/repository-two-project-surface-model.md)
   Sources: `AGENTS.sidecar.md`, `vida/config/instructions/system-maps/framework.map.md`, root `vida.config.yaml`, and the current repository separation decisions for `vida-stack` and extracted `vida-mobile`
   Config families: active current-project routing, extracted second-project bundle boundaries, root config continuity, and two-project repository map discipline
25. [party-chat-v2-problem-party-model.md](/home/unnamed/project/vida-stack/docs/product/spec/party-chat-v2-problem-party-model.md)
   Sources: Airtable `Vida` base `Table 1` records `Party Chat v2 Spec — Part 1/4` through `Part 4/4`, `vida/config/instructions/runtime-instructions/work.problem-party-protocol.md`, `docs/product/spec/agent-role-skill-profile-flow-model.md`, and the current project extension/runtime integration decisions
   Config families: `docs/process/agent-extensions/**`, `vida.config.yaml`, `.vida/logs/problem-party/**`, single-agent or multi-agent Party Chat execution plans, and runtime consumption by `taskflow-v0`
26. [autonomous-report-continuation-law.md](/home/unnamed/project/vida-stack/docs/product/spec/autonomous-report-continuation-law.md)
   Sources: `vida/config/instructions/agent-definitions/entry.orchestrator-entry.md`, `vida/config/instructions/command-instructions/routing.command-layer-protocol.md`, `vida/config/instructions/command-instructions/operator.vida-research-guide.md`, `vida/config/instructions/runtime-instructions/work.spec-intake-protocol.md`, `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`, `vida/config/instructions/runtime-instructions/bridge.project-overlay-protocol.md`, and the current overlay/routing continuation decisions
   Config families: `vida.config.yaml`, `vida/config/instructions/**`, TaskFlow routing and autonomous execution behavior
27. [taskflow-v1-runtime-modernization-plan.md](/home/unnamed/project/vida-stack/docs/product/spec/taskflow-v1-runtime-modernization-plan.md)
   Sources: `docs/product/spec/canonical-runtime-layer-matrix.md`, `docs/product/spec/party-chat-v2-problem-party-model.md`, `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`, `vida/config/instructions/runtime-instructions/work.problem-party-protocol.md`, `taskflow-v0/src/vida.nim`, `taskflow-v0/src/core/config.nim`, `taskflow-v0/src/state/problem_party.nim`, and the current TaskFlow v1 runtime modernization decisions
   Config families: `taskflow-v0/**`, `vida/config/instructions/**`, runtime feature registration, shared runtime kernel, provider registry, modular config validation, and the active `taskflow-rs` modernization backlog
28. [docflow-v1-runtime-modernization-plan.md](/home/unnamed/project/vida-stack/docs/product/spec/docflow-v1-runtime-modernization-plan.md)
   Sources: `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`, `docs/product/spec/canonical-inventory-law.md`, `docs/product/spec/canonical-relation-law.md`, `docs/product/spec/canonical-runtime-readiness-law.md`, `docs/product/spec/root-map-and-runtime-surface-model.md`, `vida/config/instructions/system-maps/runtime-family.docflow-map.md`, `vida/config/instructions/runtime-instructions/runtime.direct-runtime-consumption-protocol.md`, `docs/process/documentation-tooling-map.md`, `codex-v0/codex.py`, `codex-v0/docsys_policy.yaml`, `codex-v0/docsys_schema.yaml`, and the current DocFlow runtime modernization decisions
   Config families: `codex-v0/**`, transitional `vida/config/codex-*.jsonl` artifacts, `vida/config/instructions/**`, documentation tooling operator surfaces, runtime-family migration, and explicit final `taskflow -> docflow` consumption seams
29. [compiled-autonomous-delivery-runtime-architecture.md](/home/unnamed/project/vida-stack/docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md)
   Sources: `docs/product/spec/canonical-runtime-layer-matrix.md`, `docs/product/spec/root-map-and-runtime-surface-model.md`, `docs/product/spec/agent-role-skill-profile-flow-model.md`, `docs/product/spec/agent-lane-selection-and-conversation-mode-model.md`, `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`, `docs/product/spec/docflow-v1-runtime-modernization-plan.md`, `vida/config/instructions/runtime-instructions/work.project-agent-extension-protocol.md`, `vida/config/instructions/runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`, `vida/config/instructions/runtime-instructions/runtime.direct-runtime-consumption-protocol.md`, and current OpenAI/Anthropic/Microsoft official orchestration baselines
   Config families: `vida/config/instructions/**`, `.vida/config/**`, `.vida/project/**`, `.vida/cache/**`, transitional source-mode bridge surfaces such as root `vida.config.yaml` and `docs/process/agent-extensions/**`, `taskflow-v0/**`, `codex-v0/**`, and future compiled orchestration bundle surfaces
30. [emerging-architectural-patterns-model.md](/home/unnamed/project/vida-stack/docs/product/spec/emerging-architectural-patterns-model.md)
   Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/canonical-runtime-layer-matrix.md`, `docs/product/spec/team-coordination-model.md`, `docs/product/spec/status-families-and-query-surface-model.md`, OpenAI Prompt Caching docs, Anthropic Prompt Caching docs, LiteLLM docs, Portkey AI Gateway docs, Helicone AI Gateway docs, `vllm-project/semantic-router`, and the current production-pattern synthesis around runtime-owned tool execution, graph-based multi-agent orchestration, reliability/evaluation/governance, caching, and gateway-layer improvements
   Config families: runtime loop ownership, specialist-agent topology, routing, verifier aggregation, persistent workflow state, production observability, evaluation posture, governance/security expectations, caching strategy, and gateway/proxy control surfaces across `vida/config/instructions/**`, `taskflow-v0/**`, and future compiled runtime surfaces
31. [release-1-wave-plan.md](/home/unnamed/project/vida-stack/docs/product/spec/release-1-wave-plan.md)
   Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`, `docs/product/spec/docflow-v1-runtime-modernization-plan.md`, `vida/config/instructions/runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`, `vida/config/instructions/runtime-instructions/work.project-agent-extension-protocol.md`, and current Rust/TaskFlow operator-shell donor proofs
   Config families: Release-1 execution sequencing across `vida/config/**`, `taskflow-v0/**`, `codex-v0/**`, and current Rust `vida` operator surfaces
32. [compiled-runtime-bundle-contract.md](/home/unnamed/project/vida-stack/docs/product/spec/compiled-runtime-bundle-contract.md)
   Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/agent-role-skill-profile-flow-model.md`, `docs/product/spec/agent-lane-selection-and-conversation-mode-model.md`, `vida/config/instructions/runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`, `vida/config/instructions/runtime-instructions/work.project-agent-extension-protocol.md`, and current OpenAI/Anthropic/Microsoft official orchestration baselines
   Config families: compiled control bundles with `control_core`, `activation_bundle`, `protocol_binding_registry`, and `cache_delivery_contract`, `.vida/config/**`, `.vida/project/**`, `.vida/db/**`, `.vida/cache/**`, runtime init/boot activation, bundle validation, and future machine-readable orchestration bundle surfaces
33. [project-activation-and-configurator-model.md](/home/unnamed/project/vida-stack/docs/product/spec/project-activation-and-configurator-model.md)
   Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/agent-role-skill-profile-flow-model.md`, `docs/product/spec/runtime-paths-and-derived-cache-model.md`, `vida/config/instructions/runtime-instructions/work.project-agent-extension-protocol.md`, transitional root `vida.config.yaml`, and the current DB-first activation decisions for Release 1
   Config families: DB-first project activation, `.vida/config/**`, `.vida/project/**`, roles/skills/profiles/flows/agents/teams/model/backend policy, sync/reconcile surfaces, and project lifecycle control
34. [team-coordination-model.md](/home/unnamed/project/vida-stack/docs/product/spec/team-coordination-model.md)
   Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/agent-role-skill-profile-flow-model.md`, `docs/product/spec/agent-lane-selection-and-conversation-mode-model.md`, and current OpenAI/Anthropic/Microsoft official multi-agent coordination baselines
   Config families: team composition, coordination pattern, activation, shared policy, handoff/context posture, and closure semantics
35. [status-families-and-query-surface-model.md](/home/unnamed/project/vida-stack/docs/product/spec/status-families-and-query-surface-model.md)
   Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`, `docs/product/spec/docflow-v1-runtime-modernization-plan.md`, and current `vida` boot/status/doctor/memory shell surfaces
   Config families: CLI query/status families, operator-facing render surfaces, bounded runtime snapshots, and status-family routing
36. [project-protocol-promotion-law.md](/home/unnamed/project/vida-stack/docs/product/spec/project-protocol-promotion-law.md)
   Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/compiled-runtime-bundle-contract.md`, `vida/config/instructions/runtime-instructions/work.project-agent-extension-protocol.md`, and the current project activation versus executable bundle admission decisions
   Config families: known versus compiled project protocol admission, project discovery/mapping, executable bundle promotion, and fail-closed protocol binding
37. [project-document-naming-law.md](/home/unnamed/project/vida-stack/docs/product/spec/project-document-naming-law.md)
   Sources: `docs/product/spec/project-documentation-law.md`, `docs/product/spec/framework-project-documentation-layer-model.md`, `docs/product/spec/instruction-artifact-model.md`, and the current project-doc naming standardization decisions for `docs/**`
   Config families: `docs/product/spec/**`, `docs/process/**`, `docs/product/research/**`, `docs/project-memory/**`, lane-root naming, reserved filename handling, and bounded rename-wave law for project-owned documentation
38. [github-public-repository-law.md](/home/unnamed/project/vida-stack/docs/product/spec/github-public-repository-law.md)
   Sources: `docs/product/spec/project-documentation-law.md`, `docs/product/spec/project-document-naming-law.md`, root repository narrative/governance files, and official GitHub public-repository/community-health documentation
   Config families: root repository entrypoints, `.github/**`, public-repository community surfaces, code ownership, issue/PR templates, security disclosure, and release/tag publication posture
39. [release-build-packaging-law.md](/home/unnamed/project/vida-stack/docs/product/spec/release-build-packaging-law.md)
   Sources: `docs/product/spec/github-public-repository-law.md`, `docs/process/release-formatting-protocol.md`, `install/install.sh`, `scripts/build-release.sh`, and the current 0.2.x release-archive minimization decisions
   Config families: public release archive composition, installer/archive boundary, runtime-only package contents, sidecar scaffold packaging, and public release-page formatting alignment
40. [taskflow-protocol-runtime-binding-model.md](/home/unnamed/project/vida-stack/docs/product/spec/taskflow-protocol-runtime-binding-model.md)
   Sources: `docs/product/spec/canonical-runtime-layer-matrix.md`, `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`, `docs/product/spec/compiled-runtime-bundle-contract.md`, `vida/config/instructions/runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`, `vida/config/instructions/runtime-instructions/runtime.direct-runtime-consumption-protocol.md`, `vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md`, and `vida/config/instructions/diagnostic-instructions/analysis.protocol-consistency-audit-protocol.md`
   Config families: script-era protocol binding bridge, Rust-native protocol runtime crate, activation resolution, gate enforcement, protocol receipts, binding matrices, and the dedicated TaskFlow protocol-binding subrelease
41. [embedded-runtime-and-editable-projection-model.md](/home/unnamed/project/vida-stack/docs/product/spec/embedded-runtime-and-editable-projection-model.md)
   Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/compiled-runtime-bundle-contract.md`, `docs/product/spec/project-activation-and-configurator-model.md`, `docs/product/spec/release-build-packaging-law.md`, `docs/product/spec/taskflow-protocol-runtime-binding-model.md`, `vida/config/instructions/runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`, and the current installed/runtime bootstrap direction
   Config families: embedded framework artifacts, binary-only runtime execution, project projection export/import loops, hidden runtime-owned config/activation surfaces under `.vida/**`, DB-first runtime truth, and release/runtime separation between sealed framework state and editable project surfaces
42. [runtime-paths-and-derived-cache-model.md](/home/unnamed/project/vida-stack/docs/product/spec/runtime-paths-and-derived-cache-model.md)
   Sources: `docs/product/spec/embedded-runtime-and-editable-projection-model.md`, `docs/product/spec/project-activation-and-configurator-model.md`, `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/taskflow-protocol-runtime-binding-model.md`, `taskflow-v0/**`, `crates/vida/**`, and the current `.vida/` path/cache unification decisions
   Config families: `.vida/config/**`, `.vida/db/**`, `.vida/cache/**`, `.vida/framework/**`, `.vida/project/**`, derived serving cache invalidation, hidden runtime-owned config and activation surfaces, and bridge migration away from root runtime files
43. [user-facing-runtime-flow-and-operating-loop-model.md](/home/unnamed/project/vida-stack/docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md)
   Sources: `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/release-1-wave-plan.md`, `docs/product/spec/embedded-runtime-and-editable-projection-model.md`, `docs/product/spec/project-activation-and-configurator-model.md`, `docs/product/spec/project-protocol-promotion-law.md`, `docs/product/spec/agent-lane-selection-and-conversation-mode-model.md`, `docs/product/spec/checkpoint-commit-and-replay-model.md`, `docs/product/spec/gateway-resume-handle-and-trigger-index.md`, `docs/product/spec/status-families-and-query-surface-model.md`, `docs/product/spec/runtime-paths-and-derived-cache-model.md`, `docs/product/research/execution-approval-and-interrupt-resume-survey.md`, and the current operator-journey concretization decisions for install, init, bootstrap, project activation, intake/planning, execution, approval, interrupt/resume, readiness gating, and bounded remediation
   Config families: operator-facing install/init/bootstrap flow, project-local runtime onboarding, project activation/config sequencing, intake/planning sequencing, execution/approval/resume sequencing, bounded pre-readiness allowlists, runtime bootstrap posture, and the staged user-facing operating loop across `.vida/**`, installed runtime assets, and DB-first readiness state
44. [bootstrap-carriers-and-project-activator-model.md](/home/unnamed/project/vida-stack/docs/product/spec/bootstrap-carriers-and-project-activator-model.md)
   Sources: `AGENTS.md`, `AGENTS.sidecar.md`, `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`, `docs/product/spec/root-map-and-runtime-surface-model.md`, `docs/product/spec/framework-project-documentation-layer-model.md`, `docs/product/spec/project-activation-and-configurator-model.md`, and the current onboarding/init split decisions for orchestrator-init, agent-init, project-activator, and sidecar enrichment
   Config families: bootstrap carriers, runtime init command split, project activator pipeline, sidecar/project-map enrichment, host-template onboarding, and bounded protocol-load separation between orchestrator and agent lanes
45. [execution-preparation-and-developer-handoff-model.md](/home/unnamed/project/vida-stack/docs/product/spec/execution-preparation-and-developer-handoff-model.md)
   Sources: `docs/product/spec/agent-role-skill-profile-flow-model.md`, `docs/product/spec/team-coordination-model.md`, `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`, `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`, `docs/product/spec/release-1-wave-plan.md`, and the current v1 decision to insert an explicit preparation stage between planning and code-shaped implementation
   Config families: `solution_architect`, execution preparation, architecture-preparation reports, developer handoff packets, change-boundary shaping, dependency-impact summaries, and fail-closed pre-execution gating for code-shaped work

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
3. deleted framework-formation plans/research survive only as provenance in `docs/process/framework-source-lineage-index.md`, not as active product canon.

## Shared Runtime Spine Rule

1. The active `TaskFlow v1` modernization program and `VIDA 1.0` share one semantic runtime-spec spine.
2. Stable product-law portions of that spine are promoted here into `docs/product/spec/**`.
3. Historical formation inputs for that spine are preserved only in `docs/process/framework-source-lineage-index.md`.
4. `taskflow-v0/**` is the current transitional implementation substrate and donor bridge for the `TaskFlow v1` line, not a separate semantic canon.

## Project Documentation Rule

1. Root repository docs, `docs/product/**`, `docs/process/**`, and `docs/project-memory/**` are part of the active project documentation surface.
2. Active canonical markdown documents in those lanes must carry machine-readable footer metadata and a sibling `*.changelog.jsonl`.
3. During the pre-runtime phase, only the latest markdown revision is kept as the active body; historical lineage stays in sidecars and git history.

-----
artifact_path: product/spec/current-spec-map
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/current-spec-map.md
created_at: '2026-03-09T20:28:59+02:00'
updated_at: '2026-03-13T00:00:00+02:00'
changelog_ref: current-spec-map.changelog.jsonl
