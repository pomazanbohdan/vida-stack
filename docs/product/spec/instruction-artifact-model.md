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

## 5. Config Mapping

The executable law home for this model is `vida/config/instructions/**`.

Target families:

1. flat canonical Markdown authoring artifacts in `vida/config/instructions/*.md`
2. projected machine-readable artifacts in `vida/config/instructions/{agent_definitions,instruction_contracts,prompt_templates,skills,bundles,activation}/**`

Canonical flat naming rule:

1. active Markdown instruction artifacts stay in the root of `vida/config/instructions/`,
2. logical family placement is expressed by pseudo-directory prefixes in the filename and in `artifact_path`,
3. example: `vida/config/instructions/agent-definitions.orchestrator-entry.md`

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

1. `docs/framework/plans/vida-0.3-instruction-kernel-spec.md`
2. `vida/config/instructions/agent-definitions.protocol.md`
3. `docs/product/spec/instruction-artifact-model.md`
4. `vida/config/instructions/prompt-templates.worker-packet-templates.md`
5. `docs/framework/templates/instruction-contract.yaml`
6. `docs/framework/templates/prompt-template-config.yaml`

-----
artifact_path: product/spec/instruction-artifact-model
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/product/spec/instruction-artifact-model.md
created_at: 2026-03-09T20:28:59+02:00
updated_at: 2026-03-09T22:51:59+02:00
changelog_ref: instruction-artifact-model.changelog.jsonl
