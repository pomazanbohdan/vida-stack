# Repair Design Backed Reseed Canonicalization Does Not Deadlock Qwen Design

Status: `approved`

## Summary
- Feature / change: repair the fresh qwen blocker path so an explicit design-backed remediation task no longer reseeds into a `pbi_discussion/pm` conversation shape that later canonicalizes into `dispatch_target=specification` with `runtime_role=pm` and `selected_backend=null`.
- Owner layer: `mixed`
- Runtime surface: `taskflow | launcher | docflow`
- Status: approved

## Current Context
- Existing system overview
  - The qwen cleanup task now has an approved bounded design doc at `docs/product/spec/reconcile-qwen-cli-carrier-drift-design.md`.
  - Explicit continuation binding for post-closure backlog tasks is already preserved by the freshly landed blocker fix in `taskflow_consume_resume.rs` and `taskflow_continuation.rs`.
  - `vida taskflow run-graph dispatch-init <old-run-id> --json` can now lawfully reseed the explicit qwen task into a fresh run id equal to the qwen task id.
- Key components and relationships
  - `crates/vida/src/taskflow_run_graph.rs` seeds a fresh run-graph status from task/request text and may auto-upgrade existing design-backed implementation work.
  - `crates/vida/src/taskflow_consume.rs` canonicalizes `dispatch_target` from the seeded status and builds the runtime dispatch receipt.
  - `crates/vida/src/taskflow_routing.rs` maps runtime roles to canonical dispatch targets and lane contracts.
  - `crates/vida/src/runtime_dispatch_state.rs` shapes the packet body, handoff runtime role, and backend/effective posture truth used by `agent-init`.
  - `crates/vida/src/runtime_lane_summary.rs` contributes the weak-vs-explicit intent heuristics that can still seed `pbi_discussion`.
- Current pain point or gap
  - Fresh 2026-04-21 source-tree proof shows:
    - `cargo run -p vida -- taskflow run-graph dispatch-init feature-reconcile-autonomous-execution-flag-runtime-drift --json` reseeds the qwen task into a fresh run `feature-reconcile-qwen-cli-carrier-drift-across-config-code`.
    - That reseeded run has `task_class=pbi_discussion`, `next_node=pm`, and `tracked_flow_entry=work-pool-pack`.
    - The generated dispatch packet then canonicalizes to:
      - `dispatch_target = specification`
      - `handoff_runtime_role = pm`
      - `activation_agent_type = null`
      - `selected_backend = null`
    - `cargo run -p vida -- agent-init --dispatch-packet <fresh-packet> --execute-dispatch --json` fails closed with:
      - `Dispatch target specification is routed to an agent lane but no lawful backend could be resolved from the execution route`
  - This is not the same owner as the previously closed specification-backend bug. The live defect is a routing/canonicalization mismatch introduced by the new reseeded shape for a design-backed bounded remediation task.

## Goal
- What this change should achieve
  - Ensure a design-backed bounded remediation task reseeds into a lawful route that preserves agreement between task class, next node, canonical dispatch target, activation runtime role, and selected backend.
  - Prevent conversation/default-route activation metadata from surviving when the dispatch target has already been canonicalized into a development/specification lane.
  - Unblock lawful delegated progression from the fresh qwen run into either the correct implementation lane or the correct specification lane with admissible activation/backend truth.
- What success looks like
  - The reseeded qwen run no longer produces `dispatch_target=specification` with `runtime_role=pm` and `selected_backend=null`.
  - `vida agent-init --dispatch-packet <fresh-packet> --execute-dispatch --json` either executes a lawful lane or fails closed on a more specific gate that preserves canonical route truth.
  - The qwen cleanup task can then resume as the next lawful bounded unit instead of spawning another routing blocker.
- What is explicitly out of scope
  - The qwen docs/tests/config cleanup itself.
  - Broad redesign of all conversation lanes.
  - Reopening older closed owner tasks unless the exact new mismatch proves they are incomplete.

## Requirements

### Functional Requirements
- Must not reseed a design-backed bounded remediation task into weak `pbi_discussion` routing when the approved design doc already fixes bounded file ownership and proof targets for implementation-shaped work.
- If `dispatch_target` is canonicalized away from a conversational role alias, activation/runtime-role/backend selection must also canonicalize to the same lane contract instead of mixing `default_route` conversation activation with the canonical lane target.
- Must prevent packet shapes where `dispatch_target=specification` but `handoff_runtime_role=pm`.
- Must ensure the fresh qwen packet resolves an admissible backend or fails closed with a more specific canonical route/gate blocker than `no lawful backend could be resolved`.
- Must preserve already-correct behavior for genuine scope/spec discussions and work-pool conversations.

### Non-Functional Requirements
- Performance
  - No significant overhead beyond bounded route/canonicalization checks during reseed and packet shaping.
- Scalability
  - The fix should generalize to future explicit design-backed remediation tasks beyond qwen cleanup.
- Observability
  - Operator/runtime surfaces must expose which route/canonicalization rule won and why.
- Security
  - The fix must not unlock local root write authority or bypass delegated execution law.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/repair-design-backed-reseed-canonicalization-does-not-deadlock-qwen-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/current-spec-provenance-map.md`
  - `docs/product/spec/reconcile-qwen-cli-carrier-drift-design.md`
  - `docs/product/spec/existing-design-implementation-routing-blocked-design.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `taskflow`
  - `launcher`
- Config / receipts / runtime surfaces affected:
  - seeded run-graph status
  - runtime dispatch receipt canonicalization
  - runtime dispatch packet handoff fields
  - `vida taskflow run-graph dispatch-init`
  - `vida agent-init --dispatch-packet ... --execute-dispatch`

## Design Decisions

### 1. Approved design-backed remediation tasks must not fall back to weak pbi-discussion routing
Will implement / choose:
- When a tracked task already has an approved design doc with bounded file ownership and proof targets, reseed should prefer the implementation-shaped path instead of weak `pbi_discussion` heuristics based only on generic task/backlog wording.
- Why
  - The design doc is stronger evidence than weak conversation keywords.
- Trade-offs
  - Some task descriptions that previously reopened planning/spec conversations will now route more aggressively into execution when a finalized design doc already exists.
- Alternatives considered
  - Keep current keyword-only override rules.
  - Rejected because they are too weak for explicit design-backed remediation work.
- ADR link if this must become a durable decision record
  - none

### 2. Canonical dispatch target and activation/backend resolution must stay in one lane universe
Will implement / choose:
- If receipt canonicalization maps a conversational runtime role to a canonical lane target, activation runtime role, activation agent type, and backend selection must come from that same canonical lane contract rather than the raw conversation `default_route`.
- Why
  - Mixed shapes like `specification + pm + null backend` are internally contradictory and create artificial deadlocks.
- Trade-offs
  - Some packet builders will stop inheriting `default_route` fields in mixed canonicalization cases.
- Alternatives considered
  - Patch only backend fallback selection.
  - Rejected because the primary defect is inconsistent route identity, not only missing fallback.
- ADR link if needed
  - none

### 3. Fail-closed operator truth must describe the canonical mismatch, not a downstream symptom
Will implement / choose:
- The runtime should fail on the canonical route mismatch itself when it still occurs, rather than only surfacing the later “no lawful backend” symptom.
- Why
  - This keeps future blocker creation tied to the real owner invariant.
- Trade-offs
  - Operators may see a more specific routing/canonicalization error where they previously saw a generic backend-resolution error.
- Alternatives considered
  - Leave the current generic backend error as the only evidence.
  - Rejected because it already caused stale-owner confusion once.
- ADR link if needed
  - none

## Technical Design

### Core Components
- Main components
  - design-backed reseed override
  - dispatch-target canonicalization
  - lane activation/runtime-role selection
  - admissible backend resolution
- Key interfaces
  - `cargo run -p vida -- taskflow run-graph dispatch-init <run-id> --json`
  - `cargo run -p vida -- agent-init --dispatch-packet <path> --execute-dispatch --json`
  - `cargo run -p vida -- orchestrator-init --json`
- Bounded responsibilities
  - `taskflow_run_graph` decides whether the fresh run is conversation-shaped or implementation-shaped
  - `taskflow_consume` builds a dispatch receipt whose activation/backend fields agree with the canonical target
  - `taskflow_routing` and `runtime_dispatch_state` keep packet/route truth aligned

### Data / State Model
- Important entities
  - approved design-backed remediation task
  - reseeded run-graph status
  - canonical dispatch target
  - activation runtime role
  - selected backend
- Receipts / runtime state / config fields
  - `task_class`
  - `route_task_class`
  - `next_node`
  - `tracked_flow_entry`
  - `dispatch_target`
  - `handoff_runtime_role`
  - `activation_agent_type`
  - `selected_backend`
- Migration or compatibility notes
  - Existing spec/scope discussion flows must keep their lawful canonicalization; only mismatched design-backed remediation reseeds should change.

### Integration Points
- APIs
  - none external
- Runtime-family handoffs
  - explicit continuation bind -> run-graph dispatch-init reseed
  - seeded run -> dispatch receipt canonicalization
  - dispatch receipt/packet -> agent-init execution
- Cross-document / cross-protocol dependencies
  - qwen carrier-drift remediation design
  - existing design-backed implementation routing design
  - continuation-binding preservation fix just landed

### Bounded File Set
- `docs/product/spec/repair-design-backed-reseed-canonicalization-does-not-deadlock-qwen-design.md`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/current-spec-provenance-map.md`
- `crates/vida/src/taskflow_run_graph.rs`
- `crates/vida/src/taskflow_consume.rs`
- `crates/vida/src/taskflow_routing.rs`
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/runtime_lane_summary.rs`
- `crates/vida/src/runtime_assignment_builder.rs` only if activation/backend resolution still needs constraint-level repair

## Fail-Closed Constraints
- Forbidden fallback paths
  - No local root-session implementation for qwen cleanup while delegated route truth remains contradictory.
  - No “fix” that only injects a backend while leaving `dispatch_target` and `handoff_runtime_role` inconsistent.
- Required receipts / proofs / gates
  - Fresh qwen reseed must produce an internally coherent dispatch packet.
  - `agent-init --execute-dispatch` must either execute lawfully or fail on a more specific canonical mismatch.
- Safety boundaries that must remain true during rollout
  - qwen cleanup remains blocked until this owner slice closes.
  - root-session write guard remains blocked by default.

## Implementation Plan

### Phase 1
- Finalize and register this bounded blocker design in the spec canon.
- Reconfirm the exact mismatch between reseeded run status, receipt canonicalization, and packet handoff fields.
- First proof target
  - `cargo run -p vida -- docflow check --root . docs/product/spec/repair-design-backed-reseed-canonicalization-does-not-deadlock-qwen-design.md`

### Phase 2
- Repair either:
  - design-backed reseed selection so qwen remediation stops entering weak conversation routing, or
  - packet/receipt canonicalization so canonical lane targets also pull canonical activation/backend truth.
- Second proof target
  - targeted `cargo test -p vida ...` for reseed/canonicalization/backend regression coverage

### Phase 3
- Re-run live source-tree qwen proofs:
  - `dispatch-init`
  - packet inspection
  - `agent-init --execute-dispatch`
- Rebind continuation back to qwen remediation only after the fresh routed packet is lawful.
- Final proof target
  - live qwen delegated path no longer deadlocks on `specification + pm + null backend`

## Validation / Proof
- Unit tests:
  - design-backed reseed prefers execution path over weak `pbi_discussion` drift
  - canonical dispatch-target selection does not mix conversation activation/runtime role with canonical lane target
  - admissible backend resolution stays non-null for the repaired qwen shape
- Integration tests:
  - explicit qwen blocker reseed from `feature-reconcile-autonomous-execution-flag-runtime-drift`
- Runtime checks:
  - `cargo run -p vida -- taskflow run-graph dispatch-init feature-reconcile-autonomous-execution-flag-runtime-drift --json`
  - `cargo run -p vida -- agent-init --dispatch-packet <fresh-packet> --execute-dispatch --json`
  - `cargo run -p vida -- orchestrator-init --json`
- Canonical checks:
  - `cargo run -p vida -- docflow check --root . docs/product/spec/repair-design-backed-reseed-canonicalization-does-not-deadlock-qwen-design.md docs/product/spec/current-spec-map.md docs/product/spec/current-spec-provenance-map.md`

## Observability
- Logging points
  - reseed override selection
  - canonical dispatch-target derivation
  - activation/backend source selection for canonicalized targets
- Metrics / counters
  - none new expected
- Receipts / runtime state written
  - seeded run-graph status
  - canonical dispatch receipt / packet
  - docflow changelog artifacts

## Rollout Strategy
- Development rollout
  - source-tree fix first, then live source-tree runtime proof
- Validation gate
  - qwen delegated packet must become lawful before returning to qwen cleanup itself
- Rollback / containment
  - if the first repair worsens lawful spec-discussion routing, fail closed and keep the blocker task active

## Open Questions
- Whether the cleanest owner is:
  - widening the existing design-backed implementation override, or
  - teaching packet/receipt canonicalization to stop mixing `default_route` activation with canonical lane targets.
- Whether both adjustments are needed for the qwen shape or one of them alone resolves it.

## Recommendation
- Recommended option
  - Start by writing a focused regression around the exact live qwen packet shape, then repair the narrowest owner that restores one coherent route identity end-to-end.
- Why
  - The current mismatch spans reseed, dispatch-target canonicalization, and backend selection; a reproducer test will prevent another symptom-only fix.

-----
artifact_path: product/spec/repair-design-backed-reseed-canonicalization-does-not-deadlock-qwen-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-21
schema_version: 1
status: canonical
source_path: docs/product/spec/repair-design-backed-reseed-canonicalization-does-not-deadlock-qwen-design.md
created_at: 2026-04-21T18:38:23.279421437Z
updated_at: 2026-04-21T18:40:48.920401999Z
changelog_ref: repair-design-backed-reseed-canonicalization-does-not-deadlock-qwen-design.changelog.jsonl
