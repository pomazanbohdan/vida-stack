# Project Operations

Current operating baseline:

- bootstrap through `AGENTS.md` followed by the bounded VIDA init surfaces
- use `AGENTS.sidecar.md` as the project documentation map
- while project activation is pending, do not enter TaskFlow execution; use `vida project-activator` and `vida docflow`

Default feature-delivery flow:

1. If the request asks for research, specifications, a plan, and then implementation, start with a bounded design document.
2. Use the local template at `docs/product/spec/templates/feature-design-document.template.md`.
3. Open one feature epic and one spec-pack task in `vida taskflow` before code execution.
4. Keep the design artifact canonical through `vida docflow init`, `vida docflow finalize-edit`, and `vida docflow check`.
5. Close the spec-pack task and shape the next work-pool/dev packet in `vida taskflow` after the design document names the bounded file set, proof targets, and rollout.
6. When the selected host runtime surface is materialized, use the delegated host team surface instead of collapsing the root session directly into coding.
7. Treat `vida.config.yaml` as the owner of carrier tiers, host-system inventory, and any optional internal aliases; project-visible activation should still use the selected carrier tier plus explicit runtime role.
8. Let runtime map the current packet role into the cheapest capable carrier tier with a healthy local score from `.vida/state/worker-strategy.json`.
9. For normal write-producing work, treat project agent-first execution as the delegated lane flow through `vida agent-init`; host-tool-specific subagent APIs are optional executor details and not the canonical project control surface.
10. Keep the root session in orchestration posture unless an explicit exception path is recorded.
11. Before any local write decision, re-check `vida status --json`, `vida taskflow recovery latest --json`, and `vida taskflow consume continue --json`; if the root-session write guard is still active, continue through packet shaping or `vida agent-init` dispatch instead of local coding.
12. Host-local shell/edit capability is not a lane-change receipt and does not authorize root-session coding.
13. If the user explicitly orders agent-first or parallel-agent execution, keep that routing intent sticky; do not silently substitute root-session coding.
14. Finding the patch location, reproducing a runtime defect, hitting a worker timeout, or tripping a thread-limit/`not_found` lane failure does not authorize root-session coding; recover delegated lanes, wait, reroute, or record the exception path first.
15. If delegated execution returns only an activation view without execution evidence and a bounded read-only diagnostic path still exists, continue diagnosis to a code-level blocker or next bounded fix before asking the user to choose a route.
16. Saturation recovery means: inspect active lanes, synthesize completed returns, reclaim closeable lanes, and retry lawful `vida agent-init` dispatch before any local fallback is considered.
17. Under continued-development intent, stay in commentary/progress mode until the user explicitly asks to stop; do not emit final closure wording while a next lawful TaskFlow continuation item is already known.
18. Do not treat commentary, an intermediate status update, or “I have explained the result” as a lawful pause boundary.
19. If closure-style wording is emitted by mistake, immediately re-enter commentary mode and bind the next lawful continuation item without waiting for more user input.
20. After any bounded result, green test, successful build, or delegated handoff, immediately bind the next lawful continuation item in the same cycle instead of pausing at a summary.
21. Sticky continuation intent is not permission to self-select `ready_head[0]`, the first ready backlog item, or any adjacent slice; fail closed unless the active bounded unit is explicit from user wording or runtime evidence.
22. If continued-development intent is active but `vida status --json` or `vida orchestrator-init --json` cannot state `active_bounded_unit`, `why_this_unit`, `primary_path`, and sequential-vs-parallel posture, publish an ambiguity report instead of continuing implementation.
23. When recording progress into the backlog from shell, prefer `vida task update <task-id> --notes-file <path> --json` over inline shell quoting for complex text.

-----
artifact_path: process/project-operations
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-04-04'
schema_version: '1'
status: scaffold
source_path: docs/process/project-operations.md
created_at: '2026-04-04T00:00:00Z'
updated_at: '2026-04-04T00:00:00Z'
changelog_ref: project-operations.changelog.jsonl
