# Process Lane

Purpose: provide the canonical root entrypoint for the active project process lane and keep process-facing documents discoverable without turning process docs into product law.

This directory is the project-owned process lane for active operating documents.

Rules:

1. `docs/process/**` is for project-specific process docs, runbooks, and execution conventions.
2. It must not redefine framework law owned by `vida/config/instructions/**`, `docs/product/spec/**`, or executable law under `vida/config/**`.
3. If a process rule becomes stable product law, promote it into `docs/product/spec/**`.
4. If a process rule needs executable enforcement, project it into runtime/config artifacts instead of leaving it as prose only.

Canonical entrypoints:

1. `docs/process/README.md`
   - process lane root
2. `docs/process/documentation-tooling-map.md`
   - project-owned documentation tooling and operator-command map
3. `docs/process/vida1-development-conditions.md`
   - proven local development, build, install, and launcher conditions for active `VIDA 1`
4. `docs/process/agent-system-guide.md`
   - project-owned agent-system process surface
5. `docs/process/agent-extensions/README.md`
   - project-owned role/skill/profile/flow extension map
6. `docs/process/codex-agent-configuration-guide.md`
   - project-owned guide for local OpenAI Codex multi-agent configuration and development-team mapping
7. `docs/process/team-development-and-orchestration-protocol.md`
   - project-owned protocol for manager-led delivery-task decomposition, delegated lane packets, and development-team closure routing
8. `docs/process/project-orchestrator-operating-protocol.md`
   - project-owned top-level operating protocol for a cheaper but logical orchestrator, including default decomposition depth, delegation defaults, and escalation rules
9. `docs/process/project-orchestrator-session-start-protocol.md`
   - project-owned repeatable start checklist for development orchestrator sessions
10. `docs/process/project-orchestrator-reusable-prompt.md`
   - project-owned reusable root-session prompt for repeated orchestrator development sessions
11. `docs/process/project-skill-initialization-and-activation-protocol.md`
   - project-owned mandatory rule for inspecting the available skill catalog and activating relevant skills before bounded work begins
12. `vida/config/instructions/instruction-contracts/meta.protocol-naming-grammar-protocol.md`
   - canonical framework naming law and sequential rename-wave protocol for instruction artifacts
13. `docs/process/framework-source-lineage-index.md`
   - project-owned provenance index for deleted framework-formation plans/research documents and their promoted canonical homes
14. `docs/process/framework-three-layer-refactoring-audit.md`
   - unified-format consolidated report for the first three refactored framework layers: `core`, orchestration shell, and runtime-family execution
15. `docs/process/release-formatting-protocol.md`
   - canonical project process for rendering clean public GitHub release pages from canonical release-note artifacts

-----
artifact_path: process/readme
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/process/README.md
created_at: '2026-03-10T00:00:00+02:00'
updated_at: '2026-03-13T19:11:00+02:00'
changelog_ref: README.changelog.jsonl
