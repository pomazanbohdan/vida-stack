# VIDA Coder Service Mode Executor Contract

Status: active product contract

Use this contract as the bounded service-mode executor contract for `vida coder`, provider-backed execution, service orchestration, guarded tools, session state, and receipt-backed TaskFlow execution.

## Summary
- Contract: add `vida coder` as a first-class VIDA subcommand and service-capable bounded executor backed by Rig, VIDA runtime tools, provider auth/model selection, guarded file/MCP tools, session state, multi-project service orchestration, and receipt-backed TaskFlow execution.
- Owner layer: `mixed`
- Runtime surface: `launcher | taskflow | agent-init | status | service | external/provider runtime | mcp`
- Status: active product contract

## Current Context
- VIDA already separates runtime roles from execution carriers and resolves carrier/model/profile selection from `vida.config.yaml`, TaskFlow state, runtime assignment, readiness, score, cost, and task-class constraints.
- Current internal and external execution paths include host subagents, external CLI carriers, `vida-pi-agent`, guarded owned-path validation, activation-vs-execution evidence checks, and receipt-backed dispatch parsing.
- The proposed `vida-coder` is not a second orchestrator. It is a selected executor backend that may automatically run `vida agent-init` for its assigned lane and then execute exactly one bounded packet.
- Future runtime goals include session continuity, multi-threaded scheduling, multi-project execution, and service-mode operation where a resident VIDA service launches configured project flows and executor lanes.
- Rig can provide the inner LLM agent/tool loop, provider clients, structured output, and MCP-compatible tooling, but VIDA must continue to own packet routing, runtime assignment, model/profile policy, write-scope law, and completion receipts.
- The current gap is a native VIDA-owned executor surface that can run inside the product/service boundary instead of relying only on the active host development environment or external standalone CLI agents.

## Goal
- Provide `vida coder` as a first-class bounded executor command and service-executable backend.
- Preserve VIDA orchestration authority: TaskFlow and the service/orchestrator select the flow, project, packet, runtime role, carrier/backend, model profile, and proof target.
- Let `vida-coder` automatically bootstrap its lane with `vida agent-init`, build a compact VIDA runtime knowledge pack, run one Rig-backed agent loop, use typed VIDA/file/MCP tools, validate touched paths, run verification, and emit a canonical receipt.
- Support both interactive CLI use and resident service use with the same execution contract.
- Support session continuity without letting a session become a hidden orchestrator or bypass packet ownership.
- Support multi-threaded service execution only across disjoint sessions/projects/packets whose TaskFlow scheduling semantics and conflict domains allow it.
- Support multi-project operation by requiring every execution to bind to an explicit project root, state dir, session id, packet id, and ownership claim.
- Out of scope:
  - Replacing `vida orchestrator-init` or TaskFlow dispatch policy with Rig.
  - Letting `vida-coder` call `vida orchestrator-init`, `vida agent dispatch-next`, or `vida task next-lawful` during normal packet execution.
  - Exposing arbitrary shell or arbitrary `vida` commands to the model.
  - Treating MCP tools as safe without classifier and allowlist policy.
  - Closing epics/tasks from the executor without orchestrator/service synthesis.

## Requirements

### Functional Requirements
- `vida coder run`
  - Must execute one bounded packet selected by TaskFlow/service/orchestrator.
  - Must run adapter-owned automatic `agent-init` before the Rig loop.
  - Must fail closed if the lane is root/orchestrator-shaped, packet data is missing, runtime role mismatches, write-scope metadata is missing for write mode, or activation is view-only without an execution packet.
- `vida coder capabilities`
  - Must report provider support, tool support, MCP support, write-guard support, session support, service support, and known disabled features.
- `vida coder provider-check`
  - Must validate command/runtime availability, auth posture, selected provider/model compatibility, and readiness blocker codes without sending secrets to prompts.
- `vida service`
  - Must be able to launch `vida coder run` for project-configured flows with explicit project root, state dir, session id, packet id, runtime role, selected backend, and selected model profile.
  - Must preserve one packet per executor process or one isolated executor session per packet when embedded execution is later introduced.
- Session support
  - Must store session identity and packet execution state separately from TaskFlow ownership.
  - Must allow resume only for the same project root, packet id, runtime role, backend id, model profile, and owned/read-only scope.
  - Must not let previous conversation/session state widen the current packet.
- Multi-threading support
  - Must allow concurrent executor workers only when TaskFlow scheduling marks work as parallel-safe and conflict domains do not overlap.
  - Must serialize tool calls inside one `vida-coder` packet by default.
  - Must prevent nested self-dispatch and recursive agent spawning.
- Multi-project support
  - Must run each project in an isolated project context with explicit root/state/runtime paths.
  - Must keep provider auth and service session data scoped by project/user policy.
  - Must not let a service worker infer project root from ambient cwd when the launch request includes an explicit root.
- VIDA runtime tools
  - Must expose typed tools such as `vida_current_packet`, `vida_task_status`, `vida_protocol_view`, `vida_record_evidence`, `vida_report_blocker`, and `vida_run_verification`.
  - Must not expose `vida_any_command` in normal execution.
- File tools
  - Must expose read/search and guarded patch tools.
  - Must validate canonical paths, symlink escapes, absolute path escapes, parent traversal, and touched-path reporting.
- MCP tools
  - Must be routed through an MCP policy gateway that classifies tools before exposure.
  - Must allow only configured read/evidence/file-write tools and block shell/network/mutating tools unless the packet explicitly grants them.

### Non-Functional Requirements
- Performance
  - Service launch must avoid repeated heavy runtime discovery where a valid project/session cache exists.
  - One-packet execution must remain bounded by max runtime and idle timeouts from runtime config.
- Scalability
  - Service scheduler must support multiple projects and workers without sharing mutable task/session state unsafely.
  - Worker pools must be capped by project policy, global service policy, provider rate limits, and TaskFlow parallelism.
- Observability
  - Every execution must write structured lifecycle events, tool audit records, provider/model metadata, usage when available, touched paths, verification result, and blocker codes.
- Security
  - Secrets must never be injected into prompts or receipts.
  - Provider auth must resolve through env/secret references or configured auth profiles.
  - MCP/tool descriptions must be treated as untrusted input and filtered before model exposure.
  - Raw shell is disabled by default.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/vida-coder-service-mode-executor-contract.md`
  - `docs/product/spec/current-spec-map.md`
  - `active spec/catalog maps and Git history`
  - `docs/process/external-cli-carrier-operator-procedure.md`
  - `docs/process/agent-system.md`
- Framework protocols affected:
  - `instruction-contracts/lane.worker-dispatch-protocol`
  - `instruction-contracts/core.agent-system-protocol`
  - `instruction-contracts/overlay.session-context-continuity-protocol`
- Runtime families affected:
  - launcher
  - taskflow
  - agent-init
  - service runtime
  - status/readiness
  - provider auth/runtime tools
- Config / receipts / runtime surfaces affected:
  - `vida.config.yaml -> agent_system.subagents.vida_coder`
  - `vida.config.yaml -> host_environment.systems.vida_coder`
  - `vida.config.yaml -> service`
  - `vida coder run --json`
  - `vida coder capabilities --json`
  - `vida coder provider-check --json`
  - `vida status --json`
  - `vida taskflow consume agent-system --json`
  - execution receipts under `.vida/data/state`

## Design Decisions

### 1. First-Class VIDA Subcommand, Separate Crate Boundary
Will implement / choose:
- Add `vida coder ...` as a native `vida` subcommand.
- Keep Rig/provider/tool implementation in `crates/vida-coder` and reusable typed runtime tools in `crates/vida-runtime-tools`.
- Keep `crates/vida` as thin command wiring and runtime integration.
- Why: the operator/service gets one VIDA binary, one runtime version, one project-root resolver, and one receipt/status ecosystem while heavy provider dependencies remain isolated.
- Trade-offs: release binary size and dependency complexity increase when the coder feature is enabled.
- Alternatives considered: standalone `vida-coder-agent` binary only; rejected as the primary form because it duplicates runtime context transfer and makes service integration harder.
- ADR link if this must become a durable decision record: later ADR for `vida-coder` crate boundary if feature flags/release packaging require a permanent policy.

### 2. Agent-Init-Owned Executor Bootstrap
Will implement / choose:
- `vida coder run` automatically runs `vida agent-init` or equivalent internal lane bootstrap for its assigned packet.
- The Rig model cannot call this bootstrap tool directly.
- Why: `agent-init` is lane activation/runtime contract loading, not orchestration.
- Trade-offs: service/operator launch must provide enough packet identity to keep `agent-init` scoped.
- Alternatives considered: exposing `agent-init` as a model-callable tool; rejected because it lets the model retry/rebind outside the adapter contract.
- ADR link if needed: none yet.

### 3. Service Scheduler Owns Multi-Threading, Coder Owns One Packet
Will implement / choose:
- `vida service` may run multiple `vida-coder` executors concurrently across projects or parallel-safe packets.
- Each `vida-coder` instance remains single-packet and serializes model tool calls by default.
- Why: multi-threaded throughput belongs at the service scheduler and TaskFlow conflict-domain layer, not inside one model loop.
- Trade-offs: service scheduler needs claims/leases, worker accounting, provider rate limiting, and project isolation.
- Alternatives considered: one `vida-coder` process internally managing many tasks; rejected because it blurs packet ownership and session boundaries.
- ADR link if needed: later service scheduler ADR.

### 4. Typed VIDA Tools Instead Of Raw Commands
Will implement / choose:
- Expose only narrow structured tools to the Rig agent.
- Keep arbitrary `vida` and shell commands unavailable during normal execution.
- Why: the executor needs VIDA context, not orchestration authority.
- Trade-offs: every new capability needs a typed tool and tests.
- Alternatives considered: allowlisted raw CLI wrapper; deferred to diagnostic-only mode because it is harder to audit.
- ADR link if needed: none yet.

### 5. MCP Via Policy Gateway
Will implement / choose:
- MCP tools are ingested, classified, allowlisted, and wrapped before model exposure.
- Why: MCP tool names/descriptions and remote servers are not enough to establish VIDA write authority.
- Trade-offs: useful MCP tools may be unavailable until classified.
- Alternatives considered: direct Rig `rmcp_tools` exposure; rejected for normal executor mode.
- ADR link if needed: later MCP policy ADR.

## Technical Design

### Core Components
- `crates/vida`
  - CLI wiring for `vida coder`, `vida service`, status/readiness projections, and feature-gated release integration.
- `crates/vida-coder`
  - Rig agent factory.
  - Provider factory and auth resolver.
  - Automatic agent-init bootstrap.
  - Runtime knowledge pack builder.
  - Coder session loader/saver.
  - Receipt builder.
- `crates/vida-runtime-tools`
  - Typed VIDA runtime tools.
  - Guarded file tools.
  - Verification command runner.
  - MCP policy gateway.
  - Tool audit envelope.
- `vida service`
  - Resident scheduler.
  - Project registry.
  - Session/claim manager.
  - Worker pool and provider rate-limit accounting.
  - Flow launcher that consumes project-configured dev-team flows and dispatches bounded lanes.
- `ServiceProjectRuntime`
  - Holds project root, state dir, vida binary fingerprint, config hash, active sessions, and worker leases.
- `CoderSession`
  - Holds session id, project id, packet id, runtime role, backend id, model profile id, owned/read-only paths, tool policy hash, provider auth profile ref, and resume state.

### Data / State Model
- `VidaRuntimeKnowledgePack`
  - `project_root`
  - `state_dir`
  - `session_id`
  - `packet_id`
  - `task_id`
  - `runtime_role`
  - `selected_backend_id`
  - `selected_model_provider`
  - `selected_model_ref`
  - `selected_model_profile_id`
  - `selected_reasoning_effort`
  - `owned_paths`
  - `read_only_paths`
  - `allowed_tools`
  - `stop_rules`
  - `verification_target`
  - `receipt_contract`
- `CoderReceipt`
  - `status`
  - `packet_id`
  - `session_id`
  - `provider`
  - `model_ref`
  - `tool_audit`
  - `touched_paths`
  - `verification`
  - `blockers`
  - `raw_provider`
  - `handoff_summary`
- `ServiceWorkerLease`
  - `project_id`
  - `task_id`
  - `packet_id`
  - `conflict_domain`
  - `parallel_group`
  - `expires_at`
  - `heartbeat_at`
- Compatibility notes:
  - External CLI mode and embedded mode must emit the same receipt schema.
  - Session resume must validate the knowledge-pack hash and packet identity before use.
  - Provider auth profiles must be references only; no secret value is persisted in receipts.

### Integration Points
- APIs:
  - `vida coder run --dispatch-packet <path> --json`
  - `vida coder run --packet-id <id> --runtime-role <role> --json`
  - `vida coder capabilities --json`
  - `vida coder provider-check --model <provider/model> --json`
  - `vida service start --json`
  - `vida service status --json`
  - `vida service project add|remove|status --json`
- Runtime-family handoffs:
  - TaskFlow creates/binds packet.
  - Service scheduler claims runnable lane.
  - `vida coder run` bootstraps with agent-init.
  - Coder emits receipt.
  - Orchestrator/service synthesizes next lane or closure.
- Cross-document / cross-protocol dependencies:
  - Agent system docs must describe `vida_coder` as service-capable executor backend.
  - External carrier procedure must cover provider auth/model readiness for `vida-coder` when running outside the current host environment.
  - Session continuity protocol must define resume boundaries for model conversation state.

### Bounded File Set
- Expected design/spec/doc files:
  - `docs/product/spec/vida-coder-service-mode-executor-contract.md`
  - `docs/product/spec/current-spec-map.md`
  - `active spec/catalog maps and Git history`
  - `docs/process/agent-system.md`
  - `docs/process/external-cli-carrier-operator-procedure.md`
- Expected runtime/config files:
  - `Cargo.toml`
  - `crates/vida/src/cli.rs`
  - `crates/vida/src/lib.rs`
  - `crates/vida-coder/**`
  - `crates/vida-runtime-tools/**`
  - `crates/vida/src/status_surface.rs`
  - `crates/vida/src/init_surfaces.rs`
  - `crates/vida/src/runtime_dispatch_execution.rs`
  - `crates/vida/src/carrier_runtime_projection.rs`
  - `crates/vida/src/agent_dispatch_surface.rs`
  - `vida.config.yaml`
  - `docs/framework/templates/vida.config.yaml.template`
- Expected service/session files:
  - `crates/vida/src/service_*`
  - `.vida/data/state/service/**`
  - `.vida/data/state/coder-sessions/**`

## Fail-Closed Constraints
- `vida-coder` must not run `vida orchestrator-init` during normal executor operation.
- `vida-coder` must not call dispatch-next, next-lawful, or task selection surfaces.
- `vida-coder` must not self-dispatch or spawn nested agents.
- `vida-coder` must not self-close tasks or epics.
- `vida-coder` must not expose arbitrary shell or arbitrary `vida` commands to the model.
- Write mode must require packet-owned paths and active guard validation.
- A returned receipt is invalid if touched paths are missing, unparseable, or outside scope.
- MCP tools are forbidden unless classified and allowlisted.
- Session resume is forbidden if project root, packet id, runtime role, backend id, model profile, owned paths, or tool policy differs.
- Service parallelism is forbidden when TaskFlow scheduling semantics, claims, or conflict-domain evidence are missing.

## Implementation Plan

### Phase 1
- Create TaskFlow epic and spec-pack task.
- Keep this contract finalized through DocFlow.
- Add `vida coder capabilities --json` and skeleton `vida coder run --json` surfaces with no provider calls.
- Add runtime tests proving `vida coder run` rejects orchestrator-shaped/no-packet invocations.
- First proof target:
  - `cargo test -p vida coder_ -- --nocapture`
  - `vida coder capabilities --json`

### Phase 2
- Add `crates/vida-coder` with Rig provider factory and structured result builder.
- Add automatic agent-init bootstrap and `VidaRuntimeKnowledgePack`.
- Add typed VIDA runtime tools and guarded read/search/patch tools.
- Add provider auth resolver and provider-check surface.
- Add config backend `agent_system.subagents.vida_coder`.
- Second proof target:
  - targeted unit tests for model selection/auth readiness
  - out-of-scope write rejection tests
  - agent-init bootstrap packet validation tests

### Phase 3
- Add service scheduler integration, project registry, session/claim manager, and worker pool.
- Add multi-project service status/readiness.
- Add MCP policy gateway with read-only tool support first, then guarded file-write support.
- Add dispatch/status/readiness projections and release/install packaging.
- Final proof target:
  - service starts and reports configured projects
  - scheduler rejects overlapping conflict domains
  - two disjoint project/packet executions can run concurrently
  - coder receipts are accepted by TaskFlow dispatch parsing

## Validation / Proof
- Unit tests:
  - CLI parser for `vida coder`.
  - Knowledge pack validation.
  - Scope guard canonicalization and symlink escape rejection.
  - Session resume identity mismatch rejection.
  - MCP classifier allow/deny cases.
- Integration tests:
  - `vida coder capabilities --json`
  - `vida coder provider-check --model <provider/model> --json`
  - `vida coder run --dispatch-packet <fixture> --json`
  - service scheduler claim/lease tests.
- Runtime checks:
  - `vida orchestrator-init --json`
  - `vida taskflow consume agent-system --json`
  - `vida status --json`
  - `vida service status --json`
- Canonical checks:
  - `activation-check`
  - `protocol-coverage-check`
  - `check`
  - `doctor`

## Observability
- Logging points:
  - service worker claim acquired/released
  - coder session created/resumed/rejected
  - agent-init bootstrap start/end
  - provider client selected
  - tool call start/end/blocked
  - patch/touched-path validation
  - verification start/end
  - receipt persisted
- Metrics / counters:
  - active projects
  - active coder sessions
  - active service workers
  - provider request count and failures
  - tool calls by class
  - blocked tool attempts
  - out-of-scope write attempts
  - verification pass/fail
- Receipts / runtime state written:
  - coder execution receipt
  - service worker lease
  - tool audit record
  - provider readiness record
  - session resume record
  - blocker report

## Rollout Strategy
- Development rollout:
  - Start with disabled-by-default `vida coder` feature in config.
  - Enable `capabilities` and `provider-check` before write execution.
  - Enable read-only/spec packets before write-capable packets.
  - Enable guarded writes only after scope tests and receipt parsing are green.
- Migration / compatibility notes:
  - Existing internal/external carriers remain unchanged.
  - `vida_coder` is a new backend, not a replacement for `internal_subagents`.
  - Service mode must support projects that do not enable `vida_coder`.
- Operator or user restart / restart-notice requirements:
  - Enabling service mode requires restarting the resident VIDA service.
  - Provider auth changes require service readiness refresh.
  - Binary release enabling coder may require refreshed PATH/install proof on Windows.

## Future Considerations
- Embedded in-process executor mode after process-per-packet mode is stable.
- Durable ADR for provider auth profile storage.
- Durable ADR for service scheduler lease model.
- Streaming UI/TUI for service-managed coder sessions.
- Provider usage budgeting and spend ledger.
- Remote project workers and remote MCP servers.
- Tool evaluation harness for unsafe MCP/tool prompt-injection attempts.
- Long-term memory support that remains packet-scoped and never overrides TaskFlow authority.

## References
- Related specs:
  - `docs/product/spec/pi-primary-environment-and-agent-carrier-design.md`
  - `docs/product/spec/taskflow-execution-semantics-and-scheduler-design.md`
  - `docs/product/spec/multi-orchestrator-session-ownership-and-claims-design.md`
- Related protocols:
  - `docs/process/agent-system.md`
  - `docs/process/team-development-and-orchestration-protocol.md`
  - `docs/process/external-cli-carrier-operator-procedure.md`
  - `docs/process/documentation-tooling-map.md`
- Related ADRs:
  - none yet
- External references:
  - Rig core: `https://docs.rs/rig-core/latest/rig_core/`
  - Rig AgentBuilder: `https://docs.rs/rig-core/latest/rig/agent/struct.AgentBuilder.html`
  - MCP authorization: `https://modelcontextprotocol.io/specification/draft/basic/authorization`

-----
artifact_path: product/spec/vida-coder-service-mode-executor-contract
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-06-03'
schema_version: '1'
status: canonical
source_path: docs/product/spec/vida-coder-service-mode-executor-contract.md
created_at: '2026-06-03T16:20:00+03:00'
updated_at: '2026-06-03T16:20:00+03:00'
changelog_ref: vida-coder-service-mode-executor-contract.changelog.jsonl
