# VIDA Instruction Artifact Model

Status: active product instruction law

Revision: `2026-03-09`

Purpose: define the canonical product instruction/config substrate after the `_vida` cutover.

## 1. Canonical Artifact Classes

VIDA product law recognizes four peer artifact classes:

1. `Agent Definition`
2. `Instruction Contract`
3. `Prompt Template Configuration`
4. `Skill`

Runtime composition note:

1. `Flow Set` is a runtime composition artifact, not a fifth peer instruction-artifact class.
2. Flow-set selection remains a runtime/taskflow concern layered above these four core artifact classes.

## 2. Ownership

1. All four artifact classes are product-owned configuration artifacts.
2. They may be packaged into a binary, but packaged presence does not imply default activation.
3. `Agent Definition`, `Instruction Contract`, and `Prompt Template Configuration` form the core behavior/config chain for AI-agent runtime behavior.
4. `Skill` is a separate peer artifact with independent inventory, enablement, and attachment semantics.

## 3. Core Semantics

### 3.1 Agent Definition

`Agent Definition` is the umbrella assembly root.

It owns:

1. compatible role/mission identity,
2. allowable contract/template bindings,
3. activation policy references,
4. skill allowlist or attachment policy references.

### 3.2 Instruction Contract

`Instruction Contract` is the canonical behavior-law artifact.

It owns:

1. role obligations,
2. output contracts,
3. proof expectations,
4. escalation and fallback rules.

### 3.3 Prompt Template Configuration

`Prompt Template Configuration` is product configuration, not a framework-only appendix.

It owns:

1. rendering/materialization parameters,
2. prompt-surface structure,
3. bounded attachment of reusable behavioral config,
4. runtime composition metadata.

### 3.4 Skill

`Skill` is a separately managed extension artifact.

It owns:

1. focused capability payload,
2. independent packaging metadata,
3. enablement policy,
4. attach/select semantics.

## 4. Runtime Distinctions

Runtime must distinguish these postures:

1. packaged into binary,
2. present in inventory,
3. enabled by policy,
4. selectable,
5. attached to current agent/run/task,
6. active in current execution.

Rule:

1. no runtime may assume that all packaged skills are enabled by default,
2. skill inventories may be physically large,
3. explicit management surfaces are required for discovery and enablement.

## 4.1 VIDA 1.0 Runtime Contract Targets

For `VIDA 1.0`, the instruction/config substrate must support explicit multi-agent runtime behavior rather than relying on implicit chat-context inheritance.

Target requirements:

1. `Instruction Contract` and rendered runtime packets must support explicit handoff/delegation boundaries between orchestrator and worker lanes.
2. Handoff/runtime packets must support bounded context shaping so the receiving lane gets only the history and task state required for its role.
3. The agent/runtime chain must preserve explicit output and proof contracts across handoff boundaries, including independent verification lanes when route law requires them.
4. Runtime packet semantics must remain replay-safe:
   - repeated delivery of the same bounded packet must not silently widen scope,
   - retries and recovery must not require hidden chat-memory reconstruction,
   - fail-closed escalation rules must survive restart/replay.
5. Prompt Template Configuration may shape rendering and bindings for a provider/runtime, but it must not become the hidden owner of:
   - handoff law,
   - context-filtering law,
   - proof/verification law,
   - replay/recovery semantics.

External alignment references:

1. OpenAI Agents SDK overview:
   - https://developers.openai.com/api/docs/guides/agents-sdk
2. OpenAI Agents SDK handoffs:
   - https://openai.github.io/openai-agents-js/guides/handoffs/
3. LangGraph supervisor/handoff patterns:
   - https://langchain-ai.github.io/langgraphjs/reference/modules/langgraph-supervisor.html
4. Temporal durable execution:
   - https://docs.temporal.io/
5. Eventuous checkpointing:
   - https://eventuous.dev/docs/subscriptions/checkpoint/

## 5. Config Mapping

The executable law home for this model is `vida/config/instructions/**`.

Target families:

1. flat canonical Markdown authoring artifacts under `vida/config/instructions/**/*.md`
2. projected machine-readable artifacts under `vida/config/instructions/{agent_definitions,instruction_contracts,prompt_templates,skills,bundles,activation}/**`

Canonical flat naming rule:

1. active Markdown instruction artifacts stay in the root of `vida/config/instructions/`,
2. logical family placement is expressed by pseudo-directory prefixes in the filename and in `artifact_path`,
3. example: `agent-definitions/entry.orchestrator-entry.md`

## 5.1 Authoring Format

Canonical authoring rule:

1. `Agent Definition` and `Instruction Contract` artifacts must remain human-auditable and repo-readable.
2. The preferred canonical authoring format for instruction-bearing artifacts is Markdown written in natural language, following the inspectable posture of `AGENTS.md`.
3. `vida/config/instructions/**` defines the canonical location of the product-owned artifacts; it does not require YAML-only authoring.
4. YAML/JSON artifacts may exist as executable bridge or compiled runtime forms when the transitional runtime needs machine-readable loading.
5. When both human-readable and machine-readable forms exist, the human-readable Markdown artifact is the canonical authoring surface unless a stricter runtime protocol explicitly promotes the machine-readable projection.

## 5.2 Versioning And Latest-Revision Rule

Canonical rule:

1. until `vida 0.2.0` and `vida 1.0` consume the registry/runtime model directly, the repository keeps only the latest active Markdown revision for each canonical instruction artifact,
2. historical states live in adjacent `*.changelog.jsonl` files and in Git history, not as parallel active Markdown copies,
3. every canonical instruction artifact must carry explicit metadata for:
   - `artifact_version`
   - `artifact_revision`
   - `schema_version`
   - `status`
4. `artifact_version` is the semantic version boundary of the artifact,
5. `artifact_revision` identifies the current latest Markdown revision for that version,
6. `schema_version` versions the footer/changelog metadata contract itself,
7. runtime and migration logic must treat unresolved or incompatible version tuples as fail-closed conditions.

## 6. Historical Sources

This spec absorbs and supersedes product-instruction semantics previously scattered across:

1. `docs/process/framework-source-lineage-index.md`
2. `agent-definitions/model.agent-definitions-contract.md`
3. `docs/product/spec/instruction-artifact-model.md`
4. `prompt-templates/worker.packet-templates.md`
5. deleted historical YAML template/schema input formerly at `docs/framework/templates/instruction-contract.yaml`
6. deleted historical YAML template/schema input formerly at `docs/framework/templates/prompt-template-config.yaml`

-----
artifact_path: product/spec/instruction-artifact-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: docs/product/spec/instruction-artifact-model.md
created_at: '2026-03-09T20:28:59+02:00'
updated_at: '2026-03-12T08:12:40+02:00'
changelog_ref: instruction-artifact-model.changelog.jsonl
