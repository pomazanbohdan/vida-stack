# Spec Sync Protocol

Purpose: require autonomous development to keep nearby governing specs and protocols in sync with framework or product behavior changes.

## Core Contract

1. When autonomous work changes framework or product behavior, the agent must check for nearby canonical specs/protocols governing that behavior.
2. If a nearby canonical spec/protocol exists and is now stale, the agent must update it in the matching ownership layer before considering the work closure-ready.
3. If no such spec/protocol exists and the behavior is law-bearing, the agent must add one.
4. For planning/task artifacts, prefer updating an existing nearby task/spec before creating a new one.
5. If a spec change introduces a new executable requirement, the same slice must also update task coverage for that requirement.
6. The same update-first rule applies when autonomous next-task boundary analysis discovers stale dependent specs/tasks before the next task starts.

## Task Coverage Rule

1. First check whether an existing epic/task/plan item already covers the new requirement.
2. If coverage exists, update the existing task rather than creating a duplicate.
3. If no coverage exists, create a new task in the correct plan/epic location.
4. Do not leave new executable spec requirements without explicit backlog ownership.

## Trigger Cases

1. new runtime behavior,
2. new migration/version behavior,
3. new storage ownership or export behavior,
4. new instruction activation or composition behavior,
5. new autonomous execution or prioritization behavior,
6. new operator-visible command or output contract.
7. new next-task boundary analysis/report/update behavior.

## Minimum Check

1. search for the nearest canonical protocol/spec,
2. decide whether the change is already covered,
3. update the existing artifact when coverage already exists but is stale or incomplete,
4. add a new artifact only when coverage is genuinely missing,
5. update or create the corresponding task when the spec delta creates executable scope,
6. record the spec touch in tracked execution,
7. when the delta is discovered during autonomous next-task boundary analysis, finish the coverage refresh before the next task enters implementation.

## Fail-Closed Rule

1. Do not leave law-bearing behavior implemented only in code or chat narration.
2. Do not close autonomous framework work if nearby canonical specs are stale and the mismatch is material.

-----
artifact_path: config/runtime-instructions/spec-sync.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/bridge.spec-sync-protocol.md
created_at: '2026-03-09T12:00:46+02:00'
updated_at: '2026-03-11T13:04:10+02:00'
changelog_ref: bridge.spec-sync-protocol.changelog.jsonl
