# Process Lane

Purpose: provide the canonical root entrypoint for the active project process lane and keep process-facing documents discoverable without turning process docs into product law.

This directory is the project-owned process lane for active operating documents.

Rules:

1. `docs/process/**` is for project-specific process docs, runbooks, and execution conventions.
2. It must not redefine framework law owned by `vida/config/instructions/**`, `docs/product/spec/**`, or executable law under `vida/config/**`.
3. If a process rule becomes stable product law, promote it into `docs/product/spec/**`.
4. If a process rule needs executable enforcement, project it into runtime/config artifacts instead of leaving it as prose only.
5. `README.md` is reserved for the repository root; process lane orientation lives in this `index.md`.

Canonical entrypoints:

1. `docs/process/index.md`
   - process lane root
2. `docs/process/documentation-tooling-map.md`
   - project-owned documentation tooling and operator-command map
3. `docs/process/vida1-development-conditions.md`
   - proven local development, build, install, and launcher conditions for active `VIDA 1`
4. `docs/process/agent-system.md`
   - current canonical process surface for host-system selection, carrier ownership, and agent-first execution posture
5. `docs/process/agent-extensions/index.md`
   - project-owned role/skill/profile/flow extension map
6. `docs/process/codex-agent-configuration-guide.md`
   - project-owned guide for local OpenAI Codex multi-agent configuration and development-team mapping
7. `docs/process/decisions.md`
   - activation-time project decisions including the currently selected host CLI execution posture
8. `docs/process/team-development-and-orchestration-protocol.md`
   - project-owned protocol for manager-led delivery-task decomposition, delegated lane packets, and development-team closure routing
9. `docs/process/project-orchestrator-operating-protocol.md`
   - project-owned top-level operating protocol for a cheaper but logical orchestrator, including default decomposition depth, delegation defaults, and escalation rules
10. `docs/process/project-orchestrator-session-start-protocol.md`
   - project-owned repeatable start checklist for development orchestrator sessions
11. `docs/process/project-orchestrator-reusable-prompt.md`
   - project-owned reusable root-session prompt for repeated orchestrator development sessions
12. `docs/process/project-orchestrator-startup-bundle.md`
   - compact project-side startup bundle that aggregates the routine orchestrator read set over the current project capsules
13. `docs/process/project-packet-and-lane-runtime-capsule.md`
   - compact runtime-facing projection of project packet and delegated-lane law for routine orchestrator startup
14. `docs/process/project-start-readiness-runtime-capsule.md`
   - compatibility projection that keeps the runtime-facing startup-readiness path stable while routing owner law to the startup bundle, session-start protocol, and skill-initialization protocol
15. `docs/process/project-packet-rendering-runtime-capsule.md`
   - compact runtime-facing projection of project packet rendering and prompt-stack interpretation for routine startup and dispatch preparation
16. `docs/process/project-skill-initialization-and-activation-protocol.md`
   - project-owned mandatory rule for inspecting the available skill catalog and activating relevant skills before bounded work begins
17. `docs/process/project-development-packet-template-protocol.md`
   - project-owned canonical packet-template family for session framing, delivery-task packets, execution-block refinement, and coach/verifier/escalation handoffs
18. `docs/process/project-agent-prompt-stack-protocol.md`
   - project-owned prompt-stack model that fixes the precedence between framework bootstrap, project role prompts, dynamic packets, skill overlays, and runtime state
19. `docs/process/project-operations.md`
   - current canonical process surface for feature-delivery flow, delegated execution posture, and launcher-owned progression commands
20. `docs/process/environments.md`
   - current canonical process surface for local environment assumptions, long-lived state roots, and temp-state proof posture
21. `instruction-contracts/meta.protocol-naming-grammar-protocol.md`
   - canonical framework naming law and sequential rename-wave protocol for instruction artifacts
22. `docs/process/release-formatting-protocol.md`
   - canonical project process for rendering clean public GitHub release pages from canonical release-note artifacts
23. `docs/process/external-cli-carrier-operator-procedure.md`
   - canonical project operator procedure for external CLI carrier auth repair, model fixation, and smoke validation
24. `docs/process/github-issues-triage-guide.md`
   - project-owned process for GitHub Issues label taxonomy, triage, diagnostic publication, and issue-form alignment
25. `docs/process/github-pr-processing-protocol.md`
   - project-owned process for validating, merging or closing PRs, manually integrating useful stale fixes, deleting branches, and returning to `main`
26. `docs/process/command-timing-and-gate-optimization-protocol.md`
   - project-owned process for timing significant operations, diagnosing slow gates, and turning repeated command/script/CI latency into optimization work
27. `docs/process/project-error-search-runtime-diagnostics-protocol.md`
   - project-owned process overlay for applying generic `Error Search / Bug Reasoning` to VIDA runtime, TaskFlow, DocFlow, agent-lane, ownership, routing, and CI defect clusters
28. `docs/product/spec/multi-agent-stage-ensemble-contract.md`
   - product/runtime capability contract for stage-level independent agent attempts, consolidation receipts, and append-only TaskFlow updates
29. `docs/process/vida-runtime-development-environment.md`
   - compact project-owned runbook for keeping runtime development skills, TaskFlow, DocFlow, GitHub issue processing, and operator-efficiency work aligned
30. `docs/product/spec/feature-design-and-adr-model.md`
   - product-law owner for the split between structured feature/change design documents and linked ADRs
31. `docs/framework/templates/feature-design-document.template.md`
    - framework-owned reusable feature/change design template with stable sections and bounded variable fields
32. `docs/process/runtime-defect-function-option-matrix-protocol.md`
    - project-owned matrix protocol for runtime defect invariants, command surfaces, CLI options, output contracts, owning functions, fixtures, and proof tests
33. `docs/process/vida-runtime-hardening-release-readiness-guide.md`
    - project-owned operator guide for final `VIDA-RUNTIME-HARDENING` release-readiness closure, quality gates, release-install proof, and close sequence
34. `docs/process/zombie-d-test-writing-protocol.md`
    - project-owned ZOMBIE-D protocol for planning Rust, CLI, fixture/golden, coverage-gate, runtime defect proof, and test-task batches before writing or updating tests

-----
artifact_path: process/index
artifact_type: process_doc
artifact_version: '1'
artifact_revision: 2026-06-19
schema_version: '1'
status: canonical
source_path: docs/process/index.md
created_at: '2026-06-13T00:00:00+03:00'
updated_at: 2026-06-19T17:35:00+03:00
changelog_ref: index.changelog.jsonl
