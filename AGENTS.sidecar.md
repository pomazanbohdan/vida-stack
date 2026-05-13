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
26. During active development, in addition to continuously tracking and fixing runtime defects, the system must also continuously track command surfaces, command options, and operator information output; when gaps, missing commands/options, or needed output additions are detected, immediately analyze the related code paths, relevant specs, and operator/runtime impact, then add or correct those command and output surfaces proactively and automatically rather than deferring them.
27. When a user explicitly orders post-fix release actions, treat that sequence as mandatory: after the bounded implementation wave is complete, run a fresh release build, update the system-installed binary, create a commit, and push it before declaring that wave finished.
27a. When a user explicitly requires release actions after task closure, treat that rule as sticky for the active session: immediately after closing each bounded task, run a fresh release build, update the system-installed binary, create a commit, push it, and then bind the next lawful agent task without waiting for another user prompt.
28. Commentary, status output, and intermediate reports are visibility only; they do not create a lawful pause boundary and must not be treated as completion or as permission to idle when the next lawful continuation item is already evidenced.
29. After any bounded result, green test, successful build, or delegated handoff, immediately bind the next already-evidenced lawful continuation item and continue in the same execution cycle rather than pausing at summary/reporting.
30. Agent carriers, visible host-agent templates, carrier topology, and default lane-to-carrier assumptions must not be hardcoded in owner/runtime code paths; the source of truth is the active configuration and registries, primarily `vida.config.yaml` plus the enabled agent-extension registries.
31. File-system template layouts such as `.codex/agents/*.toml` are materialization outputs, not authority surfaces; when code or runtime summaries need the available carriers/templates, resolve them from the configured carrier catalog first and treat on-disk templates as projection/evidence only.

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
updated_at: 2026-05-12T22:54:22.6287522Z
changelog_ref: AGENTS.sidecar.changelog.jsonl
