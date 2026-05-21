# Vida Service Tui Wizard Architecture Research

Purpose: capture the current research and architecture decisions for the VIDA project activation wizard, Ratatui operator TUI, and future VIDA service/daemon control plane before implementation work begins.

## Status

This is a research artifact, not final product law. It records the current direction and the decisions made during the May 2026 planning discussion so follow-up implementation can start from a stable context instead of repeating discovery.

Current conclusion:

1. Fix session identity and session-scoped continuation before starting TUI or service work.
2. Keep VIDA service headless and make CLI/TUI clients of the same command envelope.
3. Use Ratatui for the local operator console, not as the service UI itself.
4. Treat the activation wizard as a UI over a shared activation core: inspect, plan, preview, apply, receipt, sync.
5. Support multi-project operation in one service by routing every request to a project-local DB/state root.

## Research Capture Protocol

This document is the active research log for the TUI, wizard, and service architecture track.

Rules:

1. Every new decision, clarified assumption, external reference, codebase finding, or rejected option from planning discussion should be appended here before implementation starts.
2. Each update should preserve the distinction between research conclusion, product-law candidate, implementation precondition, and non-goal.
3. Runtime defects discovered during this planning track should be linked to the owning TaskFlow task or epic instead of being buried as prose only.
4. External ecosystem findings should include source links and a short fitness assessment for VIDA's actual needs.
5. Every edit must be finalized through `vida docflow finalize-edit` and validated with `vida docflow check-file`.

## Related Work Items

TaskFlow epic:

- `feature-multi-orchestrator-session-scoped-ownership-clai`
  - reopened after research found the previous multi-session closure was incomplete for pre-TUI/pre-service operation.

Precondition defect task:

- `fix-session-identity-scoped-continuation-before-tui-service`
  - scope: canonical `VIDA_SESSION_ID`, per-session fallback, session-scoped status/continuation/admission, claim-scoped blocker behavior.
  - explicit non-goals: no TUI, no service daemon, no service API transport, no dashboard.

## Research Sources

Project specs and process docs:

- `docs/product/spec/project-activation-and-configurator-model.md`
- `docs/product/spec/bootstrap-carriers-and-project-activator-model.md`
- `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
- `docs/product/spec/status-families-and-query-surface-model.md`
- `docs/product/spec/multi-orchestrator-session-ownership-and-claims-design.md`
- `docs/product/spec/authoritative-state-lock-recovery-design.md`
- `docs/process/agent-system.md`
- `docs/process/external-cli-carrier-operator-procedure.md`
- `docs/process/agent-extensions/README.md`

External TUI/service references:

- Ratatui documentation: `https://ratatui.rs/`
- Ratatui 0.30 release notes: `https://ratatui.rs/highlights/v030/`
- Ratatui component architecture: `https://ratatui.rs/concepts/application-patterns/component-architecture/`
- Ratatui backend comparison: `https://ratatui.rs/concepts/backends/comparison/`
- Ratatui snapshot testing: `https://ratatui.rs/recipes/testing/snapshots/`
- Ratzilla: `https://docs.rs/ratzilla/latest/ratzilla/`
- Textual: `https://textual.textualize.io/`
- OpenCode CLI docs: `https://dev.opencode.ai/docs/cli/`
- OpenCode sessions docs: `https://opencode.school/lessons/sessions/`
- Claude Code environment variables: `https://code.claude.com/docs/en/env-vars`

## Current Codebase State

Existing activation surface:

- `crates/vida/src/project_activator_surface.rs`
  - owns the current CLI/JSON project activation flow.
  - builds a project activator view, resolves answers, applies activation answers, writes sidecar/docs scaffolds, and writes activation receipts.
  - current mutation is direct; there is no separate reusable `inspect -> plan -> preview -> apply` core.

- `crates/vida/src/project_activator_activation_summary.rs`
  - computes activation pending status and required inputs.

- `crates/vida/src/project_activator_runtime_surface.rs`
  - reports the activation algorithm as bounded interview plus materialization.

- `crates/vida/src/project_activator_host_cli_summary.rs`
  - summarizes configured host CLI systems and template materialization posture.

Current activation CLI:

- `crates/vida/src/cli.rs`
  - `ProjectActivatorArgs` supports flat flags such as project id, project name, languages, host CLI system, repair, and JSON output.
  - no wizard/TUI entrypoint exists yet.

Current init/materialization:

- `crates/vida/src/init_surfaces.rs`
  - materializes `AGENTS.md`, `AGENTS.sidecar.md`, `vida.config.yaml`, docs scaffold, `.vida/**`, and runtime agent-extension projections.

Current host systems:

- `vida.config.yaml -> host_environment.systems`
  - active systems include `codex`, `hermes`, `opencode`, and `pi`.
  - carrier metadata is config-owned, not hardcoded host law.

Current TUI dependencies:

- `crates/vida/Cargo.toml`
  - no `ratatui`, `crossterm`, `inquire`, `dialoguer`, or equivalent prompt/TUI dependency is currently present.

## Ratatui Research Summary

Ratatui is a strong fit for a local VIDA operator console, especially for terminal and SSH workflows. It should be used as a UI client layer, not as the service runtime itself.

Recommended TUI stack:

| Need | Candidate | Current recommendation |
|---|---|---|
| Terminal UI core | `ratatui` | Primary choice |
| Terminal backend | `crossterm` | Primary backend for Windows/Linux/macOS |
| Component architecture | Ratatui templates/patterns | Use for screen/component organization |
| Prompt/form widgets | `tui-prompts` or small custom layer | Evaluate during implementation |
| Multiline editing | `tui-textarea-2` | Useful for YAML/docs snippets |
| Scroll/diff preview | `tui-scrollview` plus custom diff rows | Useful for plan/apply preview |
| File picker | `ratatui-explorer` | Optional for project/import selection |
| UI tests | Ratatui `TestBackend` plus snapshots | Required for deterministic UI proof |

Libraries not recommended for MVP:

- animation/effects libraries such as `tachyonfx`,
- image rendering such as `ratatui-image`,
- decorative text widgets such as `tui-big-text`.

Reason:

- The activation wizard and runtime console need dense operational UI, validation, diffs, logs, receipts, and stable keyboard workflows. Visual polish should not expand the MVP dependency surface.

## TUI Plus Dashboard Research Summary

No single Rust library should be treated as both the canonical terminal TUI and the web dashboard framework.

Options considered:

| Option | Assessment |
|---|---|
| Ratatui terminal dashboard | Strong for local/SSH operator control |
| Ratzilla / Ratatui in browser | Interesting experiment, not primary web dashboard path |
| egui-ratatui | Potential desktop/browser bridge, not core dependency |
| Textual | Strong combined terminal/web story, but Python and outside VIDA's Rust core |
| Separate web dashboard | Best long-term architecture if browser UI is required |

Decision:

- Build the TUI and browser dashboard as separate clients over the same VIDA service API/event stream.
- Do not make the TUI render path the canonical dashboard abstraction.

## Service / Daemon Architecture

VIDA service should be a headless local control plane.

Responsibilities:

- runtime state,
- config activation state,
- DB/filesystem sync,
- update/reconfigure engine,
- project registry,
- jobs,
- receipts,
- events,
- logs,
- claims/leases,
- session/request identity.

Non-responsibilities:

- terminal rendering,
- browser rendering,
- direct operator wizard UX,
- host-specific carrier identity heuristics in core.

Target split:

```text
vida-core
  config model
  activation model
  project registry model
  session identity
  claims/leases
  inspect/plan/apply
  sync/reconcile
  receipts/events

vida-service
  daemon process
  local API / IPC
  per-project DB routing
  job queue
  event stream
  locks/leases

vida-cli
  thin command client
  bootstrap/offline fallback

vida-tui
  Ratatui operator console
  wizard, diffs, logs, jobs, receipts
```

## CLI As Client

Long-term direction:

- most `vida ...` commands become client calls to the service,
- CLI resolves project and session identity,
- CLI sends a request envelope,
- service owns validation, locking, claims, DB routing, jobs, receipts, and events.

Direct/local mode remains only for:

- `vida init`,
- `vida service install/start/status`,
- service bootstrap,
- emergency/offline recovery,
- explicit diagnostic fallback.

## Command Envelope

All CLI, TUI, dashboard, and future host adapter requests should converge on one request envelope.

Proposed shape:

```json
{
  "session_id": "vida-session-id",
  "request_id": "uuid-or-ulid",
  "client_kind": "cli|tui|dashboard|service|codex|claude_code|opencode|hermes|pi",
  "project_id": "vida-stack",
  "project_root": "C:/project/vida-stack",
  "operation": "project.activation.plan",
  "claim_kind": "observe|shared_read|exclusive_write|dispatch|proof",
  "payload": {}
}
```

Every command that can mutate state must declare its intended claim shape. Read-only status and research commands should use `observe` or `shared_read`.

## Session Identity Decision

Canonical environment variable:

- `VIDA_SESSION_ID`

Backward-compatible alias:

- `VIDA_ORCHESTRATOR_SESSION_ID`

Known host aliases:

- `CLAUDE_CODE_SESSION_ID`
- `CLAUDE_CODE_REMOTE_SESSION_ID`
- `CODEX_SESSION_ID`
- `CODEX_THREAD_ID`

Resolver order:

1. explicit service/session attach id,
2. CLI `--session-id`,
3. `VIDA_SESSION_ID`,
4. `VIDA_ORCHESTRATOR_SESSION_ID`,
5. configured host alias candidates,
6. generated local session token.

Critical rule:

- The fallback token must be per live session/process/terminal instance.
- It must not be derived only from `project_root + state_dir`.
- `project_root + state_dir` identifies project context, not session identity.

## Multi-Project Service Model

The service may manage multiple projects, but every request must route to one project-local state root.

Conceptual registry:

```yaml
service:
  projects:
    vida-stack:
      root: C:/project/vida-stack
      state_dir: C:/project/vida-stack/.vida/data/state
      db_profile: project_local
      status: active
```

Rules:

1. One service can know many projects.
2. Each project keeps its own DB/state root.
3. TUI and CLI working in the same project must see the same project-local DB truth.
4. Cross-project operations must be explicit service-level operations, not accidental current-directory leakage.
5. Project registry writes should be service-owned, receipt-backed, and visible in TUI/CLI status.

## IPC / API Direction

Preferred local protocol stack:

1. Unix domain socket on Linux/macOS.
2. Named pipe on Windows.
3. Localhost HTTP fallback for debug or constrained environments.

Recommended transport format:

- JSON-RPC-like request/response envelope for commands,
- streaming JSON lines or SSE-style event stream for jobs/logs,
- opaque receipt ids for persisted operations.

Reason:

- VIDA needs debuggable local control plane behavior before it needs a heavy distributed API framework.

## Service Installation Direction

Initial installation should be user-level, not system-level.

Platforms:

| Platform | First target |
|---|---|
| Windows | Windows Service or user-level service wrapper, with admin/system install as later option |
| Linux | systemd user service first |
| macOS | launchd user agent first |

Rule:

- System-level installation should not be required for normal developer use.
- Project routing and service identity must be explicit before broad install automation starts.

## TUI Architecture

`vida tui` is an attachable operator console.

It should:

- connect to service,
- resolve or select active project,
- show status,
- run activation wizard,
- show config/skills/roles/profiles/flows,
- show agents/carriers,
- show sync/reconcile state,
- show jobs,
- stream logs/events,
- show receipts/history,
- request plan/apply through service.

It should not:

- write project files directly,
- own activation lifecycle semantics,
- duplicate CLI mutation logic,
- bypass claim/lease/session guards.

Primary screens:

1. Projects
2. Status
3. Activation Wizard
4. Config
5. Skills/Roles/Profiles/Flows
6. Agents/Carriers
7. Sync/Reconcile
8. Jobs
9. Receipts
10. Logs

## Wizard Architecture

The wizard is not a separate activator implementation. It is a UI over a shared activation core.

Target core:

```text
ProjectActivationCore
  inspect(project_root)
  build_plan(answers, mode)
  render_diff(plan)
  apply(plan)
  emit_receipt(result)
  sync_projection(result)
```

Wizard modes:

- activate new project,
- reconfigure existing project,
- update docs/agents/templates,
- repair drift,
- import/export config bundle.

Wizard steps:

1. Select or confirm project.
2. Inspect existing activation and project shape.
3. Collect project identity and language policy.
4. Select host CLI/runtime system.
5. Configure docs roots and bootstrap carriers.
6. Configure agent extensions and initial skills/roles/profiles/flows posture.
7. Build activation/update plan.
8. Preview config/docs/agents/sidecar diff.
9. Validate and show blockers.
10. Apply with progress stream.
11. Write receipt.
12. Show final status and restart/reload instructions if host template materialization requires it.

## Skills / Roles / Profiles / Flows Management

The TUI should eventually manage more than project activation.

Required operator views:

- skills inventory,
- roles inventory,
- profiles inventory,
- flow sets,
- agents/carriers,
- active/effective runtime config,
- import/export/update/replace/disable/restore actions,
- validation errors,
- dependency conflicts,
- receipts and history,
- sidecar/docs projection status.

Lifecycle operations should match the configurator model:

- import,
- activate,
- update,
- replace,
- disable,
- restore,
- export,
- reconcile.

## Additional Research Pass: Service And TUI Implementation Options

Date: 2026-05-21.

Research question:

- Before selecting the implementation path, verify which Rust ecosystem pieces should shape the service, IPC, installation, event stream, and TUI architecture.

Additional local codebase findings:

1. `crates/vida/Cargo.toml` currently has no service/TUI/HTTP/RPC dependencies. It has `tokio`, `clap`, `serde`, `serde_json`, `serde_yaml`, `surrealdb`, TaskFlow, and DocFlow dependencies.
2. `ProjectActivatorArgs` is currently flag-driven and direct CLI-owned: `state_dir`, `project_id`, `project_name`, language policy fields, `host_cli_system`, `repair`, and `json`.
3. `project_activator_surface.rs` already has substantial activation behavior, including project view construction, activation answer resolution, direct mutation, host template materialization, config/docs writes, and receipt creation.
4. Current session identity code still resolves `VIDA_ORCHESTRATOR_SESSION_ID`, `CODEX_SESSION_ID`, `CODEX_THREAD_ID`, then falls back to `stable_local_worktree_session_id`. This reinforces the precondition that session identity must be fixed before service/TUI write paths are built.
5. `operator_session_projection.rs` already separates current session evidence, foreign claims, blockers, conflicts, and global blockers. That shape should be preserved and made more precise, not replaced by a service-local ad hoc lock model.

Additional external ecosystem findings:

1. `interprocess` is the strongest low-level local IPC candidate for a Rust-local service client because it supports local sockets and abstracts platform differences across Unix-domain-style sockets and Windows named-pipe-backed local sockets.
2. `service-manager` is a useful cross-platform service installation/control candidate. It targets service manager surfaces such as systemd, launchd, Windows service, WinSW, OpenRC, and rc.d. It is an install/control helper, not the service runtime itself.
3. `windows-service` is a Windows-specific crate for implementing native Windows Service behavior. It can be used for the Windows service runtime path if VIDA needs direct SCM integration instead of relying on a wrapper.
4. `axum` is useful if VIDA exposes a localhost HTTP fallback or a later browser dashboard. It should not be the first local control-plane requirement if named pipes/local sockets are enough for CLI/TUI.
5. `jsonrpsee` is mature for JSON-RPC over HTTP/WebSocket-style transports. It is heavier than needed for the first local IPC path and may pull architecture toward transport/framework concerns before VIDA command semantics are stable.
6. `tarpc` is viable for Rust-to-Rust typed RPC, but it can hide VIDA's command envelope behind generated service traits. That is a mismatch if CLI, TUI, dashboard, and external host runtimes must share one explicit request/receipt model.
7. MCP's transport model is useful as a reference pattern: JSON-RPC messages with multiple transports. VIDA does not need to become MCP, but the same separation of protocol semantics from transport is relevant.
8. Ratatui remains the right terminal UI layer. Its component/event-loop patterns fit an operator console, but Ratatui should talk to a client trait/event stream and must not own activation semantics.

Source links:

- `https://docs.rs/interprocess/latest/interprocess/local_socket/`
- `https://docs.rs/service-manager/latest/service_manager/`
- `https://docs.rs/windows-service/latest/windows_service/`
- `https://docs.rs/axum/latest/axum/response/sse/`
- `https://docs.rs/jsonrpsee/latest/jsonrpsee/`
- `https://docs.rs/tarpc/latest/tarpc/`
- `https://modelcontextprotocol.io/specification/2025-06-18/basic/transports`
- `https://ratatui.rs/concepts/application-patterns/component-architecture/`
- `https://ratatui.rs/tutorials/counter-async-app/`

Implication:

- VIDA should not pick a large RPC framework as the architecture owner.
- The architecture owner should be the VIDA command envelope, request/session/claim model, activation core, and receipt/event model.
- IPC, HTTP, TUI, CLI, and dashboard should be adapters.

## Meta-Analysis: Implementation Strategy Options

Evaluation criteria:

1. Preserves session/request/claim correctness.
2. Avoids duplicate activation logic.
3. Allows wizard progress before the full daemon is complete.
4. Keeps CLI, TUI, and future dashboard on one semantic contract.
5. Supports multi-project service routing.
6. Keeps Windows/Linux/macOS service installation feasible.
7. Minimizes rewrite risk.

### Option A: Core-First Direct CLI/TUI

Shape:

- Extract `ProjectActivationCore`.
- CLI calls it directly.
- TUI calls it directly.
- Service is added later.

Strengths:

- Fastest route to a visible wizard.
- Lowest first-step dependency surface.
- Useful if the first goal is only project activation UX.

Weaknesses:

- TUI can accidentally become another direct writer.
- CLI/TUI/service will need a later adapter migration.
- Multi-session and multi-project rules may be bolted on late.
- Higher risk of duplicate lock/receipt/job logic.

Assessment:

- Acceptable only as a throwaway prototype or if TUI is read-only.
- Not recommended for production VIDA wizard/service architecture.

### Option B: Service-First Daemon

Shape:

- Build daemon, project registry, IPC, job queue, events, service install first.
- Convert CLI to service client.
- Build TUI only after service API exists.

Strengths:

- Strongest long-term ownership model.
- Good for multi-project and concurrent clients.
- Cleanest single-writer state posture.

Weaknesses:

- Large first implementation slice.
- TUI/wizard delivery waits on service plumbing.
- Risk of designing too much transport before activation semantics are refactored.
- Harder to validate user workflows early.

Assessment:

- Architecturally clean but too heavy as the next immediate path.
- Better as the second stage after core/envelope stabilization.

### Option C: Envelope/Core-First With Adapters

Shape:

1. Fix session identity and scoped continuation.
2. Extract activation core as `inspect -> plan -> diff -> validate -> apply -> receipt`.
3. Define `VidaCommandEnvelope`, `VidaCommandResponse`, `VidaEvent`, and `VidaReceiptRef`.
4. Add a local in-process adapter used by CLI.
5. Add a service IPC adapter later with the same envelope.
6. Build TUI against a `VidaClient` trait, not against direct files/DB.
7. Add service install and multi-project registry once the envelope is stable.

Strengths:

- Lets wizard development start before full daemon completion.
- Keeps the UI from owning mutation semantics.
- Makes CLI/TUI/service share the same command contract.
- Reduces rewrite risk because local and daemon modes share one client trait.
- Fits current codebase: existing CLI surfaces can be wrapped incrementally.

Weaknesses:

- Requires discipline to keep the local adapter from becoming permanent direct-write authority.
- Some job/event semantics must be modeled before daemon exists.
- Needs explicit tests proving local adapter and future service adapter semantics stay equivalent.

Assessment:

- Recommended path.
- This should replace the previous broad "hybrid staged path" wording.

### Option D: RPC-Framework-First

Shape:

- Choose `jsonrpsee`, `tarpc`, gRPC, or another RPC framework first.
- Define service methods around that framework.
- Generate or implement clients for CLI/TUI.

Strengths:

- Clear client/server structure.
- Can provide mature transport and tooling quickly.

Weaknesses:

- Framework can become the product protocol.
- Harder to preserve VIDA-specific command envelope, receipts, claims, and task/session semantics.
- May be overkill for local machine IPC.
- Less friendly to offline/local adapter and host-runtime adapters.

Detailed rationale:

1. VIDA's primary protocol concept is not a remote method. It is an operator command envelope carrying `session_id`, `request_id`, `client_kind`, `project_id`, `claim_kind`, operation payload, receipt reference, and event stream correlation.
2. A generic RPC-first design tends to make typed methods the source of truth. That is useful for narrow service APIs, but risky for VIDA because correctness depends on cross-cutting metadata and state-admission semantics on every request.
3. VIDA needs multiple execution transports over the same semantics: in-process local adapter, future daemon IPC, possible localhost HTTP fallback, TUI, CLI, dashboard, and host-runtime adapters. If the first contract is a framework service trait, local/offline/direct adapters become secondary compatibility work instead of first-class paths.
4. The current codebase already has direct activation/status/runtime surfaces. The next refactor should extract activation core and command semantics before selecting a transport framework; otherwise the service API may simply wrap current direct mutation instead of correcting ownership.
5. Frameworks such as `jsonrpsee` and `tarpc` remain useful candidates under a `VidaTransport` boundary. They should implement transport mechanics, not define product law.

When RPC can be accepted:

- after `VidaCommandEnvelope`, `VidaCommandResponse`, `VidaEvent`, `VidaReceiptRef`, and `VidaClient` are defined;
- when the RPC method is a thin carrier such as `vida.command(envelope) -> response`;
- when request/session/claim validation lives above the framework layer;
- when in-process and IPC adapters can pass the same conformance tests.

### jsonrpsee Vs tarpc Efficiency Comparison

Date: 2026-05-21.

Current versions observed from crates.io:

- `jsonrpsee = 0.26.0`
- `tarpc = 0.37.0`

Source references:

- `https://docs.rs/jsonrpsee/latest/jsonrpsee/`
- `https://docs.rs/tarpc/latest/tarpc/`
- `https://docs.rs/crate/tarpc/latest/features`

Comparison:

| Dimension | `jsonrpsee` | `tarpc` | VIDA assessment |
|---|---|---|---|
| Raw runtime efficiency | JSON parse/serialize overhead; optimized enough for operator commands but not the leanest wire format | Can use in-process transport and serde/bincode transport; likely lower CPU and byte overhead for Rust-to-Rust calls | `tarpc` wins for raw Rust-to-Rust throughput/latency |
| Transport fit | HTTP, WebSocket, WASM/client features, server modules, subscriptions | Pluggable transport; TCP and Unix features; in-process channel examples; custom transport possible | `jsonrpsee` wins for browser/dashboard/interop; `tarpc` wins for Rust-only local calls |
| Envelope fit | Natural fit for explicit JSON command envelope and debug logs | Tends to model service traits/methods first; envelope can be carried but feels less native | `jsonrpsee` fits VIDA protocol semantics better |
| Type safety | JSON-RPC params/results plus optional macros | Strong Rust service trait generation | `tarpc` wins inside Rust-only service boundaries |
| Cancellation/deadlines | Supports server/client mechanics but cancellation/deadline semantics are not the primary design point | Built-in request context, cascading cancellation, deadline propagation, tracing | `tarpc` wins for structured RPC execution control |
| Subscriptions/events | Built-in JSON-RPC subscription concepts and WebSocket fit | Can stream via RPC design/custom transport, but less directly aligned to JSON event clients | `jsonrpsee` wins for TUI/dashboard event stream compatibility |
| Debuggability | JSON messages are easy to log, replay, inspect, and reproduce | Binary transport is less inspectable; JSON transport possible but reduces raw-efficiency advantage | `jsonrpsee` wins for operator diagnostics |
| Non-Rust clients | Stronger: JSON-RPC over HTTP/WebSocket is broadly consumable | Weaker: Rust-first service trait model, although serde transport can be adapted | `jsonrpsee` wins for future external clients |
| Dependency/complexity posture | More protocol/web transport surface | More generated RPC/service machinery and tracing/deadline machinery | Tie; choose based on adapter role |

Efficiency conclusion:

1. If "efficient" means **lowest overhead Rust-to-Rust RPC**, `tarpc` is the better candidate.
2. If "efficient" means **fastest path to a debuggable, multi-client VIDA control protocol**, `jsonrpsee` is the better candidate.
3. If "efficient" means **best architecture for VIDA**, neither should own the core. Define `VidaCommandEnvelope` and `VidaClient` first.

Recommended use:

- Do not use `tarpc` or `jsonrpsee` for the first in-process `VidaClient` adapter.
- For the first daemon RPC backend, prefer `tarpc` carrying `VidaCommandEnvelope` through a narrow generic service trait.
- Keep `jsonrpsee` as the later dashboard/browser/API backend for HTTP/WebSocket, JSON-RPC subscriptions, and non-Rust clients.
- Keep `interprocess` local socket JSON framing as an optional diagnostic/fallback transport, not the primary first daemon RPC backend.

### Generic Transport Abstraction For Multiple RPC Backends

Question:

- Can VIDA support both `jsonrpsee` and `tarpc` without binding product architecture to either one?

Answer:

- Yes, but the generic boundary must be a VIDA protocol boundary, not a generic RPC-framework boundary.

Required layering:

```text
vida-core
  VidaCommandEnvelope
  VidaCommandResponse
  VidaEvent
  VidaReceiptRef
  VidaClient trait
  session/request/claim validation
  project routing
  activation/status/task operations

vida-transport
  VidaTransport trait
  InProcessTransport
  LocalSocketJsonTransport
  JsonRpseeTransport
  TarpcTransport

vida-cli / vida-tui / future dashboard
  depend on VidaClient, not on transport-specific clients
```

Candidate trait shape:

```rust
#[async_trait]
pub trait VidaClient {
    async fn execute(
        &self,
        envelope: VidaCommandEnvelope,
    ) -> Result<VidaCommandResponse, VidaClientError>;

    async fn subscribe(
        &self,
        filter: VidaEventFilter,
    ) -> Result<VidaEventStream, VidaClientError>;
}
```

Transport adapters:

| Adapter | Mapping |
|---|---|
| `InProcessTransport` | calls VIDA command handler directly in the same process |
| `LocalSocketJsonTransport` | sends newline-delimited JSON `VidaCommandEnvelope` over `interprocess` local sockets |
| `JsonRpseeTransport` | exposes one or a few generic methods, for example `vida.command(envelope)` and `vida.subscribe(filter)` |
| `TarpcTransport` | exposes a Rust service trait that still carries `VidaCommandEnvelope`, not many product-specific RPC methods |

Important rule:

- The adapter may optimize serialization, framing, cancellation, or subscriptions, but it must not own command admission, session identity, claims, receipts, project routing, or activation behavior.

Conformance tests required before multiple backends are accepted:

1. Same envelope produces the same response across in-process, local socket, `jsonrpsee`, and `tarpc` adapters.
2. Same missing/invalid `session_id` fails the same way across adapters.
3. Same claim conflict fails closed with the same machine-readable blocker code.
4. Same apply operation produces equivalent receipt/event sequence.
5. Same project selection routes to the same project-local state root.
6. Event stream ordering and terminal job state are equivalent for TUI-observed operations.

Design consequence:

- `tarpc` and `jsonrpsee` should both be supported through optional adapter crates/features.
- The first implementation should still land the core types and in-process adapter first, because those are the proof surface for protocol correctness.
- The first service/daemon RPC adapter should be `TarpcTransport`.
- `JsonRpseeTransport` should be added later for dashboard, browser-facing API, HTTP/WebSocket access, and external/non-Rust clients.

Assessment:

- Not recommended as the first architecture owner.
- A framework may be used behind the `VidaTransport` boundary later.

### Option E: Dashboard-First

Shape:

- Start with localhost HTTP/web dashboard and add TUI later.

Strengths:

- Good for visual monitoring and broad inspection.
- Easier logs/tables/forms than terminal for some users.

Weaknesses:

- Does not solve CLI/TUI/service ownership.
- Adds browser security/auth/CORS/static assets concerns too early.
- Less aligned with terminal/SSH/operator workflows.

Assessment:

- Defer.
- Keep the future dashboard as a separate client over the same service API/event stream.

Recommended implementation path:

1. Complete the session identity defect before TUI/service writes.
2. Define the internal command envelope and response/event/receipt types.
3. Extract activation core from direct `project_activator_surface.rs` mutation.
4. Build local in-process `VidaClient` adapter.
5. Convert selected CLI activation/status operations to use the adapter.
6. Introduce service daemon with `TarpcTransport` carrying `VidaCommandEnvelope`.
7. Add multi-project service registry and per-project DB routing.
8. Build Ratatui TUI wizard against `VidaClient`, using the same tarpc-backed client when attached to the daemon.
9. Add service install/control commands.
10. Add `JsonRpseeTransport` and dashboard/browser API only after the service event stream is stable.

## Proposed Architecture For Approval

Date: 2026-05-21.

This section is the current approval packet for the VIDA service, TUI, and activation wizard architecture.

### Architecture Thesis

VIDA should become a headless local control plane with multiple clients.

The durable product boundary is:

```text
client action
  -> VidaCommandEnvelope
  -> VidaClient
  -> transport adapter
  -> vida-service command handler
  -> project-scoped state/router
  -> receipt + events + response
```

The TUI, CLI, daemon, and future dashboard must share this path. No UI or CLI command should own a separate mutation path once the client boundary exists.

### Layers

| Layer | Responsibility | First implementation posture |
|---|---|---|
| `vida-core` | command envelope, response/event/receipt types, session identity, claim/admission validation, project routing model, activation core | required first |
| `vida-client` | `VidaClient` trait plus in-process adapter used by CLI/tests | required first |
| `vida-service` | headless daemon, command handler, project registry, per-project state routing, job/event/receipt runtime | after core/client |
| `vida-transport-tarpc` | first daemon RPC adapter carrying `VidaCommandEnvelope` | first service transport |
| `vida-tui` | Ratatui operator console and wizard, using `VidaClient` only | after service client path exists |
| `vida-transport-jsonrpsee` | HTTP/WebSocket/dashboard/browser API transport | later |
| `vida-dashboard` | browser dashboard client over JSON-RPC/WebSocket/event stream | later |

### Core Types

Required command contract:

```text
VidaCommandEnvelope
  session_id
  request_id
  client_kind
  project_ref
  operation
  claim_kind
  payload
  correlation

VidaCommandResponse
  request_id
  status
  result | error
  receipt_ref
  job_ref
  blockers

VidaEvent
  event_id
  request_id
  session_id
  project_id
  job_id
  kind
  payload

VidaReceiptRef
  receipt_id
  project_id
  operation
  state_root
```

Rule:

- `session_id`, `request_id`, `project_ref`, and `claim_kind` are mandatory for mutating operations.
- Read-only operations still carry session/request identity for observability and multi-session correctness.

### Service Capabilities

MVP service capabilities:

1. Project registry.
2. Project selection and routing to project-local state root.
3. Session/request identity validation.
4. Claim/admission validation.
5. Activation inspect/plan/diff/validate/apply.
6. Status projection.
7. Job tracking.
8. Event stream for progress/logs.
9. Receipt writing and lookup.
10. Service status/health.
11. Service install/configuration/status integration for activation.

Post-MVP service capabilities:

1. Update/reconfigure engine for docs/agents/sidecar/template refresh.
2. Drift detection and repair.
3. Skills/roles/profiles/flows mutation lifecycle.
4. Multi-project search/filter/history.
5. Dashboard HTTP/WebSocket API through `JsonRpseeTransport`.
6. Remote-safe auth policy if non-local access is ever allowed.

### Wizard Capabilities

MVP wizard modes:

1. Activate new project.
2. Reconfigure existing project.
3. Update docs/agents/sidecar/templates.
4. Repair activation drift.

MVP wizard flow:

1. Select/confirm project.
2. Inspect current project shape and activation state.
3. Collect project identity and language policy.
4. Select host CLI/runtime system.
5. Configure docs roots and bootstrap carriers.
6. Configure service installation posture.
7. Show skills/roles/profiles/flows inventory as read-only.
8. Build plan.
9. Preview diff.
10. Validate blockers.
11. Apply through service/client.
12. Install/configure service when selected and supported.
13. Stream progress/logs.
14. Show receipt and final status.

Post-MVP wizard modes:

1. Import/export config bundle.
2. Replace/disable/restore skills, roles, profiles, and flows.
3. Team/runtime profile switching.
4. Cross-project config comparison.

### TUI Capabilities

MVP TUI screens:

1. Projects.
2. Status.
3. Activation Wizard.
4. Diff Preview.
5. Apply Progress.
6. Logs/Events.
7. Receipts.
8. Skills/Roles/Profiles/Flows inventory.
9. Agents/Carriers inventory.

MVP TUI rules:

- TUI never writes files or DB directly.
- TUI never bypasses service/client claim validation.
- TUI handles reconnect/retry as a client concern only.
- TUI uses Ratatui component architecture and snapshot tests for stable screens.

### CLI Capabilities

CLI direction:

1. Preserve current direct commands while core/client extraction is in progress.
2. Move selected status and activation operations to `VidaClient`.
3. Prefer daemon client mode once `TarpcTransport` exists.
4. Keep explicit direct fallback for bootstrap/offline recovery.
5. Make fallback visible in output and receipts.

### Transport Decisions

Approved transport sequence candidate:

1. In-process `VidaClient` adapter for core proof and CLI migration.
2. `TarpcTransport` as the first daemon RPC backend.
3. Optional `LocalSocketJsonTransport` for diagnostics/fallback if needed.
4. `JsonRpseeTransport` for dashboard/browser/non-Rust API later.

Constraint:

- `TarpcTransport` must expose a narrow service carrying `VidaCommandEnvelope`, not many product-specific methods.

### Multi-Project Model

One VIDA service can manage many projects.

Rules:

1. Every request resolves to exactly one project for project-scoped operations.
2. Every project keeps its own DB/state root.
3. TUI and CLI operating in the same project observe the same project-local truth.
4. Cross-project operations are explicit service-level operations.
5. Project registry mutation is service-owned and receipt-backed.

### Session Model

Precondition:

- Fix the current session identity defect before service/TUI write paths.

Target identity model:

```text
session_id -> request_id -> project_ref -> claim -> operation -> receipt/events
```

Rules:

1. `VIDA_SESSION_ID` is canonical.
2. Legacy/vendor session ids are aliases into the resolver.
3. Fallback session id is per live session/process, not project-root-derived.
4. Foreign sessions are visible but nonblocking unless claim/task/path/conflict-domain/global-integrity overlap exists.

### Installation Model

Initial posture:

1. User-level service install first.
2. `service-manager` for install/start/stop/uninstall orchestration.
3. Native `windows-service` only when needed for Windows service runtime integration.
4. System-level install later.

Service commands:

```text
vida service install
vida service start
vida service stop
vida service status
vida service uninstall
vida service logs
```

### Meta-Analysis Result

Best path:

- Envelope/Core-first with adapters, `tarpc` as first daemon RPC transport, `jsonrpsee` later for dashboard/API.

Why:

1. Preserves VIDA semantics before transport lock-in.
2. Uses `tarpc` where it is strongest: Rust-to-Rust daemon/client RPC.
3. Keeps `jsonrpsee` for the place where it is strongest: JSON-RPC HTTP/WebSocket and non-Rust/browser clients.
4. Lets TUI and CLI share the same client contract.
5. Keeps multi-session and multi-project correctness above transport adapters.

Rejected as primary path:

1. Direct TUI/filesystem writes.
2. Dashboard-first implementation.
3. RPC-framework-first product API.
4. TUI before service/client contract.

### Approved Decisions

Approved by operator on 2026-05-21:

1. Use `Envelope/Core-first with adapters` as the architecture strategy.
2. Use `TarpcTransport` as the first daemon RPC backend.
3. Add `JsonRpseeTransport` later for dashboard/browser/API.
4. Use the TUI MVP scope: projects, status, activation wizard, diff, apply progress, logs/events, receipts, read-only skills/roles/profiles/flows inventory.
5. Use the service MVP scope: project registry, project routing, session/request/claim validation, activation plan/apply, jobs/events, receipts, status/health.
6. Target user-level service installation first.
7. Treat the session identity fix as a hard precondition before service/TUI mutation paths.

## Next Research Pass And Clarifications

Date: 2026-05-21.

Purpose:

- Identify the remaining decisions that must be clarified before converting the approved architecture into implementation tasks.

Sources:

- `https://docs.rs/tarpc/latest/tarpc/trait.Transport.html`
- `https://docs.rs/tarpc/latest/tarpc/serde_transport/index.html`
- `https://docs.rs/tarpc/latest/tarpc/serde_transport/tcp/index.html`
- `https://docs.rs/tarpc/latest/tarpc/serde_transport/unix/index.html`
- `https://docs.rs/interprocess/latest/interprocess/local_socket/tokio/index.html`
- `https://docs.rs/service-manager/latest/service_manager/index.html`
- `https://ratatui.rs/faq/`
- `https://ratatui.rs/tutorials/counter-async-app/`

### Finding 1: Tarpc Needs A Concrete Cross-Platform Transport Decision

Evidence:

1. `tarpc::serde_transport` can serialize over any medium implementing `AsyncRead` and `AsyncWrite`.
2. Built-in tarpc helpers include TCP and Unix Domain Socket modules.
3. Tarpc Unix transport is Unix-only.
4. `interprocess` provides Tokio local sockets and maps local sockets to platform-local IPC, including Windows named-pipe-backed local sockets and Unix-domain-socket-backed local sockets.

Clarification needed:

- Should `TarpcTransport` use:
  1. `tarpc::serde_transport::tcp` over loopback for all platforms,
  2. built-in tarpc Unix transport on Unix plus TCP fallback on Windows,
  3. custom tarpc serde transport over `interprocess` Tokio local sockets?

Recommendation:

- Target option 3 for the real local daemon transport: tarpc service semantics over `interprocess` Tokio local socket byte streams using tarpc serde transport/framing.
- Allow loopback TCP only as a development/debug fallback with explicit auth token and warning.

Reason:

- It preserves the tarpc-first decision while keeping local IPC local and cross-platform.

### Finding 2: TUI Event Streaming Should Not Depend On Full RPC Streaming In MVP

Evidence:

1. TUI needs progress/log/event updates.
2. Tarpc is strong for Rust-to-Rust request/response, cancellation, deadline, and typed service calls.
3. Jsonrpsee is stronger for standard JSON-RPC subscriptions and WebSocket/dashboard-style event streams.

Clarification needed:

- Should MVP event flow use:
  1. tarpc long-lived streaming-style calls,
  2. service-persisted events plus `events_since(cursor)` polling/long-polling over tarpc,
  3. separate local socket event stream immediately?

Recommendation:

- Use service-persisted event log plus `events_since(cursor)` or bounded long-poll over tarpc for MVP.
- Add jsonrpsee subscriptions later for dashboard/browser/non-Rust clients.

Reason:

- It gives deterministic receipts/events and simple reconnection semantics for TUI without forcing a complex stream protocol into the first daemon slice.

### Finding 3: User-Level Service Install Is Platform-Asymmetric

Evidence:

1. `service-manager` supports several managers: `sc.exe`, WinSW, launchd, systemd, OpenRC, rc.d.
2. Its docs explicitly state that some platforms like systemd and launchd support user-level service management.
3. Windows service management through `sc.exe` is a Windows Service path and may require elevated/system service semantics depending on install mode and environment.

Clarification needed:

- Should the MVP define Windows as:
  1. native Windows Service from day one,
  2. foreground/session daemon first, native Windows Service later,
  3. WinSW wrapper path first?

Recommendation:

- Keep user-level service install first for Linux systemd-user and macOS launchd-user.
- For Windows MVP, support foreground/session daemon plus explicit install diagnostics first; add native Windows Service path when permission/elevation behavior is proven.

Reason:

- This avoids promising cross-platform user-level install semantics that service managers do not provide uniformly.

### Finding 4: Crate Split Needs One Shared Protocol Crate Before Many Runtime Crates

Evidence:

1. The current workspace already separates TaskFlow/DocFlow into contracts/core/cli/state crates.
2. `crates/vida` is currently monolithic for VIDA runtime surfaces.
3. TUI, service, CLI, and transports need shared envelope/event/receipt types.

Clarification needed:

- Should the first implementation create many crates immediately, or only the shared protocol crate?

Recommendation:

- First add a small shared `vida-protocol` or `vida-contracts` crate for `VidaCommandEnvelope`, `VidaCommandResponse`, `VidaEvent`, `VidaReceiptRef`, ids, errors, and conformance fixtures.
- Keep activation core/service/client extraction inside `crates/vida` initially unless a crate boundary becomes necessary.
- Split `vida-service`, `vida-tui`, and transport crates after the protocol types are stable.

Reason:

- This gives type sharing without over-splitting before the seams are proven.

### Finding 5: Activation Core Needs A Real Plan/Diff Contract Before UI Work

Clarification needed:

- What must `ActivationPlan` describe?

Recommended minimum fields:

```text
ActivationPlan
  plan_id
  project_ref
  mode
  inputs_summary
  file_operations
  config_operations
  agent_template_operations
  docs_operations
  service_install_operations
  state_operations
  validation_blockers
  required_restarts
  receipt_preview
```

Recommendation:

- Extract activation around `inspect -> plan -> diff -> validate -> apply -> receipt`.
- `apply` must be idempotent and receipt-backed.
- TUI consumes plan/diff only; it does not compute file writes.

### Finding 6: Local Security And Ownership Need An MVP Policy

Clarification needed:

- What security model does local service use before dashboard/remote access?

Recommendation:

1. Per-user daemon by default.
2. Local socket path/pipe name includes user-scoped service identity.
3. Local socket/pipe permissions restrict access to the current user where the OS supports it.
4. Loopback TCP fallback requires a generated local auth token.
5. No remote bind in MVP.
6. Every mutating request still requires `session_id`, `request_id`, `project_ref`, and `claim_kind`.

### Finding 7: Project Registry Authority Must Be Split Carefully

Clarification needed:

- Where does the multi-project registry live?

Recommendation:

- Service-level registry lives in user-level VIDA service state.
- Project-local DB/state remains authority for project-scoped runtime truth.
- Registry entries point to project root, project id, state dir, db profile, activation status, and last health.
- Registry mutations are service receipts; project-scoped mutations also write project receipts.

### Approved Clarifications

Approved by operator on 2026-05-21:

1. Use tarpc over `interprocess` Tokio local sockets as the target local daemon transport, with loopback TCP only as debug fallback.
2. Use tarpc `events_since(cursor)`/bounded long-polling for MVP TUI events; reserve jsonrpsee subscriptions for dashboard/browser later.
3. Use Linux/macOS user-level service first; Windows MVP starts with foreground/session daemon plus install diagnostics, then native Windows Service after proof.
4. Make the first crate split a small shared `vida-protocol` or `vida-contracts` crate, not immediate full crate explosion.
5. Treat `ActivationPlan` as a first-class plan/diff/apply contract before TUI screens.
6. Use a local-only, per-user security policy with socket/pipe permissions and auth-token-protected TCP fallback.
7. Use service-level project registry plus project-local DB authority split.
8. Add service install/configuration to project activation and wizard flow.

### Activation Service Install Decision

Activation must support service installation/configuration as part of the plan/apply lifecycle.

Rules:

1. Service install is represented in `ActivationPlan.service_install_operations`.
2. Service install has preview/diff/blocker output before apply.
3. Linux/macOS activation can offer user-level service install when platform checks pass.
4. Windows activation MVP offers foreground/session daemon setup and install diagnostics; native Windows Service install is a later platform-proofed path.
5. Service install writes receipts and emits events like other activation mutations.
6. Service install must not hide permission/elevation requirements; blockers must be explicit and machine-readable.
7. Skipping service install is allowed, but the final activation status must report service posture as `not_installed`, `foreground_session`, `user_service_installed`, `system_service_installed`, or `unsupported`.

## Implementation Clarification Research Pass

Date: 2026-05-21.

Purpose:

- Identify the next set of implementation details that remain risky after approving activation-owned service install.

Sources:

- `https://docs.rs/tarpc/latest/tarpc/serde_transport/struct.Transport.html`
- `https://docs.rs/tarpc/latest/tarpc/serde_transport/index.html`
- `https://docs.rs/interprocess/latest/interprocess/local_socket/tokio/index.html`
- `https://docs.rs/tokio-serde/latest/tokio_serde/`
- `https://docs.rs/async-trait/latest/async_trait/`
- `https://doc.rust-lang.org/reference/types/trait-object.html`
- `https://doc.rust-lang.org/reference/items/traits.html`

Local evidence:

1. Workspace currently has no `async-trait`, `tarpc`, `interprocess`, or `tokio-serde` dependency.
2. Root workspace dependencies already include `uuid` with `v7`; this is suitable for request/job/event ids.
3. `crates/vida` is currently a single binary crate with direct runtime surfaces.
4. Existing receipt concepts exist, but there is no service command envelope/idempotency contract yet.
5. The installed compiler is `rustc 1.94.0`; native async functions in traits exist, but dyn/object-safe async trait use still needs an explicit strategy if `VidaClient` is used behind trait objects.

### Finding 8: Activation Has A Service Bootstrap Paradox

Problem:

- Activation now includes service install/configuration, but the service may not exist yet.
- Therefore the first activation run cannot require the daemon path for all mutations.

Recommended model:

```text
phase 1: local bootstrap activation
  vida project-activator / vida activate
  -> InProcessVidaClient
  -> inspect/plan/diff/validate/apply bootstrap-safe mutations
  -> install or configure service when selected
  -> emit bootstrap receipt

phase 2: service attach
  start/attach to vida-service
  -> register project
  -> verify service health/protocol version
  -> emit service attach receipt

phase 3: service-owned operation
  CLI/TUI use service-backed VidaClient for later mutations
```

Clarification needed:

- Should the first activation run be explicitly named `bootstrap activation`, with service-owned activation only after daemon attach succeeds?

Recommendation:

- Yes. The activation flow should report both `activation_status` and `service_posture`.

### Finding 9: Tarpc Over Interprocess Needs A Proof Spike

Evidence:

1. `tarpc::serde_transport::Transport` can wrap any byte stream implementing `AsyncRead + AsyncWrite`.
2. `interprocess` provides Tokio local socket byte streams.
3. The intended design is plausible: `interprocess` Tokio stream -> length-delimited framing -> `tokio-serde` codec -> tarpc transport.

Risk:

- The exact type stack and platform behavior must be proven on Windows, Linux, and macOS before it becomes product law.

Clarification needed:

- Should the first transport implementation task be a narrow proof spike before service command implementation?

Recommendation:

- Yes. Add a proof task that compiles and runs a tiny tarpc request/response over `interprocess` Tokio local sockets on the available development platform, with TCP fallback kept separate.

### Finding 10: VidaClient Should Be Object-Safe For CLI/TUI

Problem:

- CLI/TUI code will likely hold `Arc<dyn VidaClient + Send + Sync>`.
- Native `async fn` in traits is not enough if dyn compatibility is required.

Options:

1. Use `async-trait`.
2. Use explicit boxed futures, for example `fn execute(&self, envelope) -> BoxFuture<'_, Result<...>>`.
3. Keep everything generic over concrete client types.

Recommendation:

- Use explicit boxed future type aliases in the client crate for public `VidaClient` object safety.
- Avoid `async-trait` in the protocol/core boundary unless implementation ergonomics clearly outweigh the dependency/macro cost.
- Concrete internal handlers may still use `async fn`.

Reason:

- This keeps `VidaClient` dyn-friendly and makes allocation/type erasure explicit at the boundary where dynamic dispatch is actually needed.

### Finding 11: Apply Needs Idempotency And Plan Preconditions

Problem:

- TUI/service apply operations can be retried after reconnect, timeout, or service restart.
- Without idempotency, retry can duplicate file writes, receipts, or service install attempts.

Recommended contract:

```text
ActivationPlan
  plan_id
  plan_hash
  project_ref
  generated_at
  preconditions
  operations
  receipt_preview

ActivationApplyRequest
  request_id
  session_id
  plan_id
  plan_hash
  apply_token
  idempotency_key
```

Rules:

1. `request_id` is unique per request.
2. `idempotency_key` is stable for retried apply.
3. `apply_token` binds the approved plan/diff to apply.
4. Service returns the existing job/receipt when the same idempotency key is replayed.
5. File/config operations include precondition checks where possible.

### Finding 12: Event Model Needs Cursor Semantics Before TUI

Problem:

- TUI needs stable progress after reconnect.
- Long-polling without persisted events loses history.

Recommended minimum event contract:

```text
VidaEvent
  event_id
  event_seq
  scope
  project_id
  request_id
  job_id
  kind
  level
  payload
  recorded_at

events_since(cursor, filter, max_events, timeout_ms)
  -> events
  -> next_cursor
  -> terminal_job_state?
```

Rules:

1. Events are persisted at least for active/recent jobs.
2. `event_seq` is monotonic within a scope.
3. TUI resumes from cursor after reconnect.
4. Terminal job state is returned even if event retention trimmed older entries.

### Finding 13: Protocol Version And Capability Negotiation Are Needed Early

Problem:

- CLI, TUI, daemon, and later dashboard may not always be the same binary version.

Recommended handshake:

```text
vida.service.hello
  client_version
  protocol_version
  client_kind
  session_id

response
  service_version
  protocol_version
  supported_operations
  supported_transports
  feature_flags
  service_id
  service_posture
```

Rules:

1. Client fails closed on incompatible protocol version for mutating operations.
2. Read-only status may degrade with clear warnings.
3. TUI displays protocol/service mismatch as a first-class blocker.

### Finding 14: Service Runtime Should Use Per-Project Actors Or Queues

Problem:

- A single service can manage many projects, and project-local DB/state writes must not interleave unsafely.

Recommendation:

- Use a service-level router plus per-project command queue/actor for mutating operations.
- Allow read-only status operations to use cached/projection reads when safe.
- Keep claims/admission above the queue and receipts/events inside the queue result path.

Reason:

- This gives clear serialization for project-scoped writes while allowing multi-project concurrency.

### Approved Clarifications, Set 2

Approved by operator on 2026-05-21:

1. Use `bootstrap activation` as the first activation phase before daemon attach; service-owned activation begins after service health/register succeeds.
2. Run a tarpc-over-interprocess proof spike before implementing the full daemon command surface.
3. Make `VidaClient` object-safe using explicit boxed futures at the client boundary.
4. Use an idempotent apply contract with `plan_id`, `plan_hash`, `apply_token`, and `idempotency_key`.
5. Use persisted event cursor contract with `events_since(cursor, filter, max_events, timeout_ms)`.
6. Use service hello/version/capability negotiation before mutating daemon commands.
7. Use per-project command queues/actors for service-owned mutations.

## Protocol And Runtime Contract Research Pass

Date: 2026-05-21.

Purpose:

- Identify the next clarifications needed before turning the approved architecture into concrete TaskFlow implementation packets.

Sources:

- `https://serde.rs/enum-representations.html`
- `https://docs.rs/schemars/latest/schemars/`
- `https://www.rfc-editor.org/rfc/rfc9457`
- `https://spec.openapis.org/oas/latest.html`

Local evidence:

1. The workspace already uses the `*-contracts` naming pattern through `taskflow-contracts` and `docflow-contracts`.
2. Existing contracts crates are intentionally small serde-oriented shared type crates.
3. Root workspace dependencies already include `uuid` with `v7`, which fits request/job/event ids.
4. VIDA currently uses many string blocker/status codes in runtime surfaces, but there is no central service operation catalog or error/problem type yet.

### Finding 15: Shared Crate Should Be `vida-contracts`

Problem:

- Earlier notes allowed `vida-protocol` or `vida-contracts`.
- The current workspace already uses `taskflow-contracts` and `docflow-contracts`.

Recommendation:

- Name the first shared crate `vida-contracts`.
- Put protocol-neutral shared data contracts there:
  - ids,
  - operation ids,
  - command envelope,
  - typed payload structs,
  - response/problem/blocker types,
  - event/job/receipt refs,
  - schema version constants,
  - conformance fixtures.

Reason:

- It follows existing repo conventions and avoids implying that a transport protocol owns these types.

### Finding 16: Use Generic Wire Envelope Plus Typed Payload Contracts

Problem:

- A fully untyped `serde_json::Value` payload loses compile-time structure.
- A fully typed RPC method set would undermine the approved generic envelope design.

Recommendation:

- Use both levels:

```text
wire/runtime envelope
  VidaCommandEnvelope {
    schema_version,
    protocol_version,
    operation,
    session_id,
    request_id,
    project_ref,
    claim_kind,
    payload,
    correlation
  }

typed contracts
  ProjectActivationInspectRequest
  ProjectActivationPlanRequest
  ProjectActivationApplyRequest
  ServiceHelloRequest
  EventsSinceRequest
  ...
```

Rules:

1. Transport carries the generic envelope.
2. Command handler dispatches by `operation`.
3. Handler deserializes `payload` into the typed request for that operation.
4. Conformance tests verify operation id plus payload schema.
5. Internal helper enums may use serde adjacently tagged representation for fixtures, but the transport boundary remains envelope-first.

### Finding 17: Operation Catalog Must Be Stable Before Service/TUI

Problem:

- Free-form operation strings make client/server compatibility and TUI routing fragile.

Recommended MVP operation ids:

```text
vida.service.hello
vida.service.status
vida.service.install.plan
vida.service.install.apply
vida.project.registry.list
vida.project.registry.register
vida.project.status
vida.project.activation.inspect
vida.project.activation.plan
vida.project.activation.apply
vida.events.since
vida.receipts.get
```

Rules:

1. Operation ids are stable string constants.
2. Operation ids are documented in `vida-contracts`.
3. Mutating operations declare allowed `claim_kind`.
4. Deprecated operation ids remain recognized until an explicit compatibility window ends.

### Finding 18: Error/Blocker Model Should Follow Problem-Details Shape Without Being HTTP-Owned

Evidence:

- RFC 9457 obsoletes RFC 7807 and defines a standard problem-details data model for machine-readable errors in JSON.

Recommendation:

- Define `VidaProblem` modeled after problem-details concepts but not tied to HTTP:

```text
VidaProblem
  type
  title
  detail
  code
  severity
  retryable
  blockers
  remediation
  instance
  related_receipt
```

Rules:

1. `code` is the primary VIDA machine-readable error code.
2. `type` can later map to HTTP/dashboard problem types.
3. `blockers` carry structured blocker codes, scope, and next actions.
4. No stack traces or internal paths are exposed to dashboard/non-local clients by default.
5. Local CLI/TUI can request diagnostic detail explicitly when authorized.

### Finding 19: Job/Event/Receipt Storage Needs Ownership Boundaries

Problem:

- Service-level jobs and project-scoped jobs have different authority.

Recommendation:

```text
service-level state
  service registry
  project registry
  service health
  service install jobs
  attach/handshake receipts

project-level state
  activation jobs
  project mutation receipts
  project event log
  idempotency records
  project status projections
```

Rules:

1. Service-level registry writes produce service receipts.
2. Project-scoped mutations produce project receipts.
3. Activation with service install can produce both a project receipt and a service receipt.
4. Idempotency records live at the same authority level as the mutation they protect.
5. Event cursors include scope so TUI can distinguish service-level and project-level event streams.

### Finding 20: Event Retention And Job Recovery Need MVP Defaults

Problem:

- Persisted events are approved, but retention/recovery behavior is still undefined.

Recommendation:

1. Keep all events for active jobs.
2. Keep recent terminal job events for a bounded retention window.
3. Persist terminal job summary even after detailed events are trimmed.
4. On service restart, mark non-terminal jobs as `recovering`, `resumable`, `failed_recoverable`, or `failed_terminal`.
5. Mutating recovery uses idempotency records and receipts before retrying any operation.

Open parameter:

- Exact retention window can start with a conservative config default, for example 7 days or last N events per project, then tune after usage.

### Finding 21: Conformance Matrix Should Precede TUI Screens

Problem:

- Multiple adapters are approved. Without a shared test matrix, adapters can drift.

Recommendation:

- Add a conformance test harness in or beside `vida-contracts`:

```text
same command fixtures:
  in-process client
  tarpc client
  later jsonrpsee client

assertions:
  same response status
  same blocker/problem code
  same receipt refs
  same event sequence shape
  same idempotent replay result
```

Rules:

1. In-process adapter is the baseline oracle.
2. Tarpc adapter must pass the same command fixtures before TUI uses it.
3. Jsonrpsee adapter must pass the same fixtures before dashboard uses it.
4. Schema/golden fixtures are stored with the contracts crate or test support crate.

### Finding 22: Schema Generation Is Useful But Should Not Block The First Contracts

Evidence:

- `schemars` can generate JSON Schema from serde-compatible Rust types.

Recommendation:

- Start with serde types and golden JSON fixtures.
- Add `schemars` once contracts stabilize enough to publish generated schema artifacts.
- Do not block `vida-contracts` creation on full schema publishing.

Reason:

- The first risk is contract shape and conformance, not external schema publishing.

### Approved Clarifications, Set 3

Approved by operator on 2026-05-21:

1. Use `vida-contracts` as the first shared crate name and home for envelope/response/event/receipt/problem/id contracts.
2. Use generic wire `VidaCommandEnvelope` plus typed payload structs per operation.
3. Use the MVP operation catalog listed above as the initial stable operation id set.
4. Use `VidaProblem` based on RFC 9457-style problem details, with VIDA `code` as the primary machine-readable field.
5. Use service-level vs project-level state/receipt/idempotency ownership split.
6. Use event retention/recovery defaults: all active job events, bounded recent terminal events, terminal job summaries, restart recovery states.
7. Require conformance matrix before TUI screens: in-process baseline, tarpc adapter parity, later jsonrpsee parity.
8. Use serde/golden fixtures first and defer generated JSON Schema via `schemars` until contracts stabilize.

## Service State, Install, Upgrade, And TUI Proof Research Pass

Date: 2026-05-21.

Purpose:

- Identify the next clarifications needed around installable service runtime, service state storage, binary/version ownership, registry reconciliation, and UI proof before implementation tasks are created.

Sources:

- `https://docs.rs/service-manager/latest/service_manager/`
- `https://docs.rs/directories/latest/directories/`
- `https://docs.rs/camino/latest/camino/`
- `https://docs.rs/ratatui/latest/ratatui/backend/struct.TestBackend.html`
- `https://ratatui.rs/recipes/testing/snapshots/`

Local evidence:

1. `release_surface.rs` already defines release install layout under an install root, including `current/bin`, env files, and binary fingerprint metadata.
2. `runtime_consumption_surface.rs` already reports active executable path, active executable fingerprint, installed binary evidence, path resolution, and divergent installed binaries.
3. `StateStore` is currently project-runtime oriented and SurrealDB-backed; service-level registry/state should not be accidentally coupled to project-local DB truth.
4. Root workspace dependencies include `uuid` with `v7`, `tracing`, and `serde_json`, which are enough for service ids, event ids, logs, and JSONL-style fixtures.
5. Current workspace has no separate `vida-service` binary.

### Finding 23: Use One `vida` Binary For The First Service Runtime

Problem:

- A separate `vida-service` binary would add packaging/release/install complexity before service semantics are proven.

Recommendation:

- Use the existing `vida` binary with service subcommands first:

```text
vida service run
vida service install
vida service start
vida service stop
vida service status
vida service uninstall
vida service logs
```

Rules:

1. Service manager install points to installed `vida` plus `service run`.
2. Separate `vida-service` binary is a later optimization only if lifecycle or privilege boundaries require it.
3. `vida service run` is the daemon entrypoint used by service-manager and by foreground/session daemon mode.

### Finding 24: Service Install Must Bind Binary Fingerprint And Install Layout

Problem:

- Activation-owned service install could otherwise point to `target/debug/vida`, a stale binary, or a path that is not the shell-resolved production binary.

Recommendation:

- Service install plan should include:

```text
ServiceInstallPlan
  binary_path
  binary_fingerprint
  install_root
  runtime_bin_dir
  service_name
  service_home
  service_args
  environment
  platform_manager
  restart_policy
```

Rules:

1. Production service install should prefer the release-installed binary under the existing release install layout.
2. Dev foreground/session mode may use the active executable but must report `service_posture=foreground_session`.
3. Service status compares running binary fingerprint against expected installed fingerprint.
4. Stale fingerprint produces `service_binary_stale` blocker/warning and a restart/update recommendation.
5. Activation cannot silently install a service from an unknown binary path.

### Finding 25: Service Home Should Be User-Level And Overrideable

Problem:

- Service state cannot live inside a single project because one service manages many projects.

Recommendation:

- Define service home resolution:

```text
VIDA_SERVICE_HOME override
else VIDA_HOME/service
else release_install_root()/service
else platform user data dir / vida-stack / service
```

Rules:

1. Service home stores service registry, service receipts, service events, auth token, socket metadata, and service config.
2. Project-local DB/state remains under each project.
3. Service home path is shown in `vida service status` and TUI diagnostics.
4. Service home drift or inaccessible home is a service-level blocker, not a project activation failure unless activation selected service install.

### Finding 26: Service State Store Should Start Small And Append-Only

Problem:

- Reusing project `StateStore` for service-level registry/events risks coupling service truth to project-runtime schema.

Recommendation:

- MVP service state should use a small service-local store:

```text
service-home/
  service.config.json
  auth/token.json
  registry/projects.json
  receipts/service-receipts.jsonl
  events/service-events.jsonl
  jobs/service-jobs.jsonl
  idempotency/service-idempotency.jsonl
```

Rules:

1. Service is the single writer for service-home state.
2. Append-only JSONL is acceptable for service-level low-volume receipts/events/jobs in MVP.
3. Project-scoped mutations still use project-local DB/state.
4. Later migration to SurrealDB or another service DB is allowed behind a service-state abstraction.
5. Service state files include schema versions and compacted summaries where needed.

### Finding 27: Project Registry Needs Reconciliation Semantics

Problem:

- A registry entry can drift from filesystem/project truth: project root moved, state dir missing, duplicate project id, config changed, DB locked, activation incomplete.

Recommendation:

- Project registry entries carry:

```text
ServiceProjectRegistryEntry
  project_id
  project_root
  state_dir
  config_path
  db_profile
  activation_status
  service_binding_status
  last_seen_at
  last_health
  source
```

Rules:

1. Missing project root marks entry `unhealthy_missing_root`; it does not auto-delete.
2. Duplicate project id/path is a registry blocker until operator resolves it.
3. Registry reconciliation reads project-local status but does not mutate project DB unless an explicit repair/apply operation is requested.
4. TUI Projects screen shows registry health and project activation status separately.

### Finding 28: Service Upgrade/Reconfigure Needs Its Own Plan/Apply Flow

Problem:

- Service binary updates, config changes, socket changes, and registry migrations can interrupt active clients.

Recommendation:

- Add later operation ids:

```text
vida.service.upgrade.plan
vida.service.upgrade.apply
vida.service.reconfigure.plan
vida.service.reconfigure.apply
vida.service.restart
```

Rules:

1. Upgrade/reconfigure uses the same plan/diff/apply/idempotency model as activation.
2. Service drains or rejects new mutating jobs before restart when possible.
3. Active TUI receives a restart/reconnect event before daemon restart when possible.
4. After restart, client uses hello/capability negotiation again.
5. Rollback is explicit only after binary install/backup semantics are defined.

### Finding 29: Operation Catalog Needs Diff/Validate/Job Operations Before TUI

Problem:

- Current operation catalog has activation inspect/plan/apply, but TUI needs explicit diff, validation, job, and receipt surfaces.

Recommendation:

- Extend MVP operation catalog with:

```text
vida.project.activation.diff
vida.project.activation.validate
vida.jobs.get
vida.jobs.cancel
vida.receipts.list
```

Rules:

1. `diff` and `validate` can be derived from plan but are exposed for TUI ergonomics and proof clarity.
2. `jobs.cancel` is best-effort and must return a terminal or non-cancellable state.
3. Receipt listing is filtered by project/service scope and operation.

### Finding 30: TUI Should Be Proven Against A Fixture Client Before Live Daemon

Problem:

- Waiting for daemon readiness before TUI proof creates unnecessary coupling.

Recommendation:

- Build TUI tests with a fixture `VidaClient` first:

```text
FixtureVidaClient
  canned projects
  canned status
  canned activation plan/diff
  canned events_since cursor responses
  canned receipts
```

Rules:

1. TUI components use Ratatui `TestBackend` and snapshot tests.
2. TUI integration tests use fixture client before tarpc-backed client.
3. Live daemon tests are separate smoke tests, not the first UI proof gate.
4. Text/layout must fit narrow terminal widths used in snapshot fixtures.

### Approved Clarifications, Set 4

Approved by operator on 2026-05-21:

1. Use one-binary service runtime first: `vida service run`, with separate `vida-service` binary deferred.
2. Use service install plans that bind installed binary path/fingerprint/install layout and block unknown/stale binary paths.
3. Use service home resolution with `VIDA_SERVICE_HOME` override, then `VIDA_HOME/service` or release-install-root service home.
4. Use small append-only service-local state store for MVP service registry/events/jobs/receipts/idempotency, separate from project DB.
5. Use project registry reconciliation semantics: unhealthy entries are marked, duplicates block, reconciliation is read-only unless repair/apply is explicit.
6. Reserve service upgrade/reconfigure plan/apply operations as post-MVP operation ids.
7. Add activation diff/validate, jobs get/cancel, and receipts list operations before TUI implementation.
8. Prove TUI through fixture `VidaClient` plus Ratatui snapshot tests before live daemon tests.
9. Treat connected project management as a first-class wizard and TUI capability, not only an internal service concern.

### Finding 31: Connected Project Management Must Be A First-Class Wizard And TUI Surface

Problem:

- If project registry management remains an internal daemon detail, the operator cannot safely manage a multi-project service.
- Wizard and TUI must expose which projects are connected, which project is active for the current client/session, and which project owns a given DB/state path.
- A service-level registry without explicit operator flows would make repair, reconfigure, detach, and duplicate-resolution behavior opaque.

Recommendation:

- Add a first-class project management capability shared by CLI, TUI, and wizard through the service command envelope.
- Treat activation as an operation on a selected/registered project, not as the only way to discover a project.
- Keep destructive filesystem deletion out of MVP; project management controls service bindings, registry metadata, activation/reconfigure state, and repair plans.

Wizard entry points:

```text
ProjectManagementWizard
  connect_existing_project
  activate_new_project
  repair_or_reconnect_project
  reconfigure_connected_project
  detach_project_from_service
  archive_or_restore_registry_entry
  inspect_project_registry_conflicts
```

TUI surfaces:

```text
Projects screen
  connected projects table
  selected project detail pane
  registry health
  activation status
  service binding status
  DB/state path
  active sessions/jobs
  recent receipts/events
  actions: connect, activate, reconfigure, repair, detach, archive, forget, set active, refresh
```

Rules:

1. TUI has a persistent `Projects` area, not only a hidden activation step.
2. Wizard starts with project selection or project connection before activation plan generation.
3. CLI resolves project from current working directory first; TUI resolves it from explicit operator selection.
4. Service default project is scoped to client/session context and must not become an unsafe global mutation target.
5. Every mutating operation requires `project_ref`, `session_id`, `request_id`, and idempotency fields.
6. `detach` removes the service binding but leaves project files and project DB untouched.
7. `archive` hides or de-prioritizes a registry entry while preserving history and receipts.
8. `forget` removes the service registry entry only after confirmation and must not delete the project root.
9. Project root deletion is out of MVP scope and should require a separate future explicit destructive-flow design.
10. Registry reconciliation is read-only unless the operator confirms a repair/apply plan.

### Finding 32: Project Registry API Needs Lifecycle Operations Before TUI Implementation

Problem:

- `list` and `register` alone are insufficient for a TUI that manages connected projects.
- The operator needs explicit flows for discovery, active selection, repair, detach, archive, and conflict resolution.

Recommendation:

- Extend the initial operation catalog before building TUI screens:

```text
vida.project.registry.list
vida.project.registry.get
vida.project.registry.discover
vida.project.registry.register
vida.project.registry.update
vida.project.registry.reconcile
vida.project.registry.set_active
vida.project.registry.detach
vida.project.registry.archive
vida.project.registry.restore
vida.project.registry.forget
vida.project.registry.health
```

Project lifecycle states:

```text
registry_status:
  connected
  archived
  detached
  unhealthy_missing_root
  unhealthy_inaccessible
  conflict_duplicate_project_id
  conflict_duplicate_root

activation_status:
  not_activated
  activation_pending
  activated
  reconfigure_pending
  activation_blocked

service_binding_status:
  not_bound
  bound_current_service
  bound_stale_service
  bound_foreign_service
  binding_conflict
```

Rules:

1. `project_ref` accepts project id, canonical root path, or registry entry id, but service normalizes it before mutation.
2. Duplicate project id or duplicate root blocks mutation until an explicit resolve/repair plan is accepted.
3. Missing root marks the registry entry unhealthy and offers reconnect/detach/archive; it does not auto-forget.
4. Foreign or stale service binding is shown as a repairable state, not silently overwritten.
5. Active project selection is per client/session; CLI commands may also pass explicit `--project` to override cwd resolution.
6. Registry operations emit service events and service receipts; project-scoped activation/reconfigure operations also emit project receipts.

### Finding 33: Multi-Project TUI Proof Needs Conflict Fixtures

Problem:

- A single healthy project fixture will not prove the project management surface.
- The riskiest UX failures are stale registry entries, duplicate identities, wrong active project, and ambiguous DB/state ownership.

Recommendation:

- Build TUI fixture coverage before daemon-backed tests:

```text
FixtureVidaClient.projects
  healthy_project
  activation_pending_project
  missing_root_project
  duplicate_project_id_pair
  stale_service_binding_project
  archived_project
  db_locked_project
```

Proof targets:

1. Projects table renders health, activation, binding, and selected-project state at narrow terminal widths.
2. Wizard can start from connect existing, activate new, repair reconnect, and reconfigure connected paths.
3. Conflict detail panes show duplicate id/root without choosing an unsafe default.
4. Mutating actions show preview/diff/plan before apply.
5. Event log and job progress stay scoped to the selected project unless the operator opens a service-wide view.
6. Snapshot tests cover the fixture states before live daemon smoke tests.

### Proposed Clarifications For Approval, Set 5

1. Make `Projects` a top-level TUI area and make project selection/connection the first wizard gate before activation.
2. Add registry lifecycle operations before TUI implementation: discover/get/update/reconcile/set_active/detach/archive/restore/forget/health.
3. Define detach/archive/forget as non-destructive service-registry actions; project root deletion is out of MVP.
4. Scope active project selection per client/session, with CLI cwd resolution and explicit `--project` override.
5. Require fixture-client proof for healthy, pending, missing-root, duplicate, stale-binding, archived, and DB-locked project states.

### Finding 34: Project Initialization Wizard Must Configure The Development Environment And Agent Topology

Problem:

- Current activation already has more configuration authority than project id and docs scaffolding.
- `vida.config.yaml` owns host systems, execution classes, materialization roots, dispatch adapters, carrier catalogs, model profiles, reasoning settings, normalized cost units, role/task-class bindings, agent-extension registries, dev-team flow, autonomous execution policy, and routing policies.
- A wizard that only asks for project identity would activate an incomplete project and would hide the most important operator choices.

Observed configuration authority:

```text
vida.config.yaml
  project / project_bootstrap / language_policy
  host_environment
    cli_system
    systems
      codex: internal
      hermes: external
      opencode: external
      pi: external/config_projection_only plus pi_cli write-guarded adapter
    carrier catalogs
    dispatch commands/adapters
    materialization roots
  party_chat
    multi-agent board limits
    role_model_bindings
  agent_extensions
    roles / skills / profiles / flows / dispatch_aliases registries
    enabled framework/project roles, skills, profiles, flows
    validation fail-closed policy
  dev_team
    analyst -> developer -> duplication_reviewer -> coach -> tester -> prover -> release_closure
    default carriers, models, reasoning effort, budget units, handoff contracts
  agent_system
    scoring
    pricing
    model_selection
    subagents
    routing
```

Wizard map:

```text
ActivationWizard
  1. Project identity
     project_id, title/name, language policy, docs/process/research roots
  2. Project registry binding
     connect/register project, select active project for this client/session
  3. Development environment selection
     select host CLI system: codex, hermes, opencode, pi, or hybrid profile
     choose internal / external / hybrid posture
     validate platform-specific command and auth readiness
  4. Agent system mode
     agent_system.mode, init_on_boot, max_parallel_agents, state owner
     autonomous_execution and agent-only development policy
  5. Carrier catalog
     enabled carriers and subagent backends
     internal Codex: junior/middle/senior/architect
     external read/review: hermes_cli/opencode_cli/kilo_cli/vibe_cli
     external write-guarded: pi_cli
  6. Model and reasoning profile selection
     model_ref, reasoning_effort, thinking_level where supported
     speed/quality tier
     write_scope and readiness requirements
  7. Cost policy
     normalized_cost_units
     provider price-source freshness
     max budget units per route/role
     escalation-over-budget policy
  8. Role and task-class activation
     framework roles: orchestrator, worker, business_analyst, pm, solution_architect, coach, verifier, prover
     project roles/profiles/skills from agent_extensions registries
     task_classes and role compatibility validation
  9. Flow selection
     minimal / reviewed / verified
     dev_team.default_delivery
     party_chat council flows
  10. Dispatch alias and route plan
      development_specification, development_execution_preparation, development_implementer,
      development_coach, development_verifier, development_escalation
      route policies: research, analysis, review, implementation, architecture, verification
  11. Materialization plan
      generate/update AGENTS sidecar, docs, agent projections, .codex/.pi/.opencode/.hermes roots
      service install/configuration operations when selected
  12. Diff/validate/apply
      show config diff, generated files, readiness blockers, command detection, auth/model checks
      apply idempotently with receipts and events
```

UI rules:

1. The wizard should present simple presets first: `Internal Codex`, `External Read/Review`, `Hybrid External-First`, and `Hybrid Write-Guarded Pi`.
2. Advanced mode exposes the underlying carriers, models, reasoning efforts, cost units, roles, flows, and routing policies.
3. The TUI must show the active development environment on the project detail screen.
4. The TUI must show readiness per carrier: command found, auth present, model selected, write guard available, and projection materialized.
5. The wizard must fail closed when `agent_extensions.validation` or `dev_team.validation` fails.
6. Materialized host files are projections; `vida.config.yaml` and agent-extension registries remain authority.
7. Existing project settings must be read first and offered as current defaults during reconfigure.

### Proposed Clarifications For Approval, Set 6

1. Expand activation wizard from project onboarding into project + development-environment + agent-topology onboarding.
2. Add wizard presets: `Internal Codex`, `External Read/Review`, `Hybrid External-First`, and `Hybrid Write-Guarded Pi`.
3. Make `vida.config.yaml` plus agent-extension registries the source of truth for wizard options.
4. Add readiness checks for selected host systems, dispatch adapters, auth files, model refs, write guards, projections, role/profile/flow validation, and cost policy.
5. Show active development environment and carrier readiness in the TUI project detail view.
6. Include development-environment reconfiguration in the same plan/diff/validate/apply/receipt lifecycle as project activation.

### Finding 35: Wizard Options Need A Typed Dependency Graph, Not A Flat Form

Problem:

- Development environment options depend on each other: selected host systems constrain carriers; carriers constrain models; models constrain reasoning and cost; roles constrain profiles; profiles constrain flows; write routes constrain readiness and guards.
- A linear form would either show invalid combinations or hide why an option is unavailable.

Recommendation:

- Represent wizard controls as a schema-driven option graph:

```text
WizardOptionSpec
  option_id
  label
  value_type
  source_path
  default_source
  dependencies
  visibility_rule
  enabled_rule
  validation_rule
  materialization_targets
  migration_behavior
```

Required value types:

```text
single_choice       one selected value from a finite set
multi_select        zero or more selected values from a finite set
boolean_toggle      true/false
text                free text with validation
slug                project-safe id
path                file or directory path, absolute/relative policy explicit
integer             bounded numeric value
duration            timeout/runtime limit
budget_units        normalized cost budget
enum_matrix         matrix of role/task/profile bindings
model_profile       provider/model/reasoning/cost/write-scope profile
derived             computed read-only value
secret_reference    auth/token path or provider-managed credential reference
diff_preview        generated plan/diff output
```

Dependency map:

```text
project_identity
  -> project_registry_binding
  -> development_environment_preset
  -> host_systems
  -> carriers
  -> model_profiles
  -> reasoning_controls
  -> cost_policy
  -> roles_and_task_classes
  -> profiles_and_skills
  -> flows
  -> dispatch_aliases
  -> route_policies
  -> materialization_targets
  -> readiness_checks
  -> activation_plan
```

Key dependency rules:

1. `development_environment_preset` is a `single_choice`; it sets defaults but does not replace advanced configuration.
2. `host_environment.cli_system` is a `single_choice` current primary system.
3. `host_environment.systems.*.enabled` is `multi_select` in advanced mode.
4. `execution_class` is constrained by host system: Codex internal, Hermes/OpenCode external, Pi external with projection/write-guard adapter.
5. Carriers are available only when their host system is enabled and their readiness checks can be evaluated.
6. Model options are available for every carrier, but the valid model set is carrier/profile scoped.
7. `reasoning_effort` is enabled only when the selected profile supports it; Pi also maps to `thinking_level`.
8. Write-producing profiles require `write_scope != none` and any required write guard to be available.
9. Role enablement depends on available carriers that can serve the role's runtime role and task classes.
10. Skill enablement depends on compatible base roles.
11. Profile enablement depends on role and skill resolution.
12. Flow enablement depends on all roles/profiles/dispatch aliases in the flow resolving.
13. Dispatch aliases depend on a carrier tier or backend that exists in the selected catalog.
14. Route policies depend on executor backend, fallback backend, budget, and write/readiness constraints.
15. Service install options depend on platform, service runtime mode, binary fingerprint, and operator permission.

### Finding 36: Config And Materialized Files Need Version Pins And Projection Manifests

Problem:

- Current root docs commonly carry artifact metadata, and `.codex` TOML files state that they are projections, but generated TOML projections do not carry machine-readable source version, source digest, rendered digest, template version, or update policy.
- Without projection manifests, the update wizard cannot safely report what is stale, locally edited, missing, or newly required.

Recommendation:

- Add a config manifest to `vida.config.yaml` and a projection manifest to every materialized file or sidecar.

Config manifest shape:

```yaml
vida_config_manifest:
  schema_id: vida.project.config
  schema_version: 1
  config_version: 1
  option_graph_version: 1
  template_set_version: 2026-05-21
  generated_by: vida project-activator
  generated_at: 2026-05-21T00:00:00Z
  vida_version: <binary-version>
  source_template_digest: sha256:<digest>
  materialization_generation: 1
```

Projection manifest shape:

```yaml
vida_projection_manifest:
  artifact_path: .codex/agents/junior.toml
  artifact_kind: host_agent_projection
  artifact_schema_version: 1
  template_set_version: 2026-05-21
  source_config_path: vida.config.yaml
  source_config_digest: sha256:<digest>
  source_registry_digests:
    roles: sha256:<digest>
    skills: sha256:<digest>
    profiles: sha256:<digest>
    flows: sha256:<digest>
    dispatch_aliases: sha256:<digest>
  source_template_digest: sha256:<digest>
  rendered_digest: sha256:<digest>
  rendered_at: 2026-05-21T00:00:00Z
  generated_by: vida project-activator
  ownership: vida_projection
  local_edit_policy: preserve_or_report_conflict
  update_policy: rerender_when_source_or_template_changes
```

Rules:

1. Markdown files may use existing footer metadata plus changelog.
2. TOML files use comment-header metadata.
3. YAML files use top-level `vida_projection_manifest` or a sidecar when the target format must remain pure.
4. JSON files use a reserved metadata field or a sidecar when the consumer forbids extra fields.
5. Projection-owned files can be cleanly regenerated when their source digest and rendered digest still match.
6. Locally edited projection files become `drift_local_edit` and require preserve/overwrite/merge choice.
7. Missing projection files become `missing_materialized_file`.
8. New template options become `new_option_available`.
9. Removed template options become `deprecated_option_present`.
10. All update/reconfigure applies emit receipts and events with per-file actions.

### Finding 37: Update Wizard Must Inspect, Diff, Reconcile, And Optionally Clean-Update Everything

Problem:

- Reconfiguration is not only changing `vida.config.yaml`; it must compare current config, template versions, registries, and all generated files.
- Operators need a simple "update everything to the new version" path with a clear report, and a safer drift-aware path when local edits exist.

Recommendation:

- Add an update/reconfigure wizard mode:

```text
UpdateWizard
  1. inspect_current
     read config manifest, registries, service registry entry, materialized projections
  2. resolve_target
     current VIDA version, template set version, option graph version, selected preset
  3. compare
     config schema drift, template drift, registry drift, projection drift, local edits
  4. classify
     clean_current, update_available, missing_file, local_edit_conflict,
     deprecated_option, new_option, invalid_dependency, readiness_blocker
  5. plan
     update config, migrate schema, rerender projections, preserve local edits,
     add new files, remove or archive obsolete projections
  6. diff
     show config diff, file diff, new options, removed/deprecated options, readiness changes
  7. validate
     schema, dependencies, carriers, models, auth, write guards, service install
  8. apply
     idempotent writes with backup/receipt/event/job progress
  9. report
     updated, added, unchanged, skipped, conflicted, deprecated, failed
```

Clean update mode:

```text
clean_update_all
  allowed when projection files are unchanged from their recorded rendered digest
  rerenders all projection-owned files from the selected target template set
  updates config manifest and materialization_generation
  adds newly required files/options
  marks deprecated options
  writes a complete update receipt
```

Conflict mode:

```text
drift_aware_update
  preserves local edits by default
  reports conflicts with exact file/action reason
  offers overwrite/skip/export-diff/manual-resolve per artifact
  never silently deletes local changes
```

Report shape:

```text
ActivationUpdateReport
  config_version_before / after
  template_set_version_before / after
  option_graph_version_before / after
  selected_preset
  files_updated
  files_added
  files_removed_or_archived
  files_unchanged
  files_skipped
  local_edit_conflicts
  new_options
  deprecated_options
  readiness_changes
  receipts
```

### Approved Clarifications, Set 7

Approved by operator on 2026-05-21:

1. Model wizard configuration as typed option graph with dependency, visibility, enabled, validation, and materialization rules.
2. Treat all model choices as carrier/profile-scoped; every carrier can expose models, but only valid model profiles appear for its runtime role, task class, write scope, reasoning support, and budget.
3. Add `vida_config_manifest` to `vida.config.yaml` with schema/config/template/option-graph versions and source digest.
4. Add machine-readable projection manifests to every materialized file or sidecar, including source config digest, registry digests, template digest, rendered digest, ownership, and update policy.
5. Add update/reconfigure wizard modes: inspect, compare, classify, plan, diff, validate, apply, report.
6. Support `clean_update_all` when projection files are unchanged from recorded rendered digest; otherwise use drift-aware update with per-file conflict choices.
7. TUI must expose update status: current, update available, local edit conflict, missing projection, deprecated option, new option, readiness blocker.

### Finding 38: Wizard And Update Logic Need Service-Owned Operations, Not UI-Local Reimplementation

Problem:

- The TUI and CLI must not independently parse `vida.config.yaml`, infer dependencies, compute projection drift, or decide update legality.
- The wizard is an operator surface; the service/activation core must own schema resolution, dependency validation, readiness checks, diff generation, materialization, receipts, and events.
- If each client implements these rules, service, TUI, CLI, and future dashboard will drift.

Recommendation:

- Add wizard/config/materialization operations to the VIDA command catalog and expose them through the shared `VidaClient`.

Operation catalog additions:

```text
vida.wizard.schema.get
vida.wizard.session.start
vida.wizard.session.get
vida.wizard.session.update_input
vida.wizard.session.validate
vida.wizard.session.plan
vida.wizard.session.apply
vida.wizard.session.cancel

vida.project.config.inspect
vida.project.config.option_graph
vida.project.config.validate
vida.project.config.diff
vida.project.config.update.plan
vida.project.config.update.apply

vida.project.materialization.inspect
vida.project.materialization.diff
vida.project.materialization.update.plan
vida.project.materialization.update.apply
vida.project.materialization.report.get
```

Rules:

1. `vida.wizard.schema.get` returns typed option metadata, dependency edges, presets, control types, and source paths.
2. TUI renders controls from schema and wizard session state; it does not hardcode option legality.
3. CLI can use the same operations for non-interactive activation/reconfigure with supplied inputs.
4. Service owns plan/diff/validate/apply and emits events for progress.
5. Wizard sessions are resumable, scoped by `session_id`, `project_ref`, `wizard_kind`, and `wizard_session_id`.
6. Wizard session state is service-level for disconnected/pending projects and project-scoped after a project is registered.
7. A wizard session can be read-only inspect/diff, mutation plan, or apply job.

### Finding 39: Wizard Session State Needs Typed Inputs, Derived State, And Explainable Disabled Options

Problem:

- Dependency-driven controls need a way to explain why an option is hidden, disabled, defaulted, invalid, or blocked.
- The TUI needs this to render a predictable operator console; CLI needs it for actionable error output.

Recommendation:

- Define wizard session state as a first-class contract:

```text
WizardSessionState
  wizard_session_id
  wizard_kind
  session_id
  project_ref
  current_step
  selected_preset
  inputs
  derived_values
  option_states
  validation_findings
  readiness_findings
  diff_summary
  plan_ref
  apply_job_ref
  receipt_refs
```

Option state:

```text
WizardOptionState
  option_id
  value
  effective_value
  source
  visible
  enabled
  required
  dirty
  valid
  blocked_reason
  warning_reason
  dependency_inputs
  affected_materialization_targets
```

Rules:

1. Disabled options must include an external reason such as missing dependency, unsupported carrier, failed readiness, budget cap, or write guard unavailable.
2. Derived values are read-only and must show their source.
3. Default values must distinguish config default, preset default, inferred environment default, and existing project value.
4. Invalid dependency combinations return structured `VidaProblem` entries, not free-text only errors.
5. Update mode must show new/deprecated options even when they are not selected.

### Finding 40: Apply Must Be A Job With Idempotency, Receipts, And Rollback-Aware Reporting

Problem:

- Activation/update applies may touch config, docs, sidecar, host projections, registry entries, service install configuration, and project state.
- A synchronous UI action cannot safely own the whole mutation or recover progress after TUI disconnect.

Recommendation:

- Treat `wizard.session.apply`, `project.config.update.apply`, and `project.materialization.update.apply` as service jobs:

```text
WizardApplyJob
  job_id
  wizard_session_id
  project_ref
  plan_id
  plan_hash
  apply_token
  idempotency_key
  operation_scope
  status
  current_stage
  file_actions
  config_actions
  registry_actions
  service_actions
  rollback_notes
  receipt_refs
  event_cursor
```

Rules:

1. Apply requires a validated `plan_id`, `plan_hash`, and `apply_token`.
2. Repeated apply with the same idempotency key returns the same job/result.
3. Each file action records before/after digest where possible.
4. Backups are created for overwritten projection-owned files unless clean regeneration policy proves they are disposable.
5. Rollback is best-effort and explicit; successful partial apply must be reported as partial, not hidden.
6. TUI subscribes/polls events by cursor; CLI can wait or return the job id.
7. Final report separates updated, added, archived, skipped, conflicted, failed, and unchanged artifacts.

### Approved Clarifications, Set 8

Approved by operator on 2026-05-21:

1. Add service-owned wizard/config/materialization operation catalog before TUI implementation.
2. Make TUI and CLI consume `VidaClient` wizard schema/session/plan/apply operations instead of parsing config and projection drift locally.
3. Represent wizard sessions as resumable `WizardSessionState` scoped by `session_id`, `project_ref`, `wizard_kind`, and `wizard_session_id`.
4. Require explainable option states for hidden/disabled/invalid/defaulted controls.
5. Treat apply as a service job with `plan_id`, `plan_hash`, `apply_token`, `idempotency_key`, events, receipts, and per-artifact report.
6. Support disconnected TUI recovery by reading wizard session, job state, event cursor, and final receipts from the service.

### Finding 41: TUI Needs A Stable Operator Shell, Not Independent Screen Silos

Problem:

- Projects, wizard, update, jobs, logs, receipts, service diagnostics, and config topology all need the same context: selected project, current session, service health, active job/event cursor, and readiness blockers.
- If every screen owns its own connection/state, project switching and reconnect/resume behavior will drift.

Recommendation:

- Build the Ratatui app around one operator shell and shared app state:

```text
VidaTuiApp
  AppShell
    HeaderBar
      service_status
      selected_project
      session_id
      active_job
      update_status
    NavigationRail
      Projects
      Overview
      Wizard
      Update Center
      Config
      Agent Topology
      Materialization
      Jobs
      Receipts
      Logs
      Service
    MainPane
      active_screen
    SidecarPane
      events
      logs
      job_progress
      validation_findings
      receipt_preview
    FooterCommandBar
      key hints
      command palette
      connection/error status
```

Rules:

1. The shell owns service connection, project selection, current wizard session, event cursor, active job refs, and refresh cadence.
2. Screens are components over shared state and `VidaClient` commands.
3. Sidecar pane is a persistent operator aid, not the project `AGENTS.sidecar.md`; it shows contextual events/logs/findings.
4. Narrow terminals can collapse `NavigationRail` and `SidecarPane`; content remains accessible through focus panels.
5. Mutating actions are never triggered from passive sidecar rows; they route through plan/diff/apply screens.

### Finding 42: TUI Screen Map Must Separate Read Models From Mutating Workflows

Problem:

- Operator screens mix read-only observability, plan generation, and apply jobs. Without explicit separation, the UI may accidentally make a status panel into a mutation surface.

Recommendation:

- Split screens into read-only dashboards, wizard workflows, and apply/job views:

```text
Projects
  read: list connected projects, active project, health, activation, binding, DB/state paths
  actions: connect, discover, register, set active, repair, detach, archive, restore, forget

Overview
  read: service/project/session status, readiness, current blockers, recent jobs/events
  actions: refresh, open wizard, open update center, open diagnostics

Wizard
  workflow: project activation, environment setup, agent topology, service install, reconfigure
  actions: update input, validate, plan, diff, apply

Update Center
  workflow: config/projection/template/version reconcile
  actions: inspect, clean_update_all, drift_aware_update, per-file conflict choice

Config
  read: effective config, option graph, current values, source paths, versions
  actions: open reconfigure wizard

Agent Topology
  read: carriers, models, roles, skills, profiles, flows, dispatch aliases, route policies
  actions: open topology reconfigure wizard

Materialization
  read: AGENTS, sidecar, docs, .codex/.pi/.opencode/.hermes projections, manifests, drift
  actions: update/re-render through plan/apply only

Jobs
  read: queued/running/completed jobs, stages, event cursor
  actions: cancel where cancellable, open report, retry through a new plan only

Receipts
  read: service and project receipts, filters, final reports
  actions: export/open receipt

Logs
  read: event stream, service logs, project-scoped logs
  actions: filter/follow/pause

Service
  read: daemon health, transport, service home, install status, version, binary fingerprint
  actions: install/start/stop/reconfigure through service plan/apply where supported
```

Rules:

1. Read-only screens can refresh directly.
2. Any mutation opens a workflow screen and requires plan/diff/validate/apply.
3. Dangerous lifecycle actions such as `forget` require explicit confirmation and remain non-destructive to project root in MVP.
4. Project switch always updates header, sidecar context, event filters, and active wizard/job context.
5. Service-wide views must visually distinguish service-level events from selected-project events.

### Finding 43: TUI Control Mapping Should Be Generated From Option Types

Problem:

- The option graph has explicit value types; the TUI should not hand-code a different form model.

Recommendation:

- Map option value types to Ratatui controls:

```text
single_choice     list/select/radio group
multi_select      checklist table
boolean_toggle    checkbox/toggle row
text              single-line input
slug              validated single-line input
path              path input plus optional explorer
integer           bounded numeric input
duration          numeric input with unit selector
budget_units      numeric input plus budget summary
enum_matrix       editable table with validation markers
model_profile     searchable table grouped by carrier/provider
derived           read-only value row
secret_reference  masked path/status row, no secret display
diff_preview      scrollable diff pane
```

Rules:

1. Disabled controls show `blocked_reason` inline or in the sidecar findings pane.
2. Dirty values show source and effective value before plan generation.
3. Validation findings are tied to option ids and focusable from the sidecar.
4. Model profile tables show provider, model ref, reasoning, thinking level, cost units, quality/speed, write scope, readiness.
5. Matrix controls are used only when dependencies are inherently cross-product, such as role/task/profile bindings.

### Finding 44: TUI Runtime State Machine Needs Reconnect And Resume Paths

Problem:

- TUI can disconnect while a wizard session or apply job is active.
- Terminal UI can be closed without cancelling service jobs.

Recommendation:

- Use an explicit TUI runtime state machine:

```text
starting
  -> connecting
  -> service_hello
  -> project_select
  -> ready
  -> wizard_active
  -> job_active
  -> disconnected
  -> reconnecting
  -> resume_session_or_job
  -> ready
```

Rules:

1. TUI startup calls `vida.service.hello`, then project registry/list, then restores last selected project for the current `session_id` when safe.
2. If an apply job is active, TUI resumes through job state and event cursor.
3. If a wizard session is active but no apply job exists, TUI resumes the draft session.
4. Disconnection does not cancel jobs by default.
5. Cancel is explicit and routed to job/session cancel operations.
6. Event polling/subscription is cursor-based and can replay missed events after reconnect.
7. Stale sessions are shown as recover/restart choices, not auto-merged.

### Approved Clarifications, Set 9

Approved by operator on 2026-05-21:

1. Build `vida tui` around an `AppShell` with header, navigation, main pane, persistent sidecar pane, and footer command bar.
2. Treat sidecar pane as contextual events/logs/findings/receipt preview, separate from project `AGENTS.sidecar.md`.
3. Split TUI screens into read-only dashboards, workflow screens, and job/report screens; every mutation routes through plan/diff/validate/apply.
4. Generate wizard controls from option value types and option states instead of hand-coded form legality.
5. Add reconnect/resume state machine for service connection, selected project, wizard sessions, apply jobs, and event cursors.
6. Prove the TUI shell and main screens with fixture `VidaClient` snapshots before live daemon smoke tests.

### Finding 45: Implementation Boundaries Should Be Staged Around Contracts First

Problem:

- The workspace already has split crates for TaskFlow and DocFlow, but service/TUI/wizard dependencies are not present yet in `crates/vida/Cargo.toml`.
- Adding Ratatui, tarpc, service install, config migration, and wizard engines directly into the existing launcher surface would increase coupling and make it hard for CLI, TUI, service, and future dashboard to share semantics.
- A full crate explosion before contracts are stable would also be premature.

Recommendation:

- Use a staged boundary plan:

```text
Stage 1: vida-contracts crate
  pure serde contracts only
  no Ratatui
  no tarpc
  no service runtime
  no filesystem mutation

Stage 2: internal vida modules
  vida_client
  vida_wizard_core
  vida_config_graph
  vida_materialization
  vida_service_runtime
  vida_service_state
  vida_project_registry
  vida_jobs
  vida_events

Stage 3: transport adapters
  in_process_client
  tarpc_client/server over local IPC
  jsonrpsee adapter later

Stage 4: UI clients
  vida tui command in the main vida binary first
  browser dashboard later through jsonrpsee/HTTP adapter

Stage 5: crate extraction only after contracts stabilize
  optional vida-service-core
  optional vida-tui
  optional vida-dashboard-api
```

Rules:

1. `vida-contracts` is the only immediate new crate required.
2. `crates/vida` remains the one-binary host for `vida service run` and `vida tui` during MVP.
3. Ratatui dependencies must not enter `vida-contracts`.
4. Tarpc-specific generated service traits must not become the semantic contract; they carry `VidaCommandEnvelope`.
5. Wizard, config graph, materialization, project registry, jobs, events, and service runtime modules are internal implementation boundaries until their APIs prove stable.

### Finding 46: `vida-contracts` Needs A Minimal But Complete Contract Surface

Problem:

- The service/TUI work needs shared types before implementation begins.
- If types are defined separately in CLI, service, TUI, and tests, the system will drift.

Recommendation:

- Define `vida-contracts` around stable semantic contracts:

```text
identity
  VidaSessionId
  VidaRequestId
  VidaProjectRef
  VidaProjectId
  VidaClientKind

envelope
  VidaCommandEnvelope
  VidaCommandResponse
  VidaProblem
  VidaOperation
  VidaIdempotencyKey

events_receipts
  VidaEvent
  VidaEventCursor
  VidaReceiptRef
  VidaReceiptSummary

project_registry
  ServiceProjectRegistryEntry
  ProjectRegistryStatus
  ProjectActivationStatus
  ServiceBindingStatus
  ProjectHealthSummary

wizard
  WizardKind
  WizardSessionId
  WizardOptionSpec
  WizardOptionValue
  WizardOptionState
  WizardSessionState
  WizardValidationFinding
  WizardReadinessFinding

config_update
  VidaConfigManifest
  VidaProjectionManifest
  MaterializationArtifactStatus
  ActivationUpdateReport

planning_jobs
  VidaPlanRef
  VidaPlanSummary
  VidaDiffSummary
  VidaApplyToken
  VidaJobRef
  WizardApplyJob
  VidaJobStatus
```

Rules:

1. Types derive `Serialize`, `Deserialize`, `Clone`, `Debug`, `PartialEq` where appropriate.
2. Contract tests use golden JSON fixtures before service/TUI implementation.
3. Operation ids are string-compatible for CLI/dashboard while still represented as typed constants/enums.
4. Schema generation can be added later; JSON fixtures are the first proof target.
5. Contracts must represent both service-level and project-level scopes.

### Finding 47: Core Modules Should Own Semantics Before Transport And TUI

Problem:

- If the TUI is implemented before the core modules, it will pressure the codebase toward UI-shaped semantics.
- If tarpc is implemented before the command envelope/client trait, transport can become the hidden API.

Recommendation:

- Add internal modules in this order:

```text
vida_client
  VidaClient trait
  InProcessVidaClient
  FixtureVidaClient

vida_config_graph
  reads vida.config.yaml and agent-extension registries
  builds WizardOptionSpec graph
  evaluates dependencies and option states

vida_materialization
  computes config/projection manifests
  inspects materialized files
  classifies drift
  builds update report

vida_wizard_core
  wizard session state machine
  input updates
  validation/readiness
  plan/diff/apply job creation

vida_project_registry
  connected projects
  active project per session/client
  lifecycle operations

vida_service_state
  append-only service registry/events/jobs/receipts/idempotency store

vida_jobs
  job lifecycle
  idempotency
  progress events

vida_events
  persisted cursor-based events
```

Rules:

1. `FixtureVidaClient` is built before Ratatui screens.
2. `InProcessVidaClient` exists before tarpc so CLI/TUI proof can run without daemon.
3. Tarpc adapter wraps the same `VidaClient` semantics; it does not define new behavior.
4. The service runtime calls core modules; it does not duplicate wizard/config/materialization logic.
5. CLI thin-client conversion starts with read/status/inspect paths before mutation.

### Finding 48: Implementation Proof Gates Should Follow The Dependency Chain

Problem:

- This feature spans contracts, config parsing, projection manifests, service state, jobs/events, and TUI. A single integration proof at the end would be too late.

Recommendation:

- Use proof gates aligned to the implementation chain:

```text
Gate 1: contract fixtures
  vida-contracts compiles
  golden JSON for envelope, option graph, project registry, wizard session, apply job

Gate 2: config graph
  existing vida.config.yaml resolves into option graph
  invalid dependencies produce structured VidaProblem

Gate 3: materialization inspect
  existing .codex projections classify as projection_without_manifest
  future manifested fixtures classify clean/current/drift/missing/new/deprecated

Gate 4: wizard core
  start/update/validate/plan session with fixture project
  disabled options expose blocked reasons

Gate 5: service state
  project registry, jobs, events, receipts persist and replay by cursor

Gate 6: client abstraction
  FixtureVidaClient and InProcessVidaClient pass same behavior tests

Gate 7: transport smoke
  Tarpc local IPC carries VidaCommandEnvelope and event cursor calls

Gate 8: TUI snapshots
  AppShell, Projects, Wizard, Update Center, Agent Topology, Jobs, Logs render from fixture client

Gate 9: live daemon smoke
  service hello, project registry list, wizard schema get, update inspect, events_since
```

Rules:

1. Ratatui work starts only after Gate 6.
2. Live daemon tests start only after Gate 7.
3. Mutating apply smoke tests wait until idempotency and receipts are proven.
4. Existing direct CLI mutation paths are not removed until the service client path has equivalent proof and explicit fallback policy.

### Approved Clarifications, Set 10

Approved by operator on 2026-05-21:

1. Add `vida-contracts` as the first crate split; keep it pure serde/contracts without Ratatui, tarpc, filesystem mutation, or service runtime.
2. Keep `vida service run` and `vida tui` in the main `vida` binary for MVP; defer separate `vida-service`/`vida-tui` crates or binaries until contracts stabilize.
3. Build internal modules first: client trait, config graph, materialization, wizard core, project registry, service state, jobs, events.
4. Implement `FixtureVidaClient` and `InProcessVidaClient` before tarpc and TUI screens.
5. Let tarpc carry `VidaCommandEnvelope`; do not make tarpc traits the semantic API.
6. Use staged proof gates from contract fixtures through live daemon smoke before mutating apply workflows are considered ready.

### Finding 49: CLI Migration Needs A Command Classification Matrix

Problem:

- Current `vida` CLI has many direct runtime/store surfaces: `project-activator`, `task`, `status`, `doctor`, `agent`, `orchestrator-session`, `release`, `docs`, and TaskFlow/DocFlow proxy families.
- Moving every command behind service IPC at once would be high risk.
- Leaving every command as direct local mutation would violate the service/TUI architecture goal.

Recommendation:

- Classify CLI commands by service migration posture:

```text
direct_only_bootstrap
  vida init
  vida boot
  vida service run
  vida service install/start/stop/status/uninstall/logs
  emergency/offline repair commands

service_first
  vida tui
  vida project list/get/status
  vida project connect/register/discover/reconcile/detach/archive/restore/forget
  vida wizard ...
  vida config inspect/validate/diff/update
  vida materialization inspect/diff/update
  vida jobs ...
  vida receipts ...
  vida events ...

service_preferred_with_direct_fallback
  vida status
  vida doctor
  vida project-activator
  selected read-only task/status projections after service store routing exists

direct_until_taskflow_service_adapter_exists
  vida task ...
  vida taskflow ...
  vida agent-init
  vida agent ...
  vida orchestrator-session ...
  vida lane/approval/recovery/consume proxies

external_family_direct
  vida docflow ...
  vida docs update
  vida protocol view
  vida release install
```

Rules:

1. Bootstrap and service lifecycle commands cannot depend on a running service.
2. New project/wizard/config/materialization commands should be service-first from the start.
3. Existing mutation-heavy TaskFlow commands stay direct until service routing can preserve TaskFlow law, claims, receipts, and performance.
4. CLI fallback must be explicit in output and receipts; no silent direct mutation when service mode was requested.
5. `--offline` or `--direct` should be an explicit diagnostic/recovery mode, not the normal path for service-owned commands.
6. `--service-required` should fail closed if the daemon is unavailable.

### Finding 50: CLI Needs Shared Context Flags For Service-Client Mode

Problem:

- CLI and TUI must resolve the same project/session/service context.
- Existing commands frequently accept `--state-dir`; service-client commands need richer context without overloading state-dir.

Recommendation:

- Add shared service-client context flags to new service-backed command families:

```text
--project <project-ref>
--session-id <vida-session-id>
--service-endpoint <auto|local-socket|pipe|tcp>
--service-home <path>
--offline
--direct
--service-required
--wait
--no-wait
--format <plain|json|jsonl|toon>
```

Resolution rules:

1. `--project` overrides current working directory detection.
2. Without `--project`, CLI resolves project from cwd and service registry.
3. `VIDA_SESSION_ID` is primary session identity; host-specific aliases normalize into it.
4. Service endpoint auto-resolution uses service home metadata and platform defaults.
5. `--wait` follows job/event progress until terminal state.
6. `--no-wait` returns job id, event cursor, and receipt refs where available.
7. `--format json` output must match `VidaCommandResponse` or typed result payloads.

### Finding 51: CLI Command Families Should Mirror Service Operations Without Exposing Transport

Problem:

- Operators need discoverable commands, but command names should not leak tarpc/jsonrpsee/IPC details.
- Commands should map to service operation ids and return predictable responses.

Recommendation:

- Add service-backed CLI families:

```text
vida service
  run
  install plan/apply
  start
  stop
  status
  uninstall plan/apply
  logs
  doctor

vida project
  list
  get
  status
  discover
  connect/register
  reconcile
  set-active
  detach
  archive
  restore
  forget

vida wizard
  schema
  start
  show
  set
  validate
  plan
  diff
  apply
  cancel

vida config
  inspect
  option-graph
  validate
  diff
  update plan/apply

vida materialization
  inspect
  diff
  update plan/apply
  report

vida jobs
  list
  get
  watch
  cancel

vida events
  since
  follow

vida receipts
  list
  get
  export
```

Rules:

1. CLI commands map one-to-one or many-to-one onto `VidaOperation` ids.
2. CLI help should show whether a command is service-first, direct-only, or fallback-capable.
3. Mutating commands use plan/apply split unless they are existing direct TaskFlow commands not yet migrated.
4. Apply commands require plan/apply tokens or explicit non-interactive confirmation.
5. Every service-backed command emits `request_id`, `session_id`, and `operation_id` in JSON output.

### Finding 52: CLI Migration Needs Compatibility And Deprecation Policy

Problem:

- Existing users and scripts may rely on current direct commands.
- Service-backed equivalents must not break automation by changing output shape without a migration window.

Recommendation:

- Use a compatibility policy:

```text
Phase A: additive
  add new service-backed families
  no existing direct command behavior changes
  expose service/direct posture in help/status

Phase B: preferred service path
  selected read/status commands try service first
  direct fallback is explicit and reported
  JSON output includes service_mode field

Phase C: mutation migration
  service-backed plan/apply equivalents for activation/config/materialization/project registry
  direct mutations remain only for bootstrap/offline recovery

Phase D: TaskFlow service adapter
  task/taskflow commands route through service only after TaskFlow semantics are preserved
  direct mode remains explicit diagnostic fallback
```

Rules:

1. Existing `vida project-activator` remains as bootstrap/direct fallback until wizard service path is green.
2. New `vida wizard`/`vida config`/`vida materialization` commands become the canonical interactive/non-interactive activation path after proof.
3. Help output and JSON output must make service/direct/fallback mode visible.
4. Deprecation notices require a replacement command and a stable compatibility window.
5. Tests must cover command help, JSON response envelopes, fallback mode, and service-required failure.

### Approved Clarifications, Set 11

Approved by operator on 2026-05-21:

1. Classify CLI commands into direct-only bootstrap, service-first, service-preferred-with-direct-fallback, direct-until-TaskFlow-service-adapter, and external-family-direct.
2. Add service-backed CLI families for `project`, `wizard`, `config`, `materialization`, `jobs`, `events`, and `receipts`.
3. Add shared service-client context flags: `--project`, `--session-id`, `--service-endpoint`, `--service-home`, `--offline`, `--direct`, `--service-required`, `--wait`, `--no-wait`, and output format selection.
4. Require service-backed JSON output to expose request/session/operation ids and service/direct/fallback mode.
5. Keep existing direct TaskFlow/activation mutation paths until equivalent service-client behavior has proof and explicit fallback/deprecation policy.
6. Make CLI help identify command posture so operators know whether a command requires daemon, can fallback, or is direct bootstrap/recovery.

### Finding 53: Service State Needs A Versioned File Layout And Single-Writer Contract

Problem:

- Earlier findings approved a small append-only service state store, but implementation still needs a precise layout and ownership model.
- Service state must not couple itself to project-local SurrealDB truth, but it still needs durable registry, jobs, events, receipts, idempotency, sessions, endpoint metadata, and compaction summaries.

Recommendation:

- Use a versioned `service-home` layout with one service process as writer:

```text
service-home/
  service.config.json
  service.manifest.json
  service.lock
  endpoint/
    current.json
    auth-token.json
  registry/
    projects.json
    projects.compact.json
    projects.events.jsonl
  sessions/
    sessions.jsonl
    sessions.compact.json
  jobs/
    jobs.jsonl
    jobs.compact.json
    active/
      <job-id>.json
  events/
    service-events.jsonl
    project-events.<project-id>.jsonl
    cursors.compact.json
  receipts/
    service-receipts.jsonl
    project-receipt-index.jsonl
  idempotency/
    service-idempotency.jsonl
    project-idempotency-index.jsonl
  materialization/
    projection-index.json
    manifests/
      <artifact-id>.json
  recovery/
    startup-recovery.jsonl
    crash-markers.jsonl
```

Rules:

1. `service.manifest.json` records schema version, service-home version, service instance id, created/updated timestamps, binary fingerprint, and last clean shutdown marker.
2. `service.lock` prevents concurrent writers to one service home.
3. All append-only files use schema-versioned JSONL records.
4. Compact files are derived projections and can be regenerated from append-only logs.
5. The service is the only writer to service-home state; CLI/TUI are clients.
6. Project-local DB/state remains authoritative for project-scoped runtime truth.
7. Service state may store indexes into project receipts/events, but must not become the owner of project TaskFlow truth before a TaskFlow service adapter exists.

### Finding 54: Service State Records Need Scope, Revision, And Digest Fields

Problem:

- Multi-project and multi-session operation requires every record to state which scope it belongs to and which revision it observes or mutates.
- Without digests and revisions, update/reconfigure/report flows cannot reliably classify stale, replayed, or drifted state.

Recommendation:

- Standardize common record fields:

```text
ServiceStateRecordCommon
  record_id
  record_kind
  schema_version
  service_instance_id
  scope_kind              service | project | session | job | artifact
  project_ref             optional
  session_id              optional
  request_id              optional
  operation_id            optional
  idempotency_key         optional
  created_at
  updated_at
  sequence
  resource_revision
  observed_config_digest  optional
  observed_registry_digest optional
  payload_digest
```

Record families:

```text
project_registry_entry
project_registry_event
session_record
job_record
job_stage_event
vida_event
receipt_index_record
idempotency_record
projection_manifest_record
recovery_marker
```

Rules:

1. `sequence` is monotonic per append-only file.
2. `resource_revision` is monotonic per logical resource such as registry entry, job, or wizard session.
3. Mutating operations perform optimistic revision checks where stale writes could overwrite newer state.
4. Digest fields are used for drift detection and idempotency, not as security boundaries.
5. Records with `project_ref` must be normalized before write.
6. Cross-scope operations record both service receipt refs and project receipt refs when both authorities are touched.

### Finding 55: Job/Event/Receipt Recovery Needs Deterministic Startup Reconciliation

Problem:

- The service can crash or be killed while jobs are running or after partial file/project writes.
- TUI/CLI must see deterministic recovery state, not ambiguous "maybe running" jobs.

Recommendation:

- On service startup, run recovery reconciliation:

```text
startup_recovery
  1. acquire service.lock
  2. read service.manifest.json
  3. detect unclean shutdown
  4. rebuild compact projections from JSONL if needed
  5. scan non-terminal jobs
  6. reconcile idempotency records and receipts
  7. classify jobs:
     completed
     failed_terminal
     failed_recoverable
     recovering
     resumable
     cancelled
  8. emit service recovery events
  9. update service.manifest.json clean startup state
```

Rules:

1. Jobs without terminal receipts after crash do not resume mutation automatically unless the job contract is explicitly resumable.
2. If an idempotency record has a terminal receipt, replay returns that terminal result.
3. If a file action has before/after digests but no terminal job receipt, recovery reports partial state and requires a repair plan.
4. Active TUI sessions after reconnect see recovery events and job classifications through `events_since`.
5. Recovery never deletes project files or registry entries silently.
6. Recovery markers are append-only and visible in service diagnostics.

### Finding 56: Service Locks Must Not Recreate Current Project State Lock Problems

Problem:

- Existing runtime specs already warn against holding authoritative project datastore locks across long-running agent execution.
- A service daemon could accidentally recreate this issue at service-home level if it holds a broad lock while waiting on jobs, dispatches, or file IO.

Recommendation:

- Use short critical sections:

```text
lock critical section
  append record
  update compact projection
  fsync/flush where needed
  release

outside lock
  run long job step
  wait on external process
  wait on IPC/client
  compute heavy diff
```

Rules:

1. Service-home lock is for state file mutation, not job execution lifetime.
2. Project-local DB locks are opened only for bounded project read/write phases.
3. Long-running jobs persist stage start, release locks, execute work, then reopen bounded state handles for stage result.
4. Status/read operations should use compact projections where possible to avoid blocking on append logs.
5. Lock contention is a service diagnostic blocker with explicit owner/process evidence.
6. TUI status must distinguish service-home lock contention from project DB lock contention.

### Approved Clarifications, Set 12

Approved by operator on 2026-05-21:

1. Use a versioned `service-home` layout with service manifest, endpoint metadata, project registry, sessions, jobs, events, receipts, idempotency, materialization manifests, and recovery logs.
2. Keep service-home state single-writer by the service process; CLI/TUI never write it directly.
3. Use schema-versioned append-only JSONL plus derived compact projections for MVP service state.
4. Standardize service state records with scope, project/session/request/operation ids, idempotency key, sequence, resource revision, and digest fields.
5. Add deterministic startup recovery that rebuilds compact projections, classifies non-terminal jobs, reconciles idempotency/receipts, and emits recovery events.
6. Keep service-home and project DB locks short-lived; no locks may be held across long-running jobs, external process waits, or client IPC waits.
7. TUI/CLI diagnostics must distinguish service-level lock/state recovery from project-local DB/state recovery.

### Finding 57: Endpoint Discovery Needs A Signed-By-State Metadata Contract

Problem:

- CLI/TUI need to find the correct daemon without trusting stale sockets, stale pipes, or another service instance using the same path.
- Endpoint metadata must be service-home state, but clients need enough read-only metadata to connect and verify identity.

Recommendation:

- Store endpoint metadata under service-home:

```text
endpoint/current.json
  endpoint_id
  service_instance_id
  service_home
  service_home_digest
  transport_kind           local_socket | named_pipe | loopback_tcp
  endpoint_name_or_addr
  auth_mode                os_user_boundary | local_token
  token_id                 optional
  protocol_version
  service_version
  binary_fingerprint
  pid
  started_at
  heartbeat_at
  stale_after_ms
  permissions_summary
```

Discovery order:

```text
1. explicit --service-endpoint / --service-home
2. VIDA_SERVICE_HOME
3. VIDA_HOME/service
4. release-install-root service home
5. platform default user service home
```

Rules:

1. Endpoint metadata is advisory until `vida.service.hello` confirms the same `service_instance_id`, service home, protocol version, and binary fingerprint.
2. Clients reject stale endpoints when heartbeat is beyond `stale_after_ms`.
3. Clients reject endpoint/service-home mismatches for mutating operations.
4. TCP fallback must never be selected silently; it requires explicit debug/config posture and local token auth.
5. Endpoint metadata does not grant write authority; it only locates the daemon.

### Finding 58: Service Hello Must Authenticate Context And Negotiate Capabilities

Problem:

- Earlier handshake notes covered version/capability negotiation, but endpoint security and session/project context need to be part of the same early exchange.
- Mutating operations must fail closed if the client is connected to the wrong service instance, wrong project, wrong protocol, or insufficient capability posture.

Recommendation:

- Extend `vida.service.hello`:

```text
request
  client_kind
  client_version
  protocol_version
  session_id
  project_ref optional
  requested_capabilities
  endpoint_id
  auth_token_proof optional

response
  service_instance_id
  service_version
  protocol_version
  service_home
  service_home_digest
  binary_fingerprint
  supported_operations
  supported_transports
  feature_flags
  auth_posture
  client_capabilities
  project_resolution
  session_resolution
  warnings
  blockers
```

Rules:

1. `hello` is required before any mutating command.
2. Read-only commands may degrade on version mismatch only when the response says the operation is read-compatible.
3. Auth posture distinguishes `os_user_boundary`, `local_token`, and future `remote_auth`.
4. `auth_token_proof` is required for loopback TCP fallback.
5. `project_resolution` must say whether `project_ref` resolved to a registered project, cwd project, missing project, or conflict.
6. `session_resolution` must say whether the session id is accepted, created, resumed, stale, or conflicting.
7. TUI displays `warnings` and `blockers` in the service/status sidecar before allowing apply.

### Finding 59: Local Auth Token Must Be Scoped, Rotatable, And Non-Secret In Logs

Problem:

- TCP fallback and possibly some local IPC environments need a token, but token handling can easily leak into logs, receipts, or TUI screens.

Recommendation:

- Store token metadata and secret separately:

```text
endpoint/auth-token.json
  token_id
  created_at
  rotated_at
  expires_at optional
  auth_scope
  token_digest
  token_material_path optional
  allowed_transports
  rotation_generation
```

Rules:

1. Receipts/logs/events may include `token_id` and `auth_scope`, never token material.
2. Token proof uses a challenge/response or equivalent proof, not raw token echo in ordinary request logs.
3. Token rotation invalidates old TCP fallback sessions after a bounded grace window.
4. Local socket/named pipe remains primary; token auth is primarily for loopback TCP fallback and explicit debug cases.
5. `vida service doctor` reports missing/stale token as a fallback readiness issue, not as a primary local IPC blocker.

### Finding 60: Permission And Capability Model Must Distinguish Read, Plan, Apply, And Admin

Problem:

- A local per-user daemon is not the same as "every client can mutate everything".
- TUI/CLI/dashboard surfaces need capability checks so future dashboard/auth work can reuse the model.

Recommendation:

- Define capability scopes:

```text
read_status
read_events
read_receipts
read_config
project_registry_read
project_registry_write
wizard_read
wizard_plan
wizard_apply
config_plan
config_apply
materialization_plan
materialization_apply
service_install_plan
service_install_apply
service_admin
diagnostic_detail
```

Rules:

1. MVP local CLI/TUI under the same OS user gets normal local capabilities, but apply/admin still pass through plan/apply tokens and service state checks.
2. Future dashboard clients can receive narrower scopes without changing operation semantics.
3. `diagnostic_detail` controls whether internal paths, process ids, and lock owner details are returned.
4. Capability denial returns structured `VidaProblem` with required scope and current auth posture.
5. Every receipt for mutating operations records client kind, session id, request id, operation id, capability scope, and auth posture.
6. Capability checks happen after endpoint/service identity and before claim/admission/project queue routing.

### Finding 61: Stale Or Foreign Daemon Detection Must Be First-Class

Problem:

- Multiple projects, multiple sessions, and versioned installs make stale daemon/socket situations likely.
- Connecting to a stale or foreign daemon could route operations to the wrong service home or wrong project registry.

Recommendation:

- Add explicit stale/foreign classifications:

```text
endpoint_status
  current
  stale_heartbeat
  pid_missing
  service_home_mismatch
  binary_fingerprint_mismatch
  protocol_incompatible
  permission_denied
  foreign_user_or_scope
  tcp_token_required
  tcp_token_invalid
```

Rules:

1. Mutating commands fail closed on anything except `current`.
2. Read-only diagnostics can connect to incompatible/stale endpoints only through explicit diagnostic mode where safe.
3. TUI service screen shows endpoint status, service instance id, service home, binary fingerprint, protocol version, and auth posture.
4. `vida service status` must report both endpoint metadata status and hello-confirmed status.
5. Stale endpoint cleanup is an explicit service command or recovery action; clients do not delete sockets/pipes/token files by hand.

### Approved Clarifications, Set 13

Approved by operator on 2026-05-21:

1. Add versioned endpoint metadata under service-home and require hello-confirmation before trusting it.
2. Extend `vida.service.hello` to include endpoint identity, service-home digest, auth posture, project resolution, session resolution, capabilities, warnings, and blockers.
3. Keep local socket/named pipe as primary IPC; loopback TCP is explicit debug/fallback only and requires local token auth.
4. Store local auth token metadata without leaking token material into logs/events/receipts/TUI.
5. Add capability scopes for read, plan, apply, service install/admin, and diagnostic detail.
6. Make stale/foreign daemon detection first-class and fail closed for mutating operations.
7. TUI and CLI service diagnostics must show endpoint status, auth posture, service instance id, service home, binary fingerprint, and protocol compatibility.

## Service Install, Update, And Reconfigure Lifecycle Research Pass

Date: 2026-05-21.

Purpose:

- Define how VIDA service installation, service update, daemon reconfiguration, rollback, and activation-wizard integration should work across Windows, Linux, and macOS.

Sources:

- `https://docs.rs/service-manager/latest/service_manager/index.html`
- `https://docs.rs/crate/windows-services/latest/source/`
- `https://learn.microsoft.com/en-us/windows/win32/services/about-services`
- `https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/sc-create`
- `https://www.freedesktop.org/software/systemd/man/253/systemd.service.html`
- `https://keith.github.io/xcode-man-pages/launchd.plist.5.html`
- `https://keith.github.io/xcode-man-pages/launchctl.1.html`

### Finding 62: Service Lifecycle Must Be A Declarative Plan, Not A Direct Command Wrapper

Evidence:

1. The Rust `service-manager` crate provides a cross-platform install/start/stop/uninstall abstraction over service managers such as Windows `sc.exe`, WinSW, launchd, systemd, OpenRC, and rc.d.
2. `service-manager` also supports a user-level service mode on platforms that expose it, notably systemd and launchd.
3. Native service managers have different semantics: Windows SCM stores service configuration in the service-control database, systemd uses unit files and manager state, and launchd uses plist files loaded into a launchd domain.

Problem:

- If VIDA exposes only `vida service install` as an immediate side effect, the wizard cannot preview permission requirements, generated manager files, binary fingerprint, service-home location, endpoint posture, or rollback risk.
- Install/update/reconfigure can change the active daemon endpoint or temporarily stop the service, so clients must treat it as an idempotent job with reconnect semantics.

Recommendation:

- Model service lifecycle as first-class plan/apply operations:

```text
vida.service.install.inspect
vida.service.install.plan
vida.service.install.apply
vida.service.status
vida.service.start
vida.service.stop
vida.service.restart
vida.service.update.plan
vida.service.update.apply
vida.service.reconfigure.plan
vida.service.reconfigure.apply
vida.service.rollback.plan
vida.service.rollback.apply
vida.service.uninstall.plan
vida.service.uninstall.apply
```

Rules:

1. `install.plan`, `update.plan`, `reconfigure.plan`, `rollback.plan`, and `uninstall.plan` are read/plan operations.
2. `install.apply`, `update.apply`, `reconfigure.apply`, `rollback.apply`, and `uninstall.apply` require service admin capability, an apply token, and an idempotency key.
3. Every lifecycle mutation is recorded as a service-level job, receipt, event stream, and service-home generation.
4. The TUI never shells out directly to platform service tools; it asks the service/client core for a plan and then applies that plan.
5. CLI may provide direct bootstrap execution only for the pre-daemon install path, but it must still produce the same plan, receipt, and manifest shapes.

### Finding 63: Platform Service Adapters Must Be Behind A VIDA Lifecycle Contract

Problem:

- A single cross-platform crate is useful for installation/control, but VIDA still needs canonical diffs, state manifests, endpoint validation, and update recovery.
- Windows is not equivalent to Linux/macOS user-level service install. A native Windows Service has SCM runtime expectations; a foreground/session daemon is easier but does not satisfy the final service-install target.

Recommendation:

- Introduce a platform adapter boundary:

```text
ServiceLifecycleAdapter
  inspect_platform()
  inspect_current_install()
  render_install_plan(desired)
  apply_install(plan, token)
  status()
  start()
  stop()
  restart()
  render_update_plan(desired)
  apply_update(plan, token)
  render_reconfigure_plan(desired)
  apply_reconfigure(plan, token)
  render_uninstall_plan(desired)
  apply_uninstall(plan, token)
```

Initial adapter set:

```text
SystemdUserAdapter
SystemdSystemAdapter              later/elevated
LaunchdAgentAdapter
LaunchdDaemonAdapter              later/elevated
WindowsForegroundSessionAdapter   bootstrap/MVP fallback
WindowsNativeServiceAdapter       target Windows service path
WindowsWinSwAdapter               optional wrapper path if native SCM runtime is not ready
```

Implementation posture:

1. Use `service-manager` as the first install/control helper where it fits.
2. Use native rendering/inspection around `service-manager` so VIDA owns the plan, diff, manifest, and receipts.
3. Use `windows-service` or equivalent direct SCM integration for the Windows native service runtime path when Windows service behavior must be implemented inside `vida service run`.
4. Keep the Windows foreground/session daemon as a bootstrap and no-admin fallback, but do not treat it as satisfying the final Windows service-install requirement.
5. Each adapter reports `supported`, `requires_elevation`, `requires_gui_session`, `requires_login_session`, `requires_system_manager`, `can_autostart`, `can_restart_on_failure`, and `can_collect_native_logs`.

### Finding 64: Service Install State Needs Versioned Generations

Problem:

- Service install state is not just "installed true/false".
- The active install binds a binary path/fingerprint, service manager file or SCM record, service-home path, endpoint transport, launch args, environment, autostart policy, and schema generation.

Recommendation:

- Store a service install manifest under service-home:

```text
service-install/current.json
  installation_id
  generation_id
  previous_generation_id optional
  platform
  platform_manager
  service_level              foreground_session | user | system
  service_label
  service_display_name
  binary_path
  binary_fingerprint
  binary_version
  service_home
  service_home_digest
  working_directory
  args
  environment_digest
  endpoint_transport
  endpoint_name_or_addr
  autostart
  restart_policy
  manager_artifact_path optional
  manager_artifact_digest optional
  installed_at
  updated_at
  last_status_at
  desired_state
  observed_state
  rollback_available
```

Rules:

1. Every install/update/reconfigure writes a new generation, not an in-place opaque mutation.
2. The manifest records the generated systemd unit, launchd plist, Windows service configuration, WinSW config, or foreground-session descriptor digest.
3. Drift detection compares desired generation, observed platform manager state, endpoint hello, and binary fingerprint.
4. `vida service status` and TUI service status must show install generation and drift status.
5. Project registry state remains separate; service install generation owns only service process/platform configuration.

### Finding 65: Update And Rollback Need A Two-Phase Service Handoff

Problem:

- Updating the daemon can require replacing the running binary or changing service manager configuration.
- The service may drop IPC during its own update, so a normal request/response apply is not enough.

Recommended update flow:

```text
1. current service builds update.plan
2. update.plan stages new binary/config into a versioned staging directory
3. current service records pre-update job state, apply token, target generation, and rollback generation
4. apply enters external handoff if running binary/service manager state must be replaced
5. update helper or bootstrap CLI stops service, swaps manager config/binary pointer, starts service
6. new service performs hello/status self-check and completes the job/receipt
7. TUI/CLI reconnects by endpoint metadata and job id
8. failure triggers rollback.plan/apply or leaves rollback_available with explicit blocker
```

Rules:

1. Never silently overwrite the running binary in place.
2. Always compute and record target binary fingerprint before apply.
3. Service-home schema migration is a separate planned step inside update/reconfigure, with backup and recovery markers.
4. The service must not hold project DB or service-home locks across stop/start or external helper waits.
5. If update cannot complete because the client disconnected, the next service startup resumes or classifies the non-terminal update job.
6. Rollback applies the previous generation and validates it through endpoint hello and service status.

### Finding 66: Activation Wizard Needs A Service Runtime Stage

Problem:

- The project activation wizard now owns service install/configuration, but service state depends on OS, permissions, existing installs, endpoint health, and update policy.

Recommended wizard stage:

```text
Service Runtime
  mode:
    attach_existing
    install_user_service
    install_system_service
    foreground_session
    skip_for_now
    repair_existing
    reconfigure_existing
  service_home
  service_label
  autostart
  restart_policy
  endpoint_transport
  tcp_fallback_enabled
  local_token_rotation
  log_retention
  event_retention
  update_policy
  apply_permission_mode
```

Option dependencies:

1. `install_user_service` is enabled only when the platform adapter reports user-level service support.
2. `install_system_service` requires elevated/admin posture and a clear blocker if not available.
3. `foreground_session` is available as a bootstrap/no-admin mode but is reported separately from installed service modes.
4. `attach_existing` requires endpoint metadata plus hello-confirmed identity.
5. `repair_existing` and `reconfigure_existing` require an observed install generation or manager artifact.
6. TCP fallback options are disabled unless the endpoint/auth stage enables local token auth.
7. Update policy is disabled until binary fingerprint and install generation tracking are active.

Wizard output:

```text
ActivationPlan.service_lifecycle_operations[]
  inspect
  install_plan
  attach_plan
  repair_plan
  reconfigure_plan
  update_plan
  skip_record
```

### Finding 67: TUI Needs A Service Lifecycle Console, Not Only Wizard Screens

Problem:

- Service install/update/reconfigure is operational state that can drift after activation.
- Operators need to see whether the daemon, endpoint, registry, jobs, and logs are healthy before applying project changes.

Recommendation:

- Add a top-level `Service` area in the Ratatui console:

```text
Service Overview
  daemon status
  endpoint status
  service home
  service instance id
  install generation
  binary fingerprint
  platform manager
  auth posture
  protocol compatibility
  project registry health

Service Jobs
  install/update/reconfigure/rollback progress
  current job phase
  reconnect/resume state
  last receipt
  blockers

Service Drift
  desired generation
  observed manager state
  observed endpoint state
  binary mismatch
  config mismatch
  stale manager artifact

Service Logs
  service events
  lifecycle receipts
  native log hints
```

Rules:

1. TUI apply progress follows service jobs/events by cursor and job id.
2. If service restart drops IPC, TUI enters reconnect mode using endpoint metadata and the update job id.
3. TUI never hides permission/elevation blockers behind generic "failed install" text.
4. The persistent sidecar shows service blockers whenever project wizard apply depends on service health.

### Finding 68: Service Lifecycle Security Must Be Capability-Scoped

Problem:

- Service install/update/reconfigure can change executable paths, autostart behavior, endpoint exposure, token posture, and log locations.
- These operations are more privileged than project config planning.

Rules:

1. `service_install_plan`, `service_update_plan`, `service_reconfigure_plan`, and `service_uninstall_plan` are plan capabilities.
2. `service_install_apply`, `service_update_apply`, `service_reconfigure_apply`, `service_rollback_apply`, and `service_uninstall_apply` are admin apply capabilities.
3. Receipts include client kind, session id, request id, operation id, capability, platform manager, service level, binary fingerprint, target generation, and auth posture.
4. Diagnostics redact token material and sensitive environment values by default.
5. Generated manager artifacts must not be group/world writable where the target platform forbids it.

### Approved Clarifications, Set 14

Approved by operator on 2026-05-21:

1. Service install/update/reconfigure/rollback must use the same declarative `inspect -> plan -> diff -> validate -> apply -> receipt` lifecycle as activation.
2. Add a `ServiceLifecycleAdapter` boundary with systemd, launchd, Windows foreground-session, Windows native service, and optional WinSW adapters.
3. Use `service-manager` as an install/control helper where it fits, but keep VIDA-owned manifests, diffs, idempotency, receipts, and endpoint validation.
4. Treat Windows native service install as a target requirement; foreground/session daemon remains only bootstrap/no-admin fallback.
5. Store versioned service install generations under service-home and use them for drift detection, update, reconfigure, and rollback.
6. Service self-update uses a two-phase handoff with staged binary/config, pre-update receipt, external helper/bootstrap apply when needed, reconnect by endpoint metadata, and post-update hello health check.
7. Add a dedicated activation wizard `Service Runtime` stage with attach/install/foreground/skip/repair/reconfigure choices and platform-dependent option gating.
8. Add a TUI `Service` console area for daemon status, endpoint status, install generation, jobs, drift, receipts, and logs.
9. Service lifecycle mutations require admin apply capabilities and must expose permission/elevation blockers explicitly.

## Multi-Project Registry And DB/Filesystem Sync Research Pass

Date: 2026-05-21.

Purpose:

- Define how one VIDA service manages multiple connected projects while preserving project-local DB authority, project-specific filesystem materialization, per-session routing, and safe concurrent TUI/CLI access.

Sources:

- `vida.config.yaml`
- `docs/product/research/db-authority-and-migration-runtime-research.md`
- `docs/process/team-development-and-orchestration-protocol.md`
- `docs/process/project-orchestrator-operating-protocol.md`
- `crates/vida/src/project_root_paths.rs`
- `crates/taskflow-cli/src/lib.rs`
- `https://docs.rs/notify/latest/notify/`
- `https://docs.rs/notify-debouncer-mini/latest/notify_debouncer_mini/`
- `https://docs.rs/ignore/latest/ignore/`

Local evidence:

1. `vida.config.yaml` has `project.id` and a large project-owned host/agent/runtime configuration surface.
2. The activated project root is currently recognized by `AGENTS.md`, `vida.config.yaml`, `.vida/config`, `.vida/db`, and `.vida/project`.
3. The standalone TaskFlow wrapper already binds missing `VIDA_STATE_DIR` to `<project-root>/.vida/data/state` after resolving the active project root.
4. DB authority research says VIDA needs one project-local authoritative DB truth under `.vida/db/**`, with project activation state and runtime operational state imported before trusted execution.
5. Process docs require session-scoped multi-orchestrator behavior: one blocked session must not block another disjoint session in the same project root.

External evidence:

1. `notify` is the current cross-platform Rust filesystem notification crate and exposes `recommended_watcher`, but its own docs list platform caveats: network filesystems, WSL paths, editor save behavior, parent deletion behavior, inotify limits, and large directory reliability.
2. `notify-debouncer-mini` filters incoming notify events and emits one event per timeframe per file, which matches service-side dirty-state detection needs better than raw event floods.
3. `ignore` provides fast recursive directory walking that respects `.gitignore`/ignore-style filters, suitable for project scans and resync baselines.

### Finding 69: Service Registry Is Discovery Authority, Not Project Runtime Truth

Problem:

- A single daemon needs a list of connected projects, but a connected-project record cannot replace the project's own `.vida/db/**` runtime truth.
- If service registry becomes the source of truth for TaskFlow/activation/runtime rows, multi-project support will centralize state incorrectly and break project portability.

Recommendation:

- Split authority:

```text
service-home/project-registry
  owns: project discovery, attachment state, health summary, last observed revision, active claims, service-level jobs, registry receipts

project-root/.vida/db
  owns: project activation state, TaskFlow state, runtime operational state, project receipts, memory state, protocol binding state

project-root/.vida/project
  owns: editable/materialized project projection family

project-root/vida.config.yaml
  owns: project config source and activation inputs
```

Rules:

1. Service registry can cache project summaries, but project-local DB remains authority for project-scoped execution.
2. Registry mutations write service receipts; project mutations write project receipts and may also emit service-level summary events.
3. Removing a project from service registry does not delete project DB/files.
4. A project can be detached from service and later re-attached by identity proof.

### Finding 70: Project Identity Needs Stable And Observed Components

Problem:

- Paths move, git remotes change, branch/worktree context changes, and copied projects can duplicate `project.id`.
- The service must distinguish "same project moved" from "different project with same config id" from "same filesystem path with changed identity".

Recommendation:

- Define `ProjectIdentity`:

```text
project_identity
  project_id                  from vida.config.yaml
  project_instance_id          generated at activation under .vida/project/identity.json
  project_root
  project_root_canonical
  project_root_digest
  config_digest
  activation_revision
  db_identity_digest
  db_schema_version
  vcs_kind optional
  vcs_worktree_id optional
  vcs_remote_fingerprint optional
  display_name
  created_at
  last_attached_at
```

Rules:

1. `project_id` is human/config identity, not globally unique enough by itself.
2. `project_instance_id` is generated once per activated project instance and is the primary service registry identity key.
3. `project_root` is mutable attachment evidence, not identity authority.
4. Re-attachment with same `project_instance_id` but different root is a `project_moved` plan, not a silent update.
5. Same root with different `project_instance_id` is a conflict that requires explicit repair/rebind.
6. Same `project_id` with different `project_instance_id` is allowed, but the TUI must display disambiguating names/roots.

### Finding 71: Project Registry Entries Need Lifecycle State

Recommendation:

- Store registry entries as versioned records:

```text
project_registry/current.jsonl
  registry_entry_id
  project_instance_id
  project_id
  display_name
  project_root
  state_dir
  db_root
  config_path
  activation_status
  attachment_status
  health_status
  last_seen_at
  last_hello_at
  last_scan_at
  last_event_seq
  current_session_count
  current_claim_count
  active_job_count
  drift_status
  archived
  detached
  capabilities_summary
```

Lifecycle states:

```text
attachment_status
  discovered
  attached
  detached
  missing_root
  identity_conflict
  db_unreadable
  activation_pending
  migration_required
  archived

health_status
  healthy
  warning
  blocked
  unknown
```

Rules:

1. `register_project` creates or updates a registry entry through an identity-aware plan.
2. `attach_project` validates identity, DB readiness, config digest, and activation status.
3. `detach_project` stops service management for the project without deleting project files.
4. `archive_project` hides the entry by default but preserves receipt history.
5. `forget_project` removes the registry entry only after explicit confirmation and never deletes the project root.

### Finding 72: Active Project Resolution Must Be Request-Scoped

Problem:

- TUI can show multiple projects; CLI can run from a cwd; dashboard can select a project id; sessions can switch active project.
- A daemon-global "current project" would create cross-session bugs.

Recommendation:

- Every request that touches project scope carries `ProjectRef`:

```text
project_ref
  kind:
    cwd
    project_instance_id
    project_id
    registry_entry_id
    explicit_root
    session_active_project
  value optional
  cwd optional
  require_registered
  allow_discovery
```

Resolution output:

```text
project_resolution
  status:
    resolved_registered
    resolved_discovered_unregistered
    missing
    ambiguous
    identity_conflict
    activation_pending
    db_blocked
  project_instance_id optional
  registry_entry_id optional
  project_root optional
  db_root optional
  blockers[]
  warnings[]
```

Rules:

1. CLI default resolution starts from cwd and then registry identity.
2. TUI sends explicit `project_instance_id` for selected project.
3. A session may have `active_project_id`, but that is session state, not daemon-global state.
4. Mutating commands require `resolved_registered` unless the operation is project registration/activation bootstrap.
5. Ambiguous `project_id` requires explicit project instance selection.

### Finding 73: Per-Project Actors/Queues Protect Project DB Without Blocking Other Projects

Problem:

- One service can receive concurrent TUI/CLI/dashboard operations.
- Project-local DB and materialized files require serialized mutations per project, while read/status events should remain responsive.

Recommendation:

- Use one project actor/queue per attached project:

```text
ProjectActor(project_instance_id)
  read_status lane
  mutation_queue
  filesystem_sync lane
  event_cursor
  claim_table
  db_connection_pool or db_handle
```

Rules:

1. Mutating project operations route through the project's mutation queue.
2. Mutations in different projects can run concurrently when service resources allow.
3. Project reads may run concurrently unless a migration/exclusive claim is active.
4. Long-running jobs must not hold DB or filesystem locks across external process waits.
5. Claims are scoped by project, session, task/run, conflict domain, and owned path set.
6. Foreign-session blockers are visibility by default; they block only on overlapping claims or global integrity blockers.

### Finding 74: DB/Filesystem Sync Should Be Event-Assisted, Not Event-Trusted

Problem:

- Filesystem watchers are not fully reliable across all platforms and filesystems.
- Editor save behavior can generate create/delete/rename/write sequences instead of one clean write event.
- Network/WSL/container filesystems may miss events.

Recommendation:

- Use watchers for dirty hints and explicit scans for authority:

```text
filesystem_sync
  baseline_scan using ignore::WalkBuilder
  watch_hints using notify
  debounce using notify-debouncer-mini
  dirty_set
  bounded_rescan
  materialization_manifest_compare
  projection_drift_report
```

Rules:

1. `notify` events mark paths dirty; they do not directly mutate authority.
2. Debounced dirty paths trigger bounded rescans and manifest comparisons.
3. Full scans run on attach, service startup recovery, update/reconfigure completion, watcher overflow/error, and explicit user refresh.
4. Scan filters respect `.gitignore` plus VIDA-owned include/exclude rules; `.vida/db/**` is handled by DB adapters, not normal projection scanning.
5. Watcher failures degrade to polling/rescan mode with a visible TUI/CLI warning.
6. DB truth wins over stale materialized files except for editable source surfaces that require import/reconcile plans.

### Finding 75: Materialized Files Need Per-Artifact Manifests And Drift Policy

Problem:

- The wizard will drop docs, agents, sidecar files, configs, and projections.
- Later updates must know what VIDA generated, what the user edited, and what belongs to the current version.

Recommendation:

- For each generated artifact, record:

```text
materialization_manifest
  artifact_id
  artifact_kind
  project_instance_id
  source_template_id
  source_template_version
  source_template_digest
  target_path
  target_path_policy
  generated_at
  generated_by_version
  content_digest_at_generation
  last_observed_digest
  user_edit_policy
  update_policy
  conflict_policy
```

Drift states:

```text
materialization_drift
  unchanged
  user_modified
  missing
  template_updated
  generated_obsolete
  conflict_requires_merge
  unmanaged_existing_file
```

Rules:

1. Update wizard never overwrites `user_modified` artifacts without an explicit merge/replace choice.
2. Clean update can replace generated-only unchanged files automatically when version changes.
3. New options/templates appear as `new_artifact` or `new_config_option` plan entries.
4. Materialization receipts list added, updated, skipped, conflicted, deleted-from-generation, and user-preserved files.

### Finding 76: TUI Project Management Needs Registry, Health, Drift, And Claims Views

Recommendation:

- TUI should have a first-class `Projects` area:

```text
Projects List
  project display name
  project id
  project instance id short
  root
  attachment status
  activation status
  health
  drift
  active sessions
  active jobs

Project Detail
  identity
  config digest
  DB readiness
  filesystem watcher state
  materialization drift
  active claims
  sessions
  recent receipts/events
  available actions

Project Actions
  register
  attach
  detach
  archive
  forget
  repair identity
  refresh scan
  run activation wizard
  run reconfigure/update wizard
```

Rules:

1. TUI project selection updates session active project only, not daemon-global state.
2. Project wizard starts from a selected project or explicit "new/discovered project" flow.
3. CLI and TUI changes in the same project converge through the same project actor, DB, event log, and receipts.
4. The sidecar shows project-specific blockers only for the selected project and service/global blockers separately.

### Approved Clarifications, Set 15

Approved by operator on 2026-05-21:

1. Service-level project registry owns discovery/attachment/health/claims summaries, but project-local `.vida/db/**` remains authority for project runtime truth.
2. Add stable `project_instance_id` generated at activation; `project_id` remains human/config identity and path remains attachment evidence.
3. Project registration/attach/detach/archive/forget are service lifecycle operations with receipts and no destructive project file deletion.
4. Every project-scoped request carries `ProjectRef`; CLI defaults from cwd, TUI sends explicit `project_instance_id`, and session active project is request/session state only.
5. Use one per-project actor/queue so project mutations serialize inside one project but different projects can progress concurrently.
6. Claims and blockers are scoped by project, session, task/run, conflict domain, and owned paths; foreign sessions block only on overlap or global integrity.
7. DB/filesystem sync uses filesystem events as dirty hints only; authority comes from DB reads, manifest comparison, bounded rescans, and receipts.
8. Use `notify` plus debouncing for watcher hints and `ignore` for baseline/rescan traversal with project include/exclude rules.
9. Materialized docs/agents/config files get per-artifact manifests, source template versions, generation digests, drift states, and update/merge policies.
10. TUI gets a first-class `Projects` area with list/detail/actions, health, drift, active sessions, active jobs, claims, receipts, and wizard entry points.

## Jobs, Events, Receipts, And Logs Research Pass

Date: 2026-05-21.

Purpose:

- Define the shared operational truth model for wizard apply progress, service lifecycle operations, multi-project mutations, reconnect/resume, audit receipts, and diagnostic logs.

Sources:

- `docs/product/research/db-authority-and-migration-runtime-research.md`
- `docs/process/project-orchestrator-operating-protocol.md`
- `docs/process/team-development-and-orchestration-protocol.md`
- `crates/vida/src/state_store_run_graph_state.rs`
- `crates/vida/src/state_store_run_graph_summary.rs`
- `crates/vida/src/state_store_task_reconciliation.rs`
- `https://docs.rs/tracing/latest/tracing/`
- `https://docs.rs/tracing-subscriber/latest/tracing_subscriber/`
- `https://docs.rs/tracing-appender/latest/tracing_appender/`

Local evidence:

1. Existing runtime code already has specialized receipts and summaries for run-graph dispatch, approval delegation, replay lineage, projection checkpoints, migration, and task reconciliation.
2. DB authority research states that successful runtime state transitions require receipts and that lack of receipt means the transition is not trusted as complete.
3. Current receipt concepts are strong but domain-specific; the service/TUI layer needs a shared operation/job/event envelope that can reference those domain receipts without replacing them.

External evidence:

1. `tracing` is the Rust ecosystem's structured instrumentation framework for event-based diagnostics and spans.
2. `tracing-subscriber` supplies subscribers/layers for collecting and formatting `tracing` data.
3. `tracing-appender` provides non-blocking and rolling file appenders, which fits service-local diagnostic log files.

### Finding 77: Jobs Are Operational Progress, Receipts Are State-Transition Proof

Problem:

- Wizard apply, service install/update, materialization, project attach, DB migration, and filesystem reconciliation can be long-running and reconnect-prone.
- Treating the final command response as the only truth makes TUI progress fragile and makes service restart recovery ambiguous.

Recommendation:

- Introduce a shared `VidaJob` record:

```text
job
  job_id
  operation_id
  request_id
  session_id
  client_kind
  scope_kind                 service | project | session | global
  project_instance_id optional
  operation_kind
  plan_id optional
  plan_hash optional
  idempotency_key optional
  apply_token_id optional
  status                     queued | admitted | running | waiting | reconnecting | cancelling | cancelled | succeeded | failed | blocked | abandoned_recoverable | abandoned_terminal
  phase
  progress_current optional
  progress_total optional
  started_at optional
  updated_at
  completed_at optional
  result_receipt_id optional
  failure_problem optional
  blockers[]
  warnings[]
```

Rules:

1. Every mutating apply creates or resumes a job before mutation begins.
2. Job status is progress truth, not proof of successful state transition.
3. A success receipt is required before a job can be trusted as completed state change.
4. Failed/cancelled/blocked jobs may have audit receipts, but those do not count as successful mutation receipts.
5. Jobs are idempotency-aware: same `idempotency_key` plus same `plan_hash` returns the existing terminal or active job.

### Finding 78: Events Need Stable Cursors Across Service And Project Scope

Problem:

- TUI requires progress, warnings, blockers, logs hints, and reconnect recovery.
- Tarpc-first MVP should avoid depending on complex streaming before the event contract is stable.

Recommendation:

- Persist append-only event records:

```text
event
  event_id
  sequence
  scope_kind                 service | project | job | session
  project_instance_id optional
  job_id optional
  request_id optional
  session_id optional
  operation_id optional
  event_kind
  severity                   trace | debug | info | warn | error
  phase optional
  message
  structured_payload
  redaction_class            public | operator | diagnostic_detail | secret_forbidden
  receipt_id optional
  occurred_at
```

Cursor model:

```text
event_cursor
  scope_kind
  scope_id
  sequence
```

Rules:

1. `events_since(cursor, filter)` is the MVP API for TUI/CLI progress and reconnect.
2. Long-poll can be added over the same cursor contract.
3. Service and project event sequences may be separate, but each cursor must be monotonic within its scope.
4. Events are not authority by themselves; receipts and DB state decide final truth.
5. Event retention compaction must preserve receipt references and terminal job summaries.

### Finding 79: Receipts Need A Common Envelope With Domain-Specific Payloads

Problem:

- Existing receipts are domain-specific. Service lifecycle, activation, materialization, registry, session, and project claims need the same audit envelope so TUI/CLI can list and inspect them consistently.

Recommendation:

- Add `VidaReceiptEnvelope`:

```text
receipt
  receipt_id
  receipt_kind
  receipt_schema_version
  operation_id
  job_id optional
  request_id
  session_id
  client_kind
  scope_kind
  project_instance_id optional
  plan_id optional
  plan_hash optional
  idempotency_key optional
  capability_scope
  auth_posture
  subject_refs[]
  artifact_refs[]
  before_digests[]
  after_digests[]
  result_status              succeeded | failed | cancelled | superseded | blocked
  problem optional
  payload
  recorded_at
```

Rules:

1. Domain-specific receipts remain valid but should be wrapped or referenced by the common envelope.
2. Success receipts are immutable state-transition proof.
3. Failure receipts are audit proof of attempted work and blocker classification, not completion proof.
4. Supersession links must identify the superseded receipt/job/operation.
5. Receipts must not contain token material, secret environment values, or raw private model prompts unless explicitly allowed by a diagnostic-detail policy.

### Finding 80: Logs Are Diagnostics, Not Runtime Authority

Problem:

- Operators need logs, but logs are too verbose and unstable to drive TUI state or lifecycle recovery.
- Secrets and local paths can leak if logs are treated as raw user-facing output.

Recommendation:

- Use structured `tracing` instrumentation internally and write service-local rolling logs:

```text
logs/
  service.current.jsonl
  service.YYYY-MM-DD.N.jsonl
  project-<project_instance_id>.current.jsonl optional
```

Rules:

1. Logs include `request_id`, `session_id`, `operation_id`, `job_id`, and `project_instance_id` when available.
2. TUI default progress comes from jobs/events, not raw logs.
3. Logs are accessed through diagnostic APIs with redaction and retention policy.
4. `diagnostic_detail` capability controls process ids, full paths, lock owner details, and verbose stack/error payloads.
5. Log retention is service configurable and separate from receipt/event retention.

### Finding 81: Recovery Needs Deterministic Non-Terminal Job Classification

Problem:

- Service restart during update/apply can leave active jobs without a final response.
- TUI reconnect must know whether the job is still running, succeeded, failed, or needs repair.

Recommendation:

- Startup recovery reads jobs, events, receipts, service manifests, project DB state, and endpoint state, then classifies each non-terminal job:

```text
recovery_classification
  resumed_running
  completed_by_receipt
  failed_with_problem
  blocked_needs_operator
  abandoned_recoverable
  abandoned_terminal
  superseded
```

Rules:

1. Recovery never marks success without a success receipt or authoritative DB/manifest proof plus a recovery receipt.
2. Recovery emits events and a recovery receipt for every changed non-terminal job.
3. Service update handoff jobs are recovered by target generation and endpoint hello status.
4. Project jobs are recovered by project-local DB state plus materialization manifest comparison.
5. Recovery is bounded and visible in `vida service status`, TUI Service Jobs, and diagnostics.

### Finding 82: Cancellation Is Cooperative And Phase-Aware

Problem:

- Some operations can be cancelled safely before mutation, while others are in a critical section and must complete, roll forward, or roll back.

Recommendation:

- Add cancellation state to job phases:

```text
cancellation
  not_requested
  requested
  acknowledged
  refused_critical_section
  completed_cancelled
  completed_roll_forward
  completed_rollback
```

Rules:

1. Plan/inspect jobs can usually cancel immediately.
2. Apply jobs cancel only at declared cancellation points.
3. Critical sections report `refused_critical_section` and continue to a safe terminal state.
4. Cancellation writes events and a terminal receipt when it changes observable state.
5. TUI shows cancel availability per job phase rather than a universal cancel button.

### Finding 83: TUI Progress Should Be Built From Job Snapshots Plus Event Cursor

Recommendation:

- TUI uses:

```text
jobs.list(filter)
jobs.get(job_id)
events_since(cursor, filter)
receipts.get(receipt_id)
receipts.list(scope/filter)
logs.tail(filter) diagnostic only
```

Rules:

1. Wizard apply screen pins one active job and consumes scoped events by cursor.
2. Reconnect flow reloads `jobs.get(job_id)` first, then resumes `events_since(last_cursor)`.
3. Project and Service sidecars show active blockers from job/problem state, not parsed log text.
4. Terminal success screen links to receipt summary and materialization/service/project diff summary.
5. Failed apply screen shows `VidaProblem`, blockers, safe retry posture, and receipt references.

### Finding 84: Retention And Compaction Need Separate Policies

Problem:

- Events and logs can grow quickly, but receipts and terminal job summaries are audit and recovery-critical.

Recommendation:

```text
retention_policy
  receipts: keep by default, compact only with explicit archive/export
  terminal_job_summaries: keep
  events: retain by count/time per scope, preserve terminal summaries
  logs: rolling files by size/time, redact by default
  diagnostic_exports: explicit operator action
```

Rules:

1. Event compaction cannot remove the only link between a terminal job and its receipt.
2. Receipt archive/export is a planned operation with its own receipt.
3. TUI shows retention health and warns when diagnostic logs are unavailable due to rotation.
4. `vida service doctor` reports retention misconfiguration and failed compaction/recovery jobs.

### Approved Clarifications, Set 16

Approved by operator on 2026-05-21:

1. Add a shared `VidaJob` model for long-running service/project/session operations; jobs are progress truth, not final proof.
2. Success receipts remain mandatory state-transition proof; failed/cancelled receipts are audit evidence only.
3. Add append-only scoped `VidaEvent` records with monotonic cursors and `events_since(cursor, filter)` as the MVP TUI progress API.
4. Add `VidaReceiptEnvelope` so service lifecycle, activation, materialization, registry, session, and project receipts can be listed and inspected consistently.
5. Keep existing domain receipts, but wrap/reference them from the common receipt envelope instead of replacing them.
6. Use structured `tracing` plus rolling service-local logs for diagnostics; TUI state must come from jobs/events/receipts, not parsed logs.
7. Startup recovery classifies non-terminal jobs deterministically and emits recovery events/receipts.
8. Cancellation is cooperative and phase-aware; critical sections may refuse cancellation and roll forward/rollback to a safe terminal state.
9. TUI progress screens use `jobs.get`, `events_since`, `receipts.get/list`, and diagnostic-only `logs.tail`.
10. Define separate retention policies for receipts, terminal job summaries, events, logs, and diagnostic exports.

## Wizard And Reconfigure Engine Research Pass

Date: 2026-05-21.

Purpose:

- Define the canonical wizard session, option graph, draft config, validation, diff, apply, update, and reconfigure architecture for project activation and service-managed VIDA runtime configuration.

Sources:

- `docs/product/spec/project-activation-and-configurator-model.md`
- `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
- `docs/product/research/embedded-runtime-bootstrap-and-projection-research.md`
- `docs/process/codex-agent-configuration-guide.md`
- `vida.config.yaml`
- `crates/vida/src/project_activator_surface.rs`
- `crates/vida/src/project_activator_host_cli_materialization.rs`
- `crates/vida/src/host_runtime_materialization.rs`

Local evidence:

1. Project activation law is DB-first: SurrealDB/project DB is operational truth, filesystem artifacts are editable projections, and Git is backup/history.
2. Active project-owned runtime surfaces should converge under `.vida/project/**` plus DB truth; root `vida.config.yaml` and root registries are bridge-compatible source/export surfaces, not final active runtime authority.
3. `vida.config.yaml` already contains multi-system host environment options, materialization modes, carrier catalogs, model profiles, readiness checks, pricing/freshness policy, roles, skills, profiles, flows, dev-team roles, and agent-system settings.
4. Current activator/materialization code can render host CLI templates, but the future wizard needs a resumable plan/apply engine rather than direct one-shot template mutation.

### Finding 85: Wizard Must Be A Service-Owned Session, Not A TUI Form

Problem:

- TUI may disconnect, daemon may restart, and CLI/dashboard may need to resume or inspect the same activation/reconfigure flow.
- If wizard state lives only in the Ratatui process, long-running apply, validation blockers, and update prompts cannot be recovered.

Recommendation:

- Introduce `WizardSessionState` under service/project authority:

```text
wizard_session
  wizard_session_id
  wizard_kind              project_activation | project_reconfigure | service_runtime | materialization_update | repair
  session_id
  client_kind
  project_instance_id optional
  service_instance_id
  status                   draft | validating | ready_to_plan | planned | applying | applied | blocked | abandoned | superseded
  current_step_id
  option_graph_version
  source_config_revision
  source_activation_revision
  draft_revision
  plan_id optional
  plan_hash optional
  active_job_id optional
  created_at
  updated_at
```

Rules:

1. TUI renders wizard state returned by the service; it does not own wizard truth.
2. CLI can create, inspect, update, plan, apply, abandon, and resume wizard sessions through the same API.
3. A wizard session is scoped to service/session/project identity and must not be daemon-global.
4. A new source config/activation revision invalidates or revalidates stale draft state before apply.

### Finding 86: Option Graph Must Be Typed And Explainable

Problem:

- VIDA options include single-choice systems, multi-select roles/skills/flows, numeric budgets, booleans, free text paths, model profiles, provider settings, platform-specific service install modes, and generated materialization policies.
- Hardcoded wizard screens would drift from `vida.config.yaml`, registries, and platform capability checks.

Recommendation:

- Generate a typed `WizardOptionGraph`:

```text
option_graph
  graph_id
  graph_version
  wizard_kind
  nodes[]
  edges[]
  validation_rules[]
  source_refs[]

option_node
  option_id
  step_id
  label
  description
  value_type               bool | string | path | integer | decimal | enum | multi_enum | ordered_list | object | secret_ref
  current_value
  default_value
  allowed_values[]
  required
  visible
  enabled
  locked
  reason
  source_ref
```

Dependency edge kinds:

```text
requires
excludes
enables
disables
narrows_allowed_values
sets_default
requires_readiness
requires_capability
requires_platform
requires_service_health
requires_project_identity
```

Rules:

1. Every disabled/hidden/locked option must carry a reason code visible to TUI/CLI.
2. Option values are typed; no ad hoc string parsing for model profiles, roles, or materialization mode.
3. Platform/service capability output participates in option gating.
4. Registry/config schema output participates in allowed values and defaults.
5. TUI controls are generated from value type and option metadata.

### Finding 87: Wizard Draft Must Separate Source, Draft, Plan, And Projection

Problem:

- The wizard will read existing config/DB state, modify draft values, preview diffs, and eventually write DB truth plus materialized files.
- Mixing these states causes accidental overwrites and makes update/reconfigure unsafe.

Recommendation:

```text
source_state
  project DB activation state
  service install state
  imported root vida.config.yaml
  registries/projections

wizard_draft
  typed option values
  user selections
  pending generated defaults
  unresolved blockers

activation_plan
  DB mutations
  service lifecycle operations
  materialization operations
  registry operations
  receipts required

projections
  vida.config.yaml bridge/export
  .vida/project agent-extension exports
  .codex/.pi/.opencode materialized host files
  docs/bootstrap carriers
```

Rules:

1. Draft changes are not runtime truth.
2. Plan generation is deterministic from source state plus draft.
3. Apply writes project DB/service state first, then materializes projections.
4. Projection files carry materialization manifests and drift policy.
5. Import/reconcile is explicit when root config or projections changed outside the wizard.

### Finding 88: Wizard Stages Should Match VIDA Activation Domains

Recommended stage map:

```text
0. Project Identity
   project_id
   project_instance_id
   display_name
   root/state/db paths
   language policy

1. Service Runtime
   attach/install/foreground/skip/repair/reconfigure
   service_home
   endpoint/auth
   update policy

2. Development Environment
   host systems: codex, hermes, opencode, pi, future systems
   execution class: internal | external | hybrid
   materialization mode
   readiness checks

3. Agent Topology
   internal/external/hybrid
   roles/skills/profiles/flows/teams
   carrier admission
   write-scope policy

4. Models And Cost
   model profiles
   reasoning effort floors
   quality floors
   normalized cost units
   pricing freshness policy
   budget policy

5. Runtime Policy
   max parallel agents
   scoring/promote/demote thresholds
   session policy
   claim/conflict posture

6. Materialization
   AGENTS.md/sidecar/docs
   .vida/project registries
   host templates
   CLI templates
   manifests and drift policy

7. Review Diff
   DB diff
   service diff
   file diff
   readiness blockers
   receipts to be emitted

8. Apply Progress
   job/events/receipts
   reconnect/resume
   rollback/repair if needed
```

Rules:

1. Later stages consume validated outputs from earlier stages.
2. Service Runtime stage can be skipped, but final activation status must record service posture.
3. Development Environment stage controls available Agent Topology and Models options.
4. Models are available for all carrier types, but allowed profiles are narrowed by provider/system readiness and role/task compatibility.
5. Materialization stage is a projection plan, not the source of authority.

### Finding 89: Update Wizard Must Handle New Options And Version Drift

Problem:

- VIDA version updates can introduce new options, new template versions, changed defaults, deprecated options, and new required manifests.
- Users may also edit exported config/materialized files manually.

Recommendation:

- Add version-aware update modes:

```text
update_mode
  inspect_only
  update_generated_only
  guided_merge
  clean_update_all_generated
  reconfigure_from_current
  repair_drift
```

Diff categories:

```text
new_option
changed_default
deprecated_option
removed_option
template_version_changed
generated_file_unchanged_update_available
generated_file_user_modified
missing_generated_file
unmanaged_existing_file
db_schema_migration_required
service_generation_update_required
```

Rules:

1. New required options block apply until selected or defaulted by explicit rule.
2. Deprecated options are preserved for import/report but removed only through a planned migration.
3. `clean_update_all_generated` may update only generated/unmodified artifacts automatically.
4. User-modified artifacts require guided merge, skip, preserve, or replace choice.
5. Update apply emits a report of added, updated, skipped, preserved, conflicted, deprecated, and migrated items.

### Finding 90: Validation Must Run At Three Levels

Recommendation:

```text
validation_levels
  option_validation:
    type correctness
    required values
    dependency graph consistency

  readiness_validation:
    service capability
    project identity
    host CLI availability
    auth/model/provider readiness
    DB/schema readiness

  activation_validation:
    framework/project law
    role/profile/flow compatibility
    model/task-class compatibility
    materialization conflict checks
    receipt/proof requirements
```

Rules:

1. TUI shows validation blockers inline at option level and in the sidecar summary.
2. Plan generation is blocked on option validation failures.
3. Apply is blocked on readiness or activation validation failures unless the operation is a repair/bootstrap operation that explicitly resolves the blocker.
4. Validation output uses `VidaProblem` codes, not free-form text only.

### Finding 91: Wizard APIs Should Be Contract-First

Recommended service operations:

```text
vida.wizard.start
vida.wizard.get
vida.wizard.set_option
vida.wizard.validate
vida.wizard.plan
vida.wizard.diff
vida.wizard.apply
vida.wizard.resume
vida.wizard.abandon
vida.wizard.available_options
vida.wizard.import_current
vida.wizard.reconfigure_from_current
vida.wizard.update_generated
```

Rules:

1. All operations are carried by `VidaCommandEnvelope`.
2. `wizard.apply` returns or resumes a `VidaJob`.
3. `wizard.diff` is pure/read-only and safe for TUI repeated calls.
4. `wizard.set_option` writes draft state only and emits draft events, not activation receipts.
5. `wizard.apply` emits receipts only for actual state transitions and materialization operations.

### Approved Clarifications, Set 17

Approved by operator on 2026-05-21:

1. Wizard truth lives in service/project `WizardSessionState`; TUI is a renderer/client, not wizard authority.
2. Wizard state is resumable by CLI/TUI/dashboard and scoped by session, project, service instance, and source revisions.
3. Build a typed, explainable `WizardOptionGraph` with dependency edges, typed values, reasons for disabled/hidden/locked options, and platform/service/readiness gating.
4. Separate source state, wizard draft, activation plan, and materialized projections.
5. Apply writes DB/service truth first and materializes files second with manifests and drift policy.
6. Wizard stages are Project Identity, Service Runtime, Development Environment, Agent Topology, Models/Cost, Runtime Policy, Materialization, Review Diff, Apply Progress.
7. Update/reconfigure wizard supports new options, changed defaults, deprecated options, template changes, user-modified files, DB migrations, and service generation updates.
8. Validation runs at option, readiness, and activation-law levels and returns structured `VidaProblem` blockers.
9. Wizard API is contract-first over `VidaCommandEnvelope`; apply returns/resumes `VidaJob`, diff is read-only, set-option mutates only draft state.
10. `vida.config.yaml` and host templates remain bridge/projection outputs until imported/applied into DB-first activation truth.

## Ratatui Operator Console Architecture Research Pass

Date: 2026-05-21.

Purpose:

- Define the concrete Ratatui operator console architecture for `vida tui`, including app shell, component boundaries, async client integration, generated wizard controls, reconnect behavior, screen map, accessibility, and proof strategy.

Sources:

- `https://ratatui.rs/concepts/application-patterns/component-architecture/`
- `https://ratatui.rs/concepts/backends/comparison/`
- `https://ratatui.rs/recipes/testing/snapshots/`
- `https://ratatui.rs/tutorials/counter-async-app/`
- `https://ratatui.rs/faq/`
- Earlier approved sets in this document: Set 9, Set 10, Set 16, Set 17.

External evidence:

1. Ratatui's component architecture pattern organizes TUI code around components with initialization, event handling, state update, and render responsibilities.
2. Ratatui backend guidance recommends Crossterm for most tasks and especially when Windows compatibility matters.
3. Ratatui snapshot guidance uses `TestBackend` plus `insta` to capture deterministic terminal render output.
4. Ratatui itself is not inherently async; async is useful for application IO and key event streaming through Crossterm event-stream.

### Finding 92: TUI Must Be A Client-Side Projection Over Service State

Problem:

- The TUI needs rich local interactions, but VIDA state authority belongs to service/project DB, jobs, events, receipts, and wizard sessions.
- If components call filesystem or DB helpers directly, the TUI becomes a second runtime owner.

Recommendation:

```text
vida tui
  AppShell
  TuiStore
  TuiRouter
  TuiEventLoop
  VidaClient adapter
  FixtureVidaClient tests
```

Rules:

1. TUI components render view models derived from `TuiStore`.
2. Only the client/effect layer calls `VidaClient`.
3. TUI local state is limited to focus, navigation, input buffers, local filters, scroll offsets, and last-seen cursors.
4. Mutations always dispatch command intents to `VidaClient`; they never write project files, service state, or DB directly.
5. Every screen can be rendered from fixture data without a live daemon.

### Finding 93: Event Loop Should Separate Terminal Input, Client Effects, And Rendering

Problem:

- Service calls, event polling, reconnect, and job progress must not block terminal rendering.
- Ratatui rendering can remain synchronous while service IO runs through async tasks.

Recommendation:

```text
TuiEvent
  TerminalKey
  TerminalMouse
  Resize
  Tick
  ClientResponse
  ClientEventBatch
  ReconnectTimer
  Shutdown

TuiAction
  Navigate
  SetFocus
  StartClientRequest
  ApplyClientResponse
  PollEvents
  StartReconnect
  ResumeWizard
  ResumeJob
  Render
  Quit
```

Rules:

1. Use Crossterm event-stream for async terminal events.
2. Client requests run outside render and return `ClientResponse` actions.
3. Rendering reads a consistent `TuiStore` snapshot.
4. Slow service calls show pending state and never freeze key handling.
5. Event polling uses job/project/service cursors, not raw log tailing.

### Finding 94: Component Boundaries Need Screen-Level Ownership And Shared Shell Chrome

Recommendation:

```text
AppShell
  HeaderBar
  NavRail
  MainPane(Screen)
  ContextSidecar
  FooterCommandBar
  ModalLayer

Screen
  init(view_context)
  handle_input(input, focus)
  reduce(action, store_view)
  render(frame, area, store_view)
```

Screen classes:

```text
read_only_dashboard
  ServiceOverview
  Projects
  ProjectDetail
  AgentInventory
  Jobs
  Receipts
  Logs

workflow
  ActivationWizard
  ReconfigureWizard
  ServiceRuntimeWizard
  MaterializationUpdate
  RepairFlow

report
  DiffReview
  ApplyProgress
  ReceiptDetail
  ProblemDetail
```

Rules:

1. Header shows service endpoint, selected project, session id short, protocol compatibility, and connection status.
2. NavRail is stable across screens and includes service, projects, wizard, config, agents, jobs, receipts, logs.
3. Sidecar is contextual and may show blockers, validation findings, events, receipt preview, or help for focused control.
4. FooterCommandBar shows actual available actions for current focus and capability posture.
5. ModalLayer is only for confirmations, command palette, conflict choices, and non-secret text input.

### Finding 95: Navigation Must Be Project-Scoped And Session-Scoped

Problem:

- One daemon manages many projects and many sessions. A global selected project would recreate the session defect in UI form.

Recommendation:

```text
TuiSessionView
  session_id
  selected_project_instance_id optional
  selected_screen
  selected_wizard_session_id optional
  pinned_job_id optional
  event_cursors
  local_layout_preferences
```

Rules:

1. Project selection updates the TUI session view and optionally service session state, not daemon-global state.
2. CLI and TUI can share the same project actor because requests carry explicit `ProjectRef`.
3. Navigation to wizard requires an explicit selected project or new/discovered project flow.
4. Cross-project screens never allow a mutation until a specific project is selected and resolved.
5. Stale selected project state triggers re-resolution and visible blockers before apply.

### Finding 96: Generated Wizard Controls Need A Focus And Validation Model

Problem:

- Option graph controls can be nested, disabled, validated, and dependent on readiness.
- Operators need keyboard-first navigation and explanations without reading hidden config.

Recommendation:

```text
WizardControlView
  option_id
  value_type
  value
  effective_value
  dirty
  focused
  visible
  enabled
  locked
  validation_status
  blocked_reason optional
  help_ref optional
```

Control mapping:

```text
bool              checkbox/toggle row
enum              select/list/radio
multi_enum        checklist/table
string/slug       validated input
path              input + project picker
integer/decimal   bounded stepper/input
duration          numeric + unit select
model_profile     searchable table
object/matrix      validated table editor
secret_ref         masked status/ref control
derived           read-only row
diff              scrollable diff pane
```

Rules:

1. All controls are keyboard-operable; mouse is optional.
2. Focused validation findings are mirrored into sidecar.
3. Disabled controls remain inspectable when useful, with reason and dependency chain.
4. Dirty controls show source, draft, and effective values.
5. Model-profile tables show provider, model ref, reasoning, cost units, quality/speed, write scope, runtime roles, task classes, readiness.

### Finding 97: Reconnect And Resume UX Must Be First-Class

Recommendation:

```text
connection_state
  disconnected
  discovering_endpoint
  hello_pending
  connected
  degraded_read_only
  reconnecting
  blocked

resume_targets
  selected_project
  wizard_session
  active_job
  event_cursor
  last_receipt
```

Rules:

1. Startup calls endpoint discovery, service hello, capability negotiation, project registry list, then session restore.
2. Reconnect first reloads service hello and capability posture, then selected project, then active wizard/job.
3. If apply was running, TUI resumes from `jobs.get(job_id)` and `events_since(cursor)`.
4. TUI never assumes disconnect means failure or cancellation.
5. Degraded read-only mode is allowed only when service hello says the requested reads are compatible.

### Finding 98: Terminal Safety And Accessibility Need Explicit Constraints

Problem:

- Operator console failure can leave the terminal in raw/alternate-screen mode.
- Dense config screens can become unusable on small terminals.

Rules:

1. Terminal setup/teardown must be guard-backed so panic/error paths restore terminal mode.
2. Minimum supported viewport should be explicit; smaller terminals show a compact blocker screen.
3. Keyboard shortcuts must have command-bar visibility and avoid requiring mouse.
4. Color cannot be the only signal for status; include symbols/text labels.
5. Secret values are never displayed; only secret refs/readiness status are shown.
6. Long text must wrap or scroll within stable layout bounds.

### Finding 99: Snapshot And Fixture Testing Should Gate TUI Work

Recommendation:

```text
TUI proof gates
  reducer tests for TuiStore/TuiAction
  fixture client contract tests
  screen view-model tests
  Ratatui TestBackend snapshots
  viewport matrix snapshots
  reconnect/resume simulation
  degraded read-only simulation
  live daemon smoke after tarpc proof
```

Snapshot matrix:

```text
120x36 desktop terminal
100x30 standard terminal
80x24 minimum normal terminal
60x20 compact blocker/degraded view
```

Rules:

1. Snapshot tests use `FixtureVidaClient` and deterministic fixture data.
2. Snapshot tests cover AppShell, Projects, Service Overview, Wizard, Diff Review, Apply Progress, Jobs, Receipts, Logs.
3. Reducer tests prove that service responses cannot bypass plan/apply state transitions.
4. Live terminal tests are smoke-only after fixture snapshots pass.
5. TUI implementation is not considered ready until reconnect/resume fixture tests pass.

### Finding 100: TUI MVP Should Be Operational, Not Decorative

Recommended MVP screen order:

```text
1. Service Overview
2. Projects List + Project Detail
3. Wizard Shell using fixture option graph
4. Diff Review
5. Apply Progress from job/events fixture
6. Jobs
7. Receipts
8. Logs diagnostic view
9. Agent/Config inventory read-only
```

Rules:

1. The first TUI slice may be fixture-backed but must use the real `VidaClient` trait.
2. No screen may own a direct mutation path.
3. Wizard shell must render from `WizardOptionGraph` before any live service apply.
4. Jobs/events/receipts screens must exist before mutating wizard apply is exposed.
5. Agent/config inventory can be read-only until reconfigure apply is proven.

### Approved Clarifications, Set 18

Approved by operator on 2026-05-21:

1. `vida tui` is a Ratatui client-side projection over `VidaClient`, service/project state, jobs, events, receipts, and wizard sessions.
2. TUI components never call filesystem/DB mutation helpers directly; only the client/effect layer calls `VidaClient`.
3. Use Crossterm as the primary backend for Windows/Linux/macOS and async event-stream for terminal input.
4. Use an event/action/store architecture: terminal input, service responses, event batches, reconnect timers, reducers, and render snapshots are separated.
5. AppShell has HeaderBar, NavRail, MainPane, ContextSidecar, FooterCommandBar, and ModalLayer.
6. Screens are grouped as read-only dashboards, workflow screens, and report screens.
7. Navigation and selected project are session-scoped; no daemon-global selected project exists.
8. Wizard controls are generated from `WizardOptionGraph` with typed controls, focus state, dirty/effective values, validation markers, and blocked reasons.
9. Reconnect/resume is first-class: service hello, capability posture, selected project, wizard session, active job, and event cursors are restored in order.
10. TUI proof gates require reducer tests, fixture client tests, TestBackend/insta snapshots across viewport sizes, reconnect simulation, and live daemon smoke only after tarpc proof.

## Service API Operation Catalog Research Pass

Date: 2026-05-21.

Purpose:

- Define the canonical service API operation catalog carried by `VidaCommandEnvelope`, so CLI, TUI, daemon, future dashboard, in-process client, tarpc transport, and later jsonrpsee transport share one semantic contract.

Sources:

- `docs/product/spec/project-activation-and-configurator-model.md`
- `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
- `docs/process/project-orchestrator-operating-protocol.md`
- `docs/process/team-development-and-orchestration-protocol.md`
- `crates/vida/src/cli.rs`
- Earlier approved sets in this document: Set 3, Set 10, Set 11, Set 13, Set 16, Set 17, Set 18.

Local evidence:

1. The current CLI already has many direct command families: `init`, `boot`, `orchestrator-init`, `agent-init`, `agent`, `protocol`, `project-activator`, `task`, `status`, `doctor`, `diagnostics`, `docs`, `orchestrator-session`, `consume`, `lane`, `approval`, `recovery`, `release`, `taskflow`, and `docflow`.
2. Earlier approved decisions require new service-backed `project`, `wizard`, `config`, `materialization`, `jobs`, `events`, and `receipts` families without exposing tarpc/jsonrpsee details.
3. Existing runtime receipt/projection surfaces are domain-specific; the service API must wrap/reference them without replacing their owner semantics.

### Finding 101: Operation Ids Need A Registry, Not Scattered Strings

Problem:

- CLI, TUI, service, tests, tarpc, and later jsonrpsee will all refer to operation ids.
- If operation strings are scattered, compatibility, capability checks, docs, and tests will drift.

Recommendation:

- Add a versioned operation registry to `vida-contracts`:

```text
VidaOperationSpec
  operation_id
  operation_version
  domain
  resource
  verb
  scope_kind                 service | project | session | job | receipt | event | global
  mutability                 read | plan | apply | admin | diagnostic
  async_kind                 immediate | job_returning | long_poll | stream_later
  required_capabilities[]
  required_context_fields[]
  idempotency_required
  project_ref_required
  session_required
  produces_job
  produces_receipt
  emits_events
  request_payload_schema_ref
  response_payload_schema_ref
  deprecation optional
```

Rules:

1. Operation ids are stable string constants and must be valid in JSON output.
2. Operation ids use dot namespaces: `vida.<domain>.<resource>.<verb>`.
3. Operation metadata is used by CLI help, TUI action availability, capability checks, and conformance tests.
4. New operations are additive unless explicitly versioned or deprecated.
5. Deprecated operations require replacement operation id, compatibility window, and diagnostic warning.

### Finding 102: Envelope Fields Must Be Semantically Mandatory By Operation Class

Recommendation:

```text
VidaCommandEnvelope
  protocol_version
  operation_id
  operation_version optional
  request_id
  session_id
  client_kind
  client_version
  project_ref optional
  service_context optional
  idempotency_key optional
  apply_token optional
  capability_claims[]
  event_cursor optional
  wait_policy optional
  payload
```

Rules:

1. Every request has `request_id`, `session_id`, `client_kind`, `client_version`, and `operation_id`.
2. Every project-scoped operation has `project_ref`, except project discovery/register bootstrap operations that explicitly allow discovery.
3. Every mutating apply operation has `idempotency_key`; every plan-derived apply has `plan_id`, `plan_hash`, and `apply_token`.
4. Every request is correlated to events, jobs, and receipts through `request_id` and `operation_id`.
5. Missing mandatory context returns `VidaProblem` before domain logic starts.

### Finding 103: Response Shape Must Support Immediate Results, Jobs, Problems, And Receipts

Recommendation:

```text
VidaCommandResponse
  protocol_version
  request_id
  operation_id
  status                    ok | accepted | blocked | failed | unsupported | unauthorized | stale_context
  service_instance_id
  session_resolution
  project_resolution optional
  capability_resolution
  result optional
  job_ref optional
  receipt_refs[]
  event_cursor optional
  warnings[]
  blockers[]
  problem optional
```

Rules:

1. `ok` means immediate operation completed without creating a running job.
2. `accepted` means a job exists and must be followed through `jobs.get`/`events_since`.
3. `blocked` means the request is valid but cannot proceed until listed blockers are resolved.
4. `failed` means execution failed after admission and should include `VidaProblem`.
5. `unsupported`, `unauthorized`, and `stale_context` are pre-admission failures.
6. Mutating successful apply responses must include a success receipt or a job that will produce one.

### Finding 104: MVP Operation Families Should Be Explicitly Bounded

Recommended MVP operation catalog:

```text
service
  vida.service.hello
  vida.service.status
  vida.service.capabilities
  vida.service.doctor
  vida.service.endpoint.status
  vida.service.install.inspect
  vida.service.install.plan
  vida.service.install.apply
  vida.service.start
  vida.service.stop
  vida.service.restart

session
  vida.session.open
  vida.session.heartbeat
  vida.session.get
  vida.session.set_active_project
  vida.session.close

project_registry
  vida.project.registry.list
  vida.project.registry.get
  vida.project.discover
  vida.project.register.plan
  vida.project.register.apply
  vida.project.attach
  vida.project.detach.plan
  vida.project.detach.apply
  vida.project.archive.apply
  vida.project.restore.apply
  vida.project.forget.plan
  vida.project.forget.apply
  vida.project.reconcile

project_activation
  vida.project.activation.inspect
  vida.project.activation.plan
  vida.project.activation.diff
  vida.project.activation.validate
  vida.project.activation.apply

wizard
  vida.wizard.start
  vida.wizard.get
  vida.wizard.set_option
  vida.wizard.validate
  vida.wizard.plan
  vida.wizard.diff
  vida.wizard.apply
  vida.wizard.resume
  vida.wizard.abandon
  vida.wizard.available_options

config_materialization
  vida.config.inspect
  vida.config.option_graph
  vida.config.validate
  vida.config.diff
  vida.config.update.plan
  vida.config.update.apply
  vida.materialization.inspect
  vida.materialization.diff
  vida.materialization.update.plan
  vida.materialization.update.apply
  vida.materialization.report

operations
  vida.jobs.list
  vida.jobs.get
  vida.jobs.watch
  vida.jobs.cancel
  vida.events.since
  vida.receipts.list
  vida.receipts.get
  vida.receipts.export.plan
  vida.receipts.export.apply
  vida.logs.tail
```

Rules:

1. TaskFlow/DocFlow runtime families are not moved into this MVP catalog yet.
2. `vida.logs.tail` is diagnostic read-only and never drives TUI state.
3. `watch` is a CLI convenience over `jobs.get` plus `events_since`, not a distinct authority path.
4. Future dashboard/jsonrpsee can expose the same operation ids over JSON-RPC method names or a generic call method.

### Finding 105: Capability And Scope Resolution Must Precede Domain Execution

Admission order:

```text
1. decode envelope
2. validate protocol/operation version
3. service hello/capability compatibility
4. session resolution
5. endpoint/auth posture
6. project resolution when required
7. operation capability check
8. idempotency/apply-token check
9. claim/admission check
10. enqueue job or execute immediate operation
```

Rules:

1. Domain handlers receive only admitted requests.
2. Capability denial returns required scope and current auth posture.
3. Project ambiguity returns `project_resolution.status=ambiguous` and does not enter domain logic.
4. Mutating operations fail closed on stale endpoint, stale source revision, stale wizard draft, or plan hash mismatch.
5. Read-only operations may degrade only when the operation spec allows degraded reads.

### Finding 106: Transport Adapters Must Prove The Same Operation Semantics

Recommendation:

```text
VidaClient
  call(envelope) -> VidaCommandResponse

EventClient
  events_since(cursor, filter) -> EventBatch

Transport adapters
  FixtureVidaClient
  InProcessVidaClient
  TarpcVidaClient
  JsonRpseeVidaClient later
```

Rules:

1. Tarpc exposes a narrow generic call that carries `VidaCommandEnvelope`, not product-specific RPC methods.
2. Jsonrpsee later may expose either a generic `vida.call` or operation-specific JSON-RPC method aliases, but aliases must map to the operation registry.
3. In-process, tarpc, and jsonrpsee responses must have equivalent status/problem/job/receipt/event semantics.
4. Transport-specific errors are normalized into `VidaProblem` codes at the client boundary.
5. Transport adapters do not perform project DB mutation; they only carry admitted commands to the service/core.

### Finding 107: Conformance Fixtures Are The First Proof Target

Recommendation:

- Add a conformance matrix before live daemon/TUI mutation work:

```text
contract_fixtures
  service.hello ok/protocol_mismatch
  service.status ok/stale_endpoint
  project.registry.list ok
  project.resolve ambiguous/missing/resolved
  wizard.start ok
  wizard.set_option disabled_option
  wizard.validate blocker
  wizard.plan ok
  wizard.apply accepted_job
  jobs.get running/succeeded/failed
  events.since replay/empty
  receipts.get found/missing
  config.update.plan new_option/user_modified_file
```

Adapter conformance:

```text
FixtureVidaClient
InProcessVidaClient
TarpcVidaClient
JsonRpseeVidaClient later
```

Rules:

1. Golden JSON fixtures live with `vida-contracts`.
2. Fixture client and in-process client must pass the same operation conformance tests before TUI implementation.
3. Tarpc adapter must pass the same conformance tests before live TUI smoke.
4. Jsonrpsee adapter must pass the same conformance tests before dashboard/API exposure.
5. Conformance includes negative cases: missing session id, ambiguous project, unauthorized capability, stale draft revision, idempotency mismatch, unsupported protocol.

### Finding 108: CLI/TUI/Dashboard Must Be Generated From Operation Metadata Where Practical

Problem:

- Manual command help, TUI action availability, and dashboard route docs can drift from service operation truth.

Recommendation:

1. CLI command posture reads operation metadata for service-first/help output where possible.
2. TUI footer actions and disabled-state reasons are derived from operation metadata plus current capability resolution.
3. Dashboard/API docs later use operation registry metadata and JSON fixtures.
4. Operation metadata includes whether a command is safe to repeat, job-returning, apply-token-gated, project-scoped, or diagnostic-only.

### Approved Clarifications, Set 19

Approved by operator on 2026-05-21:

1. Add a versioned `VidaOperationSpec` registry in `vida-contracts`; operation ids are stable dot-namespaced strings.
2. Operation metadata drives CLI help posture, TUI action availability, capability checks, and conformance tests.
3. `VidaCommandEnvelope` has mandatory request/session/client/operation identity for every request, with project/idempotency/apply-token fields mandatory by operation class.
4. `VidaCommandResponse` supports immediate results, accepted jobs, blockers, structured `VidaProblem`, receipt refs, event cursors, and capability/project/session resolution.
5. MVP operation families are service, session, project registry, project activation, wizard, config/materialization, jobs/events/receipts/logs.
6. TaskFlow/DocFlow service migration is explicitly outside the MVP service operation catalog until their semantics are preserved.
7. Admission order is decode -> protocol/op version -> hello/capability -> session -> auth -> project -> capability -> idempotency/apply token -> claim -> execute/enqueue.
8. Tarpc carries generic `VidaCommandEnvelope`; jsonrpsee later maps aliases or generic `vida.call` back to the same operation registry.
9. Fixture, in-process, tarpc, and later jsonrpsee clients must pass the same golden JSON conformance matrix.
10. CLI/TUI/dashboard should derive action/help/API metadata from the operation registry where practical.

## Implementation Roadmap And Bounded Slice Plan Research Pass

Scope:

- Translate the approved architecture into implementation slices that can be delegated, reviewed, tested, and stopped independently.
- Keep the current session-identity/multisession defect as an explicit precondition before write-capable service or TUI mutation paths.
- Preserve the research protocol as the authority for planning; do not start service/TUI code implementation from UI screens or daemon bootstrap alone.

Reference surfaces:

- `docs/product/spec/project-activation-and-configurator-model.md`
- `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
- `docs/process/project-orchestrator-operating-protocol.md`
- `docs/process/team-development-and-orchestration-protocol.md`
- `docs/product/research/vida-service-tui-wizard-architecture-research.md`
- `crates/vida/src/cli.rs`

### Finding 109: Session Identity Is Gate 0

Problem:

- Service/TUI architecture assumes every request, operation, job, event, receipt, claim, and apply action is scoped by the current session identity.
- The known defect makes worktree/root identity too strong as session identity and lets global latest continuation evidence block unrelated current-session work.
- Building a daemon, project registry, wizard apply, or TUI action system before fixing this would encode the wrong ownership model into the new architecture.

Recommendation:

1. Treat `fix-session-identity-scoped-continuation-before-tui-service` as the first implementation gate.
2. Do not implement write-capable service/TUI mutation paths until current-session scoped continuation and claim-scoped blockers are proven.
3. Permit read-only documentation, contract drafting, and fixture design before that fix, because those artifacts define the target without mutating runtime state.
4. Require the later service protocol to reject or block requests without canonical `VIDA_SESSION_ID`/fallback session identity.

### Finding 110: Implementation Should Be Sliced By Proof Gates, Not UI Screens

Recommended slice order:

1. Slice 0: fix session identity and multisession continuation behavior.
2. Slice 1: create `vida-contracts` with pure Serde types for ids, envelope, response, problem, event, job, receipt, project, operation, and wizard option contracts.
3. Slice 2: add golden JSON conformance fixtures for positive and negative request/response/event cases.
4. Slice 3: add `VidaClient` trait plus fixture and in-process clients that pass the same conformance matrix.
5. Slice 4: extract config graph/materialization inspect and diff as read-only core services.
6. Slice 5: add wizard core session state, validation, plan, and diff without apply.
7. Slice 6: add service state store skeleton for registry/jobs/events/receipts/idempotency/sessions using append-only JSONL or an equivalent auditable local format.
8. Slice 7: add service shell with `vida service run` plus hello/status/capabilities/endpoints; keep it read-only.
9. Slice 8: add project registry operations and per-project actor skeleton.
10. Slice 9: add tarpc over `interprocess` proof carrying `VidaCommandEnvelope`.
11. Slice 10: add service lifecycle plan/install/status operations with platform adapters and no destructive install by default.
12. Slice 11: enable wizard apply through jobs/events/receipts once registry and service state are stable.
13. Slice 12: build first Ratatui screens against fixture client and snapshot-test them.
14. Slice 13: attach Ratatui to live tarpc service and run smoke tests for status, wizard diff, logs, and apply progress.
15. Slice 14: migrate selected CLI commands to the service client path while preserving direct paths for TaskFlow/DocFlow until semantics are proven.
16. Slice 15: add jsonrpsee/dashboard transport later through the same operation registry and conformance suite.

### Finding 111: First Implementation Tickets Should Be Small And Contract-Heavy

Recommended first tickets after research approval:

1. Ticket A: repair session identity scoped continuation and claim blocker behavior under the existing multisession epic.
2. Ticket B: add `vida-contracts` with pure Serde contract types and no daemon/TUI dependency.
3. Ticket C: add `VidaOperationSpec` registry plus golden JSON conformance fixtures.
4. Ticket D: add `VidaClient` trait plus fixture and in-process clients.
5. Ticket E: add read-only service hello/status/capabilities/endpoints and service-home metadata.

Why:

- These tickets create stable protocol truth before UI, transport, project mutation, or platform service installation.
- They make it possible to test CLI, TUI, dashboard, tarpc, and jsonrpsee against the same contract.
- They keep the blast radius small while the runtime still has known session-scoping debt.

### Finding 112: Proof Policy Must Stay Green At Every Slice

Minimum proof gates:

1. Contract slices: `cargo test -p vida-contracts` plus golden JSON fixture round-trips.
2. Client slices: fixture and in-process clients pass the same conformance suite.
3. Config/wizard core slices: inspect/plan/diff tests compare structured plans, not rendered text.
4. Service state slices: idempotency, append-only event/receipt persistence, restart/reload, and project-scoped routing tests.
5. Tarpc slice: local socket smoke test plus conformance matrix through tarpc client.
6. Ratatui slice: fixture-backed snapshot tests before live service attach.
7. CLI migration slice: selected command proof through service client plus direct fallback behavior where still allowed.
8. Doc slices: `vida docflow check-file` remains required after research/spec mutations.

### Finding 113: Dependency And Risk Matrix

Main risks:

1. Scope explosion: daemon, TUI, wizard, dashboard, update engine, and service install can become one unreviewable change.
2. TUI direct mutation: Ratatui screens could bypass the service state machine if built first.
3. Windows service semantics: native service installation has privileges and session-bound UI constraints.
4. Update self-handoff: service updating its own binary needs staged handoff and receipts.
5. Filesystem watcher unreliability: project sync cannot rely only on watcher events.
6. Token leakage: local TCP fallback auth tokens must not be printed into logs or receipts.
7. Multi-project ambiguity: commands from different working directories must resolve to the intended project authority.

Mitigations:

1. Contracts-first implementation with a single operation registry.
2. Plan/diff/apply-token split for all mutating activation, config, and materialization work.
3. Fixture client before live service to prove TUI behavior without daemon timing.
4. Service as the single writer for service state; per-project DB authority remains project-local.
5. Capability scopes and session identity in every envelope.
6. Versioned manifests for generated docs, agents, sidecar, and config.
7. Explicit fallback modes for service unavailable, service outdated, and ambiguous project context.

### Finding 114: First Wave Exclusions

Exclude from the first implementation wave:

1. Native Windows service apply/install without a dry-run or plan-only gate.
2. Jsonrpsee dashboard transport and browser dashboard UI.
3. TaskFlow/DocFlow service migration.
4. Destructive project delete or purge operations.
5. Remote service authentication or multi-user network access.
6. Full external schema publishing beyond local golden fixtures.
7. Web dashboard operation docs generation.

Rationale:

- These features depend on stable contracts, session identity, project registry, job/event/receipt semantics, and transport conformance.
- They are useful follow-up work, but they would weaken the first proof loop if included immediately.

### Finding 115: Development Branch And Task Management Should Mirror The Slice Plan

Recommendation:

1. Keep the active multisession/session identity defect as a separate TaskFlow unit and proof it before service/TUI code mutation.
2. Open a new service/TUI architecture implementation epic only after Set 20 is approved or after the current research protocol is accepted as the implementation basis.
3. Create child tasks matching the first contract-heavy tickets rather than one broad "build TUI/service" task.
4. Each child task must declare owned files, proof commands, expected fixtures, and whether it is read-only, contract-only, or mutation-capable.
5. Root-session implementation remains forbidden unless runtime law permits it or delegated execution evidence/active exception takeover exists for the bounded packet.

### Finding 116: Architecture-To-Code Readiness Criteria

The architecture is ready to become implementation tickets when all of the following are true:

1. Set 20 is approved by the operator.
2. Session identity/multisession defect task is either fixed or explicitly remains Gate 0 for any mutation-capable slice.
3. A `vida-contracts` task is opened with a bounded write scope and proof targets.
4. Golden JSON fixture families are listed before transport or TUI code starts.
5. The first service slice is read-only hello/status/capabilities/endpoints, not wizard apply.
6. Ratatui first slice is fixture-backed snapshot UI, not live daemon-only UI.
7. Exclusions from the first wave are recorded so dashboard/jsonrpsee/platform service apply do not silently enter the MVP.

### Approved Clarifications, Set 20

Approved by operator on 2026-05-21:

1. Gate 0 is the session identity/multisession fix; no service/TUI mutation paths should be implemented before it is proven.
2. First implementation starts with contracts and proofs, not Ratatui screens or daemon process management.
3. The first code slice after Gate 0 is `vida-contracts` with pure Serde ids/envelope/response/problem/operation registry/event/job/receipt/wizard types.
4. Golden JSON conformance fixtures are mandatory before transports, TUI, or dashboard.
5. Fixture and in-process `VidaClient` implementations precede tarpc and Ratatui.
6. The service starts as read-only hello/status/capabilities/endpoints before registry, wizard, or apply mutations.
7. Project registry, per-project actor skeleton, and service state store are separate slices before wizard apply.
8. Ratatui first slice is fixture-backed snapshot UI, then live attach after tarpc proof.
9. CLI migration is additive and service-first for new families; existing TaskFlow/DocFlow direct paths remain direct until their semantics are preserved.
10. Jsonrpsee/dashboard, native Windows service apply, TaskFlow/DocFlow service adapter, destructive project delete, and remote auth are excluded from the first wave.

## Service State And IPC Attach Protocol Research Pass

Scope:

- Define the service state model and IPC attach protocol before daemon/TUI implementation starts.
- Preserve the split between service-level coordination state and project-local authority.
- Make CLI, TUI, fixture client, tarpc, and future jsonrpsee clients share one semantic contract.

Reference surfaces:

- `docs/product/spec/multi-orchestrator-session-ownership-and-claims-design.md`
- `docs/product/spec/canonical-runtime-layer-matrix.md`
- `vida.config.yaml`
- `docs/product/research/vida-service-tui-wizard-architecture-research.md`

### Finding 117: Service Owns Coordination State, Not All Project Truth

Problem:

- A daemon can easily become a second project authority if it stores project config, project DB state, TaskFlow truth, generated files, and activation state as its own canonical data.
- VIDA already has project-local truth: project root, `.vida/**`, `vida.config.yaml`, docs, generated agents, and runtime state.

Recommendation:

1. The service owns local coordination state: service identity, sessions, claims, project registry, jobs, events, receipts, idempotency records, endpoint metadata, and install/update state.
2. The project owns project-local truth: config, project DB, docs, sidecar, generated agent files, TaskFlow/DocFlow state, and materialized artifacts.
3. Service project records may cache evidence and fingerprints, but project-local files remain authoritative unless an operation explicitly applies a plan to them.
4. Per-project actors bridge service coordination to project-local authority and prevent one global daemon loop from becoming a broad mutation owner.

### Finding 118: Service Home Needs A Small, Auditable Layout

Recommended service home layout:

```text
service-home/
  service.json
  endpoints.json
  sessions.jsonl
  claims.jsonl
  projects.jsonl
  jobs.jsonl
  events.jsonl
  receipts/
    <receipt-id>.json
  idempotency.jsonl
  manifests/
    <project-id>.materialization.json
  logs/
    service.log.jsonl
```

MVP storage posture:

1. Use append-only JSONL for coordination records to keep audit/replay simple.
2. Add periodic compacted snapshots only after append/replay semantics are tested.
3. Keep receipts as standalone immutable JSON files when they may become large or operator-facing.
4. Keep raw logs separate from events; events are contract-level operator stream, logs are diagnostic implementation detail.
5. Do not store secrets or fallback TCP tokens in event or receipt payloads.

### Finding 119: Project Registry Requires Two-Level Identity

Project registry records need both stable project identity and concrete environment identity.

Recommended record fields:

```text
project_id
display_name
root_path
worktree_environment_id
config_path
state_root_path
db_path
activation_status
config_revision
materialization_manifest_ref
aliases
last_seen_at
last_verified_at
status
```

Rules:

1. `project_id` identifies the logical VIDA project.
2. `worktree_environment_id` identifies a concrete checkout/environment where file mutations and proofs happen.
3. A service may know multiple worktrees for one logical project only when conflict-domain rules can distinguish them.
4. CWD-based project resolution is allowed only when it resolves to exactly one registered project/worktree.
5. Ambiguous project resolution returns a structured `VidaProblem`, not a guessed default.

### Finding 120: Session And Claim State Must Align With The Multisession Spec

Session record:

```text
session_id
client_id
client_kind
host_tool
host_thread_id
process_id
user_id
worktree_environment_id
created_at
last_heartbeat_at
status
```

Claim record:

```text
claim_id
session_id
project_id
worktree_environment_id
task_id
run_id
lane_id
claim_kind
conflict_domain
owned_paths
read_only_paths
lease_mode
lease_expires_at
resource_revision
status
```

Admission rule:

1. Foreign claims are visibility evidence by default.
2. They block only when task/run, owned path, exclusive conflict domain, or global state integrity class intersects.
3. Stale claims are not ignored; they require explicit reclaim/supersede transition with receipt.
4. Service/TUI requests without a resolved current session can inspect limited service metadata but cannot mutate project state.

### Finding 121: Endpoint Discovery Must Be Platform-Specific But Contract-Neutral

Endpoint discovery should be a small file plus platform-native socket/pipe names.

Recommended `endpoints.json`:

```text
protocol_version
service_instance_id
service_home
primary_transport
tarpc_endpoint
loopback_endpoint
token_ref
created_at
updated_at
service_binary_fingerprint
```

Platform posture:

1. Windows: named pipe first; loopback TCP only as fallback.
2. Linux/macOS: Unix domain socket first; loopback TCP only as fallback.
3. Endpoint metadata can be read by CLI/TUI, but auth tokens must be stored separately with restrictive permissions.
4. If endpoint metadata points to a dead service, clients should report stale endpoint and offer service start/status diagnostics.

### Finding 122: Attach Handshake Should Be A First-Class Operation

Attach flow:

1. Client reads endpoint metadata.
2. Client connects through tarpc/local transport.
3. Client sends `service.hello` with protocol, client, session, project hint, and capability declarations.
4. Service resolves or creates session.
5. Service resolves project or returns ambiguity.
6. Service returns service capability matrix, negotiated protocol, event cursor, and current scoped status.

`service.hello` request fields:

```text
protocol_version
client_id
client_kind
client_version
session_id
session_source
project_hint
supported_features
requested_capabilities
```

`service.hello` response fields:

```text
service_instance_id
service_version
protocol_version
session
project_resolution
capabilities
operation_registry_revision
event_cursor
status_summary
```

### Finding 123: Every Operation Needs The Same Envelope Semantics

Required envelope fields:

```text
request_id
client_id
session_id
operation_id
operation_version
protocol_version
project_ref
idempotency_key
apply_token
capability_scope
payload
```

Rules:

1. `request_id` is always required.
2. `session_id` is always required except the minimal pre-session `service.hello` path.
3. `project_ref` is required for project-scoped operations.
4. `idempotency_key` is required for mutating operations.
5. `apply_token` is required for operations that apply a prior diff/plan.
6. Operation registry metadata declares which fields are required by operation class.

### Finding 124: Jobs, Events, Receipts Need Separate Semantics

Definitions:

1. Job: active or completed long-running operation with progress and cancellation policy.
2. Event: append-only operator-visible status/change item.
3. Receipt: immutable completion proof for an operation/job/apply.
4. Log: diagnostic stream not guaranteed as stable contract.

Event fields:

```text
event_id
cursor
timestamp
session_id
project_id
job_id
operation_id
level
kind
message
structured_payload
receipt_ref
```

Job fields:

```text
job_id
operation_id
session_id
project_id
status
started_at
updated_at
progress
cancel_policy
result_receipt_ref
problem
```

Receipt fields:

```text
receipt_id
operation_id
request_id
session_id
project_id
job_id
started_at
completed_at
inputs_hash
outputs
changed_artifacts
problems
service_binary_fingerprint
config_revision_before
config_revision_after
```

### Finding 125: Event Streaming Should Start With Cursor Long-Poll

Options:

1. True streaming subscription from day one.
2. Cursor-based `events_since(cursor)` with optional wait timeout.

Recommendation:

1. Start with cursor long-poll because it works for TUI, CLI, tests, and future dashboard with less transport complexity.
2. Add true streaming later as a transport optimization.
3. Store event cursor in TUI state so reconnect can resume without losing progress.
4. Cap long-poll timeout to preserve responsive TUI shutdown and CLI cancellation.

### Finding 126: Idempotency And Apply Tokens Are Separate Controls

Problem:

- Idempotency prevents duplicate execution of the same request.
- Apply tokens prove the user approved a specific plan/diff.
- Treating them as one concept would make replay and approval semantics unclear.

Recommendation:

1. `idempotency_key` is per mutating request intent.
2. `apply_token` is minted by plan/diff validation and bound to project/config revision, draft revision, operation id, and selected changes.
3. Apply token expires when base config/materialization revision changes.
4. Reusing the same idempotency key with different payload returns an idempotency mismatch problem.
5. Reusing an apply token after successful apply returns the prior receipt or a token-consumed problem, depending on operation metadata.

### Finding 127: Local Security Should Be Minimal But Explicit

Security posture:

1. Service is local-only and per-user by default.
2. Socket/pipe permissions are the primary access control.
3. Loopback TCP fallback requires a local token.
4. Tokens are never included in normal events, receipts, or debug summaries.
5. Mutating operations require resolved session, project, capability, idempotency, and claim admission even for local clients.
6. Remote/multi-user auth is out of first wave.

### Finding 128: Service Unavailable And Outdated Cases Need Contracted Fallbacks

Client behavior matrix:

1. Service missing:
   - CLI may run direct only for explicitly allowed legacy/direct commands.
   - TUI shows service unavailable and can offer start/diagnostic actions.
2. Service endpoint stale:
   - client reports stale endpoint and does not silently create a second service unless operation metadata allows it.
3. Service outdated:
   - read-only status/diagnostics allowed;
   - mutations blocked until upgrade/compatible protocol.
4. Project unregistered:
   - wizard can start project registration/activation plan.
5. Project ambiguous:
   - require explicit selection.

### Finding 129: First Operation Catalog Should Be Small

First service operations:

```text
service.hello
service.status
service.capabilities
service.endpoints
service.events_since
session.status
project.list
project.resolve
project.status
wizard.inspect
wizard.draft.start
wizard.draft.update
wizard.validate
wizard.diff
job.status
receipt.get
```

Mutating apply operations should wait until after registry, state store, idempotency, claim admission, and apply-token proof are implemented:

```text
project.register.apply
wizard.apply
materialization.apply
service.install.apply
```

### Finding 130: Proof Fixtures Should Cover Attach Before UI

Required fixture groups:

1. `service.hello` with valid session and no project.
2. `service.hello` with project resolved from cwd.
3. `service.hello` with ambiguous project.
4. `service.status` with current session plus foreign nonblocking session.
5. `project.resolve` ambiguous path.
6. `events_since` empty, populated, and cursor replay.
7. missing session id rejected for project-scoped operation.
8. stale endpoint metadata reported.
9. idempotency key replay accepted for same payload.
10. idempotency key replay rejected for changed payload.

### Approved Clarifications, Set 21

Approved by operator on 2026-05-21:

1. Service home owns coordination state only; project config/DB/docs/generated artifacts remain project-local authority.
2. MVP service state uses append-only JSONL plus immutable receipt JSON files; compaction can follow after replay semantics are tested.
3. Project registry records both `project_id` and `worktree_environment_id` to support multiple checkouts/environments.
4. Project resolution must fail closed on ambiguity and return structured `VidaProblem` alternatives.
5. Session and claim records must reuse the multisession ownership model: foreign claims are visible but block only on task/run/path/conflict/global integrity intersections.
6. Endpoint discovery is platform-specific, but the attach contract is transport-neutral.
7. `service.hello` is the first-class attach operation and returns session, project resolution, capabilities, operation registry revision, event cursor, and status summary.
8. Every non-hello operation uses the same `VidaCommandEnvelope`; operation metadata declares required project/idempotency/apply-token fields.
9. Jobs, events, receipts, and logs are separate concepts; TUI consumes events/jobs/receipts, not raw logs as its primary contract.
10. Event delivery starts with cursor-based `events_since(cursor)` long-poll; true streaming is a later optimization.
11. Idempotency keys and apply tokens are separate controls.
12. Local security is per-user and local-only in the first wave; remote/multi-user auth is explicitly out of scope.
13. First operation catalog stays read-heavy: hello/status/capabilities/endpoints/events/session/project resolve/status/wizard inspect/draft/validate/diff/job status/receipt get.
14. Mutation apply operations wait until registry, state store, idempotency, claim admission, and apply-token proof exist.

## Wizard State Machine And Artifact Version Update Research Pass

Scope:

- Define the wizard as a domain state machine, not as a Ratatui screen sequence.
- Cover project initialization, project reconfiguration, generated artifact updates, and generated artifact drift reporting.
- Preserve apply-token, idempotency, session, project, and materialization manifest semantics before write-capable wizard implementation.

Reference surfaces:

- `vida.config.yaml`
- `docs/product/spec/project-activation-and-configurator-model.md`
- `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
- `docs/product/spec/bootstrap-carriers-and-project-activator-model.md`
- `docs/process/agent-extensions/README.md`
- `docs/product/research/vida-service-tui-wizard-architecture-research.md`

### Finding 131: Wizard Must Be A Persisted Domain State Machine

Problem:

- If the wizard is implemented as TUI screen-local state, CLI, TUI, service jobs, and future dashboard will drift.
- Reconfigure/update flows need resumable drafts, stale draft detection, and receipts after the UI exits or service restarts.

Recommended wizard states:

```text
created
inspecting
drafting
validating
invalid
diff_ready
approval_required
apply_queued
applying
applied
stale
cancelled
blocked
failed
```

Recommended state transitions:

```text
project.resolve -> wizard.inspect -> wizard.draft.start
wizard.draft.update -> wizard.validate -> wizard.diff
wizard.diff -> apply_token.issue -> wizard.apply -> job -> events -> receipt
```

Rules:

1. The service owns wizard draft state after attach; the TUI only renders and edits it through operations.
2. A wizard draft is project-scoped and session-associated, but can be resumed by another session only through explicit handoff/reclaim policy.
3. Every draft stores `base_config_revision` and `draft_revision`.
4. If base project config or materialization manifest changes, the draft enters `stale`.
5. Apply is forbidden unless the latest validation and diff are bound to the current draft revision.

### Finding 132: Wizard Modes Need Shared Core With Different Required Inputs

Wizard modes:

1. `project_init`: create a new VIDA project activation plan for the current root.
2. `project_register`: add an existing VIDA project to the service registry.
3. `reconfigure`: update `vida.config.yaml` and enabled host/agent settings.
4. `materialization_update`: update generated docs, sidecar, agents, and manifests after template/schema version changes.
5. `service_install`: plan service install/startup integration for the current user environment.
6. `repair`: diagnose missing/stale generated artifacts and propose bounded repair.

Mode-specific constraints:

1. `project_init` requires project identity, language policy, docs roots, host system selection, agent system selection, and materialization plan.
2. `project_register` requires root path, project id evidence, config path, and state root detection.
3. `reconfigure` requires current config revision and writes only through diff/apply.
4. `materialization_update` requires existing manifest comparison and drift classification.
5. `service_install` requires platform detection and install mode; apply stays gated by service lifecycle proof.
6. `repair` can propose changes but must preserve user-modified files unless explicitly approved.

### Finding 133: Wizard Input Graph Must Model Dependencies Explicitly

Core option groups:

```text
project_identity
language_policy
docs_roots
host_system
execution_class
agent_system_mode
agent_registries
enabled_roles
enabled_skills
enabled_profiles
enabled_flows
dev_team_flow
model_profiles
pricing_policy
scoring_policy
parallelism_policy
materialization_targets
service_install_mode
update_policy
```

Dependency examples:

1. `host_system` controls available materialization modes and carrier templates.
2. `execution_class=internal` permits internal Codex carriers; `execution_class=external` requires external CLI readiness checks.
3. `agent_system_mode=disabled` disables role/skill/profile/flow selection and dev-team flow setup.
4. `agent_system_mode=hybrid` requires at least one internal-capable and one external-capable carrier group when enabled.
5. `enabled_profiles` must resolve to enabled roles and compatible skills.
6. `enabled_flows` require referenced roles/profiles/handoff contracts to resolve.
7. `dev_team.enabled=true` requires ordered roles, default carriers, task class binding, handoff outputs, and closure flow.
8. Model profile choices are constrained by runtime role, task class, write scope, reasoning floor, quality floor, and budget policy.
9. `materialization_targets=agents` requires host system template paths and agent registry validation.
10. `service_install_mode=native` requires platform support and elevated/apply confirmation where applicable.

### Finding 134: Option Types Should Be Contracted For TUI, CLI, And Dashboard

Wizard option schema needs typed controls:

```text
string
path
boolean
integer
decimal
enum_one
enum_multi
ordered_list
map
secret_ref
computed
read_only
```

Option metadata:

```text
option_id
label
description
type
required
default_source
allowed_values
depends_on
conflicts_with
enables
disables
validation_rules
diff_path
config_path
materialization_effects
restart_required
apply_requires
```

Rules:

1. TUI widgets, CLI prompts, and future dashboard controls must derive from the same option metadata where practical.
2. Validation errors must point to option ids and config/materialization paths.
3. Computed/read-only options can be displayed but not edited.
4. Secret values are never stored inline; wizard stores a `secret_ref` or marks the feature unsupported in first wave.

### Finding 135: Config Revisions Need Structured Hashing

Problem:

- Text hash only catches file changes but does not explain semantic drift.
- Direct text merge is risky for `vida.config.yaml` because ordering, comments, and generated blocks may matter.

Recommendation:

1. Track `config_file_hash` for fast drift detection.
2. Track `config_semantic_hash` for parsed normalized config state.
3. Track `config_schema_version` and `config_generator_version`.
4. Store `base_config_revision` on every wizard draft.
5. If semantic hash changes, force re-inspect before apply.
6. If only formatting/comment hash changes and parser preserves semantic identity, allow rebase with warning.

### Finding 136: Materialized Artifacts Need Per-Artifact Manifests

Manifest entry:

```text
artifact_id
path
artifact_kind
owner
generator_id
generator_version
template_id
template_version
schema_version
source_config_revision
source_registry_revisions
created_at
updated_at
last_generated_hash
current_hash
semantic_hash
drift_status
update_policy
protected_regions
receipt_ref
```

Artifact kinds:

```text
config
agents_bootstrap
sidecar
agent_template
agent_registry
skill_registry
profile_registry
flow_registry
docs_index
process_doc
research_doc
service_manifest
host_app_config
```

Owner modes:

1. `vida_generated`: update allowed when previous generated hash matches.
2. `user_owned`: report only unless explicitly included in manual repair.
3. `mixed`: update only protected VIDA regions or structured generated sections.
4. `external_tool_owned`: report compatibility, do not mutate without adapter-specific plan.

### Finding 137: Drift Classification Must Be Operator-Visible

Drift statuses:

```text
clean
missing
generated_changed_by_version
user_modified
mixed_region_changed
obsolete
conflict
untracked
unsupported_format
blocked
```

Classification rules:

1. `clean`: current hash equals last generated hash.
2. `missing`: manifest entry exists but file is absent.
3. `generated_changed_by_version`: source template/schema/generator changed and current file is still generated-clean.
4. `user_modified`: file differs from last generated hash outside known generated regions.
5. `mixed_region_changed`: generated region differs but user region can be preserved.
6. `obsolete`: manifest entry no longer belongs to current selected host/agent config.
7. `conflict`: planned update and user changes overlap.
8. `unsupported_format`: parser cannot safely classify the file.

TUI must show drift by artifact with reason, not just a bulk "update required" flag.

### Finding 138: Update Modes Need Explicit Apply Semantics

Update modes:

```text
report_only
safe_update
merge_structured
overwrite_generated
full_rematerialize
manual_conflict
skip
```

Rules:

1. `report_only` never writes.
2. `safe_update` writes only clean/generated artifacts.
3. `merge_structured` requires parser support and emits structured diff.
4. `overwrite_generated` requires `owner=vida_generated` and matching prior generated hash.
5. `full_rematerialize` requires explicit apply token and receipt because it can add/remove many files.
6. `manual_conflict` returns instructions and does not write.
7. `skip` records an explicit skipped item in the receipt.

### Finding 139: Diff Must Be Structured By Config, Registry, And File Effects

Wizard diff sections:

1. Config changes:
   - YAML path,
   - old value,
   - new value,
   - reason,
   - option id.
2. Registry changes:
   - role/skill/profile/flow added/removed/changed,
   - compatibility impact.
3. Materialization changes:
   - path,
   - action,
   - drift status,
   - update mode,
   - conflict status.
4. Service changes:
   - install/start/restart required,
   - endpoint changes,
   - service capability changes.
5. Runtime impact:
   - current session impact,
   - project registry impact,
   - required restart/reload,
   - affected clients.

Rules:

1. Diff is the artifact that apply token signs.
2. TUI renders the same structured diff that CLI can print as JSON/plain.
3. Apply cannot include hidden changes absent from the diff.

### Finding 140: Apply Token Must Bind The Exact Diff

Apply token binding fields:

```text
apply_token_id
operation_id
project_id
worktree_environment_id
session_id
wizard_session_id
base_config_revision
draft_revision
diff_hash
materialization_plan_hash
expires_at
capability_scope
```

Rules:

1. Applying with a changed draft revision rejects as stale.
2. Applying after config/materialization drift rejects as stale.
3. Applying with a token from another session requires explicit transfer/handoff policy.
4. Apply token is one-use unless operation metadata permits idempotent receipt replay.
5. Apply job writes receipt with before/after revisions and changed artifact list.

### Finding 141: Receipts Need Artifact-Level Update Evidence

Materialization/update receipt sections:

```text
config_changes
registry_changes
artifact_changes
service_changes
jobs
events_cursor_start
events_cursor_end
skipped_items
conflicts
warnings
proof
```

Artifact change entry:

```text
path
action
owner
drift_before
drift_after
hash_before
hash_after
template_version_before
template_version_after
schema_version_before
schema_version_after
reason
```

Receipt must let the operator answer:

1. What changed?
2. Why did it change?
3. Was it generated-clean or user-modified?
4. What was skipped?
5. What must be manually resolved?

### Finding 142: Wizard Resume Needs Session-Aware But Not Session-Trapped Drafts

Resume rules:

1. Original session can resume active draft directly.
2. Same user/worktree can reclaim a stale abandoned draft after lease expiry.
3. Another live session cannot silently take over an active draft.
4. Draft handoff requires a recorded transition and receipt/event.
5. TUI should list resumable drafts with status, owner session, project, age, and blocker reason.

### Finding 143: Service Events Should Represent Wizard Progress, Not Raw Implementation Logs

Wizard event kinds:

```text
wizard.created
wizard.inspect.started
wizard.inspect.completed
wizard.draft.updated
wizard.validation.failed
wizard.validation.passed
wizard.diff.ready
wizard.apply_token.issued
wizard.apply.started
wizard.artifact.updated
wizard.artifact.skipped
wizard.conflict.detected
wizard.apply.completed
wizard.apply.failed
```

Rules:

1. Events are stable operator contract.
2. Logs may include lower-level details but are not the TUI progress authority.
3. TUI progress should use job progress plus wizard events.

### Finding 144: First Wizard MVP Should Be Read-Heavy And Apply-Limited

MVP wizard capabilities:

1. project inspect,
2. current config summary,
3. current host/agent system inventory,
4. draft option graph,
5. validation,
6. structured diff,
7. drift report,
8. generated artifact update plan,
9. apply token issue for safe generated-clean updates only after Gate 0 and service state proof,
10. receipts/events for completed apply.

Deferred:

1. destructive cleanup,
2. remote auth setup,
3. complex three-way merge for arbitrary user-edited docs,
4. TaskFlow/DocFlow service migration,
5. dashboard-specific UX,
6. native service apply without plan-only gate.

### Finding 145: Wizard Screens Should Follow The State Machine

Ratatui screen map derived from the domain state:

1. Project select/resolve.
2. Inspect summary.
3. Option graph editor.
4. Validation blockers.
5. Diff review.
6. Artifact drift/update plan.
7. Apply confirmation.
8. Job progress/events.
9. Receipt detail.
10. Resume drafts.

Rules:

1. Disabled actions show operation metadata blockers.
2. Navigation cannot bypass validation/diff/apply-token gates.
3. Screens can be snapshot-tested against fixture `VidaClient` states.

### Finding 146: Proof Fixtures Should Cover Wizard Draft And Drift Before TUI

Required fixture groups:

1. `wizard.inspect` returns config summary and option graph.
2. `wizard.draft.start` records `base_config_revision`.
3. `wizard.draft.update` increments `draft_revision`.
4. invalid option dependency returns option-scoped validation problem.
5. `wizard.diff` binds to draft revision.
6. config revision changes after draft mark draft stale.
7. generated-clean artifact update is planned as `safe_update`.
8. user-modified artifact is planned as `manual_conflict` or `skip`.
9. missing artifact is planned as add/regenerate.
10. apply token signs exact diff hash.
11. changed diff rejects old apply token.
12. apply receipt lists artifact-level before/after hashes.

### Proposed Clarifications For Approval, Set 22

1. Wizard is a persisted service/domain state machine, not TUI-local state.
2. Wizard states include created/inspecting/drafting/validating/invalid/diff_ready/approval_required/apply_queued/applying/applied/stale/cancelled/blocked/failed.
3. Wizard modes are project init, project register, reconfigure, materialization update, service install planning, and repair.
4. Option metadata must be typed and shared by TUI, CLI, and future dashboard where practical.
5. Option dependencies must encode host system, execution class, agent system mode, registries, roles, skills, profiles, flows, dev-team flow, model profiles, pricing, scoring, parallelism, materialization targets, service install mode, and update policy.
6. Config revisions use both file hash and parsed semantic hash.
7. Generated artifacts require per-artifact manifests with generator/template/schema/source revisions and ownership mode.
8. Drift classification is explicit: clean, missing, generated_changed_by_version, user_modified, mixed_region_changed, obsolete, conflict, untracked, unsupported_format, blocked.
9. Update modes are explicit: report_only, safe_update, merge_structured, overwrite_generated, full_rematerialize, manual_conflict, skip.
10. Diff is structured by config, registry, materialization, service, and runtime effects; apply token signs that exact diff.
11. Apply token binds project/worktree/session/wizard/base config/draft/diff/materialization plan and expires on drift.
12. Receipts must list artifact-level before/after hashes, drift status, template/schema versions, skipped items, conflicts, and warnings.
13. Wizard drafts can be resumed or reclaimed only through session/lease-aware rules.
14. Ratatui screens follow the wizard state machine and are snapshot-tested against fixture client states.
15. First wizard MVP remains read-heavy and apply-limited until service state, idempotency, claim admission, and apply-token proof are implemented.

## Dependency Order

Implementation should not begin with Ratatui screens.

Required order:

1. Fix session identity and session-scoped continuation.
2. Define the VIDA command envelope, response, event, receipt, and client trait.
3. Extract project activation core from direct CLI mutation.
4. Add local in-process client adapter and route selected CLI operations through it.
5. Add `TarpcTransport` as the first daemon RPC adapter, still carrying `VidaCommandEnvelope`.
6. Add service project registry, project lifecycle operations, and per-project DB routing.
7. Add service job/events/receipts API and daemon install/control.
8. Prove project management and activation wizard paths against a fixture `VidaClient`.
9. Build Ratatui TUI against the shared client trait and tarpc-backed daemon client.
10. Convert more CLI commands gradually to service client mode with explicit direct fallback.
11. Add `JsonRpseeTransport` and web dashboard later if needed.

## Current Blocking Defect

The current multi-session runtime behavior must be fixed before TUI/service implementation.

Observed root problem:

- `stable_local_worktree_session_id` uses worktree/state root identity as session identity.
- global latest run/continuation evidence can block unrelated current-session work.
- live foreign sessions can be treated as a broad mutation gate instead of visibility plus claim-scoped conflicts.

Required correction:

- canonical `VIDA_SESSION_ID`,
- per-session fallback token,
- current-session scoped status/continuation,
- claim-scoped admission blockers,
- global blockers only for real shared state integrity problems.

## Open Questions

1. Should `VIDA_SESSION_ID` be a global CLI flag, inherited environment variable, service attach token, or all three?
2. Should the generated fallback token live in `.vida/data/state/orchestrator-sessions`, a service registry, or a separate local client cache?
3. Should service installation be inside the main `vida` binary or split into a helper binary later?
4. Should TUI support offline direct mode before service exists, or wait until service/client envelope exists?
5. Should the first TUI MVP manage only activation/status, or also skills/roles/profiles/flows inventory?

Current recommended answers:

1. Support all three, but normalize to one `VidaSessionIdentity`.
2. Reuse or extend the existing orchestrator session store first.
3. Keep install commands in `vida` initially.
4. Build core plan/apply first; allow CLI direct fallback, but make TUI primarily service-attached.
5. MVP should include activation/status plus read-only inventory for skills/roles/profiles/flows; mutation can follow after core lifecycle APIs are stable.

-----
artifact_path: product/research/vida-service-tui-wizard-architecture-research
artifact_type: product_research_doc
artifact_version: 1
artifact_revision: 2026-05-21
schema_version: 1
status: canonical
source_path: docs/product/research/vida-service-tui-wizard-architecture-research.md
created_at: 2026-05-21T08:10:55.0420002Z
updated_at: 2026-05-21T11:26:17.4146326Z
changelog_ref: vida-service-tui-wizard-architecture-research.changelog.jsonl
