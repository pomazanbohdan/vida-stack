# Project Bootstrap Protocol (PBP)

Purpose: define how VIDA can audit and scaffold the minimal project-owned artifact layer required for autonomous execution in a new or transferred repository.

## Core Contract

1. Framework policy stays in `AGENTS.md` and `vida/config/instructions/*`.
2. Project bootstrap creates or audits only project-owned artifacts in `docs/*`, `docs/process/*`, and root `vida.config.yaml`.
3. Bootstrap must never overwrite existing project files unless explicitly forced.
4. Bootstrap is the first self-reproduction layer, not a replacement for project-specific implementation work.

Framework template rule:

1. project-owned artifacts may be scaffolded from framework-owned templates,
2. the active framework scaffold template for root project activation is `docs/framework/templates/vida.config.yaml.template`,
3. project bootstrap must treat that template as the canonical target during project initialization and project-config update work.

## Input Surface

Bootstrap reads:

1. root `vida.config.yaml`
2. `vida/config/instructions/runtime-instructions/bridge.project-overlay-protocol.md`
3. optional `project_bootstrap.*` settings from the root overlay

Fail-fast rule:

1. if root `vida.config.yaml` exists, bootstrap must validate schema before emitting contract, audit, or scaffold output,
2. invalid overlay schema blocks bootstrap commands until the project artifact is corrected.

## Bootstrap Outputs

Minimum scaffoldable project artifacts:

1. `vida.config.yaml`
2. project docs root readme/map
3. project architecture document
4. project decisions document
5. project environment/operations notes when the project needs them
6. host-project operations doc resolved by the emitted overlay/bootstrap contract
7. project process/agent-system doc when the project bootstrap contract declares it

## Commands

```bash
taskflow-v0 config validate
taskflow-v0 boot read-contract lean
taskflow-v0 boot snapshot --json
taskflow-v0 system snapshot
```

## Machine-Readable Contract

Bootstrap resolves one contract object:

1. project id
2. documentation language
3. required artifact paths
4. launch-confirmation policy
5. scaffold-missing policy
6. activated framework bundles

This contract is the portable basis for project self-reproduction.

## Self-Reproduction Boundary

Bootstrap can:

1. scaffold missing project docs/runbooks,
2. emit a machine-readable project contract,
3. verify that the project-owned artifact layer is present.

Bootstrap cannot:

1. infer real business requirements,
2. replace live API validation,
3. bypass user launch confirmation for implementation,
4. invent project-specific executable commands beyond the bootstrap templates.

## Verification

Minimum proof:

1. `emit-contract` returns a complete contract,
2. overlay schema validation passes when the file exists,
3. `audit` reports missing/present required artifacts,
4. `scaffold` creates only missing files unless `--force` is used.
5. root `vida.config.yaml` is rendered from the framework template when missing.

-----
artifact_path: config/command-instructions/project-bootstrap.protocol
artifact_type: command_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/command-instructions/execution.project-bootstrap-protocol.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-12T08:12:40+02:00'
changelog_ref: execution.project-bootstrap-protocol.changelog.jsonl
