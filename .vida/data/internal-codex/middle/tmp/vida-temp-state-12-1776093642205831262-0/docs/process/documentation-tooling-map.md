# Documentation Tooling Map

Use `vida docflow` for documentation inventory, mutation, validation, and readiness checks.

Design-document rule:

1. For bounded feature/change work that requires research, detailed specifications, planning, and implementation, begin with one design document before code execution.
2. Start from `docs/product/spec/templates/feature-design-document.template.md`.
3. Open one epic and one spec-pack task in `vida taskflow` before writing code.
4. Suggested command sequence:
- `vida docflow init docs/product/spec/<feature>-design.md product/spec/<feature>-design product_spec "initialize feature design"`
- edit the document using the local template shape
- `vida docflow finalize-edit docs/product/spec/<feature>-design.md "record bounded feature design"`
- `vida docflow check --root . docs/product/spec/<feature>-design.md`
- `vida task close <spec-task-id> --reason "design packet finalized and handed off" --json`

Activation rule:

1. During project activation, `vida project-activator` owns bounded config/doc materialization.
2. `vida taskflow` and any non-canonical external TaskFlow runtime are not lawful activation-entry surfaces while activation is pending.
3. After activation writes, prefer `vida docflow` for documentation-oriented inspection and proof before multi-step implementation.

-----
artifact_path: process/documentation-tooling-map
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-04-04'
schema_version: '1'
status: scaffold
source_path: docs/process/documentation-tooling-map.md
created_at: '2026-04-04T00:00:00Z'
updated_at: '2026-04-04T00:00:00Z'
changelog_ref: documentation-tooling-map.changelog.jsonl
