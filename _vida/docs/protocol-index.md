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
| Project bootstrap/self-reproduction | `_vida/docs/project-bootstrap-protocol.md` | `_vida/scripts/project-bootstrap.py`, `_vida/templates/vida.config.yaml.template`, `vida.config.yaml` |
| VIDA framework self-analysis | `_vida/docs/framework-self-analysis-protocol.md` | `_vida/docs/framework-map-protocol.md`, `_vida/docs/self-reflection-protocol.md` |
| Core agent policy | `AGENTS.md` | `_vida/docs/README.md`, `docs/README.md` |
| Thinking algorithms | `_vida/docs/thinking-protocol.md` | `_vida/docs/algorithms-one-screen.md`, `_vida/docs/algorithms-quick-reference.md` |
| Runtime orchestration | `_vida/docs/orchestration-protocol.md` | `AGENTS.md`, `_vida/docs/use-case-packs.md`, `_vida/scripts/vida-pack-helper.sh` |
| Change-impact reconciliation (absorbed cascade) | `_vida/docs/use-case-packs.md` | `_vida/docs/form-task-protocol.md`, `_vida/docs/implement-execution-protocol.md`, `_vida/commands/vida-spec.md` |
| Task state (SSOT) | `_vida/docs/beads-protocol.md` | `_vida/docs/todo-protocol.md` |
| Shared reference catalog (non-runtime) | `docs/**` | `_vida/docs/beads-protocol.md` |
| Execution pipelines | `_vida/docs/pipelines.md` | `_vida/scripts/quality-health-check.sh`, `_vida/scripts/framework-boundary-check.sh` |
| Use-case routing | `_vida/docs/use-case-packs.md` | `_vida/scripts/vida-pack-router.sh`, `_vida/scripts/vida-pack-helper.sh` |
| Bug-fix unified flow | `_vida/docs/bug-fix-protocol.md` | `_vida/commands/vida-bug-fix.md`, `_vida/docs/use-case-packs.md` |
| Web/internet validation | `_vida/docs/web-validation-protocol.md` | `_vida/docs/thinking-protocol.md#section-web-search`, `_vida/docs/spec-contract-protocol.md` |
| Spec contract (non-dev flows) | `_vida/docs/spec-contract-protocol.md` | `_vida/docs/spec-contract-artifacts.md`, `_vida/commands/vida-spec.md`, `_vida/scripts/skill-discovery.py`, `_vida/scripts/scp-confidence.py` |
| Form-task bridge (spec->dev) | `_vida/docs/form-task-protocol.md` | `_vida/commands/vida-form-task.md`, `_vida/docs/use-case-packs.md` |
| Planning decomposition (Q-Gate -> TODO plan) | `_vida/docs/todo-protocol.md` | `_vida/docs/form-task-protocol.md`, `_vida/scripts/todo-plan-validate.sh`, `_vida/scripts/stateful-sequence-check.sh` |
| Implement execution (dev) | `_vida/docs/implement-execution-protocol.md` | `_vida/commands/vida-implement.md`, `_vida/docs/use-case-packs.md`, `_vida/docs/command-layer-protocol.md` |
| VIDA migration decisions | `docs/research/vida-framework/vida-migration-registry.md` | `docs/research/vida-framework/**`, `_vida/**` |
| Subagent system activation/routing | `_vida/docs/subagent-system-protocol.md` | `_vida/scripts/subagent-system.py`, `vida.config.yaml`, `docs/process/agent-system.md`, `_vida/docs/DEV-AGENTS-MATRIX.md` |
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
