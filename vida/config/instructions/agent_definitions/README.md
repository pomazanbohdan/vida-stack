Product config home for `Agent Definition` artifacts.

Current runtime-bearing artifacts:

1. `framework_orchestrator.yaml`
2. `framework_worker.yaml`

Canonical authoring format:

1. Human-readable Markdown is the canonical authoring surface for instruction-bearing artifacts in this family.
2. YAML files in this directory are transitional executable bridge artifacts where runtime loading still expects machine-readable config.

Bootstrap note:

1. Markdown entry contracts in `AGENTS.md` and `docs/framework/*ENTRY*.MD` remain the current boot surface.
2. Product-law migration moves executable role assembly into this directory without silently deleting the bootstrap layer.
