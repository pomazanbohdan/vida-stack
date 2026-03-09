# Project Bootstrap Protocol (PBP)

Purpose: define how VIDA can audit and scaffold the minimal project-owned artifact layer required for autonomous execution in a new or transferred repository.

## Core Contract

1. Framework policy stays in `AGENTS.md` and `docs/framework/*`.
2. Project bootstrap creates or audits only project-owned artifacts in `docs/*`, `docs/process/*`, and root `vida.config.yaml`.
3. Bootstrap must never overwrite existing project files unless explicitly forced.
4. Bootstrap is the first self-reproduction layer, not a replacement for project-specific implementation work.

Framework template rule:

1. project-owned artifacts may be scaffolded from framework-owned templates,
2. canonical framework templates live in `docs/framework/templates/*`,
3. current external-artifact template owner: `docs/framework/templates/vida.config.yaml.template`.

## Input Surface

Bootstrap reads:

1. root `vida.config.yaml`
2. `docs/framework/project-overlay-protocol.md`
3. optional `project_bootstrap.*` settings from the root overlay

Fail-fast rule:

1. if root `vida.config.yaml` exists, bootstrap must validate schema before emitting contract, audit, or scaffold output,
2. invalid overlay schema blocks bootstrap commands until the project artifact is corrected.

## Bootstrap Outputs

Minimum scaffoldable project artifacts:

1. `vida.config.yaml`
2. `docs/README.md`
2. `docs/architecture.md`
3. `docs/decisions.md`
4. `docs/environments.md`
5. host-project operations doc resolved by the emitted overlay/bootstrap contract
6. `docs/process/agent-system.md`

## Commands

```bash
vida-v0 config validate
vida-v0 boot read-contract lean
vida-v0 boot snapshot --json
vida-v0 system snapshot
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
