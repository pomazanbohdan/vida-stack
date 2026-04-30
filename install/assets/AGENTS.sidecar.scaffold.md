# Project Agent Instructions

Purpose: provide a clean project agent-instructions overlay scaffold for the repository being developed on top of the VIDA framework.

## Authority Boundary

1. Repository: `<fill-your-project-name>`
2. `AGENTS.md` owns VIDA framework bootstrap, lane routing, and hard runtime invariants.
3. This sidecar owns project-local agent instructions after framework bootstrap.
4. The project docs map is a required section of this sidecar, not the sidecar's only purpose.
5. This sidecar may carry project operating rules, local commands, coding/testing/release constraints, project-agent/team conventions, domain constraints, and project-document discovery pointers.
6. It must not become a second framework map or a mixed runtime/bootstrap carrier.
7. This scaffold is project-owned template material; it is not a framework owner-policy surface.

## Project Operating Rules

Fill in project-local rules that agents must follow in this repository.

1. Local build command:
   - `<build-command>`
2. Local test command:
   - `<test-command>`
3. Local release or verification command:
   - `<release-or-verification-command>`
4. Project constraints:
   - `<project-constraints>`

## Project Canonical Maps

Fill in the project-owned canonical surfaces used by this repository.

1. Current project root map:
   - `<project-root-map-path>`
2. Project product index:
   - `<product-index-path>`
3. Product spec map:
   - `<product-spec-map-path>`
4. Project documentation system:
   - `<project-documentation-law-path>`
5. Documentation/process tooling map:
   - `<documentation-tooling-map-path>`
6. Project extensions or project-side agent map:
   - `<project-extension-map-path>`

## Bootstrap Read Path

1. After `AGENTS.md`, read this sidecar immediately.
2. Use this sidecar as the project agent-instructions overlay after `AGENTS.md` selects and runs the bounded bootstrap route.
3. Continue first to the project root map when the task depends on current project understanding.
4. Continue into the project canonical maps listed above when the task depends on product/spec/process understanding.
5. Keep project/product pointers here, not in framework-owned map/index surfaces addressed by framework shorthand ids.

## Working Rule

1. Use `AGENTS.md` for framework lane routing and hard invariants.
2. Use this sidecar for project-local agent instructions and project-document orientation.
3. Replace all placeholder paths with project-owned canonical targets before relying on this sidecar in active work.
4. Keep `AGENTS.md` synchronized with `install/assets/AGENTS.scaffold.md`; when one changes, update the other in the same bounded change.
5. Treat that synchronization as bidirectional and mandatory: changing either side requires updating its counterpart before closure.

-----
artifact_path: install/assets/agents-sidecar-scaffold
artifact_type: bootstrap_doc
artifact_version: '1'
artifact_revision: '2026-03-17'
schema_version: '1'
status: canonical
source_path: install/assets/AGENTS.sidecar.scaffold.md
created_at: '2026-03-12T12:20:00+02:00'
updated_at: 2026-04-30T22:15:50.6007718Z
changelog_ref: AGENTS.sidecar.scaffold.changelog.jsonl
