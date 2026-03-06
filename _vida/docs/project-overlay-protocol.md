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
5. `providers`
6. `routing`
7. `scoring`

Supported provider-level keys:

1. `enabled`
2. `provider_class`
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
16. `speed_tier`
17. `quality_tier`
18. `specialties`
19. `dispatch`

Supported provider-level `dispatch` keys:

1. `command`
2. `static_args`
3. `workdir_flag`
4. `model_flag`
5. `output_mode`
6. `output_flag`
7. `prompt_mode`
8. `prompt_flag`

Repeated-scalar encoding:

1. repeated-scalar fields may be expressed as CSV strings or YAML lists,
2. prefer YAML lists in new overlays and framework templates,
3. runtime helpers must accept both formats for backward compatibility.

Common repeated-scalar examples:

1. provider `profiles`
2. provider `models_hint`
3. provider `capability_band`
4. provider `specialties`
5. route `providers`
6. route `fanout_providers`
7. provider `dispatch.static_args`

Supported routing-level keys:

1. `providers`
2. `models`
3. `profiles`
4. `write_scope`
5. `verification_gate`
6. `max_runtime_seconds`
7. `min_output_bytes`
8. `fanout_providers`
9. `fanout_min_results`
10. `merge_policy`
11. `dispatch_required`
12. `external_first_required`
13. `bridge_fallback_provider`
14. `internal_escalation_trigger`

Validation scope:

1. required top-level sections and required fields inside them,
2. unsupported keys in canonical sections,
3. type checks for booleans, integers, strings, mappings, and repeated-string fields,
4. provider `dispatch` requirements for enabled `external_cli` providers,
5. route/provider consistency checks such as `default_profile in profiles` and `fanout_min_results <= fanout_providers`.

## Portability Rule

Framework scripts must treat missing overlay as a valid state.

Portable default behavior:

1. no project overlay -> no project-specific bundle activation,
2. no project overlay -> framework still executes using generic protocols only.

## Enforcement Rule

Overlay may configure:

1. which provider classes are allowed,
2. which routing preferences are preferred,
3. which escalation thresholds apply,
4. external-first routing preference for eligible read-only classes,
5. which provider is the canonical bridge fallback before internal escalation.

Overlay may not configure:

1. permission to bypass `br` as SSOT,
2. permission to bypass verification gates,
3. permission to let external providers mutate framework task state directly.

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
