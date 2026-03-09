Product config home for `Instruction Contract` artifacts.

Current runtime-bearing artifacts:

1. `framework_base.yaml` for minimal/shared schema anchoring
2. `framework_orchestrator.yaml` for orchestrator behavior law
3. `framework_worker.yaml` for worker-lane behavior law

Canonical authoring format:

1. Human-readable Markdown is the canonical authoring surface for instruction-bearing artifacts in this family.
2. YAML files in this directory are transitional executable bridge artifacts where runtime loading still expects machine-readable config.
