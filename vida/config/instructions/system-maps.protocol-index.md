# Protocol Index (Single Source Map)

Purpose: one entry point for protocol governance. This file maps canonical sources and required gates.

## Canonical Sources

| Domain | Canonical Source | Secondary/Reference |
|---|---|---|
| Framework topology map | `vida/config/instructions/system-maps.framework-map-protocol.md` | `vida/config/instructions/system-maps.protocol-index.md` |
| Command layer matrix | `vida/config/instructions/command-instructions.command-layer-protocol.md` | `vida/config/instructions/command-instructions.commands.md`, `vida/config/instructions/command-instructions.vida-*.md`, `vida/config/instructions/command-instructions.implement-execution-protocol.md`, `vida/config/instructions/command-instructions.bug-fix-protocol.md`, `vida/config/instructions/command-instructions.use-case-packs.md`, `vida/config/instructions/runtime-instructions.taskflow-protocol.md`, `vida/config/instructions/instruction-contracts.worker-dispatch-protocol.md`, `vida-command-audit.sh`, `render-worker-prompt.sh`, `vida/config/instructions/system-maps.framework-map-protocol.md` |
| Runtime script architecture | `vida/config/instructions/system-maps.script-runtime-architecture.md` | `vida/config/instructions/system-maps.framework-map-protocol.md`, `*.sh`, `*.py` |
| Runtime transition map | `vida/config/instructions/system-maps.runtime-transition-map.md` | `vida/config/instructions/system-maps.script-runtime-architecture.md`, `taskflow-v0/**`, `**` |
| Tooling/search guide | `vida/config/instructions/system-maps.tooling.md` | `vida/config/instructions/command-instructions.pipelines.md`, `AGENTS.md` |
| Framework change log | `sidecar changelog plus Git history` | `vida/config/instructions/system-maps.protocol-index.md` |
| Instruction activation and decomposition | `vida/config/instructions/instruction-contracts.instruction-activation-protocol.md` | `AGENTS.md`, `vida/config/instructions/agent-definitions.orchestrator-entry.md`, `vida/config/instructions/system-maps.protocol-index.md` |
| Documentation operation using only green documentation layers | `vida/config/instructions/instruction-contracts.documentation-operation-protocol.md` | `vida/config/instructions/instruction-contracts.instruction-activation-protocol.md`, `vida/config/instructions/agent-definitions.orchestrator-entry.md`, `docs/product/spec/project-documentation-system.md`, `docs/product/spec/canonical-documentation-and-inventory-layers.md`, `AGENTS.sidecar.md`, `codex-v0/codex.py` |
| Agent definition runtime contract | `vida/config/instructions/agent-definitions.protocol.md` | `docs/product/spec/instruction-artifact-model.md`, `docs/product/spec/skill-management-and-activation.md`, `vida/config/instructions/`, `vida/config/instructions/`, `vida/config/instructions/`, `vida/config/instructions/skills/` |
| Autonomous follow-through mode | `vida/config/instructions/instruction-contracts.autonomous-execution-protocol.md` | `vida/config/instructions/command-instructions.implement-execution-protocol.md`, `vida/config/instructions/runtime-instructions.taskflow-protocol.md`, `vida/config/instructions/runtime-instructions.beads-protocol.md`, `vida/config/instructions/instruction-contracts.agent-system-protocol.md` |
| Autonomous next-task selector helper | `autonomous-next-task.py` | `vida/config/instructions/instruction-contracts.autonomous-execution-protocol.md`, `vida/config/instructions/runtime-instructions.execution-priority-protocol.md` |
| Execution prioritization and reprioritization | `vida/config/instructions/runtime-instructions.execution-priority-protocol.md` | `vida/config/instructions/command-instructions.form-task-protocol.md`, `vida/config/instructions/runtime-instructions.taskflow-protocol.md`, `vida/config/instructions/command-instructions.implement-execution-protocol.md`, `vida/config/instructions/instruction-contracts.autonomous-execution-protocol.md` |
| Project overlay activation | `vida/config/instructions/runtime-instructions.project-overlay-protocol.md` | `vida.config.yaml`, `docs/framework/templates/vida.config.yaml.template`, `AGENTS.md`, `taskflow-v0 config ...`, `taskflow-v0 system ...` |
| Boot packet runtime artifact | `vida/config/instructions/runtime-instructions.boot-packet-protocol.md` | `taskflow-v0 boot ...`, `AGENTS.md`, `vida/config/instructions/agent-definitions.orchestrator-entry.md`, `vida/config/instructions/agent-definitions.worker-entry.md` |
| Project bootstrap/self-reproduction | `vida/config/instructions/command-instructions.project-bootstrap-protocol.md` | `taskflow-v0 boot ...`, `docs/framework/templates/vida.config.yaml.template`, `vida.config.yaml` |
| VIDA framework self-analysis | `vida/config/instructions/diagnostic-instructions.framework-self-analysis-protocol.md` | `vida/config/instructions/system-maps.framework-map-protocol.md`, `vida/config/instructions/diagnostic-instructions.self-reflection-protocol.md` |
| Silent framework diagnosis | `vida/config/instructions/diagnostic-instructions.silent-framework-diagnosis-protocol.md` | `vida-silent-diagnosis.py`, `vida.config.yaml`, `vida/config/instructions/diagnostic-instructions.framework-self-analysis-protocol.md`, `vida/config/instructions/runtime-instructions.taskflow-protocol.md` |
| Human approval lifecycle | `vida/config/instructions/runtime-instructions.human-approval-protocol.md` | `human-approval-gate.py`, `worker-dispatch.py`, `vida/config/instructions/instruction-contracts.agent-system-protocol.md`, `vida/config/instructions/command-instructions.implement-execution-protocol.md` |
| Framework memory ledger | `vida/config/instructions/runtime-instructions.framework-memory-protocol.md` | `framework-memory.py`, `vida-silent-diagnosis.py`, `vida/config/instructions/diagnostic-instructions.silent-framework-diagnosis-protocol.md` |
| DB-first runtime ownership | `docs/framework/plans/vida-0.3-db-first-runtime-spec.md` | `docs/framework/plans/vida-0.3-storage-kernel-spec.md`, `docs/framework/plans/vida-0.3-instruction-memory-and-sidecar-spec.md`, `vida/config/instructions/runtime-instructions.export-protocol.md` |
| Export surfaces | `vida/config/instructions/runtime-instructions.export-protocol.md` | `docs/framework/plans/vida-0.3-db-first-runtime-spec.md`, `docs/framework/plans/vida-0.3-migration-kernel-spec.md` |
| Spec sync after autonomous changes | `vida/config/instructions/runtime-instructions.spec-sync-protocol.md` | `vida/config/instructions/instruction-contracts.autonomous-execution-protocol.md`, `vida/config/instructions/command-instructions.implement-execution-protocol.md` |
| Spec freshness and newer-decision precedence | `vida/config/instructions/runtime-instructions.spec-freshness-protocol.md` | `vida/config/instructions/runtime-instructions.spec-sync-protocol.md`, `vida/config/instructions/runtime-instructions.task-approval-loop-protocol.md` |
| Protocol self-diagnosis and runtime drift checks | `vida/config/instructions/diagnostic-instructions.protocol-self-diagnosis-protocol.md` | `vida/config/instructions/runtime-instructions.taskflow-protocol.md`, `vida/config/instructions/instruction-contracts.autonomous-execution-protocol.md`, `vida/config/instructions/instruction-contracts.agent-system-protocol.md`, `vida/config/instructions/runtime-instructions.spec-sync-protocol.md`, `vida/config/instructions/diagnostic-instructions.silent-framework-diagnosis-protocol.md` |
| Debug escalation after repeated errors | `vida/config/instructions/diagnostic-instructions.debug-escalation-protocol.md` | `vida/config/instructions/instruction-contracts.autonomous-execution-protocol.md`, `vida/config/instructions/runtime-instructions.spec-sync-protocol.md` |
| External-agent and web escalation for repeated technical failures | `vida/config/instructions/diagnostic-instructions.debug-escalation-protocol.md` | `vida/config/instructions/instruction-contracts.agent-system-protocol.md`, `vida/config/instructions/runtime-instructions.taskflow-protocol.md`, `vida/config/instructions/instruction-contracts.autonomous-execution-protocol.md` |
| Library evaluation and live alternatives matrix | `vida/config/instructions/diagnostic-instructions.library-evaluation-protocol.md` | `vida/config/instructions/diagnostic-instructions.debug-escalation-protocol.md`, `vida/config/instructions/runtime-instructions.spec-sync-protocol.md` |
| User approval loop between tasks | `vida/config/instructions/runtime-instructions.task-approval-loop-protocol.md` | `vida/config/instructions/instruction-contracts.autonomous-execution-protocol.md`, `vida/config/instructions/runtime-instructions.human-approval-protocol.md` |
| Document lifecycle and freshness | `vida/config/instructions/runtime-instructions.document-lifecycle-protocol.md` | `doc-lifecycle.py`, `vida/config/instructions/runtime-instructions.project-overlay-protocol.md`, `vida/config/instructions/diagnostic-instructions.silent-framework-diagnosis-protocol.md` |
| Context governance ledger | `vida/config/instructions/runtime-instructions.context-governance-protocol.md` | `context-governance.py`, `worker-dispatch.py`, `framework-operator-status.py`, `future.md` |
| Durable run-graph ledger | `vida/config/instructions/runtime-instructions.run-graph-protocol.md` | `run-graph.py`, `future.md`, `worker-dispatch.py` |
| Local trace grading and datasets | `vida/config/instructions/runtime-instructions.trace-eval-protocol.md` | `trace-eval.py`, `eval-pack.sh`, `worker-eval-pack.py`, `future.md` |
| Typed capability registry | `vida/config/instructions/runtime-instructions.capability-registry-protocol.md` | `taskflow-v0 registry ...`, `vida.config.yaml` |
| Task-state reconciliation | `vida/config/instructions/runtime-instructions.task-state-reconciliation-protocol.md` | `task-state-reconcile.py`, `vida/config/instructions/runtime-instructions.taskflow-protocol.md`, `vida/config/instructions/runtime-instructions.beads-protocol.md`, `quality-health-check.sh` |
| Problem-party discussion | `vida/config/instructions/runtime-instructions.problem-party-protocol.md` | `problem-party.py`, `vida/config/instructions/instruction-contracts.orchestration-protocol.md`, `vida/config/instructions/runtime-instructions.taskflow-protocol.md` |
| Future platform alignment (non-canonical reference) | `future.md` | `vida/config/instructions/system-maps.protocol-index.md`, `sidecar changelog plus Git history` |
| Current product canon map | `docs/product/spec/current-spec-map.md` | `docs/product/index.md`, `vida/config/**` |
| Core bootstrap router | `AGENTS.md` | `vida/config/instructions/agent-definitions.orchestrator-entry.md`, `vida/config/instructions/agent-definitions.worker-entry.md`, `vida/config/instructions/instruction-contracts.worker-thinking.md`, `vida/config/instructions/system-maps.framework-readme.md`, `docs/README.md` |
| Orchestrator entry contract | `vida/config/instructions/agent-definitions.orchestrator-entry.md` | `AGENTS.md`, `vida/config/instructions/instruction-contracts.orchestration-protocol.md`, `vida/config/instructions/command-instructions.use-case-packs.md` |
| Thinking algorithms | `vida/config/instructions/instruction-contracts.thinking-protocol.md` | `vida/config/instructions/references.algorithms-one-screen.md`, `vida/config/instructions/references.algorithms-quick-reference.md` |
| Runtime orchestration | `vida/config/instructions/instruction-contracts.orchestration-protocol.md` | `AGENTS.md`, `vida/config/instructions/command-instructions.use-case-packs.md`, `vida/config/instructions/system-maps.runtime-transition-map.md` |
| Change-impact reconciliation (absorbed cascade) | `vida/config/instructions/command-instructions.use-case-packs.md` | `vida/config/instructions/command-instructions.form-task-protocol.md`, `vida/config/instructions/command-instructions.implement-execution-protocol.md`, `vida/config/instructions/command-instructions.vida-spec.md` |
| Task state (SSOT) | `vida/config/instructions/runtime-instructions.beads-protocol.md` | `vida/config/instructions/runtime-instructions.taskflow-protocol.md` |
| Framework wave starter | `vida/config/instructions/system-maps.runtime-transition-map.md` | `vida/config/instructions/diagnostic-instructions.framework-self-analysis-protocol.md`, `vida/config/instructions/runtime-instructions.taskflow-protocol.md`, `vida/config/instructions/command-instructions.use-case-packs.md` |
| Product/framework proving packs | `vida/config/instructions/diagnostic-instructions.product-proving-packs-protocol.md` | `proving-pack.py` |
| Framework wave task-state sync | `vida/config/instructions/system-maps.runtime-transition-map.md` | `.vida/state/framework-wave-task-sync.json`, `vida/config/instructions/runtime-instructions.taskflow-protocol.md`, `vida/config/instructions/runtime-instructions.beads-protocol.md` |
| Shared reference catalog (non-runtime) | `docs/**` | `vida/config/instructions/runtime-instructions.beads-protocol.md` |
| Execution pipelines | `vida/config/instructions/command-instructions.pipelines.md` | `quality-health-check.sh`, `framework-boundary-check.sh` |
| Use-case routing | `vida/config/instructions/command-instructions.use-case-packs.md` | `vida/config/instructions/system-maps.runtime-transition-map.md`, `vida/config/instructions/instruction-contracts.orchestration-protocol.md` |
| Bug-fix unified flow | `vida/config/instructions/command-instructions.bug-fix-protocol.md` | `vida/config/instructions/command-instructions.vida-bug-fix.md`, `vida/config/instructions/command-instructions.use-case-packs.md` |
| Issue-as-contract bridge | `vida/config/instructions/runtime-instructions.issue-contract-protocol.md` | `vida/config/instructions/command-instructions.bug-fix-protocol.md`, `vida/config/instructions/command-instructions.implement-execution-protocol.md`, `worker-dispatch.py`, `execution-auth-gate.py` |
| Web/internet validation | `vida/config/instructions/runtime-instructions.web-validation-protocol.md` | `vida/config/instructions/instruction-contracts.thinking-protocol.md#section-web-search`, `vida/config/instructions/runtime-instructions.spec-contract-protocol.md` |
| Spec intake normalization | `vida/config/instructions/runtime-instructions.spec-intake-protocol.md` | `spec-intake.py`, `vida/config/instructions/runtime-instructions.spec-contract-protocol.md`, `vida/config/instructions/runtime-instructions.issue-contract-protocol.md`, `vida/config/instructions/command-instructions.form-task-protocol.md` |
| Spec delta reconciliation | `vida/config/instructions/runtime-instructions.spec-delta-protocol.md` | `spec-delta.py`, `vida/config/instructions/runtime-instructions.issue-contract-protocol.md`, `vida/config/instructions/command-instructions.bug-fix-protocol.md`, `vida/config/instructions/command-instructions.form-task-protocol.md` |
| Spec contract (non-dev flows) | `vida/config/instructions/runtime-instructions.spec-contract-protocol.md` | `vida/config/instructions/system-maps.spec-contract-artifacts.md`, `vida/config/instructions/command-instructions.vida-spec.md`, `skill-discovery.py`, `scp-confidence.py` |
| Draft execution-spec helper | `vida/config/instructions/system-maps.spec-contract-artifacts.md` | `draft-execution-spec.py`, `vida/config/instructions/runtime-instructions.spec-contract-protocol.md`, `vida/config/instructions/command-instructions.form-task-protocol.md` |
| Form-task bridge (spec->dev) | `vida/config/instructions/command-instructions.form-task-protocol.md` | `vida/config/instructions/command-instructions.vida-form-task.md`, `vida/config/instructions/command-instructions.use-case-packs.md` |
| Planning decomposition (Q-Gate -> TaskFlow plan) | `vida/config/instructions/runtime-instructions.taskflow-protocol.md` | `vida/config/instructions/command-instructions.form-task-protocol.md`, `vida/config/instructions/diagnostic-instructions.silent-framework-diagnosis-protocol.md`, `todo-plan-validate.sh`, `stateful-sequence-check.sh` |
| Implement execution (dev) | `vida/config/instructions/command-instructions.implement-execution-protocol.md` | `vida/config/instructions/command-instructions.vida-implement.md`, `vida/config/instructions/command-instructions.use-case-packs.md`, `vida/config/instructions/command-instructions.command-layer-protocol.md` |
| VIDA migration decisions | `docs/research/vida-framework/vida-migration-registry.md` | `docs/research/vida-framework/**`, `legacy helper surfaces*` |
| Agent-system activation/routing | `vida/config/instructions/instruction-contracts.agent-system-protocol.md` | `vida/config/instructions/instruction-contracts.agent-system-protocol.md`, `taskflow-v0 system ...`, `taskflow-v0 registry ...`, `vida.config.yaml`, `docs/process/agent-system.md`, `vida/config/instructions/agent-backends.matrix.md` |
| Agent-backend onboarding and recovery | `vida/config/instructions/agent-backends.lifecycle-protocol.md` | `vida/config/instructions/agent-backends.lifecycle-protocol.md`, `taskflow-v0 system ...`, `taskflow-v0 prepare-execution ...`, `vida.config.yaml`, `docs/framework/templates/vida.config.yaml.template` |
| Worker entry contract | `vida/config/instructions/agent-definitions.worker-entry.md` | `AGENTS.md`, `vida/config/instructions/instruction-contracts.worker-dispatch-protocol.md`, `vida/config/instructions/instruction-contracts.agent-system-protocol.md`, `vida/config/instructions/instruction-contracts.worker-thinking.md` |
| Worker thinking subset | `vida/config/instructions/instruction-contracts.worker-thinking.md` | `AGENTS.md`, `vida/config/instructions/agent-definitions.worker-entry.md`, `vida/config/instructions/prompt-templates.worker-packet-templates.md` |
| Worker dispatch | `vida/config/instructions/instruction-contracts.worker-dispatch-protocol.md` | `vida/config/instructions/instruction-contracts.worker-dispatch-protocol.md`, `vida/config/instructions/agent-definitions.worker-entry.md`, `vida/config/instructions/instruction-contracts.worker-thinking.md`, `vida/config/instructions/prompt-templates.worker-packet-templates.md`, `taskflow-v0 worker ...` |
| Runtime log policy | `vida/config/instructions/runtime-instructions.log-policy.md` | `.gitignore` |
| TaskFlow overhead diagnostics | `taskflow-overhead-report.sh` | `vida/config/instructions/runtime-instructions.taskflow-protocol.md` |
| Project operations (build/run/observability/live checks) | host-project operations doc declared by project overlay when overlay exists; otherwise framework-owned wrappers under `*` | host-project scripts and runbooks |
| Environment/auth notes | `docs/environments.md` | - |
| Skill catalog | `.agents/skills/` | - |
| GitHub operations | `vida/config/instructions/command-instructions.pipelines.md` | `gh` CLI help |

## Mandatory Gates

1. Before close/handoff on transitioned runtime slices: run the relevant `taskflow-v0` tests or build proof from `vida/config/instructions/system-maps.runtime-transition-map.md`; legacy-only health wrappers remain migration-only until replaced.
2. Before `finish`: strict execution-log verify must pass.
3. For server/API assumptions: live request validation is required.
4. For external assumptions (API/package/platform/security/migration): WVP evidence is required (`vida/config/instructions/runtime-instructions.web-validation-protocol.md`).
5. For topology/refactor changes: update `vida/config/instructions/system-maps.framework-map-protocol.md` in the same change.
6. For worker entry-contract changes: keep `AGENTS.md`, `vida/config/instructions/agent-definitions.orchestrator-entry.md`, `vida/config/instructions/agent-definitions.worker-entry.md`, and `vida/config/instructions/instruction-contracts.worker-thinking.md` synchronized in the same change.
7. Before pack/command/TaskFlow engagement, run request-intent classification and skip task machinery for `answer_only` unless the user explicitly asks for an artifact or mutation.
8. Broad reads of `.vida/logs`, `.vida/state`, and `.beads` are forbidden by default; use exact-key, specific-file, short-window reads unless the active lane contract explicitly escalates.

## Execution Command Path

```bash
nim c taskflow-v0/src/vida.nim
nim c -r taskflow-v0/tests/test_boot_profile.nim
nim c -r taskflow-v0/tests/test_worker_packet.nim
nim c -r taskflow-v0/tests/test_kernel_runtime.nim
```

## Scope Rule: `vida/config/instructions/` vs `docs/`

1. `vida/config/instructions/` = active framework instruction canon and system maps.
2. `docs/framework/plans/` = active strategic and execution-spec program layer.
3. `docs/framework/research/` = active research layer.
4. `docs/product/spec/` = current VIDA product canon.
5. `docs/product/research/` = product research and migration crosswalk inputs.
6. `docs/` = active project/domain documentation.
7. `` = historical framework plans, research, and migration evidence.
8. `docs/process/` = project operational runbooks.
9. `scripts/` = executable project operations.
10. If project guidance becomes runtime protocol, move only the protocol portion to `vida/config/instructions/*.md`; keep project commands in `docs/process/` and `scripts/`.

## Maintenance Rule

When a protocol changes:

1. Update the canonical file first.
2. Update linked references in the same change.
3. Keep this index synchronized.
4. If a `vida/config/instructions/*.md` file is referenced as a canonical, mandatory, or full operational guide anywhere else in active canon, it must appear in this index before the change is considered complete.
5. If an active instruction artifact is intentionally excluded from this index, the excluding protocol must state that it is non-canonical reference material.
6. Use `python3 codex-v0/codex.py protocol-coverage-check --profile active-canon` as the bounded operational proof that canonical protocol-bearing artifacts remain indexed and activation-covered after changes.

-----
artifact_path: config/system-maps/protocol.index
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps.protocol-index.md
created_at: '2026-03-09T20:28:59+02:00'
updated_at: '2026-03-10T04:07:10+02:00'
changelog_ref: system-maps.protocol-index.changelog.jsonl
