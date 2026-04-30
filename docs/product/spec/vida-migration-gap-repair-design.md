# Vida Migration Gap Repair Design

Purpose: repair the migration gap between `vida init`/`project-activator` and the runtime contract expected by a migrated project.

## Problem

Recent project migration exposed several coupled operator defects:

1. `AGENTS.sidecar.md` is treated as a docs-map-only artifact, but it must be the project-level agent instruction overlay. A docs map is a required sidecar section, not the whole sidecar contract.
2. Nested help paths can return non-zero exit codes or require a project root even when they only print usage.
3. `vida init` and `vida project-activator` do not reliably repair a project to ready-enough state without manual config and docs surgery.
4. DocFlow can identify missing markdown footer metadata, but lacks a direct repair path for legacy docs with existing changelog sidecars.
5. Runtime startup bundle/capsule projections treat footer status `canonical` as blocked even when promotion metadata says the projection is executable.
6. Operator errors often name blockers without pointing to a concrete command that fixes them.

## Architecture Decision

Use the following ownership model:

1. `AGENTS.md` remains the generated framework bootstrap carrier. It owns command-first startup, lane routing, write-guard invariants, and runtime law handoff.
2. `AGENTS.sidecar.md` becomes the project agent instruction overlay. It may contain project operating rules, local commands, coding/testing/release constraints, project-agent/team conventions, and a mandatory project docs map section.
3. The docs map remains discoverable from the sidecar, but sidecar validation must not reject project-local instruction content merely because it is not a map entry.
4. Init migration must classify pre-existing `AGENTS.md` content:
   - framework-like legacy bootstrap content is archived as a legacy snapshot and a clean sidecar overlay is created,
   - project instruction content is embedded under a migrated-project-instructions section inside the sidecar overlay,
   - an existing non-placeholder sidecar is preserved.
5. `project-activator --repair` is the bounded repair surface for ready-enough config/docs scaffolding. It materializes missing project docs/config/host projections where safe defaults are available and records an activation receipt.
6. Help surfaces are rootless and successful: any `--help`, `-h`, `help`, or version-only path must avoid project-root/state resolution and exit `0` after printing help.
7. DocFlow footer repair is explicit through `repair-footer` plus `finalize-edit --init-missing-footer`, using deterministic default metadata and appending changelog evidence.

## Bounded File Set

Primary implementation scope:

1. `AGENTS.md`
2. `AGENTS.sidecar.md`
3. `install/assets/AGENTS.scaffold.md`
4. `install/assets/AGENTS.sidecar.scaffold.md`
5. `crates/vida/src/cli.rs`
6. `crates/vida/src/root_command_router.rs`
7. `crates/vida/src/docflow_proxy.rs`
8. `crates/vida/src/taskflow_consume.rs`
9. `crates/vida/src/taskflow_consume_bundle.rs`
10. `crates/vida/src/taskflow_runtime_bundle.rs`
11. `crates/vida/src/init_surfaces.rs`
12. `crates/vida/src/project_activator_surface.rs`
13. `crates/docflow-cli/src/lib.rs`
14. product/process specs touched by the changed operator contract.

Out of scope:

1. replacing the DB-first runtime store,
2. rewriting the entire TaskFlow dispatch graph,
3. introducing new host-agent hardcoding.

## Proof Targets

Required checks:

1. `vida docflow init --help` exits `0` and prints help.
2. `vida taskflow consume final --help` exits `0` and does not require a project root.
3. `vida project-activator --help` lists `--repair`.
4. `vida project-activator --repair --json` reports repair status and next actions.
5. `vida docflow repair-footer --help` exits `0`.
6. `vida docflow finalize-edit --help` shows `--init-missing-footer`.
7. `vida orchestrator-init --json` reports startup bundle/capsules with non-blocked status when footer status is `canonical`.
8. `cargo test -p vida` and `cargo test -p docflow-cli` pass.
9. `cargo build -p vida --release` passes.
10. Installed `vida --version` reports the new built binary version after release install.

-----
artifact_path: product/spec/vida-migration-gap-repair-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-30
schema_version: 1
status: canonical
source_path: docs/product/spec/vida-migration-gap-repair-design.md
created_at: 2026-04-30T21:20:12.2481681Z
updated_at: 2026-04-30T22:15:50.6532042Z
changelog_ref: vida-migration-gap-repair-design.changelog.jsonl
