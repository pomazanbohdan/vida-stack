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
1. After `AGENTS.md`, bootstrap must read two maps as one mandatory step:
   - the framework root map at `vida/root-map.md`,
   - the project docs map in `AGENTS.sidecar.md`.
2. Neither map is optional during initialization.
3. `AGENTS.sidecar.md` is the project docs map only; it does not replace the framework map.

Instruction activation note:
1. Use `vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md` as the canonical rule for when instruction surfaces are `always-on`, `lane-entry`, `triggered`, or `closure/reflection` only.
2. If the active task context is documentation-shaped, activate `vida/config/instructions/instruction-contracts/work.documentation-operation-protocol.md` immediately at `L0` without waiting for a second manual selection step.

Canonical lane entries:
1. Orchestrator entry: `vida/config/instructions/agent-definitions/entry.orchestrator-entry.md`
2. Worker entry: `vida/config/instructions/agent-definitions/entry.worker-entry.md`
3. Worker thinking subset: `vida/config/instructions/instruction-contracts/role.worker-thinking.md`

Language policy:
1. Framework-owned files stay in English.
2. User communication, reasoning, and project documentation language follow root `vida.config.yaml` when present.
</identity>

---

## Role Dispatch

Use this file only to determine which entry contract applies next.

1. If the active task packet or runtime packet explicitly confirms worker lane semantics, follow `vida/config/instructions/agent-definitions/entry.worker-entry.md`.
2. If worker-lane confirmation is absent or ambiguous, follow `vida/config/instructions/agent-definitions/entry.orchestrator-entry.md`.
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
2. **[MUST]** Immediately after reading `AGENTS.md`, read both mandatory initialization maps: `AGENTS.sidecar.md` and the framework root map at `vida/root-map.md`, before lane resolution, repository inspection, or task continuation.
3. **[MUST]** During bootstrap, keep the two-map split explicit:
   - framework-owned documentation discovery comes from `vida/root-map.md` and its downstream framework map/index surfaces,
   - project documentation discovery comes from `AGENTS.sidecar.md`.
4. **[MUST]** Treat the repository as two layers unless a higher-precedence artifact says otherwise:
   - VIDA framework knowledge belongs to framework-owned surfaces such as `AGENTS.md`, `vida/root-map.md`, and `vida/config/instructions/*`,
   - project/product knowledge belongs to `AGENTS.sidecar.md` and the downstream project-owned documentation surfaces it resolves.
5. **[MUST NOT]** Never auto-commit without explicit user permission.
6. **[MUST]** Prefer root-cause, architecture-oriented fixes over hotfixes.
7. **[MUST]** Read and apply `vida/config/instructions/instruction-contracts/overlay.step-thinking-protocol.md` algorithms before analysis/decisions in orchestrator lane.
8. **[MUST]** Keep `vida/config/instructions/instruction-contracts/overlay.session-context-continuity-protocol.md` active in orchestrator lane for cross-step context preservation throughout the session.
9. **[MUST]** If root `vida.config.yaml` exists, apply `vida/config/instructions/runtime-instructions/bridge.project-overlay-protocol.md`.
10. **[MUST]** Keep bootstrap routing in `AGENTS.md`, the project docs map in `AGENTS.sidecar.md`, active instruction canon in `vida/config/instructions/*`, and runtime implementation in `taskflow-v0/*`; keep project-owned behavior in `docs/product/*`, `docs/process/*`, and `scripts/*`.
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
23. **[MUST]** If the active task context is documentation-shaped, activate `vida/config/instructions/instruction-contracts/work.documentation-operation-protocol.md` immediately at `L0`; do not defer documentation protocol activation to a later optional read.

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
2. Do not expose chain-of-thought details.

---

## Boot Sequence

### Hard Stop

After context compression/clearing:
1. Read `AGENTS.md`.
2. Read `AGENTS.sidecar.md` as the project docs map.
3. Read the framework root map through `vida/root-map.md`.
4. Use `vida/root-map.md` for framework-owned discovery and `AGENTS.sidecar.md` for project-document discovery.
5. Resolve lane:
   - worker lane -> `vida/config/instructions/agent-definitions/entry.worker-entry.md`
   - orchestrator lane -> `vida/config/instructions/agent-definitions/entry.orchestrator-entry.md`
6. If the active task is clearly about documentation, sidecar lineage, canonical maps, or documentation tooling, activate `vida/config/instructions/instruction-contracts/work.documentation-operation-protocol.md` immediately.
7. Complete the selected boot path before resuming work.

### Orchestrator Boot Pointer

For orchestrator lane, use `vida/config/instructions/agent-definitions/entry.orchestrator-entry.md` as the canonical source for:
1. L0 contract,
2. request-intent gate,
3. TaskFlow engagement gate,
4. worker-first orchestration,
5. boot profile read-set,
6. runtime execution rules.
7. instruction activation by phase via `vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md`.
8. explicit boot sequencing via `vida/config/instructions/system-maps/bootstrap.orchestrator-boot-flow.md`.


### Worker Boot Pointer

For worker lane, use:
1. `vida/config/instructions/agent-definitions/entry.worker-entry.md`
2. `vida/config/instructions/instruction-contracts/role.worker-thinking.md`
3. `vida/config/instructions/system-maps/bootstrap.worker-boot-flow.md`

Workers must not bootstrap repository-wide orchestration policy unless the task packet explicitly asks for framework-lane audit behavior.

---

## Minimal Runtime Rules

1. Use canonical project commands from the active project operations runbook resolved by the project overlay; if no overlay exists, fall back only to canonical wrappers and commands declared in `vida/config/instructions/*` or `taskflow-v0/*`, never to inferred host-project runbooks.
2. Keep temporary artifacts in `_temp/`; large logs in `.vida/scratchpad/`.
3. Prefer sparse, exact, bounded reads over broad context loading.
4. Broad `.vida/logs`, `.vida/state`, or `.beads` reads are forbidden by default unless the active lane contract explicitly escalates to them.

Instruction precedence:
1. `AGENTS.md`
2. lane entry contract (`vida/config/instructions/agent-definitions/entry.orchestrator-entry.md` or `vida/config/instructions/agent-definitions/entry.worker-entry.md`)
3. canonical protocol for the active domain from `vida/config/instructions/system-maps/protocol.index.md`
4. project overlay data (`vida.config.yaml`) without weakening framework invariants
5. command doc / helper wrapper
6. script implementation details

Conflict rule:
1. If two sources disagree, obey the highest-precedence source and treat the lower one as drift to be corrected, not as a second valid option.

Operational references:
1. `vida/root-map.md`
2. `vida/config/instructions/agent-definitions/entry.orchestrator-entry.md`
3. `vida/config/instructions/agent-definitions/entry.worker-entry.md`
4. `vida/config/instructions/instruction-contracts/role.worker-thinking.md`
5. `vida/config/instructions/system-maps/framework.map.md`
6. `vida/config/instructions/system-maps/protocol.index.md`
7. `vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md`
8. `vida/config/instructions/instruction-contracts/work.documentation-operation-protocol.md`
9. `vida/config/instructions/system-maps/bootstrap.orchestrator-boot-flow.md`
10. `vida/config/instructions/system-maps/bootstrap.worker-boot-flow.md`

Initialization bootstrap rule:
1. During project initialization, read `AGENTS.sidecar.md` immediately after `AGENTS.md`, then resolve the framework-owned bootstrap path in `vida/root-map.md` before lane resolution or broad manual inspection.
