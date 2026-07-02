# Architecture Decision: Team-Flow State-Machine Ownership Boundary

## ID
`ADR-team-flow-state-machine-owner-20260701`

## Status
PROPOSED — requires implementation before closure.

## Date
2026-07-01

## Context

The VIDA runtime contains hardcoded literals for team-flow continuation logic scattered across multiple crates:

- `runtime_dispatch*` — downstream packet generation, lane completion verdicts
- `taskflow_routing*` — dispatch contract lane sequence extraction
- `taskflow_consume_resume*` — resume target computation
- `taskflow_packet*` — packet shaping and handoff
- `state_store*` — persisted state projections

These literals include:
- `allowed_next_node` values (e.g., `"developer"`, `"developer_rework"`, `"tester"`, `"closure"`)
- `lane_sequence` arrays (e.g., `["analyst", "designer", "autotester"]`)
- `execution_lane_sequence` arrays (e.g., `["implementer", "coach", "verification"]`)

Each crate independently interprets these literals, leading to:
1. Duplicate logic for the same state-machine transitions
2. Inconsistent handling when new roles/lane sequences are added
3. No single source of truth for what constitutes a lawful next-node transition
4. Hard-to-maintain test fixtures that encode specific literal values

## Decision

**One canonical state-machine owner task must replace all scattered hardcoded literals with a config-backed shared boundary.**

### Shared Boundary Design

1. **Config-backed lane sequence resolution:**
   - All `lane_sequence` and `execution_lane_sequence` lookups must resolve from `vida.config.yaml -> dev_team.flows.<flow_id>.steps[]` rather than parsing JSON dispatch contracts directly.
   - The shared boundary function: `resolve_execution_lane_sequence(flow_config, current_role) -> Vec<String>`

2. **Allowed-next-node validation:**
   - All `allowed_next_node` checks must use a single authority function: `validate_allowed_next_node(current_role, requested_next_node, execution_plan) -> Verdict`
   - This function reads from the same config-backed lane sequence source.

3. **Role-to-lane mapping:**
   - Runtime roles (`worker`, `business_analyst`, `coach`, `verifier`, `prover`, `solution_architect`) map to concrete lane IDs through a single registry in `vida.config.yaml -> dev_team.roles.<role_id>.runtime_role`.
   - No crate may hardcode its own role-to-lane mapping.

4. **State-machine transitions:**
   - The state machine for team-flow continuation (analyst → test_author → developer → coach → verifier → prover) is defined once in `vida.config.yaml -> dev_team.flows.task_delivery_verified.steps[]`.
   - All crates that need to know "what comes next" must query this canonical definition.

### Implementation Scope

**Files to refactor:**
- `crates/vida/src/runtime_dispatch_downstream_packets.rs`
- `crates/vida/src/runtime_dispatch_lane_completion.rs`
- `crates/vida/src/runtime_dispatch_execution.rs`
- `crates/vida/src/runtime_dispatch_result_evidence.rs`
- `crates/vida/src/runtime_dispatch_status.rs`
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/taskflow_routing.rs`
- `crates/vida/src/taskflow_consume_resume.rs`
- `crates/vida/src/taskflow_packet*` (if exists)
- `crates/vida/src/state_store*.rs`

**New shared module:**
- `crates/vida/src/team_flow_state_machine.rs` — canonical state-machine owner with:
  - `resolve_next_lane(current_role, execution_plan) -> Option<String>`
  - `validate_transition(current_role, requested_next_node) -> Verdict`
  - `get_execution_sequence(flow_config) -> Vec<String>`

### Non-Goals

1. This ADR does not change the dev_team flow definitions in `vida.config.yaml`.
2. This ADR does not modify the packet template schema.
3. This ADR does not change the agent dispatch protocol or host-bridge contract.

### Consequences

**Positive:**
- Single source of truth for team-flow state-machine transitions
- Easier to add new roles/lane sequences (one config change)
- Reduced test duplication
- Clear ownership for state-machine behavior

**Negative:**
- Requires refactoring ~10 files across multiple crates
- Tests that encode specific literal values must be updated
- Temporary risk of regression during migration

### Migration Plan

1. Create `team_flow_state_machine.rs` with the shared boundary functions.
2. Update `taskflow_routing.rs` to use the new shared boundary for lane sequence resolution.
3. Update `runtime_dispatch*` files to use the new shared boundary for allowed_next_node validation.
4. Update `taskflow_consume_resume.rs` to use the new shared boundary for resume target computation.
5. Update all test fixtures to use config-backed values instead of hardcoded literals.
6. Run full test suite and smoke tests.

### Verification

1. `cargo test --all` passes with no regressions.
2. `vida taskflow run-graph dispatch-init <task-id> --json` returns correct lane sequences from config.
3. `vida agent-init --dispatch-packet ... --execute-dispatch` follows the new state-machine boundaries.
4. DocFlow proofcheck passes for all affected process docs.

### Related Tasks

- TaskFlow task: `runtime-arch-team-flow-state-machine-owner-20260701`
- Parent epic: `runtime-team-flow-activity-meeting-dx-20260701`

-----
artifact_path: product/spec/adr-team-flow-state-machine-owner
artifact_type: architecture_decision
artifact_version: '1'
artifact_revision: '2026-07-01'
schema_version: '1'
status: proposed
source_path: docs/product/spec/adr-team-flow-state-machine-owner.md
created_at: '2026-07-01T00:00:00+03:00'
updated_at: '2026-07-02T04:18:00+03:00'
changelog_ref: adr-team-flow-state-machine-owner.changelog.jsonl
