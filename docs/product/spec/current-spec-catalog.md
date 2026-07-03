# VIDA Current Spec Catalog

Status: active canonical companion
Revision: 2026-06-21

Purpose: carry the detailed current product/spec artifact catalog that used to live inside current-spec-map.md, so the map can stay a short routing surface while preserving the full active canon list.

Companion rule:

1. Start from [current-spec-map.md](current-spec-map.md) for routing decisions.
2. Use this catalog for detailed active artifact discovery and config-family notes.
3. This catalog does not replace the owning artifact docs; it only lists and routes them.

## Detailed Current Canon

### Core

1. [partial-development-kernel-model.md](partial-development-kernel-model.md)
   Config families: `vida/config/machines/**`, `vida/config/routes/**`, `vida/config/policies/**`
2. [canonical-machine-map.md](canonical-machine-map.md)
   Config families: `vida/config/machines/**`
3. [receipt-and-proof-law.md](receipt-and-proof-law.md)
   Config families: `vida/config/receipts/**`, `vida/config/policies/**`
4. [external-pattern-borrow-map.md](external-pattern-borrow-map.md)
   Config families: cross-cutting semantic borrow law only
5. [projection-listener-checkpoint-model.md](projection-listener-checkpoint-model.md)
   Config families: `vida/config/machines/**`, runtime consumption by the TaskFlow runtime family
6. [gateway-resume-handle-and-trigger-index.md](gateway-resume-handle-and-trigger-index.md)
   Config families: future-direction route/gateway law
7. [checkpoint-commit-and-replay-model.md](checkpoint-commit-and-replay-model.md)
   Config families: runtime-derived checkpoint law
8. [verification-merge-law.md](verification-merge-law.md)
   Config families: active verification routing law
9. [instruction-artifact-model.md](instruction-artifact-model.md)
   Config families: `vida/config/instructions/**`
10. [skill-management-and-activation-law.md](skill-management-and-activation-law.md)
   Config families: `skills/**`, `activation/**`
11. [instruction-migration-map.md](instruction-migration-map.md)
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
12. [protocol-authoring-and-token-economy-law.md](protocol-authoring-and-token-economy-law.md)
    Config families: protocol and instruction authoring, block-level compression, quality-versus-size algorithm routing, protected-atom validation, token-budget gates, and bootstrap-visible registration for `docs/product/spec/**`, `docs/process/**`, and `AGENTS.sidecar.md`
    Registered path: `docs/product/spec/protocol-authoring-and-token-economy-law.md`

### Runtime And Agent Control

1. [canonical-runtime-layer-matrix.md](canonical-runtime-layer-matrix.md)
   Config families: layered runtime capability progression across `vida/config/**`, TaskFlow runtime-family implementation surfaces, runtime ledgers, readiness gates, and future direct runtime consumption
2. [agent-role-skill-profile-flow-model.md](agent-role-skill-profile-flow-model.md)
   Config families: framework role law, project role/skill/profile/flow activation through `vida.config.yaml`, project-owned agent-extension registries, and runtime validation for the TaskFlow runtime family
3. [multi-agent-stage-ensemble-contract.md](multi-agent-stage-ensemble-contract.md)
   Config families: stage-level independent agent attempts, attempt artifacts, consolidation receipts, TaskFlow append-only update boundaries, and future runtime dispatch/status/collect/consolidate surfaces
4. [development-flow-catalog-schema-contract.md](development-flow-catalog-schema-contract.md)
   Config families: `vida.config.yaml -> dev_team.default_flow_id`, `dev_team.work_item_flow_bindings`, `dev_team.flows.*.ordered_steps`, `docs/process/agent-extensions/flows.yaml`, `docs/product/spec/hook-templates.yaml`, host-agent adapter projection fields, and future approval-gate fields
   Registered path: `docs/product/spec/development-flow-catalog-schema-contract.md`
5. [workflow-policy-loader-service-orchestration-contract.md](workflow-policy-loader-service-orchestration-contract.md)
   Config families: optional `WORKFLOW.md` service-orchestration policy overlay, `vida.config.yaml`, `docs/process/agent-extensions/flows.yaml`, prompt-template references, snapshot/reload projection, and TUI-visible validation errors
   Registered path: `docs/product/spec/workflow-policy-loader-service-orchestration-contract.md`
6. [agent-lane-selection-and-conversation-mode-model.md](agent-lane-selection-and-conversation-mode-model.md)
   Config families: overlay-driven auto-lane selection, bounded conversational modes, one-task scope/PBI discussion, and lawful handoff into pack/taskflow routing
7. [party-chat-v2-problem-party-model.md](party-chat-v2-problem-party-model.md)
   Config families: `docs/process/agent-extensions/**`, `vida.config.yaml`, `.vida/logs/problem-party/**`, single-agent or multi-agent Party Chat execution plans, and runtime consumption by the TaskFlow runtime family
8. [autonomous-report-continuation-law.md](autonomous-report-continuation-law.md)
   Config families: `vida.config.yaml`, `vida/config/instructions/**`, TaskFlow routing and autonomous execution behavior
9. [fast-high-signal-pre-commit-contract.md](fast-high-signal-pre-commit-contract.md)
   Config families: repository-local pre-commit hook matrix, file hygiene gates, lightweight script/diff checks, and Cargo-heavy hook exclusions for fast local proof
   Registered path: `docs/product/spec/fast-high-signal-pre-commit-contract.md`
10. [mempalace-vida-memory-implementation-model.md](mempalace-vida-memory-implementation-model.md)
   Config families: VIDA memory owner-law boundaries, MemPalace donor-pattern mapping, memory validity/supersession mechanics, and ordinary-search-first runtime implementation posture
   Registered path: `docs/product/spec/mempalace-vida-memory-implementation-model.md`
11. [production-observability-and-operator-baselines-contract.md](production-observability-and-operator-baselines-contract.md)
   Config families: production observability baselines, operator/tool contract fields, trace/evidence linkage, runtime SLO posture, and incident evidence bundle minimums
   Registered path: `docs/product/spec/production-observability-and-operator-baselines-contract.md`
12. [prompt-lifecycle-evaluation-and-safety-baseline-contract.md](prompt-lifecycle-evaluation-and-safety-baseline-contract.md)
   Config families: prompt rollout lifecycle, evaluation runs, feedback events, safety/adversarial baseline coverage, and operator-visible prompt-change evidence
   Registered path: `docs/product/spec/prompt-lifecycle-evaluation-and-safety-baseline-contract.md`
13. [config-driven-host-system-runtime-contract.md](config-driven-host-system-runtime-contract.md)
   Config families: `vida.config.yaml -> host_environment.systems`, host-system template roots, runtime roots, selected host-system resolution, and config-backed host-system materialization
   Registered path: `docs/product/spec/config-driven-host-system-runtime-contract.md`
14. [internal-backend-executor-route-policy-contract.md](internal-backend-executor-route-policy-contract.md)
   Config families: `vida.config.yaml -> agent_system.subagents`, route-level executor backend fields, internal/external backend registry authority, and compatibility aliases for legacy route hints
   Registered path: `docs/product/spec/internal-backend-executor-route-policy-contract.md`
15. [codex-host-agent-boundary-and-cli-bridge-contract.md](codex-host-agent-boundary-and-cli-bridge-contract.md)
   Config families: `vida.config.yaml -> host_environment.systems.codex`, `agent_system.subagents.internal_subagents`, `agent_system.subagents.codex_cli_exec`, TaskFlow host-bridge dispatch requests
   Registered path: `docs/product/spec/codex-host-agent-boundary-and-cli-bridge-contract.md`
16. [host-agent-bridge-adapter-contract.md](host-agent-bridge-adapter-contract.md)
   Config families: `vida.config.yaml -> host_environment.host_agent_bridge_contract`, `host_environment.systems.<system>.host_tool_bridge`, TaskFlow host-bridge request/result/receipt adapters for Codex, Claude Code, Pi, Vibe Kanban, OpenCode, and custom host adapters
   Registered path: `docs/product/spec/host-agent-bridge-adapter-contract.md`
17. [vida-coder-service-mode-executor-contract.md](vida-coder-service-mode-executor-contract.md)
   Config families: `vida.config.yaml -> agent_system.subagents.vida_coder`, `vida.config.yaml -> host_environment.systems.vida_coder`, `vida.config.yaml -> service`, `vida coder`, `vida service`, typed VIDA runtime tools, session state, service worker leases, provider auth/model readiness, MCP policy gateway, and receipt-backed TaskFlow execution
   Registered path: `docs/product/spec/vida-coder-service-mode-executor-contract.md`
18. [hybrid-host-executor-semantics-model.md](hybrid-host-executor-semantics-model.md)
   Config families: `vida.config.yaml -> host_environment`, `agent_system.subagents`, policy-selected internal/external executor semantics, and host posture versus executor backend separation
   Registered path: `docs/product/spec/hybrid-host-executor-semantics-model.md`
19. [compiled-autonomous-delivery-runtime-architecture.md](compiled-autonomous-delivery-runtime-architecture.md)
   Config families: `vida/config/instructions/**`, `.vida/config/**`, `.vida/project/**`, `.vida/cache/**`, transitional source-mode bridge surfaces such as root `vida.config.yaml` and `docs/process/agent-extensions/**`, TaskFlow runtime-family implementation surfaces, DocFlow runtime-family implementation surfaces, and future compiled orchestration bundle surfaces
20. [emerging-architectural-patterns-model.md](emerging-architectural-patterns-model.md)
   Config families: runtime loop ownership, specialist-agent topology, routing, verifier aggregation, persistent workflow state, production observability, evaluation posture, governance/security expectations, caching strategy, and gateway/proxy control surfaces across `vida/config/instructions/**`, TaskFlow runtime-family implementation surfaces, and future compiled runtime surfaces
21. [compiled-runtime-bundle-contract.md](compiled-runtime-bundle-contract.md)
    Config families: compiled control bundles with `control_core`, `activation_bundle`, `protocol_binding_registry`, and `cache_delivery_contract`, `.vida/config/**`, `.vida/project/**`, `.vida/db/**`, `.vida/cache/**`, runtime init/boot activation, bundle validation, and future machine-readable orchestration bundle surfaces
22. [project-activation-and-configurator-model.md](project-activation-and-configurator-model.md)
    Config families: DB-first project activation, `.vida/config/**`, `.vida/project/**`, roles/skills/profiles/flows/agents/teams/model/backend policy, sync/reconcile surfaces, and project lifecycle control
23. [team-coordination-model.md](team-coordination-model.md)
    Config families: team composition, coordination pattern, activation, shared policy, handoff/context posture, and closure semantics
24. [status-families-and-query-surface-model.md](status-families-and-query-surface-model.md)
    Config families: CLI query/status families, operator-facing render surfaces, bounded runtime snapshots, status-family routing, execution-preparation artifact queries, and routing/model-selection config-actuation census
    Registered path: `docs/product/spec/status-families-and-query-surface-model.md`
25. [project-protocol-promotion-law.md](project-protocol-promotion-law.md)
    Config families: known versus compiled project protocol admission, project discovery/mapping, executable bundle promotion, and fail-closed protocol binding
26. [taskflow-protocol-runtime-binding-model.md](taskflow-protocol-runtime-binding-model.md)
    Config families: script-era protocol binding bridge, Rust-native protocol runtime crate, activation resolution, gate enforcement, protocol receipts, binding matrices, and the dedicated TaskFlow protocol-binding subrelease
    Registered path: `docs/product/spec/taskflow-protocol-runtime-binding-model.md`
27. [user-facing-runtime-flow-and-operating-loop-model.md](user-facing-runtime-flow-and-operating-loop-model.md)
    Config families: operator-facing install/init/bootstrap flow, project-local runtime onboarding, project activation/config sequencing, intake/planning sequencing, execution/approval/resume sequencing, bounded pre-readiness allowlists, runtime bootstrap posture, and the staged user-facing operating loop across `.vida/**`, installed runtime assets, and DB-first readiness state
28. [bootstrap-carriers-and-project-activator-model.md](bootstrap-carriers-and-project-activator-model.md)
    Config families: bootstrap carriers, runtime init command split, project activator pipeline, sidecar/project-map enrichment, host-template onboarding, and bounded protocol-load separation between orchestrator and agent lanes
29. [execution-preparation-and-developer-handoff-model.md](execution-preparation-and-developer-handoff-model.md)
    Config families: `solution_architect`, execution preparation, architecture-preparation reports, developer handoff packets, change-boundary shaping, dependency-impact summaries, artifact registry query surfaces, and fail-closed pre-execution gating for code-shaped work
    Registered path: `docs/product/spec/execution-preparation-and-developer-handoff-model.md`
30. [ldrk-baseline/execution-preparation.md](ldrk-baseline/execution-preparation.md)
    Config families: LDRK `ldr-001` runtime mutation inventory, drift-map baseline, deletion candidates, generated baseline JSON, and execution-preparation artifact packet under `docs/product/spec/ldrk-baseline/**`
    Registered path: `docs/product/spec/ldrk-baseline/execution-preparation.md`
31. [ldrk-baseline/drift-map.md](ldrk-baseline/drift-map.md)
    Config families: generated LDRK `ldr-001` runtime mutation, classifier, command, and host-bridge drift-map evidence under `docs/product/spec/ldrk-baseline/**`
    Registered path: `docs/product/spec/ldrk-baseline/drift-map.md`
32. [ldrk-baseline/deletion-candidates.md](ldrk-baseline/deletion-candidates.md)
    Config families: generated LDRK `ldr-001` deletion and reduction candidate evidence under `docs/product/spec/ldrk-baseline/**`
    Registered path: `docs/product/spec/ldrk-baseline/deletion-candidates.md`
33. [ldrk-operation-catalog/operation-cli-map.json](ldrk-operation-catalog/operation-cli-map.json)
    Config families: generated LDRK `ldr-003` machine-readable command-to-operation catalog, baseline reduction counts, host-bridge outcome payload law, and compatibility alias policy
    Registered path: `docs/product/spec/ldrk-operation-catalog/operation-cli-map.json`
34. [ldrk-operation-catalog/before-after-command-tree.md](ldrk-operation-catalog/before-after-command-tree.md)
    Config families: generated LDRK `ldr-003` before/after command tree, canonical six-family target tree, command leaf reduction proof, and command-specific option reduction proof
    Registered path: `docs/product/spec/ldrk-operation-catalog/before-after-command-tree.md`
35. [ldrk-operation-catalog/top-ten-operator-workflow-walkthrough.md](ldrk-operation-catalog/top-ten-operator-workflow-walkthrough.md)
    Config families: generated LDRK `ldr-003` top-ten operator workflow migration walkthrough and host-bridge structured completion payload example
    Registered path: `docs/product/spec/ldrk-operation-catalog/top-ten-operator-workflow-walkthrough.md`
36. [local-durable-runtime-kernel-architecture-and-migration-law.md](local-durable-runtime-kernel-architecture-and-migration-law.md)
    Config families: accepted LDRK `ldr-004` event journal authority, projection semantics, command pipeline, aggregate boundaries, consistency levels, effect lifecycle, adapter boundaries, migration phase gates, and state classification law
    Registered path: `docs/product/spec/local-durable-runtime-kernel-architecture-and-migration-law.md`
37. [../decisions/ldr-004-local-durable-runtime-kernel-architecture-adr.md](../decisions/ldr-004-local-durable-runtime-kernel-architecture-adr.md)
    Config families: accepted LDRK `ldr-004` ADR for event journal mutation truth, no-dual-authority cutover, storage-neutral durable engine ports, owner boundaries, migration gates, and rejected alternatives
    Registered path: `docs/product/decisions/ldr-004-local-durable-runtime-kernel-architecture-adr.md`
30. [operational-state-and-synchronization-model.md](operational-state-and-synchronization-model.md)
    Config families: DB-first operational state, filesystem projection, Git lineage, synchronization law, conflict handling, and reactive domain routing
31. [multi-orchestrator-session-ownership-claims-contract.md](multi-orchestrator-session-ownership-claims-contract.md)
    Config families: orchestrator session identity, worktree-scoped claims, lease/heartbeat ownership, scoped status/continuation admission, foreign blocker visibility, and multi-session TaskFlow scheduling
    Registered path: `docs/product/spec/multi-orchestrator-session-ownership-claims-contract.md`
32. [session-scoped-orchestrator-protocol-foundation-contract.md](session-scoped-orchestrator-protocol-foundation-contract.md)
    Config families: session-scoped orchestrator protocol foundation, active claim admission, delegated lane ownership, exception takeover state naming, and continuation posture across TaskFlow, lane, status, and self-diagnostic surfaces
    Registered path: `docs/product/spec/session-scoped-orchestrator-protocol-foundation-contract.md`
33. [host-agent-layer-status-matrix.md](host-agent-layer-status-matrix.md)
    Config families: host-agent activation layers, overlay-owned tier ladders, tier selection economics, local score/state surfaces, task-close feedback ingestion, and status/budget observability over `.vida/state/**`
34. [specification-lane-scope-hardening-contract.md](specification-lane-scope-hardening-contract.md)
    Config families: runtime dispatch packet scope policy, tracked design-doc write ownership for specification lanes, downstream packet parity, and legacy packet normalization for task-class-aware scope hardening
35. [fail-closed-resume-closure-truth-contract.md](fail-closed-resume-closure-truth-contract.md)
    Config families: fail-closed resume-time packet reconciliation, persisted specification packet repair toward tracked design-doc scope, and active A1 recovery-truth closure for stale dispatch lineage
36. [lane-supersede-shared-truth-envelope-contract.md](lane-supersede-shared-truth-envelope-contract.md)
   Config families: explicit lane supersession mutation, shared lane-envelope truth derivation across `show`/`exception-takeover`/`supersede`, admissible-versus-active takeover posture, and recovery-adjacent lane-command discoverability
37. [implementation-backend-admissibility-selection-truth-contract.md](implementation-backend-admissibility-selection-truth-contract.md)
   Config families: implementation-lane backend admissibility truth, canonical selected-backend resolution, route-primary versus effective-backend diagnostic split, and packet/summary/status projection alignment
38. [stale-blocked-dispatch-artifact-reconciliation-contract.md](stale-blocked-dispatch-artifact-reconciliation-contract.md)
   Config families: blocked dispatch semantic-mismatch detection, run-graph projection stale truth beyond executing-only timeout cases, consume-resume continuation repair for obsolete blocked artifacts, and lane/status operator parity for mismatched blocked evidence
   Registered path: `docs/product/spec/stale-blocked-dispatch-artifact-reconciliation-contract.md`
39. [test-first-runtime-defect-remediation-model.md](test-first-runtime-defect-remediation-model.md)
   Config families: test-first runtime defect repair, cross-surface scenario contracts, operator actionability proof, paused defect reparenting, and one-bounded-defect-at-a-time remediation across TaskFlow runtime-family surfaces
   Registered path: `docs/product/spec/test-first-runtime-defect-remediation-model.md`
40. [agent-mode-test-first-delivery-flow-model.md](agent-mode-test-first-delivery-flow-model.md)
    Config families: config-derived agent-mode delivery, middle-tier test authoring, orchestrator-only root posture, continuous TaskFlow actualization, cost/effectiveness telemetry, and sequential/parallel lane gating
    Registered path: `docs/product/spec/agent-mode-test-first-delivery-flow-model.md`
41. [vida-service-tui-wizard-execution-spec.md](vida-service-tui-wizard-execution-spec.md)
    Config families: service/TUI/wizard command envelope, `vida-contracts`, operation catalog, project registry, wizard state machine, service-home coordination state, fixture/in-process client proof, and staged TUI/transport rollout
    Registered path: `docs/product/spec/vida-service-tui-wizard-execution-spec.md`
42. [tower-based-canonical-command-pipeline-phase-design.md](tower-based-canonical-command-pipeline-phase-design.md)
    Config families: `VidaCommandEnvelope`, `VidaCommandResponse`, service-client execution, command pipeline middleware order, operation metadata admission, idempotency, and runtime dispatch receipt boundaries for LDRK `ldr-040`
    Registered path: `docs/product/spec/tower-based-canonical-command-pipeline-phase-design.md`

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

### Runtime Operator Follow-Up Designs

1. [taskflow-task-command-parity-proxy-contract.md](taskflow-task-command-parity-proxy-contract.md)
   Config families: bounded Release-1 command parity for root `vida task`, compatibility routing for `vida taskflow task`, shared task-store mutation law, and help/proxy alignment
   Registered path: `docs/product/spec/taskflow-task-command-parity-proxy-contract.md`
2. [operator-output-envelope-and-bounded-rendering-contract.md](operator-output-envelope-and-bounded-rendering-contract.md)
   Config families: bounded operator-output policy/envelope seam, default-summary task list JSON, explicit full export through `--all`, and Release-1 envelope parity for task inspection surfaces
   Registered path: `docs/product/spec/operator-output-envelope-and-bounded-rendering-contract.md`
3. [project-agent-first-delegation-contract.md](project-agent-first-delegation-contract.md)
   Config families: bounded clarification and enforcement for project agent-first delegated execution, anti-pause continuation law, valid release-admission snapshot selection, and packet-minimum fail-closed runtime behavior
   Registered path: `docs/product/spec/project-agent-first-delegation-contract.md`
4. [release-admission-evidence-detection-contract.md](release-admission-evidence-detection-contract.md)
   Config families: bounded release-admission evidence detection, admissible final-snapshot precedence, operator artifact-ref parity, and effective-bundle receipt citation stability
   Registered path: `docs/product/spec/release-admission-evidence-detection-contract.md`
5. [ops-state-runtime-evidence-hygiene-contract.md](ops-state-runtime-evidence-hygiene-contract.md)
   Config families: bounded post-release ops policy for authoritative state roots, runtime-consumption evidence hygiene, temp-state proof workflows, and generated-state working-tree posture
   Registered path: `docs/product/spec/ops-state-runtime-evidence-hygiene-contract.md`
6. [authoritative-state-lock-recovery-contract.md](authoritative-state-lock-recovery-contract.md)
   Config families: bounded authoritative state-store lock-lifetime reduction during agent-lane dispatch, lock-specific remediation hints, and fail-closed long-lived-state recovery posture without silent lock cleanup
   Registered path: `docs/product/spec/authoritative-state-lock-recovery-contract.md`
7. [authoritative-state-access-serialization-contract.md](authoritative-state-access-serialization-contract.md)
   Config families: bounded authoritative state-access serialization, snapshot-first read-surface mitigation for lock contention, and explicit degraded-read truth for operator/task inspection surfaces
   Registered path: `docs/product/spec/authoritative-state-access-serialization-contract.md`
8. [design-backed-implementation-routing-contract.md](design-backed-implementation-routing-contract.md)
   Config families: bounded design-gate suppression for implementation-ready tasks, tracked-flow routing repair away from stale spec-pack re-entry, and implementation-oriented dispatch truth for already finalized design-backed work
   Registered path: `docs/product/spec/design-backed-implementation-routing-contract.md`
9. [oversized-runtime-module-split-contract.md](oversized-runtime-module-split-contract.md)
   Config families: bounded ownership-based split plan for oversized TaskFlow runtime modules, compatibility-preserving facade seams, execution-preparation requirements, module-map proof targets, and guarded rollout across `crates/vida/src/runtime_dispatch_state.rs`, `taskflow_consume_resume.rs`, `taskflow_run_graph.rs`, `taskflow_proxy.rs`, `task_surface.rs`, and `init_surfaces.rs`
   Registered path: `docs/product/spec/oversized-runtime-module-split-contract.md`
10. [internal-codex-agent-execution-fail-closed-contract.md](internal-codex-agent-execution-fail-closed-contract.md)
   Config families: bounded internal-host activation-view fail-closed semantics, truthful agent-lane execution state, root-session anti-bypass guidance, and runtime dispatch bridge hardening for Codex/internal execution
   Registered path: `docs/product/spec/internal-codex-agent-execution-fail-closed-contract.md`
11. [internal-dispatch-timeout-handoff-contract.md](internal-dispatch-timeout-handoff-contract.md)
   Config families: bounded internal-host delegated handoff timeout return semantics, prompt blocked receipt/result truth for stranded implementer handoffs, and runtime dispatch wrapper hardening beyond in-flight `executing` artifacts
   Registered path: `docs/product/spec/internal-dispatch-timeout-handoff-contract.md`
12. [internal-codex-timeout-reconciliation-contract.md](internal-codex-timeout-reconciliation-contract.md)
   Config families: bounded stale in-flight reconciliation, canonical dispatch timeout reuse for internal-host handoff truth, legacy fallback compatibility, and truthful continue/recovery status for still-executing delegated work
   Registered path: `docs/product/spec/internal-codex-timeout-reconciliation-contract.md`
13. [taskflow-execution-semantics-scheduler-contract.md](taskflow-execution-semantics-scheduler-contract.md)
   Config families: bounded TaskFlow task execution semantics schema, graph-plus-semantics scheduler projection, operator-visible parallel-admission truth, and fail-closed compatibility defaults for legacy tasks
   Registered path: `docs/product/spec/taskflow-execution-semantics-scheduler-contract.md`
14. [external-cli-carrier-hardening-contract.md](external-cli-carrier-hardening-contract.md)
   Config families: bounded external CLI carrier dispatch pinning, carrier readiness/status classification, normalized opencode/kilo/vibe project profiles, and operator-visible smoke-proof routing for sandbox/auth/model activation
   Registered path: `docs/product/spec/external-cli-carrier-hardening-contract.md`
15. [orchestrator-runtime-contract-hardening-contract.md](orchestrator-runtime-contract-hardening-contract.md)
   Config families: bounded orchestrator/agent/lane/status runtime contract hardening, path-scoped exception takeover truth, preview planner output, carrier selection API, lock-resilient init reads, and Codex App agent cleanup release rollout
   Registered path: `docs/product/spec/orchestrator-runtime-contract-hardening-contract.md`
16. [continuation-binding-fail-closed-contract.md](continuation-binding-fail-closed-contract.md)
   Config families: bounded continuation-binding fail-closed enforcement, explicit active-bounded-unit init/status surfaces, ambiguity blocker vocabulary, and generated host guidance against self-selecting adjacent work
   Registered path: `docs/product/spec/continuation-binding-fail-closed-contract.md`
17. [continuation-seeded-dispatch-bridge-contract.md](continuation-seeded-dispatch-bridge-contract.md)
   Config families: bounded explicit continuation binding records, seeded run dispatch-init bridges, persisted dispatch-context rows, and packet render/inspect operator surfaces for lawful resume inputs
   Registered path: `docs/product/spec/continuation-seeded-dispatch-bridge-contract.md`
18. [lawful-closure-continuation-rebinding-contract.md](lawful-closure-continuation-rebinding-contract.md)
   Config families: bounded explicit post-closure continuation rebinding, backlog-task continuation artifacts, completed-run summary admissibility, and fail-closed rejection of stale in-flight bindings
   Registered path: `docs/product/spec/lawful-closure-continuation-rebinding-contract.md`
19. [canonical-operator-command-map-export-contract.md](canonical-operator-command-map-export-contract.md)
   Config families: bounded operator command-family export through orchestrator-init and agent-init, help/discoverability alignment across root/task/taskflow surfaces, and canonical command-map parity between init views and primary help entrypoints
   Registered path: `docs/product/spec/canonical-operator-command-map-export-contract.md`
20. [reconciled-runtime-projection-output-contract.md](reconciled-runtime-projection-output-contract.md)
   Config families: bounded reconciled runtime projection truth output, effective projection source/reason reporting, downstream target/blocker parity, stale-state suspicion, and next-lawful-operator-action surfaces across init/status/recovery/continue
   Registered path: `docs/product/spec/reconciled-runtime-projection-output-contract.md`
21. [task-close-closure-truth-exception-contract.md](task-close-closure-truth-exception-contract.md)
   Config families: bounded downstream closure/task-close receipt sanitation for exception-path lineage, authoritative closure truth after lawful exception-backed task close, and resume/run-graph parity without stale implementer rebinding
   Registered path: `docs/product/spec/task-close-closure-truth-exception-contract.md`
22. [qwen-cli-reference-only-carrier-contract.md](qwen-cli-reference-only-carrier-contract.md)
   Config families: bounded reconciliation of stale `qwen_cli` assumptions across active docs/specs and Rust test fixtures, template-only retention policy for qwen references, and runtime/operator parity with the current config-driven external carrier catalog
   Registered path: `docs/product/spec/qwen-cli-reference-only-carrier-contract.md`
23. [selector-precedence-bounded-repair-contract.md](selector-precedence-bounded-repair-contract.md)
   Config families: bounded selector precedence repair, concrete Rust-file repair routing, planning/specification override boundaries, and provider-compatible lane-selection proof
   Registered path: `docs/product/spec/selector-precedence-bounded-repair-contract.md`
24. [spec-compliant-exception-path-takeover-surface-contract.md](spec-compliant-exception-path-takeover-surface-contract.md)
   Config families: root lane inspection, exception-takeover receipt recording, path-scoped takeover authority, lane/status/doctor evidence parity, and delegated-cycle fail-closed law
   Registered path: `docs/product/spec/spec-compliant-exception-path-takeover-surface-contract.md`
25. [retrieval-identity-memory-governance-contract.md](retrieval-identity-memory-governance-contract.md)
   Config families: retrieval trust registry evidence, principal/delegation identity, ACL-aware citations, memory-governance fields, and approval/audit linkage across runtime operator surfaces
   Registered path: `docs/product/spec/retrieval-identity-memory-governance-contract.md`
26. [dead-code-removal-admission-contract.md](dead-code-removal-admission-contract.md)
   Config families: conservative Rust dead-code admission, command/runtime entrypoint protection, reachability evidence, removal candidate tables, and proof requirements for architecture-refactor cleanup
   Registered path: `docs/product/spec/dead-code-removal-admission-contract.md`
27. [external-coach-retry-fallback-contract.md](external-coach-retry-fallback-contract.md)
   Config families: coach-lane retry artifact law, explicit review-backend rotation before internal fallback, fresh runtime-dispatch packet materialization for lawful retries, and fail-closed prevention of identical same-packet timeout loops
   Registered path: `docs/product/spec/external-coach-retry-fallback-contract.md`
28. [codex-app-agent-lifecycle-cleanup-contract.md](codex-app-agent-lifecycle-cleanup-contract.md)
   Config families: bounded Codex App agent lifecycle cleanup discipline, debug-safe `agent-init` startup rendering, configured reasoning-profile projection proof, and `.codex/**` materialization parity with `vida.config.yaml`
   Registered path: `docs/product/spec/codex-app-agent-lifecycle-cleanup-contract.md`
29. [implementation-closure-write-evidence-contract.md](implementation-closure-write-evidence-contract.md)
   Config families: implementation completion truth, closure-candidate reconciliation gates, diagnostic-lane versus write-evidence law, and fail-closed prevention of closure-ready projection from read-only analysis execution
   Registered path: `docs/product/spec/implementation-closure-write-evidence-contract.md`
30. [design-backed-implementation-seeding-scope-contract.md](design-backed-implementation-seeding-scope-contract.md)
   Config families: design-backed explicit implementation seeding, tracked design-doc injection into run-graph seed, implementer packet owned-path derivation from bounded file sets, and fail-closed dispatch-init without generic placeholder scope
   Registered path: `docs/product/spec/design-backed-implementation-seeding-scope-contract.md`
31. [carrier-model-profile-selection-runtime-model.md](carrier-model-profile-selection-runtime-model.md)
   Config families: bounded carrier plus model-profile contract normalization across Codex/internal/external execution surfaces, profile-aware runtime assignment truth, dispatch/status profile projection, and parity-safe materialization from legacy and new-style config
   Registered path: `docs/product/spec/carrier-model-profile-selection-runtime-model.md`
32. [unified-hybrid-runtime-selection-policy-contract.md](unified-hybrid-runtime-selection-policy-contract.md)
   Config families: bounded follow-up wave after the carrier/model-profile contract rollout, including dynamic-versus-route selection truth, budget and route policy enforcement, internal-subagent candidate pooling, external reasoning/readiness enforcement, operator diagnostics, and residual qwen drift closure
   Registered path: `docs/product/spec/unified-hybrid-runtime-selection-policy-contract.md`
33. [task-graph-adaptive-planner-contract.md](task-graph-adaptive-planner-contract.md)
   Config families: bounded TaskFlow PlanGraph generation and materialization, adaptive task split/spawn/replan mutations, graph explain diagnostics, scheduler dispatch preview, and task-linked execution-preparation artifact shape
   Registered path: `docs/product/spec/task-graph-adaptive-planner-contract.md`
34. [model-provider-price-catalog-lifecycle-contract.md](model-provider-price-catalog-lifecycle-contract.md)
   Config families: bounded model/provider price-catalog source-of-truth, provider/model availability inventory, freshness/source metadata, dry-run/apply receipt lifecycle, init/status readiness projection, and price-aware selected/rejected candidate diagnostics
   Registered path: `docs/product/spec/model-provider-price-catalog-lifecycle-contract.md`
35. [closure-admission-evidence-table-contract.md](closure-admission-evidence-table-contract.md)
   Config families: bounded closure-admission evidence crosswalk, canonical evidence-family minimums, operator blocker parity, and closure-bundle fail-closed semantics
   Registered path: `docs/product/spec/closure-admission-evidence-table-contract.md`
36. [taskflow-happy-path-test-catalog-contract.md](taskflow-happy-path-test-catalog-contract.md)
   Config families: bounded ordered TaskFlow happy-path test catalog, proof-target mapping, parent/child closure consistency gate, and immediate defect-epic repair through agent mode
   Registered path: `docs/product/spec/taskflow-happy-path-test-catalog-contract.md`
37. [pi-primary-environment-agent-carrier-spec.md](pi-primary-environment-agent-carrier-spec.md)
    Config families: Pi primary host environment selection/materialization, `pi_cli` external carrier profiles, `vida-pi-agent` adapter dispatch, Pi internal-agent projections, bounded write-scope guard, template propagation, and release/package proof
    Registered path: `docs/product/spec/pi-primary-environment-agent-carrier-spec.md`
38. [runtime-web-restart-current-repo-command-contract.md](runtime-web-restart-current-repo-command-contract.md)
    Config families: current-repo scoped web proof restart command, explicit edge-proxy restart opt-in, stale listener ownership checks, compact JSON restart receipts, and TaskFlow proof consumption for `vida runtime web restart`
    Registered path: `docs/product/spec/runtime-web-restart-current-repo-command-contract.md`
39. [typed-transition-state-store-extraction-contract.md](typed-transition-state-store-extraction-contract.md)
    Config families: typed TaskFlow lifecycle transitions, run-graph reconciliation authority, state-store adapter extraction, persisted state compatibility, public surface parity, and shared/core transition law for the `typed-transition-state-store-extraction-epic`
    Registered path: `docs/product/spec/typed-transition-state-store-extraction-contract.md`

## Routing Rule

1. Add newly promoted current product/spec artifacts here only when they are active project canon.
2. Keep [current-spec-map.md](current-spec-map.md) short; it should route to this catalog instead of duplicating the detailed list.

-----
artifact_path: product/spec/current-spec-catalog
artifact_type: product_spec
artifact_version: '1'
artifact_revision: 2026-06-21
schema_version: '1'
status: canonical
source_path: docs/product/spec/current-spec-catalog.md
created_at: 2026-06-12T00:00:00+03:00
updated_at: 2026-07-03T06:54:50.9354403Z
changelog_ref: current-spec-catalog.changelog.jsonl
