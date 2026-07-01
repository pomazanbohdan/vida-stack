# AGENTS Sidecar

Purpose: provide the project-level agent instruction overlay for the repository being developed on top of the VIDA framework, without moving project-specific operating knowledge into generated `AGENTS.md`.

## Project Instruction Scope

1. Repository: `vida-stack`
2. This sidecar is the project agent-instructions overlay.
3. It may carry project operating rules, local commands, coding/testing/release constraints, project-agent/team conventions, domain constraints, and project-document discovery pointers.
4. The project docs map is a required section of this sidecar, not the sidecar's only purpose.
5. It must not become a second framework owner map or a competing runtime/bootstrap carrier.
6. Framework-owned discovery for active development bootstrap starts from bounded framework carriers referenced by canonical shorthand ids interpreted through `vida protocol view`.
7. It maps the active current project surface only; extracted secondary bundles such as `projects/vida-mobile/**` are out of default bootstrap scope unless the task explicitly targets them.

## Project Canonical Maps

1. Current project root map:
   - `docs/project-root-map.md`
2. Project product index:
   - `docs/product/index.md`
3. Product spec map:
   - `docs/product/spec/current-spec-map.md`
4. Product spec index:
   - `docs/product/spec/index.md`
5. Product spec detailed catalog:
   - `docs/product/spec/current-spec-catalog.md`
6. Project documentation system:
   - `docs/product/spec/project-documentation-law.md`
7. Documentation/product alignment matrix:
   - `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`
8. Canonical runtime readiness law used by the project:
   - `docs/product/spec/canonical-runtime-readiness-law.md`
9. Canonical runtime layer matrix:
   - `docs/product/spec/canonical-runtime-layer-matrix.md`
10. Documentation tooling map:
   - `docs/process/documentation-tooling-map.md`
11. Project agent-extension map:
   - `docs/process/agent-extensions/index.md`
12. Command timing and gate optimization protocol:
   - `docs/process/command-timing-and-gate-optimization-protocol.md`
13. Project Error Search runtime diagnostics protocol:
    - `docs/process/project-error-search-runtime-diagnostics-protocol.md`
14. Agent skill learning protocol:
    - `docs/process/agent-skill-learning-protocol.md`
15. Project-local TaskFlow runtime state and operator surfaces:
    - `.vida/data/state/`
    - `vida taskflow help`
16. Project ZOMBIE-D test-writing protocol:
    - `docs/process/zombie-d-test-writing-protocol.md`
## Bootstrap Read Path

1. After `AGENTS.md`, read this sidecar immediately.
2. Use this sidecar as the project agent-instructions overlay during active development bootstrap.
3. The framework-owned protocol-view/bootstrap-router copy of root `AGENTS.md` used by the binary/runtime protocol-discovery path lives at:
   - `system-maps/bootstrap.router-guide`
4. Framework-owned discovery should continue through bounded framework instruction-home surfaces such as:
   - `system-maps/framework.index`
   - `system-maps/protocol.index`
   - `system-maps/framework.protocol-domains-map`
   - `system-maps/framework.protocol-layers-map`
5. That framework copy must stay synchronized with the stronger live root bootstrap carrier `AGENTS.md`; when they disagree, treat root `AGENTS.md` as authoritative and repair the framework copy in the same change.
6. Continue first to `docs/project-root-map.md` when the task depends on active current-project understanding.
7. Continue into the project canonical maps listed below when the task depends on product/spec understanding.
8. For documentation/product alignment questions, continue to `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`.
9. For documentation tooling or operator-command questions, continue to `docs/process/documentation-tooling-map.md`.
10. For runtime-layering, runtime-readiness, or runtime-architecture questions, continue to `docs/product/spec/canonical-runtime-layer-matrix.md`.
11. For project role/skill/profile/flow extension questions, continue to `docs/process/agent-extensions/index.md`.
12. For command timing, slow gates, script optimization, CI/local proof latency, or operator-friction diagnostics, continue to `docs/process/command-timing-and-gate-optimization-protocol.md`.
13. For runtime defects, multi-defect pools, TaskFlow/DocFlow contradictions, run-graph/recovery/lane/dispatch/receipt blockers, session/worktree ownership conflicts, provider/model/carrier routing blockers, or CI failure clusters, continue to `docs/process/project-error-search-runtime-diagnostics-protocol.md`.
14. For agent skill learning, validation-gated skill updates, rejected skill edits, or protocol-only skill learning in projects without runtime support, continue to `docs/process/agent-skill-learning-protocol.md`.
15. For project-local TaskFlow DB-first execution/bootstrap questions, prefer `vida status --json`, `vida taskflow help`, and the project-owned `.vida/data/state/` runtime store path rather than installed shim or flat task-artifact fallback paths.
16. After bootstrap, prefer the default `vida taskflow ...` shell path with project-local runtime resolution; do not reintroduce installed shim roots that point outside this repository.
17. For project task-shaping, development-team, or delegated execution questions, continue early to `docs/process/team-development-and-orchestration-protocol.md`.
18. For cheaper orchestrator-first project execution, continue early to `docs/process/project-orchestrator-operating-protocol.md`.
19. For repeatable development-session startup, prefer the compact startup bundle:
   - `docs/process/project-orchestrator-startup-bundle.md`
20. Expand to the full session-start protocol and reusable prompt only when the bundle does not settle the startup question:
   - `docs/process/project-orchestrator-session-start-protocol.md`
   - `docs/process/project-orchestrator-reusable-prompt.md`
21. If startup readiness, skill gating, packet rendering, or packet/lane defaults remain unclear after the bundle, expand only the needed compact project runtime capsules:
   - `docs/process/project-start-readiness-runtime-capsule.md`
   - `docs/process/project-packet-rendering-runtime-capsule.md`
   - `docs/process/project-packet-and-lane-runtime-capsule.md`
22. Open deeper owner docs only when those compact project surfaces still leave an edge case unresolved:
   - `docs/process/project-skill-initialization-and-activation-protocol.md`
   - `docs/process/agent-skill-learning-protocol.md`
   - `docs/process/project-development-packet-template-protocol.md`
   - `docs/process/project-agent-prompt-stack-protocol.md`
   - `docs/process/project-orchestrator-session-start-protocol.md`
23. For long-running epic execution, model/cost optimization, per-task
    executor/validator scorecards, wave-first closure, post-task optimization,
    or task-completion checklists, continue early to:
   - `docs/process/project-orchestrator-operating-protocol.md`
   - `docs/process/team-development-and-orchestration-protocol.md`
24. For Rust test, CLI smoke/integration test, fixture/golden test, coverage-gate test, runtime defect proof test, or test-task planning work, continue early to:
   - `docs/process/zombie-d-test-writing-protocol.md`
25. This path set is mandatory bootstrap context, not an optional later lookup.

Project-routing rule:

1. Project/product document pointers belong here, not in framework-owned map/index surfaces addressed by shorthand framework ids.
2. Framework-owned bootstrap may resolve that a downstream target belongs to the project layer, but the concrete project canonical map pointers must be carried by this sidecar.
3. Preserved secondary project bundles are not the default project-doc target for this sidecar; they must be entered only by explicit task targeting.

## Working Rule

1. Use `AGENTS.md` for lane routing and hard invariants.
2. Use this sidecar for project-local agent instructions, project operating rules, and project-document orientation.
3. **MANDATORY STEP BEFORE WRITE-PRODUCING ACTION:** Before every write-producing action (file edit, file create, file delete, config change, code modification, project mutation), first create a DB-backed `step` task through the current TaskFlow command surface:
   - command shape: `vida task create <step-id> "<title>" --type step --status in_progress --parent-id <active-task-id> --description "<what/why/outcome>" --notes "<owner/activeForm/stop>" --json`
   - title — one-line imperative description of the bounded action,
   - description — what will be done, why, and the expected outcome,
   - notes — owner, present-continuous `activeForm`, stop criterion, and immediate fallback when blocked,
   - parent dependency — use `--parent-id <active-task-id>` when the step belongs to the active bounded unit.
4. **EXPLICIT STOP-CRITERION BEFORE EACH STEP:** Before every write-producing move, state explicitly:
   - `STEP N`: what will be done
   - `STOP`: what condition signals completion or a blocker
   - `IF_BLOCKED`: the immediate fallback when the stop-criterion is not met
   - If the stop-criterion cannot be stated, do NOT proceed. Ask the user to clarify the acceptance target before continuing.
5. **NO WRITE WITHOUT STEP:** After creating the step, execute only the single bounded action. Do not chain multiple write-producing actions in one turn without updating the step list. After completing the action, close the step through `vida task close <step-id> --reason "<proof>" --json` and create the next step before the following write-producing move. `todo` remains a deprecated TaskFlow input alias for `step`; do not use it in new project instructions. This prevents the "unbounded action loop" pattern where the model repeats the same action indefinitely without explicit stop conditions.
6. **READ-ONLY EXCEPTION:** This rule does not apply to read-only actions: file reads, code analysis, diagnostic commands, searches, git log, status checks, or any operation that does not modify project files or state.
7. **EXPLICIT RUNTIME-DEFECT BYPASS:** When the user/operator explicitly says VIDA runtime is defective for the current cleanup, planning, or documentation block, do not invent step receipts and do not run mutating VIDA runtime commands. Use bounded static analysis, file proof, script-only checks, and scoped commits. Record any missing TaskFlow/DocFlow evidence as a later runtime-repair follow-up once the runtime is usable.
8. Prefer the project canonical maps here over broad manual repo scanning when the task depends on project/product understanding.
9. Documentation tooling and operator commands are mapped in `docs/process/documentation-tooling-map.md`.
10. For documentation-shaped, spec-shaped, canonical-map, or runtime-law documentation work, activate the documentation tooling path early rather than treating it as a late optional step.
11. The expected early route for such work is:
   - `AGENTS.sidecar.md`
   - `docs/project-root-map.md`
   - `docs/process/documentation-tooling-map.md`
12. When runtime is usable, `vida docflow` is the canonical project-side operator/runtime surface for bounded documentation validation, readiness, relation, and proof work once the relevant project/spec context is known.
13. Do not postpone `vida docflow` usage when runtime is usable and the task already depends on documentation mutation, validation, readiness, or proof-shaped output.
14. For task/backlog lifecycle work, prefer the DB-backed `vida taskflow task` surface over flat task artifacts.
15. The expected local operator path is plain `vida taskflow ...` with project-local defaults already bound to this repository root; manual `VIDA_ROOT=...` overrides are fallback-only.
16. Project-local development routing is intentionally thin after generic runtime protocol promotion:
   - use `docs/process/project-orchestrator-operating-protocol.md` for the vida-stack top-level loop and local read set,
   - use `docs/process/command-timing-and-gate-optimization-protocol.md` for local proof ladder, slow gates, script timing, and CI/non-blocking iteration decisions,
   - use `docs/process/project-error-search-runtime-diagnostics-protocol.md` for VIDA-specific application of the generic `Error Search / Bug Reasoning` algorithm.
17. Generic runtime owner law is not duplicated here. Resolve these owners through `vida protocol view <id>` or the mapped runtime instruction docs when a case needs the full rule:
   - active-unit binding, anti-stop, final-report, and continuation law: `instruction-contracts/core.orchestration-runtime-capsule`, `instruction-contracts/core.orchestration-protocol`, and `runtime-instructions/work.taskflow-protocol`,
   - TaskFlow state, parent/child closure, scheduling, and source-neutral intake: `runtime-instructions/work.taskflow-protocol`,
   - command timing and fast/long gate discipline: `runtime-instructions/work.command-execution-discipline-protocol`,
   - delegated handoff, packet readiness, result synthesis, and host-agent bridge boundaries: `instruction-contracts/lane.worker-dispatch-protocol`, `runtime-instructions/lane.agent-handoff-context-protocol`, and active agent-system/carrier registry owners,
   - release/version/readiness gates: the mapped release and runtime readiness process/spec owners.
18. Project residue that remains valid in this sidecar:
   - active repository paths and canonical docs maps,
   - DB-backed step-before-write command shape and explicit stop criteria,
   - root `AGENTS.md` and `install/assets/AGENTS.scaffold.md` synchronization when either bootstrap carrier changes,
   - current local proof scripts and release/install timing decisions as documented in the command-timing protocol,
   - vida-stack temporary artifact commit guard and project-local worktree-root policy.
19. Role, model, carrier, host CLI, flow, and worktree authority must remain configuration-derived from `vida.config.yaml`, enabled agent-extension registries, and current TaskFlow/runtime state. Concrete values may be recorded only as observed evidence, not as owner law.
20. Historical labels, release labels, and external project names are provenance only unless a current project spec or release task makes them active. Do not add new hardcoded historical or source-project names as runtime authority.
21. Temporary artifact commit guard: the orchestrator must never commit scratch output, advisory drafts, logs, generated local release/package directories, caches, temp folders, sentinel files, or session-only analysis artifacts such as root-level `tmp*`, `tmp/`, `_temp/`, `temp/`, `logs/`, `dist/`, `target/`, root-level `false`, `true`, `null`, `undefined`, `nul`, `*.tmp`, `*.temp`, `*.log`, `*.bak`, `*.swp`, or `*.pid`. Before every commit, run a tracked-temp scan (`git ls-files` filtered for these paths/patterns, including `git ls-files tmp* false true null undefined nul`) and treat any match outside an explicitly documented, product-owned fixture/artifact path as a repo-hygiene defect to remove before committing.

## Epic Execution Optimization Overlay

1. Long-running refactor epics use a wave-first strategy: choose the wave with the smallest verified closure distance, finish its open children, then close the wave parent before selecting unrelated leaf work.
2. Closure distance must consider open child count, blocked child count, proof gaps, dirty-file overlap, release/install cost, PR/GitHub coupling, and validator residual risk.
3. Every task uses the three-step loop: `Bind -> Delegate -> Close`.
4. `Bind` must record the active task id, parent/wave, exact invariant, owned paths, non-goals, proof bundle, dirty-worktree boundaries, and whether the next move is sequential or parallel-safe.
5. `Delegate` must prefer the cheapest capable executor and the smallest prompt that can satisfy the task. For the currently observed epic pattern, use the cheap mini/highest-reasoning carrier for hunk classification, read-only preflight, source-copy documentation, test-only patches, and one-file implementation packets; use the stronger medium validator for TaskFlow, host-bridge, receipt authority, path policy, public CLI, release, or wave-closure gates.
6. `Close` must run the declared proof bundle, update TaskFlow, close the task when ready, commit only scoped files, push under the active repeatable publication instruction, record the optimization scorecard in TaskFlow closure evidence or the relevant process doc, and then check parent/wave closure readiness.
7. A current explicit operator instruction to commit and push after each task is a repeatable publication authorization for the active epic until the operator pauses, revokes, or narrows it. This resolves lower-level "push only when authorized" rules for this active epic without creating global push authority for unrelated work.
8. In dirty files, stage by invariant rather than by file. If an adjacent hunk looks useful but is outside the active bounded unit, leave it unstaged and create/update a follow-up TaskFlow item instead of bundling it into the commit.
9. After every closed task, record a compact optimization scorecard in the TaskFlow task note or closure evidence: executor, validator, reasoning effort, score, token visibility, tool-call count, proof quality, rework count, residual risks, and the next routing rule.
10. After every closed task, run the canonical Post-Task Self-Analysis STOP gate from `docs/process/project-orchestrator-operating-protocol.md` before selecting unrelated work.
11. The detailed post-task checklist and scorecard field ownership live in `docs/process/project-orchestrator-operating-protocol.md`; this sidecar only requires using that owner surface before next-task selection.
12. Do not optimize for leaf percentage alone. The primary epic milestone metric is closed waves over total waves; task percent is secondary evidence.
13. If a task closes an architectural/process slice or a wave, run the runtime self-diagnostic and release/install the system `vida` binary before treating the slice or wave as operationally closed. Any actionable residual from that diagnostic must become a TaskFlow implementation task, an update to an existing TaskFlow task, or an explicit `no_task_reason` cited in the closure evidence; a prose-only diagnostic residual is not closure.
14. Parallelism is allowed only for disjoint read-only scans, PR intake, proof-gap review, and dirty-hunk classification. Production edits in the same Rust file, TaskFlow mutation, DocFlow owner docs, release install, and GitHub mutations stay sequential unless TaskFlow conflict domains prove otherwise.
15. User-facing progress after each task should be compact: closed task, proof, commits pushed, task percent, wave count, self-analysis outcome, next bounded unit or blocker.
16. These optimization rules are top-level project overlay. Detailed prompt wording, lane responsibilities, and scorecard criteria live in `docs/process/project-orchestrator-operating-protocol.md` and `docs/process/team-development-and-orchestration-protocol.md`.

## Runtime DX And Architectural Rule Overlay

1. Batch-first operator/runtime task execution: when the active epic contains multiple small, adjacent, non-conflicting operator-surface or runtime-DX tasks, prefer taking them as a bounded batch instead of one release cycle per task. Create/mark the TaskFlow task and step for each included slice before its write-producing mutation, implement all selected slices, then run one consolidated debug/test cycle, one release install when an installed CLI proof is required, one graph/doc/hygiene check, one commit for the batch, and push under the active explicit publication authorization for that task, wave, publication batch, or repeatable publication pattern. The current epic's operator instruction to push after each task is such a repeatable publication pattern; outside that explicit active pattern, do not infer push authorization from task closure, wave closure, or clean commit alone. After a wave parent closes, the installed system `vida` binary must be updated from a release build and smoke-checked through the normal PATH before the wave is treated as operationally closed. Do not batch tasks whose owned paths, runtime state, data migrations, security boundaries, or acceptance criteria conflict; those stay sequential.
2. Recurring session-blocker priority and build-proof rule: when the same runtime defect, blocker code, command-surface failure, lane/receipt/dispatch defect, or session-blocking symptom appears more than once in the current session, raise or keep its owning TaskFlow item at priority 1, append the recurrence evidence, and route it ahead of unrelated runtime-DX cleanup unless a higher-severity blocker is already active. When such a defect blocked the current session, continuation, lane execution, receipt/proof truth, or installed CLI behavior, its closure proof must include `vida release install --json` and an installed-binary smoke check after the focused fix proof, unless the current explicit operator policy already requires a system build after every task. If release install cannot run, keep closure blocked or record a proof blocker rather than treating the task as operationally closed.
3. VIDA operator latency defect rule: lightweight VIDA operator commands used inside the orchestration loop (`task create/update/show/progress/close`, `taskflow graph-summary/status`, `orchestrator-init`, and similar non-build/non-release surfaces) have a fast-path target of <=2 seconds. If a lightweight command exceeds that target, emits huge default payloads, refreshes unrelated projections, or performs heavy closeout/release work without an explicit opt-in flag, classify it as a runtime architecture defect and create/update the appropriate performance task. Batch execution reduces repeated slow calls, but it must not normalize or hide slow operator surfaces.
4. Minimal default output rule: default VIDA command output must include only the minimal current actionable data needed for the operator decision, with the minimal number of state reads/projections required to compute that data. Human-facing default output, especially for record lists and compact summaries, should use the canonical TOON-style compact format; JSON record arrays and broad machine-readable records are explicit `--json`/details-mode output, not the default. Uniform record lists in default TOON output must declare the row count and column schema before row values with the canonical tabular-array shape such as `tasks[3]{id,status,priority,title}:`; handwritten pipe-separated bullet rows without a `{columns}` header are a runtime-DX defect unless the list is genuinely non-uniform and must fall back to TOON list form. Prefer the existing `toon-format` crate and shared `common-format-toon`/`taskflow-format-toon` helpers over ad hoc string renderers. Wider output, recursive summaries, full task/epic lists, historical receipts, global graph projections, expensive proof bundles, and release/build side effects must require an explicit option such as `--details`, `--all`, `--full`, `--include-global`, or a separate closeout/diagnostic command. Operator recipes, default next-action text, remediation hints, and human-facing command suggestions should point to default commands without `--json`; machine-readable JSON remains an explicit option documented in usage/options sections. Every command-output change must add or update public-surface proof for both the default TOON output and the explicit JSON output, plus `--help` coverage that names the default compact output and the JSON option. When a default command emits broad JSON-like records, raw tabular lists, schema-less compact rows, `--json`-biased next-action guidance, lacks JSON parity, omits output-mode help, or returns stale context instead of compact current TOON/operator evidence, record it as a runtime-DX/performance defect and route it into the active refactor backlog.
5. TOON/output proof preservation rule: when validating textual output shape, especially TOON-style compact output, capture the command output losslessly with raw shell output or redirect it to an artifact and read that artifact exactly. Do not treat lean-ctx compressed summaries as proof of formatting, line structure, indentation, item limits, or absence of broad records; compressed summaries are status hints only.
6. Missing required CLI option rule: if an operator, agent, repair, proof, closeout, task-selection, or output-standardization workflow needs a command option and that option is absent, hidden from `--help`, undocumented, rejected by the parser, lacks a machine-readable equivalent, or forces manual JSON/shell workarounds, classify it as a VIDA runtime defect. Create or update a focused TaskFlow defect, require `--help` description coverage, and add integration proof that the option is accepted, described, and returns the standardized compact/default or JSON output contract as appropriate.
7. Runtime architectural-fix rule: for runtime or TaskFlow defects, prefer architectural contract changes over narrow symptom fixes. Before implementation, perform and record a shared/deduplication research gate: inspect whether the invariant, rendering, verdict, repair action, fixture builder, integration harness, command-output schema, CLI help/options, persisted-state adapter, or next-action generation is duplicated across surfaces; decide whether it belongs in a shared helper, shared contract module, shared renderer, shared verdict model, shared fixture builder, shared integration harness, or smaller decomposed module; and name the files/callers that must move to that boundary. This gate is mandatory even when the reported defect names one file or one command. If a defect affects multiple surfaces, receipts, JSON fields, operator workflows, command outputs, or persisted evidence, move the invariant into a named helper or contract boundary and make all affected surfaces use it. If no shared boundary is introduced, the task note must explain why the behavior is intentionally isolated.
8. Invariant-completion rule: do not leave the corrected invariant implicit in adjacent code. A bounded fix should remove the duplicated or fragile behavior that allowed the defect, not only patch the first failing line. The implementation is incomplete until adjacent duplicated branches, stale local renderers, stale option/help text, old fixture builders, obsolete snapshots, and tests that preserve old duplicate behavior are rewritten, deleted, or explicitly justified as compatibility proof. When a shared boundary is introduced, update all in-scope call sites and tests in the same task unless the task note names a blocking dependency and a follow-up owned by the same epic.
9. No-legacy architecture rule: new architectural refactor work must make the new shared boundary canonical instead of adding compatibility wrappers, duplicate old/new paths, legacy modes, legacy-named modules/functions, or "legacy" aliases by default. New architecture is the product contract once introduced; do not keep a parallel old implementation path for convenience, rollout hedging, or test preservation. A temporary compatibility bridge is allowed only when a bounded migration task explicitly names the old callers, stop date/stop condition, proof that new callers use the canonical boundary, and a follow-up removal path. If the old behavior is duplicated only to keep tests passing, remove it and update the tests to the new contract instead.
10. Integration-test batch rule: for integration-test work, plan the full test batch for the target file or public surface before broad verification. While shaping the batch, run focused tests only; run broader or full suites after the planned batch is complete.
11. Integration-test variance rule: prefer smaller, varied integration tests over one oversized scenario. Cover meaningful variants such as happy path, blocked gate, persisted snapshot parity, command-output parity, recovery/next-action guidance, and cross-surface consistency.
12. Focused-defect discovery rule: if a focused test exposes a production defect while the batch is still incomplete, fix the production contract and continue completing the planned batch before broad/full verification.
13. Public-surface proof rule: architectural defects require proof at the contract boundary and through every affected public CLI/operator surface family in the bounded scope; unit-only proof is insufficient for closing a runtime behavior defect. The proof plan must include default compact TOON output where applicable, explicit JSON output, `--help`/option descriptions, fail-closed blocker shape, and rewritten integration tests for old behavior that should now be owned by the shared boundary.
## Defective Runtime Emulation Overlay

Generic defective-runtime recovery law is owned by the orchestration, TaskFlow, command-execution, lane handoff, and runtime diagnostic protocols. This sidecar keeps only vida-stack residue:

1. Prefer normal `vida orchestrator-init`, TaskFlow, run-graph, lane, dispatch, `vida agent-init`, `vida doctor`, and DocFlow surfaces before emulation.
2. If one of those surfaces is itself defective, record the blocker in TaskFlow when writable and keep `active_bounded_unit`, `why_this_unit`, and `sequential_vs_parallel_posture` explicit.
3. Use `docs/process/project-error-search-runtime-diagnostics-protocol.md` for VIDA command/blocker evidence and promote reusable repair rules only through the mapped generic runtime owner protocols.
4. Keep local emulation bounded to diagnosis, shaping, or the smallest active recovery unit needed to restore the canonical runtime path.
5. Never invent receipts, delete runtime state by hand, or treat advisory output as write authority.
6. Exit emulation when canonical surfaces can again select, dispatch, continue, recover, diagnose, and close the active bounded unit with receipt-backed evidence.

## Project-Local Advisory Fallback Overlay

Generic advisory-lane authority and handoff boundaries are owned by the agent-system and lane handoff protocols. vida-stack keeps these local operating residues:

1. Advisory lanes are read-only support for research, context, review, reproduction reasoning, proof suggestions, and junior drafts.
2. Advisory prompts must name a bounded question, output artifact path, markdown schema, success/failure criteria, and scope limit.
3. Store advisory outputs under `tmp/advisory-*.md` or another intentional project artifact path selected by the parent.
4. Validate every advisory artifact as non-empty, schema-valid, scoped, evidence-backed, and useful before using it.
5. Classify each advisory result as accepted evidence, partial evidence, conflict, content failure, process failure, or irrelevant/stale.
6. Launch rolling read-only advisory prefetch only when useful non-overlapping future work exists; do not launch advisory lanes for obvious single-command checks.
7. Advisory children must not edit files, mutate `.vida/data/state`, record receipts, mutate TaskFlow, or decide closure.
8. Host-agent ids, carrier names, model refs, reasoning effort, and host CLI systems must be resolved from config/registry/runtime evidence rather than hardcoded in prompts or docs.
9. Completed advisory or host-subagent handles must be closed or deleted in the same orchestration step after the orchestrator syncs the result and classifies it as accepted, partial, conflict, failed, irrelevant, or stale. A completed handle is not an idle resource: do not launch the next agent, start local implementation, or continue the research ring while a classified completed handle remains open. If the host exposes both close and delete/remove semantics, use the strongest available cleanup operation that preserves the result artifact or transcript already captured by the orchestrator. If cleanup fails, record the handle id and blocker in the active TaskFlow note before launching replacement agents.
10. After the orchestrator fixes the root cause and adjacent in-task defects, a read-only advisory agent pattern sweep is mandatory before task closure. The sweep must search for the same defect pattern across related files, shared helpers, command surfaces, tests, fixtures, persisted-state snapshots, operator outputs, and next-action text. The orchestrator must classify the agent result, record it in the active task note, and create or update a follow-up TaskFlow task with acceptance criteria when the pattern exists outside the bounded fix. If no wider pattern is found, record explicit negative evidence before closure.
## Complex And Architectural Processing Contract

Use this project-local sidecar contract when a user request or discovered defect requires complex, comprehensive, architectural, root-cause, end-to-end, or cross-module remediation. It also applies when a change touches API contracts, VIDA framework behavior, shared UI surfaces, auth/session behavior, routing, persistence, localization, design-system ownership, or user-visible workflow evidence.

This contract is a project overlay for `vida-stack`. It does not replace framework owner law, root `AGENTS.md`, TaskFlow lane authority, DocFlow ownership, or canonical framework protocol surfaces. When deeper framework rules are needed, resolve them through the bounded VIDA runtime surfaces and `vida protocol view <id>`.

Hotfixes are not an allowed delivery mode for this project. If a defect is urgent, narrow the bounded architectural remediation scope and execute that scope with the required investigation, design decision, implementation, and verification. Do not deliver a temporary or symptom-only change as a separate result.

Complex and architectural processing means all of the following:

1. Study the full related code path before proposing or applying a fix: callers, callees, adapters, providers, state transitions, UI surfaces, tests, generated artifacts, scripts, and documentation that can affect the bounded behavior.
2. Resolve ambiguity before implementation. If related code, runtime state, API behavior, or ownership boundaries leave open questions, continue bounded investigation or report an explicit blocker instead of guessing.
3. Study API contracts with direct evidence when API behavior matters. Prefer safe read-only direct API requests against configured targets; mutating API probes must use the existing live-mutation E2E contract, cleanup rules, and explicit environment safeguards.
4. Check authoritative online documentation for version-sensitive APIs, frameworks, libraries, widgets, and platform behavior when local source or tests do not fully define the contract. Prefer official/versioned docs and record the source/date in the design or handoff evidence when the decision depends on that external contract.
5. For registered defects, run a freshness audit before implementation. Live runtime evidence is authoritative over stale issue text, old TaskFlow notes, old comments, old branch results, and old agent reports. Classify each registered symptom as `actual_now`, `partially_fixed`, `superseded`, `merged_into_broader_invariant`, or `stale_not_reproduced` before selecting the write scope.
6. When multiple defects share one ownership layer, fix the shared architectural invariant instead of closing them one-by-one with local symptom patches. The remediation decision must name the canonical owner boundary, shared helper/contract surface, data-flow invariant, affected public surfaces, and which old defect reports become obsolete after the broader fix.
7. Before choosing the implementation seam, explicitly research whether duplicated logic should move into a shared helper, shared contract module, shared renderer, shared verdict model, shared fixture builder, shared integration harness, command-output schema owner, CLI option/help owner, or a smaller decomposed module. This check is mandatory for architectural fixes, even when the initial defect appears to be in one file. If the same invariant is computed by more than one surface, the default architectural answer is a shared boundary unless direct evidence shows the computations are intentionally different. The research note must include a `shared_boundary_decision` of `introduce`, `extend`, `reuse`, or `not_applicable`, with the evidence for that choice.
8. Form an architectural remediation decision before write-producing work. The decision must name the accepted seam, ownership layer, data flow, shared/deduplicated boundaries, files/functions to decompose, command/help/output implications, compatibility constraints, rejected alternatives, minimal bounded write scope, stale-defect handling plan, and the existing tests that must be rewritten or deleted because they encode old duplicated behavior. It must also state whether default TOON output, explicit JSON output, help/options, persisted-state fixtures, and cross-surface parity are in scope for the changed command family.
9. Verify planned-fix impact before editing by reading related code that can regress: shared helpers, generated sources, call sites, platform variants, existing tests, and user-flow documentation.
10. Apply the fix comprehensively across the bounded architecture instead of patching one symptom, one widget, one locale, one adapter, or one platform in isolation.
11. Update proof with the implementation: unit tests for pure contracts and state, UI/layout tests when a user-facing surface changes, and end-to-end checks for changed navigation, auth/session, storage, proxy/device, or external-service behavior.
12. Rewrite existing tests when they encode obsolete architecture, duplicated behavior, stale assumptions, or symptom-specific expectations that should now be owned by the shared invariant. This includes tests in the currently touched file, adjacent smoke/integration files, CLI help/output assertions, JSON parity checks, persisted-state fixtures, generated snapshots, and old fixture builders that preserve the former layout. Do not leave old narrow tests green by preserving legacy behavior in parallel. A test rewrite is part of the architectural fix, not a follow-up cleanup, when the old test would force duplicated production code to remain.
13. Update the human user flow whenever user-visible behavior changes. Keep the mapped user-flow documentation and end-to-end evidence aligned with the implementation, or record a specific rationale when the change is intentionally unit-only or process-only.

Verification checklist for complex or architectural work:

1. The related code map is complete enough that no known caller, adapter, provider, generated path, platform path, or user-flow surface is left unreviewed.
2. API behavior is backed by direct request evidence, or by an explicit reason why a direct request is unsafe or impossible in the current environment.
3. External framework/library/widget facts are checked against authoritative online documentation when local code is not sufficient.
4. Registered defects have a current-state freshness matrix before implementation starts: live reproduction command, live result, owning surface, status classification, and whether the old report is still actionable.
5. The architectural decision and impact map are written into the task, design, or handoff evidence before implementation starts, including the shared/deduplication decision, affected tests to rewrite/delete/consolidate, affected command-output/help/JSON/TOON surfaces, and the reason if no shared boundary is introduced.
6. Implementation updates every affected bounded surface and does not leave parallel legacy behavior as an accidental active path.
7. Existing tests that covered the old symptom are reviewed and either rewritten around the shared invariant or explicitly retained as compatibility proof with rationale.
8. Unit/widget coverage and the 80% local coverage target remain protected; new tests cover behavior and architectural seams rather than shallow invocations. For runtime architectural remediation, the target is 100% coverage of the changed behavior path: success, fail-closed blocker, default compact output, explicit JSON output, help/options, next-action guidance, persisted-state fixture, and cross-surface parity.
9. Web and Android E2E flow coverage is added or updated for changed user-visible workflows, with shared flow helpers used where practical.
10. User-flow documentation is updated before closure for every changed user-visible workflow.
11. Every defect discovered during this checklist is classified by severity, ownership, architectural area, and whether it is likely eliminated by the shared-boundary rewrite. Defects inside the same bounded architectural area enter the current remediation queue when they do not materially change ownership or risk; other defects become explicit next tasks/follow-ups under this same complex-processing contract, not informal notes.
12. Closure of a complex or architectural task is blocked until the post-root-cause advisory pattern sweep from the advisory-lane overlay is complete, classified, and reflected in TaskFlow notes or follow-up tasks.

-----
artifact_path: project/repository/agents.sidecar
artifact_type: bootstrap_doc
artifact_version: '1'
artifact_revision: '2026-06-12'
schema_version: '1'
status: canonical
source_path: AGENTS.sidecar.md
created_at: '2026-03-10T02:13:40+02:00'
updated_at: 2026-06-13T00:00:00+03:00
changelog_ref: AGENTS.sidecar.changelog.jsonl
