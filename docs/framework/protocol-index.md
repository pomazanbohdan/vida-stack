# Protocol Index (Single Source Map)

Purpose: one entry point for protocol governance. This file maps canonical sources and required gates.

## Canonical Sources

| Domain | Canonical Source | Secondary/Reference |
|---|---|---|
| Framework topology map | `docs/framework/framework-map-protocol.md` | `docs/framework/protocol-index.md` |
| Command layer matrix | `docs/framework/command-layer-protocol.md` | `docs/framework/commands.md`, `docs/framework/commands/vida-*.md`, `docs/framework/implement-execution-protocol.md`, `docs/framework/bug-fix-protocol.md`, `docs/framework/use-case-packs.md`, `docs/framework/todo-protocol.md`, `docs/framework/subagents.md`, `docs/framework/history/_vida-source/scripts/vida-command-audit.sh`, `docs/framework/history/_vida-source/scripts/render-subagent-prompt.sh`, `docs/framework/framework-map-protocol.md` |
| Runtime script architecture | `docs/framework/script-runtime-architecture.md` | `docs/framework/framework-map-protocol.md`, `docs/framework/history/_vida-source/scripts/*.sh`, `docs/framework/history/_vida-source/scripts/*.py` |
| Runtime transition map | `docs/framework/runtime-transition-map.md` | `docs/framework/script-runtime-architecture.md`, `vida-v0/**`, `docs/framework/history/_vida-source/scripts/**` |
| Tooling/search guide | `docs/framework/tooling.md` | `docs/framework/pipelines.md`, `AGENTS.md` |
| Framework change log | `docs/framework/history/CHANGELOG.md` | `docs/framework/protocol-index.md` |
| Instruction activation and decomposition | `docs/framework/instruction-activation-protocol.md` | `AGENTS.md`, `docs/framework/ORCHESTRATOR-ENTRY.MD`, `docs/framework/protocol-index.md` |
| Agent definition runtime contract | `docs/framework/agent-definition-protocol.md` | `docs/product/spec/instruction-artifact-model.md`, `docs/product/spec/skill-management-and-activation.md`, `vida/config/instructions/agent_definitions/`, `vida/config/instructions/instruction_contracts/`, `vida/config/instructions/prompt_templates/`, `vida/config/instructions/skills/` |
| Autonomous follow-through mode | `docs/framework/autonomous-execution-protocol.md` | `docs/framework/implement-execution-protocol.md`, `docs/framework/todo-protocol.md`, `docs/framework/beads-protocol.md`, `docs/framework/subagent-system-protocol.md` |
| Autonomous next-task selector helper | `docs/framework/history/_vida-source/scripts/autonomous-next-task.py` | `docs/framework/autonomous-execution-protocol.md`, `docs/framework/execution-priority-protocol.md` |
| Execution prioritization and reprioritization | `docs/framework/execution-priority-protocol.md` | `docs/framework/form-task-protocol.md`, `docs/framework/todo-protocol.md`, `docs/framework/implement-execution-protocol.md`, `docs/framework/autonomous-execution-protocol.md` |
| Project overlay activation | `docs/framework/project-overlay-protocol.md` | `vida.config.yaml`, `docs/framework/templates/vida.config.yaml.template`, `AGENTS.md`, `vida-v0 config ...`, `vida-v0 system ...` |
| Boot packet runtime artifact | `docs/framework/boot-packet-protocol.md` | `vida-v0 boot ...`, `AGENTS.md`, `docs/framework/ORCHESTRATOR-ENTRY.MD`, `docs/framework/SUBAGENT-ENTRY.MD` |
| Project bootstrap/self-reproduction | `docs/framework/project-bootstrap-protocol.md` | `vida-v0 boot ...`, `docs/framework/templates/vida.config.yaml.template`, `vida.config.yaml` |
| VIDA framework self-analysis | `docs/framework/framework-self-analysis-protocol.md` | `docs/framework/framework-map-protocol.md`, `docs/framework/self-reflection-protocol.md` |
| Silent framework diagnosis | `docs/framework/silent-framework-diagnosis-protocol.md` | `docs/framework/history/_vida-source/scripts/vida-silent-diagnosis.py`, `vida.config.yaml`, `docs/framework/framework-self-analysis-protocol.md`, `docs/framework/todo-protocol.md` |
| Human approval lifecycle | `docs/framework/human-approval-protocol.md` | `docs/framework/history/_vida-source/scripts/human-approval-gate.py`, `docs/framework/history/_vida-source/scripts/subagent-dispatch.py`, `docs/framework/subagent-system-protocol.md`, `docs/framework/implement-execution-protocol.md` |
| Framework memory ledger | `docs/framework/framework-memory-protocol.md` | `docs/framework/history/_vida-source/scripts/framework-memory.py`, `docs/framework/history/_vida-source/scripts/vida-silent-diagnosis.py`, `docs/framework/silent-framework-diagnosis-protocol.md` |
| DB-first runtime ownership | `docs/framework/history/plans/2026-03-08-vida-0.3-db-first-runtime-spec.md` | `docs/framework/history/plans/2026-03-08-vida-0.3-storage-kernel-spec.md`, `docs/framework/history/plans/2026-03-08-vida-0.3-instruction-memory-and-sidecar-spec.md`, `docs/framework/export-protocol.md` |
| Export surfaces | `docs/framework/export-protocol.md` | `docs/framework/history/plans/2026-03-08-vida-0.3-db-first-runtime-spec.md`, `docs/framework/history/plans/2026-03-08-vida-0.3-migration-kernel-spec.md` |
| Spec sync after autonomous changes | `docs/framework/spec-sync-protocol.md` | `docs/framework/autonomous-execution-protocol.md`, `docs/framework/implement-execution-protocol.md` |
| Spec freshness and newer-decision precedence | `docs/framework/spec-freshness-protocol.md` | `docs/framework/spec-sync-protocol.md`, `docs/framework/task-approval-loop-protocol.md` |
| Protocol self-diagnosis and runtime drift checks | `docs/framework/protocol-self-diagnosis-protocol.md` | `docs/framework/todo-protocol.md`, `docs/framework/autonomous-execution-protocol.md`, `docs/framework/subagent-system-protocol.md`, `docs/framework/spec-sync-protocol.md`, `docs/framework/silent-framework-diagnosis-protocol.md` |
| Debug escalation after repeated errors | `docs/framework/debug-escalation-protocol.md` | `docs/framework/autonomous-execution-protocol.md`, `docs/framework/spec-sync-protocol.md` |
| External-agent and web escalation for repeated technical failures | `docs/framework/debug-escalation-protocol.md` | `docs/framework/subagent-system-protocol.md`, `docs/framework/todo-protocol.md`, `docs/framework/autonomous-execution-protocol.md` |
| Library evaluation and live alternatives matrix | `docs/framework/library-evaluation-protocol.md` | `docs/framework/debug-escalation-protocol.md`, `docs/framework/spec-sync-protocol.md` |
| User approval loop between tasks | `docs/framework/task-approval-loop-protocol.md` | `docs/framework/autonomous-execution-protocol.md`, `docs/framework/human-approval-protocol.md` |
| Document lifecycle and freshness | `docs/framework/document-lifecycle-protocol.md` | `docs/framework/history/_vida-source/scripts/doc-lifecycle.py`, `docs/framework/project-overlay-protocol.md`, `docs/framework/silent-framework-diagnosis-protocol.md` |
| Context governance ledger | `docs/framework/context-governance-protocol.md` | `docs/framework/history/_vida-source/scripts/context-governance.py`, `docs/framework/history/_vida-source/scripts/subagent-dispatch.py`, `docs/framework/history/_vida-source/scripts/framework-operator-status.py`, `docs/framework/history/future.md` |
| Durable run-graph ledger | `docs/framework/run-graph-protocol.md` | `docs/framework/history/_vida-source/scripts/run-graph.py`, `docs/framework/history/future.md`, `docs/framework/history/_vida-source/scripts/subagent-dispatch.py` |
| Local trace grading and datasets | `docs/framework/trace-eval-protocol.md` | `docs/framework/history/_vida-source/scripts/trace-eval.py`, `docs/framework/history/_vida-source/scripts/eval-pack.sh`, `docs/framework/history/_vida-source/scripts/subagent-eval-pack.py`, `docs/framework/history/future.md` |
| Typed capability registry | `docs/framework/capability-registry-protocol.md` | `vida-v0 registry ...`, `vida.config.yaml` |
| Task-state reconciliation | `docs/framework/task-state-reconciliation-protocol.md` | `docs/framework/history/_vida-source/scripts/task-state-reconcile.py`, `docs/framework/todo-protocol.md`, `docs/framework/beads-protocol.md`, `docs/framework/history/_vida-source/scripts/quality-health-check.sh` |
| Problem-party discussion | `docs/framework/problem-party-protocol.md` | `docs/framework/history/_vida-source/scripts/problem-party.py`, `docs/framework/orchestration-protocol.md`, `docs/framework/todo-protocol.md` |
| Future platform alignment (non-canonical reference) | `docs/framework/history/future.md` | `docs/framework/protocol-index.md`, `docs/framework/history/CHANGELOG.md` |
| Current product canon map | `docs/product/spec/current-spec-map.md` | `docs/product/index.md`, `vida/config/**` |
| Core bootstrap router | `AGENTS.md` | `docs/framework/ORCHESTRATOR-ENTRY.MD`, `docs/framework/SUBAGENT-ENTRY.MD`, `docs/framework/SUBAGENT-THINKING.MD`, `docs/framework/README.md`, `docs/README.md` |
| Orchestrator entry contract | `docs/framework/ORCHESTRATOR-ENTRY.MD` | `AGENTS.md`, `docs/framework/orchestration-protocol.md`, `docs/framework/use-case-packs.md` |
| Thinking algorithms | `docs/framework/thinking-protocol.md` | `docs/framework/algorithms-one-screen.md`, `docs/framework/algorithms-quick-reference.md` |
| Runtime orchestration | `docs/framework/orchestration-protocol.md` | `AGENTS.md`, `docs/framework/use-case-packs.md`, `docs/framework/runtime-transition-map.md` |
| Change-impact reconciliation (absorbed cascade) | `docs/framework/use-case-packs.md` | `docs/framework/form-task-protocol.md`, `docs/framework/implement-execution-protocol.md`, `docs/framework/commands/vida-spec.md` |
| Task state (SSOT) | `docs/framework/beads-protocol.md` | `docs/framework/todo-protocol.md` |
| Framework wave starter | `docs/framework/runtime-transition-map.md` | `docs/framework/framework-self-analysis-protocol.md`, `docs/framework/todo-protocol.md`, `docs/framework/use-case-packs.md` |
| Product/framework proving packs | `docs/framework/product-proving-packs-protocol.md` | `docs/framework/history/_vida-source/scripts/proving-pack.py` |
| Framework wave task-state sync | `docs/framework/runtime-transition-map.md` | `.vida/state/framework-wave-task-sync.json`, `docs/framework/todo-protocol.md`, `docs/framework/beads-protocol.md` |
| Shared reference catalog (non-runtime) | `docs/**` | `docs/framework/beads-protocol.md` |
| Execution pipelines | `docs/framework/pipelines.md` | `docs/framework/history/_vida-source/scripts/quality-health-check.sh`, `docs/framework/history/_vida-source/scripts/framework-boundary-check.sh` |
| Use-case routing | `docs/framework/use-case-packs.md` | `docs/framework/runtime-transition-map.md`, `docs/framework/orchestration-protocol.md` |
| Bug-fix unified flow | `docs/framework/bug-fix-protocol.md` | `docs/framework/commands/vida-bug-fix.md`, `docs/framework/use-case-packs.md` |
| Issue-as-contract bridge | `docs/framework/issue-contract-protocol.md` | `docs/framework/bug-fix-protocol.md`, `docs/framework/implement-execution-protocol.md`, `docs/framework/history/_vida-source/scripts/subagent-dispatch.py`, `docs/framework/history/_vida-source/scripts/execution-auth-gate.py` |
| Web/internet validation | `docs/framework/web-validation-protocol.md` | `docs/framework/thinking-protocol.md#section-web-search`, `docs/framework/spec-contract-protocol.md` |
| Spec intake normalization | `docs/framework/spec-intake-protocol.md` | `docs/framework/history/_vida-source/scripts/spec-intake.py`, `docs/framework/spec-contract-protocol.md`, `docs/framework/issue-contract-protocol.md`, `docs/framework/form-task-protocol.md` |
| Spec delta reconciliation | `docs/framework/spec-delta-protocol.md` | `docs/framework/history/_vida-source/scripts/spec-delta.py`, `docs/framework/issue-contract-protocol.md`, `docs/framework/bug-fix-protocol.md`, `docs/framework/form-task-protocol.md` |
| Spec contract (non-dev flows) | `docs/framework/spec-contract-protocol.md` | `docs/framework/spec-contract-artifacts.md`, `docs/framework/commands/vida-spec.md`, `docs/framework/history/_vida-source/scripts/skill-discovery.py`, `docs/framework/history/_vida-source/scripts/scp-confidence.py` |
| Draft execution-spec helper | `docs/framework/spec-contract-artifacts.md` | `docs/framework/history/_vida-source/scripts/draft-execution-spec.py`, `docs/framework/spec-contract-protocol.md`, `docs/framework/form-task-protocol.md` |
| Form-task bridge (spec->dev) | `docs/framework/form-task-protocol.md` | `docs/framework/commands/vida-form-task.md`, `docs/framework/use-case-packs.md` |
| Planning decomposition (Q-Gate -> TODO plan) | `docs/framework/todo-protocol.md` | `docs/framework/form-task-protocol.md`, `docs/framework/silent-framework-diagnosis-protocol.md`, `docs/framework/history/_vida-source/scripts/todo-plan-validate.sh`, `docs/framework/history/_vida-source/scripts/stateful-sequence-check.sh` |
| Implement execution (dev) | `docs/framework/implement-execution-protocol.md` | `docs/framework/commands/vida-implement.md`, `docs/framework/use-case-packs.md`, `docs/framework/command-layer-protocol.md` |
| VIDA migration decisions | `docs/research/vida-framework/vida-migration-registry.md` | `docs/research/vida-framework/**`, `docs/framework/history/_vida-source/**` |
| Subagent system activation/routing | `docs/framework/subagent-system-protocol.md` | `vida-v0 system ...`, `vida-v0 registry ...`, `vida.config.yaml`, `docs/process/agent-system.md`, `docs/framework/DEV-AGENTS-MATRIX.md` |
| Subagent onboarding and recovery | `docs/framework/subagent-onboarding-protocol.md` | `vida-v0 system ...`, `vida-v0 prepare-execution ...`, `vida.config.yaml`, `docs/framework/templates/vida.config.yaml.template` |
| Worker entry contract | `docs/framework/SUBAGENT-ENTRY.MD` | `AGENTS.md`, `docs/framework/subagents.md`, `docs/framework/subagent-system-protocol.md`, `docs/framework/SUBAGENT-THINKING.MD` |
| Worker thinking subset | `docs/framework/SUBAGENT-THINKING.MD` | `AGENTS.md`, `docs/framework/SUBAGENT-ENTRY.MD`, `docs/framework/subagent-prompt-templates.md` |
| Subagent dispatch | `docs/framework/subagents.md` | `docs/framework/SUBAGENT-ENTRY.MD`, `docs/framework/SUBAGENT-THINKING.MD`, `docs/framework/subagent-prompt-templates.md`, `vida-v0 worker ...` |
| Runtime log policy | `docs/framework/log-policy.md` | `.gitignore` |
| TODO overhead diagnostics | `docs/framework/history/_vida-source/scripts/todo-overhead-report.sh` | `docs/framework/todo-protocol.md` |
| Project operations (build/run/observability/live checks) | host-project operations doc declared by project overlay when overlay exists; otherwise framework-owned wrappers under `docs/framework/history/_vida-source/scripts/*` | host-project scripts and runbooks |
| Environment/auth notes | `docs/environments.md` | - |
| Skill catalog | `.agents/skills/` | - |
| GitHub operations | `docs/framework/pipelines.md` | `gh` CLI help |

## Mandatory Gates

1. Before close/handoff on transitioned runtime slices: run the relevant `vida-v0` tests or build proof from `docs/framework/runtime-transition-map.md`; legacy-only health wrappers remain migration-only until replaced.
2. Before `finish`: strict execution-log verify must pass.
3. For server/API assumptions: live request validation is required.
4. For external assumptions (API/package/platform/security/migration): WVP evidence is required (`docs/framework/web-validation-protocol.md`).
5. For topology/refactor changes: update `docs/framework/framework-map-protocol.md` in the same change.
6. For entry-contract changes: keep `AGENTS.md`, `docs/framework/ORCHESTRATOR-ENTRY.MD`, `docs/framework/SUBAGENT-ENTRY.MD`, and `docs/framework/SUBAGENT-THINKING.MD` synchronized in the same change.
7. Before pack/command/TODO engagement, run request-intent classification and skip task machinery for `answer_only` unless the user explicitly asks for an artifact or mutation.
8. Broad reads of `.vida/logs`, `.vida/state`, and `.beads` are forbidden by default; use exact-key, specific-file, short-window reads unless the active lane contract explicitly escalates.

## Execution Command Path

```bash
nim c vida-v0/src/vida.nim
nim c -r vida-v0/tests/test_boot_profile.nim
nim c -r vida-v0/tests/test_worker_packet.nim
nim c -r vida-v0/tests/test_kernel_runtime.nim
```

## Scope Rule: `docs/framework/` vs `docs/`

1. `docs/framework/` = framework runtime policy and protocol source.
2. `docs/product/spec/` = current VIDA product canon.
3. `docs/product/research/` = product research and migration crosswalk inputs.
4. `docs/` = active project/domain documentation.
5. `docs/framework/history/` = historical framework plans, research, and migration evidence.
6. `docs/process/` = project operational runbooks.
7. `scripts/` = executable project operations.
8. If project guidance becomes runtime protocol, move only the protocol portion to `docs/framework/`; keep project commands in `docs/process/` and `scripts/`.

## Maintenance Rule

When a protocol changes:

1. Update the canonical file first.
2. Update linked references in the same change.
3. Keep this index synchronized.
4. If a framework-owned `docs/framework/*.md` file is referenced as a canonical, mandatory, or full operational guide anywhere else in framework docs, it must appear in this index before the change is considered complete.
5. If a framework-owned document is intentionally excluded from this index, the excluding protocol must state that it is non-canonical reference material.
