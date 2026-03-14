# Template Map

Purpose: expose the canonical template families used by VIDA so template discovery does not depend on ad hoc filesystem guessing.

## Template Families

1. Framework scaffold template
   - home: `docs/framework/templates/vida.config.yaml.template`
   - owner: framework
   - purpose: repository/bootstrap and external artifact scaffolding
2. Feature/change design template
   - home: `docs/framework/templates/feature-design-document.template.md`
   - owner: framework
   - purpose: structured design-document starting shape for one bounded feature/change with linked ADR split when needed
3. Prompt template configurations
   - home: `prompt_templates/*`
   - owner: framework/runtime instruction layer
   - purpose: runtime prompt/template projections for agent rendering
4. Instruction contract projections
   - home: `instruction_contracts/*`
   - owner: framework/runtime instruction layer
   - purpose: machine-readable instruction-contract projections
5. Agent definition projections
   - home: `agent_definitions/*`
   - owner: framework/runtime instruction layer
   - purpose: machine-readable role/agent-definition projections

Naming split rule:

1. kebab-case instruction families such as `agent-definitions`, `instruction-contracts`, and `prompt-templates` are the canonical Markdown authoring homes,
2. snake_case families such as `agent_definitions`, `instruction_contracts`, and `prompt_templates` are machine-readable projection homes,
3. these paired family names are an intentional authoring-vs-projection split, not a second competing owner layer,
4. if a topic needs normative law or human-auditable edge-case reasoning, route to the kebab-case Markdown owner first,
5. if a task needs compiled/template/runtime projection data, route to the snake_case projection family.

## Activation Triggers

Read this map when:

1. bootstrap or project scaffold work needs a framework template,
2. a bounded feature/change design document must be started from the framework-owned template shape,
3. runtime prompt rendering or agent-definition rendering is being inspected,
4. instruction projection or prompt-template ownership must be resolved,
5. a task explicitly asks where templates live or which template family is canonical.

Do not read this map by default for ordinary product-spec questions.

## Routing

1. Bootstrap/project scaffolding:
   - continue to `docs/framework/templates/vida.config.yaml.template`
2. Feature/change design authoring:
   - continue to `docs/framework/templates/feature-design-document.template.md`
   - and `docs/product/spec/feature-design-and-adr-model.md`
3. Prompt rendering and worker/orchestrator template shape:
   - continue to `prompt_templates/*`
4. Machine-readable contract/definition projections:
   - continue to `instruction_contracts/*`
   - and `agent_definitions/*`
5. Project-owned future templates:
   - resolve through the active project root map rather than assuming they belong here

## Boundary Rule

1. This map owns framework template discovery.
2. It does not make all template families framework-owned by default.
3. If a future project introduces project-owned templates, the active project root map must expose them separately.
4. This map must keep the authoring/projection split explicit so paired pseudo-family names do not read as bridge-era residue or duplicate ownership.

-----
artifact_path: config/system-maps/template.map
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-14'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/template.map.md
created_at: '2026-03-10T08:45:00+02:00'
updated_at: '2026-03-14T17:15:00+02:00'
changelog_ref: template.map.changelog.jsonl
