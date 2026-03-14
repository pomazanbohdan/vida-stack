# Release Build Packaging Law

Status: active product law

Purpose: define the canonical package-composition rule for public VIDA release archives so packaged releases contain only install-ready runtime surfaces and their direct dependencies, without bundling repository-development artifacts, parallel implementation tracks, or project-owned canon that is not part of the shipped runtime.

## Core Rule

A public VIDA release archive must contain only:

1. install-ready runtime surfaces,
2. direct runtime dependencies required for installation or execution,
3. the framework bootstrap carriers needed to enter the packaged runtime safely.

It must not contain:

1. project-owned documentation canon,
2. repository-development artifacts,
3. parallel implementation tracks that are not part of the shipped runtime line,
4. local backlog, proving, or engineering-only artifacts,
5. root-level project surfaces that are useful for repository development but not required for installation or execution.

## Package Authority Rule

Release packaging is not a repository snapshot.

Rules:

1. the release archive is a bounded runtime payload,
2. repository completeness is not a valid reason to ship unrelated development surfaces,
3. if a surface is not required to install, bootstrap, or run the shipped runtime, it must stay out of the archive,
4. public release notes, GitHub release pages, and the installer asset may describe the runtime, but the archive itself must stay minimal.

## Current 0.2.x Package Rule

For the `0.2.x` proving line, the archive may contain only:

1. `AGENTS.md`
2. a clean `AGENTS.sidecar.md` scaffold for the external project owner
3. `.codex/`
   - packaged project-local runtime configuration surface consumed by installed execution
4. `vida/`
   - framework bootstrap carriers
   - framework protocol/config surfaces
5. `bin/vida`
   - the compiled runtime binary for taskflow and docflow surfaces
6. bounded install assets required by installed runtime bootstrap
   - `install/assets/vida.config.yaml.template`
7. direct runtime dependency files required by the shipped runtime

## Explicit Exclusion Rule

For the `0.2.x` archive, exclude at minimum:

1. `docs/**`
2. `crates/**`
3. `scripts/**`
4. `install/**` except the packaged runtime-bootstrap template assets required by the installed release
5. repository-local `vida.config.yaml`
6. retired legacy runtime subtrees such as `taskflow-v0/**` or `codex-v0/**`
7. `.beads/**`
8. `.vida/**`
9. `_temp/**`
10. root repository narrative files that are not required for installation or execution

## Installer Boundary Rule

The installer may exist as a separate release asset.

Rules:

1. installer lifecycle logic does not need to live inside the versioned runtime archive,
2. if the installed `vida` wrapper needs install-management commands such as `doctor`, `upgrade`, or `use`, those commands must route through an installer-management surface outside the versioned archive tree,
3. the versioned archive must not absorb installer internals only to make wrapper dispatch work.

## Sidecar Packaging Rule

The packaged `AGENTS.sidecar.md` must be a clean external-project scaffold.

Rules:

1. it must not carry `vida-stack`-specific project pointers,
2. it must not point into `docs/project-root-map.md` or other repository-owned current-project surfaces,
3. it may contain placeholders and rules for how an external user should fill the sidecar,
4. repository-local `AGENTS.sidecar.md` may remain project-specific in the development repository, but it must not be shipped unchanged in public runtime archives.

## Metadata Rule

Build metadata used by CI or installer verification should stay outside the minimal runtime payload unless it is directly required by installed runtime behavior.

Rules:

1. release manifests, packaging diagnostics, and archive validation helpers should prefer separate build outputs or CI-side checks,
2. they should not be bundled into the runtime archive by default if the installed runtime does not consume them.

## Current Packaging Interpretation

For the active proving line:

1. the shipped public proof runtimes are the compiled `taskflow-v0` binary and the packaged `codex-v0` runtime subtree,
1. the shipped public runtime is the compiled `vida` binary,
2. `vida/` provides the framework bootstrap and protocol substrate it depends on,
3. `.codex/` is included because the installed runtime consumes that project-local configuration surface directly,
4. `install/assets/vida.config.yaml.template` is included because the installer must scaffold `vida.config.yaml` into the installed release root when it is absent,
5. Rust `crates/**` are repository implementation work and must stay outside the public archive,
6. project docs remain canonical for repository development, but they are not part of the shipped runtime payload.

-----
artifact_path: product/spec/release-build-packaging-law
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/release-build-packaging-law.md
created_at: '2026-03-12T12:28:00+02:00'
updated_at: '2026-03-12T19:00:00+02:00'
changelog_ref: release-build-packaging-law.changelog.jsonl
