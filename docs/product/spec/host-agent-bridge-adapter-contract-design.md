# Host Agent Bridge Adapter Contract Design

Status: proposed

Use this design as the carrier-neutral contract for host-mediated internal agent execution across Codex App/CLI host APIs, Claude Code, Pi sub-agent plugins, Vibe Kanban, OpenCode, and future host runtimes. Vendor-specific launch mechanics are adapter capabilities, not VIDA runtime law.

## Summary

- Feature / change: generalize `host_tool_bridge` into a host-agent bridge adapter contract.
- Owner layer: `mixed`
- Runtime surface: `launcher | taskflow | agent-init | lane | status`
- Status: proposed

## Problem

`internal_subagents` previously drifted toward a Codex-specific meaning. That is not stable enough for VIDA because the active host environment can differ by project and machine:

- Codex App or Codex CLI may expose internal host subagent tools in the parent session.
- Codex `exec` is a process execution mode and must remain an explicit process carrier.
- Claude Code supports subagents and background subagent sessions with Claude-specific configuration and transcript behavior.
- Pi can expose sub-agents through a plugin/extension model.
- Vibe Kanban can orchestrate multiple installed coding agents and agent profiles.
- OpenCode supports primary agents and subagents through JSON/Markdown configuration and task permissions.
- Future hosts may expose similar capabilities through MCP, plugins, app-server APIs, or local adapters.

VIDA must not hardcode one vendor as the definition of "internal agent".

## Core Rule

VIDA owns routing, law, packets, receipts, and closure.

Host environments own only their adapter affordance:

```text
VIDA packet + selected backend
  -> host_agent_bridge adapter request
  -> parent host invokes native agent/subagent capability
  -> adapter writes result + receipt
  -> VIDA validates receipt-backed evidence
  -> TaskFlow continues
```

An adapter request is not execution evidence. Only validated result and receipt artifacts may close a delegated lane.

## Requirements

### FR-1: Capability-based adapter registry

Config must describe host-agent bridges by capability rather than vendor id.

Required fields:

```yaml
host_environment:
  host_agent_bridge_contract:
    schema_version: 1
    selection_mode: configured_host_capability
    receipt_required: true
    no_adapter_policy: fail_closed_emit_request
    process_carrier_requires_explicit_backend: true
    supported_adapter_kinds:
      - codex_host_tools
      - codex_cli_subagents
      - claude_code_subagents
      - pi_sub_agent_plugin
      - vibe_kanban_agents
      - opencode_subagents
      - custom_host_agent_adapter
```

Each active host system may then declare:

```yaml
host_tool_bridge:
  adapter_kind: codex_host_tools
  adapter_capability_id: codex.multi_agent_v1
  invocation_mode: parent_host_tool_api
  dispatch_transport: host_tool_bridge
  receipt_mode: host_bridge_receipt
  adapter_required: true
  no_adapter_policy: fail_closed_emit_request
  result_dir: .vida/data/state/host-tool-bridge/results
  receipt_dir: .vida/data/state/host-tool-bridge/receipts
```

### FR-2: Separate internal host adapters from process carriers

Process execution such as `codex exec`, `claude --agent`, `opencode run`, `pi`, or `vibe` may be useful, but it is not automatically an internal host-agent bridge. If process execution is used, it must be modeled as an explicit backend/carrier with:

- `execution_boundary: child_process`
- `dispatch_transport` named for the process adapter
- auth/model/readiness checks
- receipt-backed result contract

### FR-3: Vendor-neutral request shape

Host bridge requests must include the generic fields needed by any adapter:

```json
{
  "schema_version": 1,
  "status": "pending",
  "request_id": "...",
  "run_id": "...",
  "dispatch_target": "implementer",
  "packet_path": "...",
  "runtime_role": "worker",
  "task_class": "implementation",
  "backend_id": "internal_subagents",
  "carrier_id": "junior",
  "execution_boundary": "parent_host_session",
  "dispatch_transport": "host_tool_bridge",
  "adapter_kind": "codex_host_tools",
  "adapter_capability_id": "codex.multi_agent_v1",
  "invocation_mode": "parent_host_tool_api",
  "request_path": "...",
  "result_path": "...",
  "receipt_path": "..."
}
```

Vendor-specific fields are allowed only under `adapter_params`.

### FR-4: Adapter completion surface

The host adapter must write:

- a result artifact,
- a receipt artifact,
- enough host-agent identity and proof summary to audit the lane.

VIDA then validates and records the result through a runtime-owned command such as:

```powershell
vida lane complete <run-id> --receipt-id <id> --host-bridge-request <path> --host-agent-id <id> --host-bridge-summary <text> --json
```

### FR-5: Fail-closed unsupported environments

If the current environment does not expose the configured adapter capability, VIDA must:

- emit a host bridge request,
- report `host_tool_bridge_adapter_required`,
- keep `receipt_backed=false`,
- keep root-local write blocked,
- recommend a configured adapter or explicit process carrier, never a silent vendor fallback.

### FR-6: Executable parent-host adapter loop

`vida agent-init --dispatch-packet <path> --execute-dispatch --json` may only execute
`internal_subagents` through a parent-host adapter. The adapter loop is:

1. Read `host_tool_bridge_request.request_path`.
2. Verify `adapter_kind`, `adapter_capability_id`, `packet_path`, `run_id`, `dispatch_target`, `backend_id`, `carrier_id`, and declared write scope.
3. Invoke the configured host capability. For `codex_host_tools`, this means `multi_agent_v1.spawn_agent`, then `multi_agent_v1.wait_agent`, then `multi_agent_v1.close_agent`.
4. Write `host_tool_bridge_result` to `result_path` and `host_tool_bridge_receipt` to `receipt_path`.
5. Call `vida lane complete <run-id> --receipt-id <id> --host-bridge-request <request_path> --host-agent-id <id> --host-bridge-summary <summary> --json`.

The adapter is parent-session code, not a child process launched by `vida.exe`. VIDA may emit the request and validate completion, but the host adapter owns native host-tool invocation because those tools are not available inside the binary process.

Before invoking host tools, the adapter or operator can normalize and validate a
pending request with:

```powershell
vida agent host-bridge --request <request_path> --json
```

The command is read-only. It returns the required `multi_agent_v1.spawn_agent`,
`multi_agent_v1.wait_agent`, and `multi_agent_v1.close_agent` sequence, the
expected result/receipt artifact paths, capacity blocker vocabulary, and the
canonical `vida lane complete ... --host-bridge-request ...` command. It must not
write completion artifacts or claim execution by itself.

After the parent host adapter has executed the host agent, it can complete the
same request through VIDA validation with:

```powershell
vida agent host-bridge --request <request_path> --complete --host-agent-id <id> --summary <summary> --json
```

`--complete` delegates the mutation to `vida lane complete`; it is not a second
state writer.

Minimum successful result:

```json
{
  "artifact_kind": "host_tool_bridge_result",
  "schema_version": 1,
  "request_id": "...",
  "run_id": "...",
  "dispatch_target": "implementer",
  "backend_id": "internal_subagents",
  "carrier_id": "junior",
  "execution_state": "executed",
  "host_agent_id": "...",
  "summary": "..."
}
```

Minimum successful receipt:

```json
{
  "artifact_kind": "host_tool_bridge_receipt",
  "schema_version": 1,
  "request_id": "...",
  "run_id": "...",
  "dispatch_target": "implementer",
  "receipt_id": "...",
  "receipt_backed": true,
  "host_agent_id": "...",
  "adapter_kind": "codex_host_tools",
  "adapter_capability_id": "codex.multi_agent_v1"
}
```

If the adapter cannot start a host agent, it must write blocked artifacts with:

- `execution_state: "blocked"`
- `blocker_code: "host_agent_capacity_unavailable" | "host_tool_capability_missing" | "host_agent_execution_failed"`
- `next_actions` with an exact reclaim, retry, repair, or explicit process-carrier command.

Thread limits and unavailable host capabilities are capacity/readiness blockers, not routing success and not `internal_codex_carrier_unavailable`.

### FR-7: Prelaunch classification consistency

Internal host bridge transport must not be prelaunch-classified as `internal_codex_carrier_unavailable`.

- `dispatch_transport: host_tool_bridge`, or an internal host system with no explicit process transport, must reach the bridge request path and report `host_tool_bridge_adapter_required` / `bridge_request_pending` until receipt-backed completion exists.
- `internal_codex_carrier_unavailable` is reserved for explicit process-carrier paths such as `dispatch_transport: codex_cli_exec` that cannot provide receipt-backed completion and have no admissible fallback.
- Resume/recovery surfaces must preserve the underlying blocker but must not obscure the bridge request path or suggest an impossible direct binary launch.

## Adapter Classes

| Adapter kind | Expected boundary | Notes |
| --- | --- | --- |
| `codex_host_tools` | parent host session | Uses parent Codex host tools such as `multi_agent_v1.spawn_agent`, `wait_agent`, and `close_agent`. |
| `codex_cli_subagents` | parent host session or CLI session capability | Allowed only when the running Codex CLI exposes native subagent APIs to the parent session. Non-interactive `codex exec` remains a process carrier. |
| `claude_code_subagents` | host session capability | Claude Code subagents are Claude-configured agents; use only through a host adapter that can observe completion and receipt evidence. |
| `pi_sub_agent_plugin` | host plugin capability | Requires installed Pi sub-agent plugin or extension and must inherit parent tool restrictions. |
| `vibe_kanban_agents` | orchestrator service capability | Vibe Kanban can select different agent profiles; bridge receipts must cite the selected workspace/attempt/profile. |
| `opencode_subagents` | host session capability | OpenCode primary/subagent modes and task permissions map to adapter params; receipt-backed completion is still required. |
| `custom_host_agent_adapter` | explicit adapter | Project or enterprise adapter with declared request/result/receipt schema. |

## Non-Goals

- Do not embed vendor-specific orchestration law in VIDA.
- Do not treat any UI-visible child agent as proof without a VIDA receipt.
- Do not use `codex exec` as the implementation of `internal_subagents`.
- Do not route to unavailable plugins or CLIs just because their config example exists.
- Do not let agent profile config replace TaskFlow role/task-class/write-scope law.

## Validation / Proof

- `cargo test -p vida internal_host_tool_bridge_transport_does_not_require_codex_exec_dispatch -- --nocapture`
- `cargo test -p vida internal_host_bridge_transport_defers_to_host_tool_bridge_request_path -- --nocapture`
- `cargo test -p vida host_bridge_adapter -- --nocapture`
- `cargo test -p vida internal_host_bridge_receipt_mode_counts_as_receipt_backed_completion_support -- --nocapture`
- `cargo test -p vida lane_complete_records_host_bridge_result_and_receipt_evidence -- --nocapture --test-threads=1`
- `vida taskflow route explain --task-class implementation --runtime-role worker --json`
- `vida status --json`

## External References

- OpenAI Codex CLI repository: `https://github.com/openai/codex`
- Vibe Kanban supported coding agents: `https://www.vibekanban.com/docs/supported-coding-agents`
- Vibe Kanban agent profiles: `https://vibekanban.com/docs/settings/agent-configurations`
- Claude Code subagents: `https://code.claude.com/docs/en/sub-agents`
- Pi sub-agent package: `https://pi.dev/packages/pi-sub-agent`
- Pi documentation: `https://pi.dev/docs/latest`
- OpenCode agents: `https://opencode.ai/docs/agents/`

-----
artifact_path: product/spec/host-agent-bridge-adapter-contract-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-06-01
schema_version: 1
status: canonical
source_path: docs/product/spec/host-agent-bridge-adapter-contract-design.md
created_at: 2026-06-01T11:40:00+03:00
updated_at: 2026-06-01T11:40:00+03:00
changelog_ref: host-agent-bridge-adapter-contract-design.changelog.jsonl
