# Agent Model Evaluation Log

Purpose: record per-task executor/validator efficiency evidence so the next VIDA
task can choose a cheaper or stronger model deliberately.

## 2026-06-11 - pr343-lane-retire-closure-proof-bypass

Scope:
- Task: `pr343-lane-retire-closure-proof-bypass`
- PR: `#343`
- File: `crates/vida/src/lane_surface.rs`
- Proof:
  - `cargo test -p vida lane_retire_rejects -- --nocapture`
  - `cargo test -p vida lane_retire_rejects_exception_takeover_missing_task_stale_blocked_run_without_closed_unit -- --nocapture`
  - `cargo test -p vida lane_retire_uses_exception_metadata_closed_unit_when_run_task_is_runtime_id -- --nocapture`
  - `cargo build`
  - `git diff --check -- crates/vida/src/lane_surface.rs`
  - `vida task validate-graph --json`
  - `vida task closure-ready pr343-lane-retire-closure-proof-bypass --json`

Observed model results:
- Initial executor `gpt-5.4-mini` with `xhigh` reasoning: 7/10. It removed the
  bridge/open bypass and added useful tests, but left the
  `exception_takeover_stale_blocked` bypass active and even had a positive test
  preserving that false-green path. Tokens were not exposed by the host; the
  final report listed 22 tool calls.
- Validator `gpt-5.5` with `medium` reasoning: 8/10. It correctly rejected
  closure because stale blocked exception takeover could still retire a lane
  without a closed TaskFlow unit. The validator reported 27 tool calls; tokens
  were not exposed by the host.
- Rework executor `gpt-5.4-mini` with `xhigh` reasoning: 9/10. It converted the
  validator finding into one exact rework packet, removed the stale-blocked
  bypass, renamed the negative regression test, and kept the metadata-backed
  positive path. Tokens were not exposed; the final report listed 27 tool calls.
- Final validator `gpt-5.5` with `medium` reasoning: 9/10 with 9/10 confidence.
  It accepted the diff and focused proof, with residual risk limited to the
  orchestrator-owned full build proof.

Next-task selection rule:
- Keep the three-step loop as the default for authority-sensitive runtime tasks:
  cheap executor with self-proof, one compact proof bundle plus one stronger
  validator, then one close/commit/push/PR/docs publication pass.
- If the validator rejects, do not reopen broad discovery. Convert the single
  blocking finding into one exact mini rework packet and rerun the compact proof
  bundle.
- Use `gpt-5.5 medium` as validator for lane, TaskFlow, exception-takeover, and
  closure-proof semantics because it caught the false-green path the cheap
  executor missed.
- Before PR #355 or any same-file dirty-target work, run a hunk-isolation
  preflight instead of giving a broad write packet to the mini executor.

## 2026-06-11 - pr356-dispatch-target-alias-policy

Scope:
- Task: `pr356-dispatch-target-alias-policy`
- PR: `#356`
- Files: `crates/vida/src/runtime_dispatch_execution.rs`,
  `crates/vida/src/runtime_dispatch_state.rs`
- Proof:
  - `cargo test -p vida preserves_lane_policy -- --nocapture`
  - `cargo test -p vida configured_dev_team_route_selects_current_task_class_slice_for_generic_task -- --nocapture`
  - `cargo build`
  - `git diff --check -- crates/vida/src/runtime_dispatch_execution.rs crates/vida/src/runtime_dispatch_state.rs`
  - `vida task validate-graph --json`
  - `vida task closure-ready pr356-dispatch-target-alias-policy --json`

Observed model results:
- Executor `gpt-5.4-mini` with `xhigh` reasoning: 8.5/10. It took a long wait
  and needed a checkpoint request, but produced a correct two-file patch,
  focused tests, build proof, and diff-check proof. Tokens were not exposed by
  the host. The final report listed tool categories instead of a numeric tool
  call count, so the orchestrator records tool-call telemetry as partial.
- Orchestrator adjustment: removed one `unused_mut` warning before final proof.
  No logic rework was needed after the mini patch.
- Validator `gpt-5.5` with `medium` reasoning: 8/10. It accepted the patch,
  reran the focused alias test and diff-check, added one ROUTE/task-class guard
  test, and identified a non-blocking gap: no positive launch test proving an
  admissible backend receives executable `dev`. The negative launch test plus
  downstream receipt test were sufficient for PR #356 closure.

Next-task selection rule:
- `gpt-5.4-mini xhigh` is efficient for two-file runtime policy work when the
  prompt names exact files, invariant, proof commands, and non-goals. Wait longer
  before classifying timeout; this task completed after extended waiting.
- Require numeric `tool_calls_used`; if an agent reports only tool categories,
  score telemetry completeness down even when implementation quality is high.
- Keep `gpt-5.5 medium` validator for routing/admissibility policy because it
  catches false-green risk and can add a targeted guard test without broadening
  implementation scope.
- For the next runtime PR slice, use mini-high/xhigh executor only if the target
  file is clean. If the file has unrelated dirty hunks, first create a hunk-safe
  isolation task or choose a clean PR.

## 2026-06-11 - pr342-packet-repair-binding-contract

Scope:
- Task: `pr342-packet-repair-binding-contract`
- Child rework task: `pr342-packet-repair-binding-contract-tests`
- PR: `#342`
- File: `crates/vida/src/taskflow_packet.rs`
- Proof:
  - `cargo test -p vida packet_repair_rejects_binding_mismatches_without_mutating_packet -- --nocapture`
  - `cargo test -p vida packet_repair_rejects_missing_or_invalid_template_kind_and_active_body -- --nocapture`
  - `cargo test -p vida packet_repair_json_cli_rejects_binding_mismatch_without_mutating_packet -- --nocapture`
  - `cargo test -p vida packet_repair -- --nocapture`
  - `cargo build`
  - `git diff --check -- crates/vida/src/taskflow_packet.rs`
  - `vida task validate-graph --json`
  - `vida task closure-ready pr342-packet-repair-binding-contract --json`

Observed model results:
- Initial executor `gpt-5.4-mini` with `high` reasoning: 7/10 for bounded
  production patching. It implemented the right ordering invariant: validate
  receipt/status/task/packet binding before mutation, then validate the repaired
  packet contract before persistence. Local proof passed before validator review.
- Validator `gpt-5.5` with `medium` reasoning: 8/10 on the first pass. It
  correctly rejected closure for missing mismatch matrix coverage and requested
  public CLI JSON proof. One rejection reason counted pre-existing dirty files as
  scope risk, so the orchestrator kept that as context rather than a blocker.
- Rework executor `gpt-5.4-mini` with `high` reasoning: 6/10. It added useful
  matrix and CLI test structure, but returned without build/diff-check and left
  two test-harness defects: overlapping `StateStore`/raw `SurrealKv` handles
  caused Windows lock failures, and the CLI JSON test initially passed through
  the missing-task load-error path instead of the packet mismatch path.
- Orchestrator repair: fixed the Windows datastore lock by scoping raw DB seeding
  before reopening `StateStore`, seeded a canonical task for the public CLI test,
  asserted JSON projection fields, and reran the proof bundle.
- Final validator `gpt-5.5` with `medium` reasoning: 9/10. It accepted closure,
  independently reran `cargo test -p vida packet_repair -- --nocapture`,
  `git diff --check -- crates/vida/src/taskflow_packet.rs`, and
  `vida task closure-ready pr342-packet-repair-binding-contract-tests --json`.
  Reported tokens were unavailable; reported validator work was 14 steps and 32
  underlying tool invocations.

Next-task selection rule:
- Keep `gpt-5.4-mini high` as executor for one-file runtime packets when the
  orchestrator supplies exact invariant, owned file, and proof commands, but do
  not let it self-close public CLI or Windows datastore-harness changes.
- Use `gpt-5.4-mini high` for test-matrix rework only after the validator names
  exact missing cases. Require local rerun because it may create false-green tests
  or leave harness locks.
- Keep `gpt-5.5 medium` as validator for packet repair, receipt authority, and
  public operator JSON. It was worth the cost here because it caught missing
  coverage before close and accepted quickly after focused rework.
- For the next PR slice, prefer mini-high executor with a smaller prompt and a
  mandatory final checklist: changed files, exact tests run, build status,
  diff-check status, `tokens_used` or `not_exposed_by_host`, `steps_taken`, and
  `tool_calls_used`.

## 2026-06-11 - pr352-358-dispatch-packet-read-path-safety

Scope:
- Task: `pr352-358-dispatch-packet-read-path-safety`
- PRs: `#352`, `#358`
- Files: `crates/vida/src/lane_surface.rs`, `crates/vida/src/status_surface.rs`,
  `crates/vida/src/taskflow_consume_resume.rs`,
  `crates/vida/src/taskflow_operator_diagnostics.rs`
- Proof:
  - `cargo test -p vida host_bridge_request_rejects_out_of_root_or_oversized_file -- --nocapture`
  - `cargo test -p vida read_lane_packet_reads_contained_packet_and_rejects_traversal_symlink_and_oversized_file -- --nocapture`
  - `cargo test -p vida status_dispatch_packet_refs -- --nocapture`
  - `cargo test -p vida consume_resume_error_payload_does_not_read_outside_packet_refs -- --nocapture`
  - `cargo test -p vida consume_continue_resume_error_payload_does_not_read_outside_packet_refs -- --nocapture`
  - `cargo build`
  - `git diff --check`

Observed model results:
- Executor `gpt-5.4-mini` with `xhigh` reasoning: 4/10 for broad shared
  implementation. It eventually returned a useful partial patch, but only after
  several long waits and an interrupt checkpoint; it also ran false-green
  `--lib` test filters, left a red focused test, and required rework.
- Rework executor `gpt-5.4-mini` with `high` reasoning: 8/10 for narrow red-test
  repair. With one failing test and one warning family, it fixed the issue,
  removed warning noise, and returned focused proof quickly.
- Validator `gpt-5.5` with `medium` reasoning: 8/10 for scope and risk review.
  It caught false-green filters and adjacent agent-dispatch path-safety gaps.
  One rejection point treated pre-existing dirty files as if they were part of
  the patch; the orchestrator split those findings into a separate TaskFlow item
  instead of mixing hunks.

Next-task selection rule:
- Do not give broad cross-file shared-invariant implementation to mini xhigh as
  one large packet when the same objective can be split by public surface.
- Prefer mini high for precise rework after the orchestrator has a red test,
  target files, and exact acceptance.
- Keep 5.5-medium as the validator for host-bridge/path-safety work; require it
  to distinguish scoped patch files from pre-existing dirty files.
- When a validator finds a real adjacent issue in a dirty file, create a new
  bounded TaskFlow item rather than silently expanding the current commit.

## 2026-06-11 - lane-surface-host-bridge-receipt-authority

Scope:
- Task: `lane-surface-host-bridge-receipt-authority`
- Files: `crates/vida/src/lane_surface.rs`, `crates/vida/tests/task_smoke.rs`
- Proof:
  - `cargo fmt --all -- --check`
  - `cargo test -p vida --test task_smoke external_attempt_scope_guard -- --exact --nocapture`
  - `cargo test -p vida lane_surface::tests::lane_complete_host_bridge_rejects_unverified_request_artifacts -- --exact --nocapture`

Observed model results:
- Executor `gpt-5.4-mini` with `xhigh` reasoning: 7/10. It produced the
  correct success-path direction and useful regression assertions, but timed out
  without a final report and missed blocked-result blocker preservation.
- Validator `gpt-5.5` with `medium` reasoning: 0/10 for this slice because it
  timed out and returned no actionable validation evidence.
- Orchestrator scoped fix: one narrow local correction under active exception
  takeover restored the blocked-result summary/delegated-cycle blocker.

Next-task selection rule:
- Use `gpt-5.4-mini` with `xhigh` only for one-file execution packets with one
  focused proof command and an explicit stop point.
- Do not use long-running `gpt-5.5 medium` validation prompts until the host
  agent timeout/runtime blocker is fixed; prefer shorter read-only prompts split
  by concrete risk.
- Require final reports to include proof commands, pass/fail counts, token usage
  if exposed, and tool-call count or an explicit "not exposed" statement.

## 2026-06-11 - wave0-consume-resume-missing-task-guard-order
- Task: `wave0-consume-resume-missing-task-guard-order`
- File: `crates/vida/src/taskflow_consume_resume.rs`
- Proof:
  - `cargo fmt --all -- --check`
  - `cargo test -p vida validate_run_graph_resume_state -- --nocapture`
  - `cargo test -p vida resolve_runtime_consumption_resume_inputs -- --nocapture`
Observed model results:
- Executor `gpt-5.4-mini` with `xhigh` reasoning: 0/10 for execution in this
  slice because the worker timed out without a completion receipt or patch
  closure.
- Validator `gpt-5.5` with `medium` reasoning: 8/10. It caught three real
  false-green risks: unsuperseded exception takeover authority, retry-eligible
  missing-task bypass, and candidate downstream packet evidence without result
  lineage. It did not run tests.
- Validator `gpt-5.4-mini` with `xhigh` reasoning: 7/10. It caught the remaining
  default no-`--run-id` explicit-binding redirect gap and supplied focused test
  evidence, but it missed the broader predicate risks caught by 5.5.
- Orchestrator scoped fix: accepted both validators, narrowed authority
  predicates, kept stale missing-task fail-closed behavior, and passed focused
  proof.
Next-task selection rule:
- Use `gpt-5.4-mini` with `xhigh` as a cheap focused validator for one failing
  test or one resolver path, not as the only broad reviewer.
- Use `gpt-5.5 medium` validator for predicate/authority reviews where
  false-green risk matters.

## 2026-06-11 - wave-0-runtime-authority-rt001-rt004

Scope:
- Parent: `wave-0-runtime-tests`
- Closed tasks:
  - `wave-0-red-tests-rt001`
  - `wave-0-red-tests-rt002`
  - `wave-0-red-tests-rt003`
  - `wave-0-red-tests-rt004`
- Proof families:
  - focused public CLI smoke tests for the changed operator surface,
  - focused unit/module tests for the shared authority helper,
  - `cargo fmt --all -- --check`,
  - `cargo build`,
  - `git diff --check`.

Observed model results:
- `gpt-5.4-mini` with `xhigh` reasoning is useful as a cheap executor only
  when the packet has one narrow file/surface cluster, named acceptance lines,
  one explicit proof command, and a hard stop. Across RT-001..RT-004 it produced
  useful decomposition or partial patches, but timed out or under-covered
  closure often enough that it should not self-close authority-sensitive work.
- `gpt-5.4-mini` with `xhigh` reasoning is useful as a cheap focused validator
  for one failing test, one resolver path, or one source-fidelity check. It is
  not sufficient as the only broad validator for TaskFlow, receipt authority,
  host-bridge, projection-cache, or public operator-surface changes.
- `gpt-5.5-low` is the preferred bounded rework executor after mini timeout,
  shutdown, partial patch, or validator rejection. It is cheaper than medium
  while still strong enough for narrow runtime implementation when the
  orchestrator provides exact files, invariants, proof commands, and non-goals.
- `gpt-5.5-medium` is the preferred authority validator. It caught false-green
  risks, scope limits, adjacent failures, and closure evidence gaps more
  consistently than mini validators. Keep prompts short and risk-specific
  because long broad validation prompts can still time out.
- Root orchestrator remains the consolidator. It may accept cheap-agent work as
  evidence, but closure requires local proof, TaskFlow update, debug build,
  commit, push when currently authorized, and a recorded scorecard.

Current routing recommendation:
- Executor first pass: `gpt-5.4-mini` with highest available reasoning for
  read-only decomposition, source-sync docs, exact regression-test authoring, or
  one small implementation packet with a single proof command.
- Executor rework: `gpt-5.5-low` when mini output is partial, times out, misses
  acceptance, or touches production runtime authority.
- Validator: `gpt-5.5-medium` for authority-sensitive code, public operator
  JSON, TaskFlow state, receipt logic, host bridge, release closure, and PR
  integration.
- Extra cheap validator: `gpt-5.4-mini` with highest reasoning only for a narrow
  second opinion over one named risk.
- Triple validation: use only when validators disagree, the patch changes a
  shared authority predicate, or the task would close a wave/epic/release gate.
- PR/open-source intake: use the same ladder as any source-neutral work item;
  bind an explicit TaskFlow item first, split by non-conflicting PR families, and
  keep GitHub mutation authority in the orchestrator.
- Timeout/shutdown/no-artifact: classify as `process_failure`, close the handle,
  score the attempt low, and either narrow the packet or escalate to
  `gpt-5.5-low`/`gpt-5.5-medium` according to executor or validator role.

Required agent final report:
- `changed_files` or reviewed read-only scope,
- `verification`,
- `gaps`,
- exact proof commands with pass/fail/not-run status,
- residual risks and blockers,
- `tokens_used` or `not_exposed_by_host`,
- `steps_taken`,
- `tool_calls_used`,
- `agent_score_10` assigned by the orchestrator after validation.

## 2026-06-11 - todo-agent-routing-optimization-docs

Scope:
- Task: `todo-agent-routing-optimization-docs`
- Files: `docs/process/team-development-and-orchestration-protocol.md`,
  `docs/process/multi-agent-stage-ensemble-protocol.md`,
  `docs/process/agent-model-evaluation-log.md`, `docs/process/agent-system.md`

Observed model results:
- Analyst `gpt-5.4-mini` with `xhigh` reasoning: 7/10. The first 120-second
  wait timed out and the orchestrator closed the handle too early, but after
  resume and an explicit compact final-report request the agent returned a
  useful section-level edit plan with `tokens_used`, `steps_taken`, and
  `tool_calls_used`.
- Orchestrator action: classified the attempt as `process_failure`, closed the
  host handle too early, resumed it after operator correction, accepted the
  returned handoff as evidence, activated a docs-only exception takeover, and
  updated the process docs directly.

Next-task selection rule:
- Use mini for read-only analysis when its result can arrive in parallel and the
  critical path can proceed without waiting, or when the orchestrator is willing
  to wait a longer interval and request a compact partial/final report before
  cleanup.
- For blocker-critical analysis, use `gpt-5.5-medium` or split mini prompts into
  smaller single-question probes with shorter expected artifacts.

-----
artifact_path: process/agent-model-evaluation-log
artifact_type: process_doc
artifact_version: '1'
artifact_revision: 2026-06-11
schema_version: '1'
status: active
source_path: docs/process/agent-model-evaluation-log.md
created_at: 2026-06-11T00:00:00+03:00
updated_at: 2026-06-11T22:31:00+03:00
changelog_ref: agent-model-evaluation-log.changelog.jsonl
