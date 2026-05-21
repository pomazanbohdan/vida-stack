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
11. Project-local TaskFlow runtime state and operator surfaces:
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
12. For project-local TaskFlow DB-first execution/bootstrap questions, prefer `vida status --json`, `vida taskflow help`, and the project-owned `.vida/data/state/` runtime store path rather than installed shim or flat task-artifact fallback paths.
13. After bootstrap, prefer the default `vida taskflow ...` shell path with project-local runtime resolution; do not reintroduce installed shim roots that point outside this repository.
14. For project task-shaping, development-team, or delegated execution questions, continue early to `docs/process/team-development-and-orchestration-protocol.md`.
15. For cheaper orchestrator-first project execution, continue early to `docs/process/project-orchestrator-operating-protocol.md`.
16. For repeatable development-session startup, prefer the compact startup bundle:
   - `docs/process/project-orchestrator-startup-bundle.md`
17. Expand to the full session-start protocol and reusable prompt only when the bundle does not settle the startup question:
   - `docs/process/project-orchestrator-session-start-protocol.md`
   - `docs/process/project-orchestrator-reusable-prompt.md`
18. If startup readiness, skill gating, packet rendering, or packet/lane defaults remain unclear after the bundle, expand only the needed compact project runtime capsules:
   - `docs/process/project-start-readiness-runtime-capsule.md`
   - `docs/process/project-packet-rendering-runtime-capsule.md`
   - `docs/process/project-packet-and-lane-runtime-capsule.md`
19. Open deeper owner docs only when those compact project surfaces still leave an edge case unresolved:
   - `docs/process/project-skill-initialization-and-activation-protocol.md`
   - `docs/process/project-development-packet-template-protocol.md`
   - `docs/process/project-agent-prompt-stack-protocol.md`
   - `docs/process/project-boot-readiness-validation-protocol.md`
20. This path set is mandatory bootstrap context, not an optional later lookup.

Project-routing rule:

1. Project/product document pointers belong here, not in framework-owned map/index surfaces addressed by shorthand framework ids.
2. Framework-owned bootstrap may resolve that a downstream target belongs to the project layer, but the concrete project canonical map pointers must be carried by this sidecar.
3. Preserved secondary project bundles are not the default project-doc target for this sidecar; they must be entered only by explicit task targeting.

## Working Rule

1. Use `AGENTS.md` for lane routing and hard invariants.
2. Use this sidecar for project-local agent instructions, project operating rules, and project-document orientation.
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
11. For development work, assume `delivery_task` is the default decomposition leaf and use `execution_block` only when one-owner bounded closure still fails.
12. For normal write-producing work, assume delegated agents are the default execution path once a lawful packet exists, and interpret that path as the project runtime's delegated lane flow through `vida agent-init` rather than host-tool-specific subagent primitives.
13. For cheaper orchestrator lanes, prefer the project orchestrator operating protocol over broad free-form planning.
14. Host-local shell/edit capability is read-side convenience only; it does not grant root-session write ownership while the delegated path remains lawful.
15. For new or resumed development orchestration sessions, prefer the compact project startup bundle for routine reads and expand only the needed deeper startup/packet owner surfaces when the bundle and project runtime capsules do not settle the question.
16. Do not pre-split the whole backlog into `execution_block` leaves during bootstrap; keep launch readiness at `delivery_task` depth and refine just-in-time for the next active item.
17. Before bounded work begins, inspect the current available skill catalog and activate the minimal relevant skill set or make `no_applicable_skill` explicit.
18. Use the canonical project packet-template and prompt-stack protocols rather than inventing packet structure or prompt-layer precedence ad hoc.
19. Treat boot readiness as incomplete until the project boot-readiness validation protocol can be satisfied for the current session.
20. Keep the live root bootstrap carrier `AGENTS.md` synchronized with `install/assets/AGENTS.scaffold.md`; when one changes, update the other in the same bounded change.
21. Synchronization is bidirectional and mandatory: if either `AGENTS.md` or `install/assets/AGENTS.scaffold.md` is changed, update the counterpart in the same bounded change before closure.
21. During active development, if a runtime/workflow blockage is detected and that blockage does not conform to project specs or canonical runtime requirements, treat it as implementation debt: fix the code path to restore spec-compliant behavior, prove it with bounded tests/evidence, and then continue execution without manual blocker bypass hacks.
22. After completing a complex task, or after each individual runtime defect/error/blocking-condition fix, run a fresh release build, update the environment-resolved system `vida` binary that subsequent `vida ...` commands will execute, verify the installed binary path/fingerprint reflects that release build, and continue development immediately in the next cycle before starting the next defect-fix wave.
22a. After each individual runtime defect fix has passed its proof and the release build plus environment-resolved system `vida` binary update/verification has completed, run the VIDA runtime self-diagnostic before selecting the next defect. In this project-local self-test cycle, record and route diagnostic findings through the project TaskFlow backlog/tasks; do not search, comment on, or create GitHub issues unless the user explicitly asks for GitHub issue workflow.
23. Do not apply hotfix-style write mutations based on the first visible symptom alone; first study the related code paths, runtime state, and relevant specs so the bounded unit is anchored in the surrounding architecture.
24. Prefer architectural corrections over narrow symptom patches: once the affected codebase slice and spec contract are understood, fix the underlying invariant or routing rule rather than layering one-off local exceptions.
25. During active development, if a newly uncovered runtime defect is found, immediately analyze the related code paths, relevant specs, operator and runtime impact, architectural integrity, and severity; create or update the corresponding work item under the correct epic; reconsider development flow and ordering against the project goal; and then fix the defect architecturally rather than routing around it.
25a. For active defect work, keep the current working cadence explicitly numbered as steps 1 through 3 and repeat that cycle as needed. After a defect is found and selected for work, the path from selection to code fix is capped at these three repair steps only: `Крок 1/3: дослідження`, `Крок 2/3: архітектурне рішення + bounded write scope`, and `Крок 3/3: code fix`. One numbered investigation step may read an unlimited number of relevant code, test, generated, runtime-state, and documentation files; writing or fixing tests is outside this step cap.
25b. Every progress update, handoff, and status note for a selected defect before the code fix must name the active cadence marker using exactly one of these shapes: `Крок 1/3: дослідження`, `Крок 2/3: архітектурне рішення + bounded write scope`, or `Крок 3/3: code fix`. Do not introduce extra repair-step labels, unnumbered design phases, pre-fix summaries, or additional investigation/design rounds after selection; if more research is needed, it remains inside `Крок 1/3: дослідження`.
25c. `Крок 3/3: code fix` must apply the bounded code fix for the selected defect, or explicitly close the cycle as `no-code-fix-required` with current runtime evidence that the selected defect no longer reproduces. Test authoring, proof execution, release build/install, diagnostics, TaskFlow closure, commit, and push are mandatory post-fix gates, but they occur after the three repair steps and are not counted as repair steps. If the defect changes shape during investigation, restate the active bounded unit, why it remains the same unit or bind a new one, then restart the numbered 1/3 through 3/3 cadence for that new defect shape.
25d. During `Крок 1/3: дослідження`, after identifying the defect and reading the related/dependent code paths but before deciding how to fix it, explicitly actualize the expected behavior from the project documentation that specifies agent behavior across the affected levels: the relevant project spec, protocol, design note, canonical runtime/agent-behavior map, command/operator contract, and any mapped docs from this sidecar. Runtime code contracts are supporting evidence, not a substitute for the project documentation when agent behavior is specified there. The architecture decision in `Крок 2/3: архітектурне рішення + bounded write scope` must state the expected behavior being restored, the project-doc/spec/protocol evidence used, and how the selected bounded fix will be validated against that expected behavior. If the relevant expected behavior cannot be resolved from existing project documentation or mapped runtime/operator contracts, fail closed to a specification/contract clarification task instead of implementing from intuition.
25e. Parent/child closure consistency is a runtime invariant in both directions. A closed parent with any open or non-closed child is a defect; an open parent with no open or ready child is also a defect unless the runtime records an explicit continuation, diagnostic, or blocker reason that explains why the parent remains open.
25f. During happy-path TaskFlow testing, every discovered defect must be created or updated under the correct defect epic and immediately repaired through agent mode before the happy-path sequence continues.
25g. The canonical repair path is VIDA `agent-init`. If `vida agent-init` is blocked by the runtime defect being repaired, record blocker evidence and use Defective Runtime Emulation Mode only as bounded recovery until `agent-init` is restored, then return to canonical delegated execution.
25h. Happy-path TaskFlow tests must progress from the simplest case to the most complex case, and each failure found in that sequence must route into the defect remediation flow before advancing to more complex happy-path coverage.
25i. When multiple related runtime defects, slow operator commands, stale projections, blocked continuation states, test regressions, or task-actualization gaps are visible in the same active case, handle them as a batch-analysis unit before selecting individual fixes. Batch intake must collect all currently evidenced defects for the active case, including commands above the two-second target, commands above the five-second hard ceiling, stale or contradictory runtime surfaces, impossible next actions, continuation blockers, task actualization gaps, release-admission blockers, dispatch blockers, and parallelism blockers.
25j. Batch defect work requires meta-analysis before code changes: identify the shared runtime invariant, affected command family, state-store/graph/IO path, operator contract, TaskFlow impact, severity, and whether the observed failures are one root cause, dependent blockers, or independent slices. This batch-level analysis supplements the three-step defect cadence; it does not add extra repair-step labels and it does not replace `Крок 1/3: дослідження`, `Крок 2/3: архітектурне рішення + bounded write scope`, or `Крок 3/3: code fix`.
25k. Batch analysis may cover multiple defects at once, but each write-producing fix must still have an explicit bounded unit, bounded file scope, proof target, and conflict-domain classification. Repair multiple defects in one code batch only when their write scopes and runtime conflict domains are disjoint or when the same bounded invariant is being repaired coherently. If parallel safety is missing, contradictory, or blocked by runtime state, fail closed to sequential repair order.
25l. P0 latency, timeout, stale continuation, impossible next-action, release-admission, blocked-dispatch, and task-actualization defects may preempt ordinary ready tasks until normal orchestration throughput is restored. Every newly discovered defect in the batch must be created or updated under the correct case/epic before or during the batch when TaskFlow mutation is available; if TaskFlow mutation is itself blocked, keep explicit session evidence and backfill the task notes once writable.
25m. Proof for batch defect work must be layered: after each bounded fix, run focused regression proof for that defect; after the batch, run the combined command/test proof set that covers the shared invariant and adjacent operator surfaces. This rule does not weaken root write guard, `vida agent-init`, exception takeover, explicit bounded-unit binding, sticky continuation, release/install/self-diagnostic gates, or sequential-only blockers.
26. During active development, in addition to continuously tracking and fixing runtime defects, the system must also continuously track command surfaces, command options, and operator information output; when gaps, missing commands/options, or needed output additions are detected, immediately analyze the related code paths, relevant specs, and operator/runtime impact, then add or correct those command and output surfaces proactively and automatically rather than deferring them.
26a. Runtime/operator command latency is a product invariant. The target operator command processing speed is up to two seconds for normal inspection, planning, scheduling, continuation, and status surfaces unless a command is explicitly documented as a long-running build/test/release action. Any `vida` operator operation that takes more than five seconds is an architectural defect: do not normalize it as acceptable latency and do not fix it by merely increasing a timeout. If a runtime operation times out, repeatedly exceeds the two-second target, exceeds the five-second hard ceiling, or blocks orchestration throughput, classify it as Priority 0 architectural runtime debt. Handle it through meta-analysis first: identify the slow command family, state-store/lock/IO/graph/reconciliation path, expected spec behavior, and architectural refactor needed to restore fast bounded operation. P0 timeout work may preempt ordinary ready tasks until command throughput is restored.
27. When a user explicitly orders post-fix release actions, treat that sequence as mandatory: after the bounded implementation wave is complete, run a fresh release build, update the system-installed binary, create a commit, and push it before declaring that wave finished.
27a. When a user explicitly requires release actions after task closure, treat that rule as sticky for the active session: immediately after closing each bounded task, run a fresh release build, update the system-installed binary, create a commit, push it, and then bind the next lawful agent task without waiting for another user prompt.
28. Commentary, status output, and intermediate reports are visibility only; they do not create a lawful pause boundary and must not be treated as completion or as permission to idle when the next lawful continuation item is already evidenced.
29. After any bounded result, green test, successful build, or delegated handoff, immediately bind the next already-evidenced lawful continuation item and continue in the same execution cycle rather than pausing at summary/reporting.
30. Agent carriers, visible host-agent templates, carrier topology, and default lane-to-carrier assumptions must not be hardcoded in owner/runtime code paths; the source of truth is the active configuration and registries, primarily `vida.config.yaml` plus the enabled agent-extension registries.
31. File-system template layouts such as `.codex/agents/*.toml` are materialization outputs, not authority surfaces; when code or runtime summaries need the available carriers/templates, resolve them from the configured carrier catalog first and treat on-disk templates as projection/evidence only.
32. Active runtime/project code must not keep legacy code paths or legacy functionality as current behavior. The current implementation must be owned by canonical config, templates, registries, runtime contracts, and state-store truth. Historical artifact support is allowed only as bounded, explicit migration or normalization with recorded evidence and must not become an active routing, retry, dispatch, closure, materialization, or operator-output branch. When legacy code or functionality is found, create or update a TaskFlow defect under the current defect epic, then replace it with current canonical behavior rather than extending compatibility.
33. For token-saving delegated support, the orchestrator may use project-local qwen fallback lanes only as read-only research, context, review, or junior-draft execution carriers. These lanes never transfer project write authority, never mutate `.vida/data/state`, receipts, packets, or lane metadata, and never replace canonical VIDA/TaskFlow `vida agent-init` for write-producing work. Accepted qwen findings or draft changes must be synthesized by the parent and routed through a lawful bounded VIDA write unit before project files are changed.
34. When canonical VIDA delegation, dispatch, continuation, closure, or carrier execution is blocked by a runtime defect, stale state, activation-view-only handoff, missing receipt evidence, or carrier unavailability, the orchestrator must not silently collapse into solo root implementation. It must first use the available project/host agents as bounded advisory lanes when safe: read-only qwen/Pi research agents, review agents, planners, scouts, or other non-mutating carriers may gather code evidence, spec evidence, test designs, risk analysis, and implementation sketches. These advisory lanes are evidence inputs only; they do not grant write authority, completion receipts, exception takeover, or TaskFlow closure. If no agent lane can be launched, record the launch blocker explicitly before proceeding under Defective Runtime Emulation Mode.

## Defective Runtime Emulation Mode

Use this project-local recovery mode when the VIDA runtime surfaces that should normally plan, dispatch, continue, prioritize, or close work are themselves defective, timing out, returning contradictory state, or blocking progress with evidence that violates the project specs. This mode exists to keep recovery development fast and spec-faithful while the broken runtime is being repaired. It is an emulation of VIDA runtime behavior by the orchestrator, not a replacement owner layer and not a weakening of framework law.

Activation criteria:

1. Prefer normal `vida orchestrator-init`, `vida taskflow graph-summary`, `vida task next`, `vida taskflow run-graph dispatch-init`, `vida agent-init --execute-dispatch`, `vida lane ...`, `vida task update`, `vida doctor`, and DocFlow surfaces first.
2. Enter this mode only after one or more canonical surfaces are proven defective for the active bounded recovery unit: timeout without receipt, activation-view-only handoff, stale/contradictory continuation binding, impossible recommended command, missing dispatch context, datastore lock contention, release/install command denial, or diagnostics that cannot be satisfied by the command they recommend.
3. Record the defect as evidence in the active task notes or a new TaskFlow defect task when the task store is writable. If TaskFlow mutation is broken, keep a concise in-session evidence ledger until the store is writable again, then backfill the task notes.
4. Keep `active_bounded_unit`, `why_this_unit`, and `sequential_vs_parallel_posture` explicit before any emulated planning or execution move.
5. If a blocking condition is discovered that prevents continuation of the active recovery/development flow, fixing that blocker is allowed and should be treated as elevated-priority work. Keep the blocker repair bounded to the smallest file/state/operator surface that restores continuation, record why it outranks the previous ready item, and resume the interrupted flow immediately after proof.
6. If a command timeout or slow runtime operation is discovered, treat it as Priority 0 when it affects planning, scheduling, status, recovery, continuation, dispatch, lane, packet, doctor, or task surfaces. First run meta-analysis over the command family and related state-store/graph/IO paths, then perform an architectural refactor rather than increasing timeouts or working around the symptom. The target restored behavior is command processing within two seconds for ordinary operator surfaces.

Emulated runtime responsibilities:

1. Planning: derive the next bounded unit from TaskFlow-ready evidence, critical-path position, explicit user priority, dependency graph, and conflict domains. Do not self-select adjacent backlog work when those fields are ambiguous.
2. Replanning: when a selected unit changes shape, restate the bounded unit and either keep it with evidence or create/update the correct follow-up task under the correct epic. Do not hide new runtime defects as informal notes.
3. Prioritization: continuously re-rank ready work by unblock value, critical path, severity, proof cost, command-latency impact, and conflict-domain safety. A newly discovered blocker that prevents continuation is allowed to preempt the current ready item with elevated priority until the flow can continue. Command timeout and slow-operator defects are Priority 0 when they affect normal runtime operation, with a two-second target for ordinary command processing. Prefer recovery work that restores normal runtime operation over cosmetic cleanup.
4. Parallelization: treat tasks as parallel-safe only when runtime evidence or task execution semantics show disjoint conflict domains, disjoint owned paths, and no current delegated-cycle conflict. If the scheduler surface is contradictory, fail closed to sequential execution.
5. Agent execution: emulate the canonical delegated lane sequence by producing the same external evidence an agent lane should have produced: bounded goal, owner role, read/write scope, inputs, outputs, proof target, result summary, and blocker/receipt status. Host subagent APIs may be used only as carrier details; the canonical lane model remains TaskFlow/agent-init. When canonical VIDA execution is blocked but host/Pi subagents are available, launch bounded read-only advisory agents before or alongside root diagnosis for complex work; prefer independent single async lanes with explicit artifact outputs, and synthesize their evidence before architectural decisions. Do not treat advisory agent output as a dispatch receipt, write authorization, or completion proof.
6. Continuation: after every green proof, build, release install attempt, diagnostic result, handoff, or closure, immediately re-evaluate the next lawful continuation item instead of pausing at commentary.
7. Closure: close or update tasks only with concrete proof evidence, including command names, pass/fail status, installed binary fingerprint when release is required, and any diagnostic blocker that remains.

Write and safety rules during emulation:

1. This mode may bypass a defective command surface as an operator mechanism, but it must not bypass the expected behavior defined by the specs.
2. Host-local write remains bounded to the active recovery unit and the smallest safe file set needed to restore spec-compliant runtime behavior.
3. Before write-producing work, first attempt canonical delegated dispatch or scoped exception takeover when those surfaces are available. If those surfaces are the defect under repair, document why they cannot provide lawful execution evidence and keep the emulated write scope tied to the active defect task.
4. Never treat `activation_view_only`, `receipt_recorded`, stale lane state, or a ready patch idea as completion evidence.
5. Never delete runtime locks, packets, receipts, or state artifacts by hand as a bypass. Use them as evidence and repair the code path that produced the bad state.
6. Never invent dispatch receipts, completion receipts, or green diagnostics. If a command cannot produce a receipt, record the failure shape and continue with explicit emulated evidence until the runtime can be repaired.
7. Keep the three-step active-defect cadence from this sidecar active: `Крок 1/3: дослідження`, `Крок 2/3: архітектурне рішення + bounded write scope`, and `Крок 3/3: code fix`.

Spec evidence to consult before and during this mode:

1. `docs/process/team-development-and-orchestration-protocol.md`
2. `docs/process/project-orchestrator-operating-protocol.md`
3. `docs/process/project-packet-and-lane-runtime-capsule.md`
4. `docs/product/spec/canonical-runtime-readiness-law.md`
5. `docs/product/spec/canonical-runtime-layer-matrix.md`
6. `docs/product/spec/continuation-and-seeded-dispatch-bridge-design.md`
7. The active task, lane, run-graph, packet, recovery, and doctor JSON surfaces when they are readable.

Exit criteria:

1. The repaired runtime can again select, dispatch, continue, recover, diagnose, and close the active bounded unit through canonical surfaces.
2. Focused tests for the repaired invariant pass.
3. A release build has been produced and the environment-resolved `vida` binary has been updated or the install blocker has been recorded as its own runtime defect.
4. Runtime self-diagnostic has run and any remaining blockers are routed as TaskFlow work.
5. The next continuation item is selected through the restored runtime surfaces, or the reason emulation must continue is recorded explicitly.

## Project-Local Qwen Fallback Research And Junior Execution Mode

Use this project-local fallback mode when the parent/orchestrator needs token-saving delegated support for bounded read-only research, context building, review evidence, mini-analysis, or junior-draft implementation reasoning. This mode is a Pi/subagents carrier pattern for support work; it never replaces VIDA/TaskFlow ownership for write-producing work.

Activation criteria:

1. The parent session explicitly launches Pi subagents with `context=fresh` and model `qwen3.6-35b-a3b-mtp` for bounded support work.
2. The child task is scoped as read-only research, context, review, reproduction reasoning, test/proof suggestion, or junior-draft code reasoning; it must not directly modify project files.
3. The parent provides a concrete prompt with a required non-empty markdown output schema and an explicit output artifact path.
4. The parent prompt must require artifact-first execution: the qwen child creates an initial markdown skeleton at the requested artifact path before deep analysis, updates/overwrites that artifact after each major evidence chunk, and treats the final chat response as a short pointer to the already-written artifact rather than the only place where findings exist.
5. The qwen child is never treated as a VIDA write lane, execution receipt, closure receipt, or exception-takeover authority.

Default invocation shape:

```ts
subagent({
  agent: "delegate",
  task: "Bounded read-only research slice with required markdown output schema...",
  async: true,
  context: "fresh",
  model: "qwen3.6-35b-a3b-mtp",
  reads: false,
  progress: false,
  output: "tmp/qwen-<slice>.md",
  outputMode: "file-only"
})
```

A user-level `qwen-research` agent may be used for the same pattern when available:

```ts
subagent({
  agent: "qwen-research",
  task: "Bounded read-only research slice with required markdown output schema...",
  async: true,
  context: "fresh",
  output: "tmp/qwen-<slice>.md",
  outputMode: "file-only"
})
```

Default agent-gated parent work strategy:

1. This is the default work mode for defect analysis, architecture decisions, and preparation for code changes in this project.
2. Before launching qwen/Pi advisory lanes, state the bounded decision, active write scope, and what evidence would change the decision.
3. Plan the lanes explicitly: sequential for one decision path; parallel only for independent, read-only, disjoint questions or when the user explicitly asks for parallel comparison.
4. Each lane must have a bounded prompt, artifact path, scope limit, output schema, and clear success/failure criteria.
5. After launching advisory lanes for that decision, do not patch, finalize, or silently continue solo until every relevant lane is complete, failed, or explicitly classified as irrelevant/stale.
6. Track run IDs, expected artifacts, completion state, and relevance to the active decision.
7. Validate every artifact before using it: non-empty, schema-valid, evidence-backed, scoped to the prompt, and useful for the decision.
8. Classify each result as `accepted evidence`, `partial evidence`, `conflict`, `content failure`, `process failure`, or `irrelevant/stale`.
9. Synthesize agent findings against each other and direct parent code/runtime reads before architecture decisions or patches.
10. If advisory findings conflict on a material point, resolve the conflict with a focused follow-up or direct parent validation before patching.
11. Parallel analysis of future defects is allowed; parallel write fixes are not allowed unless VIDA explicitly grants disjoint lawful write ownership.
12. Avoid advisory-agent idleness when useful safe read-only work exists: maintain a small rolling backlog of future-looking analysis lanes (normally 2-3 active) for root-cause hypotheses, affected files, spec/operator impact, test ideas, and risk ranking.
13. Future-looking advisory lanes must remain bounded and actionable; drop or pause any lane category that produces repetitive, schema-invalid, too broad, or non-actionable artifacts.
14. Advisory lanes do not grant write authority, completion receipts, exception takeover, or TaskFlow closure; the parent owns validation and lawful VIDA routing.

Concurrency policy:

1. Prefer independent single async qwen runs over grouped `tasks:[...]` parallel wrapper for qwen fallback work.
2. Default to sequential rolling micro-lanes for active defect diagnosis: launch one qwen slice, verify the artifact, narrow the next question, and launch another slice only if useful.
3. Use parallel qwen lanes only for independent read-only questions with disjoint evidence targets, or when the user explicitly asks for parallel comparison.
4. A `0 B`, skeleton-only, meta-only, or schema-invalid artifact is process/content failure; retry once with a narrower focused prompt if the question remains useful.

Prompt and output rules:

1. Avoid pure no-op smoke prompts for real work.
2. Use qwen in micro-iterations by default: one concrete question, one suspected seam, and at most 1-3 named files, symbols, or line ranges per child unless wider scope is explicitly required.
3. Give each qwen child a strict session-size budget in the prompt: no broad `grep .`, no broad repository globbing, exact-symbol search only when search is needed, at most 2 evidence chunks, and a concise artifact unless the parent explicitly asks for more. Do not cap tool calls as a primary metric; allow as many reads/searches as needed inside the narrow evidence scope, but do not expand scope.
4. Optimize for quality per unit of context, not speed alone. A qwen lane is successful only when it is evidence-backed enough for the decision being made, names uncertainty/risks, and gives an actionable next step. A faster artifact that only repeats parent assumptions is acceptable only for cheap hypothesis checks, not for code-fix authority.
5. Use the hybrid quality prompt as the current default for complex seam audits: compact fact-pack, narrow range-read, explicit hypothesis validation, direct evidence, `accept/reject/modify` decision, risk/regression guard, minimal test implication, confidence, and next micro-step.
6. Give each qwen child required markdown headings appropriate to the prompt, at minimum `## Result`, direct findings/evidence, `## Confidence/Risks` for evidence-backed decisions, and `## Next micro-step` when another bounded question remains.
7. Every qwen prompt with an output path must include incremental artifact instructions: write a skeleton first, rewrite findings after each bounded evidence chunk, and keep the artifact valid markdown even if compact, token exhaustion, stale-run reconciliation, or runner failure happens before the final answer.
8. Accept a qwen lane as content-complete only after verifying the output artifact is non-empty, schema-valid, and useful for the requested decision.
9. Store qwen outputs under `tmp/qwen-*.md` or a lawful research/documentation artifact path selected by the parent.

Write and authority boundaries:

1. Qwen fallback children must not edit, write, delete, move, or mutate project files unless a future lawful VIDA packet explicitly assigns that write scope through canonical runtime authority.
2. Qwen fallback children must not mutate `.vida/data/state`, TaskFlow DB, receipts, packets, lane metadata, release artifacts, or runtime ownership state.
3. The parent/orchestrator owns synthesis, validation, and any conversion of qwen findings into TaskFlow tasks, DocFlow work, code changes, or documentation changes.
4. For code-producing requests, qwen may provide a junior draft, patch idea, or implementation sketch only; the parent must validate and route any actual project mutation through lawful VIDA write ownership.
5. This mode does not weaken `AGENTS.md` root write guard, delegated lane evidence requirements, or exception-takeover state rules.

Local process note:

1. Current local `pi-subagents` behavior should keep child process windows hidden for background and foreground qwen lanes.
2. Qwen prompts must use non-ambiguous output-target wording plus explicit artifact-first instructions; avoid markdown-section-looking output directives that can be mistaken for response content.
3. If `pi-subagents` is reinstalled or upgraded, re-check child window behavior, output-target injection, artifact-first behavior, and Windows interrupt behavior before relying on long runs.

## Complex And Architectural Processing Contract

Use this project-local sidecar contract when a user request or discovered defect requires complex, comprehensive, architectural, root-cause, end-to-end, or cross-module remediation. It also applies when a change touches API contracts, VIDA framework behavior, shared widgets, auth/session behavior, routing, persistence, localization, Material/Shad/STAC ownership, or Web/Android E2E-visible user workflows.

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
8. Update proof with the implementation: unit tests for pure contracts and state, widget tests for UI states and accessibility/layout stability, and Web/Android E2E tests for changed user-visible navigation, auth/account, record/list/detail, storage, proxy/browser/device, or live Odoo behavior.
9. Update the human user flow whenever user-visible behavior changes. Keep `docs/specs/e2e-user-flows.md` and the mapped Patrol/Web/Android flow evidence aligned with the implementation, or record a specific rationale when the change is intentionally unit/widget-only.

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
updated_at: 2026-05-16T18:15:00+03:00
changelog_ref: AGENTS.sidecar.changelog.jsonl
