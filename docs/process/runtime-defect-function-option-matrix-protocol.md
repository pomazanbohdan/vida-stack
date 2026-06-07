# Runtime Defect Function And Option Matrix Protocol

Status: canonical

Use this protocol before closing runtime quality work that touches TaskFlow,
DocFlow, lane, run-graph, recovery, agent-init, packet rendering, command output,
or operator JSON surfaces.

## Purpose

Runtime defects must be described as reusable matrix rows, not only as one-off
symptoms. The matrix connects the public command behavior to the owning Rust
functions and proof fixtures so duplicated logic, stale packet truth, missing
CLI options, and output-contract drift are found before final epic closure.

## Required Matrix Columns

Each row must include:

- `invariant`: the behavior that must stay true.
- `command_surface`: every public command that exposes the invariant.
- `option_help_contract`: required options and help/default-mode expectations.
- `default_output_contract`: compact human output, normally TOON/plain.
- `json_output_contract`: machine fields such as `status`, `blocker_codes`,
  `next_actions`, and `artifact_refs`.
- `owning_functions`: Rust functions or shared helpers that compute the truth.
- `fixture_shape`: persisted-state or snapshot shape required to prove it.
- `expected_operator_fields`: blockers, next command, packet, receipt, and task
  identity fields the operator can act on.
- `proof_test`: focused unit/integration/smoke test proving the row.

## Workflow

1. Add or update a matrix row before implementing the fix.
2. Prefer a table-driven test helper for executable rows.
3. Keep docs/process as the durable schema and escalation rule.
4. Fix duplicated logic through a shared helper or contract boundary.
5. Preserve pack/task identity separately from runtime dispatch target.
6. Do not hardcode carriers or roles. Resolve the configured flow.
7. Prove default output and explicit `--json` when the command surface changes.
8. Convert findings outside the bounded row into follow-up TaskFlow tasks.

## Seed Rows

| invariant | command_surface | owning_functions | proof_test |
| --- | --- | --- | --- |
| Spec-first handoff with closed spec and work-pool plus open dev task must dispatch the configured dev-team runtime target, not hardcoded `dev-pack`. | `taskflow consume continue`, `run-graph status`, `dispatch-init` | `spec_first_dev_handoff_gate_satisfied_for_task`, `derive_downstream_dispatch_preview`, `try_bridge_bounded_specification_completion_to_downstream_receipt`, `normalize_spec_first_work_pool_handoff_receipt_truth` | `consume_continue_bridges_closed_spec_and_work_pool_into_dev_progress` |
| Downstream packet-ready agent lanes must have an executable command, top-level `dispatch_target`, canonical target, and packet metadata preserving the source lane id. | `lane complete`, `agent-init --downstream-packet`, downstream packet writer | `resolve_runtime_dispatch_target`, `refresh_downstream_dispatch_preview_with_owned_paths`, `downstream_dispatch_packet_body_with_owned_paths` | `refresh_downstream_dispatch_preview_canonicalizes_lane_id_and_packet_command`, `downstream_packet_canonicalizes_lane_id_to_top_level_dispatch_target` |

## Closure Gate

A runtime quality task using this protocol is not closure-ready until:

- every in-scope row has focused proof;
- any failing old test that encoded stale behavior is updated or removed;
- command suggestions prefer default commands and reserve `--json` for machine
  workflows;
- follow-up rows are represented by TaskFlow tasks;
- `vida task validate-graph --json` passes.

-----
artifact_path: process/runtime-defect-function-option-matrix-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-06-07'
schema_version: '1'
status: canonical
source_path: docs/process/runtime-defect-function-option-matrix-protocol.md
created_at: 2026-06-07T00:00:00+03:00
updated_at: 2026-06-07T00:00:00+03:00
changelog_ref: runtime-defect-function-option-matrix-protocol.changelog.jsonl
