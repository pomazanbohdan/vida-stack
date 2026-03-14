# VIDA Project Document Naming Law

Status: active product law

Purpose: define the canonical naming law for project-owned documentation under `docs/**` without collapsing project documents into the framework instruction naming grammar and without breaking GitHub-native repository/community filenames.

## Scope

This law governs:

1. `docs/product/spec/**`
2. `docs/process/**`
3. `docs/product/research/**`
4. `docs/project-memory/**`

This law does not govern:

1. framework instruction artifacts under `vida/config/instructions/**`
2. root/community repository files such as `README.md`, `CONTRIBUTING.md`, or `CODEOWNERS`
3. sidecar changelog files
4. generated operator output

Framework instruction naming remains owned by:

1. `instruction-contracts/meta.protocol-naming-grammar-protocol.md`

GitHub-native repository/community filenames remain owned by:

1. `docs/product/spec/github-public-repository-law.md`

## Core Grammar

Canonical `docs/**` naming grammar:

1. `<subject>[-<qualifier>]-<document-role>.md`

Rules:

1. `subject` carries the primary semantic topic,
2. `qualifier` is optional and narrows the topic,
3. `document-role` is mandatory unless the file is a reserved lane-root carrier,
4. the filename must expose one terminal semantic role only,
5. the directory carries lane ownership; the filename carries local semantic meaning.

## Directory Ownership Rule

Directory selection must express document ownership before filename style.

Rules:

1. `docs/product/spec/**` owns stable product law, models, plans, and contracts,
2. `docs/process/**` owns project process/runbook/operator surfaces,
3. `docs/product/research/**` owns research staging and comparative studies,
4. `docs/project-memory/**` owns project-memory records and snapshots.

If the directory owner is wrong, renaming the file alone does not make the document naming-green.

## Reserved Filename Rule

Reserved lane-root carriers inside `docs/**`:

1. `README.md`
2. `index.md`

Rules:

1. `README.md` is allowed only for a human lane-root or lane-entry surface,
2. `index.md` is allowed only for a structured index or root entrypoint,
3. do not create arbitrary extra `README.md` files when the directory is not a real lane root,
4. do not use `README.md` or `index.md` as substitutes for a missing semantic subject/role filename.

Repository/community filenames outside `docs/**` keep their native GitHub-recognized names and are not normalized by this law.

## Allowed Terminal Roles

### `docs/product/spec/**`

Allowed terminal roles:

1. `law`
2. `model`
3. `map`
4. `index`
5. `contract`
6. `plan`
7. `architecture`
8. `template`
9. `matrix`

### `docs/process/**`

Allowed terminal roles:

1. `guide`
2. `map`
3. `audit`
4. `report`
5. `conditions`
6. `index`

### `docs/product/research/**`

Allowed terminal roles:

1. `study`
2. `comparison`
3. `survey`
4. `note`
5. `report`

### `docs/project-memory/**`

Allowed terminal roles:

1. `record`
2. `snapshot`
3. `index`
4. `note`

## Subject And Qualifier Rule

1. Use lowercase kebab-case.
2. Prefer concrete topic names over vague umbrella words.
3. Keep the subject stable when the owner topic is stable.
4. Use a qualifier only when it materially disambiguates a sibling document.
5. Do not encode directory ownership again in the filename when the directory already carries that meaning.

Examples:

1. `compiled-runtime-bundle-contract.md`
2. `team-coordination-model.md`
3. `documentation-tooling-map.md`
4. `framework-three-layer-refactoring-audit.md`

## Forbidden Terminal Drift

The following words must not be introduced as new terminal document-role suffixes unless this law is explicitly extended:

1. `program`
2. `continuation`
3. `crosswalk`
4. `taxonomy`
5. `kernel`

These words may still appear inside the `subject` when semantically needed.

## Transitional Grandfathering Rule

This repository currently contains active canonical documents created before this naming law.

Rules:

1. pre-existing nonconforming filenames are tolerated temporarily,
2. new `docs/**` files created after this law must follow the new grammar immediately,
3. pre-existing filenames with non-allowlist terminal suffixes must be normalized only through bounded rename waves,
4. naming cleanup must not silently widen into mass rename churn without explicit migration scope.

## Rename Wave Rule

When a nonconforming document is renamed:

1. update the filename,
2. update `artifact_path` when the semantic naming target changed,
3. update all active canonical references and indexes,
4. preserve the same owner lane unless ownership itself is being corrected,
5. update sidecar lineage lawfully,
6. validate the bounded scope before closure.

## Closure Rule

`docs/**` naming work is closed only when:

1. the filename follows this law or is explicitly grandfathered,
2. the document lives in the correct owner directory,
3. `artifact_path` and `source_path` are aligned,
4. active indexes/maps/reference surfaces are updated,
5. the bounded documentation validation path passes.

-----
artifact_path: product/spec/project-document-naming-law
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/project-document-naming-law.md
created_at: '2026-03-12T09:00:00+02:00'
updated_at: '2026-03-12T07:32:33+02:00'
changelog_ref: project-document-naming-law.changelog.jsonl
