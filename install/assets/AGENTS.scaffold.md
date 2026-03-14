# VIDA Project Bootstrap Carrier

<identity>
You are operating inside a VIDA-initialized project.

This file is the generated downstream bootstrap carrier.
It is a delivery surface, not the framework owner layer.

Core rule:
1. Use command-first bootstrap through the local `vida` binary.
2. Use `AGENTS.sidecar.md` as the project docs map.
3. Use bounded framework canonical ids through `vida protocol view <id>` only when the runtime init surfaces leave an edge case unresolved.

Canonical bootstrap routes:
1. Main/root lane: `vida orchestrator-init`
2. Worker/agent lane: `vida agent-init`
3. Pending onboarding or activation: `vida project-activator`

Activation rule:
1. If `vida orchestrator-init` or `vida agent-init` reports `pending_activation`, do not enter normal execution.
2. Use `vida project-activator` to record project identity, language policy, docs roots, and host CLI setup.
3. During pending activation, use `vida docflow` for bounded documentation/readiness inspection.
4. During pending activation, do not enter `vida taskflow` or any legacy runtime surface.

Normal feature-delivery rule:
1. If a request asks for research, detailed specifications, an implementation plan, and then code, create or update one bounded design document before code execution.
2. Start from the local project template referenced by `AGENTS.sidecar.md`.
3. Keep that document canonical through `vida docflow`.
4. After the design document fixes the bounded file set and proof targets, continue through orchestrated execution rather than collapsing immediately into root-session coding.

Host CLI rule:
1. Host agent templates are activated through `vida project-activator`, not `vida init`.
2. When activation materializes the selected host template, close and restart that tool so the agents become visible to the runtime environment.
</identity>

## Bootstrap Sequence

1. Read `AGENTS.sidecar.md`.
2. Run the bounded runtime init surface for the active lane:
   - `vida orchestrator-init`
   - or `vida agent-init`
3. If the init surface reports `pending_activation`, run `vida project-activator` before ordinary work.
4. Prefer project-local docs/process/spec guidance resolved from `AGENTS.sidecar.md`.
5. Open deeper framework protocol surfaces only on demand through canonical shorthand ids interpreted via `vida protocol view`.

## Working Boundary

1. This file routes bootstrap only.
2. Project documentation ownership belongs to project docs resolved through `AGENTS.sidecar.md`.
3. Framework owner law remains in the framework runtime and bounded protocol-view surfaces.
4. Do not treat this generated carrier as the owner of framework policy.

-----
artifact_path: install/assets/agents-scaffold
artifact_type: bootstrap_doc
artifact_version: '1'
artifact_revision: '2026-03-14'
schema_version: '1'
status: canonical
source_path: install/assets/AGENTS.scaffold.md
created_at: '2026-03-14T18:10:00+02:00'
updated_at: '2026-03-14T18:10:00+02:00'
changelog_ref: AGENTS.scaffold.changelog.jsonl
