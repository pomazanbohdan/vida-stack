# VIDA Skill Management And Activation Law

Status: active product direction

Revision: `2026-03-09`

Purpose: define bounded product law for skill inventory, packaging, and activation without mixing skills into the core instruction-contract chain.

## 1. Skill Posture

`Skill` is a separate product artifact class.

Rules:

1. skills may be packaged alongside core instruction artifacts,
2. skills are not all available by default,
3. skills require explicit inventory and activation handling,
4. skills remain separate from `Instruction Contract` and `Prompt Template Configuration`.

## 2. Required Runtime Distinctions

The product must distinguish:

1. installed or packaged skills,
2. skills visible in runtime inventory,
3. skills enabled by default,
4. skills attached to a specific agent or run,
5. skills active in current execution.

## 3. Policy Direction

Default policy:

1. packaged skills are discoverable only through explicit product functionality,
2. default activation is deny-by-default unless law/config explicitly enables a skill,
3. attachment may be driven by agent definition, route context, or operator selection,
4. large inventories must not imply large default prompt surfaces.

## 4. Config Mapping

Primary executable-law homes:

1. `skills/**`
2. `activation/**`
3. `bundles/**`

## 5. Transitional Runtime Mapping

the active TaskFlow runtime family is the first transitional runtime expected to consume this split:

1. inventory,
2. enablement,
3. selection,
4. attachment,
5. non-default activation boundaries.

-----
artifact_path: product/spec/skill-management-and-activation-law
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/skill-management-and-activation-law.md
created_at: '2026-03-09T20:28:59+02:00'
updated_at: '2026-03-12T07:48:27+02:00'
changelog_ref: skill-management-and-activation-law.changelog.jsonl
