# Project Operations

Current operating baseline:

- bootstrap through `AGENTS.md` followed by the bounded VIDA init surfaces
- use `AGENTS.sidecar.md` as the project agent-instructions overlay and project documentation map
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
11. Before any local write decision, re-check `vida status`, `vida taskflow recovery latest`, and `vida taskflow consume continue`; if the root-session write guard is still active, continue through packet shaping or `vida agent-init` dispatch instead of local coding.
12. Host-local shell/edit capability is not a lane-change receipt and does not authorize root-session coding.
13. Finding the patch location, reproducing a runtime defect, or hitting a worker timeout does not authorize root-session coding; wait, reroute, or record the exception path first.
14. Continuation, pause-boundary, and generic-ready-item behavior is owned by `project-orchestrator-session-start-protocol.md` and the decision table in `project-orchestrator-reusable-prompt.md`; continue only when the active bounded unit is explicit from user wording or runtime evidence.
15. If `vida status` or `vida orchestrator-init` cannot state bounded-unit and route fields when runtime is usable, fail closed to ambiguity instead of continuing implementation.
16. When recording progress into the backlog from shell, prefer `vida task update <task-id> --notes-file <path>` over inline shell quoting for complex text.
17. Prefer the launcher-owned intake/runtime progression surfaces over manual reconstruction:
   - `vida taskflow consume final "<request>"` to materialize the routed intake, dispatch receipt, and first lawful packet
   - `vida taskflow consume continue [--run-id <run-id>] [--dispatch-packet <path> | --downstream-packet <path>] [--json]` to resume one persisted chain entry; legacy runtime packets may be normalized to the canonical packet-minimum path scope before fail-closed validation
   - `vida taskflow consume advance [--run-id <run-id>] [--max-rounds <n>] [--json]` to let the bounded scheduler progress ready steps automatically
22. Treat the default `.vida/data/state/` root as long-lived local operator state, not as disposable scratch output.
23. For repeatable audits, release-proof checks, or scenario probes, prefer a fresh temp root via `VIDA_STATE_DIR=<temp-dir>` instead of cleaning pieces out of the long-lived project state.
24. When a probe needs project-bound runtime surfaces such as `vida taskflow consume bundle check --json`, do not assume that `vida boot` alone is sufficient on a raw temp root; bind the temp state through the matching project activation/bootstrap workflow first.
25. Do not manually prune backing-store subdirectories such as `manifest/`, `wal/`, `vlog/`, `sstables/`, or `runtime-consumption/` from a long-lived state root; if that state root is broken, use an explicit reset/reinit workflow instead of partial deletion.
26. Treat generated files under `.vida/data/state/**` as runtime operational artifacts rather than reviewable product changes unless a bounded task explicitly targets state-store fixtures or runtime-state debugging.
27. For Windows local framework development, use the current system tools directly: PowerShell Core through `pwsh.exe`, Rust tools from `rustup`/Cargo, ripgrep from `winget` or an explicit `RG` path, and install the release `vida.exe` only when installed-runtime validation or release admission is the active proof target.
28. Use the Windows proof ladder in cost order: script/docs-only proof through `scripts\vida-dev-gate.ps1 -Mode script-check -Json`, cheap source proof through `scripts\vida-dev-gate.ps1 -Mode quick -Json`, focused regression proof through `scripts\vida-dev-gate.ps1 -Mode focused-nextest -TestFilter <filter> -Json`, package proof through `scripts\vida-dev-gate.ps1 -Mode package-nextest -Json`, local workspace proof through `scripts\vida-dev-gate.ps1 -Mode workspace-nextest -Json` when the coherent batch is assembled, doc-test proof through `scripts\vida-dev-gate.ps1 -Mode doc-test -Json`, debug build proof through `scripts\vida-dev-gate.ps1 -Mode build-debug -Json`, debug runtime smoke through `scripts\vida-dev-gate.ps1 -Mode runtime-smoke -Json` if current state-store compatibility is needed, release archive proof through `scripts\vida-dev-gate.ps1 -Mode release-package -Json`, then installed runtime validation (`vida status --json` or the exact operator command), and only then `vida release install --json` for installed launcher or release-admission proof. Treat Windows Application Control failures for generated integration-test binaries under `target\debug\deps\*.exe` or nextest-launched test binaries as host policy blockers unless the policy is changed.
29. After Windows release install, smoke a disposable state root with `vida boot --state-dir <temp-dir>` and `vida status --state-dir <temp-dir> --summary --json` so SurrealKV filesystem compatibility is proven outside the long-lived project state.
30. For repeatable local gate timing, prefer `scripts\vida-dev-gate.ps1 -Mode <mode> -Json` on Windows over ad hoc command chains. The current modes are discoverable with `scripts\vida-dev-gate.ps1 -Help` and include `script-check`, `quick`, `focused-nextest`, `package-nextest`, `workspace-nextest`, `doc-test`, `build-debug`, `runtime-smoke`, `release-package`, `release-install`, and `target-dir-policy`.

-----
artifact_path: process/project-operations
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-06-13'
schema_version: '1'
status: canonical
source_path: docs/process/project-operations.md
created_at: '2026-04-04T20:24:09+03:00'
updated_at: 2026-06-13T01:35:00+03:00
changelog_ref: project-operations.changelog.jsonl
