# VIDA 1 Development Conditions

Purpose: keep the current proven development, build, install, and runtime-entry
conditions for active `VIDA 1` work in one compact project process surface.

This file is an active summary, not the historical proof ledger. Historical
proof events remain in `vida1-development-conditions.changelog.jsonl`.

## Scope

This file records only conditions that are still useful for current operators or
agents.

It does not replace:

1. product law in `docs/product/spec/**`,
2. runtime instruction law in `vida/config/instructions/**`,
3. detailed command timing and gate policy in
   `docs/process/command-timing-and-gate-optimization-protocol.md`,
4. exact historical proof rows in this file's changelog.

## Retention Rule

1. Keep the active proof ladder and current operator boundaries here.
2. Keep historical milestone details in the changelog instead of repeating long
   per-command ledgers in the body.
3. When another retained owner carries a detail more precisely, link that owner
   and remove the duplicate wording here.
4. Add a new body item only when future work needs it to select a command,
   environment, or proof class.

## Current Proof Ladder

Use `scripts/vida-dev-gate.ps1` for local Windows proof loops and deterministic
Cargo target-dir policy.

Current reusable gate modes:

1. `-Mode script-check -Json`
   - docs, process, and script-only edits that do not need Cargo.
2. `-Mode quick -Json`
   - cheap source proof through diff check, formatting, and `cargo check`.
3. `-Mode focused-nextest -TestFilter <filter> -Json`
   - bounded regression proof for a named Rust test/filter.
4. `-Mode package-nextest -Json`
   - full `vida` package nextest proof.
5. `-Mode workspace-nextest -Json`
   - broader workspace proof after a coherent batch is assembled.
6. `-Mode doc-test -Json`
   - Rust documentation tests.
7. `-Mode build-debug -Json`
   - debug runtime entrypoint build.
8. `-Mode runtime-smoke -Json`
   - debug runtime state-compatibility smoke when the debug binary is usable.
9. `-Mode release-package -Json`
   - release archive packaging.
10. `-Mode release-install -Json`
    - installed-runtime validation only when release/install proof is the
      bounded acceptance target.
11. `-Mode target-dir-policy -Json`
    - cheap policy probe before Cargo work from a new linked worktree.

Docs-only cleanup under explicit runtime-defective mode should use static proof
only: `git diff --check`, script parser checks through `script-check`, owner
grep checks, and scoped Git diff review. Do not invent TaskFlow/DocFlow runtime
receipts while the runtime is explicitly out of scope.

## Current Proven Surface Summary

The active repository has previously proven these broad condition families:

1. Rust workspace formatting, checking, nextest, doc-test, debug build, runtime
   smoke, release package, and release install paths through the dev-gate script.
2. DocFlow Rust/in-process surfaces for bounded overview, validation,
   readiness, registry, relation, proofcheck, link, dependency, artifact-impact,
   task-impact, footer/changelog mutation, move, rename, and changelog reads.
3. TaskFlow launcher-owned surfaces for help, query, dependency planning,
   graph validation, dependency mutation, ready/show/list semantics, missing-task
   fail-closed behavior, and snapshot export/import.
4. State-store and Surreal adapter contract checks for storage metadata, state
   spine, backend summary, schema drift, and snapshot round-trip.
5. Release and installer packaging paths that separate debug sanity builds from
   release-binary producer artifacts and packaging jobs.

Use the owning tests, scripts, and changelog entries for exact historical proof
commands. Do not copy the old full proof ledger back into this body.

## Release And Install Boundary

1. Use debug source proof for routine Rust repair loops.
2. Use debug runtime smoke only after the debug binary proves it can open the
   current project state.
3. Use installed-runtime validation when the acceptance target is the operator's
   installed launcher, release package, installer behavior, PATH resolution, or
   state compatibility through the normal installed binary.
4. Do not run release install merely because source files changed inside an
   unfinished task.

Detailed release timing and gate policy lives in
`docs/process/command-timing-and-gate-optimization-protocol.md`.

## Environment Boundary

1. Project-local runtime state defaults to `.vida/data/state/`.
2. Fresh proof roots may use `VIDA_STATE_DIR=<temp-dir>` when the proof is meant
   to avoid long-lived local state.
3. Runtime-root and host-system selection are config-driven through
   `vida.config.yaml`, not hardcoded per host.
4. Manually deleting backing-store state is not a valid cleanup shortcut; use a
   documented reset/recovery workflow or an explicit temp-state proof root.

The operator-facing environment summary lives in
`docs/process/environments.md`.

## Documentation Update Rule

After a successful bounded implementation step changes what future developers
can run, build, install, or verify:

1. update this file only with the current reusable condition,
2. keep exact one-off proof details in the changelog or task evidence,
3. update `command-timing-and-gate-optimization-protocol.md` when the reusable
   lesson is about gate selection, slow commands, or proof timing,
4. update product specs only when the condition changes product/runtime law.

## Current Known Transitional Limits

1. VIDA runtime may be explicitly bypassed for static documentation cleanup when
   the operator says the runtime is defective.
2. Some runtime/config surfaces still reference this document as the canonical
   project development-evidence target; do not delete it until those references
   are intentionally migrated.
3. Historical release-specific facts are provenance. Current release truth must
   come from release notes, tags, installer state, or explicit release proof, not
   this process summary.

-----
artifact_path: process/vida1-development-conditions
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-06-13'
schema_version: '1'
status: canonical
source_path: docs/process/vida1-development-conditions.md
created_at: '2026-03-11T09:00:00+02:00'
updated_at: 2026-06-13T02:05:00+03:00
changelog_ref: vida1-development-conditions.changelog.jsonl
