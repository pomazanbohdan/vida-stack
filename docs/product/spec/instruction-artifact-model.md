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

1. `agent_definitions/**`
2. `instruction_contracts/**`
3. `prompt_templates/**`
4. `skills/**`
5. `bundles/**`
6. `activation/**`

## 5.1 Authoring Format

Canonical authoring rule:

1. `Agent Definition` and `Instruction Contract` artifacts must remain human-auditable and repo-readable.
2. The preferred canonical authoring format for instruction-bearing artifacts is Markdown written in natural language, following the inspectable posture of `AGENTS.md`.
3. `vida/config/instructions/**` defines the canonical location of the product-owned artifacts; it does not require YAML-only authoring.
4. YAML/JSON artifacts may exist as executable bridge or compiled runtime forms when the transitional runtime needs machine-readable loading.
5. When both human-readable and machine-readable forms exist, the human-readable Markdown artifact is the canonical authoring surface unless a stricter runtime protocol explicitly promotes the machine-readable projection.

## 6. Historical Sources

This spec absorbs and supersedes product-instruction semantics previously scattered across:

1. `docs/framework/history/plans/2026-03-08-vida-0.3-instruction-kernel-spec.md`
2. `docs/framework/history/_vida-source/instructions/framework/agent-definition.md`
3. `docs/framework/history/_vida-source/instructions/framework/instruction-contract.md`
4. `docs/framework/history/_vida-source/instructions/framework/prompt-template-config.md`
5. `docs/framework/templates/instruction-contract.yaml`
6. `docs/framework/templates/prompt-template-config.yaml`
