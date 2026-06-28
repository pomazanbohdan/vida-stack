# Runtime Library FSM Pilot Decision

Status: accepted for isolated pilot only
Date: 2026-06-21

## Decision

Adopt `rust-fsm` `0.8.0` only inside `taskflow-core` for the isolated consume/resume lifecycle pilot.

The pilot is limited to `consume::resume_state_machine` and models local resume lifecycle movement:

- `Idle -> Dispatching`
- `Dispatching -> Resumed`
- `Dispatching -> Blocked`
- `Blocked -> Dispatching`

## Rationale

`rust-fsm` provides a small declarative transition DSL and direct `StateMachine::from_state` support, so the pilot can evaluate generated transition boilerplate without taking authority over TaskFlow state law, run-graph scheduling, or runtime DB mutation.

`statig` is better suited to hierarchical state machines and is too broad for this pilot.

`smlang` is a viable macro DSL, but it does not improve this slice over `rust-fsm` enough to justify a second macro style.

## Boundary

This decision does not authorize replacing TaskFlow authority, run-graph scheduling, closure law, or state-store transitions with an external FSM crate.

Further adoption requires a separate task with public-surface parity proof.

-----
artifact_path: product/spec/runtime-library-fsm-pilot-decision
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-06-21'
schema_version: '1'
status: canonical
source_path: docs/product/spec/runtime-library-fsm-pilot-decision.md
created_at: '2026-06-21T00:00:00+03:00'
updated_at: '2026-06-28T01:16:18Z'
changelog_ref: runtime-library-fsm-pilot-decision.changelog.jsonl
