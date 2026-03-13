# Core Orchestration Runtime Capsule

Purpose: provide a compact runtime-facing projection of the highest-frequency orchestration law for active execution.

Boundary rule:

1. this file is a compact projection, not the owner of orchestration law,
2. the canonical owner remains `vida/config/instructions/instruction-contracts/core.orchestration-protocol.md`,
3. use the owner file when an edge case, conflict, or uncommon routing condition is not resolved by this capsule.

## Always-Keep-Visible

1. active bounded unit,
2. lawful next slices,
3. current route posture,
4. current proof target,
5. whether `in_work` remains 1,
6. whether delegated cycle / handoff / blocker state is open.

## Control Loop

1. identify active bounded unit,
2. identify lawful next slices,
3. remove blocked/gated slices,
4. choose sequential vs parallel-safe route,
5. apply priority law if more than one candidate remains,
6. shape the next bounded packet,
7. dispatch or continue,
8. after worker return, timeout, discovery, or bounded closure, rerun the loop before any pause-like report.

## High-Frequency Laws

1. bind execution to one explicit bounded unit before write-producing work,
2. do not silently map ambiguous wording like `next task` to `ready_head[0]`,
3. commentary, summary, timeout, dispatch, or one bounded success do not create a natural stop while `in_work=1`,
4. after any bounded closure, rebuild the parent bounded unit and classify:
   - `next_leaf_required`
   - `blocked`
   - `fully_closed`
5. if the next lawful item is already known, continue routing instead of stopping on a report,
6. `final` is invalid while any delegated agent/handoff remains active or unresolved,
7. local write work requires explicit exception-path receipt,
8. exception-path receipt is not sufficient while an open delegated cycle for the same packet remains active.

## Escalate To Owner File When

1. path selection between diagnosis and normal delivery is unclear,
2. delegated-cycle takeover legality is disputed,
3. partial-return / rework / review-repair law is in play,
4. active-unit binding is ambiguous,
5. reporting boundary or final-report legality is not obvious,
6. a framework/protocol mutation may change routing law itself.

-----
artifact_path: config/instructions/instruction-contracts/core.orchestration-runtime-capsule
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/core.orchestration-runtime-capsule.md
created_at: '2026-03-13T21:50:00+02:00'
updated_at: '2026-03-13T21:50:00+02:00'
changelog_ref: core.orchestration-runtime-capsule.changelog.jsonl
