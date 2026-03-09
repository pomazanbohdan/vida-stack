# Spec Delta Protocol (SDP)

Purpose: normalize non-equivalent issue, release, or research-driven behavior changes into an explicit spec-delta artifact before task formation or writer execution continues.

Scope:

1. Canonical reconciliation path when `issue_contract.status=spec_delta_required`.
2. Canonical reconciliation path when `spec_intake.status=needs_spec_delta`.
3. Bridges `vida/config/instructions/runtime-instructions.spec-intake-protocol.md`, `vida/config/instructions/runtime-instructions.spec-contract-protocol.md`, `vida/config/instructions/command-instructions.form-task-protocol.md`, and `vida/config/instructions/runtime-instructions.issue-contract-protocol.md`.

## Core Principle

Do not handle non-equivalent behavior change as if it were an equivalent bugfix.

Materialize the contract delta first, then reconcile scope and launch.

## Canonical Artifact

1. `.vida/logs/spec-deltas/<task_id>.json`

Minimum fields:

1. `task_id`
2. `delta_source`
3. `trigger_status`
4. `current_contract`
5. `proposed_contract`
6. `delta_summary`
7. `behavior_change`
8. `scope_impact`
9. `user_confirmation_required`
10. `reconciliation_targets`
11. `status`

## Trigger Sources

Normalize each delta source into one of:

1. `issue_contract`
2. `spec_intake`
3. `release_signal`
4. `research_findings`
5. `coach_reopen`

## Status Mapping

`spec_delta.status` must normalize to one of:

1. `delta_ready`
   - the delta is explicit enough to route into SCP reconciliation.
2. `needs_user_confirmation`
   - the contract delta is understood but still needs explicit user decision.
3. `needs_scp_reconciliation`
   - route back into SCP before task formation or launch.
4. `not_required`
   - no material contract delta remains.
5. `insufficient_delta`
   - the signal is not explicit enough to reconcile yet.

## Routing Rule

1. `delta_ready` -> `vida/config/instructions/runtime-instructions.spec-contract-protocol.md`
2. `needs_user_confirmation` -> stay in clarification flow until the delta decision is explicit
3. `needs_scp_reconciliation` -> block writer launch and return to SCP
4. `not_required` -> continue with normal equivalent flow
5. `insufficient_delta` -> gather more evidence before reconciliation

## Normalization Rules

1. `current_contract` must describe the currently accepted behavior/expectation.
2. `proposed_contract` must describe the changed behavior being considered.
3. `delta_summary` must be one bounded statement of what changes.
4. `behavior_change` must distinguish user-visible or API-visible change from internal-only refactor.
5. `scope_impact` must list affected modules, flows, or acceptance areas.
6. `reconciliation_targets` must identify which spec/task surfaces require update before launch.
7. `user_confirmation_required=yes` is mandatory for material UX/product/API behavior changes.

## Writer Gate Rule

1. If `spec_delta.status` is `delta_ready`, `needs_user_confirmation`, or `needs_scp_reconciliation`, writer execution remains blocked until the reconciliation path closes the delta explicitly.
2. `not_required` is the only status that clears the delta gate.

## Commands

```bash
python3 docs/framework/history/_vida-source/scripts/spec-delta.py write <task_id> <input.json> [--output PATH]
python3 docs/framework/history/_vida-source/scripts/spec-delta.py validate <task_id> [--path PATH]
python3 docs/framework/history/_vida-source/scripts/spec-delta.py status <task_id> [--path PATH]
```

-----
artifact_path: config/runtime-instructions/spec-delta.protocol
artifact_type: runtime_instruction
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/runtime-instructions.spec-delta-protocol.md
created_at: 2026-03-08T02:15:22+02:00
updated_at: 2026-03-10T00:55:00+02:00
changelog_ref: runtime-instructions.spec-delta-protocol.changelog.jsonl
