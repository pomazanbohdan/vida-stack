# Project Script Authoring Master

Status: active canonical project process document

## Purpose

Purpose: define the single source of truth for authoring, building, installing, invoking, and proving project scripts in the active vida-stack repository. New sessions and agents use this document before creating or changing a script.

## Scope

1. This document owns project script-authoring conventions, Go-tool layout, shell boundaries, environment assumptions, and script proof requirements.
2. docs/process/vida-runtime-development-environment.md remains the broader runtime/toolchain runbook; this document is the owner for script implementation decisions.
3. docs/process/documentation-tooling-map.md remains the owner for DocFlow commands and documentation mutation/finalization.
4. AGENTS.sidecar.md and docs/project-root-map.md route bootstrap to this document; they do not duplicate its rules.
5. Project code and tests are authoritative when this document and an implementation disagree. Update this document in the same bounded change when the implementation contract changes.
6. This document covers project-owned scripts under scripts/, standalone Go tools under tools/, workflow invocations, and script-related tests/fixtures. It does not redefine Rust/Cargo or framework runtime law.

## Authority

1. This document is the project owner for script authoring, Go-tool layout, environment assumptions, CLI contracts, binary proof, and wrapper deletion.
2. The project gate at scripts/vida-dev-gate.ps1 owns orchestration and timing; it does not replace this document or the Go implementations.
3. DocFlow and TaskFlow remain the validation and execution authorities for documentation and bounded work.
4. AGENTS.sidecar.md and docs/project-root-map.md are bootstrap routing surfaces; they point to this document without duplicating its rules.

## Trigger

Read this document when a task:

1. creates, deletes, or migrates a script;
2. changes a script caller, CI workflow, gate mode, CLI option, output envelope, or exit contract;
3. installs or resolves Go, PowerShell, Git, VIDA, or related host tools;
4. needs to decide whether a PowerShell file is an implementation, an orchestration gate, or obsolete compatibility surface;
5. starts a new session that may write a project script.

First-session route:

    1. Read AGENTS.md.
    2. Read AGENTS.sidecar.md.
    3. Read docs/project-root-map.md.
    4. Read this document.
    5. Run vida orchestrator-init --json and record active_bounded_unit, why_this_unit, and sequential_vs_parallel_posture.
    6. Inventory callers before editing:
       rg --files scripts | Sort-Object
       rg -n "tools/|scripts/|go build|go run|pwsh .*\\.ps1" .github scripts tools docs
    7. Create a DB-backed TaskFlow step with owned paths and proof targets before write-producing changes.

If runtime dispatch is unavailable and the operator explicitly authorizes direct fallback, keep the edit set static and bounded, record the runtime blocker in TaskFlow, and leave closure pending until receipt-backed runtime evidence is restored.

## Inputs

Every script-authoring task must identify:

1. target script or Go module path;
2. direct callers, workflows, fixtures, and documentation owners;
3. CLI flags, output fields, exit codes, and environment overrides;
4. TaskFlow owned paths, acceptance targets, and proof targets;
5. required Go/PowerShell/Cargo/DocFlow commands and the smallest proof ladder;
6. whether the change is implementation, orchestration, compatibility, or deletion.

## Outputs

A completed script-authoring batch produces:

1. tested implementation or explicit deletion;
2. compiled binary proof for every changed Go module;
3. rewired callers and zero live references to deleted wrappers;
4. updated master documentation, owning maps, and changelog;
5. TaskFlow evidence, DocFlow evidence, diff check, and a verified hook-backed commit.

## Rules

## Canonical Classification

Use this decision order for every script:

1. Go implementation: deterministic validation, parsing, inventory, policy checks, fixture processing, or other reusable logic that benefits from tests and a compiled binary.
2. PowerShell orchestration: Windows environment normalization, Cargo/MSVC setup, gate sequencing, timing/artifact collection, release/install flow, or commands that coordinate several tools.
3. Bash/command wrapper: platform entrypoint only when the repository already owns that surface and the shell is required by the caller.
4. Test/fixture helper: test-only code must remain under the relevant test tree or Go test file and must not become a hidden production entrypoint.
5. Obsolete wrapper: a compatibility shim with no independent policy or orchestration value. Delete it after all callers are moved and binary proof is green.

Rules:

1. New validator or scanner logic is written in Go first.
2. A PowerShell file is retained only when it owns orchestration or a supported Windows entrypoint; a thin wrapper around a Go validator is not a default architecture.
3. Do not add a second wrapper, duplicate parser, or ad-hoc one-off script to compensate for a missing documented command.
4. Delete an obsolete wrapper only after an exact caller audit, direct Go invocation is wired everywhere, and the gate proves the compiled binary.

## Canonical Repository Layout

Each standalone Go script owns one directory and one module:

    tools/<script-name>/
      go.mod
      main.go
      main_test.go

Current canonical examples:

1. tools/check-agent-evaluation-log
2. tools/check-runtime-boundaries
3. tools/check-host-bridge-capability-neutrality
4. tools/verify-rust-toolchain

Module rules:

1. Use a local module path rooted at vida-stack/tools/<script-name>.
2. Pin the module directive to the project-approved Go baseline: go 1.26.0.
3. Keep dependencies in the standard library unless a bounded TaskFlow task proves a dependency is necessary.
4. Keep main.go independently buildable from its module directory.
5. Keep tests beside the implementation and use temporary fixtures for filesystem scans.
6. Do not place generated binaries in Git. Build to a temporary path or an ignored .vida/tmp path and remove it in a finally/cleanup block.
7. Keep the Go implementation free of credentials, network calls, hidden process mutation, and broad filesystem deletion.

The canonical Windows gate remains scripts/vida-dev-gate.ps1. It orchestrates proof modes and may call Go binaries, Cargo, PowerShell, Bash, and VIDA. It is not a validator implementation.

## CLI Contract

Every reusable Go script must define and test:

1. Explicit root/cwd/path flags. Do not infer a repository root from an arbitrary current directory when a caller can pass it.
2. A machine-readable JSON mode named --json.
3. A concise text mode for operators.
4. Stable status values. pass means all checks passed; blocked means a policy/validation condition failed; operational errors are reported to stderr and exit nonzero.
5. Exit 0 only for a pass result. Exit 1 for blocked validation or operational failure unless a narrower documented contract is required.
6. JSON on stdout only; diagnostic error text on stderr. Do not mix progress lines into JSON stdout.
7. Stable field names, surface identifier, blocker/issue arrays, and source paths. Add fields compatibly; do not rename existing fields without a migration note.
8. No silent failure when a required root, fixture, command, or config file is missing.
9. Paths are normalized with filepath helpers and outputs use slash-normalized relative paths where the contract exposes paths.
10. Environment overrides are explicit and documented, for example RG for ripgrep or a binary-specific *_BIN override.

When migrating an existing script, compare the Go result to the pre-migration implementation on pass, blocked, malformed-input, missing-file, and JSON cases. Record intentional differences in the TaskFlow note and this document.

## Authoring Workflow

1. Bind: run bootstrap/init, inspect current status, and identify one bounded script unit.
2. Trace: inventory the implementation, direct callers, workflow references, tests, fixtures, docs, and environment variables.
3. Design: decide Go implementation versus orchestration; define flags, output, exit codes, owned paths, and proof targets in TaskFlow.
4. Implement: write the smallest isolated Go module or bounded orchestration change. Keep unrelated dirty files untouched.
5. Test: add unit tests for normal, blocked, malformed, missing, and edge cases; use temporary fixtures for path-sensitive behavior.
6. Build: run go test ./..., go vet ./..., and go build -trimpath -o <temporary-binary> . from the module directory.
7. Binary proof: invoke the compiled binary, not only go test or go run. Exercise text and JSON modes and any self-test mode.
8. Rewire: update every caller, CI workflow, gate, map, and documentation pointer before deleting a wrapper.
9. Audit: run an exact caller search, git diff --check, and the smallest project gate.
10. Document: update this master document and the owning maps/changelog in the same logical batch.
11. Integrate: stage only the bounded files, commit through the normal hook, verify the commit SHA and staged scope, and run the required post-commit runtime/DocFlow diagnostics.

## Required Proof Ladder

For any script or script-documentation change, run in this order:

    pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Help
    pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode script-check -Json
    vida task validate-graph --json
    vida docflow check-file --path docs/process/project-script-authoring-master.md --json
    vida docflow readiness-check --profile active-canon
    git diff --check

For a changed Go module, script-check must include:

    go test ./...
    go vet ./...
    go build -trimpath -o <temporary-binary> .
    <temporary-binary> --json

The gate owns the canonical binary proof for the current three migrated validators:

1. check-agent-evaluation-log: fixture JSON pass smoke; blocked fixtures remain covered by Go tests.
2. check-runtime-boundaries: repository-root JSON pass smoke; blocked imports/exports remain covered by Go tests.
3. check-host-bridge-capability-neutrality: self-test and repository-root JSON pass smoke.

Use quick, focused-nextest, package-nextest, workspace-nextest, or release modes only when the change crosses the Rust/runtime or installed-runtime boundary. Do not use a full Cargo gate for a docs-only or isolated Go validator change unless the task explicitly requires it.

## Environment And Installation

The verified environment for this session is:

1. Windows PowerShell 7.6.4 at C:\Program Files\WindowsApps\Microsoft.PowerShell_7.6.4.0_x64__8wekyb3d8bbwe\pwsh.exe.
2. Go go1.26.5 windows/amd64 at C:\Program Files\Go\bin\go.exe.
3. Go module baseline: go 1.26.0.
4. VIDA command resolved from C:\Users\pomaz\AppData\Local\vida-stack\current\bin\vida.exe.
5. Project root: C:\project\vida-stack.

Resolution rules:

1. Resolve go through PATH or an explicit GO environment override; fail closed when it is missing.
2. Resolve ripgrep through RG when supplied, otherwise LookPath("rg"); Go tools must also work with their documented fallback behavior when rg is absent.
3. Resolve PowerShell through the project PowerShell/Core rules in docs/process/vida-runtime-development-environment.md. Do not silently fall back to legacy Windows PowerShell for project gates.
4. Keep TEMP/TMP writable. Temporary binaries belong in the OS temp directory or an ignored .vida/tmp directory.
5. Do not install or replace a system VIDA binary as part of a micro-edit. Use release-install only at the final coherent pack/epic gate.
6. Do not store credentials, tokens, private keys, authorization headers, or secret-bearing URLs in source, fixtures, JSON artifacts, TaskFlow notes, or docs.
7. Go tools are installed by building the module; there is no repository-wide Go workspace or hidden runtime registration required:

       Set-Location tools/<script-name>
       go test ./...
       go build -trimpath -o <approved-temp-path> .

8. CI must build the module and invoke the resulting binary. Do not rely on a developer-specific installed binary.

## Current Migration Record

The first three independent validator implementations are now canonical under tools/:

| Former surface | Canonical implementation | Caller policy |
| --- | --- | --- |
| scripts/check-agent-evaluation-log.ps1 | tools/check-agent-evaluation-log | Gate builds and smokes the binary; no compatibility wrapper remains. |
| scripts/check-runtime-boundaries.ps1 | tools/check-runtime-boundaries | Gate builds and smokes the binary; no compatibility wrapper remains. |
| scripts/check-host-bridge-capability-neutrality.ps1 | tools/check-host-bridge-capability-neutrality | Gate and GitHub Actions build/invoke the binary; no compatibility wrapper remains. |

The old PowerShell paths are historical migration references only. They must not appear as live callers, workflow commands, gate invocations, or new documentation examples.

## Deletion And Compatibility Rule

Before deleting a legacy script:

1. Search exact path and basename references across .github, scripts, tools, tests, docs, and config.
2. Classify every match as live caller, test/fixture, documentation history, or dead text.
3. Replace live callers with the canonical Go build/invoke path.
4. Update Go surface identifiers and fixtures so outputs no longer claim the deleted implementation path.
5. Run script-check after the replacement.
6. Delete the file in the same bounded change.
7. Re-run the exact search; zero live callers is required.
8. Keep a short migration record in this document and the TaskFlow note; do not restore a wrapper merely to make an old command appear to work.

## Bootstrap And Documentation Registration

The bootstrap-visible source of truth is this chain:

    AGENTS.md
      -> AGENTS.sidecar.md
        -> docs/project-root-map.md
          -> docs/process/index.md
            -> docs/process/project-script-authoring-master.md

The pointer must be present in:

1. AGENTS.sidecar.md Project Canonical Maps and Project Script Discovery Before Implementation.
2. docs/project-root-map.md Canonical Entry Points and Task Routing.
3. docs/process/index.md Canonical entrypoints.
4. docs/process/documentation-tooling-map.md Canonical Entry Points and Activation Triggers.
5. This document's metadata footer and sibling changelog.

When this document is missing, renamed, or not registered, a new script task is not bootstrap-ready. Update all five surfaces in one bounded documentation batch and run DocFlow check/readiness.

## Maintenance Contract

When adding or changing a script:

1. Update this document before the next session can depend on the new contract.
2. Add or update a sibling changelog entry for the logical documentation batch.
3. Keep implementation comments short; durable policy belongs here.
4. Keep module-specific behavior in Go tests and module code; do not duplicate full implementation prose in another runbook.
5. Refresh the code index only after checking the configured ccc provider; local-only indexing is allowed, remote/unknown indexing is fail-closed.
6. Record an optimization note after the bounded task: batch independent read-only searches, reuse one proof bundle, avoid duplicate status/gate reruns, clean completed handles, and preserve fail-closed authority.
7. If a rule cannot be enforced by current tooling, state the gap and the bounded next proof instead of claiming automatic enforcement.

## Forbidden

1. A new Go validator with no unit tests or no compiled-binary smoke.
2. A PowerShell wrapper kept only because an old caller was not searched.
3. go run as the only proof of a production script.
4. Hidden network access, credential reads, broad deletion, or mutation of files outside owned paths.
5. JSON mixed with human progress output.
6. Swallowing a nonzero exit or converting blocked validation into pass.
7. Untracked generated binaries or machine-specific paths in source.
8. Duplicate “how to write scripts” documents that can drift from this master.
9. Closing a TaskFlow step when runtime/DocFlow evidence or integration proof is still pending.

## Escalation

Escalate instead of guessing when:

1. the runtime cannot bind the requested bounded unit or the exception scope excludes the owned paths;
2. a caller requires a legacy command that has no documented compatibility owner;
3. Go, PowerShell, or the project VIDA binary resolves to an unapproved or missing path;
4. the binary output or exit behavior differs from the established contract;
5. DocFlow reports a missing owner-map registration or a blocking canonical-doc issue;
6. a proof failure could be caused by unrelated dirty worktree changes.

Record the exact blocker, command, artifact path, and next lawful action in TaskFlow. Do not bypass a gate or restore a deleted wrapper without an explicit new bounded decision.

## Validation

Acceptance requires:

1. exact caller audit;
2. Go tests, vet, trimpath build, and compiled-binary text/JSON proof;
3. project gate result with preserved artifact references;
4. DocFlow check/readiness for this document and changed maps;
5. TaskFlow graph validation;
6. diff check, scoped staging, hook-backed commit, and verified commit SHA.

## Token Budget

This is a no-fixed-target canonical master document. Preserve exact commands, paths, flags, field names, versions, and safety rules even when compression would reduce tokens. Keep implementation explanations in code/tests and keep this document as the compact owner contract. Any future compression must record the measured before/after token count and rerun DocFlow protocol-authoring validation.

## Metadata

-----
artifact_path: process/project-script-authoring-master
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-08-12'
schema_version: '1'
status: canonical
protocol_authoring_gate: enforced
source_path: docs/process/project-script-authoring-master.md
created_at: '2026-08-12T00:00:00+03:00'
updated_at: '2026-08-12T00:00:00+03:00'
changelog_ref: project-script-authoring-master.changelog.jsonl
