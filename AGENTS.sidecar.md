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
4. Product spec provenance companion:
   - `docs/product/spec/current-spec-provenance-map.md`
5. Project documentation system:
   - `docs/product/spec/project-documentation-law.md`
6. Documentation/product alignment matrix:
   - `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`
7. Canonical runtime readiness law used by the project:
   - `docs/product/spec/canonical-runtime-readiness-law.md`
8. Canonical runtime layer matrix:
   - `docs/product/spec/canonical-runtime-layer-matrix.md`
9. Documentation tooling map:
   - `docs/process/documentation-tooling-map.md`
10. Project agent-extension map:
   - `docs/process/agent-extensions/README.md`
11. Command timing and gate optimization protocol:
   - `docs/process/command-timing-and-gate-optimization-protocol.md`
12. Project Error Search runtime diagnostics protocol:
    - `docs/process/project-error-search-runtime-diagnostics-protocol.md`
13. Generic runtime protocol promotion plan:
    - `docs/process/generic-runtime-protocol-promotion-plan.md`
14. Project-local TaskFlow runtime state and operator surfaces:
    - `.vida/data/state/`
    - `vida taskflow help`
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
11. For project role/skill/profile/flow extension questions, continue to `docs/process/agent-extensions/README.md`.
12. For command timing, slow gates, script optimization, CI/local proof latency, or operator-friction diagnostics, continue to `docs/process/command-timing-and-gate-optimization-protocol.md`.
13. For runtime defects, multi-defect pools, TaskFlow/DocFlow contradictions, run-graph/recovery/lane/dispatch/receipt blockers, session/worktree ownership conflicts, provider/model/carrier routing blockers, or CI failure clusters, continue to `docs/process/project-error-search-runtime-diagnostics-protocol.md`.
14. For project-side workflow rules that may belong in generic runtime protocols, continue to `docs/process/generic-runtime-protocol-promotion-plan.md` before editing framework owner instructions.
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
   - `docs/process/project-development-packet-template-protocol.md`
   - `docs/process/project-agent-prompt-stack-protocol.md`
   - `docs/process/project-boot-readiness-validation-protocol.md`
22. This path set is mandatory bootstrap context, not an optional later lookup.

Project-routing rule:

1. Project/product document pointers belong here, not in framework-owned map/index surfaces addressed by shorthand framework ids.
2. Framework-owned bootstrap may resolve that a downstream target belongs to the project layer, but the concrete project canonical map pointers must be carried by this sidecar.
3. Preserved secondary project bundles are not the default project-doc target for this sidecar; they must be entered only by explicit task targeting.

## Working Rule

1. Use `AGENTS.md` for lane routing and hard invariants.
2. Use this sidecar for project-local agent instructions, project operating rules, and project-document orientation.
3. **MANDATORY TODO BEFORE WRITE-PRODUCING ACTION:** Before every write-producing action (file edit, file create, file delete, config change, code modification, project mutation), first create a DB-backed `todo` task through the current TaskFlow command surface:
   - command shape: `vida task create <todo-id> "<title>" --type todo --status in_progress --parent-id <active-task-id> --description "<what/why/outcome>" --notes "<owner/activeForm/stop>" --json`
   - title — one-line imperative description of the bounded action,
   - description — what will be done, why, and the expected outcome,
   - notes — owner, present-continuous `activeForm`, stop criterion, and immediate fallback when blocked,
   - parent dependency — use `--parent-id <active-task-id>` when the todo belongs to the active bounded unit.
4. **EXPLICIT STOP-CRITERION BEFORE EACH STEP:** Before every write-producing move, state explicitly:
   - `STEP N`: what will be done
   - `STOP`: what condition signals completion or a blocker
   - `IF_BLOCKED`: the immediate fallback when the stop-criterion is not met
   - If the stop-criterion cannot be stated, do NOT proceed. Ask the user to clarify the acceptance target before continuing.
5. **NO WRITE WITHOUT TODO:** After creating the todo, execute only the single bounded action. Do not chain multiple write-producing actions in one turn without updating the todo list. After completing the action, close the todo through `vida task close <todo-id> --reason "<proof>" --json` and create the next todo before the following write-producing move. This prevents the "unbounded action loop" pattern where the model repeats the same action indefinitely without explicit stop conditions.
6. **READ-ONLY EXCEPTION:** This rule does not apply to read-only actions: file reads, code analysis, diagnostic commands, searches, git log, status checks, or any operation that does not modify project files or state.
3. Prefer the project canonical maps here over broad manual repo scanning when the task depends on project/product understanding.
4. Documentation tooling and operator commands are mapped in `docs/process/documentation-tooling-map.md`.
5. For documentation-shaped, spec-shaped, canonical-map, or runtime-law documentation work, activate the documentation tooling path early rather than treating it as a late optional step.
6. The expected early route for such work is:
   - `AGENTS.sidecar.md`
   - `docs/project-root-map.md`
   - `docs/process/documentation-tooling-map.md`
7. `vida docflow` is the canonical project-side operator/runtime surface for bounded documentation validation, readiness, relation, and proof work once the relevant project/spec context is known.
8. Do not postpone `vida docflow` usage until after broad manual documentation work when the task already depends on documentation mutation, validation, readiness, or proof-shaped output.
9. For task/backlog lifecycle work, prefer the DB-backed `vida taskflow task` surface over flat task artifacts.
10. The expected local operator path is plain `vida taskflow ...` with project-local defaults already bound to this repository root; manual `VIDA_ROOT=...` overrides are fallback-only.
11. Project-local development routing is intentionally thin after generic runtime protocol promotion:
   - use `docs/process/project-orchestrator-operating-protocol.md` for the vida-stack top-level loop and local read set,
   - use `docs/process/generic-runtime-protocol-promotion-plan.md` to decide whether a reusable rule belongs in generic runtime owners or remains local residue,
   - use `docs/process/command-timing-and-gate-optimization-protocol.md` for local proof ladder, slow gates, script timing, and CI/non-blocking iteration decisions,
   - use `docs/process/project-error-search-runtime-diagnostics-protocol.md` for VIDA-specific application of the generic `Error Search / Bug Reasoning` algorithm.
12. Generic runtime owner law is not duplicated here. Resolve these owners through `vida protocol view <id>` or the mapped runtime instruction docs when a case needs the full rule:
   - active-unit binding, anti-stop, final-report, and continuation law: `instruction-contracts/core.orchestration-runtime-capsule`, `instruction-contracts/core.orchestration-protocol`, and `runtime-instructions/work.taskflow-protocol`,
   - TaskFlow state, parent/child closure, scheduling, and source-neutral intake: `runtime-instructions/work.taskflow-protocol`,
   - command timing and fast/long gate discipline: `runtime-instructions/work.command-execution-discipline-protocol`,
   - delegated handoff, packet readiness, result synthesis, and host-agent bridge boundaries: `instruction-contracts/lane.worker-dispatch-protocol`, `runtime-instructions/lane.agent-handoff-context-protocol`, and active agent-system/carrier registry owners,
   - release/version/readiness gates: the mapped release and runtime readiness process/spec owners.
13. Project residue that remains valid in this sidecar:
   - active repository paths and canonical docs maps,
   - DB-backed TODO-before-write command shape and explicit stop criteria,
   - root `AGENTS.md` and `install/assets/AGENTS.scaffold.md` synchronization when either bootstrap carrier changes,
   - current local proof scripts and release/install timing decisions as documented in the command-timing protocol,
   - vida-stack temporary artifact commit guard and project-local worktree-root policy.
14. Role, model, carrier, host CLI, flow, and worktree authority must remain configuration-derived from `vida.config.yaml`, enabled agent-extension registries, and current TaskFlow/runtime state. Concrete values may be recorded only as observed evidence, not as owner law.
15. Historical labels, release labels, and external project names are provenance only unless a current project spec or release task makes them active. Do not add new hardcoded historical or source-project names as runtime authority.
16. Temporary artifact commit guard: the orchestrator must never commit scratch output, advisory drafts, logs, generated local release/package directories, caches, temp folders, sentinel files, or session-only analysis artifacts such as root-level `tmp*`, `tmp/`, `_temp/`, `temp/`, `logs/`, `dist/`, `target/`, root-level `false`, `true`, `null`, `undefined`, `nul`, `*.tmp`, `*.temp`, `*.log`, `*.bak`, `*.swp`, or `*.pid`. Before every commit, run a tracked-temp scan (`git ls-files` filtered for these paths/patterns, including `git ls-files tmp* false true null undefined nul`) and treat any match outside an explicitly documented, product-owned fixture/artifact path as a repo-hygiene defect to remove before committing.

17. Batch-first operator/runtime task execution: when the active epic contains multiple small, adjacent, non-conflicting operator-surface or runtime-DX tasks, prefer taking them as a bounded batch instead of one release cycle per task. Create/mark the TaskFlow task and TODO for each included slice before its write-producing mutation, implement all selected slices, then run one consolidated debug/test cycle, one release install when an installed CLI proof is required, one graph/doc/hygiene check, and one commit/push for the batch. Do not batch tasks whose owned paths, runtime state, data migrations, security boundaries, or acceptance criteria conflict; those stay sequential.
18. VIDA operator latency defect rule: lightweight VIDA operator commands used inside the orchestration loop (`task create/update/show/progress/close`, `taskflow graph-summary/status`, `orchestrator-init`, and similar non-build/non-release surfaces) have a fast-path target of <=2 seconds. If a lightweight command exceeds that target, emits huge default payloads, refreshes unrelated projections, or performs heavy closeout/release work without an explicit opt-in flag, classify it as a runtime architecture defect and create/update the appropriate performance task. Batch execution reduces repeated slow calls, but it must not normalize or hide slow operator surfaces.
19. Minimal default output rule: default VIDA command output must include only the minimal current actionable data needed for the operator decision, with the minimal number of state reads/projections required to compute that data. Human-facing default output, especially for record lists and compact summaries, should use the canonical TOON-style compact format; JSON record arrays and broad machine-readable records are explicit `--json`/details-mode output, not the default. Uniform record lists in default TOON output must declare the row count and column schema before row values with the canonical tabular-array shape such as `tasks[3]{id,status,priority,title}:`; handwritten pipe-separated bullet rows without a `{columns}` header are a runtime-DX defect unless the list is genuinely non-uniform and must fall back to TOON list form. Prefer the existing `toon-format` crate and shared `common-format-toon`/`taskflow-format-toon` helpers over ad hoc string renderers. Wider output, recursive summaries, full task/epic lists, historical receipts, global graph projections, expensive proof bundles, and release/build side effects must require an explicit option such as `--details`, `--all`, `--full`, `--include-global`, or a separate closeout/diagnostic command. When a default command emits broad JSON-like records, raw tabular lists, schema-less compact rows, or stale context instead of compact current TOON/operator evidence, record it as a runtime-DX/performance defect and route it into the active refactor backlog.
20. TOON/output proof preservation rule: when validating textual output shape, especially TOON-style compact output, capture the command output losslessly with raw shell output or redirect it to an artifact and read that artifact exactly. Do not treat lean-ctx compressed summaries as proof of formatting, line structure, indentation, item limits, or absence of broad records; compressed summaries are status hints only.
21. Missing required CLI option rule: if an operator, agent, repair, proof, closeout, task-selection, or output-standardization workflow needs a command option and that option is absent, hidden from `--help`, undocumented, rejected by the parser, lacks a machine-readable equivalent, or forces manual JSON/shell workarounds, classify it as a VIDA runtime defect. Create or update a focused TaskFlow defect, require `--help` description coverage, and add integration proof that the option is accepted, described, and returns the standardized compact/default or JSON output contract as appropriate.
## Defective Runtime Emulation Overlay

Generic defective-runtime recovery law is owned by the orchestration, TaskFlow, command-execution, lane handoff, and runtime diagnostic protocols. This sidecar keeps only vida-stack residue:

1. Prefer normal `vida orchestrator-init`, TaskFlow, run-graph, lane, dispatch, `vida agent-init`, `vida doctor`, and DocFlow surfaces before emulation.
2. If one of those surfaces is itself defective, record the blocker in TaskFlow when writable and keep `active_bounded_unit`, `why_this_unit`, and `sequential_vs_parallel_posture` explicit.
3. Use `docs/process/project-error-search-runtime-diagnostics-protocol.md` for VIDA command/blocker evidence and `docs/process/generic-runtime-protocol-promotion-plan.md` before promoting any reusable repair rule.
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
## Complex And Architectural Processing Contract

Use this project-local sidecar contract when a user request or discovered defect requires complex, comprehensive, architectural, root-cause, end-to-end, or cross-module remediation. It also applies when a change touches API contracts, VIDA framework behavior, shared UI surfaces, auth/session behavior, routing, persistence, localization, design-system ownership, or user-visible workflow evidence.

This contract is a project overlay for `vida-stack`. It does not replace framework owner law, root `AGENTS.md`, TaskFlow lane authority, DocFlow ownership, or canonical framework protocol surfaces. When deeper framework rules are needed, resolve them through the bounded VIDA runtime surfaces and `vida protocol view <id>`.

Hotfixes are not an allowed delivery mode for this project. If a defect is urgent, narrow the bounded architectural remediation scope and execute that scope with the required investigation, design decision, implementation, and verification. Do not deliver a temporary or symptom-only change as a separate result.

Complex and architectural processing means all of the following:

1. Study the full related code path before proposing or applying a fix: callers, callees, adapters, providers, state transitions, UI surfaces, tests, generated artifacts, scripts, and documentation that can affect the bounded behavior.
2. Resolve ambiguity before implementation. If related code, runtime state, API behavior, or ownership boundaries leave open questions, continue bounded investigation or report an explicit blocker instead of guessing.
3. Study API contracts with direct evidence when API behavior matters. Prefer safe read-only direct API requests against configured targets; mutating API probes must use the existing live-mutation E2E contract, cleanup rules, and explicit environment safeguards.
4. Check authoritative online documentation for version-sensitive APIs, frameworks, libraries, widgets, and platform behavior when local source or tests do not fully define the contract. Prefer official/versioned docs and record the source/date in the design or handoff evidence when the decision depends on that external contract.
5. Form an architectural remediation decision before write-producing work. The decision must name the accepted seam, ownership layer, data flow, compatibility constraints, rejected alternatives, and minimal bounded write scope.
6. Verify planned-fix impact before editing by reading related code that can regress: shared helpers, generated sources, call sites, platform variants, existing tests, and user-flow documentation.
7. Apply the fix comprehensively across the bounded architecture instead of patching one symptom, one widget, one locale, one adapter, or one platform in isolation.
8. Update proof with the implementation: unit tests for pure contracts and state, UI/layout tests when a user-facing surface changes, and end-to-end checks for changed navigation, auth/session, storage, proxy/device, or external-service behavior.
9. Update the human user flow whenever user-visible behavior changes. Keep the mapped user-flow documentation and end-to-end evidence aligned with the implementation, or record a specific rationale when the change is intentionally unit-only or process-only.

Verification checklist for complex or architectural work:

1. The related code map is complete enough that no known caller, adapter, provider, generated path, platform path, or user-flow surface is left unreviewed.
2. API behavior is backed by direct request evidence, or by an explicit reason why a direct request is unsafe or impossible in the current environment.
3. External framework/library/widget facts are checked against authoritative online documentation when local code is not sufficient.
4. The architectural decision and impact map are written into the task, design, or handoff evidence before implementation starts.
5. Implementation updates every affected bounded surface and does not leave parallel legacy behavior as an accidental active path.
6. Unit/widget coverage and the 80% local coverage target remain protected; new tests cover behavior and architectural seams rather than shallow invocations.
7. Web and Android E2E flow coverage is added or updated for changed user-visible workflows, with shared flow helpers used where practical.
8. User-flow documentation is updated before closure for every changed user-visible workflow.
9. Every defect discovered during this checklist is classified by severity, ownership, and architectural area. Defects inside the same bounded architectural area enter the current remediation queue when they do not materially change ownership or risk; other defects become explicit next tasks/follow-ups under this same complex-processing contract, not informal notes.

-----
artifact_path: project/repository/agents.sidecar
artifact_type: bootstrap_doc
artifact_version: '1'
artifact_revision: '2026-03-15'
schema_version: '1'
status: canonical
source_path: AGENTS.sidecar.md
created_at: '2026-03-10T02:13:40+02:00'
updated_at: 2026-06-02T07:05:00+03:00
changelog_ref: AGENTS.sidecar.changelog.jsonl
