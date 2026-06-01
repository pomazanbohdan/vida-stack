# Codex Host Agent Boundary And CLI Bridge Design

Status: proposed

Use this design as the bounded Codex adapter slice of the generic host-agent bridge contract. The carrier-neutral owner design is `docs/product/spec/host-agent-bridge-adapter-contract-design.md`; this document applies it to Codex App/CLI surfaces and separates true Codex host/internal agents from process-based `codex exec` execution.

## Summary
- Feature / change: make `internal_subagents` a host-mediated internal-agent backend and move `codex exec` to an explicit process-based carrier.
- Owner layer: `mixed`
- Runtime surface: `launcher | taskflow | agent-init | status`
- Status: proposed

## Current Context
- `vida.config.yaml -> host_environment.systems.codex.execution_class` is internal.
- The same configured system previously carried `dispatch.command: codex` with `exec --json --enable multi_agent`.
- That made `internal_subagents` look internal in routing/status while the runtime actually launched a child `codex exec` process.
- A child `vida.exe` process cannot call the parent Codex host session tools such as `multi_agent_v1.spawn_agent`, `multi_agent_v1.wait_agent`, or `multi_agent_v1.close_agent`.

## Goal
- `internal_subagents` means host-mediated internal agents.
- `codex exec` means an explicit child-process carrier such as `codex_cli_exec`.
- Runtime surfaces expose the execution boundary, dispatch transport, and receipt mode.
- Missing host bridge support fails closed by emitting a host-tool bridge request instead of silently launching `codex exec`.

## Requirements

### Functional Requirements
- `internal_subagents` must declare:
  - `execution_boundary: parent_host_session`
  - `dispatch_transport: host_tool_bridge`
  - `receipt_mode: host_bridge_receipt`
- `codex exec` must live under a distinct process carrier, initially `codex_cli_exec`.
- `vida agent-init` must not synthesize `std::process::Command` for `internal_subagents`.
- When no host bridge adapter is available, runtime must return a blocked bridge request with `activation_view_is_execution_evidence=false`.
- The host bridge request must include request/result/receipt paths, packet path, run id, dispatch target, backend id, carrier id, owned paths, and proof target when present.
- The request must also expose generic adapter capability fields such as `adapter_kind`, `adapter_capability_id`, and `invocation_mode` so future host environments do not depend on Codex-specific field names.

### Non-Functional Requirements
- No extra process hop is allowed for true internal host agents.
- Process carriers remain available only through explicit route policy or fallback.
- Status, route, and dispatch diagnostics must make the boundary visible.

## Design Decisions

### 1. Internal Means Parent Host Session
Will implement:
- Treat `internal_subagents` as a parent-host-session backend.
- Emit `host_tool_bridge` requests for parent/app adapters.
- Why: the parent Codex host has access to internal agent tools; a child process does not.

### 2. Codex CLI Exec Is A Process Carrier
Will implement:
- Add `codex_cli_exec` with `subagent_backend_class: external_cli`, `execution_boundary: child_process`, and `dispatch_transport: codex_cli_exec`.
- Keep it disabled by default until explicitly selected.
- Why: `codex exec` is useful automation, but it is not the same execution boundary as host internal agents.

### 3. Fail Closed Without Adapter
Will implement:
- `vida.exe` emits `host_tool_bridge_request` and blocks when the adapter is missing.
- Only a host/app adapter completion can create receipt-backed execution evidence.
- Why: activation views and bridge requests are not execution receipts.

## Technical Design

### Core Components
- Config:
  - `vida.config.yaml`
  - `docs/framework/templates/vida.config.yaml.template`
- Runtime:
  - `crates/vida/src/runtime_dispatch_execution.rs`
  - `crates/vida/src/runtime_dispatch_state.rs`
  - `crates/vida/src/status_surface_host_agents.rs`
- Proof:
  - `crates/vida/tests/project_routing_shape.rs`

### Data / State Model
Host bridge request shape:

```json
{
  "schema_version": 1,
  "status": "pending",
  "request_id": "...",
  "run_id": "...",
  "task_id": "...",
  "dispatch_target": "...",
  "packet_path": "...",
  "runtime_role": "...",
  "task_class": "...",
  "backend_id": "internal_subagents",
  "carrier_id": "junior",
  "execution_boundary": "parent_host_session",
  "dispatch_transport": "host_tool_bridge",
  "adapter_kind": "codex_host_tools",
  "adapter_capability_id": "codex.multi_agent_v1",
  "invocation_mode": "parent_host_tool_api",
  "spawn_tool": "multi_agent_v1.spawn_agent",
  "wait_tool": "multi_agent_v1.wait_agent",
  "close_tool": "multi_agent_v1.close_agent",
  "request_path": "...",
  "result_path": "...",
  "receipt_path": "..."
}
```

## Fail-Closed Constraints
- Do not launch `codex exec` for `internal_subagents`.
- Do not treat `bridge_request_pending` as execution evidence.
- Do not mark receipt-backed completion until a host/app adapter writes a validated result.
- Do not widen root-session write authority from bridge failure.

## Implementation Plan

### Phase 1
- Add config/template boundary fields.
- Add `codex_cli_exec` as a disabled process carrier.
- Add runtime/status boundary diagnostics.
- Add fail-closed bridge request emission.

### Phase 2
- Add a host-bridge completion command or adapter API.
- Add fake-adapter tests for receipt-backed completion.

### Phase 3
- Wire a real Codex host/app adapter that calls internal host tools.
- Retire compatibility tests that assert internal `codex exec`.

## Validation / Proof
- Unit tests:
  - internal host bridge does not build `codex exec` args.
  - `codex_cli_exec` remains the only `codex exec` process carrier.
- Integration tests:
  - `vida status --json` exposes execution boundary and dispatch transport.
  - bridge request pending is not receipt-backed execution.
- Runtime checks:
  - `vida taskflow route explain --task-class implementation --runtime-role worker --json`
  - `vida status --json`

## References
- `docs/process/agent-system.md`
- `docs/process/codex-agent-configuration-guide.md`
- `docs/product/spec/host-agent-bridge-adapter-contract-design.md`
- `docs/product/spec/internal-codex-agent-execution-fail-closed-design.md`
- `docs/product/spec/release-1-carrier-neutral-runtime-and-host-materialization-design.md`
- OpenAI Codex CLI non-interactive mode: `https://developers.openai.com/codex/noninteractive`
- OpenAI Codex subagents: `https://developers.openai.com/codex/concepts/subagents`

-----
artifact_path: product/spec/codex-host-agent-boundary-and-cli-bridge-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-06-01
schema_version: 1
status: canonical
source_path: docs/product/spec/codex-host-agent-boundary-and-cli-bridge-design.md
created_at: 2026-06-01T10:00:00+03:00
updated_at: 2026-06-01T10:00:00+03:00
changelog_ref: codex-host-agent-boundary-and-cli-bridge-design.changelog.jsonl
