# Test-First Runtime Defect Remediation Design

Status: `proposed`

Purpose: define the current defect-repair program where VIDA runtime defects are first captured as spec-backed failing tests, then fixed one bounded defect at a time.

## Summary

- Feature / change: introduce a test-first remediation epic for runtime/operator defects discovered during Case 18 and later runtime self-testing.
- Owner layer: `mixed`
- Runtime surface: `taskflow | status | lane | agent | route | release`
- Status: `proposed`

## Current Context

The active Case 18 recovery work exposed several runtime defects and near-defects in the same batch:

1. route/readiness parity contradictions between selected runtime assignment and selected-backend readiness projections,
2. activation-view-only or internal-carrier blockers without receipt-backed execution,
3. continuation-binding blockers such as missing or stale next action targets,
4. operator output/actionability gaps where commands, options, or recovery details are absent,
5. latency and cache-admissibility defects on normal operator surfaces.

Existing tests cover many local helpers and smoke paths, but several defects only became clear after manual runtime reproduction across multiple command surfaces. The missing layer is a spec-shaped scenario contract suite that seeds one runtime state and asserts cross-surface agreement.

## Goal

VIDA defect repair must become test-first at the runtime-contract layer:

1. every selected defect is first tied to a project spec, process doc, operator contract, or canonical runtime law,
2. a failing regression test is written before the code fix,
3. the failing test must reproduce the same field-level contradiction as the runtime evidence,
4. the code fix is bounded to one defect or one coherent invariant,
5. proof after the fix runs focused, adjacent, release, installed-binary, and diagnostic gates.

## Requirements

### Functional Requirements

- Must create one active defect-remediation epic that owns newly discovered runtime defects from test-first work.
- Must move current runtime defect candidates out of ordinary Case 18 continuation and into the defect-remediation epic before further code fixes.
- Must keep current partially started defects paused until they have a failing test and a bounded write scope.
- Must allow batch analysis over multiple defects, but only one write-producing bounded defect fix at a time.
- Must require a failing regression test between `Крок 2/3: архітектурне рішення + bounded write scope` and `Крок 3/3: code fix`.
- Must treat missing command, missing option, incomplete next action, or undiscoverable implemented command as runtime/operator defects.
- Must not hardcode carrier ids, host CLI systems, model refs, or agent ids as production authority in tests or fixes; tests may use synthetic names and may report observed runtime values as evidence.

### Non-Functional Requirements

- Test fixtures must prefer config-derived or synthetic carrier names over real production carrier/model literals unless parser compatibility for a literal is the subject under test.
- Cross-surface tests must assert selected JSON fields and invariants rather than large brittle snapshots.
- Latency proof must avoid flaky wall-clock CI assertions by testing cache admissibility and fast-path correctness; local timing smoke can remain an ignored/manual gate.
- Runtime state mutation must use VIDA task/operator surfaces, not direct state-file edits.

## Test Layers

### 1. Local Contract Tests

Use when the defect is a pure helper or classifier invariant:

- blocker-code classification,
- selected readiness payload selection,
- cache admissibility decisions,
- activation-view/receipt boolean derivation,
- operator action string sanitation.

Proof target shape:

```text
cargo test -p vida <focused_test_name> -- --nocapture
```

### 2. Scenario Contract Tests

Use when two or more operator surfaces disagree over the same runtime state:

- `route explain` vs `validate-routing`,
- `status` vs `recovery latest`,
- `lane show` vs run-graph status,
- `next-lawful` vs continuation binding,
- activation view vs receipt-backed execution state.

These tests should seed a minimal runtime state once, then assert the same blocker, selected backend, bounded unit, and next action across all relevant projections.

### 3. CLI/Operator Surface Tests

Use when the defect is discoverability, help, JSON envelope, or actionable command output:

- a blocked surface lacks a concrete command,
- a command output omits the option required to execute the recommended action,
- an implemented command or alias is missing from the help/discoverability path,
- an operator envelope reports `pass` while nested blockers remain unexplained.

### 4. Installed-Binary Validation

Use after the fix, not as the first failing test:

- release build,
- install active `vida.exe`,
- verify version/build timestamp/fingerprint,
- rerun the original reproduction commands against the installed binary.

## Post-Push Operator-Friction Diagnostic

After each proven defect fix, commit, push, release build/install, and runtime self-diagnostic refresh, the orchestrator must also inspect whether the just-completed work required avoidable extra operator iterations.

The explicit optimization target is fewer VIDA command invocations and less token-heavy output reading per lawful result. The diagnostic should prefer command/output improvements that collapse repeated inspect/update/show loops into one machine-readable, evidence-preserving surface.

The audit must classify a new runtime/operator-surface defect when any of these conditions occurred:

1. a command rejected a useful machine-readable mode such as `--json` without an alternative machine-readable surface,
2. a failed mutation reported a graph or runtime guard but did not return a concrete repair command,
3. create/ensure surfaces could not set task metadata needed for the same bounded task,
4. a tree/list/projection output omitted fields needed to verify the result without extra `show` calls,
5. a common multi-step remediation flow lacked an atomic or guided command,
6. a recommended next action had placeholders or missing options that required guessing,
7. an implemented command, alias, or option was absent from help/discoverability output.

Confirmed gaps are not backlog polish. They are operator-actionability defects because they increase orchestration iterations and make autonomous runtime repair less reliable.

## Current Defect Intake

The first defect-remediation batch should contain these task-owned candidates:

1. `case-18-route-readiness-parity`
   - Invariant: selected runtime assignment and selected-backend readiness must not contradict one another without an explicit stale/mismatch blocker.
   - Preferred first test layer: scenario contract test.
2. `case-18-activation-receipt-contract`
   - Invariant: `activation_view_only` is never receipt-backed execution and must not complete delegated work.
   - Preferred first test layer: local contract plus CLI/operator surface test.
3. `case-18-continuation-binding-consistency`
   - Invariant: active bounded unit, why, posture, recovery target, and next lawful action must agree across init/status/recovery/lane/next-lawful surfaces.
   - Preferred first test layer: scenario contract test.
4. `case-18-operator-actionability-contract`
   - Invariant: missing command, missing option, or undiscoverable implemented command is a defect.
   - Preferred first test layer: CLI/operator surface test.
   - Current subcases:
     - `docflow check` rejected `--json`.
     - `task update --parent-id` reported `open_parent_has_no_open_child` without concrete parent-pause/reparent recovery.
     - `task ensure/create` could not set owned paths, proof targets, acceptance targets, or notes in the same create operation.
     - `task tree --json` child rows lacked enough stable detail for one-pass verification.
     - no atomic defect-batch rehome/pause/start command exists for the observed remediation setup.
5. `case-18-operator-latency-cache-admissibility`
   - Invariant: ordinary operator surfaces must remain bounded and cache fast paths must include all decision-critical fields.
   - Preferred first test layer: local contract and cache-admissibility test.

## Repair Cadence

For each selected defect:

1. `Крок 1/3: дослідження`
   - reproduce current runtime evidence,
   - read code owner paths,
   - actualize expected behavior from specs and operator contracts.
2. `Крок 2/3: архітектурне рішення + bounded write scope`
   - name the invariant,
   - name the exact write scope,
   - name the failing regression test to add.
3. Test-first gate
   - write the failing test,
   - verify it fails for the expected field-level reason,
   - if it fails for fixture weakness, repair the test fixture before code fix.
4. `Крок 3/3: code fix`
   - implement only the bounded fix needed to make the failing test pass.
5. Post-fix proof
   - focused test,
   - adjacent tests,
   - `cargo fmt --check`,
   - release build,
   - install active `vida.exe`,
   - direct installed-binary validation,
   - runtime diagnostic/status refresh,
   - commit and push,
   - refresh advisory sidecar for the next defect batch.

## Pause And Reparent Rule

When this program becomes active, current runtime defect candidates that were being handled as ordinary Case 18 continuation work should be paused, moved under the test-first defect-remediation epic, and resumed only after the first failing regression test is selected.

Pausing does not close or discard evidence. It records that the defect is now owned by the test-first remediation program and must not receive code changes until its failing test and bounded write scope are explicit.

-----
artifact_path: product/spec/test-first-runtime-defect-remediation-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-05-21'
schema_version: '1'
status: proposed
source_path: docs/product/spec/test-first-runtime-defect-remediation-design.md
created_at: '2026-05-21T20:30:00+03:00'
updated_at: '2026-05-21T20:30:00+03:00'
changelog_ref: test-first-runtime-defect-remediation-design.changelog.jsonl
