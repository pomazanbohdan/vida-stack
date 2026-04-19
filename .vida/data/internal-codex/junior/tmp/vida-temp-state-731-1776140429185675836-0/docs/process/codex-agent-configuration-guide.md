# Codex Agent Configuration Guide

This project uses framework-materialized `.codex/**` as the local Codex runtime surface.

Source-of-truth rule:

- `vida.config.yaml -> host_environment.codex.agents` owns carrier-tier metadata, rates, runtime-role fit, and task-class fit
- `vida.config.yaml -> agent_extensions.registries.dispatch_aliases` owns the dispatch-alias registry for executor-local overlays
- `.codex/**` is the rendered executor surface used by Codex after activation
- `.codex/config.toml` should expose the carrier tiers materialized from overlay

Carrier rule:

- the primary visible agent model is `junior`, `middle`, `senior`, `architect`
- runtime role remains explicit activation state such as `worker`, `coach`, `verifier`, or `solution_architect`
- internal alias ids may exist in registry state, but they must not replace the carrier-tier model at the project surface

Working rule:

1. The root session stays the orchestrator.
2. Documentation/specification work should complete the bounded design document first.
3. Before delegated implementation starts, open the feature epic/spec task in `vida taskflow` and close the spec task only after the design artifact is finalized.
4. After a bounded packet exists, route research, specification, planning, implementation, review, and verification through the configured tier ladder instead of collapsing into root-session coding.
5. Let runtime choose the cheapest capable configured carrier tier with a healthy local score from `.vida/state/worker-strategy.json` and pass the lawful runtime role explicitly.
6. Canonical delegated execution still dispatches through `vida agent-init`; host-tool-specific Codex subagent APIs are optional executor details and not the primary project delegation surface.
7. Before any local write decision, re-check `vida status --json`, `vida taskflow recovery latest --json`, and `vida taskflow consume continue --json`; an active root-session write guard still means orchestration-only.
8. If the user explicitly orders agent-first or parallel-agent execution, keep that routing sticky; do not silently substitute root-session coding because a host tool offers local write access.
9. Finding the patch location, reproducing a runtime defect, hitting a worker timeout, or tripping a thread-limit/`not_found` lane failure is not a lane-change receipt and does not authorize root-session coding.
10. Recover delegated-lane saturation first: inspect active lanes, synthesize completed returns, reclaim closeable lanes, and retry lawful `vida agent-init` dispatch before any local fallback is considered.
11. Under continued-development intent, stay in commentary/progress mode and continue routing; do not emit final closure wording while a next lawful continuation item is already known.
12. Do not treat commentary, an intermediate status update, or “I have explained the result” as a lawful pause boundary.
13. If closure-style wording is emitted by mistake, immediately re-enter commentary mode and bind the next lawful continuation item without waiting for more user input.
14. Sticky continuation intent does not authorize choosing the first ready task or an adjacent slice by plausibility; continue only when the active bounded unit is explicit from user wording or runtime evidence.
15. If `vida status --json` or `vida orchestrator-init --json` does not expose explicit `active_bounded_unit`, `why_this_unit`, `primary_path`, and sequential-vs-parallel posture, fail closed to an ambiguity report instead of continuing implementation.
16. When recording task progress from shell, prefer `vida task update <task-id> --notes-file <path> --json` over inline shell quoting for complex text.
17. Use `.vida/project/agent-extensions/**` for project-local role and skill overlays; do not treat `.codex/**` as the owner of framework or product law.

-----
artifact_path: process/codex-agent-configuration-guide
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-04-04'
schema_version: '1'
status: scaffold
source_path: docs/process/codex-agent-configuration-guide.md
created_at: '2026-04-04T00:00:00Z'
updated_at: '2026-04-04T00:00:00Z'
changelog_ref: codex-agent-configuration-guide.changelog.jsonl
