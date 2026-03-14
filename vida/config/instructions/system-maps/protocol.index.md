# Protocol Index (Single Source Map)

Purpose: one entry point for canonical protocol discovery and protocol-domain routing.

Registry boundary:

1. this file is a canonical discovery registry, not a second owner-layer taxonomy,
2. row text must stay thin enough to route readers toward the correct owner, capsule, or companion map,
3. when domain classification is needed, route to `framework.protocol-domains-map.md`,
4. when owner-layer placement is needed, route to `framework.protocol-layers-map.md`,
5. when routine runtime/startup consumption is already covered by an approved compact capsule or startup bundle, route there first and open the heavier owner artifact only on demand.

## Canonical Sources

| Domain | Canonical Source | Secondary/Reference |
|---|---|---|
| Framework topology map | `system-maps/framework.map` | `system-maps/protocol.index` |
| Framework core protocol cluster map | `system-maps/framework.core-protocols-map` | `system-maps/framework.index`, `system-maps/framework.map`, `system-maps/protocol.index` |
| Framework protocol layers map | `system-maps/framework.protocol-layers-map` | `system-maps/framework.index`, `system-maps/framework.map`, `docs/product/spec/framework-project-documentation-layer-model.md` |
| Framework protocol domains map | `system-maps/framework.protocol-domains-map` | `system-maps/framework.index`, `system-maps/framework.map`, `system-maps/framework.core-protocols-map`, `instruction-contracts/bridge.instruction-activation-protocol` |
| Governance discovery map | `system-maps/governance.map` | `AGENTS.md`, `CONTRIBUTING.md`, `vida/config/policies/*.yaml`, `runtime-instructions/work.human-approval-protocol`, `runtime-instructions/bridge.task-approval-loop-protocol` |
| Observability discovery map | `system-maps/observability.map` | `runtime-instructions/core.run-graph-protocol`, `runtime-instructions/core.context-governance-protocol`, `runtime-instructions/observability.trace-grading-protocol`, `diagnostic-instructions/evaluation.product-proving-pack-scaffold-contract` |
| Runtime family discovery | `system-maps/runtime-family.index` | `system-maps/runtime-family.docflow-map`, `system-maps/runtime-family.taskflow-map` |
| Orchestrator runtime boot capsule (compact projection) | `system-maps/bootstrap.orchestrator-runtime-capsule` | `AGENTS.md`, `agent-definitions/entry.orchestrator-entry`, `system-maps/bootstrap.orchestrator-boot-flow` |
| Template discovery map | `system-maps/template.map` | `docs/framework/templates/vida.config.yaml.template`, `prompt-templates/worker.packet-templates`, `prompt-templates/cheap-worker.prompt-pack-reference`, `agent-definitions/model.agent-definitions-contract` |
| Command layer matrix | `command-instructions/routing.command-layer-protocol` | `command-instructions/operator.command-catalog-index`, `command-instructions/execution.implement-execution-protocol`, `command-instructions/execution.bug-fix-protocol`, `command-instructions/routing.use-case-packs-protocol`, `runtime-instructions/work.taskflow-protocol`, `instruction-contracts/lane.worker-dispatch-protocol`, `system-maps/tooling.search-guide` |
| Runtime script architecture | `system-maps/migration.script-runtime-architecture-map` | `system-maps/framework.map`, `system-maps/runtime-family.taskflow-map` |
| Runtime transition map | `system-maps/migration.runtime-transition-map` | `system-maps/migration.script-runtime-architecture-map`, `system-maps/runtime-family.taskflow-map`, `runtime-instructions/work.taskflow-protocol` |
| Tooling and search guide | `system-maps/tooling.search-guide` | `command-instructions/operator.runtime-pipeline-guide`, `AGENTS.md` |
| Framework history evidence | `sidecar changelog plus Git history` | `system-maps/protocol.index` |
| Instruction activation and decomposition | `instruction-contracts/bridge.instruction-activation-protocol` | `AGENTS.md`, `agent-definitions/entry.orchestrator-entry`, `system-maps/protocol.index` |
| Protocol naming grammar and rename-wave law | `instruction-contracts/meta.protocol-naming-grammar-protocol` | `docs/product/spec/instruction-artifact-model.md` |
| Core protocol standard and boundary law | `instruction-contracts/meta.core-protocol-standard-protocol` | `system-maps/framework.core-protocols-map`, `system-maps/framework.protocol-layers-map`, `instruction-contracts/meta.protocol-naming-grammar-protocol`, `docs/product/spec/framework-project-documentation-layer-model.md`, `docs/product/spec/canonical-runtime-layer-matrix.md` |
| Documentation operation using only green documentation layers | `instruction-contracts/work.documentation-operation-protocol` | `instruction-contracts/bridge.instruction-activation-protocol`, `agent-definitions/entry.orchestrator-entry`, `AGENTS.sidecar.md`, `docs/process/documentation-tooling-map.md`, `system-maps/runtime-family.docflow-map` |
| Documentation migration of any project toward Layer 7 closure | `instruction-contracts/work.documentation-layer7-migration-protocol` | `instruction-contracts/work.documentation-operation-protocol`, `instruction-contracts/bridge.instruction-activation-protocol`, `docs/process/documentation-tooling-map.md`, `system-maps/template.map`, `system-maps/runtime-family.docflow-map` |
| Step-thinking runtime capsule (compact projection) | `instruction-contracts/overlay.step-thinking-runtime-capsule` | `instruction-contracts/overlay.step-thinking-protocol`, `AGENTS.md`, `agent-definitions/entry.orchestrator-entry` |
| Core orchestration runtime capsule (compact projection) | `instruction-contracts/core.orchestration-runtime-capsule` | `instruction-contracts/core.orchestration-protocol`, `agent-definitions/entry.orchestrator-entry`, `docs/process/project-orchestrator-operating-protocol.md` |
| Core agent-system runtime capsule (compact projection) | `instruction-contracts/core.agent-system-runtime-capsule` | `instruction-contracts/core.agent-system-protocol`, `agent-definitions/entry.orchestrator-entry`, `system-maps/bootstrap.orchestrator-boot-flow` |
| Autonomous execution runtime capsule (compact projection) | `instruction-contracts/overlay.autonomous-execution-runtime-capsule` | `instruction-contracts/overlay.autonomous-execution-protocol`, `instruction-contracts/bridge.instruction-activation-runtime-capsule`, `runtime-instructions/bridge.project-overlay-runtime-capsule` |
| Verification lane runtime capsule (compact projection) | `runtime-instructions/work.verification-lane-runtime-capsule` | `runtime-instructions/work.verification-lane-protocol`, `agent-definitions/entry.orchestrator-entry`, `instruction-contracts/bridge.instruction-activation-runtime-capsule` |
| Instruction activation runtime capsule (compact projection) | `instruction-contracts/bridge.instruction-activation-runtime-capsule` | `instruction-contracts/bridge.instruction-activation-protocol`, `AGENTS.md`, `agent-definitions/entry.orchestrator-entry` |
| Project overlay runtime capsule (compact projection) | `runtime-instructions/bridge.project-overlay-runtime-capsule` | `runtime-instructions/bridge.project-overlay-protocol`, `AGENTS.md`, `agent-definitions/entry.orchestrator-entry` |
| Agent definition model/spec contract | `agent-definitions/model.agent-definitions-contract` | `docs/product/spec/instruction-artifact-model.md`, `docs/product/spec/skill-management-and-activation-law.md`, `system-maps/template.map`, `agent-definitions/entry.worker-entry`, `agent-definitions/entry.orchestrator-entry` |
| Orchestrator role contract | `instruction-contracts/role.orchestrator-contract` | `agent-definitions/entry.orchestrator-entry`, `system-maps/framework.protocol-domains-map`, `docs/process/framework-three-layer-refactoring-audit.md` |
| Worker role contract | `instruction-contracts/role.worker-contract` | `agent-definitions/entry.worker-entry`, `system-maps/framework.protocol-domains-map`, `docs/process/framework-three-layer-refactoring-audit.md` |
| Skill activation for orchestrator and worker lanes | `instruction-contracts/core.skill-activation-protocol` | `AGENTS.md`, `instruction-contracts/core.orchestration-protocol`, `instruction-contracts/lane.worker-dispatch-protocol`, `docs/process/project-skill-initialization-and-activation-protocol.md` |
| Bounded packet decomposition and JIT refinement | `instruction-contracts/core.packet-decomposition-protocol` | `instruction-contracts/core.orchestration-protocol`, `instruction-contracts/lane.worker-dispatch-protocol`, `docs/process/team-development-and-orchestration-protocol.md`, `docs/process/project-development-packet-template-protocol.md` |
| Agent prompt stack precedence | `instruction-contracts/core.agent-prompt-stack-protocol` | `instruction-contracts/core.orchestration-protocol`, `instruction-contracts/lane.worker-dispatch-protocol`, `docs/process/project-agent-prompt-stack-protocol.md` |
| Role profile contract | `agent-definitions/role.role-profile-contract` | `agent-definitions/model.agent-definitions-contract`, `docs/product/spec/instruction-artifact-model.md`, `docs/process/framework-source-lineage-index.md` |
| Agent-system new-protocol development, update, and safe optimization rollout | `instruction-contracts/work.agent-system-new-protocol-development-and-update-protocol` | `instruction-contracts/core.agent-system-protocol`, `docs/product/spec/instruction-artifact-model.md`, `system-maps/framework.protocol-domains-map`, `system-maps/protocol.index`, `instruction-contracts/work.documentation-operation-protocol`, `command-instructions/routing.command-layer-protocol` |
| Agent-system new-protocol artifact templates (non-canonical reference) | `references/protocol.agent-system-new-protocol-artifact-templates` | `instruction-contracts/work.agent-system-new-protocol-development-and-update-protocol`, `system-maps/template.map` |
| Project agent extension | `runtime-instructions/work.project-agent-extension-protocol` | `vida.config.yaml`, `docs/process/agent-extensions/README.md`, `docs/product/spec/agent-role-skill-profile-flow-model.md`, `instruction-contracts/core.agent-system-protocol`, `system-maps/runtime-family.taskflow-map` |
| Agent lane selection | `runtime-instructions/work.agent-lane-selection-protocol` | `vida.config.yaml`, `agent-definitions/entry.orchestrator-entry`, `command-instructions/routing.use-case-packs-protocol`, `command-instructions/planning.form-task-protocol`, `docs/product/spec/agent-lane-selection-and-conversation-mode-model.md` |
| Autonomous follow-through mode | `instruction-contracts/overlay.autonomous-execution-protocol` | `command-instructions/execution.implement-execution-protocol`, `runtime-instructions/work.taskflow-protocol`, `runtime-instructions/runtime.task-state-telemetry-protocol`, `instruction-contracts/core.agent-system-protocol` |
| Autonomous next-task selector helper | `autonomous-next-task.py` | `instruction-contracts/overlay.autonomous-execution-protocol`, `runtime-instructions/work.execution-priority-protocol`, `system-maps/runtime-family.taskflow-map` |
| Execution prioritization | `runtime-instructions/work.execution-priority-protocol` | `command-instructions/planning.form-task-protocol`, `runtime-instructions/work.taskflow-protocol`, `command-instructions/execution.implement-execution-protocol`, `instruction-contracts/overlay.autonomous-execution-protocol` |
| Execution health-check and close/handoff gates | `runtime-instructions/work.execution-health-check-protocol` | `command-instructions/operator.runtime-pipeline-guide`, `runtime-instructions/work.taskflow-protocol`, `runtime-instructions/work.web-validation-protocol`, `runtime-instructions/bridge.project-overlay-protocol` |
| Command execution discipline | `runtime-instructions/work.command-execution-discipline-protocol` | `command-instructions/operator.runtime-pipeline-guide`, `system-maps/tooling.search-guide`, `runtime-instructions/bridge.project-overlay-protocol` |
| Project overlay activation | `runtime-instructions/bridge.project-overlay-protocol` | `vida.config.yaml`, `system-maps/template.map`, `AGENTS.md`, `system-maps/runtime-family.taskflow-map` |
| Development evidence sync | `runtime-instructions/work.development-evidence-sync-protocol` | `docs/process/vida1-development-conditions.md`, `runtime-instructions/bridge.project-overlay-protocol`, `runtime-instructions/bridge.spec-sync-protocol`, `instruction-contracts/work.documentation-operation-protocol` |
| Boot packet artifact | `runtime-instructions/model.boot-packet-protocol` | `system-maps/runtime-family.taskflow-map`, `AGENTS.md`, `agent-definitions/entry.orchestrator-entry`, `agent-definitions/entry.worker-entry` |
| Orchestrator boot flow | `system-maps/bootstrap.orchestrator-boot-flow` | `AGENTS.md`, `agent-definitions/entry.orchestrator-entry`, `instruction-contracts/bridge.instruction-activation-protocol` |
| Worker boot flow | `system-maps/bootstrap.worker-boot-flow` | `AGENTS.md`, `agent-definitions/entry.worker-entry`, `instruction-contracts/role.worker-thinking`, `instruction-contracts/lane.worker-dispatch-protocol` |
| Project bootstrap/self-reproduction | `command-instructions/execution.project-bootstrap-protocol` | `system-maps/runtime-family.taskflow-map`, `system-maps/template.map`, `vida.config.yaml` |
| Host CLI agent setup during project activation | `runtime-instructions/work.host-cli-agent-setup-protocol` | `command-instructions/execution.project-bootstrap-protocol`, `docs/product/spec/bootstrap-carriers-and-project-activator-model.md`, `docs/process/codex-agent-configuration-guide.md` |
| VIDA framework self-analysis | `diagnostic-instructions/analysis.framework-self-analysis-protocol` | `system-maps/framework.map`, `diagnostic-instructions/analysis.self-reflection-protocol` |
| Silent framework diagnosis | `diagnostic-instructions/analysis.silent-framework-diagnosis-protocol` | `vida-silent-diagnosis.py`, `vida.config.yaml`, `diagnostic-instructions/analysis.framework-self-analysis-protocol`, `runtime-instructions/work.taskflow-protocol` |
| Protocol consistency audit | `diagnostic-instructions/analysis.protocol-consistency-audit-protocol` | `system-maps/framework.protocol-layers-map`, `system-maps/framework.protocol-domains-map`, `system-maps/protocol.index`, `instruction-contracts/bridge.instruction-activation-protocol`, `docs/process/framework-three-layer-refactoring-audit.md` |
| Human approval receipts and gates | `runtime-instructions/work.human-approval-protocol` | `system-maps/governance.map`, `instruction-contracts/core.agent-system-protocol`, `system-maps/runtime-family.taskflow-map` |
| Framework memory ledger | `runtime-instructions/runtime.framework-memory-protocol` | `diagnostic-instructions/analysis.silent-framework-diagnosis-protocol`, `system-maps/governance.map`, `system-maps/runtime-family.taskflow-map` |
| DB-first runtime ownership | `docs/process/framework-source-lineage-index.md` | `docs/process/framework-source-lineage-index.md`, `docs/process/framework-source-lineage-index.md`, `runtime-instructions/runtime.export-protocol` |
| Export surfaces | `runtime-instructions/runtime.export-protocol` | `docs/process/framework-source-lineage-index.md`, `docs/process/framework-source-lineage-index.md` |
| Spec sync | `runtime-instructions/bridge.spec-sync-protocol` | `instruction-contracts/overlay.autonomous-execution-protocol`, `command-instructions/execution.implement-execution-protocol` |
| Spec freshness and newer-decision precedence | `runtime-instructions/work.spec-freshness-protocol` | `runtime-instructions/bridge.spec-sync-protocol`, `runtime-instructions/bridge.task-approval-loop-protocol` |
| Protocol self-diagnosis and runtime drift checks | `diagnostic-instructions/analysis.protocol-self-diagnosis-protocol` | `runtime-instructions/work.taskflow-protocol`, `instruction-contracts/overlay.autonomous-execution-protocol`, `instruction-contracts/core.agent-system-protocol`, `runtime-instructions/bridge.spec-sync-protocol`, `diagnostic-instructions/analysis.silent-framework-diagnosis-protocol` |
| Repeated technical failure escalation | `diagnostic-instructions/escalation.debug-escalation-protocol` | `instruction-contracts/core.agent-system-protocol`, `instruction-contracts/lane.worker-dispatch-protocol`, `runtime-instructions/work.web-validation-protocol`, `instruction-contracts/overlay.autonomous-execution-protocol` |
| Library evaluation | `diagnostic-instructions/evaluation.library-evaluation-protocol` | `diagnostic-instructions/escalation.debug-escalation-protocol`, `runtime-instructions/bridge.spec-sync-protocol` |
| Task approval loop | `runtime-instructions/bridge.task-approval-loop-protocol` | `instruction-contracts/overlay.autonomous-execution-protocol`, `runtime-instructions/work.human-approval-protocol` |
| Document lifecycle and freshness | `runtime-instructions/work.document-lifecycle-protocol` | `doc-lifecycle.py`, `runtime-instructions/bridge.project-overlay-protocol`, `diagnostic-instructions/analysis.silent-framework-diagnosis-protocol` |
| Context governance ledger | `runtime-instructions/core.context-governance-protocol` | `system-maps/governance.map`, `system-maps/runtime-family.taskflow-map`, `instruction-contracts/core.agent-system-protocol`, `instruction-contracts/core.orchestration-protocol`, `runtime-instructions/core.run-graph-protocol` |
| Durable run-graph ledger | `runtime-instructions/core.run-graph-protocol` | `system-maps/runtime-family.taskflow-map`, `runtime-instructions/work.taskflow-protocol`, `instruction-contracts/core.orchestration-protocol`, `runtime-instructions/core.context-governance-protocol`, `instruction-contracts/bridge.instruction-activation-protocol`, `system-maps/bootstrap.orchestrator-boot-flow`, `system-maps/bootstrap.worker-boot-flow` |
| Agent handoff, next-agent prompt formation, and context shaping | `runtime-instructions/lane.agent-handoff-context-protocol` | `instruction-contracts/lane.worker-dispatch-protocol`, `instruction-contracts/overlay.session-context-continuity-protocol`, `prompt-templates/worker.packet-templates`, `runtime-instructions/core.context-governance-protocol`, `docs/process/framework-source-lineage-index.md` |
| Checkpoint, replay, and recovery | `runtime-instructions/recovery.checkpoint-replay-recovery-protocol` | `runtime-instructions/core.run-graph-protocol`, `runtime-instructions/core.context-governance-protocol`, `docs/product/spec/checkpoint-commit-and-replay-model.md`, `system-maps/runtime-family.taskflow-map`, `instruction-contracts/bridge.instruction-activation-protocol`, `system-maps/bootstrap.orchestrator-boot-flow`, `system-maps/bootstrap.worker-boot-flow` |
| Verification lane and proving boundary | `runtime-instructions/work.verification-lane-protocol` | `instruction-contracts/core.orchestration-protocol`, `instruction-contracts/core.agent-system-protocol`, `diagnostic-instructions/evaluation.product-proving-pack-scaffold-contract`, `docs/process/framework-source-lineage-index.md`, `instruction-contracts/bridge.instruction-activation-protocol`, `system-maps/bootstrap.orchestrator-boot-flow`, `system-maps/bootstrap.worker-boot-flow` |
| Runtime kernel bundle composition | `runtime-instructions/runtime.runtime-kernel-bundle-protocol` | `system-maps/runtime-family.taskflow-map`, `bundles/default_runtime.yaml`, `docs/product/spec/partial-development-kernel-model.md`, `docs/product/spec/canonical-machine-map.md`, `docs/product/spec/receipt-and-proof-law.md` |
| Verification merge and admissibility | `runtime-instructions/work.verification-merge-protocol` | `system-maps/runtime-family.taskflow-map`, `docs/product/spec/verification-merge-law.md`, `runtime-instructions/work.verification-lane-protocol`, `instruction-contracts/bridge.instruction-activation-protocol`, `system-maps/bootstrap.orchestrator-boot-flow`, `system-maps/bootstrap.worker-boot-flow` |
| Direct runtime consumption | `runtime-instructions/runtime.direct-runtime-consumption-protocol` | `system-maps/runtime-family.taskflow-map`, `system-maps/runtime-family.docflow-map`, `docs/process/framework-source-lineage-index.md` |
| Local trace grading and datasets | `runtime-instructions/observability.trace-grading-protocol` | `system-maps/runtime-family.taskflow-map`, `runtime-instructions/work.taskflow-protocol` |
| Typed capability registry | `runtime-instructions/core.capability-registry-protocol` | `instruction-contracts/core.agent-system-protocol`, `instruction-contracts/core.orchestration-protocol`, `runtime-instructions/work.agent-lane-selection-protocol`, `instruction-contracts/bridge.instruction-activation-protocol`, `system-maps/runtime-family.taskflow-map`, `vida.config.yaml` |
| Task-state reconciliation | `runtime-instructions/work.task-state-reconciliation-protocol` | `system-maps/runtime-family.taskflow-map`, `runtime-instructions/work.taskflow-protocol`, `runtime-instructions/runtime.task-state-telemetry-protocol` |
| Problem-party decision protocol | `runtime-instructions/work.problem-party-protocol` | `vida taskflow problem-party`, `instruction-contracts/core.orchestration-protocol`, `runtime-instructions/work.taskflow-protocol`, `docs/product/spec/party-chat-v2-problem-party-model.md` |
| Future platform alignment (non-canonical reference) | `docs/process/framework-source-lineage-index.md` | `system-maps/framework.index`, `sidecar changelog plus Git history` |
| Current project-doc bootstrap map | `AGENTS.sidecar.md` | `docs/project-root-map.md`, `docs/product/index.md` |
| Current product canon map | `docs/project-root-map.md` | `AGENTS.sidecar.md`, `docs/product/index.md`, `docs/product/spec/current-spec-map.md` |
| Core bootstrap router | `system-maps/bootstrap.router-guide` | `AGENTS.md`, `agent-definitions/entry.orchestrator-entry`, `agent-definitions/entry.worker-entry`, `instruction-contracts/role.worker-thinking`, `system-maps/framework.readme` |
| Orchestrator entry contract | `agent-definitions/entry.orchestrator-entry` | `AGENTS.md`, `instruction-contracts/core.orchestration-protocol`, `command-instructions/routing.use-case-packs-protocol` |
| Step thinking algorithms | `instruction-contracts/overlay.step-thinking-protocol` | `references/algorithms.one-screen-reference`, `references/algorithms.quick-reference` |
| Session context continuity | `instruction-contracts/overlay.session-context-continuity-protocol` | `instruction-contracts/overlay.step-thinking-protocol`, `agent-definitions/entry.orchestrator-entry`, `instruction-contracts/bridge.instruction-activation-protocol` |
| Runtime orchestration | `instruction-contracts/core.orchestration-protocol` | `AGENTS.md`, `instruction-contracts/core.agent-system-protocol`, `runtime-instructions/core.capability-registry-protocol`, `runtime-instructions/core.context-governance-protocol`, `runtime-instructions/core.run-graph-protocol`, `command-instructions/routing.use-case-packs-protocol`, `system-maps/migration.runtime-transition-map`, `system-maps/framework.core-protocols-map` |
| Runtime-visible lawful-next / replan / parallelization loop | `instruction-contracts/core.orchestration-protocol` | `runtime-instructions/work.execution-priority-protocol`, `instruction-contracts/core.packet-decomposition-protocol`, `agent-definitions/entry.orchestrator-entry`, `docs/process/project-orchestrator-session-start-protocol.md` |
| Change-impact reconciliation | `runtime-instructions/work.change-impact-reconciliation-protocol` | `instruction-contracts/core.orchestration-protocol`, `command-instructions/planning.form-task-protocol`, `command-instructions/execution.implement-execution-protocol`, `command-instructions/operator.vida-spec-guide` |
| Pack completion gate | `runtime-instructions/work.pack-completion-gate-protocol` | `runtime-instructions/work.pack-handoff-protocol`, `runtime-instructions/work.execution-health-check-protocol`, `runtime-instructions/runtime.task-state-telemetry-protocol`, `runtime-instructions/work.taskflow-protocol` |
| Task-state SSOT and workflow telemetry | `runtime-instructions/runtime.task-state-telemetry-protocol` | `runtime-instructions/work.taskflow-protocol` |
| Framework wave starter (migration-only helper reference) | `system-maps/migration.runtime-transition-map` | `diagnostic-instructions/analysis.framework-self-analysis-protocol`, `runtime-instructions/work.taskflow-protocol`, `command-instructions/routing.use-case-packs-protocol` |
| Product proving-pack scaffolds | `diagnostic-instructions/evaluation.product-proving-pack-scaffold-contract` | `system-maps/runtime-family.taskflow-map`, `runtime-instructions/work.verification-lane-protocol`, `diagnostic-instructions/escalation.debug-escalation-protocol` |
| Framework wave task-state sync (migration-only helper reference) | `system-maps/migration.runtime-transition-map` | `.vida/state/framework-wave-task-sync.json`, `runtime-instructions/work.task-state-reconciliation-protocol`, `runtime-instructions/runtime.task-state-telemetry-protocol` |
| Shared reference catalog (non-runtime) | `docs/project-root-map.md` | `docs/product/index.md`, `docs/process/README.md`, `docs/project-memory/README.md` |
| Runtime operator tooling map | `system-maps/tooling.runtime-operator-tooling-map` | `command-instructions/operator.runtime-pipeline-guide`, `runtime-instructions/work.execution-health-check-protocol`, `runtime-instructions/work.command-execution-discipline-protocol`, `system-maps/runtime-family.taskflow-map` |
| Runtime pipeline operator guide | `command-instructions/operator.runtime-pipeline-guide` | `system-maps/tooling.runtime-operator-tooling-map`, `runtime-instructions/work.execution-health-check-protocol`, `runtime-instructions/work.command-execution-discipline-protocol` |
| Use-case pack routing | `command-instructions/routing.use-case-packs-protocol` | `system-maps/migration.runtime-transition-map`, `instruction-contracts/core.orchestration-protocol` |
| Pack wrapper migration note (non-canonical reference) | `command-instructions/migration.pack-wrapper-note` | `system-maps/migration.runtime-transition-map`, `instruction-contracts/core.orchestration-protocol`, `command-instructions/routing.use-case-packs-protocol` |
| Pack handoff boundaries | `runtime-instructions/work.pack-handoff-protocol` | `command-instructions/routing.use-case-packs-protocol`, `command-instructions/planning.form-task-protocol`, `command-instructions/execution.implement-execution-protocol`, `command-instructions/execution.bug-fix-protocol` |
| Bug-fix unified flow | `command-instructions/execution.bug-fix-protocol` | `command-instructions/operator.vida-bug-fix-guide`, `command-instructions/routing.use-case-packs-protocol` |
| Issue contract bridge | `runtime-instructions/bridge.issue-contract-protocol` | `command-instructions/execution.bug-fix-protocol`, `command-instructions/execution.implement-execution-protocol`, `system-maps/runtime-family.taskflow-map` |
| Web/internet validation | `runtime-instructions/work.web-validation-protocol` | `instruction-contracts/overlay.step-thinking-protocol#section-web-search`, `runtime-instructions/work.spec-contract-protocol` |
| Spec intake normalization | `runtime-instructions/work.spec-intake-protocol` | `runtime-instructions/work.spec-contract-protocol`, `runtime-instructions/bridge.issue-contract-protocol`, `command-instructions/planning.form-task-protocol` |
| Spec delta reconciliation | `runtime-instructions/work.spec-delta-protocol` | `runtime-instructions/bridge.issue-contract-protocol`, `command-instructions/execution.bug-fix-protocol`, `command-instructions/planning.form-task-protocol` |
| Spec contract (non-dev flows) | `runtime-instructions/work.spec-contract-protocol` | `system-maps/template.spec-contract-artifact-templates`, `command-instructions/operator.vida-spec-guide`, `system-maps/template.map` |
| Draft execution-spec helper | `system-maps/template.spec-contract-artifact-templates` | `runtime-instructions/work.spec-contract-protocol`, `command-instructions/planning.form-task-protocol`, `system-maps/template.map` |
| Form-task bridge (spec->dev) | `command-instructions/planning.form-task-protocol` | `command-instructions/operator.vida-form-task-guide`, `command-instructions/routing.use-case-packs-protocol` |
| Planning decomposition (Q-Gate -> TaskFlow plan) | `runtime-instructions/work.taskflow-protocol` | `command-instructions/planning.form-task-protocol`, `diagnostic-instructions/analysis.silent-framework-diagnosis-protocol`, `system-maps/runtime-family.taskflow-map` |
| Implement execution (dev) | `command-instructions/execution.implement-execution-protocol` | `command-instructions/operator.vida-implement-guide`, `command-instructions/routing.use-case-packs-protocol`, `command-instructions/routing.command-layer-protocol`, `instruction-contracts/bridge.instruction-activation-protocol`, `system-maps/bootstrap.orchestrator-boot-flow`, `system-maps/bootstrap.worker-boot-flow`, `agent-definitions/entry.worker-entry` |
| VIDA migration/source lineage (non-canonical reference) | `docs/process/framework-source-lineage-index.md` | `system-maps/framework.map`, `docs/product/spec/current-spec-map.md` |
| Agent-system activation/routing | `instruction-contracts/core.agent-system-protocol` | `system-maps/runtime-family.taskflow-map`, `vida.config.yaml`, `docs/process/agent-system-guide.md`, `agent-backends/matrix.agent-backends-matrix` |
| Agent-backend onboarding and recovery | `agent-backends/role.backend-lifecycle-protocol` | `system-maps/runtime-family.taskflow-map`, `vida.config.yaml`, `system-maps/template.map` |
| Worker entry contract | `agent-definitions/entry.worker-entry` | `AGENTS.md`, `instruction-contracts/lane.worker-dispatch-protocol`, `instruction-contracts/core.agent-system-protocol`, `instruction-contracts/role.worker-thinking` |
| Worker thinking subset | `instruction-contracts/role.worker-thinking` | `AGENTS.md`, `agent-definitions/entry.worker-entry`, `prompt-templates/worker.packet-templates` |
| Worker dispatch | `instruction-contracts/lane.worker-dispatch-protocol` | `agent-definitions/entry.worker-entry`, `instruction-contracts/role.worker-thinking`, `system-maps/template.map`, `system-maps/runtime-family.taskflow-map` |
| Runtime log policy | `runtime-instructions/runtime.log-policy` | `.gitignore` |
| TaskFlow overhead diagnostics | `taskflow-overhead-report.sh` | `runtime-instructions/work.taskflow-protocol`, `system-maps/runtime-family.taskflow-map` |
| Project operations docs | host-project operations doc declared by the active project overlay when overlay exists | `docs/process/README.md`, `docs/process/project-orchestrator-operating-protocol.md`, `docs/process/project-development-packet-template-protocol.md`, `docs/process/project-agent-prompt-stack-protocol.md`, `docs/process/project-boot-readiness-validation-protocol.md`, `scripts/` |
| Project environment notes | project environment doc declared by the active project docs map or overlay | `docs/project-root-map.md` |
| Skill catalog | `.agents/skills/` | - |
| GitHub operations | `system-maps/tooling.runtime-operator-tooling-map` | `gh` CLI help |

## Governance / Scope Routing

1. Required gates, approval behavior, publication/contribution rules, and lifecycle policies are owned by `system-maps/governance.map`.
2. Framework topology and scope boundaries are owned by `system-maps/framework.index` and `system-maps/framework.map`.
3. Concrete runtime command paths are owned by runtime-family maps and their runtime homes, not by this registry.
4. Domain-family explanations that start to look like owner law belong in `system-maps/framework.protocol-domains-map`, not in expanded row prose here.
5. Routine read posture should remain `capsule first, owner on demand`; this index must not re-expand compact-routing wins by becoming a broad-read surrogate.

## Maintenance Rule

When a protocol changes:

1. Update the canonical file first.
2. Update linked references in the same change.
3. Keep this index synchronized.
4. If a `vida/config/instructions/**` file is referenced as a canonical, mandatory, or full operational guide anywhere else in active canon, it must appear in this index before the change is considered complete.
5. If an active instruction artifact is intentionally excluded from this index, the excluding protocol must state that it is non-canonical reference material.
6. Use `vida docflow protocol-coverage-check --profile active-canon` as the bounded operational proof that canonical protocol-bearing artifacts remain indexed and activation-covered after changes.
7. Do not solve discoverability debt by copying owner-law detail into row prose; add or tighten the appropriate companion map, capsule, or startup bundle instead.

-----
artifact_path: config/system-maps/protocol.index
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/protocol.index.md
created_at: '2026-03-09T20:28:59+02:00'
updated_at: '2026-03-13T23:20:00+02:00'
changelog_ref: protocol.index.changelog.jsonl
