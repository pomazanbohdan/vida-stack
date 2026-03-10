# Template Map

Purpose: expose the canonical template families used by VIDA so template discovery does not depend on ad hoc filesystem guessing.

## Template Families

1. Framework scaffold templates
   - home: `docs/framework/templates/*`
   - owner: framework
   - purpose: repository/bootstrap and external artifact scaffolding
2. Prompt template configurations
   - home: `vida/config/instructions/prompt_templates/*`
   - owner: framework/runtime instruction layer
   - purpose: runtime prompt/template projections for agent rendering
3. Instruction contract projections
   - home: `vida/config/instructions/instruction_contracts/*`
   - owner: framework/runtime instruction layer
   - purpose: machine-readable instruction-contract projections
4. Agent definition projections
   - home: `vida/config/instructions/agent_definitions/*`
   - owner: framework/runtime instruction layer
   - purpose: machine-readable role/agent-definition projections

## Activation Triggers

Read this map when:

1. bootstrap or project scaffold work needs a framework template,
2. runtime prompt rendering or agent-definition rendering is being inspected,
3. instruction projection or prompt-template ownership must be resolved,
4. a task explicitly asks where templates live or which template family is canonical.

Do not read this map by default for ordinary product-spec questions.

## Routing

1. Bootstrap/project scaffolding:
   - continue to `docs/framework/templates/vida.config.yaml.template`
2. Prompt rendering and worker/orchestrator template shape:
   - continue to `vida/config/instructions/prompt_templates/*`
3. Machine-readable contract/definition projections:
   - continue to `vida/config/instructions/instruction_contracts/*`
   - and `vida/config/instructions/agent_definitions/*`
4. Project-owned future templates:
   - resolve through the active project root map rather than assuming they belong here

## Boundary Rule

1. This map owns framework template discovery.
2. It does not make all template families framework-owned by default.
3. If a future project introduces project-owned templates, the active project root map must expose them separately.

-----
artifact_path: config/system-maps/template.map
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps.template-map.md
created_at: '2026-03-10T08:45:00+02:00'
updated_at: '2026-03-10T08:45:00+02:00'
changelog_ref: system-maps.template-map.changelog.jsonl
