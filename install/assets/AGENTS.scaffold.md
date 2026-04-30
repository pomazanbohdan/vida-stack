# VIDA Project Bootstrap Carrier

<identity>
You are operating inside a VIDA-initialized project.

This file is the generated downstream bootstrap carrier.
It is a delivery surface, not the framework owner layer.

Core rule:
1. Use command-first bootstrap through the local `vida` binary.
2. Use `AGENTS.sidecar.md` as the project agent-instructions overlay; its project docs map is a required section, not the whole sidecar contract.
3. Use bounded framework canonical ids through `vida protocol view <id>` only when the runtime init surfaces leave an edge case unresolved.
4. In host-agent execution, treat agent ids as execution carriers (model/tier/cost/effectiveness), while runtime role remains a separate activation state.
5. Runtime may bind any admissible carrier to any runtime role when role/task-class constraints allow it, then select by capability/admissibility, local score/telemetry guard, and cheapest eligible carrier.
6. L0 thinking activation: keep `instruction-contracts/overlay.step-thinking-protocol` and `instruction-contracts/overlay.session-context-continuity-protocol` active for orchestrator lanes; worker lanes activate them only when the packet/runtime explicitly requires them.
7. For normal write-producing development work, "agent mode" means the project runtime's delegated lane flow through `vida agent-init`; host-tool-specific subagent APIs are optional carrier/executor details and do not replace the canonical VIDA/TaskFlow delegation path.
8. Host-local write capability, shell access, or direct patch tools do not authorize root-lane implementation; while the root-session write guard is active, lawful write ownership still routes through `vida agent-init` unless runtime status reports `local_exception_takeover_state=active` and `root_local_write_allowed=true` for the same active packet. A recorded exception-path receipt by itself is only `receipt_recorded` and does not authorize local write.
9. If the user explicitly orders agent-first or parallel-agent execution, that routing intent is sticky; root must restore, reclaim, or re-dispatch delegated lanes before considering any local exception path and must not silently substitute root-session implementation.
10. If `vida agent-init` or runtime dispatch returns an activation/view-only handoff without execution evidence, treat that result as a non-executing bridge blocker, not as delegated work completion and not as permission for root-session implementation; if a bounded read-only diagnostic path remains, continue to a code-level blocker or next bounded fix before asking the user to choose a route.
11. Before any root-session write-producing mutation, require one of: receipt-backed delegated execution evidence for the active packet, or runtime-confirmed active exception takeover for the same bounded packet (`local_exception_takeover_state=active` with `root_local_write_allowed=true`). `receipt_recorded`, `admissible_not_active`, `activation_view_only`, `internal_activation_view_only`, packet location discovery, or a ready patch idea are all insufficient.
12. Exception-path evidence has three distinct operator states: `receipt_recorded`, `admissible_not_active`, and `active`; only `active` may unlock root-session local write.

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
7. Agent/thread saturation, stale lane handles, or `not_found` carrier errors require saturation recovery first: inspect active lanes, synthesize any completed returns, reclaim closeable lanes, and retry lawful `vida agent-init` dispatch before any local fallback is even considered.
8. If the active runtime snapshot/status still reports root-local write as forbidden, remain in shaping/diagnosis/reroute only; do not convert a bounded read-only diagnosis into a local fix without receipt-backed delegated execution evidence or runtime-confirmed active exception takeover (`local_exception_takeover_state=active` with `root_local_write_allowed=true`) for the active packet.

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
5. Prefer project-local operating rules and docs/process/spec guidance resolved from `AGENTS.sidecar.md`.
6. Open deeper framework protocol surfaces only on demand through canonical shorthand ids interpreted via `vida protocol view`.

## Compact Re-entry Rule

1. After any host-side compact, context drop, or continuity uncertainty, re-read `AGENTS.md` and `AGENTS.sidecar.md` before continuing.
2. Re-run the bounded runtime init surface for the active lane (`vida orchestrator-init --json` for the root lane, `vida agent-init --json` for a worker lane) before selecting the next step.
3. After re-entry, restate three runtime fields before any new write-producing move:
   - `active_bounded_unit`
   - `why_this_unit`
   - `sequential_vs_parallel_posture`
4. If any of those fields are missing, ambiguous, or stale relative to the current runtime evidence, fail closed to diagnosis/bind/recovery and do not continue implementation heuristically.
5. Keep the session and step thinking overlays active across compact boundaries; re-open the bounded overlay/runtime surfaces when the compact may have weakened confidence in the active thinking mode or task-class selection.
6. Do not duplicate full thinking algorithms into this bootstrap carrier; this file should enforce the mandatory re-entry contract and point back to the canonical runtime/init surfaces for the algorithms themselves.

## Working Boundary

1. This file routes bootstrap only.
2. Project documentation ownership belongs to project docs resolved through the project docs map section in `AGENTS.sidecar.md`.
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
14. Sticky continuation intent is not permission to self-select `ready_head[0]`, the first ready backlog item, or any adjacent slice; continuation remains blocked until the active bounded unit is explicit from the user's wording or uniquely evidenced runtime state.
15. If continuation intent is active but the agent cannot state `active_bounded_unit`, `why_this_unit`, and whether the next move is sequential or parallel-safe, fail closed to an ambiguity report rather than continuing implementation.

-----
artifact_path: install/assets/agents-scaffold
artifact_type: bootstrap_doc
artifact_version: '1'
artifact_revision: '2026-03-14'
schema_version: '1'
status: canonical
source_path: install/assets/AGENTS.scaffold.md
created_at: '2026-03-14T18:10:00+02:00'
updated_at: 2026-04-30T22:15:50.5483113Z
changelog_ref: AGENTS.scaffold.changelog.jsonl
