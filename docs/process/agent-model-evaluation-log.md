# Agent Model Evaluation Log

Purpose: record per-task executor/validator efficiency evidence so the next VIDA
task can choose a cheaper or stronger model deliberately.

## Required Scorecard Shape

Every completed task that used delegated execution, validation, or model-routing
evidence must add a compact scorecard before the next unrelated task starts.

Required fields:

1. task id, parent/wave, PR/source when applicable, owned files, commit hashes,
2. proof commands and pass/fail/not-run status,
3. executor model/carrier, reasoning effort, role, score out of 10, tokens used
   or `not_exposed_by_host`, tool-call count, step count, changed files, proof
   quality, and rework required,
4. validator model/carrier, reasoning effort, score out of 10, tokens used or
   `not_exposed_by_host`, tool-call count, step count, accepted/rejected verdict,
   false-green findings, missing proof, unrelated hunk findings, and residual
   risk,
5. orchestrator action, including local proof, staged-scope correction, TaskFlow
   close, commit, push, docs update, parent/wave closure check, and handle
   cleanup,
6. Post-Task Self-Analysis: cite the canonical STOP gate in
   `docs/process/project-orchestrator-operating-protocol.md`, record the base
   fields, all 20 fixed criteria outcomes, dynamic criteria created from the
   latest session segment, meta-analysis remediation outcomes, and
   `workflow_score_10`,
7. next-task selection rule that changes future routing, prompt shape, proof
   bundle, or model choice.

Do not invent token counts. If the host does not expose tokens or tool calls,
write `not_exposed_by_host` and record any observed approximate count only when
it is clearly labeled as approximate.

## 2026-06-12 - todo-agent-optimization-top-level-docs

Scope:
- Task: `todo-agent-optimization-top-level-docs`
- Parent: `pr-open-runtime-hardening-342-343-349-352-355-356-358-360-361`
- Commit: `01f4b14fe`
- Files: `AGENTS.sidecar.md`,
  `docs/process/project-orchestrator-operating-protocol.md`,
  `docs/process/team-development-and-orchestration-protocol.md`,
  `docs/process/project-orchestrator-startup-bundle.md`,
  `docs/process/agent-model-evaluation-log.md`
- Proof:
  - `git diff --check -- AGENTS.sidecar.md docs/process/project-orchestrator-operating-protocol.md docs/process/team-development-and-orchestration-protocol.md docs/process/project-orchestrator-startup-bundle.md docs/process/agent-model-evaluation-log.md`
  - `vida docflow check AGENTS.sidecar.md docs/process/project-orchestrator-operating-protocol.md docs/process/team-development-and-orchestration-protocol.md docs/process/project-orchestrator-startup-bundle.md docs/process/agent-model-evaluation-log.md --json`
  - `vida task validate-graph --json`
  - `cargo +1.95.0 build`

Observed model results:
- Orchestrator-only documentation implementation: 8/10. The task was prompted
  directly by the operator while pause was active, so no executor agent was
  launched. The change promoted wave-first execution, the post-task checklist,
  active publication authorization, and required scorecard shape to top-level
  project instructions. Tokens and tool-call usage are not exposed by the host.
- Validator: local DocFlow, diff hygiene, graph validation, and debug build.
  No separate model validator was launched because the bounded scope was
  documentation/process alignment and the operator explicitly asked to update
  instructions rather than run another model experiment.
- Orchestrator correction: fixed a sidecar numbering inconsistency by moving
  runtime-DX rules into a separate overlay instead of continuing the old list
  after the new epic optimization overlay.

Post-Task Self-Analysis:
- Worked: direct owner-doc edits were faster than delegating because the user
  asked for explicit instruction changes and the target docs were known.
- Waste: the first checklist wording hid self-analysis under generic
  optimization, which forced a second documentation pass.
- Risk: missing explicit self-analysis would let future tasks close with metrics
  but no process learning.
- Next change: require self-analysis as a named closure gate before unrelated
  work, and update docs immediately when the analysis changes a rule.
- Docs update: yes; top-level sidecar, orchestrator protocol, team protocol, and
  scorecard template were updated.
- workflow_score_10: 8/10. Proof and scope were solid, but the first pass missed
  one operator-required criterion.

Twenty criteria outcome:
1. Active bounded unit explicit: pass,
   `pr-open-runtime-hardening-342-343-349-352-355-356-358-360-361`.
2. Wave/parent closure distance: partial, this process task improves epic
   execution discipline but does not close a wave child.
3. Scope and non-goals stable: pass, docs-only instruction update.
4. Dirty worktree handled: pass, unrelated Rust/untracked files left unstaged.
5. Executor cheapest capable: pass, local orchestrator was cheaper than launching
   a model because the operator requested direct instruction changes.
6. Validator matched risk: pass, local DocFlow/diff/graph/build proof was enough
   for process docs; no authority code changed.
7. Prompt packet shape: pass for local TODO; no delegated prompt used.
8. Agent handles: pass, no new agents launched.
9. Token/tool/step telemetry: partial, host token/tool exact counts not exposed.
10. Avoidable commands: pass, identified missing self-analysis as avoidable
    second-pass cause.
11. Proof strength: pass, DocFlow, diff hygiene, graph, and build proof.
12. Public/release proof: not applicable, no public CLI behavior changed.
13. Debug build: pass, `cargo +1.95.0 build`.
14. TaskFlow state: pass, graph valid and closure-ready checked before close.
15. Staging by invariant: pass, docs-only files staged; unrelated Rust and
    untracked files remained unstaged.
16. Publication authorization: pass, active epic has repeatable task push
    instruction.
17. Evaluation docs: pass, this scorecard records the rule change.
18. Parent/wave metrics: pass, epic progress after close is 86/215 tasks
    closed, 40.0%; waves closed remain 0/13.
19. New defects/follow-ups: none required after this docs pass.
20. Next routing rule: pass, self-analysis STOP gate blocks future task starts
    until complete.

Dynamic criteria created from this session segment:
1. User-correction criterion: if the operator asks "where is X?" after a
   checklist answer, treat that as a missing explicit gate, not as a request for
   explanation only. Evidence: the next docs update must name the gate and show
   where it blocks continuation.
2. Dynamic-extension criterion: every Post-Task Self-Analysis must create
   session-specific criteria from events since the previous task closure, or
   explicitly prove that the fixed list already covered all new events.
3. Closure-delay criterion: when acceptance expands during a docs task, do not
   close or commit until the expanded acceptance is reflected in the owner doc,
   top-level overlay, and scorecard template.
4. Deduplication criterion: keep detailed criteria in one owner doc and make
   sidecar/team docs reference it, rather than copying full lists into multiple
   surfaces.
5. Pause/resume criterion: after pause is lifted by a write request, the
   orchestrator must state why the request is explicit resume for that bounded
   write and must not silently resume unrelated epic work until the requested
   docs/task slice closes.

Meta-analysis remediation:
- Waste remediation: converted hidden self-analysis into a named STOP gate.
- Risk remediation: expanded owner protocol to 20 criteria and required
  docs/scripts/code/tests/TaskFlow remediation decisions.
- Dynamic remediation: added a mandatory dynamic-criteria requirement so future
  self-analysis learns from the latest session segment instead of relying only
  on the fixed checklist.
- Documentation remediation: updated sidecar, orchestrator protocol, team
  protocol, startup bundle, and scorecard template.
- Script/code remediation: not needed for this docs-only task; future repeated
  misses should become a script/runtime guard or TaskFlow optimization defect.

Next-task selection rule:
- For future process-rule updates, use local orchestrator implementation when
  the operator asks for immediate instruction changes and the affected files are
  owner docs with clear wording. Use a mini read-only validator only when the
  requested rule conflicts with existing process law or spans more than one
  owner layer.
- After this task, every closed task must run the post-task checklist and record
  scorecard evidence before unrelated work starts.

## 2026-06-12 - pr355-host-bridge-artifact-state-root

Scope:
- Task: `pr355-host-bridge-artifact-state-root`
- PR: `#355`
- File: `crates/vida/src/agent_dispatch_surface.rs`
- Commit: `5885bbf11`
- Proof:
  - `cargo +1.95.0 test -p vida host_bridge_attach_artifact_records_attempt_authority_and_updates_request -- --nocapture`
  - `wsl.exe bash -lc 'cd /mnt/c/project/vida-stack && cargo +1.95.0 test -p vida --bin vida host_bridge_attach_artifact_blocks_symlinked_normalized_artifact_directory -- --nocapture'`
  - `cargo +1.95.0 build`
  - `rustfmt +1.95.0 --edition 2021 --check crates/vida/src/agent_dispatch_surface.rs`
  - `git diff --cached --check`
  - `vida task validate-graph --json`
  - `vida task closure-ready pr355-host-bridge-artifact-state-root --json`

Observed model results:
- Read-only hunk-isolation executor `gpt-5.4-mini` with `xhigh` reasoning:
  9/10. It correctly identified that PR #355 hunks were disjoint from existing
  dirty provenance hunks and recommended hunk-safe staging. Tokens were not
  exposed by the host.
- Initial write executor `gpt-5.4-mini` with `xhigh` reasoning: 8/10 after
  validation. It produced the correct production helper and call-site shape, but
  its Unix regression was false-green because Windows ran zero `#[cfg(unix)]`
  tests and the test path relied on an unrelated dirty provenance hunk.
- First validator `gpt-5.5` with `medium` reasoning: 8/10 with 8/10 confidence.
  It rejected closure for the exact false-green risk: the symlink regression did
  not independently prove the normalized artifact writer without dirty
  provenance behavior.
- Rework executor `gpt-5.4-mini` with `xhigh` reasoning: 8.5/10. It added a
  receipt-valid test fixture so the Unix regression reached the writer without
  relying on unrelated hunks, but still reported only Windows zero-test proof.
- Orchestrator correction: ran the exact Linux target with
  `cargo +1.95.0 test -p vida --bin vida ...`, fixed the missing bin-test import,
  reran Linux proof with one real test, reran Windows compatibility, build,
  file-scoped rustfmt, and cached diff-check.
- Final validator `gpt-5.5` with `medium` reasoning: 9/10 with 9/10 confidence.
  It accepted closure and confirmed unrelated dirty hunks were no longer required
  for the PR #355 proof path.

Session optimization rule:
- Before launching an executor for platform-gated tests, identify the exact cargo
  target that owns the test. Tests inside `crates/vida/src/main.rs` require
  `cargo test -p vida --bin vida <test-name>`, not broad `cargo test -p vida
  <test-name>`, which may compile integration tests and waste minutes.
- For same-file dirty targets, keep the read-only mini hunk-isolation preflight;
  it prevented broad writes and enabled hunk-safe staging.
- A cheap executor may own the patch, but it must either run the platform-gated
  proof on the platform where the test is compiled or explicitly report the proof
  gap. Treat `0 tests` as a blocker until a real platform proof exists.
- Use the stronger validator after local proof, not before the platform target is
  known. This avoids paying the validator to find a proof-target error the
  orchestrator can detect with one exact command.

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

## 2026-06-12 - pr355-agent-dispatch-host-bridge-read-path-safety

Scope:
- Task: `pr355-agent-dispatch-host-bridge-read-path-safety`
- PR: `#355`
- File: `crates/vida/src/agent_dispatch_surface.rs`
- Proof:
  - `cargo +1.95.0 test -p vida host_bridge_provenance_blocks_request_outside_state_root -- --nocapture`
  - `cargo +1.95.0 test -p vida host_bridge_request_untrusted_path_explicit_state_dir_is_authoritative -- --nocapture`
  - `cargo +1.95.0 test -p vida host_bridge_missing_receipt_blocks_pending_request -- --nocapture`
  - `cargo +1.95.0 test -p vida host_bridge_provenance_accepts_pending_bridge_receipt -- --nocapture`
  - `cargo +1.95.0 test -p vida host_bridge_request_read_rejects_state_root_escape_and_oversized_files -- --nocapture`
  - `cargo +1.95.0 test -p vida host_bridge_packet_reader_rejects_non_regular_files_and_oversized_packets -- --nocapture`
  - `cargo +1.95.0 build`
  - `rustfmt +1.95.0 --edition 2021 --check crates/vida/src/agent_dispatch_surface.rs`
  - `git diff --check -- crates/vida/src/agent_dispatch_surface.rs`
  - `git diff --cached --check`
  - `vida task validate-graph --json`
  - `vida task closure-ready pr355-agent-dispatch-host-bridge-read-path-safety --json`

Observed model results:
- Preflight analyst `gpt-5.4-mini` with `xhigh` reasoning: 8/10. It correctly
  split on-task read-path hunks from unrelated dirty hunks, named the missing
  containment/non-regular/oversize proof cases, and supplied a hunk-safe staging
  plan. Reported token usage was not exposed; observed tool-call count was about
  36.
- Executor `gpt-5.4-mini` with `xhigh` reasoning: 9/10. It implemented the
  bounded read-path invariant in one production file, added focused regression
  tests, ran focused tests plus build and rustfmt proof, and preserved unrelated
  worktree hunks. Reported token and tool-call usage were not exposed.
- Validator `gpt-5.5-medium`: 8/10. It accepted the implementation, confirmed
  state-root containment before JSON parsing, capped artifact reads, and checked
  reconciled packet/receipt reads. It left one non-blocking residual risk: no
  dedicated symlink regression test beyond the shared `symlink_metadata` guard.
- Orchestrator action: repaired the staged index so the commit excluded an
  unrelated `attach_artifacts` provenance hunk, closed the TaskFlow item, ran
  graph and closure gates, committed, and pushed.

Next-task selection rule:
- Keep `gpt-5.4-mini xhigh` as the default executor for one-file read-path
  safety work when the orchestrator supplies exact invariants, proof commands,
  and dirty-hunk boundaries.
- Keep `gpt-5.4-mini xhigh` as a cheap preflight reviewer for hunk
  classification before coding in a dirty worktree.
- Keep `gpt-5.5-medium` as the validator for host-bridge artifact intake,
  receipt/provenance authority, and path-safety work; require it to state
  residual risk separately from closure blockers.
- In dirty files, stage by invariant rather than by file. If an executor returns
  an adjacent valid-looking hunk, split it into a separate TaskFlow item unless
  the active bounded unit explicitly owns it.

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
