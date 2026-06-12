# Agent System

Project activation owns host CLI agent-template selection and runtime admission.

## Boundary And Routing

- this document is the project process surface for agent-system posture; it is not a bootstrap router, framework owner protocol, or product/runtime spec
- stable runtime contracts belong in `docs/product/spec/**` and executable/config truth belongs in `vida.config.yaml` plus runtime-owned config artifacts
- role, skill, profile, flow, carrier, and host-system values are config/runtime evidence, not hardcoded process law
- process docs may route operators to the correct owner surface, but must not duplicate carrier admission, closure authority, or TaskFlow receipt rules

- default framework agent templates become available only after the selected host CLI template is materialized
- the current supported host CLI systems are config-driven under `vida.config.yaml -> host_environment.systems`
- framework template inventory may include multiple built-in host systems, but the active host-system list is owned by `vida.config.yaml -> host_environment.systems`
- carrier metadata is owned by `vida.config.yaml -> host_environment.systems.<system>.carriers`; compatibility projections such as `host_environment.codex.agents` may exist but must not become a second canonical source
- host CLI execution posture is owned by `vida.config.yaml -> host_environment.systems.<system>.execution_class` so internal vs external runtime handling does not depend on vendor id heuristics
- canonical runtime outputs are `carrier_runtime` and `runtime_assignment`
- `codex_multi_agent` and `codex_runtime_assignment` are compatibility aliases only and must not be treated as owner-law canonical fields
- the canonical executor registry is `vida.config.yaml -> agent_system.subagents`
- the canonical development-team contract is `vida.config.yaml -> dev_team`, which defines config-selected flow ids, work-item flow bindings, ordered role steps, role/task-class bindings, command templates, lifecycle hooks, proof gates, approval pauses, and fail-closed validation posture without replacing `agent_extensions`
- dispatch aliases are owned by the configured registry path under `vida.config.yaml -> agent_extensions.registries.dispatch_aliases` and are not the primary project-visible agent model
- route policy is owned by explicit executor fields such as `executor_backend`, `fanout_executor_backends`, and `fallback_executor_backend`
- legacy `subagents`, `fanout_subagents`, and `bridge_fallback_subagent` fields are compatibility aliases only
- host posture is the primary runtime materialization and admission context, not a hard gate that can veto an explicit policy-selected backend class
- hybrid runtime means a host may lawfully select both internal and external backends when route policy allows it
- internal backends remain internal-only even in a hybrid runtime; `internal_subagents` does not acquire an external CLI dispatch contract
- selected host runtime surface (for example the configured `runtime_root` under `host_environment.systems.<system>`) is rendered/runtime materialized output, not the owner of tier/rate/task-class policy
- Pi host files under `.pi/**` are rendered host affordance projections from VIDA config/runtime truth, not a source of carrier, model-profile, write-scope, closure, or delegation authority
- Pi projected agents must carry no-recursion, no-self-dispatch, and no-closure-authority semantics; canonical delegated execution remains TaskFlow/`vida agent-init` with receipt-backed runtime assignment
- project activation materializes host templates using the configured `materialization_mode` per system
- runtime chooses the cheapest capable configured carrier tier that still satisfies the local score guard from `.vida/state/worker-strategy.json`
- local score guard evidence comes from orchestrator-classified attempts; low
  scores from timeout, shutdown, empty artifact, missing telemetry, or
  false-green validation should narrow or escalate the next packet, but do not
  by themselves close, fail, or mutate the current TaskFlow item
- project-local agent extensions remain under `.vida/project/agent-extensions/`
- research, specification, planning, implementation, and verification packets should all route through the agent system once a bounded packet exists
- delegated worker dispatch still materializes through `vida agent-init` until the operator-surface promotion slice closes
- for internal host-agent postures, runtime may emit a host-tool bridge request for the configured parent/app adapter capability; Codex host tools, Claude Code subagents, Pi sub-agent plugins, Vibe Kanban agents, OpenCode subagents, and future adapters are selected by config capability, not by vendor-id hardcoding
- when `autonomous_execution.agent_only_development=true`, a current VIDA dispatch packet or host-tool bridge request may identify an admissible configured host adapter for a bounded lane, but repository policy, runtime state, or config defaults do not by themselves satisfy host-tool explicit subagent/delegation permission requirements
- host-tool API restrictions that require explicit subagent/delegation permission remain a separate host/user approval boundary; spawn-capable host adapters must require an explicit user request or host approval surface authorizing that host subagent/delegation path for the bounded work
- any approved host-adapter permission is path-, run-, role-, and receipt-scoped; it does not weaken `vida agent-init` authority, TaskFlow binding, exception takeover, receipt-backed closure rules, or the host tool's own approval contract
- non-interactive process execution such as `codex exec` is a separate process carrier such as `codex_cli_exec`, not the implementation of `internal_subagents`
- project "agent-first" development therefore means `vida agent-init`-backed delegated lanes first; host-tool-specific subagent APIs are optional carrier mechanics, not the canonical project execution contract
- host-local shell/edit capability is an executor affordance only and must not be interpreted as lawful root-session write ownership
- if the selected host execution class is internal, optional external CLI subagents remain auxiliary carrier details and must not redefine the whole session as externally gated by default
- if the selected host execution class is internal, external CLI backends may still be admissible when route policy explicitly selects them
- patch localization, runtime-defect diagnosis, or other read-only findings feed the next delegated packet and do not transfer write ownership back to the root session
- for hybrid runtimes, `vida status --json` must report the effective mixed posture honestly instead of implying that external execution is impossible from an internal host
- for external CLI setups, `vida status --json` reports `host_agents.external_cli_preflight`; when sandbox is active and network is unavailable, preflight fails closed with actionable next steps
- external CLI readiness must distinguish transport/tool-contract pass from carrier-specific auth/model readiness; when project config declares dispatch-level model pinning, ambient carrier-local model drift must not silently redefine execution
- when `host_agents.external_cli_preflight.blocked_primary_backends` includes the route-primary backend, runtime must fail closed away from that carrier and rebind to the explicit `fallback_executor_backend` instead of launching a known-blocked external carrier again

-----
artifact_path: process/agent-system
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-04-04'
schema_version: '1'
status: canonical
source_path: docs/process/agent-system.md
created_at: '2026-04-04T20:24:09+03:00'
updated_at: 2026-06-13T00:00:00+03:00
changelog_ref: agent-system.changelog.jsonl
