# vida/config/instructions/ — Canonical VIDA Instruction Surface

This directory is the canonical source for active framework instruction artifacts.

Use `vida/config/instructions/` for:

1. Boot, routing, reasoning, command, TaskFlow, and task-state instruction artifacts.
2. Runtime topology maps and instruction-facing system maps.
3. Framework-owned worker/orchestrator entry, contracts, prompts, and automatic-management rules.
4. Flat latest-revision Markdown canon plus machine-readable YAML projections.

Do not use `vida/config/instructions/` for:

1. Product architecture or feature specifications.
2. Historical research or evidence copies.
3. Project-specific build, release, or observability runbooks.
4. App-specific commands whose executable entrypoints live in `scripts/`.

Canonical split:

1. `vida/config/instructions/` -> active framework instruction canon.
2. `docs/product/spec/` -> promoted stable product/spec canon.
3. `docs/process/framework-source-lineage-index.md` -> project-owned provenance index for deleted framework-formation sources.
4. sidecar changelogs plus Git history -> historical evidence only.
5. `docs/process/` -> canonical project operational runbooks.
6. `scripts/` -> executable project operations referenced by `docs/process/`.

Reasoning docs:

1. Canonical deep spec: `instruction-contracts/overlay.step-thinking-protocol`
2. One-screen reference: `references/algorithms.one-screen-reference`
3. Operational quick reference: `references/algorithms.quick-reference`

Migration policy:

1. New active framework instruction docs belong in flat filenames under `vida/config/instructions/`.
2. Settled framework/product law belongs in `docs/product/spec/` or `vida/config/**`, not in revived plan/research trees.
3. Historical source lineage is recorded in `docs/process/framework-source-lineage-index.md`.
4. New project docs and build/ops runbooks belong in `docs/` or `docs/process/`.
5. New executable project workflows belong in `scripts/`, not `scripts/`.

-----
artifact_path: config/system-maps/framework.readme
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/framework.readme.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-11T13:40:28+02:00'
changelog_ref: framework.readme.changelog.jsonl
