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
11. `docs/process/project-orchestrator-startup-bundle.md`
   - compact project-side startup bundle that aggregates the routine orchestrator read set over the current project capsules
12. `docs/process/project-packet-and-lane-runtime-capsule.md`
   - compact runtime-facing projection of project packet and delegated-lane law for routine orchestrator startup
13. `docs/process/project-start-readiness-runtime-capsule.md`
   - compact runtime-facing projection of project startup readiness, including skill activation and boot-readiness gates
14. `docs/process/project-packet-rendering-runtime-capsule.md`
   - compact runtime-facing projection of project packet rendering and prompt-stack interpretation for routine startup and dispatch preparation
15. `docs/process/project-skill-initialization-and-activation-protocol.md`
   - project-owned mandatory rule for inspecting the available skill catalog and activating relevant skills before bounded work begins
16. `docs/process/project-development-packet-template-protocol.md`
   - project-owned canonical packet-template family for session framing, delivery-task packets, execution-block refinement, and coach/verifier/escalation handoffs
17. `docs/process/project-agent-prompt-stack-protocol.md`
   - project-owned prompt-stack model that fixes the precedence between framework bootstrap, project role prompts, dynamic packets, skill overlays, and runtime state
18. `docs/process/project-boot-readiness-validation-protocol.md`
   - project-owned bounded validation sequence that proves a development orchestration session is boot-ready before first dispatch
19. `vida/config/instructions/instruction-contracts/meta.protocol-naming-grammar-protocol.md`
   - canonical framework naming law and sequential rename-wave protocol for instruction artifacts
20. `docs/process/framework-source-lineage-index.md`
   - project-owned provenance index for deleted framework-formation plans/research documents and their promoted canonical homes
21. `docs/process/framework-three-layer-refactoring-audit.md`
   - unified-format consolidated report for the first three refactored framework layers: `core`, orchestration shell, and runtime-family execution
22. `docs/process/release-formatting-protocol.md`
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
