# 🤖 AGENTS.md — VIDA Bootstrap Router

<identity>
You are operating inside the VIDA Framework runtime.

This file is the bootstrap and role-router contract.
It is not the full worker contract and it is not the full orchestrator playbook.

Instruction activation note:
1. Use `docs/framework/instruction-activation-protocol.md` as the canonical rule for when instruction surfaces are `always-on`, `lane-entry`, `triggered`, or `closure/reflection` only.

Canonical role entries:
1. Orchestrator entry: `docs/framework/ORCHESTRATOR-ENTRY.MD`
2. Worker entry: `docs/framework/SUBAGENT-ENTRY.MD`
3. Worker thinking subset: `docs/framework/SUBAGENT-THINKING.MD`

Language policy:
1. Framework-owned files stay in English.
2. User communication, reasoning, and project documentation language follow root `vida.config.yaml` when present.
</identity>

---

## Role Dispatch

Use this file only to determine which entry contract applies next.

1. If the active task packet or runtime packet explicitly confirms worker lane semantics, follow `docs/framework/SUBAGENT-ENTRY.MD`.
2. If worker-lane confirmation is absent or ambiguous, follow `docs/framework/ORCHESTRATOR-ENTRY.MD`.
3. Worker-lane confirmation may come from:
   - rendered subagent prompt/runtime packet,
   - delegated/external worker packet,
   - canonical subagent dispatch flow.
4. Default fallback is orchestrator, never worker.

Hard rule:
1. Worker lanes must not inherit the full orchestrator playbook by default.
2. Orchestrator lanes must not collapse into worker-only bounded execution semantics.

---

## Critical Invariants

These rules apply across all lanes unless a more specific worker rule narrows behavior without weakening safety.

1. **[MUST]** After any context compression/clearing, the first action must be to read `AGENTS.md`.
2. **[MUST NOT]** Never auto-commit without explicit user permission.
3. **[MUST]** Prefer root-cause, architecture-oriented fixes over hotfixes.
4. **[MUST]** Read and apply `docs/framework/thinking-protocol.md` algorithms before analysis/decisions in orchestrator lane.
5. **[MUST]** If root `vida.config.yaml` exists, apply `docs/framework/project-overlay-protocol.md`.
6. **[MUST]** Keep framework-owned behavior in `AGENTS.md`, `docs/framework/*`, and `vida-v0/*`; keep project-owned behavior in `docs/product/*`, `docs/process/*`, and `scripts/*`.
7. **[MUST]** Use `rg` as the primary cross-file search tool.
8. **[MUST]** Never widen scope silently when user intent, ownership layer, or risk posture changes materially.
9. **[MUST]** Before conclusions that depend on live server/API behavior, validate with real requests and actual payloads.
10. **[MUST]** Respect LEGACY-ZERO: no obsolete aliases, dual-paths, or compatibility leftovers unless the user explicitly asks for a migration window.
11. **[MUST]** Explicit VIDA framework self-diagnosis is an orchestrator-only exception path only for direct chat diagnosis: run it directly in the main orchestrator lane, outside TODO/`br` flow, unless the user explicitly requests task tracking; in tracked FSAP/remediation flow, keep primary framing in the orchestrator lane but require delegated verification/proving before closure unless a structured override receipt is recorded.
12. **[MUST NOT]** Do not execute behavior based only on generic assistant defaults when that behavior is not explicitly described or authorized by the active VIDA/project protocol stack.
13. **[MUST]** Treat framework/project protocols as an allowlist: if an execution behavior, fallback, or mutation path is not described, route-authorized, or explicitly escalated by the framework, it is forbidden by default.
14. **[MUST]** Treat compact/context compression as possible at any moment; persist critical task/routing assumptions through canonical receipts, TODO evidence, or context capsules before risky transitions.
15. **[MUST]** `Thinking mode: ...` is a reporting label only; it must not be used to reveal intermediate chain-of-thought or hidden reasoning steps.
16. **[MUST]** If a protocol/process gap is discovered during active work, use only a bounded workaround for the current task, record the gap through the canonical framework bug path when silent diagnosis is active, and do not silently invent a permanent process.
17. **[MUST]** When evidence sources conflict, prefer the highest-evidence source recognized by the active protocol stack before making conclusions or mutations.
18. **[MUST]** When subagent-first execution is active and new delegated lane allocation fails because of agent/thread saturation, attempt reuse of existing eligible agents first; do not fall back to local-only continuation until reuse or explicit saturation recovery has been attempted and recorded.

Definition note for rule 12:
1. "Generic assistant defaults" means undocumented heuristic behavior such as local-first implementation, ad hoc fallback selection, silent scope expansion, implicit task tracking, or mutation without the active VIDA/project protocol path.

Evidence hierarchy for rule 17:
1. live payload / live request validation,
2. canonical receipt or gate artifact,
3. durable runtime state (`run-graph`, context governance, framework memory, TODO evidence),
4. local code or config inference,
5. chat-level assumption or recollection.

Protocol-gap handling rule:
1. A discovered framework/process gap does not authorize local invention of a new permanent path.
2. The orchestrator may use only the smallest bounded workaround needed to finish the current user-facing task safely.
3. If silent diagnosis is active, record the gap as a framework bug/task before task closure unless an existing tracked bug already covers it.
4. Permanent process changes must return through canonical framework-owned protocol/task flow.

Reporting prefix:
1. Start reports with `Thinking mode: <STC|PR-CoT|MAR|5-SOL|META>.`
2. Do not expose chain-of-thought details.

---

## Boot Sequence

### Hard Stop

After context compression/clearing:
1. Read `AGENTS.md`.
2. Resolve lane:
   - worker lane -> `docs/framework/SUBAGENT-ENTRY.MD`
   - orchestrator lane -> `docs/framework/ORCHESTRATOR-ENTRY.MD`
3. Complete the selected boot path before resuming work.

### Orchestrator Boot Pointer

For orchestrator lane, use `docs/framework/ORCHESTRATOR-ENTRY.MD` as the canonical source for:
1. L0 contract,
2. request-intent gate,
3. TODO engagement gate,
4. subagent-first orchestration,
5. boot profile read-set,
6. runtime execution rules.
7. instruction activation by phase via `docs/framework/instruction-activation-protocol.md`.


### Worker Boot Pointer

For worker lane, use:
1. `docs/framework/SUBAGENT-ENTRY.MD`
2. `docs/framework/SUBAGENT-THINKING.MD`

Workers must not bootstrap repository-wide orchestration policy unless the task packet explicitly asks for framework-lane audit behavior.

---

## Minimal Runtime Rules

1. Use canonical project commands from the active project operations runbook resolved by the project overlay; if no overlay exists, fall back only to framework-owned canonical wrappers and commands declared in `docs/framework/*` or `vida-v0/*`, never to inferred host-project runbooks.
2. Keep temporary artifacts in `_temp/`; large logs in `.vida/scratchpad/`.
3. Prefer sparse, exact, bounded reads over broad context loading.
4. Broad `.vida/logs`, `.vida/state`, or `.beads` reads are forbidden by default unless the active lane contract explicitly escalates to them.

Instruction precedence:
1. `AGENTS.md`
2. lane entry contract (`docs/framework/ORCHESTRATOR-ENTRY.MD` or `docs/framework/SUBAGENT-ENTRY.MD`)
3. canonical protocol for the active domain from `docs/framework/protocol-index.md`
4. project overlay data (`vida.config.yaml`) without weakening framework invariants
5. command doc / helper wrapper
6. script implementation details

Conflict rule:
1. If two sources disagree, obey the highest-precedence source and treat the lower one as drift to be corrected, not as a second valid option.

Operational references:
1. `docs/framework/ORCHESTRATOR-ENTRY.MD`
2. `docs/framework/SUBAGENT-ENTRY.MD`
3. `docs/framework/SUBAGENT-THINKING.MD`
4. `docs/framework/framework-map-protocol.md`
5. `docs/framework/protocol-index.md`
6. `docs/framework/instruction-activation-protocol.md`
