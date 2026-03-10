# AGENTS Sidecar

Purpose: provide the project docs map for the repository being developed on top of the VIDA framework, without moving project-document knowledge into `AGENTS.md`.

## Project Docs Scope

1. Repository: `vida-stack`
2. This sidecar is the project docs map only.
3. It carries project-document discovery pointers and project-document orientation.
4. It must not become a second framework map or a mixed runtime/bootstrap carrier.
5. Framework-owned discovery starts at `vida/root-map.md` and continues through framework maps under `vida/config/instructions/**`.
6. It maps the active current project surface only; extracted secondary bundles such as `projects/vida-mobile/**` are out of default bootstrap scope unless the task explicitly targets them.

## Project Canonical Maps

1. Current project root map:
   - `docs/project-root-map.md`
2. Project product index:
   - `docs/product/index.md`
3. Product spec map:
   - `docs/product/spec/current-spec-map.md`
4. Project documentation system:
   - `docs/product/spec/project-documentation-system.md`
5. Documentation/product alignment matrix:
   - `docs/product/spec/canonical-documentation-and-inventory-layers.md`
6. Canonical runtime readiness law used by the project:
   - `docs/product/spec/canonical-runtime-readiness-law.md`
7. Canonical runtime layer matrix:
   - `docs/product/spec/canonical-runtime-layer-matrix.md`
8. Documentation tooling map:
   - `docs/process/documentation-tooling-map.md`
9. Project agent-extension map:
   - `docs/process/agent-extensions/README.md`

## Bootstrap Read Path

1. After `AGENTS.md`, read this sidecar immediately.
2. Use this sidecar as the project docs map during the mandatory two-map initialization step.
3. Continue first to `docs/project-root-map.md` when the task depends on active current-project understanding.
4. Continue into the project canonical maps listed below when the task depends on product/spec understanding.
5. For documentation/product alignment questions, continue to `docs/product/spec/canonical-documentation-and-inventory-layers.md`.
6. For documentation tooling or operator-command questions, continue to `docs/process/documentation-tooling-map.md`.
7. For runtime-layering, runtime-readiness, or runtime-architecture questions, continue to `docs/product/spec/canonical-runtime-layer-matrix.md`.
8. For project role/skill/profile/flow extension questions, continue to `docs/process/agent-extensions/README.md`.
9. This path is mandatory bootstrap context, not an optional later lookup.

Project-routing rule:

1. Project/product document pointers belong here, not in framework-owned map/index surfaces under `vida/config/instructions/**`.
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
7. `codex-v0` is the canonical project-side operator/runtime surface for bounded documentation validation, readiness, relation, and proof work once the relevant project/spec context is known.
8. Do not postpone `codex` usage until after broad manual documentation work when the task already depends on documentation mutation, validation, readiness, or proof-shaped output.

-----
artifact_path: project/repository/agents.sidecar
artifact_type: bootstrap_doc
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: AGENTS.sidecar.md
created_at: '2026-03-10T02:13:40+02:00'
updated_at: '2026-03-10T15:37:40+02:00'
changelog_ref: AGENTS.sidecar.changelog.jsonl
