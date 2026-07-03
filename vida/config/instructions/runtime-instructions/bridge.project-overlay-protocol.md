# Project Overlay Protocol (POP)

Purpose: define how VIDA reads project-owned root configuration without losing framework portability.

## Core Contract

1. VIDA framework remains autonomous when no project overlay exists.
2. Project overlay is optional and must live in the repository root as `vida.config.yaml`.
3. Overlay may activate framework protocol bundles, but may not weaken framework invariants.
4. Overlay is project-owned data; framework-owned behavior stays in `AGENTS.md`, `vida/config/instructions/**`, and active TaskFlow runtime-family surfaces.

No-overlay execution rule:

1. If `vida.config.yaml` is absent, VIDA must use only framework-owned canonical commands and wrappers declared in `AGENTS.md`, `system-maps/protocol.index`, and active TaskFlow runtime-family surfaces.
2. In no-overlay mode, host-project operations docs are not assumed to exist and must not be treated as the canonical command source.
3. Project-specific commands become canonical only after an overlay resolves an explicit host-project operations doc.
4. When neither overlay-resolved project operations nor framework-owned wrappers cover the requested action, the action is not implicitly authorized; the orchestrator must stop, ask the user, or route the gap through tracked framework/project clarification.

## Canonical Root File

| Surface | Contract |
| --- | --- |
| root overlay file | `vida.config.yaml` |
| location | project root |
| schema and activation semantics | framework-owned |
| actual values | project-owned |
| framework template | `docs/framework/templates/vida.config.yaml.template` |
| template role | canonical scaffold/default structure |
| instantiated root file | project-owned data |
| `project_bootstrap` docs | project-owned runbooks |
| framework runtime law | stays in `AGENTS.md`, `vida/config/instructions/**`, and active TaskFlow runtime-family surfaces; do not synchronize it into project docs by drift |
| lifecycle/freshness metadata | owned by `runtime-instructions/work.document-lifecycle-protocol` and `.vida/state/doc-lifecycle.json`, not project-owned docs |

## Language Policy

Framework-owned rule:

1. `AGENTS.md`, `vida/config/instructions/**`, and active TaskFlow runtime-family surfaces use English as the framework language.
2. Project-owned language preferences live in root `vida.config.yaml`.
3. Project language preferences may affect user-facing communication, reasoning language, and project documentation language, but they do not localize framework-owned source files.

## Activation Semantics

Overlay is evaluated after core VIDA boot is available.

Activation order:

1. complete the mandatory bootstrap and lane-entry path (`AGENTS.md`, `AGENTS.sidecar.md`, selected lane entry, and activation law),
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

Initial supported overlay domain: `protocol_activation.agent_system`.

If `true`, VIDA must activate `instruction-contracts/core.agent-system-protocol` and runtime helper `vida taskflow system`.

## Minimum Schema Surface

Required top-level sections: `project`, `protocol_activation`, and `agent_system` when `protocol_activation.agent_system=true`.

Optional top-level sections: `language_policy`, `pack_router_keywords`, `project_bootstrap`, `framework_self_diagnosis`, `autonomous_execution`, `agent_extensions`.

| Schema path | Supported keys |
| --- | --- |
| `language_policy` | `user_communication`, `reasoning`, `documentation`, `todo_protocol` |
| `pack_router_keywords` | `research`, `spec`, `pool`, `pool_strong`, `pool_dependency`, `dev`, `bug`, `reflect`, `reflect_strong` |
| `project_bootstrap` | `enabled`, `docs_root`, `process_root`, `research_root`, `readme_doc`, `architecture_doc`, `decisions_doc`, `environments_doc`, `project_operations_doc`, `agent_system_doc`, `allow_scaffold_missing`, `require_launch_confirmation` |
| `agent_extensions` | `enabled`, `map_doc`, `registries`, `enabled_framework_roles`, `enabled_standard_flow_sets`, `enabled_project_roles`, `enabled_project_skills`, `enabled_shared_skills`, `enabled_project_profiles`, `enabled_project_flows`, `default_flow_set`, `validation`, `role_selection` |
| `agent_extensions.registries` | `roles`, `skills`, `profiles`, `flows` |
| `agent_extensions.validation` | `require_registry_files`, `require_unique_ids`, `require_framework_role_compatibility`, `require_skill_role_compatibility`, `require_profile_resolution`, `require_flow_resolution`, `fail_closed_on_validation_error` |
| `agent_extensions.role_selection` | `mode`, `fallback_role`, `conversation_modes` |
| `agent_extensions.role_selection.conversation_modes.<mode_id>` | `enabled`, `role`, `single_task_only`, `tracked_flow_entry`, `allow_freeform_chat` |
| `host_environment` | `cli_system`, `codex` |
| `host_environment.codex` | `agents` |
| `host_environment.codex.agents.<agent_id>` | `tier`, `rate`, `reasoning_band`, `model`, `model_reasoning_effort`, `sandbox_mode`, `default_runtime_role`, `runtime_roles`, `task_classes` |
| `autonomous_execution` | `next_task_boundary_analysis`, `next_task_boundary_report`, `next_task_boundary_report_gating`, `dependent_coverage_autoupdate`, `continue_after_reports`, `validation_report_required_before_implementation` |

Autonomous execution overlay rule:

1. `autonomous_execution` may tune next-task boundary behavior only within framework law.
2. It may disable user-facing boundary reporting, but it may not disable required internal next-task boundary analysis.
3. It may not convert a non-gating boundary report into silent scope widening.
4. Approval gating still belongs to `runtime-instructions/bridge.task-approval-loop-protocol`.
5. `continue_after_reports=true` means intermediate lawful reports should auto-advance into the next already-authorized step when no blocker, approval gate, validation gate, explicit user pause, or explicit user request to discuss the report exists.
6. `continue_after_reports` must not bypass research/spec/approval/verification sequencing; it only removes unnecessary stopping after a lawful intermediate report.
7. Pre-execution validation reports remain gating even when `continue_after_reports=true`.
8. `validation_report_required_before_implementation=true` inserts a mandatory validation-report gate before each implementation slice or implementation-bearing task.
9. Spec-ready transition into downstream implementation flow and post-validation continuation remain runtime-defined execution-entry behaviors, not supported project overlay keys.

| Agent-system schema path | Supported keys |
| --- | --- |
| `agent_system` | `init_on_boot`, `mode`, `state_owner`, `max_parallel_agents`, `workers`, `routing`, `scoring` |
| worker-level | `enabled`, `worker_backend_class`, `detect_command`, `role`, `orchestration_tier`, `cost_priority`, `max_runtime_seconds`, `min_output_bytes`, `models_hint`, `default_model`, `profiles`, `default_profile`, `capability_band`, `write_scope`, `billing_tier`, `budget_cost_units`, `speed_tier`, `quality_tier`, `specialties`, `dispatch`, `binary_path` |
| worker-level `dispatch` | `command`, `pre_static_args`, `subcommand`, `static_args`, `write_static_args`, `models_cache_path`, `workdir_flag`, `model_flag`, `output_mode`, `output_flag`, `prompt_mode`, `prompt_flag`, `web_search_mode`, `web_search_flag`, `web_probe_static_args`, `web_probe_prompt`, `web_probe_expect_substring`, `web_probe_timeout_seconds`, `env`, `probe_static_args`, `probe_prompt`, `probe_expect_substring`, `probe_timeout_seconds`, `startup_timeout_seconds`, `no_output_timeout_seconds`, `progress_idle_timeout_seconds`, `max_runtime_extension_seconds` |

Project agent-extension overlay rule:

1. `agent_extensions` is the project-owned activation and selection surface for project roles, project skills, project profiles, and project flow sets.
2. Framework-owned role law, role-profile law, and runtime routing law remain in `vida/config/instructions/**`.
3. Project-owned custom roles, custom skills, custom profiles, and custom flows must live in project-owned registries referenced by `agent_extensions.registries`.
4. `vida.config.yaml` may enable or disable framework roles and standard flow sets for the active project, but it must not silently redefine framework role authority.
5. Project extensions are lawful only after validation confirms:
   - registry files exist,
   - ids are unique,
   - project roles resolve to known framework base roles,
   - project profiles resolve to known roles and skills,
   - project flows resolve to known roles,
   - no project extension weakens framework safety boundaries.
6. `vida taskflow config validate` is the bounded runtime proof surface for the current overlay-level validation of `agent_extensions`.

Supported `agent_system.scoring` keys: `consecutive_failure_limit`, `promotion_score`, `demotion_score`, `probation_success_runs`, `probation_task_runs`, `retirement_failure_limit`.

Repeated-scalar encoding:

1. repeated-scalar fields may be expressed as CSV strings or YAML lists,
2. prefer YAML lists in new overlays and framework templates,
3. runtime helpers must accept both formats for backward compatibility.

Common repeated-scalar examples: worker `profiles`, worker `models_hint`, worker `capability_band`, worker `specialties`, route `workers`, route `fanout_workers`, worker `dispatch.static_args`, worker `dispatch.pre_static_args`, `framework_self_diagnosis.session_reflection_criteria`.

Supported routing-level keys: `workers`, `models`, `profiles`, `analysis_required`, `analysis_route_task_class`, `analysis_fanout_workers`, `analysis_fanout_min_results`, `analysis_merge_policy`, `analysis_external_first_required`, `analysis_receipt_required`, `analysis_zero_budget_required`, `analysis_default_in_boot`, `coach_required`, `coach_route_task_class`, `write_scope`, `verification_gate`, `max_runtime_seconds`, `min_output_bytes`, `fanout_workers`, `fanout_min_results`, `merge_policy`, `dispatch_required`, `external_first_required`, `web_search_required`, `local_execution_allowed`, `local_execution_preferred`, `cli_dispatch_required_if_delegating`, `direct_internal_bypass_forbidden`, `bridge_fallback_worker`, `internal_escalation_trigger`, `allowed_internal_reasons`, `verification_route_task_class`, `independent_verification_required`, `graph_strategy`, `deterministic_first`, `budget_policy`, `max_budget_units`, `max_rounds`, `max_stalls`, `max_resets`, `max_cli_worker_calls`, `max_coach_passes`, `max_verification_passes`, `max_fallback_hops`, `max_total_runtime_seconds`, `problem_party_required`, `problem_party_task_class`.

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

Current runtime artifacts for overlay activation: `.vida/state/worker-init.json`, `.vida/state/worker-scorecards.json`, `.vida/state/worker-strategy.json`.

These files are runtime state, not canonical project configuration.

## Verification

Minimum proof for overlay support:

```bash
vida taskflow config validate
vida taskflow config dump
vida taskflow config protocol-active agent_system
vida taskflow system snapshot [task_id]
```

Minimum proof for framework template support:

```bash
vida taskflow boot read-contract lean
vida taskflow boot snapshot --json
```

-----
artifact_path: config/runtime-instructions/project-overlay.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/bridge.project-overlay-protocol.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: 2026-07-03T12:05:00+03:00
changelog_ref: bridge.project-overlay-protocol.changelog.jsonl
protocol_authoring_gate: enforced
protocol_compression_status: audit_passed
protocol_compression_algorithm: table-normalization+rfc2119-cleanup+protected-atom-validation
protocol_compression_baseline_ref: 062a45c3d:vida/config/instructions/runtime-instructions/bridge.project-overlay-protocol.md
protocol_compression_audit_at: 2026-07-03T12:05:00+03:00
protocol_compression_before_tokens: 3659
protocol_compression_after_tokens: 3410
protocol_compression_content_sha256: 29b40d6e19f59543e07e4493467c8caf6407206f20ae28d53ea11d6a39eb7fa9
