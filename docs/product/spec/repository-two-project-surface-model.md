# Repository Two-Project Surface Model

Status: active product law

Purpose: define how this repository temporarily carries one active project surface and one extracted secondary project bundle without collapsing framework canon, active project documentation, and preserved second-project material into one mixed tree.

## 1. Current Repository Posture

This repository currently contains:

1. one active project surface: `vida-stack`
2. one extracted secondary project bundle: `vida-mobile`

The active project surface remains the canonical project/documentation target for bootstrap and normal work in this repository.

The extracted secondary project bundle is preserved locally for later copy-out into its own repository and must not be treated as the active current-project canon here.

## 2. Active Project Surface

The active project surface is:

1. root repository code and docs for `vida-stack`
2. active project documentation under:
   - `docs/product/**`
   - `docs/process/**`
   - `docs/project-memory/**`

Bootstrap rule:

1. `AGENTS.sidecar.md` maps only the active current project surface.
2. The active current-project docs map is the default project-doc target after bootstrap.
3. Secondary project bundles are not part of the default bootstrap read path unless the task explicitly targets them.

## 3. Secondary Project Bundle

The extracted secondary bundle currently lives under:

1. `projects/vida-mobile/**`

Its role is:

1. preserve project-specific material that was previously mixed into framework surfaces,
2. keep that material available for later copy-out into a dedicated `vida-mobile` repository,
3. avoid deleting or publishing that material during framework cleanup.

Rules:

1. `projects/vida-mobile/**` is local preservation state, not active framework canon.
2. `projects/vida-mobile/**` is not the default project-doc bootstrap target.
3. Framework cleanup may point to this bundle as a preserved extracted target, but must not move its content back into `vida/config/instructions/**`.
4. Secondary project bundles must not be committed or published as part of public framework history unless explicitly approved for extraction/publishing.

## 4. Root Config Rule

The root `vida.config.yaml` remains:

1. the active runtime/config surface for the current repository,
2. the current TaskFlow configuration surface,
3. the forward-compatible root settings surface for future VIDA runtime integration,
4. a possible shared config surface for future Codex/runtime integration where canon later allows it.

Rules:

1. root `vida.config.yaml` is not moved into `projects/vida-mobile/**` while this repository remains the active runtime host,
2. extracted project bundles may keep snapshot copies of their historical overlay/config state,
3. framework-owned overlay examples or bridge artifacts under `vida/**` must remain generic and must not replace the active root config.

## 5. Map Routing Rule

Repository map routing must distinguish:

1. framework discovery,
2. active current-project discovery,
3. extracted secondary project bundles.

Required behavior:

1. framework maps may mention the existence of extracted secondary project bundles as repository topology,
2. framework maps must not treat those bundles as the default active project-doc target,
3. the active current-project docs map must stay in `AGENTS.sidecar.md` and current project maps under `docs/**`,
4. extracted bundles should have their own local map/readme surfaces under `projects/<name>/**`.

## 6. Ownership Rule

Ownership remains:

1. framework canon -> `AGENTS.md`, `vida/config/**`, framework maps and protocols
2. active current project -> `docs/**` and active repo-owned project docs
3. extracted secondary bundle -> `projects/vida-mobile/**`

Forbidden mixing:

1. do not keep project-specific runbook bodies inside `vida/config/instructions/**`,
2. do not treat extracted bundle docs as active current-project canon by default,
3. do not use framework maps as the owner of second-project operational detail.

## 7. Minimum Healthy Outcome

The repository is considered correctly separated when:

1. bootstrap resolves framework + active current-project maps without touching extracted bundles by default,
2. `vida-mobile` material remains preserved and discoverable in `projects/vida-mobile/**`,
3. framework surfaces remain generic,
4. root `vida.config.yaml` remains the active runtime/config surface for this repository,
5. later copy-out of `vida-mobile` can happen without re-extracting project knowledge from framework docs.

-----
artifact_path: product/spec/repository-two-project-surface-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: docs/product/spec/repository-two-project-surface-model.md
created_at: '2026-03-10T08:30:00+02:00'
updated_at: '2026-03-10T08:30:00+02:00'
changelog_ref: repository-two-project-surface-model.changelog.jsonl
