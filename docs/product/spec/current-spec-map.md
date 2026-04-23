# VIDA Current Spec Map

Status: active canonical map

Revision: `2026-04-10`

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
21. [specification-lane-scope-hardening-design.md](specification-lane-scope-hardening-design.md)
    Config families: runtime dispatch packet scope policy, tracked design-doc write ownership for specification lanes, downstream packet parity, and legacy packet normalization for task-class-aware scope hardening
22. [repair-fail-closed-resume-closure-truth-design.md](repair-fail-closed-resume-closure-truth-design.md)
    Config families: fail-closed resume-time packet reconciliation, persisted specification packet repair toward tracked design-doc scope, and active A1 recovery-truth closure for stale dispatch lineage
23. [lane-supersede-and-shared-truth-envelope-design.md](lane-supersede-and-shared-truth-envelope-design.md)
   Config families: explicit lane supersession mutation, shared lane-envelope truth derivation across `show`/`exception-takeover`/`supersede`, admissible-versus-active takeover posture, and recovery-adjacent lane-command discoverability
24. [implementation-backend-admissibility-and-selection-truth-design.md](implementation-backend-admissibility-and-selection-truth-design.md)
   Config families: implementation-lane backend admissibility truth, canonical selected-backend resolution, route-primary versus effective-backend diagnostic split, and packet/summary/status projection alignment
25. [coach-lane-inherits-internal-fallback-over-explicit-review-route-design.md](coach-lane-inherits-internal-fallback-over-explicit-review-route-design.md)
   Config families: coach-lane downstream backend canonicalization, explicit review-route preference over inherited internal fallback, mixed-lane backend lineage, and runtime dispatch receipt/status alignment
26. [blocked-external-coach-artifact-truth-not-reconciled-design.md](blocked-external-coach-artifact-truth-not-reconciled-design.md)
   Config families: blocked dispatch semantic-mismatch detection, run-graph projection stale truth beyond executing-only timeout cases, consume-resume continuation repair for obsolete blocked artifacts, and lane/status operator parity for mismatched blocked evidence
   Registered path: `docs/product/spec/blocked-external-coach-artifact-truth-not-reconciled-design.md`

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
2. [release-1-event-state-and-projection-topology-design.md](release-1-event-state-and-projection-topology-design.md)
   Config families: bounded event-state topology, projection-checkpoint contracts, replay/resumability alignment, optional feature-gated event backend posture, and SurrealDB-first projection authority for Release 1
   Registered path: `docs/product/spec/release-1-event-state-and-projection-topology-design.md`
3. [release-1-capability-matrix.md](release-1-capability-matrix.md)
   Config families: Release-1 capability ladder, cross-track closure, slice mapping, proof surfaces, and fail-closed seam ownership
   Registered path: `docs/product/spec/release-1-capability-matrix.md`
4. [release-1-seam-map.md](release-1-seam-map.md)
   Config families: Release-1 closure seam, TaskFlow-to-DocFlow activation/proof return, blocker classes, and final hardening admission
   Registered path: `docs/product/spec/release-1-seam-map.md`
5. [release-1-current-state.md](release-1-current-state.md)
   Config families: Release-1 readiness by slice/layer/seam, keep-versus-refactor posture, launcher concentration risk, and current state inputs
   Registered path: `docs/product/spec/release-1-current-state.md`
6. [release-1-closure-contract.md](release-1-closure-contract.md)
   Config families: Release-1 definition of done, non-waivable blockers, risk-acceptance law, and closure evidence bundle
   Registered path: `docs/product/spec/release-1-closure-contract.md`
7. [release-1-workflow-classification-and-risk-matrix.md](release-1-workflow-classification-and-risk-matrix.md)
   Config families: Release-1 workflow classes, risk tiers, approval posture, lifecycle variants, and supported workflow surface
   Registered path: `docs/product/spec/release-1-workflow-classification-and-risk-matrix.md`
8. [release-1-control-metrics-and-gates.md](release-1-control-metrics-and-gates.md)
   Config families: Release-1 control metrics, gate thresholds, release-candidate evidence windows, and workflow-tier gate binding
   Registered path: `docs/product/spec/release-1-control-metrics-and-gates.md`
9. [release-1-canonical-artifact-schemas.md](release-1-canonical-artifact-schemas.md)
   Config families: Release-1 minimum machine-readable contracts for traces, approvals, tool contracts, evaluation runs, incidents, memory records, and closure admission
   Registered path: `docs/product/spec/release-1-canonical-artifact-schemas.md`
10. [release-1-decision-tables.md](release-1-decision-tables.md)
   Config families: Release-1 executable control rules for approval, delegation, tool use, retrieval trust, memory writes, and rollback gates
   Registered path: `docs/product/spec/release-1-decision-tables.md`
11. [release-1-state-machine-specs.md](release-1-state-machine-specs.md)
   Config families: Release-1 canonical FSMs for lanes, approvals, tools, incidents, and prompt rollout
   Registered path: `docs/product/spec/release-1-state-machine-specs.md`
12. [release-1-error-and-exception-taxonomy.md](release-1-error-and-exception-taxonomy.md)
   Config families: Release-1 blocker codes, failure vocabulary, and exception-path taxonomy
   Registered path: `docs/product/spec/release-1-error-and-exception-taxonomy.md`
13. [release-1-ownership-to-code-map.md](release-1-ownership-to-code-map.md)
   Config families: Release-1 owner-doc to crate/module placement, shell boundary discipline, and anti-drift ownership mapping
   Registered path: `docs/product/spec/release-1-ownership-to-code-map.md`
14. [release-1-proof-scenario-catalog.md](release-1-proof-scenario-catalog.md)
   Config families: Release-1 minimum proof scenarios, negative-control scenarios, and scenario evidence requirements
   Registered path: `docs/product/spec/release-1-proof-scenario-catalog.md`
15. [release-1-schema-versioning-and-compatibility-law.md](release-1-schema-versioning-and-compatibility-law.md)
   Config families: Release-1 artifact schema evolution, compatibility classes, mixed-version rules, and migration law
   Registered path: `docs/product/spec/release-1-schema-versioning-and-compatibility-law.md`
16. [release-1-runtime-enum-and-code-contracts.md](release-1-runtime-enum-and-code-contracts.md)
   Config families: Release-1 canonical enum/value contracts for workflow classes, risk tiers, statuses, gate levels, blocker codes, and compatibility classes
   Registered path: `docs/product/spec/release-1-runtime-enum-and-code-contracts.md`
17. [release-1-conformance-matrix.md](release-1-conformance-matrix.md)
   Config families: Release-1 doc-to-code-to-proof mapping, conformance posture, and bounded implementation targets
   Registered path: `docs/product/spec/release-1-conformance-matrix.md`
18. [release-1-operator-surface-contract.md](release-1-operator-surface-contract.md)
   Config families: Release-1 stable operator output contracts for status, doctor, consume, lane, approval, and recovery surfaces
   Registered path: `docs/product/spec/release-1-operator-surface-contract.md`
19. [release-1-unsupported-surface-contract.md](release-1-unsupported-surface-contract.md)
   Config families: Release-1 unsupported and architecture-reserved surface boundaries and denial posture
   Registered path: `docs/product/spec/release-1-unsupported-surface-contract.md`
20. [release-1-fixture-and-golden-data-contract.md](release-1-fixture-and-golden-data-contract.md)
   Config families: Release-1 canonical fixtures, golden scenarios, and compatibility-proof sample contracts
   Registered path: `docs/product/spec/release-1-fixture-and-golden-data-contract.md`
21. [release-1-risk-acceptance-register.md](release-1-risk-acceptance-register.md)
   Config families: Release-1 explicit open-risk tracking, bounded acceptances, and closure-governance visibility
   Registered path: `docs/product/spec/release-1-risk-acceptance-register.md`
22. [taskflow-task-command-parity-and-proxy-alignment-design.md](taskflow-task-command-parity-and-proxy-alignment-design.md)
   Config families: bounded Release-1 command parity for root `vida task`, compatibility routing for `vida taskflow task`, shared task-store mutation law, and help/proxy alignment
   Registered path: `docs/product/spec/taskflow-task-command-parity-and-proxy-alignment-design.md`
23. [release-1-carrier-neutral-runtime-and-host-materialization-design.md](release-1-carrier-neutral-runtime-and-host-materialization-design.md)
   Config families: bounded Release-1 carrier-neutral runtime contracts, host-system materialization abstraction, runtime-assignment neutralization, and proof migration away from codex-era canonical names
   Registered path: `docs/product/spec/release-1-carrier-neutral-runtime-and-host-materialization-design.md`
24. [release-1-shared-operator-envelope-closure-design.md](release-1-shared-operator-envelope-closure-design.md)
   Config families: bounded Release-1 closure for shared operator-envelope fields, canonical compatibility-field emission, registry-backed blocker validation, and installed-launcher alignment
   Registered path: `docs/product/spec/release-1-shared-operator-envelope-closure-design.md`
25. [clarify-enforce-immediate-project-agent-first-design.md](clarify-enforce-immediate-project-agent-first-design.md)
   Config families: bounded clarification and enforcement for project agent-first delegated execution, anti-pause continuation law, valid release-admission snapshot selection, and packet-minimum fail-closed runtime behavior
   Registered path: `docs/product/spec/clarify-enforce-immediate-project-agent-first-design.md`
26. [fix-release-admission-evidence-detection-artifac-design.md](fix-release-admission-evidence-detection-artifac-design.md)
   Config families: bounded release-admission evidence detection, admissible final-snapshot precedence, operator artifact-ref parity, and effective-bundle receipt citation stability
   Registered path: `docs/product/spec/fix-release-admission-evidence-detection-artifac-design.md`
27. [clarify-enforce-immediate-continuation-shell-saf-design.md](clarify-enforce-immediate-continuation-shell-saf-design.md)
   Config families: bounded continuation recovery law, shell-safe backlog note recording, and help/prompt alignment for active continued-development sessions
   Registered path: `docs/product/spec/clarify-enforce-immediate-continuation-shell-saf-design.md`
28. [ops-state-and-runtime-evidence-hygiene-design.md](ops-state-and-runtime-evidence-hygiene-design.md)
   Config families: bounded post-release ops policy for authoritative state roots, runtime-consumption evidence hygiene, temp-state proof workflows, and generated-state working-tree posture
   Registered path: `docs/product/spec/ops-state-and-runtime-evidence-hygiene-design.md`
29. [authoritative-state-lock-recovery-design.md](authoritative-state-lock-recovery-design.md)
   Config families: bounded authoritative state-store lock-lifetime reduction during agent-lane dispatch, lock-specific remediation hints, and fail-closed long-lived-state recovery posture without silent lock cleanup
   Registered path: `docs/product/spec/authoritative-state-lock-recovery-design.md`
30. [serialize-authoritative-state-access-lock-mitigation-design.md](serialize-authoritative-state-access-lock-mitigation-design.md)
   Config families: bounded authoritative state-access serialization, snapshot-first read-surface mitigation for lock contention, and explicit degraded-read truth for operator/task inspection surfaces
   Registered path: `docs/product/spec/serialize-authoritative-state-access-lock-mitigation-design.md`
31. [existing-design-implementation-routing-blocked-design.md](existing-design-implementation-routing-blocked-design.md)
   Config families: bounded design-gate suppression for implementation-ready tasks, tracked-flow routing repair away from stale spec-pack re-entry, and implementation-oriented dispatch truth for already finalized design-backed work
   Registered path: `docs/product/spec/existing-design-implementation-routing-blocked-design.md`
32. [launcher-decomposition-and-code-hygiene-design.md](launcher-decomposition-and-code-hygiene-design.md)
   Config families: bounded launcher decomposition seams, large-file concentration reduction, dead-code and duplication validation, and proof-safe extraction planning for `crates/vida/**`
   Registered path: `docs/product/spec/launcher-decomposition-and-code-hygiene-design.md`
33. [internal-codex-agent-execution-fail-closed-design.md](internal-codex-agent-execution-fail-closed-design.md)
   Config families: bounded internal-host activation-view fail-closed semantics, truthful agent-lane execution state, root-session anti-bypass guidance, and runtime dispatch bridge hardening for Codex/internal execution
   Registered path: `docs/product/spec/internal-codex-agent-execution-fail-closed-design.md`
34. [internal-dispatch-timeout-does-not-return-design.md](internal-dispatch-timeout-does-not-return-design.md)
   Config families: bounded internal-host delegated handoff timeout return semantics, prompt blocked receipt/result truth for stranded implementer handoffs, and runtime dispatch wrapper hardening beyond in-flight `executing` artifacts
   Registered path: `docs/product/spec/internal-dispatch-timeout-does-not-return-design.md`
35. [internal-codex-activation-view-timeout-holder-release-design.md](internal-codex-activation-view-timeout-holder-release-design.md)
   Config families: bounded stale in-flight reconciliation, canonical dispatch timeout reuse for internal-host handoff truth, legacy fallback compatibility, and truthful continue/recovery status for still-executing delegated work
   Registered path: `docs/product/spec/internal-codex-activation-view-timeout-holder-release-design.md`
36. [coach-lane-inherits-internal-fallback-over-explicit-review-route-design.md](coach-lane-inherits-internal-fallback-over-explicit-review-route-design.md)
   Config families: bounded coach-lane backend canonicalization, explicit review-route preference over inherited internal fallback, mixed-lane backend lineage, and runtime dispatch receipt/status alignment
   Registered path: `docs/product/spec/coach-lane-inherits-internal-fallback-over-explicit-review-route-design.md`
37. [external-coach-timeout-truth-does-not-return-cleanly-design.md](external-coach-timeout-truth-does-not-return-cleanly-design.md)
   Config families: bounded external coach timeout truth, timeout classification by actual backend class, bounded parent return after kill-after grace, and external dispatch artifact/status alignment
   Registered path: `docs/product/spec/external-coach-timeout-truth-does-not-return-cleanly-design.md`
37. [taskflow-execution-semantics-and-scheduler-design.md](taskflow-execution-semantics-and-scheduler-design.md)
   Config families: bounded TaskFlow task execution semantics schema, graph-plus-semantics scheduler projection, operator-visible parallel-admission truth, and fail-closed compatibility defaults for legacy tasks
   Registered path: `docs/product/spec/taskflow-execution-semantics-and-scheduler-design.md`
38. [external-cli-carrier-hardening-design.md](external-cli-carrier-hardening-design.md)
   Config families: bounded external CLI carrier dispatch pinning, carrier readiness/status classification, normalized opencode/kilo/vibe project profiles, and operator-visible smoke-proof routing for sandbox/auth/model activation
   Registered path: `docs/product/spec/external-cli-carrier-hardening-design.md`
36. [continuation-binding-fail-closed-hardening-design.md](continuation-binding-fail-closed-hardening-design.md)
   Config families: bounded continuation-binding fail-closed enforcement, explicit active-bounded-unit init/status surfaces, ambiguity blocker vocabulary, and generated host guidance against self-selecting adjacent work
   Registered path: `docs/product/spec/continuation-binding-fail-closed-hardening-design.md`
37. [continuation-and-seeded-dispatch-bridge-design.md](continuation-and-seeded-dispatch-bridge-design.md)
   Config families: bounded explicit continuation binding records, seeded run dispatch-init bridges, persisted dispatch-context rows, and packet render/inspect operator surfaces for lawful resume inputs
   Registered path: `docs/product/spec/continuation-and-seeded-dispatch-bridge-design.md`
38. [lawful-closure-continuation-rebinding-design.md](lawful-closure-continuation-rebinding-design.md)
   Config families: bounded explicit post-closure continuation rebinding, backlog-task continuation artifacts, completed-run summary admissibility, and fail-closed rejection of stale in-flight bindings
   Registered path: `docs/product/spec/lawful-closure-continuation-rebinding-design.md`
39. [export-canonical-operator-command-map-through-design.md](export-canonical-operator-command-map-through-design.md)
   Config families: bounded operator command-family export through orchestrator-init and agent-init, help/discoverability alignment across root/task/taskflow surfaces, and canonical command-map parity between init views and primary help entrypoints
   Registered path: `docs/product/spec/export-canonical-operator-command-map-through-design.md`
40. [reconciled-runtime-projection-output-design.md](reconciled-runtime-projection-output-design.md)
   Config families: bounded reconciled runtime projection truth output, effective projection source/reason reporting, downstream target/blocker parity, stale-state suspicion, and next-lawful-operator-action surfaces across init/status/recovery/continue
   Registered path: `docs/product/spec/reconciled-runtime-projection-output-design.md`
41. [repair-task-close-closure-truth-exception-design.md](repair-task-close-closure-truth-exception-design.md)
   Config families: bounded downstream closure/task-close receipt sanitation for exception-path lineage, authoritative closure truth after lawful exception-backed task close, and resume/run-graph parity without stale implementer rebinding
   Registered path: `docs/product/spec/repair-task-close-closure-truth-exception-design.md`
42. [reconcile-qwen-cli-carrier-drift-design.md](reconcile-qwen-cli-carrier-drift-design.md)
   Config families: bounded reconciliation of stale `qwen_cli` assumptions across active docs/specs and Rust test fixtures, template-only retention policy for qwen references, and runtime/operator parity with the current config-driven external carrier catalog
   Registered path: `docs/product/spec/reconcile-qwen-cli-carrier-drift-design.md`
43. [repair-explicit-continuation-bind-preservation-after-qwen-rebind-design.md](repair-explicit-continuation-bind-preservation-after-qwen-rebind-design.md)
   Config families: bounded repair for explicit post-closure task binding authority, agent-init activation-view truth preservation, and fail-closed consume-continue behavior until fresh same-task packet evidence exists
   Registered path: `docs/product/spec/repair-explicit-continuation-bind-preservation-after-qwen-rebind-design.md`
44. [repair-design-backed-reseed-canonicalization-does-not-deadlock-qwen-design.md](repair-design-backed-reseed-canonicalization-does-not-deadlock-qwen-design.md)
   Config families: design-backed reseed routing, dispatch-target canonicalization, and activation/backend alignment so explicit qwen remediation does not deadlock in `pbi_discussion/specification`
   Registered path: `docs/product/spec/repair-design-backed-reseed-canonicalization-does-not-deadlock-qwen-design.md`
45. [coach-retry-reuses-same-blocked-hermes-packet-without-fallback-design.md](coach-retry-reuses-same-blocked-hermes-packet-without-fallback-design.md)
   Config families: coach-lane retry artifact law, explicit review-backend rotation before internal fallback, fresh runtime-dispatch packet materialization for lawful retries, and fail-closed prevention of identical same-packet timeout loops
   Registered path: `docs/product/spec/coach-retry-reuses-same-blocked-hermes-packet-without-fallback-design.md`
46. [analysis-lane-can-close-implementation-without-write-evidence-design.md](analysis-lane-can-close-implementation-without-write-evidence-design.md)
   Config families: implementation completion truth, closure-candidate reconciliation gates, diagnostic-lane versus write-evidence law, and fail-closed prevention of closure-ready projection from read-only analysis execution
   Registered path: `docs/product/spec/analysis-lane-can-close-implementation-without-write-evidence-design.md`
47. [explicit-implementation-seed-drops-design-backed-owned-paths-design.md](explicit-implementation-seed-drops-design-backed-owned-paths-design.md)
   Config families: design-backed explicit implementation seeding, tracked design-doc injection into run-graph seed, implementer packet owned-path derivation from bounded file sets, and fail-closed dispatch-init without generic placeholder scope
   Registered path: `docs/product/spec/explicit-implementation-seed-drops-design-backed-owned-paths-design.md`
48. [carrier-model-profile-selection-runtime-design.md](carrier-model-profile-selection-runtime-design.md)
   Config families: bounded carrier plus model-profile contract normalization across Codex/internal/external execution surfaces, profile-aware runtime assignment truth, dispatch/status profile projection, and parity-safe materialization from legacy and new-style config
   Registered path: `docs/product/spec/carrier-model-profile-selection-runtime-design.md`
49. [unified-hybrid-runtime-selection-policy-design.md](unified-hybrid-runtime-selection-policy-design.md)
   Config families: bounded follow-up wave after the carrier/model-profile contract rollout, including dynamic-versus-route selection truth, budget and route policy enforcement, internal-subagent candidate pooling, external reasoning/readiness enforcement, operator diagnostics, and residual qwen drift closure
   Registered path: `docs/product/spec/unified-hybrid-runtime-selection-policy-design.md`

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
artifact_revision: 2026-04-13
schema_version: '1'
status: canonical
source_path: docs/product/spec/current-spec-map.md
created_at: '2026-03-09T20:28:59+02:00'
updated_at: 2026-04-23T06:56:05.323250297Z
changelog_ref: current-spec-map.changelog.jsonl
