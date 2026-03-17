# AGENTS Sidecar

Purpose: provide the project docs map for the repository being developed on top of the VIDA framework, without moving project-document knowledge into `AGENTS.md`.

## Project Docs Scope

1. Repository: `vida-stack`
2. This sidecar is the project docs map only.
3. It carries project-document discovery pointers and project-document orientation.
4. It must not become a second framework map or a mixed runtime/bootstrap carrier.
5. Framework-owned discovery for active development bootstrap starts from bounded framework carriers referenced by canonical shorthand ids interpreted through `vida protocol view`.
6. It maps the active current project surface only; extracted secondary bundles such as `projects/vida-mobile/**` are out of default bootstrap scope unless the task explicitly targets them.

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
2. Use this sidecar as the project docs map during active development bootstrap.
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
2. Use this sidecar only for project docs discovery and project-document orientation.
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
12. For normal write-producing work, assume delegated agents are the default execution path once a lawful packet exists.
13. For cheaper orchestrator lanes, prefer the project orchestrator operating protocol over broad free-form planning.
14. For new or resumed development orchestration sessions, prefer the compact project startup bundle for routine reads and expand only the needed deeper startup/packet owner surfaces when the bundle and project runtime capsules do not settle the question.
15. Do not pre-split the whole backlog into `execution_block` leaves during bootstrap; keep launch readiness at `delivery_task` depth and refine just-in-time for the next active item.
16. Before bounded work begins, inspect the current available skill catalog and activate the minimal relevant skill set or make `no_applicable_skill` explicit.
17. Use the canonical project packet-template and prompt-stack protocols rather than inventing packet structure or prompt-layer precedence ad hoc.
18. Treat boot readiness as incomplete until the project boot-readiness validation protocol can be satisfied for the current session.
19. Keep the live root bootstrap carrier `AGENTS.md` synchronized with `install/assets/AGENTS.scaffold.md`; when one changes, update the other in the same bounded change.
20. Synchronization is bidirectional and mandatory: if either `AGENTS.md` or `install/assets/AGENTS.scaffold.md` is changed, update the counterpart in the same bounded change before closure.
21. During active development, if a runtime/workflow blockage is detected and that blockage does not conform to project specs or canonical runtime requirements, treat it as implementation debt: fix the code path to restore spec-compliant behavior, prove it with bounded tests/evidence, and then continue execution without manual blocker bypass hacks.
22. After completing a complex task, or after fixing any error/blocking condition, run a fresh release build, update the system-installed binary, and continue development immediately in the next cycle.

-----
artifact_path: project/repository/agents.sidecar
artifact_type: bootstrap_doc
artifact_version: '1'
artifact_revision: '2026-03-15'
schema_version: '1'
status: canonical
source_path: AGENTS.sidecar.md
created_at: '2026-03-10T02:13:40+02:00'
updated_at: 2026-03-16T10:06:24.472459377Z
changelog_ref: AGENTS.sidecar.changelog.jsonl
