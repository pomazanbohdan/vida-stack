# META Runtime Boundary Refactor Baseline Tracking Document

Status: `IO-001, DOC-001, HB-001 through HB-005, ROUTE-001, and ROUTE-002 hardened; targeted proofs green; broader Wave 0 batch still pending`

## Purpose

This document tracks the Wave 0 baseline for the META runtime boundary refactor in `vida-stack`.
It is the working companion to the source plan:

- `vida_meta_refactor_execution_plan.md`

Source-of-truth scope for this tracker:

- Wave 0 section `## 4. Wave 0 — Baseline freeze and defect fixtures`
- the broader execution plan context needed to keep the baseline artifact implementation-ready

This file is intentionally a tracker, not a rewrite of the plan. The wave definitions below are copied from the source plan where practical so developers can execute against the same wording and proof targets.

## Current Baseline State

- Baseline document created.
- Artifact IO smoke tests were added in `crates/vida/tests/task_smoke.rs`.
- The attempt-artifact read path now fail-closes on out-of-root, oversized, and non-regular files before JSON parse.
- Rework complete: `taskflow_attempt_implementation_artifacts` now threads an
  explicit `state_root` from callers instead of `std::env::current_dir()`.
- The silent-skip path for relevant invalid implementation artifacts now
  fail-closes with `Result` propagation.
- DOC-001 now hardens `docflow closeout --changed` / `docflow-cli closeout --changed`
  against repo-local `core.fsmonitor` helpers by disabling fsmonitor and
  untracked-cache in the git status invocation.
- HB-005 now hardens `vida lane complete` so implementation scope validation
  derives allowed `owned_paths` from the immutable dispatch packet / TaskFlow
  authority instead of mutable host bridge request JSON.
- HB-003 now hardens `vida lane complete` so persisted dispatch
  result/receipt evidence is authoritative and mutable request JSON cannot
  redirect completion artifacts.
- HB-004 now hardens host bridge request reads so retryable request paths are
  canonicalized under the state root and rejected before JSON parse when the
  target is out-of-root, non-regular, or oversized.
- HB-001 now proves the public `vida agent host-bridge --request ... --state-dir ...`
  path fail-closes on an attacker request path outside the explicit trusted
  state root; the public run exit and shared payload helper keep
  `host_tool_calls = []`.
- ROUTE-001 now enforces dev-team human labels through configured
  `task_class` backend admissibility instead of treating role labels as
  bypassable backend keys; PR #350 is integrated into the Wave 0 routing slice.
- ROUTE-002 now rejects `vida agent-init --role ...` aliases that resolve to the
  root `orchestrator` runtime role; PR #340 is integrated into the Wave 0
  routing slice.
- Explicit `agent-init --role developer` selection now preserves
  `dispatch_target = developer` while allowing `selected_role` to normalize to
  `worker`, so backend admissibility evaluates the human lane label.
- Validation rework tightened the DOC-001 proof: a plain `git status`
  precondition now proves the repo-local helper executes before the
  `docflow closeout --changed` assertion checks that the helper stays idle.
- Focused proof runs are green for the current rework batch:
  - `cargo fmt --all -- --check`
  - `cargo test -p vida --test task_smoke task_attempt_collect_rejects -- --nocapture`
  - `cargo test -p vida --test task_smoke task_attempt_implementation_artifact_validation -- --nocapture`
  - `cargo test -p vida host_bridge_taskflow_implementation_artifacts_blocks_invalid_artifact_evidence -- --nocapture`
  - `cargo test -p vida host_bridge_missing_receipt`
  - `cargo test -p vida host_bridge_missing_receipt -- --nocapture`
  - `cargo test -p vida host_bridge_implementation_scope_uses_immutable_packet`
  - `cargo test -p vida host_bridge_implementation_scope_uses_immutable_packet -- --nocapture`
  - `cargo test -p vida agent_init_explicit_role_preserves_requested_role_as_dispatch_target -- --nocapture`
  - `cargo test -p vida agent_init_explicit_role_rejects_dev_team_orchestrator_runtime_role_aliases -- --nocapture`
  - `cargo test -p vida backend_is_admissible_for_dispatch_target_uses_configured_lane_task_class_for_role_label -- --nocapture`
  - `cargo test -p vida admissible_selected_backend_uses_configured_lane_task_class_for_role_label -- --nocapture`
  - `cargo test -p vida admissible_selected_backend_uses_configured_test_authoring_task_class_for_role_label -- --nocapture`
  - `cargo build`
- Broad `cargo test -p vida uses_configured -- --nocapture` is not a closure
  proof for this slice because it also selects an unrelated existing
  `agent_dispatch_next_command_uses_configured_runtime_selection_truth` stack
  overflow path.
- Host-bridge authority proof was re-run after the HB-002 production authority
  fix and now blocks earlier on `host_bridge_dispatch_receipt_missing` before
  artifact attachment.
- This rework still sits inside the broader Wave 0 batch; the full batch proof
  remains pending.
- `wave-0-routing-tests` recovery note: the canonical dev-team dispatch preview
  is currently blocked by `packet_materialization_failed:task=wave-0-routing-tests`
  with repeated `materialized packet dispatch_target mismatch: expected coach,
  got coach_test_gate` evidence. The lane exception takeover is active only for
  the Wave 0 smoke-test/tracker scope:
  `crates/docflow-cli/tests/cli_smoke.rs`, `crates/vida/tests/boot_smoke.rs`,
  `crates/vida/tests/doctor_surface_contract_smoke.rs`,
  `crates/vida/tests/task_smoke.rs`, and this tracker. Production source edits
  in `crates/vida/src/**` must be rebound to the matching runtime-source task
  before modification.
- No new weak red fixture was added for the `coach_test_gate` mismatch in this
  slice because the public CLI reproduction attempt did not reach the
  `coach_test_gate` materialization branch without production-source recovery.
  Keep the blocker as runtime evidence until a source-scoped task can fix the
  canonicalization path or supply a faithful public CLI fixture.

## Wave 0 Goal

Copied from the source plan:

> Freeze current behavior and create failing fixtures for all known bug classes before large movement.

## Wave 0 Required Artifact / Test Groups

Copied from the source plan:

Create these test groups if not already present:

```text
crates/vida/tests/host_bridge_authority_smoke.rs
crates/vida/tests/runtime_authority_smoke.rs
crates/vida/tests/taskflow_routing_smoke.rs
crates/vida/tests/artifact_io_smoke.rs
crates/docflow-cli/tests/git_hardening_smoke.rs
```

If the project prefers existing smoke files, append tests to:

```text
crates/vida/tests/doctor_surface_contract_smoke.rs
crates/vida/tests/boot_smoke.rs
crates/vida/tests/task_smoke.rs
crates/docflow-cli/tests/cli_smoke.rs
```

## Wave 0 Red-Test Fixture List

Priority labels below are tracking labels for this baseline document. The source plan names the defect classes and fixtures directly; it does not assign explicit P0/P1 labels.

### P0 fixtures

#### HB-001: explicit state-dir is authoritative

```text
Surface:
  vida agent host-bridge

Setup:
  trusted_state_root = temp/trusted/.vida/data/state
  attacker_state_root = temp/attacker/.vida/data/state
  request_path = attacker_state_root/host-tool-bridge/requests/request.json
  --state-dir trusted_state_root

Expected:
  status = blocked
  blocker_codes contains host_bridge_request_untrusted_path
  host_tool_calls = []
```

#### HB-002: missing receipt blocks pending host bridge

```text
Surface:
  vida agent host-bridge

Setup:
  pending host bridge request under valid state root
  no persisted RunGraphDispatchReceipt for run_id

Expected:
  status = blocked
  blocker_codes contains host_bridge_dispatch_receipt_missing
  host_tool_calls = []
```

#### HB-003: mutable request cannot redirect result/receipt path

```text
Surface:
  vida lane complete

Setup:
  persisted dispatch result points to result_path_A / receipt_path_A
  mutable request points to result_path_B / receipt_path_B

Expected:
  exit non-zero
  no write to B
  blocker/error mentions persisted dispatch receipt evidence mismatch

Implementation status:
  covered by crates/vida/src/lane_surface.rs::host_bridge_no_request_redirect
  vida lane complete validates mutable request result/receipt paths against persisted dispatch result evidence before writing completion artifacts
  persisted dispatch result host_tool_bridge_request is authoritative for request/result/receipt paths; mutable request path redirection fails closed
```

#### HB-004: retryable request path rejects FIFO/out-of-root/huge file

```text
Surface:
  vida lane complete or helper-level unit test

Setup:
  request_path points outside state root OR to FIFO OR to > max bytes file

Expected:
  command returns quickly
  status blocked/fail closed
  no read hang
```

Implementation status:
  covered by `crates/vida/src/lane_surface.rs::read_host_bridge_request_at_path`
  `vida lane complete` now canonicalizes the mutable request path under the
  state root, rejects non-regular and oversized request files before JSON
  parse, and fail-closes on out-of-root request paths.
  regression coverage:
  `host_bridge_request_rejects_out_of_root_or_oversized_file`
  (FIFO subcase on Unix)

#### HB-005: implementation scope comes from immutable packet

```text
Surface:
  vida lane complete

Setup:
  immutable dispatch packet owned_paths = ["allowed"]
  mutable request owned_paths = ["allowed", "secret"]
  implementation artifact changed_files = ["secret/outside.txt"]

Expected:
  status = blocked
  blocker_codes contains implementation_attempt_scope_guard_violation
  scope_validation.owned_paths = ["allowed"]

Implementation status:
  covered by `crates/vida/src/lane_surface.rs::host_bridge_implementation_scope_uses_immutable_packet`
  `vida lane complete` now derives implementation scope from the immutable
  dispatch packet / TaskFlow-owned paths, not mutable host bridge request JSON.
  mutable request `owned_paths` no longer widen `scope_validation.owned_paths`;
  broader request-owned scope still blocks against the immutable packet scope.
```

#### RT-001: terminal missing-task closure needs clean receipt

```text
Surface:
  vida status / vida doctor / helper unit

Setup:
  latest RunGraphStatus = terminal closure
  dispatch receipt is exception takeover OR mismatched OR missing

Expected:
  stale write guard remains active
  no root-local write authority
```

#### RT-002: recorded exception is not active takeover

```text
Surface:
  vida lane retire

Setup:
  RunGraphDispatchReceipt.lane_status = lane_exception_recorded
  exception_path_receipt_id present
  supersedes_receipt_id missing

Expected:
  retire rejected
  status remains blocked
  continuation binding preserved
```

### P1 fixtures

#### RT-003: projection cache rejects sessionless stale state-bound pass

```text
Surface:
  vida status / taskflow graph-summary

Setup:
  cached projection has status pass and state marker only
  non-task operator state changed or session identity absent

Expected:
  cache rejected
  recompute required
```

#### RT-004: final runtime snapshot cannot mint receipt authority

```text
Surface:
  status/doctor fallback

Setup:
  forged final runtime-consumption snapshot with lane_exception_takeover and absolute paths
  no persisted dispatch receipt

Expected:
  latest_final_runtime_consumption_dispatch_receipt_summary returns None
```

#### ROUTE-001: dev-team human labels enforce task_class admissibility

```text
Surface:
  runtime dispatch / scheduler dispatch / agent-init

Setup:
  dispatch target = developer
  dispatch contract task_class = implementation
  backend is not admissible for implementation

Expected:
  blocked by backend admissibility

Implementation status:
  covered by `backend_admissibility_key_for_task_class`,
  `backend_is_admissible_for_dispatch_target_uses_configured_lane_task_class_for_role_label`,
  `admissible_selected_backend_uses_configured_lane_task_class_for_role_label`,
  and `admissible_selected_backend_uses_configured_test_authoring_task_class_for_role_label`
  across runtime dispatch and scheduler selection.
  PR #350 integrated.
```

#### ROUTE-002: agent-init aliases cannot resolve to orchestrator

```text
Surface:
  vida agent-init

Setup:
  project dev_team role alias maps to runtime_role = orchestrator

Expected:
  blocked/rejected before selected_role accepted

Implementation status:
  covered by `agent_init_selected_role_allowed`,
  `agent_init_explicit_role_rejects_dev_team_orchestrator_runtime_role_aliases`,
  and `agent_init_explicit_role_preserves_requested_role_as_dispatch_target`.
  PR #340 integrated.
```

#### IO-001: task attempt artifact reads require regular file and max size

```text
Surface:
  vida task attempt collect/consolidate

Setup:
  artifact ref = symlink/FIFO/huge file/out-of-root path

Expected:
  blocked with canonical artifact read blocker
```

#### DOC-001: docflow closeout does not execute repo-local git helpers

```text
Surface:
  vida docflow closeout --changed
  docflow-cli closeout --changed

Setup:
  repo-local core.fsmonitor helper writes sentinel file if executed

Expected:
  sentinel file absent
  closeout still reports changed markdown
```

## Baseline Proof Commands

Copied from the source plan and kept pending until execution:

### IO-001 proof commands

```bash
cargo fmt --all -- --check
git diff --check
cargo test -p vida --test doctor_surface_contract_smoke -- --nocapture
cargo test -p vida --test boot_smoke -- --nocapture
cargo test -p vida --test task_smoke -- --nocapture
cargo test -p vida host_bridge -- --nocapture
cargo test -p vida status_surface -- --nocapture
cargo test -p vida taskflow_consume_resume -- --nocapture
```

### DOC-001 proof commands

```bash
cargo test -p docflow-cli --test cli_smoke -- --nocapture
cargo test -p docflow-cli --test cli_smoke closeout_changed_ignores_repo_local_fsmonitor_helper -- --nocapture
cargo test -p docflow-cli --test cli_smoke closeout_changed -- --nocapture
cargo build
```

## Wave Tracking Table

Fill this after each wave. Current baseline row remains pending until the full
Wave 0 batch proof is run.

| Wave | Name | Goal | Required artifact / tests | Proof commands | Status | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| 0 | Baseline freeze and defect fixtures | Freeze current behavior and create failing fixtures for all known bug classes before large movement. | `crates/vida/tests/host_bridge_authority_smoke.rs`, `crates/vida/tests/runtime_authority_smoke.rs`, `crates/vida/tests/taskflow_routing_smoke.rs`, `crates/vida/tests/artifact_io_smoke.rs`, `crates/docflow-cli/tests/git_hardening_smoke.rs` or the approved existing smoke files. | `cargo fmt --all -- --check`; `git diff --check`; targeted ROUTE/HB/IO/DOC proof commands; Wave 0 baseline proof batch from above | partial | IO-001, DOC-001, HB-001 through HB-005, ROUTE-001, and ROUTE-002 targeted proofs are green; PR #340 and #350 are integrated; broader Wave 0 baseline proof and remaining RT fixtures still need execution. |
| 1 | `runtime-path-policy` | Create a single safe IO/path boundary and remove direct unsafe reads/writes from authority-sensitive runtime paths. | `crates/runtime-path-policy/*` plus targeted call-site migration. | Wave 1 proof batch from the source plan. | pending |  |
| 2 | `operator-output` | Extract shared operator rendering and envelope logic. | `crates/operator-output/*` and moved human output code. | Wave 2 proof batch from the source plan. | pending |  |
| 3 | `taskflow-host-bridge` | Move host bridge request / provenance / completion logic behind a bounded boundary. | Host bridge request, provenance, completion, artifact scope modules. | Wave 3 proof batch from the source plan. | pending |  |
| 4 | `taskflow-authority` | Centralize stale guards, projection cache, terminal closure, and final snapshot authority checks. | Authority / stale-guard / projection-cache modules. | Wave 4 proof batch from the source plan. | pending |  |
| 5+ | Remaining waves | Continue staged extraction through routing, docflow hardening, task lifecycle use cases, and shell cleanup. | See source plan waves 5 through 23. | Use the per-wave proof batch defined in the source plan. | pending | Fill as each wave lands. |

## Per-Wave Checklist

Use this checklist after each wave and record the exact proof result in the notes column above.

- [ ] Wave goal implemented
- [ ] Required artifacts/tests created or migrated
- [ ] Focused test passed
- [ ] Adjacent public smoke passed
- [ ] Baseline proof commands recorded
- [ ] Document updated with wave outcome
- [ ] Debug build passed
- [ ] Release binary installed to the system `vida` path
- [ ] Installed `vida` smoke passed through the normal PATH
- [ ] TaskFlow state updated
- [ ] Commit created
- [ ] Push completed when authorized by current explicit operator instruction
- [ ] VIDA runtime self-diagnostic run after closure
- [ ] GitHub PR processing protocol run after wave close when authorized by
      current explicit operator instruction

## Delegated Task Packet Checklist

Use this checklist when handing the next bounded task to an implementation or
validation agent.

- Name the exact TaskFlow id, parent/wave, source-of-truth document, copied
  source anchors, and owned paths.
- State whether the packet must copy source wording exactly, summarize it, or
  implement behavior from it.
- For `gpt-5.4-mini` executor packets, use the highest available reasoning
  effort by default and keep the task narrow, sequential, and proof-explicit.
- Require the final report to include `tokens_used`, `steps_taken`, and
  `tool_calls_used`; if exact token usage is unavailable, use
  `tokens_used: not_exposed_by_host`.
- Require focused tests first, then `cargo build` debug proof before closure.
- Require TaskFlow state update and commit after every completed task; push
  when authorized by the current explicit operator instruction.
- Treat invalid, empty, stale, schema-missing, or scope-widening agent reports
  as non-evidence; do not use them for task closure.
- Validate cheap-agent implementation with root review plus a stronger
  validator for runtime authority paths; current minimum is `gpt-5.5-medium`.
- After a wave parent closes, run the VIDA runtime self-diagnostic and then
  `docs/process/github-pr-processing-protocol.md` when authorized by the
  current explicit operator instruction.

## Source Plan Anchors

These are the source-plan anchors this tracker is tied to:

- `## 4. Wave 0 — Baseline freeze and defect fixtures`
- `### 4.1 Create a baseline tracking document`
- `### 4.2 Add test tags / modules for known defect classes`
- `### 4.3 Add red tests for P0/P1 defects`
- `### 4.4 Baseline proof commands`
- `## 20. Final acceptance checklist`
- `## 21. Suggested commit sequence`
- `## 22. Agent execution notes`
- `## 23. Minimal next action for implementation agent`

## Completion Notes

- IO-001 is complete at focused proof level; the full Wave 0 baseline proof
  batch remains pending.
- HB-001 red-test proof is now covered in
  `crates/vida/src/agent_dispatch_surface.rs` on the public host-bridge path
  with explicit trusted-state-dir authority over an attacker request path;
  the blocked exit is asserted directly and `host_tool_calls = []` remains
  asserted through the shared payload helper.
- HB-002 production authority fix is now covered in
  `crates/vida/src/agent_dispatch_surface.rs` on the public host-bridge path
  with a real `.vida/data/state` root, bounded packet/result/receipt paths,
  and no persisted dispatch receipt; the blocked exit is asserted directly and
  `host_tool_calls = []` remains asserted through the shared payload helper.
- HB-003 production authority fix is now covered in
  `crates/vida/src/lane_surface.rs::host_bridge_no_request_redirect`; persisted
  dispatch result/receipt paths stay authoritative over mutable request JSON.
- HB-004 request-read guard is now covered in
  `crates/vida/src/lane_surface.rs::read_host_bridge_request_at_path` and the
  focused oversized/out-of-root/non-regular request-path regression test.
- HB-005 immutable-scope guard is now covered in
  `crates/vida/src/lane_surface.rs::host_bridge_implementation_scope_uses_immutable_packet`;
  mutable request `owned_paths` cannot widen TaskFlow packet scope.
- ROUTE-001 is now covered in
  `crates/vida/src/runtime_contract_vocab.rs`,
  `crates/vida/src/runtime_dispatch_execution.rs`, and
  `crates/vida/src/runtime_dispatch_state.rs`; developer and test-authoring
  labels resolve through configured `task_class` admissibility.
- ROUTE-002 is now covered in `crates/vida/src/init_surfaces.rs`; explicit
  agent-init role aliases cannot resolve to `orchestrator`, and requested
  human labels remain the selection `dispatch_target`.
- Open PR intake mapping:
  - PR #340: integrated as ROUTE-002.
  - PR #350: integrated as ROUTE-001.
  - PR #356: follow-up required for dispatch-target alias normalization; not
    integrated in this Wave 0 routing slice.
- Delegation scorecard for IO-001 / DOC-001:
  - executor `gpt-5.4-mini`: useful for bounded edits and repetitive proof
    runs, but required an additional pass to harden the docflow git-status
    boundary against repo-local fsmonitor execution.
  - HB-001 note: `gpt-5.4-mini` handled the public-path proof and the
    production state-root authority fix after validator rework.
  - validator `gpt-5.5-medium`: caught the semantic gap after focused tests
    were green and is effective as the minimum validator for authority-sensitive
    runtime paths.
  - orchestrator self-review: required to reconcile TaskFlow state, staging,
    docs tracking, and commit scope.
- Delegation scorecard for HB-003 / HB-004 / HB-005:
  - `gpt-5.4-mini` works as a cheap executor only when the packet is narrow,
    source-anchored, and proof-explicit; recent host-bridge packets produced
    useful drafts but still timed out or needed root correction.
  - `gpt-5.5-medium` is currently effective as the authority-path validator
    because it caught source-fidelity, false-green, and residual-risk issues.
  - Root orchestrator remains responsible for accepting evidence, updating
    TaskFlow, running debug build, installing the release binary after wave
    parent closure, committing, authorized push/PR protocol, and
    self-diagnostic.
- Delegation scorecard for ROUTE-001 / ROUTE-002:
  - `gpt-5.4-mini` was useful for PR triage/exploration with concrete mapping
    quality around 8-9/10, but write-agent runs for this slice timed out or
    failed to return final evidence, so they are not closure-quality execution
    evidence in this environment.
  - `gpt-5.5-medium` validator `Ramanujan` returned PASS after rework, quality
    score 8.5/10, validator self-score 8/10, 42 tool calls/steps, and token
    usage not exposed by host.
  - Root orchestrator performed the accepted code integration, focused proof,
    docs update, TaskFlow update, commit, authorized push, and authorized PR
    closure protocol.
- Next task routing recommendation: keep `gpt-5.4-mini` for narrow
  implementation packets only when the prompt names exact caller roots,
  fail-closed behavior, TaskFlow metadata updates, proof commands, and commit
  boundaries; keep `gpt-5.5-medium` as validator for runtime/authority code.

-----
artifact_path: product/spec/meta-refactor-runtime-boundary-execution-plan
artifact_type: document
artifact_version: 1
artifact_revision: 2026-06-11
schema_version: 1
status: canonical
source_path: docs/product/spec/meta-refactor-runtime-boundary-execution-plan.md
created_at: 2026-06-11T07:33:01.9883639Z
updated_at: 2026-06-11T07:33:01.9883639Z
changelog_ref: meta-refactor-runtime-boundary-execution-plan.changelog.jsonl
