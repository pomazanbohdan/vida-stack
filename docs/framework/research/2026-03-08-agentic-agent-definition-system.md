# Agentic Agent Definition System

**Purpose:** Define the canonical umbrella layer for role logic in VIDA so the system is built from explicit, versioned, deterministic agent definitions rather than ad hoc prompts or silent model improvisation.

**Core claim:** VIDA should not treat prompts as loose prose. It should treat agent behavior as a structured system.

---

## Web Source Basis

- Microsoft Learn, *Create an Agent from a Semantic Kernel Template* — https://learn.microsoft.com/en-us/semantic-kernel/frameworks/agent/agent-templates
- OpenAI, *Model Spec (2025-10-27)* — https://model-spec.openai.com/2025-10-27
- OpenAI, *Agent Builder* — https://platform.openai.com/docs/guides/agent-builder
- Anthropic, *Create custom subagents* — https://docs.anthropic.com/en/docs/claude-code/sub-agents
- Google Cloud, *Choose a design pattern for your agentic AI system* — https://docs.cloud.google.com/architecture/choose-design-pattern-agentic-ai-system

---

## 1. Canonical Terminology

### `Agent Definition`

The umbrella artifact that defines how an agent exists and behaves in the system.

It should contain at minimum:

1. identity
2. role class
3. instruction contract
4. tool and permission policy
5. output contract
6. fallback and escalation contract
7. rendering/configuration surface
8. version metadata
9. conformance / eval hooks

### `Instruction Contract`

The canonical behavioral law for the agent.

It is the normative source for:

1. what the agent must do
2. what the agent must not do
3. what inputs it accepts
4. what evidence it requires
5. what decisions are allowed
6. what fallback ladder exists
7. when it must escalate
8. what output schema it owes downstream

Rule:
- the `Instruction Contract` is the logic source of truth

### `Prompt Template Configuration`

The render/config layer that materializes the instruction contract for a concrete runtime, provider, or templating system.

It may include:

1. system prompt body
2. parameter placeholders
3. template variables
4. model/runtime bindings
5. tool exposure metadata
6. environment-specific configuration

Rule:
- the `Prompt Template Configuration` must not become the canonical owner of behavior
- it renders the contract; it does not replace it

---

## 2. Relation Model

Use this canonical hierarchy:

1. `Agent Definition`
   - umbrella system object
2. `Instruction Contract`
   - canonical behavior law inside the definition
3. `Prompt Template Configuration`
   - runtime-specific rendering/configuration of the contract

Supporting inputs:

1. `Role Profile`
   - stable identity and behavioral stance
2. `Tool/Permission Policy`
   - what the agent can do
3. `Output Contract`
   - what the agent must return
4. `Fallback/Escalation Contract`
   - what happens when the primary path fails

Compact formula:

`Agent Definition = Role Profile + Instruction Contract + Tool/Permission Policy + Output Contract + Fallback/Escalation Contract + Prompt Template Configuration + Versioning + Conformance`

---

## 3. What The Sources Say

### Microsoft

- agent behavior can be defined from a template
- `PromptTemplateConfig` is a structured and reusable way to define behavior
- agent definitions can be externalized from code and reused

VIDA implication:
- use a structured definition object, not freeform prompt prose as the only source

### OpenAI

- model behavior should follow a clear chain of command and agreed scope of autonomy
- workflows should be versioned objects with typed edges and typed step contracts

VIDA implication:
- behavior hierarchy and allowed scope must be explicit
- workflow/versioned definition should be treated as first-class runtime surface

### Anthropic

- subagents are defined through custom system prompt, tool access, permissions, and separate context
- the system prompt body is only one part of the subagent definition

VIDA implication:
- role logic must include prompt, tools, permissions, and context boundaries together

### Google

- deterministic workflows are those with known, predefined paths
- custom logic is appropriate when explicit conditional routing is required

VIDA implication:
- predefined state transitions and explicit conditional routing are valid design goals
- "the agent will decide somehow" is not a sufficient execution model

---

## 4. Deterministic Runtime Laws

VIDA should adopt these laws for all agent definitions:

1. undefined behavior is forbidden by default
2. no implied behavior
3. no silent autonomy expansion
4. no role authority hidden in tone or persona
5. fallback must be explicit and ordered
6. escalation triggers must be explicit
7. output contracts should be schema-oriented whenever possible
8. behavior changes must be versioned

Canonical form:

- `if condition -> action`
- `if condition missing -> block`
- `if primary path fails -> fallback N`
- `if no lawful fallback -> escalate`
- `if escalation not allowed -> fail closed`

---

## 5. What Belongs In The Instruction Contract

Minimum required sections:

1. mission
2. scope boundary
3. mandatory reads
4. input contract
5. decision table
6. allowed actions
7. forbidden actions
8. tool/permission rules
9. fallback ladder
10. escalation rules
11. output contract
12. proof requirements

Recommended representation:

- prose for human readability
- tables for deterministic routing
- machine-readable schema for validation

---

## 6. What Belongs In Prompt Template Configuration

Keep this layer bounded to rendering/runtime concerns:

1. template syntax
2. system prompt assembly
3. parameter binding
4. provider-specific runtime fields
5. local environment substitutions
6. model selection hints

Do not let this layer own:

1. business logic
2. role authority
3. fallback semantics
4. approval semantics
5. proof burden

Those belong upstream in the instruction contract or broader agent definition.

---

## 7. Promotion Guidance

The target runtime chain should become:

1. research and terminology docs
2. agent-definition protocol
3. instruction-contract template/schema
4. prompt-template configuration schema
5. rendered runtime packet / role packet
6. tests and conformance evals

Framework-owned runtime artifacts created from this doc:

1. `docs/framework/agent-definition-protocol.md`
2. `docs/framework/templates/instruction-contract.yaml`
3. `docs/framework/templates/prompt-template-config.yaml`
4. `docs/framework/history/_vida-source/tests/test_agent_definition_contract.py`

---

## 8. Anti-Patterns

Avoid:

1. treating `prompt text` as the full system
2. letting provider-specific config own canonical logic
3. mixing tone with permissions or approval authority
4. undocumented fallback behavior
5. role behavior that exists only in chat habit or operator memory

---

## 9. Canonical VIDA Rule

Use this sentence as the shortest stable summary:

`Agent Definition` is the umbrella system object.
`Instruction Contract` is the canonical behavior law.
`Prompt Template Configuration` is the rendering/configuration layer.

Behavior is explicit, versioned, allowlisted, and fail-closed by default.
