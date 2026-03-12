# Development Evidence Sync Protocol

Purpose: require that after each successful bounded implementation step, the active project documentation is updated with the newly proven development conditions, run commands, build commands, install commands, launcher conditions, or other execution surfaces that now work.

## Activation

Activate this protocol when all are true:

1. implementation or runtime work is active,
2. a bounded step has just succeeded with real local evidence,
3. the success changes what a future developer/operator can now run, build, install, or verify.

This protocol is not optional in autonomous implementation mode.

## Hard Rule

After each successful bounded step, the agent must update the project-owned development-conditions document before treating the step as fully closed.

Required update content:

1. the exact successful command or command shape,
2. the newly working artifact, runtime surface, or install surface,
3. any still-transitional limits or donor-backed behavior,
4. any required environment variables, launch paths, or build roots.

Do not defer this update to the end of a large wave if the successful condition is already proven now.

## Canonical Target

For the active `vida-stack` project, the canonical target is:

1. `docs/process/vida1-development-conditions.md`

If a future project overlay declares a different project operations/development-conditions target, follow the overlay without weakening this rule.

## Evidence Standard

Only record conditions backed by real success evidence.

Accepted evidence order:

1. successful command execution in the current repository/runtime,
2. successful build/test/install artifact generated locally,
3. successful bounded validation command such as `check`, `fastcheck`, `proofcheck`, or runtime smoke.

Forbidden:

1. aspirational commands,
2. untested install/run/build claims,
3. copying old assumptions forward without rerunning them after the relevant change.

## Minimum Update Triggers

Update the development-conditions document after success in any of these classes:

1. workspace build/test turns green,
2. release build or packaging path succeeds,
3. installer path succeeds,
4. launcher or CLI entrypoint becomes runnable,
5. runtime-family delegation surface starts working,
6. new environment variable or path requirement becomes known,
7. a previously failing local setup step becomes green.

## Relationship To Other Protocols

1. `vida/config/instructions/runtime-instructions/bridge.spec-sync-protocol.md` governs framework/spec synchronization after autonomous changes.
2. This protocol governs project-owned operational development evidence after successful implementation steps.
3. `vida/config/instructions/instruction-contracts/work.documentation-operation-protocol.md` still governs how canonical documentation mutations are performed lawfully.

## Closure Rule

A bounded implementation step that changed project run/build/install conditions is not fully closed until:

1. the successful step was recorded in the canonical project development-conditions document,
2. the documentation mutation was validated through the lawful documentation tooling path,
3. the updated document reflects the current proven state rather than the previous state.

-----
artifact_path: config/runtime-instructions/development-evidence-sync.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.development-evidence-sync-protocol.md
created_at: '2026-03-11T09:20:00+02:00'
updated_at: '2026-03-12T11:44:42+02:00'
changelog_ref: work.development-evidence-sync-protocol.changelog.jsonl
