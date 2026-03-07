# Changelog

Rules:

1. Newest entries must always be added at the top.
2. Each entry must start with a full timestamp in `YYYY-MM-DD HH:MM` format.
3. Record only significant framework changes.
4. Group updates under fixed headings when applicable: `Added`, `Changed`, `Fixed`, `Protocol`.
5. Keep this file limited to VIDA framework/runtime changes, not project feature work.

## 2026-03-07 06:31

Changed:

1. `RELEASE-1-SCOPE.md` implementation audit now uses explicit `Done`, `Partial`, and `Not Done` status groups with checklist evidence instead of coarse summary labels.
2. `RELEASE-1-IMPLEMENTATION-ROADMAP.md` implementation audit now tracks completed and missing work through checklist subitems rather than broad one-line status statements.

Protocol:

1. Release-target documentation status now distinguishes fully implemented items from partially implemented areas with concrete completed sub-capabilities.

## 2026-03-07 06:05

Added:

1. `subagent-system.py` now exposes recovery helpers: `recover <subagent>` and `recover-pending`.
2. Ensemble manifests now expose live `active_subagents` and `active_count` during running fanout.

Changed:

1. Runtime vocabulary was pushed further toward canonical `cli subagent` terminology across dispatch, routing, evaluation, and operator status surfaces.
2. Worker gating now relies on structured evidence signals instead of a coarse byte-size fallback for `useful_progress` and `merge_ready`.
3. Operator status now exposes `preferred_task_classes` so lane-fit can be seen without inspecting separate route calls.
4. `quality-health-check.sh` now reads the canonical `.vida/logs/subagent-runs.jsonl` run log and surfaces `cli subagent` health state directly.

Fixed:

1. Routing now hydrates fresh scorecards from `SCORECARD_PATH` instead of relying on stale `INIT_PATH` runtime snapshots.
2. `auth_invalid` and `interactive_blocked` remediation semantics now consistently suppress routing and require bounded recovery/probe flow.
3. Health output now shows degraded/cooldown/probe-required cli subagents by name.
4. Runtime availability state migration now canonicalizes old `provider_state` payloads to `subagent_state`.

Protocol:

1. `subagent-system-protocol.md` now reflects recovery commands, suppressed-subagent visibility, and live ensemble manifest expectations.
2. `subagent-onboarding-protocol.md` now documents recovery flow and routing-block semantics for broken cli subagents.

## 2026-03-07 01:41

Changed:

1. `_vida/templates/vida.config.yaml.template` now mirrors the canonical VIDA provider stack instead of a generic single-provider example.
2. The template now includes practical runtime settings for real CLI subagents: provider tiers, `max_runtime_seconds`, `min_output_bytes`, bridge fallback, and external-first routing metadata.

Fixed:

1. The template now embeds provider-specific timeout environment settings where they are known to be operationally useful, including `OPENCODE_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS` for `kilo_cli` and `opencode_cli`.

Protocol:

1. The default overlay template is now aligned with the real subagent runtime contract, so new projects inherit working provider configuration instead of abstract placeholders.

## 2026-03-07 01:35

Added:

1. `_vida/docs/SUBAGENT-THINKING.MD` as the worker-lane thinking subset limited to `STC`, `PR-CoT`, and `MAR`.

Changed:

1. Worker-lane prompts now inject both entry and thinking contracts through `_vida/scripts/render-subagent-prompt.sh`.
2. Semantic merge now uses similarity-based clustering instead of near-full-text grouping.
3. Scorecards now track runtime maturity metrics including `useful_progress_rate`, `avg_time_to_first_useful_output_ms`, and `timeout_after_progress_count`.

Fixed:

1. Manifest fallback state no longer reports premature `provider_exhausted=true` during active fallback execution.
2. Semantic consensus with strong agreement now resolves more cleanly without unnecessary open conflicts or arbitration.

Protocol:

1. Worker reasoning is now explicitly separated from orchestrator reasoning.
2. Framework docs/scripts were de-projectized to remove host-specific identity, stack, and domain assumptions from canonical runtime policy.

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
