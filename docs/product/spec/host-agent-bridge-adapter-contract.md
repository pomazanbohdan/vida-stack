# Host Agent Bridge Adapter Contract

Status: active product contract

Use this contract as the carrier-neutral contract for host-mediated internal agent execution across Codex App/CLI host APIs, Claude Code, Pi sub-agent plugins, Vibe Kanban, OpenCode, and future host runtimes. Vendor-specific launch mechanics are adapter capabilities, not VIDA runtime law.

## Summary

- Contract: generalize `host_tool_bridge` into a host-agent bridge adapter contract.
- Owner layer: `mixed`
- Runtime surface: `launcher | taskflow | agent-init | lane | status`
- Status: active product contract

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
  operations:
    spawn: <configured-parent-host-spawn-operation>
    wait: <configured-parent-host-wait-operation>
    dispose: <configured-parent-host-dispose-operation>
  dispose_policy: configured | unavailable
  adapter_required: true
  no_adapter_policy: fail_closed_emit_request
  result_dir: .vida/data/state/host-tool-bridge/results
  receipt_dir: .vida/data/state/host-tool-bridge/receipts
```

### FR-1A: Adapter command recipe authority and schema

The master template is the authoritative catalog for host-bridge command options,
capabilities, placeholders, and route/command combinations:

- master template: `docs/framework/templates/vida.config.yaml.template`
- command schema: `vida/config/schemas/host_tool_bridge_adapter_command.schema.json`
- project config: `vida.config.yaml` may select or override only fields declared by
  that template/schema; it must not add an executable, subcommand, flag, or
  placeholder form outside the catalog.

An active host system may declare the optional command recipe below. The recipe is
an argv plan for invoking a configured parent-host adapter; it is not a process
carrier fallback and it is not execution evidence by itself.

```yaml
host_tool_bridge:
  adapter_command_schema_ref: vida/config/schemas/host_tool_bridge_adapter_command.schema.json
  adapter_command:
    executable: <project-selected-executable>
    subcommands: [<zero-or-more-project-selected-tokens>]
    args: [<project-selected-token>, "{{request_path}}"]
```

Schema invariants:

1. `executable` is a non-empty, single-line scalar and is placeholder-free.
2. `subcommands` is a sequence of non-empty, single-line, placeholder-free
   tokens; an empty sequence is admissible.
3. `args` is a non-empty sequence of non-empty, single-line tokens containing
   exactly one non-empty `{{...}}` request placeholder occurrence. The
   placeholder may be embedded in the argument token selected by the project;
   the runtime replaces that one occurrence with the request path.
4. Unknown fields, empty tokens, NUL/newline-bearing tokens, missing placeholders,
   multiple placeholders, and unterminated/empty placeholders are invalid and
   fail closed before invocation.
5. The template's machine-readable
   `host_tool_bridge.adapter_command_contract.admissible_route_command_combinations`
   matrix is exhaustive. This document explains the matrix but does not create a
   second option authority.

### Route/command admissibility matrix

| Matrix state | Dispatch transport | Execution boundary | Adapter command | Expected state | Fail closed |
| --- | --- | --- | --- | --- | --- |
| `configured_valid` | `host_tool_bridge` | `parent_host_session` | valid map | invoke configured adapter command | no |
| `missing` | `host_tool_bridge` | `parent_host_session` | absent | emit host-bridge request | yes |
| `invalid_missing_executable` | `host_tool_bridge` | `parent_host_session` | malformed | reject invalid adapter command | yes |
| `invalid_empty_token` | `host_tool_bridge` | `parent_host_session` | malformed | reject invalid adapter command | yes |
| `invalid_multiline_token` | `host_tool_bridge` | `parent_host_session` | malformed | reject invalid adapter command | yes |
| `invalid_missing_placeholder` | `host_tool_bridge` | `parent_host_session` | malformed | reject invalid adapter command | yes |
| `invalid_multiple_placeholders` | `host_tool_bridge` | `parent_host_session` | malformed | reject invalid adapter command | yes |
| `invalid_mixed_route` | `host_tool_bridge` | `child_process` | process-style/mixed | reject mixed route | yes |
| `not_applicable` | `codex_cli_exec` (or another explicitly configured process transport) | `child_process` | not used | invoke explicit process backend | no |

The matrix means a host-bridge route never silently falls back to a process
carrier. A process carrier is admissible only when its route explicitly declares
the child-process boundary and process transport. Conversely, a child-process
route may not borrow a host-bridge command recipe or claim parent-host receipt
authority.

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
  "adapter_operations": {
    "adapter_kind": "<configured-adapter-kind>",
    "adapter_capability_id": "<configured-capability-id>",
    "invocation_mode": "<configured-invocation-mode>",
    "dispatch_transport": "<configured-dispatch-transport>",
    "receipt_mode": "<configured-receipt-mode>",
    "operations": {
      "spawn": "<configured-parent-host-spawn-operation>",
      "wait": "<configured-parent-host-wait-operation>",
      "dispose": "<configured-parent-host-dispose-operation>"
    },
    "dispose_policy": "configured | unavailable"
  },
  "adapter_contract_snapshot": "<canonical-resolved-registry-object>",
  "adapter_contract_hash": "<config-digest-for-replay>",
  "adapter_contract_source": "<config-or-registry-path>",
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
3. Resolve and invoke the configured lifecycle operations in order: `operations.spawn`, then `operations.wait`, then `operations.dispose` when `dispose_policy: configured`. If disposal is unavailable, the registry must declare `dispose_policy: unavailable`; the adapter must not invent or infer a dispose operation.
4. Write `host_tool_bridge_result` to `result_path` and `host_tool_bridge_receipt` to `receipt_path`.
5. Call `vida lane complete <run-id> --receipt-id <id> --host-bridge-request <request_path> --host-agent-id <id> --host-bridge-summary <summary> --json`.

The adapter is parent-session code, not a child process launched by `vida.exe`. VIDA may emit the request and validate completion, but the host adapter owns native host-tool invocation because those tools are not available inside the binary process.

When a request resolves a configured parent-host adapter contract and
`dispatch_transport` matches that contract, VIDA must also emit a
`host_bridge_auto_invocation` scaffold. The scaffold is not execution evidence;
it is the canonical, machine-readable parent-host adapter plan that allows the
host integration to auto-invoke the registry-resolved lifecycle operations
without manual parent orchestration or shell interpolation. The scaffold must
include request, packet, result, and receipt paths plus the required result fields:
`decision`, `verdict`, `blocker_codes`, `rework_target`, and
`allowed_next_node`.

Before invoking host tools, the adapter or operator can normalize and validate a
pending request with:

```powershell
vida agent host-bridge --request <request_path> --json
```

The command is read-only. It returns the registry-resolved lifecycle operation
sequence, the expected result/receipt artifact paths, capacity blocker
vocabulary, and the canonical `vida lane complete ... --host-bridge-request ...`
command. It must not write completion artifacts or claim execution by itself.

After the parent host adapter has executed the host agent, it can complete the
same request through VIDA validation with:

```powershell
vida agent host-bridge --request <request_path> --complete --host-agent-id <id> --summary <summary> --json
```

`--complete` delegates the mutation to `vida lane complete`; it is not a second
state writer.

### Validation matrix

| ID | Scenario | Required evidence | Expected result |
| --- | --- | --- | --- |
| Z | Empty registry | no adapter identity or lifecycle mapping | typed missing-config blocker; no invented defaults |
| O | One configured operation | canonical registry operation map | request preserves the configured operation verbatim |
| M | Multiple configured operations | spawn/wait and optional dispose policy | scaffold sequence follows registry order |
| B | Blocked/missing capability | adapter-required policy + no usable contract | request emitted; receipt remains unbacked |
| I | Invalid schema | malformed identity, operation, or dispose policy | typed contract error; no dispatch |
| E | External adapter | explicit configured process carrier | process boundary remains separate from host bridge |
| S | Snapshot replay | canonical snapshot, source, and config digest | replay can compare hash before invoking |
| R | Receipt validation | result + receipt path and identity match | completion accepted only with validated evidence |
| P | Parallel lanes | distinct request/packet/result/receipt paths | no cross-lane artifact collision |
| C | Config change | changed registry operation or digest | stale request is refreshed or blocked closed |

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
  "decision": "approve",
  "verdict": "pass",
  "blocker_codes": [],
  "rework_target": null,
  "allowed_next_node": "next",
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

Minimum non-pass result:

```json
{
  "artifact_kind": "host_tool_bridge_result",
  "schema_version": 1,
  "request_id": "...",
  "run_id": "...",
  "dispatch_target": "implementer",
  "backend_id": "internal_subagents",
  "carrier_id": "junior",
  "execution_state": "blocked",
  "decision": "rework",
  "verdict": "blocked",
  "blocker_codes": [
    "host_agent_capacity_unavailable"
  ],
  "rework_target": "host_agent_bridge_adapter",
  "allowed_next_node": "reclaim_or_retry_host_bridge_request",
  "host_agent_id": null,
  "summary": "Host agent capacity is unavailable; reclaim completed host handles or retry when capacity is available.",
  "next_actions": [
    "Reclaim or close completed host-agent handles, then retry the same host bridge request."
  ]
}
```

If the adapter cannot start a host agent, it must write blocked artifacts with:

- `execution_state: "blocked"`
- `decision: "rework"` or another non-pass decision that prevents lane completion
- `verdict: "blocked"` or the precise non-pass verdict
- `blocker_codes` containing one or more of `host_agent_capacity_unavailable`, `host_tool_capability_missing`, or `host_agent_execution_failed`
- `rework_target` naming the adapter, request, host capability, or process-carrier configuration that must be repaired
- `allowed_next_node` naming only the next rework-safe node, such as reclaiming, retrying, repairing the adapter, or selecting an explicit process carrier
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
artifact_path: product/spec/host-agent-bridge-adapter-contract
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-06-01
schema_version: 1
status: canonical
source_path: docs/product/spec/host-agent-bridge-adapter-contract.md
created_at: 2026-06-01T11:40:00+03:00
updated_at: 2026-06-01T11:40:00+03:00
changelog_ref: host-agent-bridge-adapter-contract.changelog.jsonl
