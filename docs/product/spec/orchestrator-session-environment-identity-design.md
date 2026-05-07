# Orchestrator Session Environment Identity Design

Status: `approved`

## Summary
- Feature / change: bind VIDA runtime state mutations and diagnostics to a concrete orchestrator session/environment identity, with stale-owner reclaim and transfer paths.
- Owner layer: `runtime-family`
- Runtime surface: `launcher | taskflow | status | lane | project activation`
- Status: `approved for bounded implementation`

## Current Context
- Existing system overview
  - `vida orchestrator-init`, `vida status`, `vida taskflow recovery latest`, `vida taskflow consume continue`, and `vida taskflow continuation bind` already expose run-graph, recovery, continuation, and write-guard truth.
  - The state store persists run-scoped rows such as `RunGraphStatus`, `RunGraphDispatchReceipt`, `RunGraphDispatchContext`, `RunGraphContinuationBinding`, replay lineage, and projection checkpoints.
  - The current "latest" surfaces read the latest persisted run-graph or explicit continuation binding for the project state root, not for the calling orchestrator session.
- Key components and relationships
  - `crates/vida/src/init_surfaces.rs` builds `orchestrator-init` and `agent-init` startup/operator views.
  - `crates/vida/src/status_surface.rs`, `status_surface_truth_inputs.rs`, `status_surface_json_report.rs`, and `status_surface_text_report.rs` render global runtime state, root write guard, and continuation binding.
  - `crates/vida/src/taskflow_run_graph.rs` renders run-graph status/recovery and mutates run-graph status during run init/update paths.
  - `crates/vida/src/taskflow_consume_resume.rs` resolves default resume run id from latest state and emits `continuation_binding_ambiguous` style blockers.
  - `crates/vida/src/taskflow_continuation.rs` records explicit and derived continuation bindings.
  - `crates/vida/src/state_store_run_graph_state.rs` and `state_store_run_graph_summary.rs` own persisted run-graph structs and state-store reads/writes.
  - `crates/vida/src/lane_surface.rs` exposes lane show/complete/exception-takeover/reclaim, but reclaim is currently lane-reservation oriented rather than orchestrator-owner oriented.
- Current pain point or gap
  - GitHub issue #116 shows `taskflow recovery latest --json` recommending a continuation bind for a missing task after another context advanced or published state.
  - Runtime output does not tell the operator whether the latest state belongs to the current orchestrator session, another live session, a stale lease, or a global legacy row without owner evidence.
  - Execution context and owner/publication context are conflated: the downstream project/worktree where `vida` runs is not the same thing as the upstream VIDA repository or GitHub issue tracker that owns runtime defect publication.

## Goal
- Bind every runtime state mutation that can influence latest/recovery/continuation selection to an orchestrator session identity and lease.
- Make latest/recovery/continuation/status diagnostics session-aware and explicit about current-session, other-session, stale-owner, and legacy-global evidence.
- Separate:
  - execution context: current project root, state root, worktree, host app/tool, process/command invocation
  - orchestrator ownership context: session id, lease id, heartbeat, owner status, mutation authority
  - owner/publication context: upstream repository/issue tracker used for VIDA runtime defects and publication
  - worker/carrier identity: delegated execution carrier and runtime role, not the root orchestrator owner
- Provide explicit stale-owner reclaim and transfer paths instead of silently treating another session's latest state as current-session truth.
- Out of scope:
  - changing TaskFlow task graph scheduling semantics
  - replacing run ids, task ids, or carrier/runtime-role assignment
  - implementing host-app private APIs for closing UI sessions
  - making source-code changes in this analyst lane

## Requirements

### Functional Requirements
- `vida orchestrator-init --json` must expose current `orchestrator_session_identity`, lease state, execution context, owner/publication context, and any active/stale sibling owners for the same state root.
- Mutating runtime paths must record owner evidence on new or updated run-graph status, dispatch receipts, dispatch context, continuation bindings, replay lineage, projection checkpoints, lane exception metadata, and runtime-consumption snapshots when those artifacts affect continuation or recovery.
- `latest` queries must prefer current-session state when scoped to the current session and must label global latest as `global_latest` when the selected row belongs to another session.
- `taskflow recovery latest --json`, `status --json`, `taskflow status --summary --json`, and `taskflow consume continue --json` must fail closed with a session-aware blocker when latest state belongs to a live different owner and the operator did not explicitly select/reclaim/transfer it.
- Explicit `--run-id` access remains allowed, but output must show owner evidence and whether the requested run belongs to the current session, another live session, a stale session, or legacy state.
- Stale-owner reclaim must require an explicit command and persist a reclaim/transfer receipt. It must not happen as a side effect of a read surface.
- Session transfer must require explicit source owner, target owner, run id or scope, and reason. Transfer must preserve original owner/publication evidence as lineage.
- Legacy rows without owner evidence must remain readable and must be labeled `legacy_global_owner_unknown`, not silently assigned to the current session.

### Non-Functional Requirements
- Determinism:
  - identity derivation must be testable with environment overrides and temp state roots.
  - lease expiry must be computed from stored timestamps and a configurable/testable clock.
- Compatibility:
  - schema growth must be additive, with old rows defaulting to legacy/global owner status.
  - existing tests that do not involve session conflict must continue to pass with default single-session behavior.
- Observability:
  - JSON surfaces must expose stable fields for session id, lease id, owner match, lease status, blocker code, and next actions.
  - Text output should add compact owner/lease lines only where it changes operator action.
- Security and safety:
  - one session must not mutate another live session's active run by default.
  - reclaim/transfer must not grant source write authority beyond the explicit bounded state/run scope.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/orchestrator-session-environment-identity-design.md`
  - `docs/product/spec/current-spec-map.md` for required DocFlow registration
- Framework protocols affected:
  - AGENTS bootstrap continuation fields
  - orchestrator operating protocol session-start expectations
  - team development and orchestration protocol lane/receipt ownership language
- Runtime families affected:
  - `taskflow`
  - launcher init/status surfaces
  - lane recovery/reclaim surfaces
  - runtime consumption snapshots
- Config / receipts / runtime surfaces affected:
  - state-store run-graph tables and summaries
  - continuation binding and resume selection
  - recovery latest/status diagnostics
  - `orchestrator-init`, `agent-init`, `status`, `taskflow status`, `taskflow consume continue`, `taskflow continuation bind`, `lane show/reclaim`

## Design Decisions

### 1. State-Store-Owned Session Identity And Lease
Will implement / choose:
- Add an `orchestrator_session_identity` helper and state-store records for session identity, lease heartbeat, owner status, and reclaim/transfer receipts.
- The helper resolves identity from the current project/state root, current executable, process id, optional host-provided environment values, and a persisted session id/lease id.
- Why:
  - The defect is about authoritative runtime state ownership, so ownership must live beside state mutation and recovery evidence, not only in transient process memory.
- Trade-offs:
  - Adds a small state-store schema surface and migration/defaulting logic.
- Alternatives considered:
  - Environment-only session ids. Rejected because recovery surfaces need persisted owner evidence after the original process exits.
  - Process-id-only ownership. Rejected because orchestrator sessions can span multiple `vida` invocations and pids.

### 2. Latest Becomes Session-Aware, Not Replaced
Will implement / choose:
- Keep existing explicit per-run reads.
- Add session-aware latest helpers that return:
  - current-session latest
  - global latest with owner label
  - conflicting live owner blockers
  - stale owner reclaim candidates
  - legacy global owner-unknown rows
- Why:
  - Existing operator and test flows depend on latest behavior. The safe fix is to label and gate latest resolution rather than remove it.
- Trade-offs:
  - Some surfaces will contain both current-session and global/latest evidence.
- Alternatives considered:
  - Hard partitioning state by session id immediately. Rejected as too disruptive for existing run-graph and recovery flows.

### 3. Execution Context And Publication Context Are Separate First-Class Blocks
Will implement / choose:
- Emit separate JSON blocks:
  - `execution_context_identity`
  - `orchestrator_session_identity`
  - `owner_publication_context`
  - `selected_owner_evidence`
- Why:
  - Issue #116 specifically shows the operator confused by a downstream project execution context while the runtime defect belongs to the upstream VIDA repository/issue context.
- Trade-offs:
  - Slightly larger operator JSON.
- Alternatives considered:
  - One generic `context` object. Rejected because it preserves the ambiguity this design fixes.

### 4. Reclaim And Transfer Are Explicit Mutations With Receipts
Will implement / choose:
- Add a bounded command surface, preferably under `vida taskflow session`, for:
  - `list --json`
  - `show <session-id> --json`
  - `heartbeat --json` for internal/runtime use where needed
  - `reclaim --session-id <id> [--run-id <id>] --reason <text> --json`
  - `transfer --from-session <id> --to-current-session [--run-id <id>] --reason <text> --json`
- Why:
  - Reclaim/transfer changes ownership and must be auditable.
- Trade-offs:
  - Adds a small operator surface.
- Alternatives considered:
  - Reusing `vida lane reclaim` only. Rejected because lane reclaim is about execution lanes/reservations, while this defect is root orchestrator state ownership.

## Technical Design

### Core Components
- `orchestrator_session_identity` helper
  - Derives and validates current session identity.
  - Provides test-only deterministic overrides for session id, host app/thread id, clock, and lease TTL.
  - Builds `execution_context_identity` and `owner_publication_context`.
- State-store owner records
  - Persist session identity, heartbeat, lease status, reclaim/transfer receipts, and owner fields on continuation-affecting artifacts.
  - Provide compatibility defaulting for ownerless legacy rows.
- Session-aware latest selector
  - Reads latest status/receipt/binding evidence and classifies it relative to current session.
  - Feeds continuation binding and recovery/status output.
- Operator surfaces
  - Render current owner, selected evidence owner, conflicts, stale reclaim candidates, and exact next actions.

### Data / State Model
- New state entities:
  - `OrchestratorSessionRecord`
    - `session_id`
    - `lease_id`
    - `state_root`
    - `project_root`
    - `workspace_fingerprint`
    - `host_app`
    - `host_thread_id`
    - `process_id`
    - `active_bounded_unit`
    - `started_at`
    - `heartbeat_at`
    - `lease_expires_at`
    - `status`
  - `OrchestratorSessionTransferReceipt`
    - `receipt_id`
    - `from_session_id`
    - `to_session_id`
    - `run_id`
    - `reason`
    - `previous_owner_evidence`
    - `recorded_at`
  - `RuntimeOwnerEvidence`
    - embedded on run-graph status, dispatch receipt/context, continuation binding, replay lineage, projection checkpoint, and session-affecting runtime snapshots
    - fields: `orchestrator_session_id`, `orchestrator_lease_id`, `owner_status`, `execution_context_id`, `publication_context_id`, `recorded_at`
- Compatibility notes:
  - Missing owner fields deserialize as `legacy_global_owner_unknown`.
  - Legacy rows remain readable through explicit `--run-id`.
  - Default single-session behavior should remain unchanged when no conflicting live owner exists.
  - Lease status values should be stable strings: `current_owner`, `other_owner_live`, `other_owner_stale`, `legacy_global_owner_unknown`, `transferred`, `reclaimed`.

### Integration Points
- `vida orchestrator-init --json`
  - Ensure or refresh current session lease before rendering normal startup output.
  - Surface sibling owners for the same state root.
- `vida agent-init --json`
  - Include current orchestrator owner evidence in activation and dispatch output.
  - Do not treat worker/carrier id as owner session.
- `vida status --json` and `vida status --summary --json`
  - Add session ownership block and classify latest run-graph/continuation evidence owner.
- `vida taskflow recovery latest|status --json`
  - Latest must be scoped/labeled. Cross-session global latest must expose a blocker and targeted reclaim/transfer/explicit-run next actions.
- `vida taskflow consume continue --json`
  - Default run selection must reject other-live-owner latest instead of recommending a bind against another session's state.
- `vida taskflow continuation bind ... --json`
  - Binding must record current owner evidence and reject binding another live session's run unless transfer/reclaim evidence exists.
- `vida lane show|reclaim --json`
  - Lane output must show owner evidence for exception-takeover receipts. Existing lane reclaim can remain lane-focused; session reclaim should be separate.

### Bounded File Set
- Documentation:
  - `docs/product/spec/orchestrator-session-environment-identity-design.md`
  - `docs/product/spec/current-spec-map.md` for required DocFlow registration
- Source:
  - `crates/vida/src/main.rs`
  - `crates/vida/src/cli.rs`
  - `crates/vida/src/orchestrator_session_identity.rs`
  - `crates/vida/src/state_store.rs`
  - `crates/vida/src/state_store_run_graph_state.rs`
  - `crates/vida/src/state_store_run_graph_summary.rs`
  - `crates/vida/src/init_surfaces.rs`
  - `crates/vida/src/status_surface.rs`
  - `crates/vida/src/status_surface_truth_inputs.rs`
  - `crates/vida/src/status_surface_json_report.rs`
  - `crates/vida/src/status_surface_text_report.rs`
  - `crates/vida/src/status_surface_operator_contracts.rs`
  - `crates/vida/src/taskflow_run_graph.rs`
  - `crates/vida/src/taskflow_consume_resume.rs`
  - `crates/vida/src/taskflow_continuation.rs`
  - `crates/vida/src/continuation_binding_summary.rs`
  - `crates/vida/src/taskflow_proxy.rs`
  - `crates/vida/src/lane_surface.rs`
  - `crates/vida/src/runtime_consumption_state.rs`
  - `crates/vida/src/runtime_consumption_surface.rs`
  - `crates/vida/src/runtime_dispatch_state.rs`
  - `crates/vida/src/runtime_dispatch_status.rs`

## Fail-Closed Constraints
- Do not continue default `latest` recovery or consume continuation from another live session's state.
- Do not auto-reclaim a stale session from read-only surfaces.
- Do not overwrite owner evidence when transferring; append transfer lineage.
- Do not treat activation view-only, worker carrier identity, or runtime role as root orchestrator ownership.
- Do not infer publication/upstream repository from the downstream execution project root.
- Do not assign legacy ownerless rows to the current session without an explicit reclaim/transfer receipt.
- Do not use session ownership to broaden root-local write authority; write authority remains governed by existing write guard and scoped exception receipts.

## Implementation Plan

### Phase 1
- Add `orchestrator_session_identity` module and state-store session/lease records.
- Thread current owner evidence into init/status output without changing selection behavior yet.
- First proof target:
  - identity derivation unit tests with deterministic env/clock overrides
  - schema/defaulting tests for legacy ownerless rows

### Phase 2
- Add owner evidence to run-graph status, dispatch receipts/context, continuation binding, replay lineage, projection checkpoints, and runtime-consumption snapshots.
- Add session-aware latest helper and feed `status`, `recovery latest`, and `consume continue`.
- Second proof target:
  - two-session same-state-root tests where default latest blocks on other live owner
  - stale-owner classification tests

### Phase 3
- Add explicit `taskflow session` list/show/reclaim/transfer commands and wire continuation bind/recovery next actions to them.
- Final proof target:
  - reclaim/transfer receipt tests
  - runtime surface JSON tests for orchestrator-init/status/recovery/consume/continuation bind

## Validation / Proof
- Unit tests:
  - parse/build `OrchestratorSessionIdentity` from deterministic env overrides
  - lease heartbeat and expiry classification
  - owner/publication/execution context separation
  - legacy ownerless row defaulting to `legacy_global_owner_unknown`
  - session-aware latest selector prefers current-session latest over global other-session latest
  - live other-session latest returns blocker `orchestrator_session_owner_conflict`
  - stale other-session latest returns reclaim candidate but does not reclaim without command
- Integration tests:
  - same temp state root with two simulated sessions; both active sessions are reported by `orchestrator-init`
  - `taskflow recovery latest --json` labels current/global latest and blocks on live owner conflict
  - `taskflow consume continue --json` does not recommend binding a missing task from another owner's state
  - `taskflow continuation bind` rejects live other-owner runs unless transfer/reclaim receipt exists
  - reclaim then continue succeeds for stale owner and records transfer lineage
- Runtime checks:
  - `target\debug\vida.exe orchestrator-init --json`
  - `target\debug\vida.exe status --json`
  - `target\debug\vida.exe taskflow recovery latest --json`
  - `target\debug\vida.exe taskflow consume continue --json`
  - `target\debug\vida.exe taskflow session list --json`
  - `target\debug\vida.exe taskflow session reclaim --session-id <id> --reason <text> --json`
- Canonical checks:
  - `target\debug\vida.exe docflow check-file --path docs/product/spec/orchestrator-session-environment-identity-design.md`
  - `cargo fmt -p vida -- --check`
  - targeted `cargo test -p vida orchestrator_session_identity -- --nocapture`
  - targeted `cargo test -p vida session_aware_latest -- --nocapture`
  - targeted `cargo test -p vida continuation_binding -- --nocapture`

## Observability
- JSON fields:
  - `orchestrator_session_identity`
  - `execution_context_identity`
  - `owner_publication_context`
  - `selected_owner_evidence`
  - `latest_scope`
  - `owner_match`
  - `lease_status`
  - `active_orchestrator_sessions`
  - `stale_orchestrator_sessions`
  - `owner_conflict`
  - `reclaim_transfer_receipt`
- Blocker codes:
  - `orchestrator_session_owner_conflict`
  - `orchestrator_session_stale_owner_reclaim_required`
  - `orchestrator_session_legacy_owner_unknown`
  - `orchestrator_session_transfer_required`
- Next actions should name exact commands and preserve the affected run id/session id.

## Rollout Strategy
- Roll out additively behind default single-session-compatible behavior.
- Treat legacy rows as readable but owner-unknown until explicit reclaim/transfer or fresh mutation records owner evidence.
- Keep JSON field additions additive.
- No operator restart is required beyond rebuilding/installing the VIDA binary, but host apps may optionally set session identity environment variables for stronger continuity.

## Future Considerations
- Add host-app provided stable thread/session identity once the host exposes it as a documented variable or connector field.
- Add per-session state namespace partitioning after all latest consumers use session-aware selectors.
- Add stale-session cleanup automation only after explicit reclaim/transfer receipt semantics are proven.
- Extend documentation law so downstream projects can publish owner/publication context without hardcoding upstream repository assumptions.

## References
- GitHub issue #116: Bind VIDA runtime state to orchestrator session and environment identity
- `docs/process/team-development-and-orchestration-protocol.md`
- `docs/process/project-orchestrator-operating-protocol.md`
- `docs/product/spec/canonical-runtime-readiness-law.md`
- `docs/product/spec/canonical-runtime-layer-matrix.md`
- `docs/product/spec/reconciled-runtime-projection-output-design.md`
- `docs/product/spec/orchestrator-runtime-contract-hardening-design.md`
- `crates/vida/src/state_store_run_graph_state.rs`
- `crates/vida/src/state_store_run_graph_summary.rs`
- `crates/vida/src/taskflow_run_graph.rs`
- `crates/vida/src/taskflow_consume_resume.rs`
- `crates/vida/src/continuation_binding_summary.rs`

-----
artifact_path: product/spec/orchestrator-session-environment-identity-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-05-07
schema_version: 1
status: canonical
source_path: docs/product/spec/orchestrator-session-environment-identity-design.md
created_at: 2026-05-07T00:00:00+03:00
updated_at: 2026-05-07T00:00:00+03:00
changelog_ref: orchestrator-session-environment-identity-design.changelog.jsonl
