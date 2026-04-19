# Agent System

Project activation owns host CLI agent-template selection and runtime admission.

- default framework host templates become available only after the selected host CLI template is materialized
- supported and active host CLI systems are config-driven under `vida.config.yaml -> host_environment.systems`
- framework template inventory may be broader than the enabled active list in project config
- carrier metadata is owned by `vida.config.yaml -> host_environment.systems.<system>.carriers` (Codex additionally keeps `vida.config.yaml -> host_environment.codex.agents` as the rendered tier-catalog source)
- dispatch aliases are owned by the configured registry path under `vida.config.yaml -> agent_extensions.registries.dispatch_aliases` and are not the primary project-visible agent model
- the selected runtime surface is rendered under the configured runtime root and is not the owner of tier/rate/task-class policy
- project activation materializes the selected host template using the configured `materialization_mode`; Codex renders the configured TOML catalog root, while external CLI systems use their configured runtime roots
- runtime chooses the cheapest capable configured carrier tier that still satisfies the local score guard from `.vida/state/worker-strategy.json`
- project-local agent extensions remain under `.vida/project/agent-extensions/`
- research, specification, planning, implementation, and verification packets should all route through the agent system once a bounded packet exists
- project "agent-first" development means the delegated lane flow through `vida agent-init`; host-tool-specific subagent APIs are optional carrier mechanics and not the canonical execution contract
- host-local shell/edit capability is an executor affordance only and must not be interpreted as lawful root-session write ownership
- when the selected host execution class is internal, optional external CLI subagents remain auxiliary carrier details and do not make the whole session externally gated by default
- patch localization, runtime-defect diagnosis, or other read-only findings feed the next delegated packet and do not transfer write ownership back to the root session

-----
artifact_path: process/agent-system
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-04-04'
schema_version: '1'
status: scaffold
source_path: docs/process/agent-system.md
created_at: '2026-04-04T00:00:00Z'
updated_at: '2026-04-04T00:00:00Z'
changelog_ref: agent-system.changelog.jsonl
