# Project Overlay Protocol (POP)

Purpose: define how VIDA reads project-owned root configuration without losing framework portability.

## Core Contract

1. VIDA framework remains autonomous when no project overlay exists.
2. Project overlay is optional and must live in the repository root as `vida.config.yaml`.
3. Overlay may activate framework protocol bundles, but may not weaken framework invariants.
4. Overlay is project-owned data; framework-owned behavior stays in `AGENTS.md` and `_vida/*`.

## Canonical Root File

Root overlay file:

1. `vida.config.yaml`

Ownership:

1. file location is project root,
2. schema and activation semantics are framework-owned,
3. actual values inside the file are project-owned.

Framework template:

1. framework-owned starter template lives at `_vida/templates/vida.config.yaml.template`,
2. template is canonical for scaffold/default structure,
3. instantiated root `vida.config.yaml` remains project-owned data.
4. project docs referenced from `project_bootstrap` remain project-owned runbooks; framework runtime law must stay in `AGENTS.md` and `_vida/*`, not be synchronized into `docs/*`.

## Language Policy

Framework-owned rule:

1. `AGENTS.md`, `_vida/docs/*`, and `_vida/scripts/*` use English as the framework language.
2. Project-owned language preferences live in root `vida.config.yaml`.
3. Project language preferences may affect user-facing communication, reasoning language, and project documentation language, but they do not localize framework-owned source files.

## Activation Semantics

Overlay is evaluated after core VIDA boot is available.

Activation order:

1. load framework invariants (`AGENTS.md` + core protocols),
2. detect `vida.config.yaml`,
3. parse overlay,
4. read `protocol_activation.*`,
5. activate matching protocol bundles,
6. initialize runtime state for activated bundles.

Rule:

1. overlay activates protocol domains; it does not redefine framework source files.

Schema validation gate:

1. framework validates overlay schema before binding runtime behavior,
2. parse success alone is insufficient; schema validation must also pass,
3. health, bootstrap, and runtime routing/agent-system helpers should fail fast on invalid overlay schema.

## First Supported Bundle

Initial supported overlay domain:

1. `protocol_activation.agent_system`

If `true`, VIDA must activate:

1. `_vida/docs/subagent-system-protocol.md`
2. runtime helper: `_vida/scripts/subagent-system.py`

## Minimum Schema Surface

Current required top-level sections:

1. `project`
2. `protocol_activation`
3. `agent_system` when `protocol_activation.agent_system=true`

Optional top-level sections:

1. `language_policy`
2. `pack_router_keywords`
3. `project_bootstrap`

Supported `language_policy` keys:

1. `user_communication`
2. `reasoning`
3. `documentation`
4. `todo_protocol`

Supported `pack_router_keywords` keys:

1. `research`
2. `spec`
3. `pool`
4. `pool_strong`
5. `pool_dependency`
6. `dev`
7. `bug`
8. `reflect`
9. `reflect_strong`

Supported `project_bootstrap` keys:

1. `enabled`
2. `docs_root`
3. `process_root`
4. `research_root`
5. `readme_doc`
6. `architecture_doc`
7. `decisions_doc`
8. `environments_doc`
9. `project_operations_doc`
10. `agent_system_doc`
11. `allow_scaffold_missing`
12. `require_launch_confirmation`

Current supported `agent_system` keys:

1. `init_on_boot`
2. `mode`
3. `state_owner`
4. `max_parallel_agents`
5. `subagents`
6. `routing`
7. `scoring`

Supported subagent-level keys:

1. `enabled`
2. `subagent_backend_class`
3. `detect_command`
4. `role`
5. `orchestration_tier`
6. `cost_priority`
7. `max_runtime_seconds`
8. `min_output_bytes`
9. `models_hint`
10. `default_model`
11. `profiles`
12. `default_profile`
13. `capability_band`
14. `write_scope`
15. `billing_tier`
16. `budget_cost_units`
17. `speed_tier`
18. `quality_tier`
19. `specialties`
20. `dispatch`

Supported subagent-level `dispatch` keys:

1. `command`
2. `pre_static_args`
3. `subcommand`
4. `static_args`
5. `write_static_args`
6. `workdir_flag`
7. `model_flag`
8. `output_mode`
9. `output_flag`
10. `prompt_mode`
11. `prompt_flag`
12. `web_search_mode`
13. `web_search_flag`
14. `env`
15. `probe_static_args`
16. `probe_prompt`
17. `probe_expect_substring`
18. `probe_timeout_seconds`
19. `startup_timeout_seconds`
20. `no_output_timeout_seconds`
21. `progress_idle_timeout_seconds`
22. `max_runtime_extension_seconds`

Supported `agent_system.scoring` keys:

1. `consecutive_failure_limit`
2. `promotion_score`
3. `demotion_score`
4. `probation_success_runs`
5. `probation_task_runs`
6. `retirement_failure_limit`

Repeated-scalar encoding:

1. repeated-scalar fields may be expressed as CSV strings or YAML lists,
2. prefer YAML lists in new overlays and framework templates,
3. runtime helpers must accept both formats for backward compatibility.

Common repeated-scalar examples:

1. subagent `profiles`
2. subagent `models_hint`
3. subagent `capability_band`
4. subagent `specialties`
5. route `subagents`
6. route `fanout_subagents`
7. subagent `dispatch.static_args`
8. subagent `dispatch.pre_static_args`
9. `framework_self_diagnosis.session_reflection_criteria`

Supported routing-level keys:

1. `subagents`
2. `models`
3. `profiles`
4. `write_scope`
5. `verification_gate`
6. `max_runtime_seconds`
7. `min_output_bytes`
8. `fanout_subagents`
9. `fanout_min_results`
10. `merge_policy`
11. `dispatch_required`
12. `external_first_required`
13. `web_search_required`
14. `bridge_fallback_subagent`
15. `internal_escalation_trigger`
16. `verification_route_task_class`
17. `independent_verification_required`
18. `graph_strategy`
19. `deterministic_first`
20. `budget_policy`
21. `max_budget_units`
22. `max_cli_subagent_calls`
23. `max_verification_passes`
24. `max_fallback_hops`
25. `max_total_runtime_seconds`

Validation scope:

1. required top-level sections and required fields inside them,
2. unsupported keys in canonical sections,
3. type checks for booleans, integers, strings, mappings, and repeated-string fields,
4. subagent `dispatch` requirements for enabled `external_cli` subagents,
5. route/subagent consistency checks such as `default_profile in profiles` and `fanout_min_results <= fanout_subagents`,
6. web-search capability consistency between `capability_band` and dispatch wiring.
7. silent framework diagnosis overlay schema when present.

Availability-state contract:

1. subagent runtime may persist subagent availability separately from quality score,
2. canonical subagent availability states are:
   - `active`
   - `degraded`
   - `quota_exhausted`
   - `disabled_manual`
3. temporary subagent suppression should use `cooldown_until`,
4. probe-driven recovery may use `probe_required=true`,
5. new overlays should prefer explicit probe-capable dispatch for external CLI subagents that support headless smoke checks.
6. web-search-capable subagents should declare both `capability_band=web_search` and dispatch-level wiring via `dispatch.web_search_mode`.
7. `dispatch.web_search_mode=provider_configured` is an operator-trusted declaration of provider-side search enablement; it is weaker than an explicit flag-based path and does not by itself prove a live search probe.

## Portability Rule

Framework scripts must treat missing overlay as a valid state.

Portable default behavior:

1. no project overlay -> no project-specific bundle activation,
2. no project overlay -> framework still executes using generic protocols only.

## Enforcement Rule

Overlay may configure:

1. which subagent backend classes are allowed,
2. which routing preferences are preferred,
3. which escalation thresholds apply,
4. external-first routing preference for eligible read-only classes,
5. which subagent is the canonical bridge fallback before internal escalation.

Overlay may not configure:

1. permission to bypass `br` as SSOT,
2. permission to bypass verification gates,
3. permission to let external subagents mutate framework task state directly.

## Runtime Files

Current runtime artifacts for overlay activation:

1. `.vida/state/subagent-init.json`
2. `.vida/state/subagent-scorecards.json`
3. `.vida/state/subagent-strategy.json`

These are runtime state files, not canonical project configuration.

## Verification

Minimum proof for overlay support:

```bash
python3 _vida/scripts/vida-config.py path
python3 _vida/scripts/vida-config.py validate --json
python3 _vida/scripts/vida-config.py protocol-active agent_system
python3 _vida/scripts/subagent-system.py init [task_id]
```

Minimum proof for framework template support:

```bash
python3 _vida/scripts/project-bootstrap.py emit-contract --json
python3 _vida/scripts/project-bootstrap.py scaffold --json
```
