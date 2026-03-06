# Changelog

Rules:

1. Newest entries must always be added at the top.
2. Each entry must start with a full timestamp in `YYYY-MM-DD HH:MM` format.
3. Record only significant framework changes.
4. Group updates under fixed headings when applicable: `Added`, `Changed`, `Fixed`, `Protocol`.
5. Keep this file limited to VIDA framework/runtime changes, not project feature work.

## 2026-03-07 00:36

Added:

1. `install/install.sh` as a bash-only installer entrypoint for `init`, `upgrade`, and `doctor`.

Changed:

1. Release archive packaging to include only `AGENTS.md` and `_vida/`.
2. Release archive workflow to keep installer and repository-level docs out of framework distribution artifacts.
3. `install/install.sh` doctor contract to validate framework-only payload without requiring `_vida/CHANGELOG.md`.

Fixed:

1. Installer temporary cleanup after `init` and `upgrade`.
2. Added local archive override support for installer validation without GitHub network access.

## 2026-03-07 00:15

Added:

1. `_vida/CHANGELOG.md` as the canonical framework change log.

Changed:

1. `_vida/templates/vida.config.yaml.template` to reflect the real agent-system shape:
   `senior_internal`, `external_free`, `cost_priority`, `dispatch.env`, runtime budget fields, and fanout metadata examples.
2. `_vida/docs/protocol-index.md` to link the framework change log.

## 2026-03-06 23:55

Added:

1. `_vida/docs/SUBAGENT-ENTRY.MD` as the worker-lane entry contract.

Changed:

1. `_vida/docs/subagents.md` to separate orchestrator entry from worker entry.
2. `_vida/docs/subagent-prompt-templates.md` so external workers receive bounded worker semantics instead of orchestrator identity.
3. `_vida/scripts/render-subagent-prompt.sh` to inject `Worker Entry Contract` into canonical rendered prompts.

## 2026-03-06 23:20

Changed:

1. Hardened subagent dispatch runtime with managed subprocess polling.
2. Added manifest `phase` visibility for `fanout_running`, `fallback_running`, `merge_evaluating`, `arbitration_running`, and completion states.

Fixed:

1. Added timed termination, early-stop, and unreachable-stop behavior for ensemble fanout.
2. Reduced unnecessary arbitration churn through stronger merge handling.

## 2026-03-06 22:40

Changed:

1. Prioritized free external providers as the default first-pass lane for eligible read-only work.
2. Formalized `gpt-5.1-codex-mini` as the canonical bridge fallback.
3. Moved internal subagents into the senior arbitration / architecture / mutation-owning lane.

Protocol:

1. Extended routing outputs with explicit orchestration hierarchy metadata.

## 2026-03-06 22:10

Added:

1. Source-backed merge weighting.
2. `dispatch.env` support for provider-specific runtime environment variables.

Changed:

1. Started progress-aware runtime behavior with `useful_progress` tracking.

Protocol:

1. Updated subagent-system protocol to distinguish worker-entry, useful-progress, and merge-ready runtime states.
