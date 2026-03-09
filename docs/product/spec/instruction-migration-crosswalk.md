# Instruction Migration Crosswalk

Status: migration reference

Revision: `2026-03-09`

Purpose: preserve the instruction-source trail while moving current canon into `docs/product/spec/**` and `vida/config/instructions/**`.

## Source Crosswalk

1. `docs/framework/history/_vida-source/docs/agent-definition-protocol.md`
   Current role: historical framework/runtime source
   Current canon: [instruction-artifact-model.md](/home/unnamed/project/vida-stack/docs/product/spec/instruction-artifact-model.md)
2. `docs/framework/history/plans/2026-03-08-vida-0.3-instruction-kernel-spec.md`
   Current role: historical source artifact
   Current canon: [instruction-artifact-model.md](/home/unnamed/project/vida-stack/docs/product/spec/instruction-artifact-model.md)
3. `docs/framework/templates/instruction-contract.yaml`
   Current role: historical template/schema input
   Current executable law target: `vida/config/instructions/instruction_contracts/**`
4. `docs/framework/templates/prompt-template-config.yaml`
   Current role: historical template/schema input
   Current executable law target: `vida/config/instructions/prompt_templates/**`
5. `docs/framework/history/_vida-source/instructions/framework/agent-definition.md`
   Current role: historical framework seed artifact
   Current executable law target: `vida/config/instructions/agent_definitions/**`
6. `docs/framework/history/_vida-source/instructions/framework/instruction-contract.md`
   Current role: historical framework seed artifact
   Current executable law target: `vida/config/instructions/instruction_contracts/**`
7. `docs/framework/history/_vida-source/instructions/framework/prompt-template-config.md`
   Current role: historical framework seed artifact
   Current executable law target: `vida/config/instructions/prompt_templates/**`
8. `docs/framework/history/_vida-source/docs/research/2026-03-08-agentic-cheap-worker-prompt-pack.md`
   Current role: historical script-era prompt pack
   Current canonical prompt target: `vida/config/instructions/prompt_templates/cheap-worker-prompt-pack.md`
9. `docs/framework/worker-packet-templates.md`
   Current role: framework consumer/pointer document
   Current canonical prompt target: `vida/config/instructions/prompt_templates/worker-packet-templates.md`

## Preservation Rule

1. The old `_vida` artifacts remain source evidence until final cleanup.
2. New current law must be written to `docs/product/spec/**` and `vida/config/instructions/**`.
3. Historical artifacts must not silently remain the only authoritative instruction source.
4. Instruction-bearing artifacts should be authored in human-readable Markdown in `vida/config/instructions/**`; YAML/JSON bridge artifacts may coexist for transitional runtime consumption.
