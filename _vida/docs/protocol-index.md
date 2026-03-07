# Protocol Index (Single Source Map)

Purpose: one entry point for protocol governance. This file maps canonical sources and required gates.

## Canonical Sources

| Domain | Canonical Source | Secondary/Reference |
|---|---|---|
| Framework topology map | `_vida/docs/framework-map-protocol.md` | `_vida/docs/protocol-index.md` |
| Command layer matrix | `_vida/docs/command-layer-protocol.md` | `_vida/commands.md`, `_vida/commands/vida-*.md`, `_vida/docs/implement-execution-protocol.md`, `_vida/docs/bug-fix-protocol.md`, `_vida/docs/use-case-packs.md`, `_vida/docs/todo-protocol.md`, `_vida/docs/subagents.md`, `_vida/scripts/vida-command-audit.sh`, `_vida/scripts/render-subagent-prompt.sh`, `_vida/docs/framework-map-protocol.md` |
| Runtime script architecture | `_vida/docs/script-runtime-architecture.md` | `_vida/docs/framework-map-protocol.md`, `_vida/scripts/*.sh`, `_vida/scripts/*.py` |
| Framework change log | `_vida/CHANGELOG.md` | `_vida/docs/protocol-index.md` |
| Project overlay activation | `_vida/docs/project-overlay-protocol.md` | `vida.config.yaml`, `_vida/templates/vida.config.yaml.template`, `AGENTS.md`, `_vida/scripts/vida-config.py`, `_vida/scripts/boot-profile.sh`, `_vida/scripts/quality-health-check.sh` |
| Boot packet runtime artifact | `_vida/docs/boot-packet-protocol.md` | `_vida/scripts/boot-packet.py`, `_vida/scripts/boot-profile.sh`, `AGENTS.md`, `_vida/docs/ORCHESTRATOR-ENTRY.MD`, `_vida/docs/SUBAGENT-ENTRY.MD` |
| Project bootstrap/self-reproduction | `_vida/docs/project-bootstrap-protocol.md` | `_vida/scripts/project-bootstrap.py`, `_vida/templates/vida.config.yaml.template`, `vida.config.yaml` |
| VIDA framework self-analysis | `_vida/docs/framework-self-analysis-protocol.md` | `_vida/docs/framework-map-protocol.md`, `_vida/docs/self-reflection-protocol.md` |
| Silent framework diagnosis | `_vida/docs/silent-framework-diagnosis-protocol.md` | `_vida/scripts/vida-silent-diagnosis.py`, `vida.config.yaml`, `_vida/docs/framework-self-analysis-protocol.md`, `_vida/docs/todo-protocol.md` |
| Problem-party discussion | `_vida/docs/problem-party-protocol.md` | `_vida/scripts/problem-party.py`, `_vida/docs/orchestration-protocol.md`, `_vida/docs/todo-protocol.md` |
| Core bootstrap router | `AGENTS.md` | `_vida/docs/ORCHESTRATOR-ENTRY.MD`, `_vida/docs/SUBAGENT-ENTRY.MD`, `_vida/docs/SUBAGENT-THINKING.MD`, `_vida/docs/README.md`, `docs/README.md` |
| Orchestrator entry contract | `_vida/docs/ORCHESTRATOR-ENTRY.MD` | `AGENTS.md`, `_vida/docs/orchestration-protocol.md`, `_vida/docs/use-case-packs.md` |
| Thinking algorithms | `_vida/docs/thinking-protocol.md` | `_vida/docs/algorithms-one-screen.md`, `_vida/docs/algorithms-quick-reference.md` |
| Runtime orchestration | `_vida/docs/orchestration-protocol.md` | `AGENTS.md`, `_vida/docs/use-case-packs.md`, `_vida/scripts/vida-pack-helper.sh` |
| Change-impact reconciliation (absorbed cascade) | `_vida/docs/use-case-packs.md` | `_vida/docs/form-task-protocol.md`, `_vida/docs/implement-execution-protocol.md`, `_vida/commands/vida-spec.md` |
| Task state (SSOT) | `_vida/docs/beads-protocol.md` | `_vida/docs/todo-protocol.md` |
| Framework wave starter | `_vida/scripts/framework-wave-start.sh` | `_vida/docs/framework-self-analysis-protocol.md`, `_vida/docs/todo-protocol.md`, `_vida/scripts/vida-pack-helper.sh`, `_vida/scripts/boot-profile.sh` |
| Product/framework proving packs | `_vida/docs/product-proving-packs-protocol.md` | `_vida/scripts/proving-pack.py` |
| Framework wave task-state sync | `_vida/scripts/framework-task-sync.py` | `.vida/state/framework-wave-task-sync.json`, `_vida/docs/todo-protocol.md`, `_vida/docs/beads-protocol.md` |
| Shared reference catalog (non-runtime) | `docs/**` | `_vida/docs/beads-protocol.md` |
| Execution pipelines | `_vida/docs/pipelines.md` | `_vida/scripts/quality-health-check.sh`, `_vida/scripts/framework-boundary-check.sh` |
| Use-case routing | `_vida/docs/use-case-packs.md` | `_vida/scripts/vida-pack-router.sh`, `_vida/scripts/vida-pack-helper.sh` |
| Bug-fix unified flow | `_vida/docs/bug-fix-protocol.md` | `_vida/commands/vida-bug-fix.md`, `_vida/docs/use-case-packs.md` |
| Issue-as-contract bridge | `_vida/docs/issue-contract-protocol.md` | `_vida/docs/bug-fix-protocol.md`, `_vida/docs/implement-execution-protocol.md`, `_vida/scripts/subagent-dispatch.py`, `_vida/scripts/execution-auth-gate.py` |
| Web/internet validation | `_vida/docs/web-validation-protocol.md` | `_vida/docs/thinking-protocol.md#section-web-search`, `_vida/docs/spec-contract-protocol.md` |
| Spec contract (non-dev flows) | `_vida/docs/spec-contract-protocol.md` | `_vida/docs/spec-contract-artifacts.md`, `_vida/commands/vida-spec.md`, `_vida/scripts/skill-discovery.py`, `_vida/scripts/scp-confidence.py` |
| Form-task bridge (spec->dev) | `_vida/docs/form-task-protocol.md` | `_vida/commands/vida-form-task.md`, `_vida/docs/use-case-packs.md` |
| Planning decomposition (Q-Gate -> TODO plan) | `_vida/docs/todo-protocol.md` | `_vida/docs/form-task-protocol.md`, `_vida/docs/silent-framework-diagnosis-protocol.md`, `_vida/scripts/todo-plan-validate.sh`, `_vida/scripts/stateful-sequence-check.sh` |
| Implement execution (dev) | `_vida/docs/implement-execution-protocol.md` | `_vida/commands/vida-implement.md`, `_vida/docs/use-case-packs.md`, `_vida/docs/command-layer-protocol.md` |
| VIDA migration decisions | `docs/research/vida-framework/vida-migration-registry.md` | `docs/research/vida-framework/**`, `_vida/**` |
| Subagent system activation/routing | `_vida/docs/subagent-system-protocol.md` | `_vida/scripts/subagent-system.py`, `vida.config.yaml`, `docs/process/agent-system.md`, `_vida/docs/DEV-AGENTS-MATRIX.md` |
| Subagent onboarding and recovery | `_vida/docs/subagent-onboarding-protocol.md` | `_vida/scripts/subagent-system.py`, `_vida/scripts/subagent-dispatch.py`, `_vida/scripts/subagent-eval-pack.py`, `vida.config.yaml`, `_vida/templates/vida.config.yaml.template` |
| Worker entry contract | `_vida/docs/SUBAGENT-ENTRY.MD` | `AGENTS.md`, `_vida/docs/subagents.md`, `_vida/docs/subagent-system-protocol.md`, `_vida/docs/SUBAGENT-THINKING.MD` |
| Worker thinking subset | `_vida/docs/SUBAGENT-THINKING.MD` | `AGENTS.md`, `_vida/docs/SUBAGENT-ENTRY.MD`, `_vida/docs/subagent-prompt-templates.md` |
| Subagent dispatch | `_vida/docs/subagents.md` | `_vida/docs/SUBAGENT-ENTRY.MD`, `_vida/docs/SUBAGENT-THINKING.MD`, `_vida/docs/subagent-prompt-templates.md`, `_vida/scripts/render-subagent-prompt.sh` |
| Runtime log policy | `_vida/docs/log-policy.md` | `.gitignore` |
| TODO overhead diagnostics | `_vida/scripts/todo-overhead-report.sh` | `_vida/docs/todo-protocol.md` |
| Project operations (build/run/observability/live checks) | host-project operations doc declared by project overlay | host-project scripts and runbooks |
| Environment/auth notes | `docs/environments.md` | - |
| Skill catalog | `.agents/skills/` | - |
| GitHub operations | `_vida/docs/pipelines.md` | `gh` CLI help |

## Mandatory Gates

1. Before close/handoff: `bash _vida/scripts/quality-health-check.sh <task_id>`.
2. Before `finish`: strict execution-log verify must pass.
3. For server/API assumptions: live request validation is required.
4. For external assumptions (API/package/platform/security/migration): WVP evidence is required (`_vida/docs/web-validation-protocol.md`).
5. For topology/refactor changes: update `_vida/docs/framework-map-protocol.md` in the same change.
6. For entry-contract changes: keep `AGENTS.md`, `_vida/docs/ORCHESTRATOR-ENTRY.MD`, `_vida/docs/SUBAGENT-ENTRY.MD`, and `_vida/docs/SUBAGENT-THINKING.MD` synchronized in the same change.
7. Before pack/command/TODO engagement, run request-intent classification and skip task machinery for `answer_only` unless the user explicitly asks for an artifact or mutation.
8. Broad reads of `.vida/logs`, `.vida/state`, and `.beads` are forbidden by default; use exact-key, specific-file, short-window reads unless the active lane contract explicitly escalates.

## Execution Command Path

```bash
bash _vida/scripts/beads-workflow.sh start <task_id>
bash _vida/scripts/beads-workflow.sh block-start <task_id> <BXX> "goal"
bash _vida/scripts/beads-workflow.sh block-finish <task_id> <BXX> done "<next_block>" "actions" "artifacts" - - "evidence" "85"
bash _vida/scripts/quality-health-check.sh <task_id>
bash _vida/scripts/beads-workflow.sh finish <task_id> "reason"
```

## Scope Rule: `_vida/docs/` vs `docs/`

1. `_vida/docs/` = framework runtime policy and protocol source.
2. `docs/` = active project/domain documentation.
3. `docs/process/` = project operational runbooks.
4. `scripts/` = executable project operations.
5. If project guidance becomes runtime protocol, move only the protocol portion to `_vida/docs/`; keep project commands in `docs/process/` and `scripts/`.

## Maintenance Rule

When a protocol changes:

1. Update the canonical file first.
2. Update linked references in the same change.
3. Keep this index synchronized.
