# Repair Explicit Continuation Bind Preservation After Qwen Rebind Design

Status: `approved`

## Summary
- Feature / change: preserve an explicit post-closure `task_graph_task` continuation bind across qwen remediation handoff so `consume continue` and `agent-init` stop falling back to stale run/request lineage before a fresh packet for the bound task exists.
- Owner layer: `mixed`
- Runtime surface: `taskflow | launcher | docflow`
- Status: approved

## Current Context
- Existing system overview
  - The runtime already supports explicit continuation binding for post-closure work and previously hardened stale packet reuse to fail closed when packet lineage and bound task disagree.
  - The qwen cleanup task is now a child of the active audit-remediation epic, and its design doc is finalized at `docs/product/spec/reconcile-qwen-cli-carrier-drift-design.md`.
  - Source-tree `orchestrator-init --json` correctly reflects an explicit continuation bind to the new task when that bind is freshly recorded.
- Key components and relationships
  - `crates/vida/src/taskflow_continuation.rs` records explicit `task_graph_task` continuation bindings.
  - `crates/vida/src/taskflow_consume_resume.rs` reconstructs resume inputs and can still reconcile against persisted dispatch packet lineage.
  - `crates/vida/src/init_surfaces.rs` renders `agent-init` activation views and exposes `packet_activation_evidence`.
  - `crates/vida/src/taskflow_run_graph.rs` projects run-graph truth and can still advertise stale run/task lineage if reconciliation wins over the explicit bind.
- Current pain point or gap
  - Fresh source-tree proof on 2026-04-21 shows:
    - `cargo run -p vida -- orchestrator-init --json` truthfully reports `feature-reconcile-qwen-cli-carrier-drift-across-config-code` as the active bounded unit after explicit bind.
    - `cargo run -p vida -- agent-init --json \"<bounded qwen request>\"` stays activation-view-only with `packet_activation_evidence = null` and does not create fresh packet truth for the bound task.
    - `cargo run -p vida -- taskflow consume continue --run-id feature-reconcile-autonomous-execution-flag-runtime-drift --json` then rewrites continuation truth back to stale closed-run lineage (`feature-reconcile-autonomous-execution-flag-runtime-drift`) instead of preserving the explicit qwen bind or fail-closing against the missing fresh packet.
  - This blocks lawful delegated progression for the qwen cleanup task and violates the already-closed continuation-binding invariant in a new regression shape.

## Goal
- What this change should achieve
  - Keep an explicit `task_graph_task` continuation bind authoritative until the runtime records a fresh lawful packet/receipt or explicit fail-closed blocker for that same bound task.
  - Prevent `consume continue` from restoring stale run/request lineage after an explicit rebind to a new backlog task.
  - Keep `agent-init` activation views non-executing without letting them erase or supersede the explicit bind.
- What success looks like
  - After explicit rebind to the qwen blocker or qwen cleanup task, `orchestrator-init`, `consume continue`, and related status surfaces continue to point at that bound task until fresh same-task dispatch evidence exists.
  - If no lawful fresh packet can be synthesized yet, runtime fails closed explicitly instead of rewinding to the old closed run/task request.
  - The upstream blocker task can then shape the next lawful packet for qwen remediation without stale lineage bounce-back.
- What is explicitly out of scope
  - The qwen docs/tests cleanup itself.
  - Broad redesign of activation-view semantics into auto-execution.
  - Reopening older closed current-release work that is already satisfied unless the exact regression demands it.

## Requirements

### Functional Requirements
- Must preserve explicit `task_graph_task` continuation binding as the primary authority when packet lineage still belongs to an older completed run/task.
- Must not let `taskflow consume continue` rewrite continuation truth back to stale request text or stale `run_graph_task` lineage after a newer explicit bind exists.
- Must keep `agent-init` activation-view behavior non-executing while ensuring it does not implicitly clear or override the explicit bound task.
- Must either synthesize fresh lawful packet/dispatch-init truth for the bound task or fail closed with an explicit blocker tied to that same task.
- Must preserve lawful reuse only when explicit bind and persisted packet lineage still agree.

### Non-Functional Requirements
- Performance
  - No significant runtime overhead beyond bounded reconciliation checks.
- Scalability
  - The fix should generalize beyond qwen cleanup to future explicit post-closure rebinding events.
- Observability
  - Operator surfaces should expose why the explicit bind remains authoritative or why a fail-closed blocker is emitted.
- Security
  - The fix must not unlock root-local write authority or treat activation views as execution evidence.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/repair-explicit-continuation-bind-preservation-after-qwen-rebind-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/current-spec-provenance-map.md`
  - `docs/product/spec/reconcile-qwen-cli-carrier-drift-design.md`
  - `docs/product/spec/continuation-binding-fail-closed-hardening-design.md`
  - `docs/product/spec/continuation-and-seeded-dispatch-bridge-design.md`
  - `docs/product/spec/lawful-closure-continuation-rebinding-design.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `taskflow`
  - `launcher`
- Config / receipts / runtime surfaces affected:
  - explicit continuation-binding records
  - `vida taskflow consume continue`
  - `vida agent-init`
  - run-graph/status/init projection truth

## Design Decisions

### 1. Explicit post-closure task bind remains authoritative until same-task packet truth exists
Will implement / choose:
- Reconciliation logic must prefer the latest explicit `task_graph_task` continuation bind over stale run/request lineage until a fresh lawful packet or blocker is recorded for that same task.
- Why
  - The whole point of explicit continuation binding is to change the active bounded unit without heuristic fallback.
- Trade-offs
  - Some existing convenience paths that reused old run context will now fail closed sooner.
- Alternatives considered
  - Continue letting stale run lineage win whenever fresh packet truth is absent.
  - Rejected because that recreates the exact stale-packet regression that was already supposed to be closed.
- ADR link if this must become a durable decision record
  - none

### 2. Activation-view-only `agent-init` is not execution evidence and not continuation-authority
Will implement / choose:
- Keep `agent-init` activation views non-executing, but ensure their absence of `packet_activation_evidence` cannot demote the explicit continuation bind.
- Why
  - Activation-view-only was already defined as a blocker/bridge condition, not delegated completion.
- Trade-offs
  - Operators may see more explicit fail-closed messaging instead of implicit fallback.
- Alternatives considered
  - Treat activation-view-only as permission to resume old packet lineage.
  - Rejected because it erases explicit task rebinding and reintroduces stale context.
- ADR link if needed
  - none

### 3. `consume continue` must either reshape same-task truth or fail closed explicitly
Will implement / choose:
- When explicit bind points at a different task than the persisted packet lineage, `consume continue` must not restore the old run/task request. It must either materialize fresh same-task continuation inputs or emit a fail-closed mismatch/blocker outcome.
- Why
  - This keeps runtime progression lawful and debuggable instead of silently rewinding the active unit.
- Trade-offs
  - Some workflows may require one extra dispatch-init/packet-shaping step before execution resumes.
- Alternatives considered
  - Patch only status projections while leaving `consume continue` behavior unchanged.
  - Rejected because the root defect lives in resume authority, not only in display surfaces.
- ADR link if needed
  - none

## Technical Design

### Core Components
- Main components
  - explicit continuation bind records
  - resume-input reconciliation
  - `agent-init` activation-view payloads
  - run-graph/init projection summaries
- Key interfaces
  - `cargo run -p vida -- orchestrator-init --json`
  - `cargo run -p vida -- taskflow consume continue --run-id <run-id> --json`
  - `cargo run -p vida -- agent-init --json \"<bounded request>\"`
- Bounded responsibilities
  - continuation binding must stay authoritative after explicit rebind
  - agent-init activation views must remain inert but truth-preserving
  - consume/resume must never rehydrate stale request/task lineage over a newer explicit bind

### Data / State Model
- Important entities
  - explicit continuation bind record
  - bound `task_graph_task`
  - stale persisted dispatch packet lineage
  - activation-view-only worker handoff
  - same-task fresh packet truth
- Receipts / runtime state / config fields
  - `active_bounded_unit`
  - `binding_source`
  - `why_this_unit`
  - `request_text`
  - `packet_activation_evidence`
  - `dispatch_packet_path`
  - `dispatch_result_path`
- Migration or compatibility notes
  - Historical completed runs should remain readable, but stale packet reuse must lose authority whenever a newer explicit bind points elsewhere.

### Integration Points
- APIs
  - none external
- Runtime-family handoffs
  - explicit continuation bind -> orchestrator/init summaries
  - explicit bind + no fresh packet -> agent-init activation view
  - explicit bind + resume -> fail-closed blocker or fresh same-task packet
- Cross-document / cross-protocol dependencies
  - continuation-binding fail-closed law
  - seeded dispatch bridge law
  - lawful post-closure continuation rebinding
  - qwen cleanup blocker relationship

### Bounded File Set
- `docs/product/spec/repair-explicit-continuation-bind-preservation-after-qwen-rebind-design.md`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/current-spec-provenance-map.md`
- `crates/vida/src/taskflow_consume_resume.rs`
- `crates/vida/src/taskflow_continuation.rs`
- `crates/vida/src/init_surfaces.rs`
- `crates/vida/src/taskflow_run_graph.rs`
- `crates/vida/src/runtime_dispatch_packet_text.rs` only if request-text authority must be re-sourced during repair

## Fail-Closed Constraints
- Forbidden fallback paths
  - No fallback from explicit qwen blocker bind to stale `feature-reconcile-autonomous-execution-flag-runtime-drift` request/task lineage.
  - No treating activation-view-only `agent-init` output as execution evidence.
- Required receipts / proofs / gates
  - Explicit bind must remain visible in `orchestrator-init` and survive `consume continue` unless same-task fresh packet truth supersedes it.
  - If no same-task packet can be produced yet, runtime must fail closed with explicit blocker evidence.
- Safety boundaries that must remain true during rollout
  - Root-session write guard remains blocked by default.
  - No local implementation authority is unlocked by this fix.
  - The downstream qwen cleanup task remains blocked until this owner slice is resolved.

## Implementation Plan

### Phase 1
- Finalize this bounded blocker design and register it in the current spec canon.
- Confirm the exact regression boundary against the earlier closed stale-packet bug.
- First proof target
  - `cargo run -p vida -- docflow check --root . docs/product/spec/repair-explicit-continuation-bind-preservation-after-qwen-rebind-design.md`

### Phase 2
- Repair continuation/resume authority so explicit task binds survive until same-task packet truth exists.
- Ensure `agent-init` activation-view surfaces preserve bind truth without implying execution.
- Second proof target
  - targeted `cargo test -p vida ...` for continuation/consume/agent-init regression coverage

### Phase 3
- Re-run source-tree runtime proofs for the qwen blocker and confirm the active bounded unit no longer bounces back to stale lineage.
- Return the audit epic to qwen cleanup only after this blocker is green.
- Final proof target
  - source-tree `orchestrator-init`, `consume continue`, and `agent-init` evidence aligned on the blocker task

## Validation / Proof
- Unit tests:
  - explicit bind vs stale packet mismatch reconciliation
  - activation-view-only `agent-init` preserving bind truth
  - lawful no-op reuse when bind and packet lineage still match
- Integration tests:
  - bounded qwen blocker repro over `feature-reconcile-autonomous-execution-flag-runtime-drift`
- Runtime checks:
  - `cargo run -p vida -- orchestrator-init --json`
  - `cargo run -p vida -- taskflow consume continue --run-id feature-reconcile-autonomous-execution-flag-runtime-drift --json`
  - `cargo run -p vida -- agent-init --json \"<bounded blocker request>\"`
- Canonical checks:
  - `cargo run -p vida -- docflow check --root . docs/product/spec/repair-explicit-continuation-bind-preservation-after-qwen-rebind-design.md`
  - `cargo run -p vida -- docflow check --root . docs/product/spec/current-spec-map.md docs/product/spec/current-spec-provenance-map.md`

## Observability
- Logging points
  - continuation binding reconciliation
  - resume-input authority selection
  - agent-init activation-view payload shaping
- Metrics / counters
  - none new expected
- Receipts / runtime state written
  - continuation-binding records
  - fail-closed blocker or fresh same-task packet evidence
  - docflow changelog artifacts

## Rollout Strategy
- Development rollout
  - land the continuation/agent-init truth repair first, then return to qwen cleanup
- Migration / compatibility notes
  - old completed runs remain readable, but stale packet reuse after explicit rebind must no longer be admissible
- Operator or user restart / restart-notice requirements
  - source-tree `cargo run -p vida -- ...` remains the canonical proof surface until installed binary refresh is explicitly requested

## Future Considerations
- Follow-up ideas
  - emit a more explicit operator-facing blocker code when explicit bind exists but no same-task packet has been materialized yet
- Known limitations
  - this slice does not itself repair the qwen docs/tests drift; it only restores the lawful handoff path back to that task
- Technical debt left intentionally
  - no broader redesign of worker activation/execution surfaces beyond the exact bind-preservation regression

## References
- `docs/product/spec/reconcile-qwen-cli-carrier-drift-design.md`
- `docs/product/spec/continuation-binding-fail-closed-hardening-design.md`
- `docs/product/spec/continuation-and-seeded-dispatch-bridge-design.md`
- `docs/product/spec/lawful-closure-continuation-rebinding-design.md`
- `crates/vida/src/{taskflow_consume_resume,taskflow_continuation,init_surfaces,taskflow_run_graph}.rs`
- task `feature-repair-explicit-continuation-bind-preservation-after-qwen-rebind`

-----
artifact_path: product/spec/repair-explicit-continuation-bind-preservation-after-qwen-rebind-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-21
schema_version: 1
status: canonical
source_path: docs/product/spec/repair-explicit-continuation-bind-preservation-after-qwen-rebind-design.md
created_at: 2026-04-21T18:19:05.078092952Z
updated_at: 2026-04-21T18:20:56.351856238Z
changelog_ref: repair-explicit-continuation-bind-preservation-after-qwen-rebind-design.changelog.jsonl
