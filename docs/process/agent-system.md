# Agent System

Project activation owns host CLI agent-template selection and runtime admission.

- default framework agent templates become available only after the selected host CLI template is materialized
- the current supported host CLI systems are config-driven under `vida.config.yaml -> host_environment.systems`
- current built-in templates include `codex`, `qwen`, `kilo`, and `opencode`
- carrier metadata is owned by `vida.config.yaml -> host_environment.systems.<system>.carriers` (Codex additionally keeps `host_environment.codex.agents` as tier catalog source)
- dispatch aliases are owned by the configured registry path under `vida.config.yaml -> agent_extensions.registries.dispatch_aliases` and are not the primary project-visible agent model
- selected host runtime surface (for example `.codex/**`, `.qwen/**`, `.kilo/**`, `.opencode/**`) is rendered/runtime materialized output, not the owner of tier/rate/task-class policy
- project activation materializes host templates using the configured `materialization_mode` per system
- runtime chooses the cheapest capable configured carrier tier that still satisfies the local score guard from `.vida/state/worker-strategy.json`
- project-local agent extensions remain under `.vida/project/agent-extensions/`
- research, specification, planning, implementation, and verification packets should all route through the agent system once a bounded packet exists
- for external CLI setups, `vida status --json` reports `host_agents.external_cli_preflight`; when sandbox is active and network is unavailable, preflight fails closed with actionable next steps
