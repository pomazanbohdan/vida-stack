# VIDA Root Map

Purpose: provide the top-level framework root map for the `vida/` space so framework discovery starts at the framework root rather than inside one instruction subdirectory.

## Scope

This root map covers:

1. framework canon under `vida/config/**`,
2. framework maps and protocols under `vida/config/instructions/**`,
3. runtime-family discovery,
4. template discovery,
5. governance discovery,
6. framework-owned machine-readable law and projection surfaces.

## Canonical Entry Points

1. `vida/config/instructions/system-maps/framework.index.md`
   - instruction-home entrypoint for framework maps and registries
2. `vida/config/instructions/system-maps/framework.map.md`
   - framework topology, layer, and promotion/projection map
3. `vida/config/instructions/system-maps/framework.protocol-domains-map.md`
   - protocol-domain routing map across orchestration and adjacent protocol families
4. `vida/config/instructions/system-maps/protocol.index.md`
   - protocol registry for canonical instruction/runtime domains
5. `vida/config/instructions/system-maps/runtime-family.index.md`
   - runtime-family discovery for `codex`, `taskflow`, and future runtimes
6. `vida/config/instructions/system-maps/template.map.md`
   - framework template-family discovery
7. `vida/config/instructions/system-maps/governance.map.md`
   - human governance, approval, contribution, and policy-gate discovery
8. `vida/config/instructions/system-maps/observability.map.md`
   - runtime observability, traces, proving, and health discovery

## Activation Triggers

Read this map when:

1. bootstrap needs the framework map in one pass,
2. the task asks where the framework starts,
3. a task needs framework topology before selecting a lower map,
4. routing must distinguish topology, protocol domains, protocols, runtimes, templates, and governance.

## Routing

1. Framework topology or ownership/layer questions:
   - continue to `vida/config/instructions/system-maps/framework.map.md`
2. Protocol-domain family lookup:
   - continue to `vida/config/instructions/system-maps/framework.protocol-domains-map.md`
3. Canonical protocol lookup:
   - continue to `vida/config/instructions/system-maps/protocol.index.md`
4. Runtime-family lookup:
   - continue to `vida/config/instructions/system-maps/runtime-family.index.md`
5. Template lookup:
   - continue to `vida/config/instructions/system-maps/template.map.md`
6. Human governance, approval, contribution, or policy-gate lookup:
   - continue to `vida/config/instructions/system-maps/governance.map.md`
7. Runtime observability, traces, proving, or health lookup:
   - continue to `vida/config/instructions/system-maps/observability.map.md`

## Boundary Rule

1. `vida/` root discovery is framework-owned.
2. This map must not embed concrete project-doc pointers.
3. Project documentation discovery still hands off through `AGENTS.sidecar.md` and project maps under `docs/**`.

-----
artifact_path: framework/root-map
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: vida/root-map.md
created_at: '2026-03-10T09:30:00+02:00'
updated_at: '2026-03-12T11:05:54+02:00'
changelog_ref: root-map.changelog.jsonl
