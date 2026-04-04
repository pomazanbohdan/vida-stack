# VIDA Project Bootstrap Carrier

<identity>
You are operating inside a VIDA-initialized project.

This file is the generated downstream bootstrap carrier.
It is a delivery surface, not the framework owner layer.

Core rule:
1. Use command-first bootstrap through the local `vida` binary.
2. Use `AGENTS.sidecar.md` as the project docs map.
3. Use bounded framework canonical ids through `vida protocol view <id>` only when the runtime init surfaces leave an edge case unresolved.
4. In host-agent execution, treat agent ids as execution carriers (model/tier/cost/effectiveness), while runtime role remains a separate activation state.
5. Runtime may bind any admissible carrier to any runtime role when role/task-class constraints allow it, then select by capability/admissibility, local score/telemetry guard, and cheapest eligible carrier.
6. L0 thinking activation: keep `instruction-contracts/overlay.step-thinking-protocol` and `instruction-contracts/overlay.session-context-continuity-protocol` active for orchestrator lanes; worker lanes activate them only when the packet/runtime explicitly requires them.
7. For normal write-producing development work, "agent mode" means the project runtime's delegated lane flow through `vida agent-init`; host-tool-specific subagent APIs are optional carrier/executor details and do not replace the canonical VIDA/TaskFlow delegation path.
8. Host-local write capability, shell access, or direct patch tools do not authorize root-lane implementation; while the root-session write guard is active, lawful write ownership still routes through `vida agent-init` unless an explicit exception-path receipt exists.

Canonical bootstrap routes:
1. Main/root lane: `vida orchestrator-init`
2. Worker/agent lane: `vida agent-init`
3. Pending onboarding or activation: `vida project-activator`

Activation rule:
1. If `vida orchestrator-init` or `vida agent-init` reports `pending_activation`, do not enter normal execution.
2. Use `vida project-activator` to record project identity, language policy, docs roots, and host CLI setup.
3. During pending activation, use `vida docflow` for bounded documentation/readiness inspection.
4. During pending activation, do not enter `vida taskflow` or any legacy runtime surface.

Normal feature-delivery rule:
1. If a request asks for research, detailed specifications, an implementation plan, and then code, create or update one bounded design document before code execution.
2. Start from the local project template referenced by `AGENTS.sidecar.md`.
3. Keep that document canonical through `vida docflow`.
4. After the design document fixes the bounded file set and proof targets, continue through orchestrated execution rather than collapsing immediately into root-session coding.
5. When normal write-producing work is lawful, shape and dispatch the next bounded packet through `vida agent-init` rather than waiting for or substituting any host-local subagent primitive.
6. The mere ability to edit files locally from the host tool is not a lane-change receipt and must not be treated as permission to bypass delegated execution.

Host CLI rule:
1. Host agent templates are activated through `vida project-activator`, not `vida init`.
2. When activation materializes the selected host template, close and restart that tool so the agents become visible to the runtime environment.
</identity>

## Bootstrap Sequence

1. Read `AGENTS.sidecar.md`.
2. Run the bounded runtime init surface for the active lane:
   - `vida orchestrator-init`
   - or `vida agent-init`
3. If the init surface reports `pending_activation`, run `vida project-activator` before ordinary work.
4. Keep the L0 thinking activation rule active from Core rule before continuing lane-specific work.
5. Prefer project-local docs/process/spec guidance resolved from `AGENTS.sidecar.md`.
6. Open deeper framework protocol surfaces only on demand through canonical shorthand ids interpreted via `vida protocol view`.

## Working Boundary

1. This file routes bootstrap only.
2. Project documentation ownership belongs to project docs resolved through `AGENTS.sidecar.md`.
3. Framework owner law remains in the framework runtime and bounded protocol-view surfaces.
4. Do not treat this generated carrier as the owner of framework policy.

## Final Report Rule

1. A final user-facing closure report is allowed only when the user explicitly asks to end/close/finalize the session.
2. Requests such as `продовжи агентами`, `continue by agents`, or equivalent continuation wording are sticky orchestration intent, not closure intent.
3. Under sticky continuation intent, keep operating through commentary/progress updates and do not emit a final closure report.
4. Before any final closure report, enforce a pre-response gate: `active_agents == 0`, no unresolved delegated handoff state, and no ready TaskFlow continuation item unless the user explicitly asks to stop.
5. If that gate fails, continue orchestration via commentary and do not emit final closure wording.
6. For closure-ready final user-facing reports, end with the explicit terminal line `Session status: completed, closing this session.`
7. Immediately after that terminal line, emit one extra blank line.
8. Under sticky continuation intent, `final` channel/user-facing terminal wording is fail-closed forbidden; only `commentary`/progress updates are lawful until the user explicitly requests stop/closure.
9. If a premature closure-style message is emitted by mistake, immediately re-enter commentary mode, acknowledge protocol violation, and continue TaskFlow dispatch without waiting for additional user confirmation.
10. After any completed bounded step, green test, successful build, finished agent handoff, or intermediate report, immediately bind the next lawful continuation item and continue without waiting for additional user confirmation when continuation intent is active.
11. Commentary is visibility only; it is never a lawful pause boundary by itself while a next bounded continuation item is already known.
12. Under active user direction, the explicit user-ordered sequence has priority over the agent's own notion of technical completeness, cleanup order, or preferred development plan.
13. The agent must not expand scope, reorder the user-specified sequence, or begin adjacent development tracks unless the current bounded step cannot be completed without that work or the user explicitly authorizes the expansion.

-----
artifact_path: install/assets/agents-scaffold
artifact_type: bootstrap_doc
artifact_version: '1'
artifact_revision: '2026-03-14'
schema_version: '1'
status: canonical
source_path: install/assets/AGENTS.scaffold.md
created_at: '2026-03-14T18:10:00+02:00'
updated_at: 2026-04-04T20:12:10.231336708Z
changelog_ref: AGENTS.scaffold.changelog.jsonl
