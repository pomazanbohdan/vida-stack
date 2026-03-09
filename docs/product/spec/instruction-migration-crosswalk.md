# Instruction Migration Crosswalk

Status: migration reference

Revision: `2026-03-09`

Purpose: preserve the instruction-source trail while moving current canon into `docs/product/spec/**` and `vida/config/instructions/**`.

## Source Crosswalk

1. `agent-definition-protocol.md`
   Current role: historical framework/runtime source
   Current canon: [instruction-artifact-model.md](/home/unnamed/project/vida-stack/docs/product/spec/instruction-artifact-model.md)
2. `docs/framework/plans/vida-0.3-instruction-kernel-spec.md`
   Current role: historical source artifact
   Current canon: [instruction-artifact-model.md](/home/unnamed/project/vida-stack/docs/product/spec/instruction-artifact-model.md)
3. `docs/framework/templates/instruction-contract.yaml`
   Current role: historical template/schema input
   Current executable law target: `vida/config/instructions/instruction_contracts/**`
4. `docs/framework/templates/prompt-template-config.yaml`
   Current role: historical template/schema input
   Current executable law target: `vida/config/instructions/prompt_templates/**`
5. `vida/config/instructions/agent-definitions.protocol.md`
   Current role: historical framework seed artifact
   Current executable law target: `vida/config/instructions/*.md`
6. `docs/product/spec/instruction-artifact-model.md`
   Current role: historical framework seed artifact
   Current executable law target: `vida/config/instructions/*.md`
7. `vida/config/instructions/prompt-templates.worker-packet-templates.md`
   Current role: historical framework seed artifact
   Current executable law target: `vida/config/instructions/*.md`
8. `2026-03-08-agentic-cheap-worker-prompt-pack.md`
   Current role: historical script-era prompt pack
   Current canonical prompt target: `vida/config/instructions/prompt-templates.cheap-worker-prompt-pack.md`
9. `vida/config/instructions/prompt-templates.worker-packet-templates.md`
   Current role: canonical prompt authoring artifact
   Current canonical prompt target: `vida/config/instructions/prompt-templates.worker-packet-templates.md`
10. `vida/config/instructions/agent-definitions.orchestrator-entry.md`
   Current role: canonical lane-entry authoring artifact
   Current canonical instruction target: `vida/config/instructions/agent-definitions.orchestrator-entry.md`
11. `vida/config/instructions/agent-definitions.worker-entry.md`
   Current role: canonical lane-entry authoring artifact
   Current canonical instruction target: `vida/config/instructions/agent-definitions.worker-entry.md`

## Preservation Rule

1. The old `_vida` artifacts remain source evidence until final cleanup.
2. New current law must be written to `docs/product/spec/**` and flat canonical Markdown artifacts in `vida/config/instructions/*.md`.
3. Historical artifacts must not silently remain the only authoritative instruction source.
4. Instruction-bearing artifacts should be authored in human-readable Markdown in `vida/config/instructions/*.md`; YAML/JSON bridge artifacts may coexist for transitional runtime consumption.
5. Only the latest active Markdown revision remains in the canonical tree; earlier states live in adjacent `*.changelog.jsonl` files and Git history.

-----
artifact_path: product/spec/instruction-migration-crosswalk
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/product/spec/instruction-migration-crosswalk.md
created_at: 2026-03-09T20:28:59+02:00
updated_at: 2026-03-09T22:51:59+02:00
changelog_ref: instruction-migration-crosswalk.changelog.jsonl
