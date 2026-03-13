# Project Overlay Protocol (POP)

Purpose: define how VIDA reads project-owned root configuration without losing framework portability.

## Core Contract

1. VIDA framework remains autonomous when no project overlay exists.
2. Project overlay is optional and must live in the repository root as `vida.config.yaml`.
3. Overlay may activate framework protocol bundles, but may not weaken framework invariants.
4. Overlay is project-owned data; framework-owned behavior stays in `AGENTS.md`, `vida/config/instructions/*`, and `taskflow-v0/*`.

No-overlay execution rule:

1. If `vida.config.yaml` is absent, VIDA must use only framework-owned canonical commands and wrappers declared in `AGENTS.md`, `vida/config/instructions/system-maps/protocol.index.md`, and `taskflow-v0/*`.
2. In no-overlay mode, host-project operations docs are not assumed to exist and must not be treated as the canonical command source.
3. Project-specific commands become canonical only after an overlay resolves an explicit host-project operations doc.
4. When neither overlay-resolved project operations nor framework-owned wrappers cover the requested action, the action is not implicitly authorized; the orchestrator must stop, ask the user, or route the gap through tracked framework/project clarification.

## Canonical Root File

Root overlay file:

1. `vida.config.yaml`

Ownership:

1. file location is project root,
2. schema and activation semantics are framework-owned,
3. actual values inside the file are project-owned.

Framework template:

1. framework-owned starter template lives at `docs/framework/templates/vida.config.yaml.template`,
2. template is canonical for scaffold/default structure,
3. instantiated root `vida.config.yaml` remains project-owned data.
4. project docs referenced from `project_bootstrap` remain project-owned runbooks; framework runtime law must stay in `AGENTS.md`, `vida/config/instructions/*`, and `taskflow-v0/*`, not be synchronized into project docs by drift.
5. framework-owned document lifecycle/freshness metadata belongs in `vida/config/instructions/runtime-instructions/work.document-lifecycle-protocol.md` and `.vida/state/doc-lifecycle.json`, not inside project-owned docs.

## Language Policy

Framework-owned rule:

1. `AGENTS.md`, `vida/config/instructions/*`, and `taskflow-v0/*` use English as the framework language.
2. Project-owned language preferences live in root `vida.config.yaml`.
3. Project language preferences may affect user-facing communication, reasoning language, and project documentation language, but they do not localize framework-owned source files.

## Activation Semantics

Overlay is evaluated after core VIDA boot is available.

Activation order:

1. complete the mandatory bootstrap and lane-entry path (`AGENTS.md`, `AGENTS.sidecar.md`, `vida/root-map.md`, selected lane entry, and activation law),
2. detect `vida.config.yaml`,
3. parse overlay,
4. read `protocol_activation.*`,
5. activate only the matching protocol bundles authorized by the activation law and overlay values,
6. initialize runtime state for activated bundles.

Rule:

1. overlay activates protocol domains; it does not redefine framework source files.
2. overlay evaluation must not assume that `core` protocols are preloaded unless the active route or trigger matrix has already activated them.

Schema validation gate:

1. framework validates overlay schema before binding runtime behavior,
2. parse success alone is insufficient; schema validation must also pass,
3. health, bootstrap, and runtime routing/agent-system helpers should fail fast on invalid overlay schema.

## First Supported Bundle

Initial supported overlay domain:

1. `protocol_activation.agent_system`

If `true`, VIDA must activate:

1. `vida/config/instructions/instruction-contracts/core.agent-system-protocol.md`
2. runtime helper: `taskflow-v0 system`

## Minimum Schema Surface

Current required top-level sections:

1. `project`
2. `protocol_activation`
3. `agent_system` when `protocol_activation.agent_system=true`

Optional top-level sections:

1. `language_policy`
2. `pack_router_keywords`
3. `project_bootstrap`
4. `framework_self_diagnosis`
5. `autonomous_execution`
6. `agent_extensions`

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

Supported `agent_extensions` keys:

1. `enabled`
2. `map_doc`
3. `registries`
4. `enabled_framework_roles`
5. `enabled_standard_flow_sets`
6. `enabled_project_roles`
7. `enabled_project_skills`
8. `enabled_shared_skills`
9. `enabled_project_profiles`
10. `enabled_project_flows`
11. `default_flow_set`
12. `validation`
13. `role_selection`

Supported `agent_extensions.registries` keys:

1. `roles`
2. `skills`
3. `profiles`
4. `flows`

Supported `agent_extensions.validation` keys:

1. `require_registry_files`
2. `require_unique_ids`
3. `require_framework_role_compatibility`
4. `require_skill_role_compatibility`
5. `require_profile_resolution`
6. `require_flow_resolution`
7. `fail_closed_on_validation_error`

Supported `agent_extensions.role_selection` keys:

1. `mode`
2. `fallback_role`
3. `conversation_modes`

Supported `agent_extensions.role_selection.conversation_modes.<mode_id>` keys:

1. `enabled`
2. `role`
3. `single_task_only`
4. `tracked_flow_entry`
5. `allow_freeform_chat`

Supported `autonomous_execution` keys:

1. `next_task_boundary_analysis`
2. `next_task_boundary_report`
3. `next_task_boundary_report_gating`
4. `dependent_coverage_autoupdate`
5. `continue_after_reports`
6. `spec_ready_auto_development`
7. `validation_report_required_before_implementation`
8. `resume_after_validation_gate`

Autonomous execution overlay rule:

1. `autonomous_execution` may tune next-task boundary behavior only within framework law.
2. It may disable user-facing boundary reporting, but it may not disable required internal next-task boundary analysis.
3. It may not convert a non-gating boundary report into silent scope widening.
4. Approval gating still belongs to `vida/config/instructions/runtime-instructions/bridge.task-approval-loop-protocol.md`.
5. `continue_after_reports=true` means intermediate lawful reports should auto-advance into the next already-authorized step when no blocker, approval gate, validation gate, explicit user pause, or explicit user request to discuss the report exists.
6. `continue_after_reports` must not bypass research/spec/approval/verification sequencing; it only removes unnecessary stopping after a lawful intermediate report.
7. Pre-execution validation reports remain gating even when `continue_after_reports=true`.
8. `spec_ready_auto_development=true` allows autonomous entry into implementation flow after spec readiness, but only through the normal execution-entry gates.
9. `validation_report_required_before_implementation=true` inserts a mandatory validation-report gate before each implementation slice or implementation-bearing task.
10. `resume_after_validation_gate=true` means that once the validation report is accepted and no other stop condition exists, runtime should continue directly into the authorized implementation step instead of stopping again by inertia.

Current supported `agent_system` keys:

1. `init_on_boot`
2. `mode`
3. `state_owner`
4. `max_parallel_agents`
5. `workers`
6. `routing`
7. `scoring`

Supported worker-level keys:

1. `enabled`
2. `worker_backend_class`
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
21. `binary_path`

Supported worker-level `dispatch` keys:

1. `command`
2. `pre_static_args`
3. `subcommand`
4. `static_args`
5. `write_static_args`
6. `models_cache_path`
7. `workdir_flag`
8. `model_flag`
9. `output_mode`
10. `output_flag`
11. `prompt_mode`
12. `prompt_flag`
13. `web_search_mode`
14. `web_search_flag`
15. `web_probe_static_args`
16. `web_probe_prompt`
17. `web_probe_expect_substring`
18. `web_probe_timeout_seconds`
19. `env`
20. `probe_static_args`
21. `probe_prompt`
22. `probe_expect_substring`
23. `probe_timeout_seconds`
24. `startup_timeout_seconds`
25. `no_output_timeout_seconds`
26. `progress_idle_timeout_seconds`
27. `max_runtime_extension_seconds`

Project agent-extension overlay rule:

1. `agent_extensions` is the project-owned activation and selection surface for project roles, project skills, project profiles, and project flow sets.
2. Framework-owned role law, role-profile law, and runtime routing law remain in `vida/config/instructions/*`.
3. Project-owned custom roles, custom skills, custom profiles, and custom flows must live in project-owned registries referenced by `agent_extensions.registries`.
4. `vida.config.yaml` may enable or disable framework roles and standard flow sets for the active project, but it must not silently redefine framework role authority.
5. Project extensions are lawful only after validation confirms:
   - registry files exist,
   - ids are unique,
   - project roles resolve to known framework base roles,
   - project profiles resolve to known roles and skills,
   - project flows resolve to known roles,
   - no project extension weakens framework safety boundaries.
6. `taskflow-v0 config validate` is the bounded runtime proof surface for the current overlay-level validation of `agent_extensions`.

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

1. worker `profiles`
2. worker `models_hint`
3. worker `capability_band`
4. worker `specialties`
5. route `workers`
6. route `fanout_workers`
7. worker `dispatch.static_args`
8. worker `dispatch.pre_static_args`
9. `framework_self_diagnosis.session_reflection_criteria`

Supported routing-level keys:

1. `workers`
2. `models`
3. `profiles`
4. `analysis_required`
5. `analysis_route_task_class`
6. `analysis_fanout_workers`
7. `analysis_fanout_min_results`
8. `analysis_merge_policy`
9. `analysis_external_first_required`
10. `analysis_receipt_required`
11. `analysis_zero_budget_required`
12. `analysis_default_in_boot`
13. `coach_required`
14. `coach_route_task_class`
15. `write_scope`
16. `verification_gate`
17. `max_runtime_seconds`
18. `min_output_bytes`
19. `fanout_workers`
20. `fanout_min_results`
21. `merge_policy`
22. `dispatch_required`
23. `external_first_required`
24. `web_search_required`
25. `local_execution_allowed`
26. `local_execution_preferred`
27. `cli_dispatch_required_if_delegating`
28. `direct_internal_bypass_forbidden`
29. `bridge_fallback_worker`
30. `internal_escalation_trigger`
31. `allowed_internal_reasons`
32. `verification_route_task_class`
33. `independent_verification_required`
34. `graph_strategy`
35. `deterministic_first`
36. `budget_policy`
37. `max_budget_units`
38. `max_rounds`
39. `max_stalls`
40. `max_resets`
41. `max_cli_worker_calls`
42. `max_coach_passes`
43. `max_verification_passes`
44. `max_fallback_hops`
45. `max_total_runtime_seconds`
46. `problem_party_required`
47. `problem_party_task_class`

Derived route-receipt note:

1. `dispatch_policy.internal_escalation_allowed` is a runtime-derived receipt field, not a project-owned overlay key.
2. The runtime derives it from the presence of lawful `allowed_internal_reasons` and the active route policy.
3. effective route control limits (`max_rounds`, `max_stalls`, `max_resets`, `max_budget_units`, `max_total_runtime_seconds`) must be materialized into the active route receipt even when some values come from runtime defaults rather than explicit overlay data.
4. route receipts must expose the effective verification posture (`verification_route_task_class`, `independent_verification_required`) together with the effective route control limits so execution and recovery stages do not reconstruct those constraints ad hoc.

Validation scope:

1. required top-level sections and required fields inside them,
2. unsupported keys in canonical sections,
3. type checks for booleans, integers, strings, mappings, and repeated-string fields,
4. worker `dispatch` requirements for enabled `external_cli` workers,
5. route/worker consistency checks such as `default_profile in profiles` and `fanout_min_results <= fanout_workers`,
6. web-search capability consistency between `capability_band` and dispatch wiring.
7. silent framework diagnosis overlay schema when present.

Availability-state contract:

1. worker runtime may persist worker availability separately from quality score,
2. canonical worker availability states are:
   - `active`
   - `degraded`
   - `quota_exhausted`
   - `disabled_manual`
3. temporary worker suppression should use `cooldown_until`,
4. probe-driven recovery may use `probe_required=true`,
5. new overlays should prefer explicit probe-capable dispatch for external CLI workers that support headless smoke checks.
6. web-search-capable workers should declare both `capability_band=web_search` and dispatch-level wiring via `dispatch.web_search_mode`.
7. `dispatch.web_search_mode=provider_configured` is an operator-trusted declaration of provider-side search enablement; it is weaker than an explicit flag-based path and does not by itself prove a live search probe.
8. `dispatch.web_probe_*` allows provider-agnostic live web-search smoke checks without hardcoding a specific CLI into framework runtime.
9. `dispatch.models_cache_path` allows CLI-specific model-cache discovery to remain config-driven.

## Portability Rule

Framework scripts must treat missing overlay as a valid state.

Portable default behavior:

1. no project overlay -> no project-specific bundle activation,
2. no project overlay -> framework still executes using generic protocols only.

## Enforcement Rule

Overlay may configure:

1. which worker backend classes are allowed,
2. which routing preferences are preferred,
3. which escalation thresholds apply,
4. external-first routing preference for eligible read-only classes,
5. which worker is the canonical bridge fallback before internal escalation.

Overlay may not configure:

1. permission to bypass the DB-backed task runtime as SSOT,
2. permission to bypass verification gates,
3. permission to let external workers mutate framework task state directly.

## Runtime Files

Current runtime artifacts for overlay activation:

1. `.vida/state/worker-init.json`
2. `.vida/state/worker-scorecards.json`
3. `.vida/state/worker-strategy.json`

These are runtime state files, not canonical project configuration.

## Verification

Minimum proof for overlay support:

```bash
taskflow-v0 config validate
taskflow-v0 config dump
taskflow-v0 config protocol-active agent_system
taskflow-v0 system snapshot [task_id]
```

Minimum proof for framework template support:

```bash
taskflow-v0 boot read-contract lean
taskflow-v0 boot snapshot --json
```

-----
artifact_path: config/runtime-instructions/project-overlay.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/bridge.project-overlay-protocol.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-13T07:44:24+02:00'
changelog_ref: bridge.project-overlay-protocol.changelog.jsonl
