# Documentation Tooling Map

Purpose: expose the project-owned documentation tooling surface without mixing the active project docs map with a large operator command catalog.

## Scope

This map covers project-facing documentation tooling for the active `vida-stack` repository:

1. documentation inventory/status commands,
2. documentation mutation/finalization commands,
3. validation/proof commands,
4. routing from project-doc work into the bounded `codex` runtime family.

## Canonical Entry Points

1. `AGENTS.sidecar.md`
   - project docs bootstrap map that points here for documentation-tooling tasks
2. `vida/config/instructions/system-maps.runtime-family-codex.md`
   - runtime-family map for the bounded documentation/operator runtime
3. `vida/config/instructions/instruction-contracts.documentation-operation-protocol.md`
   - framework law for documentation-shaped tasks

## Read / Status Commands

1. `overview [--profile <name>]`
2. `layer-status --layer <N> [--format toon|jsonl]`
3. `doctor --layer <N> [--format toon|jsonl]`
4. `proofcheck --layer <N> [--format toon|jsonl]`
5. `summary --root <dir> [--format toon|jsonl]`
6. `registry --root <dir>`
7. `registry-write --root <dir> [--output <file>] [--canonical]`
8. `scan --root <dir> [--missing-only]`
9. `changelog <file> [--limit N] [--newest-first] [--format toon|jsonl]`
10. `changelog-task --root <dir> <task_id> [--limit N] [--newest-first] [--format toon|jsonl]`
11. `task-summary --root <dir> --task-id <id> [--format toon|jsonl]`
12. `deps <file> [--format toon|jsonl]`
13. `deps-map <file-or-dir> [--format toon|jsonl]`
14. `artifact-impact [--file <file> | --artifact <artifact_path>] [--format toon|jsonl]`
15. `task-impact --root <dir> --task-id <id> [--format toon|jsonl]`
16. `links <file-or-dir> [--format toon|jsonl]`

## Mutation / Finalization Commands

1. `touch <file> <note> [--event ...] [--task-id ...] [--actor ...] [--scope ...] [--tags ...]`
2. `finalize-edit <file> [<file> ...] <note> [--status ...] [--artifact-version ...] [--artifact-revision ...] [--set key=value ...]`
3. `init <file> <artifact_path> <artifact_type> <note> [...]`
4. `move <file> <destination> <note> [...]`
5. `rename-artifact <file> <artifact_path> <note> [...]`
6. `migrate-links <file-or-dir> <old_target> <new_target> <note> [--dry-run] [--format toon|jsonl] [...]`

## Validation Commands

1. `check --root <dir> [files...]`
2. `fastcheck --root <dir> [files...]`
3. `activation-check --root <dir> [files...]`
4. `protocol-coverage-check --profile active-canon [--format toon|jsonl]`
5. `readiness-check --profile active-canon [--format toon|jsonl]`
6. `readiness-write --profile active-canon [--canonical]`
7. `proofcheck --profile active-canon-strict [files...] [--layer <N>] [--format toon|jsonl]`
8. `doctor --root <dir> [--show-warnings] [--format toon|jsonl] [--fail-on-warnings] [--layer <N>]`

## Activation Triggers

Read this map when:

1. the task is documentation-shaped and needs concrete operator commands,
2. the task asks how to run documentation inventory, proof, or migration tooling,
3. project-doc work must stay inside the active `vida-stack` docs surface while using framework-owned documentation law.

## Operational Rule

1. perform diff edits first and run exactly one `finalize-edit` afterward for one logical document edit batch,
2. prefer one changelog entry per logical edit batch, not one entry per low-level diff step,
3. history and human-facing map commands default to `toon`; use `--format jsonl` when machine-readable output is needed,
4. machine-oriented plumbing commands such as `scan`, `registry`, `registry-write`, and `check` stay `jsonl`-first,
5. use `fastcheck` during active editing for quick bounded safety and `doctor` before closure of canonical changes,
6. use `activation-check` whenever protocol activation wiring changed,
7. use `protocol-coverage-check` whenever canonical protocol rows, protocol-bearing artifacts, or activation-index wiring changed,
8. use `readiness-check` whenever readiness law, projection parity, canonical bundle shape, compatibility classes, or boot-gate artifacts changed,
9. use `readiness-write --canonical` when downstream automation or runtime proving needs the current shared readiness artifact.

## Boundary Rule

1. This file is project-owned tooling guidance.
2. Framework documentation law remains in `vida/config/instructions/**`.
3. Runtime execution for these commands remains the bounded `codex` runtime family surface, not this map itself.

-----
artifact_path: process/documentation-tooling-map
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: docs/process/documentation-tooling-map.md
created_at: '2026-03-10T09:45:00+02:00'
updated_at: '2026-03-10T09:45:00+02:00'
changelog_ref: documentation-tooling-map.changelog.jsonl
