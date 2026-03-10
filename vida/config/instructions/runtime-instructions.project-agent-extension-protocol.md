# Project Agent Extension Protocol

Purpose: define the canonical framework/runtime contract for project-owned role, skill, profile, and flow-set extensions activated through `vida.config.yaml` without weakening framework role law or runtime safety invariants.

## Core Contract

1. framework roles and standard flow sets remain the canonical base runtime surfaces,
2. projects may enable only a subset of framework roles and standard flow sets,
3. projects may also add custom roles, custom skills, custom profiles, and custom flow sets through project-owned registries,
4. project extensions are lawful only when they are activated through `vida.config.yaml` and pass validation.

## Canonical Split

Framework-owned layer:

1. role law,
2. role-profile law,
3. instruction contracts,
4. runtime safety invariants,
5. standard flow-set semantics,
6. validation semantics,
7. first-class framework roles including `orchestrator`, `worker`, `business_analyst`, `pm`, `coach`, `verifier`, and `prover`.

Project-owned layer:

1. which framework roles are enabled,
2. which standard flow sets are enabled,
3. which project roles exist,
4. which project skills exist,
5. which shared skills are enabled for this project,
6. which project profiles exist,
7. which custom project flow sets exist.

## Overlay Activation Surface

The active project overlay section is:

1. `vida.config.yaml`
2. top-level key: `agent_extensions`

Minimum active fields:

1. `enabled`
2. `map_doc`
3. `registries.roles`
4. `registries.skills`
5. `registries.profiles`
6. `registries.flows`
7. `enabled_framework_roles`
8. `enabled_standard_flow_sets`
9. `default_flow_set`
10. `validation.*`

Optional extension fields:

1. `role_selection`

## Project Registry Family

Project-owned registries must remain outside framework-owned `vida/**`.

Current canonical project-owned registry family:

1. `docs/process/agent-extensions/roles.yaml`
2. `docs/process/agent-extensions/skills.yaml`
3. `docs/process/agent-extensions/profiles.yaml`
4. `docs/process/agent-extensions/flows.yaml`
5. `docs/process/agent-extensions/README.md`

## Runtime Composition Rule

`taskflow` must treat the active runtime agent identity as a compiled composition:

1. base framework role,
2. optional project role derived from that base,
3. optional project skills,
4. optional shared skills,
5. optional project profile,
6. selected standard or project flow set,
7. route and gate constraints from framework runtime law.

Compact form:

`compiled_agent_profile = base_role + validated_project_role + validated_project_skills + enabled_shared_skills + validated_profile + selected_flow_set + route_constraints`

## Validation Rule

Project extension activation is valid only when all enabled references resolve.

Minimum validation requirements:

1. referenced registry files exist,
2. ids are unique inside each registry family,
3. each project role resolves to one known framework base role,
4. each enabled project role resolves to a registry row,
5. each enabled project skill resolves to a registry row,
6. each enabled shared skill resolves to a lawful shared skill reference,
7. each enabled project profile resolves to a registry row,
8. each enabled project flow resolves to a registry row,
9. each profile resolves to a known role,
10. each profile skill attachment resolves to known project skills or lawful shared skill refs,
11. each project profile project-skill attachment remains compatible with the resolved base role,
12. each flow role-chain resolves to known framework or project roles,
13. `default_flow_set` resolves to one enabled standard flow set or one enabled project flow set.

Fail-closed rule:

1. unresolved or invalid project extensions must block activation,
2. runtime must not silently drop invalid project extensions and continue as if the intended flow were valid.

## Standard Flow Ladder

Current standard framework flow-set ids:

1. `minimal`
2. `reviewed`
3. `verified`
4. `governed`
5. `durable`

Projects may enable any subset of these, but they must not rename or redefine their base semantics in place.

## Boundary Rule

1. project roles may extend framework role usage, but they must not invent stronger authority than their resolved framework base role permits,
2. project skills may deepen capability, but they must not turn one role into a different authority class,
3. shared skills remain reusable capability surfaces outside one project registry; enabling them for one project must still remain explicit in `vida.config.yaml`,
4. project profiles may compose roles and skills, but they must not become a hidden owner of runtime law,
5. project flow sets may reorder or specialize project behavior, but they must remain compatible with framework gates for handoff, verification, approval, and closure.

## Binary Placement Rule

1. runtime agent backends may declare `binary_path` in `vida.config.yaml` when PATH discovery is insufficient,
2. `binary_path` is project-owned environment data, not framework-owned role law,
3. runtime detection and dispatch may prefer `binary_path` over plain PATH lookup when it is present,
4. binary placement must not be hidden inside one prompt or one local shell habit.

## Operational Proof

Current bounded proof surfaces:

1. `taskflow-v0 config validate`
2. `taskflow-v0 role-select bundle --json`
3. `taskflow-v0 role-select request "<request>" --json`
4. `python3 codex-v0/codex.py activation-check vida/config/instructions/runtime-instructions.project-agent-extension-protocol.md`
5. `python3 codex-v0/codex.py protocol-coverage-check --profile active-canon`

## References

1. `vida/config/instructions/runtime-instructions.project-overlay-protocol.md`
2. `vida/config/instructions/instruction-contracts.agent-system-protocol.md`
3. `vida/config/instructions/agent-definitions.protocol.md`
4. `vida/config/instructions/agent-definitions.role-profile-protocol.md`
5. `docs/product/spec/agent-role-skill-profile-flow-model.md`
6. `docs/process/agent-extensions/README.md`

-----
artifact_path: config/runtime-instructions/project-agent-extension.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions.project-agent-extension-protocol.md
created_at: '2026-03-10T15:45:00+02:00'
updated_at: '2026-03-10T16:53:58+02:00'
changelog_ref: runtime-instructions.project-agent-extension-protocol.changelog.jsonl
