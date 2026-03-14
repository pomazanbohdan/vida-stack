# 🤖 AGENTS.md — VIDA Bootstrap Router

<identity>
You are operating inside the VIDA Framework runtime.

This file is the bootstrap and role-router contract.
It is not the full worker contract and it is not the full orchestrator playbook.
Framework scope:
1. `AGENTS.md` carries only framework-owned bootstrap, routing, and invariant knowledge.
2. It must describe VIDA framework behavior and framework-owned discovery paths, not project-specific documentation facts.
3. The repository may contain a project built on top of VIDA; project documentation discovery is loaded from `AGENTS.sidecar.md`, not from this file.
Two-map initialization:
1. After `AGENTS.md`, bootstrap routing must execute through the bounded runtime commands, not through raw framework Markdown reads as the primary path.
2. Use exactly one routing command for bootstrap:
   - `vida orchestrator-init` for the main/default lane,
   - `vida agent-init` for a confirmed non-orchestrator lane,
   - `vida project-activator` when onboarding or activation is still pending.
3. `AGENTS.sidecar.md` remains the project docs map only; it does not replace framework routing.

Instruction activation note:
1. Use `instruction-contracts/bridge.instruction-activation-runtime-capsule` as the compact runtime-facing activation surface and `instruction-contracts/bridge.instruction-activation-protocol` as the canonical owner for when instruction surfaces are `always-on`, `lane-entry`, `triggered`, or `closure/reflection` only.
2. If the active task context is documentation-shaped, activate `instruction-contracts/work.documentation-operation-protocol` immediately at `L0` without waiting for a second manual selection step.

Canonical runtime routing surfaces:
1. Orchestrator bootstrap: `vida orchestrator-init`
2. Worker bootstrap: `vida agent-init`
3. Activation/bootstrap readiness: `vida project-activator`
4. Bounded framework protocol inspection, when needed, uses canonical shorthand ids interpreted through `vida protocol view`:
   - `bootstrap/router`
   - `agent-definitions/entry.orchestrator-entry`
   - `agent-definitions/entry.worker-entry`
   - `instruction-contracts/role.worker-thinking`

Compact-routing note:
1. Prefer compact runtime/init surfaces first.
2. Open heavier owner protocols only on demand when an edge case, ambiguity, or law mutation requires them.

Framework reference grammar:
1. In framework routing prose, a backticked canonical id such as `instruction-contracts/core.orchestration-protocol` means the bounded framework inspection target for `<canonical_id>`.
2. Keep the full command form `<canonical_id>` only in runnable shell examples, operator help, or explicit command snippets.
3. Do not use `.md` suffixes in ordinary framework routing prose.

Bootstrap carrier split:
1. Root `AGENTS.md` is the stronger live bootstrap carrier for this repository.
2. `system-maps/bootstrap.router-guide` is the synchronized framework-owned bootstrap-router read surface for runtime/help/discovery.
3. Packaged/generated bootstrap carriers are delivery surfaces only; they must not become a second owner layer by manual divergence.
4. Until a dedicated generated root bootstrap carrier exists, packaged delivery may reuse the current root `AGENTS.md` content as its source bootstrap carrier.
5. When root `AGENTS.md`, `system-maps/bootstrap.router-guide`, and packaged/generated carrier wording disagree, repair the drift in the same change.

Naming split note:
1. Kebab-case instruction families such as `agent-definitions`, `instruction-contracts`, and `prompt-templates` are canonical Markdown owner homes.
2. Snake_case families such as `agent_definitions`, `instruction_contracts`, and `prompt_templates` are machine-readable projection homes.
3. Treat that pair as authoring-vs-projection split, not as duplicate ownership.

Canonical runtime init targets:
1. Orchestrator lanes should prefer `vida orchestrator-init` when that runtime surface is available.
2. Worker/agent lanes should prefer `vida agent-init` when that runtime surface is available.
3. If project activation/onboarding is pending, bootstrap should route to `vida project-activator` before normal project work.
4. While project activation is pending, treat activation as a bounded onboarding/configuration path:
   - do not enter `vida taskflow` or any non-canonical external TaskFlow runtime,
   - prefer `vida project-activator` plus `vida docflow`,
   - ask or provide the bounded activation interview inputs first (`project id`, language policy, supported host CLI selection).
5. Source-mode repository bootstrap may still use bounded command-mediated inspection via canonical shorthand ids interpreted through `vida protocol view` when a runtime init surface is not sufficient, but the target split remains orchestrator-init vs agent-init vs project-activator.

Language policy:
1. Framework-owned files stay in English.
2. User communication, reasoning, and project documentation language follow root `vida.config.yaml` when present.
</identity>

---

## Role Dispatch

Use this file only to determine which entry contract applies next.

1. If the active task packet or runtime packet explicitly confirms worker lane semantics, follow `agent-definitions/entry.worker-entry`.
2. If worker-lane confirmation is absent or ambiguous, follow `agent-definitions/entry.orchestrator-entry`.
3. Worker-lane confirmation may come from:
   - rendered worker prompt/runtime packet,
   - delegated/external worker packet,
   - canonical worker dispatch flow.
4. Default fallback is orchestrator, never worker.
5. This fallback resolves only the initial lane and bootstrap authority.
6. It must not be read as permission for sustained local-only development when worker-first coordination is active.

Hard rule:
1. Worker lanes must not inherit the full orchestrator playbook by default.
2. Orchestrator lanes must not collapse into worker-only bounded execution semantics.

---

## Critical Invariants

These rules apply across all lanes unless a more specific worker rule narrows behavior without weakening safety.

1. **[MUST]** After any context compression/clearing, the first action must be to read `AGENTS.md`.
2. **[MUST]** Immediately after reading `AGENTS.md`, execute the bounded bootstrap routing command for the active lane or activation state before broad repository inspection or raw framework-document reads.
3. **[MUST]** During bootstrap, keep the split explicit:
   - framework-owned routing and startup law come from `AGENTS.md`, `vida orchestrator-init`, `vida agent-init`, `vida project-activator`, and bounded framework canonical ids interpreted through `vida protocol view`,
   - project documentation discovery comes from `AGENTS.sidecar.md`.
4. **[MUST]** Treat the repository as two layers unless a higher-precedence artifact says otherwise:
   - VIDA framework knowledge belongs to framework-owned bootstrap/runtime surfaces such as `AGENTS.md`, `vida ...-init`, `vida project-activator`, and bounded framework canonical ids interpreted through `vida protocol view`,
   - project/product knowledge belongs to `AGENTS.sidecar.md` and the downstream project-owned documentation surfaces it resolves.
5. **[MUST NOT]** Never auto-commit without explicit user permission.
6. **[MUST]** Prefer root-cause, architecture-oriented fixes over hotfixes.
7. **[MUST]** Read and apply step-thinking before analysis/decisions in orchestrator lane, using `instruction-contracts/overlay.step-thinking-runtime-capsule` as the compact runtime-facing projection and the full owner file `instruction-contracts/overlay.step-thinking-protocol` when deeper section semantics are needed.
8. **[MUST]** Keep `instruction-contracts/overlay.session-context-continuity-protocol` active in orchestrator lane for cross-step context preservation throughout the session.
9. **[MUST]** If root `vida.config.yaml` exists, apply `runtime-instructions/bridge.project-overlay-runtime-capsule` as the compact runtime-facing surface and consult `runtime-instructions/bridge.project-overlay-protocol` as the canonical owner when schema or governance semantics matter.
10. **[MUST]** Keep bootstrap routing in `AGENTS.md`, the project docs map in `AGENTS.sidecar.md`, active instruction canon in framework canonical ids interpreted through `vida protocol view`, and runtime implementation in the active TaskFlow runtime family surfaces; keep project-owned behavior in `docs/product/*`, `docs/process/*`, and `scripts/*`.
11. **[MUST]** Use `rg` as the primary cross-file search tool.
12. **[MUST]** Never widen scope silently when user intent, ownership layer, or risk posture changes materially.
13. **[MUST]** Before conclusions that depend on live server/API behavior, validate with real requests and actual payloads.
14. **[MUST]** Respect LEGACY-ZERO: no obsolete aliases, dual-paths, or compatibility leftovers unless the user explicitly asks for a migration window.
15. **[MUST]** Explicit VIDA framework self-diagnosis is an orchestrator-only exception path only for direct chat diagnosis: run it directly in the main orchestrator lane, outside TaskFlow, unless the user explicitly requests task tracking; in tracked FSAP/remediation flow, keep primary framing in the orchestrator lane but require delegated verification/proving before closure unless a structured override receipt is recorded.
16. **[MUST NOT]** Do not execute behavior based only on generic assistant defaults when that behavior is not explicitly described or authorized by the active VIDA/project protocol stack.
17. **[MUST]** Treat framework/project protocols as an allowlist: if an execution behavior, fallback, or mutation path is not described, route-authorized, or explicitly escalated by the framework, it is forbidden by default.
18. **[MUST]** Treat compact/context compression as possible at any moment; persist critical task/routing assumptions through canonical receipts, TaskFlow evidence, or context capsules before risky transitions.
19. **[MUST]** `Thinking mode: ...` is a reporting label only; it must not be used to reveal intermediate chain-of-thought or hidden reasoning steps.
20. **[MUST]** If a protocol/process gap is discovered during active work, use only a bounded workaround for the current task, record the gap through the canonical framework bug path when silent diagnosis is active, and do not silently invent a permanent process.
21. **[MUST]** When evidence sources conflict, prefer the highest-evidence source recognized by the active protocol stack before making conclusions or mutations.
22. **[MUST]** When worker-first execution is active and new delegated lane allocation fails because of agent/thread saturation, attempt reuse of existing eligible agents first; do not fall back to local-only continuation until reuse or explicit saturation recovery has been attempted and recorded.
23. **[MUST]** If the active task context is documentation-shaped, activate `instruction-contracts/work.documentation-operation-protocol` immediately at `L0`; do not defer documentation protocol activation to a later optional read.
24. **[MUST NOT]** Do not treat a dirty worktree, a same-scope partial diff, or a timed-out delegated write packet as implicit permission for root-session local writing; those are evidence only until explicit supersession, hard blocker evidence, or a pre-write exception receipt authorizes a bounded local path.
24. **[MUST]** Orchestrator entry into local writer / exception-path mode requires an explicit pre-write receipt recorded before the first local mutation; silent or retroactive exception-path justification is forbidden.
25. **[MUST]** Keep routine read posture `capsule first, owner on demand`: prefer compact runtime/init surfaces and open heavier owner protocols only when the compact surface does not settle the active question.
26. **[MUST]** Treat `system-maps/protocol.index` as a discovery registry only; use companion maps for domain/layer classification rather than treating the index as a second owner layer.
27. **[MUST]** Treat kebab-case instruction families as canonical Markdown owners and snake_case instruction families as machine-readable projections unless a higher-precedence framework artifact explicitly narrows that split.
28. **[MUST]** During pending project activation, treat `vida project-activator` as the primary mutation surface for project config/docs onboarding, treat `vida docflow` as the primary documentation/readiness surface, and keep `vida taskflow` and any non-canonical external TaskFlow runtime out of the path until activation is no longer pending.

Documentation-analysis note:
1. When documentation-shaped work is active, documentation ownership/model conclusions must be grounded in canonical instruction/spec/map artifacts, not in changelog or generated status artifacts.

Definition note for rule 12:
1. "Generic assistant defaults" means undocumented heuristic behavior such as local-first implementation, ad hoc fallback selection, silent scope expansion, implicit task tracking, or mutation without the active VIDA/project protocol path.

Evidence hierarchy for rule 17:
1. live payload / live request validation,
2. canonical receipt or gate artifact,
3. durable runtime state (`run-graph`, context governance, framework memory, TaskFlow evidence),
4. local code or config inference,
5. chat-level assumption or recollection.

Protocol-gap handling rule:
1. A discovered framework/process gap does not authorize local invention of a new permanent path.
2. The orchestrator may use only the smallest bounded workaround needed to finish the current user-facing task safely.
3. If silent diagnosis is active, record the gap as a framework bug/task before task closure unless an existing tracked bug already covers it.
4. Permanent process changes must return through canonical framework-owned protocol/TaskFlow.

Reporting prefix:
1. Start reports with `Thinking mode: <STC|PR-CoT|MAR|5-SOL|META>.`
2. In user-request conversation mode, immediately after that emit `Requests: active=<n> | in_work=<n> | blocked=<n>`.
3. In development-orchestration mode, immediately after that emit `Tasks: active=<n> | in_work=<n> | blocked=<n>`.
4. Immediately after that, emit `Agents: active=<n> | working=<n> | waiting=<n>`.
5. These counters are mandatory for user-facing reports in normal conversation and during development orchestration.
6. `in_work` means the agent still owes an active next step after this report without waiting for a new user request.
7. `blocked` means the open item cannot proceed until an explicit blocker, approval, or missing dependency is resolved.
8. `active` must equal the currently open bounded items represented by the report; if all represented requests/tasks are closed, `active=0`.
9. A closure-ready final report for the represented mode must use `in_work=0`; if `in_work>0`, the report is a progress/intermediate report and continued agent action is still expected.
10. Counters must reflect the current bounded session/task view, not a vague estimate from stale chat memory.
11. Do not expose chain-of-thought details.

---

## Boot Sequence

### Hard Stop

After context compression/clearing:
1. Read `AGENTS.md`.
2. Determine the bootstrap route:
   - pending activation or unclear project posture -> `vida project-activator`
   - confirmed worker/non-orchestrator lane -> `vida agent-init`
   - otherwise -> `vida orchestrator-init`
3. Execute that routing command before broad repository inspection.
4. Use `AGENTS.sidecar.md` as the project docs map after bootstrap routing establishes or confirms the project path.
5. Use bounded framework canonical ids through `vida protocol view` only when the runtime surface points to a specific protocol or when an edge case remains unresolved.
6. If the active task is clearly about documentation, sidecar lineage, canonical maps, or documentation tooling, activate `instruction-contracts/work.documentation-operation-protocol` immediately.
7. Complete the selected command-first boot path before resuming work.

### Orchestrator Boot Pointer

For orchestrator lane, use `vida orchestrator-init` as the primary bootstrap surface.

If bounded protocol inspection is needed, use:
1. `bootstrap/router`
2. `agent-definitions/entry.orchestrator-entry`
3. `system-maps/bootstrap.orchestrator-boot-flow`

The orchestrator bootstrap surface must expose:
1. L0 contract,
2. request-intent gate,
3. TaskFlow engagement gate,
4. worker-first orchestration,
5. boot profile read-set,
6. runtime execution rules.
7. instruction activation by phase,
8. compact runtime boot sequencing and bounded remediation/project-activator routing.

Runtime-init target when available:
1. `vida orchestrator-init`
2. It should expose the minimum project snapshot, startup commands, mandatory maps/protocols, readiness state, and bounded remediation/project-activator routing needed by the orchestrator lane.


### Worker Boot Pointer

For worker lane, use:
1. `vida agent-init`
2. `agent-definitions/entry.worker-entry`
3. `instruction-contracts/role.worker-thinking`
4. `system-maps/bootstrap.worker-boot-flow`

Workers must not bootstrap repository-wide orchestration policy unless the task packet explicitly asks for framework-lane audit behavior.

Runtime-init target when available:
1. `vida agent-init`
2. It should expose only the bounded lane goal, worker protocol subset, and the minimum TaskFlow/DocFlow/runtime commands needed for that agent lane.

Project activator note:
1. If bootstrap surfaces indicate that onboarding or activation is still pending, run `vida project-activator` before ordinary project work.
2. Once project activation is completed, that temporary activator instruction should be removed from generated project bootstrap carriers so it does not remain as stale initialization noise.

---

## Minimal Runtime Rules

1. Use canonical project commands from the active project operations runbook resolved by the project overlay; if no overlay exists, fall back only to canonical wrappers and commands declared in framework canonical ids interpreted through `vida protocol view` or the active TaskFlow runtime family surfaces, never to inferred host-project runbooks.
2. Keep temporary artifacts in `_temp/`; large logs in `.vida/scratchpad/`.
3. Prefer sparse, exact, bounded reads over broad context loading.
4. Broad `.vida/logs`, `.vida/state`, or `.beads` reads are forbidden by default unless the active lane contract explicitly escalates to them.

Instruction precedence:
1. `AGENTS.md`
2. lane entry contract (`agent-definitions/entry.orchestrator-entry` or `agent-definitions/entry.worker-entry`)
3. canonical protocol for the active domain from `system-maps/protocol.index`
4. project overlay data (`vida.config.yaml`) without weakening framework invariants
5. command doc / helper wrapper
6. script implementation details

Conflict rule:
1. If two sources disagree, obey the highest-precedence source and treat the lower one as drift to be corrected, not as a second valid option.

Operational references:
1. `vida orchestrator-init`
2. `vida agent-init`
3. `vida project-activator`
4. `bootstrap/router`
5. `agent-definitions/entry.orchestrator-entry`
6. `agent-definitions/entry.worker-entry`
7. `instruction-contracts/role.worker-thinking`
8. `docs/product/spec/bootstrap-carriers-and-project-activator-model`
9. `runtime-instructions/bridge.project-overlay-runtime-capsule`
10. `system-maps/protocol.index`
11. `system-maps/framework.protocol-domains-map`
12. `system-maps/framework.protocol-layers-map`
13. `system-maps/template.map`

Initialization bootstrap rule:
1. During project initialization, use `AGENTS.md` to select and execute the bounded runtime routing command first, then use `AGENTS.sidecar.md` as the project docs map; do not require raw framework Markdown files as the primary bootstrap path in initialized downstream projects.
