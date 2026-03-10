# Process Lane

This directory is the project-owned process lane for active operating documents.

Rules:

1. `docs/process/**` is for project-specific process docs, runbooks, and execution conventions.
2. It must not redefine framework law owned by `vida/config/instructions/**` or `docs/framework/plans/**`.
3. If a process rule becomes stable product law, promote it into `docs/product/spec/**`.
4. If a process rule needs executable enforcement, project it into runtime/config artifacts instead of leaving it as prose only.

Canonical entrypoints:

1. `docs/process/README.md`
   - process lane root
2. `docs/process/documentation-tooling-map.md`
   - project-owned documentation tooling and operator-command map
3. `docs/process/agent-system.md`
   - project-owned agent-system process surface

-----
artifact_path: process/readme
artifact_type: process_doc
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/process/README.md
created_at: 2026-03-10T00:00:00+02:00
updated_at: 2026-03-10T10:05:00+02:00
changelog_ref: README.changelog.jsonl
