# Project Agent Extensions

Status: active project process doc

Purpose: provide the project-owned bridge/export map for custom roles, custom skills, custom profiles, and custom flow sets that extend the VIDA framework for the active project.

## Boundary Rule

1. framework role law remains owned by `vida/config/instructions/**`,
2. this directory is a source/export/import bridge surface only,
3. active runtime-owned agent-extension projections live under `.vida/project/agent-extensions/**`,
4. project extensions must be activated through `vida.config.yaml`,
5. project extensions must pass framework validation before `taskflow` may use them.

## Bridge Registry Family

1. `docs/process/agent-extensions/roles.yaml`
   - source/export bridge for project roles derived from framework base roles
2. `docs/process/agent-extensions/skills.yaml`
   - source/export bridge for project skills and their compatibility rules
3. `docs/process/agent-extensions/profiles.yaml`
   - source/export bridge for project profiles that bind roles and skills
4. `docs/process/agent-extensions/flows.yaml`
   - source/export bridge for project custom flow sets and their role chains

## Active Runtime Projection Family

1. `.vida/project/agent-extensions/README.md`
2. `.vida/project/agent-extensions/roles.yaml`
3. `.vida/project/agent-extensions/skills.yaml`
4. `.vida/project/agent-extensions/profiles.yaml`
5. `.vida/project/agent-extensions/flows.yaml`
6. matching `.vida/project/agent-extensions/*.sidecar.yaml`

Runtime rule:

1. `.vida/project/agent-extensions/*.yaml` is the active project-local runtime projection family.
2. Matching `*.sidecar.yaml` files are the editable override surfaces for project-local changes.
3. This root `docs/process/agent-extensions/**` tree is not the live runtime source.
4. When `vida.config.yaml` omits explicit `enabled_project_roles`, `enabled_project_profiles`, or `enabled_project_flows`, runtime should treat the active registry rows as the canonical enabled set instead of requiring duplicated id lists in config.

Shared skill note:

1. shared skills are reusable capability surfaces outside this project-owned registry family,
2. enabling a shared skill for this project still happens through `vida.config.yaml`,
3. use `shared:<skill_id>` in project profiles when the skill is not project-owned.

## Activation Path

1. `vida.config.yaml`
2. `agent_extensions`
3. this map
4. `.vida/project/agent-extensions/**` as the active runtime projection family
5. explicit export/import/sync when root bridge files are used
6. `vida project-activator --json`

## Project Operating Rule

1. do not redefine framework role authority here,
2. do not add project flows that weaken framework handoff, verification, approval, or closure gates,
3. keep project role/skill/profile/flow ids stable and unique,
4. prefer extending framework roles over inventing isolated project-only authority classes.

-----
artifact_path: process/agent-extensions/readme
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: docs/process/agent-extensions/README.md
created_at: '2026-03-10T15:45:00+02:00'
updated_at: '2026-03-13T11:20:00+02:00'
changelog_ref: README.changelog.jsonl
