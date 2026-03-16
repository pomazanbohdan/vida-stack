# Agent System

Project activation owns host CLI agent-template selection and runtime admission.

- default framework agent templates become available only after the selected host CLI template is materialized
- the current supported host CLI system list is framework-owned; the current supported value is `codex`
- Codex carrier-tier metadata is owned by `vida.config.yaml -> host_environment.codex.agents`
- dispatch aliases are owned by the configured registry path under `vida.config.yaml -> agent_extensions.registries.dispatch_aliases` and are not the primary project-visible agent model
- `.codex/**` is the rendered host executor surface, not the owner of tier/rate/task-class policy
- project activation materializes the carrier tiers into `.codex/config.toml` and `.codex/agents/*.toml`
- runtime chooses the cheapest capable configured carrier tier that still satisfies the local score guard from `.vida/state/worker-strategy.json`
- project-local agent extensions remain under `.vida/project/agent-extensions/`
- research, specification, planning, implementation, and verification packets should all route through the agent system once a bounded packet exists
