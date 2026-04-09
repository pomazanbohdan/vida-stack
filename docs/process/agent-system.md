# Agent System

Project activation owns host CLI agent-template selection and runtime admission.

- default framework agent templates become available only after the selected host CLI template is materialized
- the current supported host CLI systems are config-driven under `vida.config.yaml -> host_environment.systems`
- framework template inventory may include multiple built-in host systems, but the active host-system list is owned by `vida.config.yaml -> host_environment.systems`
- carrier metadata is owned by `vida.config.yaml -> host_environment.systems.<system>.carriers` (Codex additionally keeps `host_environment.codex.agents` as tier catalog source)
- host CLI execution posture is owned by `vida.config.yaml -> host_environment.systems.<system>.execution_class` so internal vs external runtime handling does not depend on vendor id heuristics
- canonical runtime outputs are `carrier_runtime` and `runtime_assignment`
- `codex_multi_agent` and `codex_runtime_assignment` are compatibility aliases only and must not be treated as owner-law canonical fields
- the canonical executor registry is `vida.config.yaml -> agent_system.subagents`
- dispatch aliases are owned by the configured registry path under `vida.config.yaml -> agent_extensions.registries.dispatch_aliases` and are not the primary project-visible agent model
- route policy is owned by explicit executor fields such as `executor_backend`, `fanout_executor_backends`, and `fallback_executor_backend`
- legacy `subagents`, `fanout_subagents`, and `bridge_fallback_subagent` fields are compatibility aliases only
- host posture is the primary runtime materialization and admission context, not a hard gate that can veto an explicit policy-selected backend class
- hybrid runtime means a host may lawfully select both internal and external backends when route policy allows it
- internal backends remain internal-only even in a hybrid runtime; `internal_subagents` does not acquire an external CLI dispatch contract
- selected host runtime surface (for example the configured `runtime_root` under `host_environment.systems.<system>`) is rendered/runtime materialized output, not the owner of tier/rate/task-class policy
- project activation materializes host templates using the configured `materialization_mode` per system
- runtime chooses the cheapest capable configured carrier tier that still satisfies the local score guard from `.vida/state/worker-strategy.json`
- project-local agent extensions remain under `.vida/project/agent-extensions/`
- research, specification, planning, implementation, and verification packets should all route through the agent system once a bounded packet exists
- delegated worker dispatch still materializes through `vida agent-init` until the operator-surface promotion slice closes
- project "agent-first" development therefore means `vida agent-init`-backed delegated lanes first; host-tool-specific subagent APIs are optional carrier mechanics, not the canonical project execution contract
- host-local shell/edit capability is an executor affordance only and must not be interpreted as lawful root-session write ownership
- if the selected host execution class is internal, optional external CLI subagents remain auxiliary carrier details and must not redefine the whole session as externally gated by default
- if the selected host execution class is internal, external CLI backends may still be admissible when route policy explicitly selects them
- patch localization, runtime-defect diagnosis, or other read-only findings feed the next delegated packet and do not transfer write ownership back to the root session
- for hybrid runtimes, `vida status --json` must report the effective mixed posture honestly instead of implying that external execution is impossible from an internal host
- for external CLI setups, `vida status --json` reports `host_agents.external_cli_preflight`; when sandbox is active and network is unavailable, preflight fails closed with actionable next steps

-----
artifact_path: process/agent-system
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-04-04'
schema_version: '1'
status: canonical
source_path: docs/process/agent-system.md
created_at: '2026-04-04T20:24:09+03:00'
updated_at: 2026-04-09T05:43:45.435083526Z
changelog_ref: agent-system.changelog.jsonl
