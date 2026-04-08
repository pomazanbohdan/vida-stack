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
6. When `.codex/**` is materialized, use the delegated Codex team surface instead of collapsing the root session directly into coding.
7. Treat `vida.config.yaml` as the owner of carrier tiers and optional internal Codex aliases; project-visible activation should still use the selected carrier tier plus explicit runtime role.
8. Let runtime map the current packet role into the cheapest capable carrier tier with a healthy local score from `.vida/state/worker-strategy.json`.
9. For normal write-producing work, treat project agent-first execution as the delegated lane flow through `vida agent-init`; host-tool-specific subagent APIs are optional executor details and not the canonical project control surface.
10. Keep the root session in orchestration posture unless an explicit exception path is recorded.
11. Before any local write decision, re-check `vida status --json`, `vida taskflow recovery latest --json`, and `vida taskflow consume continue --json`; if the root-session write guard is still active, continue through packet shaping or `vida agent-init` dispatch instead of local coding.
12. Host-local shell/edit capability is not a lane-change receipt and does not authorize root-session coding.
13. Finding the patch location, reproducing a runtime defect, or hitting a worker timeout does not authorize root-session coding; wait, reroute, or record the exception path first.
14. Under continued-development intent, stay in commentary/progress mode until the user explicitly asks to stop; do not emit final closure wording while a next lawful TaskFlow continuation item is already known.
15. Do not treat commentary, an intermediate status update, or “I have explained the result” as a lawful pause boundary.
16. If closure-style wording is emitted by mistake, immediately re-enter commentary mode and bind the next lawful continuation item without waiting for more user input.
17. After any bounded result, green test, successful build, or delegated handoff, immediately bind the next lawful continuation item in the same cycle instead of pausing at a summary.
18. When recording progress into the backlog from shell, prefer `vida task update <task-id> --notes-file <path> --json` over inline shell quoting for complex text.
19. Prefer the launcher-owned intake/runtime progression surfaces over manual reconstruction:
   - `vida taskflow consume final "<request>" --json` to materialize the routed intake, dispatch receipt, and first lawful packet
   - `vida taskflow consume continue [--run-id <run-id>] [--dispatch-packet <path> | --downstream-packet <path>] [--json]` to resume one persisted chain entry; legacy runtime packets may be normalized to the canonical packet-minimum path scope before fail-closed validation
   - `vida taskflow consume advance [--run-id <run-id>] [--max-rounds <n>] [--json]` to let the bounded scheduler progress ready steps automatically
20. Treat the default `.vida/data/state/` root as long-lived local operator state, not as disposable scratch output.
21. For repeatable audits, release-proof checks, or scenario probes, prefer a fresh temp root via `VIDA_STATE_DIR=<temp-dir>` instead of cleaning pieces out of the long-lived project state.
22. When a probe needs project-bound runtime surfaces such as `vida taskflow consume bundle check --json`, do not assume that `vida boot` alone is sufficient on a raw temp root; bind the temp state through the matching project activation/bootstrap workflow first.
23. Do not manually prune backing-store subdirectories such as `manifest/`, `wal/`, `vlog/`, `sstables/`, or `runtime-consumption/` from a long-lived state root; if that state root is broken, use an explicit reset/reinit workflow instead of partial deletion.
24. Treat generated files under `.vida/data/state/**` as runtime operational artifacts rather than reviewable product changes unless a bounded task explicitly targets state-store fixtures or runtime-state debugging.

-----
artifact_path: process/project-operations
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-04-08'
schema_version: '1'
status: canonical
source_path: docs/process/project-operations.md
created_at: '2026-04-04T20:24:09+03:00'
updated_at: 2026-04-08T06:53:51.14572422Z
changelog_ref: project-operations.changelog.jsonl
