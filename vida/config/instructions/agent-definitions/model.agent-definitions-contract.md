# Agent Definition Protocol

Purpose: define the canonical framework/runtime contract for product-owned instruction artifacts in VIDA.

This protocol promotes the research-layer `Agent Definition System` into framework-owned runtime surfaces.

## Core Contract

1. VIDA agents are defined by explicit, versioned, allowlisted artifacts.
2. Undefined behavior is forbidden by default.
3. `Instruction Contract` is the canonical logic source.
4. `Prompt Template Configuration` is a product-owned rendering/configuration artifact.
5. `Skill` is a separate peer artifact with independent activation and attachment semantics.
6. Provider-specific prompt/config data must not silently become the source of truth for role behavior.

## Canonical Artifact Set

Required runtime artifacts:

1. `vida/config/instructions/agent-definitions/model.agent-definitions-contract.md`
2. `vida/config/instructions/agent-definitions/role.role-profile-contract.md`
3. `docs/product/spec/instruction-artifact-model.md`
4. `docs/product/spec/skill-management-and-activation-law.md`
5. `vida/config/instructions/prompt-templates/worker.packet-templates.md`
6. `vida/config/instructions/prompt-templates/cheap-worker.prompt-pack-reference.md`
7. `vida/config/instructions/skills/`

Upstream supporting artifacts:

1. `docs/process/framework-source-lineage-index.md`
2. `docs/product/spec/agent-role-skill-profile-flow-model.md`
3. `vida/config/instructions/system-maps/template.map.md`

## Canonical Terminology

Use exactly this hierarchy:

1. `Agent Definition`
2. `Instruction Contract`
3. `Prompt Template Configuration`
4. `Skill`
5. `Role Profile`
6. `Flow Set`

Rules:

1. `Agent Definition` is the umbrella system object.
2. `Instruction Contract` owns behavioral logic.
3. `Prompt Template Configuration` renders the contract for a runtime/provider and remains product-owned config.
4. `Skill` is a separately managed capability artifact, not an implicit field inside the contract or template.
5. `Role Profile` remains upstream identity/stance input, not the full runtime behavior object by itself.
6. `Role Profile` canonical contract lives in `vida/config/instructions/agent-definitions/role.role-profile-contract.md`.
7. `Flow Set` is a runtime composition artifact, not a fifth core instruction artifact class.

## Deterministic Behavior Law

VIDA agent definitions must follow these laws:

1. no implied behavior
2. no silent autonomy expansion
3. no authority hidden in tone/persona wording
4. no fallback without an explicit ladder
5. no escalation without an explicit trigger
6. no closure without declared output and proof obligations

Canonical decision form:

1. `if condition -> action`
2. `if condition missing -> block`
3. `if primary path fails -> fallback_n`
4. `if no lawful fallback -> escalate`
5. `if escalation unavailable -> fail_closed`

## Instruction Contract Contract

Every instruction contract must declare at minimum:

1. `contract_id`
2. `version`
3. `role_id`
4. `mission`
5. `scope_boundary`
6. `mandatory_reads`
7. `input_contract`
8. `decision_rules`
9. `allowed_actions`
10. `forbidden_actions`
11. `tool_permission_policy`
12. `fallback_ladder`
13. `escalation_rules`
14. `output_contract`
15. `proof_requirements`

Required logic rules:

1. `decision_rules` must be explicit, not left to generic prose.
2. `forbidden_actions` must include behavior outside scope.
3. `fallback_ladder` must be ordered.
4. `output_contract` must be specific enough for downstream verification.
5. `proof_requirements` must define what evidence is needed before completion.

## Prompt Template Configuration Contract

Every prompt template configuration must declare at minimum:

1. `config_id`
2. `version`
3. `instruction_contract_ref`
4. `rendering_target`
5. `template_format`
6. `system_prompt_template`
7. `parameter_bindings`
8. `runtime_bindings`
9. `tool_exposure`
10. `output_rendering`

Rendering rules:

1. `instruction_contract_ref` is mandatory.
2. Rendering config may bind values, but must not redefine logic.
3. Tool exposure must stay compatible with the referenced instruction contract.
4. If a provider requires extra prompt scaffolding, add it here rather than mutating the instruction contract.

## Agent Definition Assembly

The runtime assembly model is:

1. role profile provides identity and stance
2. instruction contract provides logic
3. prompt template configuration renders the logic for runtime use
4. skill attachment is separate from the core instruction chain
5. output contract and proof requirements remain inspectable after rendering
6. flow selection remains separate from role identity and prompt rendering

Compact formula:

`agent_definition = role_profile + instruction_contract + prompt_template_configuration + skill attachments + permission policy + output/proof contract`

Project extension note:

1. project-owned roles, skills, profiles, and flow sets may be enabled through `vida.config.yaml` and project-owned registries,
2. those project extensions must remain additive and must resolve back to framework role law, framework instruction contracts, and framework runtime safety invariants,
3. flow sets are runtime composition artifacts and must not be stored as hidden behavior inside one role profile or one prompt template.

## Validation Expectations

At minimum, the framework must be able to validate:

1. required fields exist in the product config artifacts
2. terminology hierarchy is not inverted
3. deterministic sections are present
4. packaged, enabled, attached, and active semantics stay distinct

## Promotion Rule

When a role becomes runtime-bearing:

1. define or reference the role profile
2. create or attach the instruction contract
3. create or attach the prompt template configuration
4. declare allowed skill attachment surfaces
5. add conformance checks
6. then allow the role into routed execution surfaces

## Anti-Patterns

Forbidden anti-patterns:

1. storing canonical role logic only in provider prompt text
2. mixing permissions into vague style guidance
3. leaving fallback unspecified
4. calling a prose note an executable contract
5. letting runtime behavior depend on chat memory

## Verification

Minimum proof:

1. instruction artifact model exists
2. skill management law exists
3. product config families exist
4. protocol index references this protocol

-----
artifact_path: config/instructions/agent-definitions/model.agent-definitions.contract
artifact_type: agent_definition
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/agent-definitions/model.agent-definitions-contract.md
created_at: '2026-03-09T12:00:46+02:00'
updated_at: '2026-03-12T11:47:59+02:00'
changelog_ref: model.agent-definitions-contract.changelog.jsonl
