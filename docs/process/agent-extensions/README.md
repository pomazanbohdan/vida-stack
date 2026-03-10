# Project Agent Extensions

Status: active project process doc

Purpose: provide the project-owned root map for custom roles, custom skills, custom profiles, and custom flow sets that extend the VIDA framework for the active project.

## Boundary Rule

1. framework role law remains owned by `vida/config/instructions/**`,
2. this directory owns only project-specific extension data and extension maps,
3. project extensions must be activated through `vida.config.yaml`,
4. project extensions must pass framework validation before `taskflow` may use them.

## Canonical Project Registry Family

1. `docs/process/agent-extensions/roles.yaml`
   - project roles derived from framework base roles
2. `docs/process/agent-extensions/skills.yaml`
   - project skills and their compatibility rules
3. `docs/process/agent-extensions/profiles.yaml`
   - project profiles that bind roles and skills
4. `docs/process/agent-extensions/flows.yaml`
   - project custom flow sets and their role chains

Shared skill note:

1. shared skills are reusable capability surfaces outside this project-owned registry family,
2. enabling a shared skill for this project still happens through `vida.config.yaml`,
3. use `shared:<skill_id>` in project profiles when the skill is not project-owned.

## Activation Path

1. `vida.config.yaml`
2. `agent_extensions`
3. this map
4. the registry files above
5. `taskflow-v0 config validate`

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
updated_at: '2026-03-10T15:41:04+02:00'
changelog_ref: README.changelog.jsonl
