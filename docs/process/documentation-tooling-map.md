# Documentation Tooling Map

Purpose: expose the project-owned documentation tooling surface without mixing the active project docs map with a large operator command catalog.

## Scope

This map covers project-facing documentation tooling for the active `vida-stack` repository:

1. documentation inventory/status commands,
2. documentation mutation/finalization commands,
3. validation/proof commands,
4. routing from project-doc work into the bounded `DocFlow` runtime family.

## Canonical Entry Points

1. `AGENTS.sidecar.md`
   - project docs bootstrap map that points here for documentation-tooling tasks
2. `system-maps/runtime-family.docflow-map.md`
   - runtime-family map for the bounded documentation/operator runtime
3. `instruction-contracts/work.documentation-operation-protocol.md`
   - framework law for documentation-shaped tasks
4. `docs/product/spec/feature-design-and-adr-model.md`
   - owner model for structured feature/change design documents and linked ADR usage
5. `docs/framework/templates/feature-design-document.template.md`
   - framework-owned reusable starting shape for one bounded feature/change design
6. `vida/config/docflow/docsys_policy.yaml`
   - canonical DocFlow scan-ignore/profile policy consumed by the active Rust DocFlow runtime

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
3. `check-file --path <file>`
4. `report-check --path <file>`
5. `activation-check --root <dir> [files...]`
6. `protocol-coverage-check --profile active-canon [--format toon|jsonl]`
7. `readiness-check --profile active-canon [--format toon|jsonl]`
8. `readiness-write --profile active-canon [--canonical]`
9. `proofcheck --profile active-canon-strict [files...] [--layer <N>] [--format toon|jsonl]`
10. `doctor --root <dir> [--show-warnings] [--format toon|jsonl] [--fail-on-warnings] [--layer <N>]`

## Activation Triggers

Read this map when:

1. the task is documentation-shaped and needs concrete operator commands,
2. the task asks how to run documentation inventory, proof, or migration tooling,
3. project-doc work must stay inside the active `vida-stack` docs surface while using framework-owned documentation law,
4. a bounded feature/change design document or linked ADR is being authored, updated, or reviewed.

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
10. canonical generated DocFlow artifacts live at:
    - `vida/config/docflow-registry.current.jsonl`
    - `vida/config/docflow-readiness.current.jsonl`
11. root-scoped inventory commands inherit the canonical ignore policy from `vida/config/docflow/docsys_policy.yaml`; do not compensate for `_temp/`, `dist/**`, or other ignored trees with manual post-filtering.
12. when authoring a bounded feature/change design doc, start from `docs/framework/templates/feature-design-document.template.md` and keep the heading order stable instead of improvising a new document shape,
13. for design-first feature delivery, open one feature epic and one spec-pack task in `vida taskflow` before implementation, keep the design artifact canonical through `vida docflow`, and close the spec-pack task only after the design artifact is finalized and validated.
14. when one decision inside that design needs durable standalone recording, link a separate ADR rather than collapsing the whole change into one narrative document.
15. use `vida taskflow bootstrap-spec "<request>" --json` as the preferred one-shot launcher surface when a design-first feature request must materialize the initial feature epic, spec-pack task, and canonical design-doc scaffold in one pass.
16. `vida docflow check-file`, `check`, `fastcheck`, and `readiness-check` now fail closed when a canonical project-visible doc is missing its required owning-map registration or when `AGENTS.sidecar.md` omits required bootstrap-visible documentation pointers.
17. use `vida docflow report-check --path <file>` when the bounded proof target is the required runtime reporting prefix shape (`Thinking mode`, `Requests|Tasks`, `Agents`, `Reasoning summary`) rather than markdown footer law.
18. when a documentation change creates or reroutes a canonical project-visible document surface, update the owning map/index and, when bootstrap-visible topology changed, update `docs/project-root-map.md` and `AGENTS.sidecar.md` in the same bounded change.

## Boundary Rule

1. This file is project-owned tooling guidance.
2. Framework documentation law remains in `vida/config/instructions/**`.
3. Runtime execution for these commands remains the bounded `DocFlow` runtime family surface, surfaced through `vida docflow`, not this map itself.

-----
artifact_path: process/documentation-tooling-map
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-14'
schema_version: '1'
status: canonical
source_path: docs/process/documentation-tooling-map.md
created_at: '2026-03-10T09:45:00+02:00'
updated_at: 2026-03-14T12:41:58.830287015Z
changelog_ref: documentation-tooling-map.changelog.jsonl
