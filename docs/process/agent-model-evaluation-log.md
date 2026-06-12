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
   fields, all 20 fixed criteria outcomes, meta-analysis remediation outcomes,
   and then run the mandatory final dynamic criteria STOP point as the last
   checklist item. That final point must analyze the session segment from the
   previous task closure to the current task closure and create additional
   criteria for the next task; it is separate from, and cannot be replaced by,
   the fixed 20 criteria. Record `workflow_score_10`,
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

## 2026-06-12 - wave-0-baseline-rustfmt-normalization

Scope:
- Task: `wave-0-baseline-rustfmt-normalization`
- Parent: `wave-0-baseline-proof`
- Commit: `473064139`
- Files: `crates/vida/src/lane_surface.rs`,
  `crates/vida/src/runtime_dispatch_execution.rs`,
  `crates/vida/src/runtime_dispatch_state.rs`,
  `crates/vida/src/state_store_run_graph_summary.rs`,
  `crates/vida/src/taskflow_consume_resume.rs`,
  `crates/vida/src/taskflow_operator_diagnostics.rs`,
  `crates/vida/src/taskflow_packet.rs`
- Proof:
  - `cargo +1.95.0 fmt --all -- --check --files-with-diff`
  - `cargo +1.95.0 fmt --all`
  - `cargo +1.95.0 fmt --all -- --check`
  - `git diff --cached --check`
  - `vida task closure-ready wave-0-baseline-rustfmt-normalization --json`

Observed model results:
- Executor: local orchestrator mechanical formatting, 9/10. No model executor
  was launched because the proof blocker was an exact rustfmt drift list and the
  safe action was deterministic. Tokens/tool-call counts are not exposed by the
  host.
- Validator: local rustfmt check plus cached diff hygiene, 9/10. It verified
  the formatting invariant and staged only the seven files reported by the
  pre-format `--files-with-diff` command.
- Agent state: the mini read-only Wave 0 proof preflight was still running and
  not used for this mechanical formatting closure.

Post-Task Self-Analysis:
- Worked: `--files-with-diff` isolated the real rustfmt blocker and avoided
  broad guessing.
- Waste: the first full proof run failed on fmt before other commands; future
  proof bundles should preflight `cargo fmt --all -- --check --files-with-diff`
  when the worktree is dirty.
- Risk: `cargo fmt --all` also formatted pre-existing dirty files. They were
  deliberately left unstaged, but the working tree now contains formatted dirty
  hunks that still need ownership classification before later commits.
- Next change: before broad proof bundles in a dirty worktree, run the cheap
  hunk/format preflight and stage only pre-identified invariant files.
- Docs update: no fixed-doc change needed; this is covered by the dynamic
  dirty-worktree and proof-bundle criteria.
- workflow_score_10: 8/10. The blocker was resolved quickly, but the broad fmt
  command touched dirty files and required extra staging discipline.

Twenty criteria outcome:
1. Active bounded unit explicit: pass, `wave-0-baseline-rustfmt-normalization`.
2. Wave/parent closure distance: pass, unblocked `wave-0-baseline-proof` fmt.
3. Scope and non-goals stable: pass, rustfmt-only normalization.
4. Dirty worktree handled: partial, dirty files were preserved unstaged but were
   formatted by the tool.
5. Executor cheapest capable: pass, deterministic local command.
6. Validator matched risk: pass, rustfmt and diff hygiene.
7. Prompt packet shape: not applicable, no executor prompt.
8. Agent handles: partial, unrelated preflight mini still running.
9. Token/tool/step telemetry: partial, host does not expose exact root tokens.
10. Avoidable commands: pass, identified fmt preflight ordering improvement.
11. Proof strength: pass for formatting invariant.
12. Public/release proof: not applicable.
13. Debug build: covered by the just-prior docs task build; this normalization
    only changed formatting and will be covered by returning to Wave 0 proof.
14. TaskFlow state: pass, closure-ready and task close succeeded.
15. Staging by invariant: pass, only seven rustfmt-reported files staged.
16. Publication authorization: pass, active epic repeatable push instruction.
17. Evaluation docs: pass, this scorecard.
18. Parent/wave metrics: pass, epic progress after close is 87/216 tasks
    closed, 40.28%; waves closed remain 0/13.
19. New defects/follow-ups: none yet; dirty formatted hunks remain to classify.
20. Next routing rule: pass, run fmt preflight before broad proof bundles in a
    dirty worktree.

Dynamic criteria created from this session segment:
1. Dirty-format criterion: when a workspace-wide formatter is needed in a dirty
   worktree, capture `--files-with-diff` before formatting and stage only that
   pre-identified set unless the active task owns the other dirty files.
2. Proof-order criterion: proof bundles in dirty worktrees should start with the
   cheapest fail-fast formatter/linter before launching long test batches.
3. Running-agent criterion: a read-only agent that is unrelated to the just
   closed mechanical subtask may remain open, but it must be classified before
   its parent proof task closes.

Meta-analysis remediation:
- Waste remediation: next Wave 0 proof rerun starts after fmt pass, avoiding
  another immediate formatter failure.
- Risk remediation: dirty formatted files remain unstaged and must be classified
  before any commit that could include them.
- Documentation remediation: no fixed checklist change; dynamic criteria above
  are enough unless this pattern repeats.

Next-task selection rule:
- Return to `wave-0-baseline-proof` and rerun its declared proof bundle now that
  the formatter blocker is removed. Keep waiting for the mini preflight long
  enough to classify its result before parent proof closure.

## 2026-06-12 - wave-0-runtime-proof-host-bridge-stale-path-fix

Scope:
- Task: `wave-0-runtime-proof-host-bridge-stale-path-fix`
- Parent: `wave-0-runtime-tests`
- Commit: `292ca411e`
- Files: `crates/vida/src/agent_dispatch_surface.rs`,
  `crates/vida/src/init_surfaces.rs`,
  `crates/vida/src/taskflow_consume_resume.rs`
- Proof:
  - `cargo +1.95.0 fmt --all -- --check`
  - `git diff --cached --check`
  - `cargo +1.95.0 test -p vida --test doctor_surface_contract_smoke -- --nocapture`

Observed model results:
- Executor: local orchestrator, 8/10. The fix was small but touched shared
  runtime authority paths, so root kept implementation local and used exact
  failing smoke tests as the primary feedback loop.
- Validator: local focused smoke proof, 9/10. `doctor_surface_contract_smoke`
  passed with 37 tests and 2 ignored tests after the fix.
- Scout: `gpt-5.4-mini` read-only explorer, 8/10. It correctly identified that
  RT coverage already existed and that `wave-0-runtime-tests` needed proof,
  not new test files. Host did not expose token counts.

Post-Task Self-Analysis:
- Worked: clean-HEAD reproduction separated real proof failures from dirty
  worktree suspicion before patching.
- Waste: broad `boot_smoke` was launched too early after focused doctor fixes
  and produced a large unrelated failure set; future Wave 0 proof should run
  focused failing package first, then broaden in smaller batches.
- Risk: `agent_dispatch_surface.rs` already had an unstaged host-bridge
  attach-artifact hunk; the final commit included host-bridge surface changes
  together, while unrelated dirty files remain unstaged.
- Next change: classify the remaining `boot_smoke` baseline blocker as a
  separate proof blocker for `wave-0-runtime-tests`; do not close the parent
  until declared proof is green.
- Docs update: this scorecard records the new dynamic proof-isolation criteria;
  no fixed instruction change is needed unless the pattern repeats.
- workflow_score_10: 8/10. The focused defect closed with proof, but broad proof
  ordering created avoidable noise.

Twenty criteria outcome:
1. Active bounded unit explicit: pass,
   `wave-0-runtime-proof-host-bridge-stale-path-fix`.
2. Wave/parent closure distance: pass, direct blocker under
   `wave-0-runtime-tests`.
3. Scope and non-goals stable: pass, host-bridge/path/stale-run proof only.
4. Dirty worktree handled: partial, unrelated dirty files preserved; one
   pre-existing host-bridge hunk shared the same staged surface.
5. Executor cheapest capable: pass, local root fix after exact reproduction.
6. Validator matched risk: pass, full `doctor_surface_contract_smoke`.
7. Prompt packet shape: not applicable, no write-producing subagent.
8. Agent handles: pass, scout was closed after result capture.
9. Token/tool/step telemetry: partial, scout tool calls visible; tokens hidden.
10. Avoidable commands: partial, broad `boot_smoke` ran before proof isolation.
11. Proof strength: pass for closed child; parent proof still blocked.
12. Public/release proof: not applicable.
13. Debug build: implicit through cargo test compile; broad build still pending.
14. TaskFlow state: pass, child task created and closed with explicit evidence.
15. Staging by invariant: partial, staged host-bridge surface included an
    already-present same-surface hunk.
16. Publication authorization: pass, active epic repeatable push instruction.
17. Evaluation docs: pass, this scorecard.
18. Parent/wave metrics: pass, epic progress after close is 88/217 tasks
    closed, 40.55%; waves closed remain 0/13.
19. New defects/follow-ups: remaining `boot_smoke` baseline blocker under
    `wave-0-runtime-tests`.
20. Next routing rule: pass, continue with the remaining declared proof blocker
    before `wave-0-baseline-proof`.

Dynamic criteria created from this session segment:
1. Clean-HEAD criterion: when dirty files plausibly explain proof failures, run
   the same focused proof against clean `HEAD` before attributing blame.
2. Same-surface staging criterion: if an existing unstaged hunk shares the same
   runtime surface as the fix, explicitly record whether it was staged or left
   unstaged and why.
3. Broad-proof escalation criterion: after focused proof is green, broaden one
   package at a time; if the first broad package explodes, stop and classify the
   new blocker instead of mixing it into the just-closed child task.

Meta-analysis remediation:
- Waste remediation: next work starts from the `boot_smoke` failure set, not from
  the already-green doctor smoke tests.
- Risk remediation: keep unrelated dirty files unstaged and re-check `git
  status --short` before every commit.
- Documentation remediation: dynamic criteria are enough for now; promote to
  fixed checklist only if another broad-proof noise event repeats.

Next-task selection rule:
- Continue `wave-0-runtime-tests` by isolating the remaining `boot_smoke`
  blocker. Do not close `wave-0-runtime-tests` or `wave-0-baseline-proof` until
  the declared proof bundle passes.

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

## 2026-06-12 - wave-0-runtime-proof-bundle-check-final-resume-fix

Scope:
- Task: `wave-0-runtime-proof-bundle-check-final-resume-fix`
- Parent: `wave-0-runtime-tests`
- Commit: `9f119139e`
- Files: `crates/vida/src/taskflow_consume_resume.rs`,
  `crates/vida/src/status_surface_json_report.rs`,
  `crates/vida/src/taskflow_operator_diagnostics.rs`
- Process instruction update: `docs/process/project-orchestrator-operating-protocol.md`
- Proof:
  - `cargo +1.95.0 fmt --all -- --check`
  - `cargo +1.95.0 test -p vida --test boot_smoke taskflow_consume_continue_prefers_latest_final_snapshot_after_bundle_check -- --nocapture --exact`
  - `cargo +1.95.0 test -p vida --test boot_smoke taskflow_consume_continue_resumes_from_persisted_final_snapshot -- --nocapture --exact`
  - `cargo +1.95.0 test -p vida --test boot_smoke consume_continue_repeated_run_id_after_success_fails_closed_without_closure_projection -- --nocapture --exact`
  - `cargo +1.95.0 test -p vida --bin vida taskflow_consume_resume::tests::resolve_runtime_consumption_resume_inputs_without_run_id_switches_to_fresh_bound_task_run -- --nocapture --exact`
  - `cargo +1.95.0 test -p vida --test doctor_surface_contract_smoke -- --nocapture`
  - `cargo +1.95.0 test -p vida status_surface_json_report -- --nocapture`
  - `cargo +1.95.0 test -p vida taskflow_operator_diagnostics -- --nocapture`
  - `git diff --check -- crates/vida/src/taskflow_consume_resume.rs crates/vida/src/status_surface_json_report.rs crates/vida/src/taskflow_operator_diagnostics.rs docs/process/project-orchestrator-operating-protocol.md`
  - `vida docflow check docs/process/project-orchestrator-operating-protocol.md --json`

Observed model results:
- Executor: local orchestrator plus read-only explorer Bohr, 8/10. Bohr found
  the correct early stale-missing-task short-circuit and was closed after the
  result was accepted.
- Validator: local focused smoke tests plus doctor/status/diagnostics filters,
  8/10. Targeted proof caught one over-broad replay regression and forced the
  final distinction between admissible final snapshots and recorded blocked
  final snapshots.
- Residual risk: full `cargo +1.95.0 test -p vida --bin vida
  taskflow_consume_resume -- --nocapture` still has one independent backend
  rotation failure: expected `opencode_cli`, actual `internal_subagents`.

Post-Task Self-Analysis:
- Worked: the explorer diagnosis shortened root-cause search; targeted tests
  exposed both the original stale-missing replay bug and the repeated-run
  regression before commit.
- Waste: several probe runs were spent before identifying the default
  stale-preflight and status summary paths; future replay fixes should search all
  stale emitters before patching one branch.
- Risk: `cargo fmt --all` also touched pre-existing dirty files, and a broad
  module filter surfaced an unrelated backend-selection failure.
- Next change: for resume/status fixes, run a three-test guard immediately:
  original failing smoke, persisted-final positive, and repeated-run negative.
- Docs update: promoted the operator-requested dynamic-criteria rule so the
  dynamic criteria step is explicitly final and expected to create at least one
  new criterion after each task.
- workflow_score_10: 8/10. The final patch is bounded and proven, but the first
  implementation pass was too broad and required regression narrowing.

Twenty criteria outcome:
1. Active bounded unit explicit: pass,
   `wave-0-runtime-proof-bundle-check-final-resume-fix`.
2. Wave/parent closure distance: pass, Wave 0 runtime proof blocker reduced.
3. Scope and non-goals stable: pass, consume-resume/status diagnostics only.
4. Dirty worktree handled: partial, unrelated dirty files remain unstaged.
5. Executor cheapest capable: pass, local fix plus one read-only explorer.
6. Validator matched risk: pass, focused smoke plus doctor/status diagnostics.
7. Prompt packet shape: pass for explorer; one bounded failing test and stop
   condition.
8. Agent handles: pass, Bohr was closed after result acceptance.
9. Token/tool/wait telemetry: partial, exact host tokens not exposed.
10. Avoidable commands: partial, repeated debug runs before all stale emitters
    were mapped.
11. Proof strength: pass, positive and negative replay contracts covered.
12. Public-surface proof: pass, `status --json` runtime-consumption parity and
    diagnostics classifier were covered by targeted filters.
13. Debug build: pass, cargo test rebuilt the `vida` binary target.
14. TaskFlow state: pass, task created and closed with proof evidence.
15. Staging by invariant: pass, code commit staged only three owned code files.
16. Publication authorization: pass, commit `9f119139e` pushed to `main`.
17. Evaluation docs: pass, this scorecard records the STOP gate.
18. Parent/wave metrics: pass, epic progress after close is 89/218 tasks
    closed, 40.83%; waves closed remain 0/13.
19. New defects/follow-ups: partial, backend-rotation unit failure remains a
    residual runtime/config drift to classify before full wave closure.
20. Next routing rule: pass, replay fixes must pair positive replay tests with
    repeated-run negative tests.

Meta-analysis remediation:
- Waste remediation: added all-stale-emitter search as a dynamic criterion.
- Risk remediation: narrowed recorded-final replay to missing-task stale
  recovery and strict explicit-run protection.
- Documentation remediation: updated the project orchestrator protocol so the
  dynamic criteria step is final and expected after every task.
- Follow-up: classify the backend-rotation unit failure before closing
  `wave-0-runtime-tests`.

Final dynamic criteria STOP point:
1. All-stale-emitter criterion: before changing a stale/blocker diagnostic,
   search every emitter and preflight path for the blocker code; expected
   evidence is an `rg` result or call-site list in the task note.
2. Positive-negative replay criterion: any persisted-final replay fix must run
   one positive replay test and one repeated/explicit-run negative test before
   commit.
3. Summary-field parity criterion: when a command writes a snapshot consumed by
   `status --json`, verify the compact/default JSON surface still exposes the
   legacy/public field expected by existing smoke tests.
4. Dynamic-final-step criterion: after the fixed self-analysis criteria and
   remediation are recorded, always add at least one session-derived dynamic
   criterion or explicitly justify why none was created.

## 2026-06-12 - wave-0-runtime-status-doctor-output-parity

Scope:
- Task: `wave-0-runtime-tests-boot-smoke-failure-classification`
- Parent: `wave-0-runtime-tests`
- Commit: `82cdbd53d`
- Files: `crates/vida/src/status_surface.rs`,
  `crates/vida/src/status_surface_text_report.rs`,
  `crates/vida/src/status_surface_json_report.rs`,
  `crates/vida/src/doctor_surface.rs`
- Proof:
  - `cargo +1.95.0 fmt --all -- --check`
  - `cargo +1.95.0 test -p vida --test boot_smoke status_surface -- --nocapture`
  - `cargo +1.95.0 test -p vida --test boot_smoke status_json_exposes_host_agent_summary -- --nocapture --exact`
  - `cargo +1.95.0 test -p vida --test boot_smoke status_and_doctor_text_surfaces_report_non_empty_latest_flow_state -- --nocapture --exact`

Observed model results:
- Executor: local orchestrator, 8/10. No worker was needed because the failing
  output shape mapped directly to existing human renderers in status and doctor.
- Validator: focused status/doctor smoke tests, 8/10. The next lock-remediation
  test still fails separately and is not included in this slice.

Post-Task Self-Analysis:
- Worked: manual reproduction exposed quoted TOON strings in default plain text.
- Waste: first status fix did not include doctor, requiring a second exact test.
- Risk: compact JSON and plain output were coupled in the status path.
- Next change: when output mode changes, test one plain text case and one explicit
  JSON case before commit.
- Docs update: no owner-doc rule change; this is covered by the dynamic
  plain-vs-json output criterion below.
- workflow_score_10: 8/10.

Twenty criteria outcome:
1. Active bounded unit explicit: pass, status/doctor output parity slice.
2. Wave/parent closure distance: pass, multiple boot_smoke failures removed.
3. Scope and non-goals stable: pass, output rendering only.
4. Dirty worktree handled: pass, unrelated dirty files remained unstaged.
5. Executor cheapest capable: pass, local exact edit.
6. Validator matched risk: pass, focused smoke tests.
7. Agent prompts: not applicable.
8. Agent handles: pass, no active agent remained.
9. Telemetry: partial, host token counts unavailable.
10. Avoidable commands: pass, manual reproduction was useful and bounded.
11. Proof strength: pass for status/doctor output parity.
12. Public-surface proof: pass, plain and JSON status surfaces covered.
13. Debug build: pass, cargo tests rebuilt `vida`.
14. TaskFlow state: pass, classification task remains active with notes.
15. Staging by invariant: pass, four output files staged.
16. Publication authorization: pass, commit pushed to `main`.
17. Evaluation docs: pass, this scorecard.
18. Parent/wave metrics: unchanged until the classification/repair task closes.
19. New defects/follow-ups: lock-remediation fail-fast remains next cluster.
20. Next routing rule: output changes must separate plain human, compact default,
    and explicit JSON contracts.

Final dynamic criteria STOP point:
1. Plain-vs-json output criterion: for every command-output repair, explicitly
   name whether the task owns plain human text, default compact output, explicit
   JSON, or all three; run at least one proof per owned mode.

## 2026-06-12 - wave-0 status/doctor read-lock fail-fast

Task / slice:
- `wave-0-runtime-tests-boot-smoke-failure-classification`
- Commit: `d968a8cf1 fix status doctor read lock fail fast`
- Goal: make status/doctor read-only operator surfaces fail closed quickly under
  read-lock contention and avoid Windows bounded-command timeouts on default
  summary JSON.

Proof:
- `cargo +1.95.0 fmt --all -- --check`
- `git diff --check -- crates/vida/src/status_surface.rs crates/vida/src/doctor_surface.rs`
- `cargo +1.95.0 test -p vida --test boot_smoke status_and_doctor_fail_closed_with_lock_remediation_hint -- --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke status_and_doctor_text_surfaces_fail_closed_with_lock_remediation_hint -- --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke diagnostics_status_and_doctor_share_closed_run_projection_blocker -- --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke status_json_exposes_host_agent_summary -- --exact`
- `cargo +1.95.0 test -p vida status_surface_json_report -- --nocapture`

Residual blockers observed:
- `cargo +1.95.0 test -p vida --test boot_smoke status_and_doctor -- --nocapture`
  now fails only on
  `taskflow_golden_route_happy_path_stitches_bootstrap_dispatch_resume_status_and_doctor`
  with `source_run_id: Null`, which belongs to consume/resume run-id repair.
- `cargo +1.95.0 test -p vida --bin vida status_surface::tests::runtime_continuation_overlay_does_not_keep_stale_root_session_write_guard -- --nocapture --exact`
  still fails on projection-cache fixture readability; this is a separate cache
  test defect and was not widened into this lock/read-surface slice.

Observed model results:
- Executor: local orchestrator, 8/10. A delegated reader would have helped with
  first-pass failure classification, but the fix needed tight local code/test
  iteration and no write-producing agent was active.
- Validator: focused boot_smoke exact tests plus status JSON report unit filter,
  8/10. Broad `status_and_doctor` is still blocked by an unrelated golden-route
  run-id assertion.

Post-Task Self-Analysis:
- Worked: exact lock tests separated stale `LOCK` file presence from real OS lock
  contention, preventing false degraded status on normal state directories.
- Worked: compact summary JSON removed a Windows stdout pipe risk without changing
  parsed JSON keys required by host-agent status proof.
- Waste: initial preflight only checked `LOCK`; the real serialization guard was
  `.vida-authoritative-open.guard`.
- Risk: lowering read-surface timeout to 2s could be too aggressive for very slow
  disks, but read-only operator surfaces should degrade instead of hanging.
- Next change: repair golden-route consume/resume `source_run_id` separately.
- Docs update: dynamic parallel/pipe-budget criterion added below.
- workflow_score_10: 8/10.

Twenty criteria outcome:
1. Active bounded unit explicit: pass, status/doctor read-lock fail-fast inside
   boot_smoke failure classification.
2. Wave/parent closure distance: pass, removed diagnostics/status timeout from
   the status/doctor cluster; parent still blocked by golden-route and broader
   boot_smoke failures.
3. Scope and non-goals stable: pass, did not widen into consume/resume run-id or
   projection-cache unit repair.
4. Dirty worktree handled: pass, unrelated dirty files remained unstaged.
5. Executor cheapest capable: pass, local orchestrator was cheaper than a new
   write agent for a two-file runtime surface patch.
6. Validator matched risk: pass, exact lock, diagnostics, JSON host-agent, and
   status JSON report tests ran.
7. Agent prompts: not applicable, no new agent was launched for this slice.
8. Agent handles: pass, no active agent remained from this slice.
9. Telemetry: partial, command durations observed; host token/cost unavailable.
10. Avoidable commands: partial, one broad status filter was useful but exposed an
    unrelated failure; future runs should pair exact proof with one scoped broad
    smoke only after residuals are classified.
11. Proof strength: pass for lock/read-surface behavior.
12. Public-surface proof: pass, status and doctor JSON/text lock-remediation
    surfaces covered.
13. Debug build: pass, cargo tests rebuilt `vida`.
14. TaskFlow state: partial, classification task remains active because this was
    a sub-cluster, not full parent closure.
15. Staging by invariant: pass, only `status_surface.rs` and `doctor_surface.rs`
    staged for the code commit.
16. Publication authorization: pass, commit pushed to `main`.
17. Evaluation docs: pass, this scorecard.
18. Parent/wave metrics: unchanged until `wave-0-runtime-tests` can close.
19. New defects/follow-ups: consume/resume `source_run_id` and projection-cache
    fixture readability remain separate blockers.
20. Dynamic criteria generation: pass, session segment from previous task closure
    to this commit produced the new criterion below.

Final dynamic criteria STOP point:
1. Parallel-suite and pipe-budget criterion: after an exact-green command-surface
   repair, run at least one default-parallel scoped filter or otherwise explain
   why it is unsafe; for Windows tests that spawn bounded child commands with
   piped stdout, verify default JSON output stays compact enough not to block on
   the pipe buffer before process exit.

## 2026-06-12 - downstream resume stale recovery projection

Task / slice:
- `wave-0-runtime-tests-boot-smoke-failure-classification`
- Commit: `c0c1865c3 fix downstream resume stale recovery projection`
- Goal: let default `consume continue --json` resume from receipt-backed ready
  downstream packets even when the original TaskFlow task identity is missing,
  and keep recovery/status projection aligned with the resumed downstream node.

Proof:
- `cargo +1.95.0 fmt --all -- --check`
- `git diff --check -- crates/vida/src/taskflow_consume_resume.rs crates/vida/src/taskflow_run_graph.rs crates/vida/tests/boot_smoke.rs`
- `cargo +1.95.0 test -p vida --test boot_smoke taskflow_golden_route_happy_path_stitches_bootstrap_dispatch_resume_status_and_doctor -- --nocapture --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke status_and_doctor -- --nocapture --test-threads=1`
- `cargo +1.95.0 test -p vida --test boot_smoke diagnostics_status_and_doctor_share_closed_run_projection_blocker -- --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke taskflow_consume_continue_prefers_latest_final_snapshot_after_bundle_check -- --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke taskflow_consume_continue_resumes_from_persisted_final_snapshot -- --exact`
- `cargo +1.95.0 test -p vida --bin vida taskflow_consume_resume::tests::resolve_runtime_consumption_resume_inputs_without_run_id_switches_to_fresh_bound_task_run -- --nocapture --exact`

Residual blockers observed:
- Default-parallel `cargo +1.95.0 test -p vida --test boot_smoke status_and_doctor -- --nocapture`
  still timed out in `diagnostics_status_and_doctor_share_closed_run_projection_blocker`
  while running concurrently with the golden-route test; the same filter passes
  with `--test-threads=1`, and both exact tests pass.
- `cargo +1.95.0 test -p vida taskflow_run_graph -- --nocapture` remains blocked
  by pre-existing broad-filter failures: command-vocabulary assertions expecting
  `--json` suffixes and a stack overflow in
  `dispatch_init_reuses_existing_routed_receipt_packet`.

Observed model results:
- Executor: local orchestrator, 8/10. The fix required repeated exact
  reproduction and temporary payload prints; a separate classifier agent would
  likely have been slower than direct iteration.
- Validator: exact golden-route plus sequential scoped filter, 8/10. The default
  parallel filter remains a known Windows watchdog residual.

Post-Task Self-Analysis:
- Worked: temporary debug prints clarified that the first failure was generic
  stale-missing output, then recovery status `open_delegated_cycle`.
- Worked: splitting receipt-backed downstream evidence from full resume
  validation kept stale guards from rejecting lawful downstream packets too early.
- Waste: the first ready-downstream detector still called full validation, so it
  reproduced the same stale guard instead of bypassing it.
- Risk: recovery projection now treats ready routed receipts as resolving stale
  missing task identity; future tests should ensure this exemption does not hide
  genuinely missing packet/receipt evidence.
- Next change: continue classifying remaining broad boot_smoke failures under
  `wave-0-runtime-tests-boot-smoke-failure-classification`.
- Docs update: dynamic staged-gate criterion added below.
- workflow_score_10: 8/10.

Twenty criteria outcome:
1. Active bounded unit explicit: pass, golden-route downstream resume/recovery
   projection inside boot_smoke failure classification.
2. Wave/parent closure distance: pass, removed one status/doctor cluster failure;
   parent remains blocked by unrelated boot_smoke failures.
3. Scope and non-goals stable: pass, did not repair broad command-vocabulary or
   stack-overflow unit failures.
4. Dirty worktree handled: pass, unrelated dirty files remained unstaged.
5. Executor cheapest capable: pass, local orchestrator was sufficient.
6. Validator matched risk: pass, exact golden-route and adjacent resume tests ran.
7. Agent prompts: not applicable, no new agent launched.
8. Agent handles: pass, no active agent remained from this slice.
9. Telemetry: partial, command durations observed; cost unavailable.
10. Avoidable commands: pass, temporary prints were removed before commit.
11. Proof strength: pass for exact and sequential scoped surfaces.
12. Public-surface proof: pass, `consume continue`, recovery status, and
    status/doctor route were covered.
13. Debug build: pass, cargo rebuilt `vida`.
14. TaskFlow state: partial, classification child remains active.
15. Staging by invariant: pass, only two runtime files staged.
16. Publication authorization: pass, pushed to `main`.
17. Evaluation docs: pass, this scorecard.
18. Parent/wave metrics: unchanged until parent closure proof passes.
19. New defects/follow-ups: parallel status/doctor watchdog and broad
    taskflow_run_graph unit failures remain separate residuals.
20. Dynamic criteria generation: pass, session segment produced the staged-gate
    criterion below.

Final dynamic criteria STOP point:
1. Staged-gate criterion: when a failing test advances through multiple
   assertions after each fix, record each newly exposed assertion as a separate
   gate in the self-analysis; do not treat the first green sub-assertion as task
   closure until the full original test passes.

## 2026-06-12 - single-root JSONL import compatibility

Task / slice:
- `wave-0-runtime-tests-boot-smoke-failure-classification`
- Commit: `b3e4b0f11 normalize single-root task jsonl imports`
- Goal: restore legacy JSONL import compatibility for tests where one epic root
  and rootless open blocker tasks are imported together, while keeping live graph
  validation strict after import.

Proof:
- `cargo +1.95.0 fmt --all -- --check`
- `git diff --check -- crates/vida/src/state_store_task_store.rs crates/vida/tests/boot_smoke.rs`
- `cargo +1.95.0 test -p vida --test boot_smoke task_blocked_supports_compact_json_summary_view -- --nocapture --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke installed_vida_ready_filters_blocked_siblings_via_helper_backed_task_store -- --nocapture --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke installed_vida_ready_orders_multiple_rows_and_filters_blocked_siblings -- --nocapture --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke installed_vida_ready -- --nocapture`

Observed model results:
- Executor: local orchestrator, 8/10. The failure required seeing the import
  error payload, then making a narrow importer compatibility change.
- Validator: exact import/ready tests, 8/10. No broad boot_smoke rerun yet because
  the classification child still has several known clusters.

Post-Task Self-Analysis:
- Worked: temporary stderr/stdout print exposed the real import graph error
  instead of assuming JSON field alias drift.
- Worked: normalization is importer-only and only when the batch has exactly one
  root-like work item, preserving strict live graph validation.
- Waste: initial hypothesis focused on `type` vs `edge_type`, but the model had
  already supported both.
- Risk: auto-parenting legacy rows changes imported graph shape; metadata marks
  the edge source as `single_root_jsonl_import_compat`.
- Next change: continue with remaining boot_smoke clusters after refreshing the
  current failure list.
- Docs update: dynamic import-normalization criterion added below.
- workflow_score_10: 8/10.

Twenty criteria outcome:
1. Active bounded unit explicit: pass, JSONL import/ready list cluster.
2. Wave/parent closure distance: pass, three exact boot_smoke failures removed.
3. Scope and non-goals stable: pass, no task ready/list renderer rewrite.
4. Dirty worktree handled: pass, unrelated dirty files stayed unstaged.
5. Executor cheapest capable: pass, local code edit.
6. Validator matched risk: pass, exact import/ready and substring ready filter.
7. Agent prompts: not applicable.
8. Agent handles: pass, no active agent remained.
9. Telemetry: partial, command durations observed; cost unavailable.
10. Avoidable commands: pass, debug print was removed before commit.
11. Proof strength: pass for import compatibility.
12. Public-surface proof: pass, `vida task import-jsonl`, `task blocked`, and
    installed `vida task ready` paths covered.
13. Debug build: pass, cargo rebuilt `vida`.
14. TaskFlow state: partial, classification child remains active.
15. Staging by invariant: pass, only `state_store_task_store.rs` staged.
16. Publication authorization: pass, pushed to `main`.
17. Evaluation docs: pass, this scorecard.
18. Parent/wave metrics: unchanged until parent closure proof passes.
19. New defects/follow-ups: current broad failure list must be refreshed because
    `red-tests.md` contains already-fixed failures.
20. Dynamic criteria generation: pass, session segment produced the import
    normalization criterion below.

Final dynamic criteria STOP point:
1. Import-normalization criterion: before changing importer compatibility, first
   identify whether the failure is parse/schema aliasing, provider mapping,
   graph validation, or post-import rendering; normalize legacy data only in the
   narrow importer boundary and stamp generated edges/fields with source
   metadata.

## 2026-06-12 - TaskFlow help JSON contract compatibility

Task / slice:
- `wave-0-runtime-tests-boot-smoke-failure-classification`
- Commit: `55973a902 restore taskflow help json contracts`
- Goal: restore TaskFlow help compatibility for JSON-capable command examples
  without reverting the newer default operator guidance that prefers compact
  plain output.

Proof:
- `cargo +1.95.0 fmt --all -- --check`
- `git diff --check -- crates/vida/src/taskflow_layer4.rs`
- `cargo +1.95.0 test -p vida --test boot_smoke taskflow_proxy_help -- --nocapture`
- `cargo +1.95.0 test -p vida --test boot_smoke` after the slice: `243 passed`,
  `30 failed`; help failures were removed and remaining failures are non-help
  runtime clusters.

Observed model results:
- Executor: local orchestrator, 8/10. The change was a narrow help-surface
  compatibility patch after the current broad failure list showed six help
  assertions.
- Validator: focused `taskflow_proxy_help` boot_smoke filter, 9/10. It caught
  missed top-level examples and topic variants before the slice was committed.
- Residual: broad boot_smoke still has recovery/status/consume/run-graph clusters
  and at least one stack overflow; those are separate slices.

Post-Task Self-Analysis:
- Worked: refreshed current broad evidence before choosing the slice, avoiding
  stale `red-tests.md` failures.
- Worked: focused filter exposed the compatibility surface incrementally until
  all taskflow help variants were green.
- Waste: the first patch updated topic help but missed top-level help examples,
  causing two extra focused reruns.
- Risk: adding compatibility examples can create duplicate help lines. This is
  acceptable here because default command examples and machine-readable JSON
  examples serve different operator paths.
- Meta-analysis remediation: future help-contract fixes must inspect top-level
  family help, topic help, and command `--help` together before editing.
- Docs update: yes; this STOP record adds the dynamic help-contract criterion.
- workflow_score_10: 8/10.

Twenty criteria outcome:
1. Active bounded unit explicit: pass, help compatibility under
   `wave-0-runtime-tests-boot-smoke-failure-classification`.
2. Wave/parent closure distance: pass, broad boot_smoke improved to `243 passed`,
   `30 failed`, while the classification child remains open.
3. Scope and non-goals stable: pass, only `taskflow_layer4.rs` help strings.
4. Dirty worktree handled: pass, unrelated dirty Rust files and scratch files
   stayed unstaged.
5. Executor cheapest capable: pass, local edit was sufficient for help strings.
6. Validator matched risk: pass, focused help filter plus broad summary.
7. Agent prompts: not applicable, no delegated executor used.
8. Agent handles: pass, no active agents were launched or left open.
9. Telemetry: partial, command durations observed; tokens/cost not exposed.
10. Avoidable commands: partial, missed top-level examples caused reruns.
11. Proof strength: pass for help contract; broad suite remains red by other
    clusters.
12. Public-surface proof: pass, public `vida taskflow help` topic family covered.
13. Debug build: pass, cargo test rebuilt the binary.
14. TaskFlow state: partial, classification child remains active.
15. Staging by invariant: pass, only `taskflow_layer4.rs` was staged.
16. Publication authorization: pass, code commit pushed to `main`.
17. Evaluation docs: pass, this STOP gate is recorded before the next fix.
18. Parent/wave metrics: unchanged until all remaining broad failures are
    classified or fixed.
19. New defects/follow-ups: next slice should target recovery/status projection
    or consume-final routing, not help text.
20. Dynamic criteria generation: pass, session segment produced the
    help-contract surface-matrix criterion below.

Final dynamic criteria STOP point:
1. Help-contract surface-matrix criterion: when fixing CLI/help drift, inspect
   and prove all three surfaces together: top-level family help, topic help, and
   command `--help`. A green single topic is not enough if the broad failure came
   from family-level compatibility assertions.

## 2026-06-12 - Consume advance TOON expectation alignment

Task / slice:
- `wave-0-runtime-tests-boot-smoke-failure-classification`
- Commit: `c385179fc align consume advance toon expectation`
- Goal: align the consume-advance default TOON smoke assertion with the
  canonical operator-contract behavior that lowercases `next_actions`.

Proof:
- `cargo +1.95.0 fmt --all -- --check`
- `git diff --check -- crates/vida/tests/boot_smoke.rs`
- `cargo +1.95.0 test -p vida --test boot_smoke taskflow_consume_advance_default_output_is_compact_toon_when_blocked -- --nocapture --exact`
- `cargo +1.95.0 test -p vida operator_contracts::tests:: -- --nocapture`
- `cargo +1.95.0 test -p vida --test boot_smoke` after the slice: `247 passed`,
  `26 failed`.

Observed model results:
- Executor: local orchestrator, 8/10. The failure looked like a missing action
  string, but source review showed `canonical_next_action_entries` intentionally
  lowercases actions and its unit tests enforce that contract.
- Validator: exact smoke plus `operator_contracts::tests::`, 9/10. The proof
  tied the public assertion to the lower-level contract instead of weakening the
  check blindly.
- Residual: broad boot_smoke still fails in status/recovery/consume-final,
  protocol-binding, run-graph, and stack-overflow clusters.

Post-Task Self-Analysis:
- Worked: checked contract owner before changing product renderer.
- Worked: exact smoke and contract unit proof made the test-only change
  defensible.
- Waste: one manual reproduction was flawed because it did not run inside the
  bootstrapped project root, but it still exposed the lowercased output shape.
- Risk: test-only fixes can hide regressions; this one is acceptable because the
  expected case now matches an explicit unit-tested contract.
- Meta-analysis remediation: when a public smoke assertion conflicts with a
  contract unit, classify whether the smoke test is stale before editing product
  code.
- Docs update: yes; this STOP record adds the smoke-vs-contract criterion.
- workflow_score_10: 8/10.

Twenty criteria outcome:
1. Active bounded unit explicit: pass, consume-advance TOON assertion under
   `wave-0-runtime-tests-boot-smoke-failure-classification`.
2. Wave/parent closure distance: pass, broad boot_smoke improved to `247 passed`,
   `26 failed`.
3. Scope and non-goals stable: pass, one assertion only.
4. Dirty worktree handled: pass, unrelated dirty files stayed unstaged.
5. Executor cheapest capable: pass, local test correction.
6. Validator matched risk: pass, exact smoke plus operator contract unit family.
7. Agent prompts: not applicable.
8. Agent handles: pass, no active agents were launched.
9. Telemetry: partial, durations observed; tokens/cost unavailable.
10. Avoidable commands: partial, manual reproduction missed `current_dir`.
11. Proof strength: pass for expectation alignment.
12. Public-surface proof: pass, public consume-advance smoke covered.
13. Debug build: pass, cargo test rebuilt the binary.
14. TaskFlow state: partial, classification child remains active.
15. Staging by invariant: pass, only `boot_smoke.rs` staged.
16. Publication authorization: pass, pushed to `main`.
17. Evaluation docs: pass, this STOP gate is recorded before next fix.
18. Parent/wave metrics: unchanged until broad classification completes.
19. New defects/follow-ups: next slice should target high-count status/recovery
    and run-graph failures.
20. Dynamic criteria generation: pass, session segment produced the
    smoke-vs-contract criterion below.

Final dynamic criteria STOP point:
1. Smoke-vs-contract criterion: before changing production behavior for a public
   smoke failure, search for a lower-level contract/unit test that defines the
   same invariant. If the lower-level contract is explicit and green, update the
   stale smoke expectation and prove both levels together.

## 2026-06-12 - Protocol-binding refresh smoke alignment

Task / slice:
- `wave-0-runtime-tests-boot-smoke-failure-classification`
- Commit: `deeae399e align protocol binding refresh smoke tests`
- Goal: update protocol-binding smoke tests from stale fail-closed expectations
  to the current self-healing command behavior, while preserving lower-level
  fail-closed coverage for untrusted evidence.

Proof:
- `cargo +1.95.0 fmt --all -- --check`
- `git diff --check -- crates/vida/tests/boot_smoke.rs`
- `cargo +1.95.0 test -p vida --test boot_smoke taskflow_protocol_binding_check_refreshes -- --nocapture`
- `cargo +1.95.0 test -p vida taskflow_protocol_binding::tests::protocol_binding_check_ok_blocks -- --nocapture`
- `cargo +1.95.0 test -p vida --test boot_smoke` after the slice: `246 passed`,
  `27 failed`; protocol-binding failures were removed, while status/recovery
  concurrency and run-graph clusters remain.

Observed model results:
- Executor: local orchestrator, 8/10. The first hypothesis was too narrow
  because the pure checker still fail-closes, but command routing refreshes
  launcher activation evidence before the check sees it.
- Validator: exact refresh smoke plus pure checker fail-closed units, 9/10.
- Residual: broad status tests are nondeterministic under full parallel
  boot_smoke and can change the total failure count run to run.

Post-Task Self-Analysis:
- Worked: temporary assertion output exposed the self-healing JSON payload and
  showed fresh `captured_at` evidence.
- Worked: helper read-back assertion now proves test fixture overwrites really
  hit the intended state-store row before command-level refresh replaces them.
- Waste: initial reasoning spent time on helper path/digest mismatch before the
  command-level refresh behavior was visible.
- Risk: renaming fail-closed tests to refresh tests could lose blocker coverage;
  mitigated by running the pure `protocol_binding_check_ok_blocks_*` units.
- Meta-analysis remediation: classify command-level self-healing separately from
  pure decision-gate fail-closed behavior.
- Docs update: yes; this STOP record adds the self-healing command criterion.
- workflow_score_10: 8/10.

Twenty criteria outcome:
1. Active bounded unit explicit: pass, protocol-binding smoke cluster under
   `wave-0-runtime-tests-boot-smoke-failure-classification`.
2. Wave/parent closure distance: partial, two protocol-binding broad failures
   removed, but full-suite nondeterminism reported `246 passed`, `27 failed`.
3. Scope and non-goals stable: pass, smoke tests and helper verification only.
4. Dirty worktree handled: pass, unrelated dirty files stayed unstaged.
5. Executor cheapest capable: pass, local test-contract update.
6. Validator matched risk: pass, command-level refresh tests plus pure unit
   fail-closed tests.
7. Agent prompts: not applicable.
8. Agent handles: pass, no active agents were launched.
9. Telemetry: partial, durations observed; tokens/cost unavailable.
10. Avoidable commands: partial, diagnostic rerun was needed to expose JSON.
11. Proof strength: pass for protocol-binding refresh behavior.
12. Public-surface proof: pass, public `taskflow protocol-binding check --json`
    covered.
13. Debug build: pass, cargo tests rebuilt the binary.
14. TaskFlow state: partial, classification child remains active.
15. Staging by invariant: pass, only `boot_smoke.rs` staged.
16. Publication authorization: pass, pushed to `main`.
17. Evaluation docs: pass, this STOP gate is recorded before next fix.
18. Parent/wave metrics: unchanged until broad classification closes.
19. New defects/follow-ups: status-surface full-suite nondeterminism remains a
    likely next target.
20. Dynamic criteria generation: pass, session segment produced the
    self-healing command criterion below.

Final dynamic criteria STOP point:
1. Self-healing command criterion: when a public command now repairs stale state
   before evaluating a lower-level gate, update smoke tests to prove repair at
   the command level and keep separate unit coverage for the pure fail-closed
   decision gate.

## 2026-06-12 - Status surface smoke harness stabilization

Task / slice:
- `wave-0-runtime-tests-boot-smoke-failure-classification`
- Commit: `b3fbdfb53 stabilize status surface smoke harness`
- Goal: remove full-suite status-surface harness flakes caused by parallel
  status tests and the Windows bounded runner deadlocking on large JSON output.

Proof:
- `cargo +1.95.0 fmt --all -- --check`
- `git diff --check -- crates/vida/tests/boot_smoke.rs`
- `cargo +1.95.0 test -p vida --test boot_smoke status_surface_ -- --nocapture`
- `cargo +1.95.0 test -p vida --test boot_smoke` after the slice: `250 passed`,
  `23 failed`.

Observed model results:
- Executor: local orchestrator, 8/10. The first retry-wrapper attempt exposed a
  Windows pipe-buffer issue: bounded process waiting without stdout draining can
  time out on large JSON even when the command produced valid output.
- Validator: focused status-surface filter plus broad boot_smoke snapshot, 8/10.
- Residual: `diagnostics_status_and_doctor_share_closed_run_projection_blocker`
  still has a separate status JSON parse/view failure path and should be handled
  independently.

Post-Task Self-Analysis:
- Worked: exact `status_surface_` filter proved the status product behavior was
  already green when isolated.
- Worked: mutex serialization plus `command_output_with_retry` removed status
  cluster flakes without changing production code.
- Waste: an initial broad patch accidentally changed unrelated helpers; this was
  caught in diff review and reverted before commit.
- Risk: serializing tests can hide true concurrency bugs. This is acceptable here
  because the serialized tests validate status rendering, while the suite already
  has an explicit parallel read-only state-lock contention test.
- Meta-analysis remediation: large JSON command tests on Windows must avoid
  custom wait loops that do not drain stdout/stderr.
- Docs update: yes; this STOP record adds the Windows pipe-drain criterion.
- workflow_score_10: 8/10.

Twenty criteria outcome:
1. Active bounded unit explicit: pass, status-surface harness cluster under
   `wave-0-runtime-tests-boot-smoke-failure-classification`.
2. Wave/parent closure distance: pass, broad boot_smoke reached `250 passed`,
   `23 failed`.
3. Scope and non-goals stable: pass, test harness only; production status code
   unchanged.
4. Dirty worktree handled: pass, unrelated dirty files stayed unstaged.
5. Executor cheapest capable: pass, local harness edit.
6. Validator matched risk: pass, focused filter plus broad snapshot.
7. Agent prompts: not applicable.
8. Agent handles: pass, no active agents launched.
9. Telemetry: partial, durations observed; tokens/cost unavailable.
10. Avoidable commands: partial, one accidental broad patch was reverted.
11. Proof strength: pass for status-surface harness stability.
12. Public-surface proof: pass, status default, color, full JSON, and summary JSON
    smoke tests passed.
13. Debug build: pass, cargo test rebuilt the binary.
14. TaskFlow state: partial, classification child remains active.
15. Staging by invariant: pass, only `boot_smoke.rs` staged.
16. Publication authorization: pass, pushed to `main`.
17. Evaluation docs: pass, this STOP gate is recorded before next fix.
18. Parent/wave metrics: unchanged until broad classification closes.
19. New defects/follow-ups: diagnostics/status bounded parse path remains a
    separate next target.
20. Dynamic criteria generation: pass, session segment produced the Windows
    pipe-drain criterion below.

Final dynamic criteria STOP point:
1. Windows pipe-drain criterion: do not run large JSON-producing commands through
   custom bounded wait loops that do not drain stdout/stderr. Use
   `Command::output()`-based retry helpers or a pipe-draining bounded runner, and
   serialize only the minimal shared-state test cluster when parallel harness
   contention is the failure source.

## 2026-06-12 - Diagnostics status/doctor pipe-drain smoke fix

Task / slice:
- `wave-0-runtime-tests-boot-smoke-failure-classification`
- Commit: `471032fd4 drain diagnostics status doctor smoke output`
- Goal: remove the EOF/empty-output parse failure in
  `diagnostics_status_and_doctor_share_closed_run_projection_blocker` by running
  status/doctor JSON commands through a pipe-draining retry helper.

Proof:
- `cargo +1.95.0 fmt --all -- --check`
- `git diff --check -- crates/vida/tests/boot_smoke.rs`
- `cargo +1.95.0 test -p vida --test boot_smoke diagnostics_status_and_doctor_share_closed_run_projection_blocker -- --nocapture --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke` after the slice: `246 passed`,
  `27 failed`; the exact diagnostics pipe path is green, while other
  status/doctor and run-graph clusters remain.

Observed model results:
- Executor: local orchestrator, 8/10. The slice reused the status harness
  finding that custom bounded waits can produce EOF/timeout on JSON output.
- Validator: exact diagnostics/status/doctor smoke, 8/10. Broad boot_smoke still
  shows separate doctor/status harness failures and runtime graph failures.

Post-Task Self-Analysis:
- Worked: applied the previous pipe-drain lesson immediately to the next failing
  status/doctor JSON path.
- Worked: kept the slice narrow to one exact diagnostics test rather than
  bundling all remaining doctor/status failures.
- Waste: broad rerun after the slice was noisy and did not improve total count in
  that sample, but it confirmed the next residual cluster.
- Risk: exact pass does not prove all status/doctor full-suite flakes are gone;
  next slice should audit every remaining doctor/status JSON command runner.
- Meta-analysis remediation: promote pipe-drain audit from one test to the full
  status/doctor smoke family before deeper runtime fixes.
- Docs update: yes; this STOP record adds the family-wide pipe-drain audit rule.
- workflow_score_10: 8/10.

Twenty criteria outcome:
1. Active bounded unit explicit: pass, diagnostics status/doctor JSON path under
   `wave-0-runtime-tests-boot-smoke-failure-classification`.
2. Wave/parent closure distance: partial, exact diagnostics test fixed; broad
   sample remains `246 passed`, `27 failed`.
3. Scope and non-goals stable: pass, one smoke test command runner path.
4. Dirty worktree handled: pass, unrelated dirty files stayed unstaged.
5. Executor cheapest capable: pass, local harness edit.
6. Validator matched risk: pass, exact diagnostics smoke plus broad sample.
7. Agent prompts: not applicable.
8. Agent handles: pass, no active agents launched.
9. Telemetry: partial, durations observed; tokens/cost unavailable.
10. Avoidable commands: pass, reused prior finding instead of re-diagnosing from
    scratch.
11. Proof strength: pass for exact diagnostics pipe path.
12. Public-surface proof: pass, public `status --json` and `doctor --json` were
    exercised through the diagnostics smoke.
13. Debug build: pass, cargo tests rebuilt the binary.
14. TaskFlow state: partial, classification child remains active.
15. Staging by invariant: pass, only `boot_smoke.rs` staged.
16. Publication authorization: pass, pushed to `main`.
17. Evaluation docs: pass, this STOP gate is recorded before next fix.
18. Parent/wave metrics: unchanged until broad classification closes.
19. New defects/follow-ups: remaining doctor/status smoke tests should be audited
    for the same bounded-runner pipe issue.
20. Dynamic criteria generation: pass, session segment produced the family-wide
    pipe-drain audit criterion below.

Final dynamic criteria STOP point:
1. Status/doctor family pipe-drain criterion: after fixing one large JSON command
   in a smoke test, search adjacent status/doctor diagnostics for the same custom
   bounded runner and migrate them as a family before treating remaining failures
   as product-runtime defects.

## 2026-06-12 - Shared status/doctor smoke helper pipe-drain migration

Task / slice:
- `wave-0-runtime-tests-boot-smoke-failure-classification`
- Commit: `d0c1a0a65 drain status doctor shared smoke helpers`
- Goal: migrate shared `status_with_timeout` and `doctor_with_timeout` smoke
  helpers away from bounded wait loops that do not drain large JSON output.

Proof:
- `cargo +1.95.0 fmt --all -- --check`
- `git diff --check -- crates/vida/tests/boot_smoke.rs`
- `cargo +1.95.0 test -p vida --test boot_smoke status_and_doctor -- --nocapture`
- `cargo +1.95.0 test -p vida --test boot_smoke doctor_surface_ -- --nocapture`
- `cargo +1.95.0 test -p vida --test boot_smoke status_json_exposes_host_agent_summary -- --nocapture --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke` after the slice: `250 passed`,
  `23 failed`.

Observed model results:
- Executor: local orchestrator, 8/10. The dynamic rule from the previous slice
  correctly identified the shared helper family.
- Validator: focused status/doctor filters, 9/10. Broad run still reports a
  `diagnostics_status...` default-view mismatch under full-suite concurrency,
  which is now separate from pipe draining.

Post-Task Self-Analysis:
- Worked: family audit found the shared helper rather than only individual test
  callsites.
- Worked: focused filters confirmed status/doctor surfaces no longer fail through
  helper pipe buffering.
- Waste: broad run still includes the diagnostics test as failed because its
  status default-view behavior differs under full-suite conditions.
- Risk: a broad count can stay flat even when a real family fix landed; closure
  evidence must record focused pass and residual cluster separately.
- Meta-analysis remediation: after a family harness fix, immediately classify the
  next residual as new semantic mismatch vs same harness class.
- Docs update: yes; this STOP record adds the flat-broad-count classification
  criterion.
- workflow_score_10: 8/10.

Twenty criteria outcome:
1. Active bounded unit explicit: pass, shared status/doctor helper family under
   `wave-0-runtime-tests-boot-smoke-failure-classification`.
2. Wave/parent closure distance: partial, focused family green; broad stayed
   `250 passed`, `23 failed`.
3. Scope and non-goals stable: pass, helper migration only.
4. Dirty worktree handled: pass, unrelated dirty files stayed unstaged.
5. Executor cheapest capable: pass, local harness edit.
6. Validator matched risk: pass, three focused status/doctor filters plus broad
   sample.
7. Agent prompts: not applicable.
8. Agent handles: pass, no active agents launched.
9. Telemetry: partial, durations observed; tokens/cost unavailable.
10. Avoidable commands: pass, followed the dynamic family audit criterion.
11. Proof strength: pass for status/doctor helper pipe-drain behavior.
12. Public-surface proof: pass, status and doctor public smoke filters covered.
13. Debug build: pass, cargo tests rebuilt the binary.
14. TaskFlow state: partial, classification child remains active.
15. Staging by invariant: pass, only `boot_smoke.rs` staged.
16. Publication authorization: pass, pushed to `main`.
17. Evaluation docs: pass, this STOP gate is recorded before next fix.
18. Parent/wave metrics: unchanged until broad classification closes.
19. New defects/follow-ups: remaining diagnostics default-view mismatch should be
    treated as a new semantic/concurrency defect.
20. Dynamic criteria generation: pass, session segment produced the
    flat-broad-count criterion below.

Final dynamic criteria STOP point:
1. Flat-broad-count criterion: if focused family proof turns green but broad
   failure count stays flat, do not assume the fix failed. Compare failure
   identities and classify the next red item as same harness class, new semantic
   mismatch, or unrelated runtime cluster before selecting the next slice.
2. Operator-correction dynamic-gate criterion: if the operator points out that
   dynamic criteria were described as optional or merely part of the fixed list,
   update the owner instructions before continuing. Evidence: the next STOP
   record must name the final dynamic criteria STOP point separately from the 20
   fixed baseline criteria.

## 2026-06-12 - Status smoke state-lock retry stabilization

Task / slice:
- `wave-0-runtime-tests-boot-smoke-failure-classification`
- Commit: `717279777 stabilize status smoke lock retries`
- Goal: make status/doctor/status-surface smoke tests retry deterministic
  state-lock degraded payloads without reintroducing the Windows bounded-output
  pipe wait problem.

Proof:
- `cargo +1.95.0 fmt --all -- --check`
- `git diff --check -- crates/vida/tests/boot_smoke.rs`
- `cargo +1.95.0 test -p vida --test boot_smoke diagnostics_status_and_doctor_share_closed_run_projection_blocker -- --nocapture --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke status_and_doctor -- --nocapture`
- `cargo +1.95.0 test -p vida --test boot_smoke doctor_surface_ -- --nocapture`
- `cargo +1.95.0 test -p vida --test boot_smoke status_surface_ -- --nocapture`
- `cargo +1.95.0 test -p vida --test boot_smoke` after the slice: `251 passed`,
  `22 failed`.

Observed model results:
- Executor: local orchestrator, 8/10. The slice was narrow after broad evidence
  showed `state_store_read_lock_contention` in stdout rather than stderr.
- Validator: focused status/doctor/status-surface filters plus broad snapshot,
  9/10. Broad failure identity improved by one and the status lock class left
  the failure list.

Post-Task Self-Analysis:
- Worked: reading broad failure stdout revealed that deterministic degraded lock
  payloads can carry the lock error only in stdout.
- Worked: separating `command_output_with_state_lock_retry` from the generic
  retry helper preserved tests that intentionally assert deterministic lock
  fail-closed output.
- Waste: the first helper change retried only stderr-detected locks and required
  a second broad run to expose the stdout-only lock payload.
- Risk: broad failure identity can migrate between adjacent status tests until
  the shared detection predicate covers stdout and stderr.
- Meta-analysis remediation: update shared harness detection rather than adding
  per-test sleeps or widening command timeouts.
- Docs update: yes; this STOP record adds the stdout+stderr lock detection
  criterion below.
- workflow_score_10: 8/10.

Twenty criteria outcome:
1. Active bounded unit explicit: pass,
   `wave-0-runtime-tests-boot-smoke-failure-classification`.
2. Wave/parent closure distance: partial, broad improved from `250/23` to
   `251/22` but the classification child remains open.
3. Scope and non-goals stable: pass, harness retry only.
4. Dirty worktree handled: pass, unrelated Rust and untracked files stayed
   unstaged.
5. Executor cheapest capable: pass, local fix from exact broad evidence.
6. Validator matched risk: pass, focused families plus broad snapshot.
7. Agent prompts: not applicable; no new subagents launched for this local
   critical-path harness fix.
8. Agent handles: pass, no active handles used.
9. Telemetry: partial, exact token/cost unavailable; proof duration and broad
   counts recorded.
10. Avoidable commands: partial, one broad rerun was needed because the first
    predicate only inspected stderr.
11. Proof strength: pass for the claimed status lock-retry class.
12. Public-surface proof: pass, status and doctor CLI smoke tests covered default,
    JSON, and render paths in the affected family.
13. Debug build: pass, cargo tests rebuilt `vida`.
14. TaskFlow state: partial, classification task remains active.
15. Staging by invariant: pass, only `boot_smoke.rs` was staged.
16. Publication authorization: pass, pushed to `main`.
17. Evaluation docs: pass, this STOP gate is recorded before selecting the next
    runtime slice.
18. Parent/wave metrics: broad status is now `251 passed`, `22 failed`; wave
    closure still blocked.
19. New defects/follow-ups: remaining failures now cluster around stale
    missing-task run graph, consume-final routing, run-graph recovery, stack
    overflow, and child-dependency projection.
20. Next routing rule: pass, continue with the first non-lock residual cluster;
    do not keep spending slices on status/doctor lock retries unless a new
    failure identity reintroduces that class.

Final dynamic criteria STOP point:
1. Stdout+stderr lock-detection criterion: when a broad-only smoke failure shows
   a degraded lock payload, inspect both stdout and stderr before changing retry
   semantics. Evidence: the next harness retry change must cite the exact stream
   where `state_store_read_lock_contention` or equivalent lock text appeared.
2. Broad-migration criterion: when a broad run improves count by one but another
   adjacent status test briefly fails, require a second broad snapshot or focused
   family proof before deciding the same class remains open.

## 2026-06-12 - Agent-init view-only stale-run blocker alignment

Task / slice:
- `wave-0-runtime-tests-boot-smoke-failure-classification`
- Commit: `40318a832 align agent init view-only smoke blocker`
- Goal: align the agent-init view-only activation smoke assertion with the
  current canonical fail-closed stale missing-task run graph blocker.

Proof:
- `cargo +1.95.0 fmt --all -- --check`
- `git diff --check -- crates/vida/tests/boot_smoke.rs`
- `cargo +1.95.0 test -p vida --test boot_smoke agent_init_dispatch_packet_reports_view_only_activation_semantics -- --nocapture --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke` after the slice: variable broad
  snapshot, latest observed `249 passed`, `24 failed`, with status lock
  contention resurfacing in unrelated callsites.

Observed model results:
- Executor: local orchestrator, 8/10. The exact failing assertion was stale and
  the current runtime blocker text was already canonical and actionable.
- Validator: exact public smoke test, 8/10. Broad proof was useful for residual
  classification but too noisy to use as the only acceptance signal for this
  narrow assertion update.

Post-Task Self-Analysis:
- Worked: exact reproduction showed the `agent-init` activation-view fields were
  already correct and only the follow-up `consume continue --json` assertion was
  stale.
- Waste: broad rerun after the exact fix reintroduced unrelated lock-contention
  failures and did not isolate this contract change.
- Risk: using broad count alone would incorrectly reject a valid exact contract
  update or send the next slice back into a previously improved lock class.
- Meta-analysis remediation: treat exact contract proof as acceptance for this
  slice, while recording broad variance as next-slice selection evidence.
- Docs update: yes; this STOP record adds the exact-vs-broad acceptance
  criterion below.
- workflow_score_10: 8/10.

Twenty criteria outcome:
1. Active bounded unit explicit: pass,
   `wave-0-runtime-tests-boot-smoke-failure-classification`.
2. Wave/parent closure distance: partial, exact red turned green; broad remains
   red and variable.
3. Scope and non-goals stable: pass, one assertion contract update.
4. Dirty worktree handled: pass, unrelated dirty files stayed unstaged.
5. Executor cheapest capable: pass, local assertion update from exact evidence.
6. Validator matched risk: pass for exact test; broad used as residual evidence.
7. Agent prompts: not applicable.
8. Agent handles: pass, no new handles used.
9. Telemetry: partial, token/cost unavailable; proof commands and broad count
   recorded.
10. Avoidable commands: partial, broad run was noisy but helped classify next
    residual risk.
11. Proof strength: pass for the exact stale assertion behavior.
12. Public-surface proof: pass, `agent-init` plus `taskflow consume continue`
    public smoke path covered.
13. Debug build: pass, cargo tests rebuilt `vida`.
14. TaskFlow state: partial, classification task remains active.
15. Staging by invariant: pass, only `boot_smoke.rs` staged.
16. Publication authorization: pass, pushed to `main`.
17. Evaluation docs: pass, STOP gate recorded before next runtime slice.
18. Parent/wave metrics: broad latest observed `249 passed`, `24 failed`;
    previous best after lock retry was `251 passed`, `22 failed`.
19. New defects/follow-ups: broad variance shows lock retry coverage is not
    complete across all status callsites, but the next deterministic exact red
    cluster is consume/run-graph recovery.
20. Next routing rule: pass, prefer exact reproducible failing test updates over
    chasing broad-only count noise unless the same broad failure identity becomes
    deterministic.

Final dynamic criteria STOP point:
1. Exact-vs-broad acceptance criterion: if a narrow assertion update fixes the
   exact failing test and broad moves for unrelated identities, accept the exact
   slice only when the broad residual is recorded with concrete failure names.
2. Variance guard criterion: do not use a single broad snapshot as closure
   evidence for noisy boot_smoke clusters; require either two consistent broad
   snapshots or a focused exact proof for the claimed behavior.

## 2026-06-12 - Consume-continue packet mismatch blocker classification

Task / slice:
- `wave-0-runtime-tests-boot-smoke-failure-classification`
- Commit: `ad92f52f5 classify consume continue packet mismatch blocker`
- Goal: classify persisted dispatch receipt packet-path mismatch as
  `consume_continue_resume_blocked` instead of generic
  `run_graph_recovery_not_ready`.

Proof:
- `cargo +1.95.0 fmt --all -- --check`
- `git diff --check -- crates/vida/src/taskflow_operator_diagnostics.rs crates/vida/tests/boot_smoke.rs`
- `cargo +1.95.0 test -p vida persisted_dispatch_packet_path_mismatch_is_consume_continue_resume_blocked -- --nocapture`
- `cargo +1.95.0 test -p vida --test boot_smoke taskflow_consume_continue_rejects_explicit_downstream_packet_without_receipt_authority -- --nocapture --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke consume_continue_repeated_run_id_after_success_fails_closed_without_closure_projection -- --nocapture --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke consume_continue -- --nocapture`
- `cargo +1.95.0 test -p vida --test boot_smoke` before sibling assertion update:
  `251 passed`, `22 failed`.

Observed model results:
- Executor: local orchestrator, 8/10. The error text already identified receipt
  path mismatch; production classifier was the right owner.
- Validator: classifier unit, two exact smokes, and 15-test consume-continue
  family, 9/10.

Post-Task Self-Analysis:
- Worked: resisted weakening the smoke test and fixed the diagnostic classifier
  so the operator contract exposes the more specific blocker code.
- Worked: broad snapshot exposed a sibling assertion that still expected the old
  generic blocker; fixing it in the same slice kept the classifier invariant
  consistent.
- Waste: one cargo command tried to pass two positional test filters and failed
  before any useful proof.
- Risk: changing a shared diagnostic classifier can break adjacent tests that
  assert exact blocker-code arrays.
- Meta-analysis remediation: after classifier changes, run a family filter, not
  just the first exact red test.
- Docs update: yes; this STOP record adds the sibling-assertion criterion below.
- workflow_score_10: 8/10.

Twenty criteria outcome:
1. Active bounded unit explicit: pass,
   `wave-0-runtime-tests-boot-smoke-failure-classification`.
2. Wave/parent closure distance: partial, consume-continue exact and family
   cluster green; broad still blocked.
3. Scope and non-goals stable: pass, one diagnostic classifier plus two smoke
   expectations.
4. Dirty worktree handled: pass, unrelated dirty files stayed unstaged.
5. Executor cheapest capable: pass, local classifier fix from concrete exact
   payload.
6. Validator matched risk: pass, unit plus public smoke family.
7. Agent prompts: not applicable.
8. Agent handles: pass, none used.
9. Telemetry: partial, token/cost unavailable; command proof recorded.
10. Avoidable commands: partial, one invalid multi-filter cargo command.
11. Proof strength: pass, unit and 15-test consume-continue family covered the
    shared classifier invariant.
12. Public-surface proof: pass, `taskflow consume continue` JSON surfaces covered.
13. Debug build: pass, cargo tests rebuilt `vida`.
14. TaskFlow state: partial, classification task remains active.
15. Staging by invariant: pass, classifier and directly affected smoke assertions
    staged together.
16. Publication authorization: pass, pushed to `main`.
17. Evaluation docs: pass, STOP gate recorded before next slice.
18. Parent/wave metrics: latest broad evidence remains red; previous broad around
    this slice was `251 passed`, `22 failed`.
19. New defects/follow-ups: remaining deterministic exact clusters include
    consume-final role routing, run-graph recovery/status projection, stack
    overflow, and child-dependency projection.
20. Next routing rule: pass, when a classifier change affects a blocker code,
    immediately search or filter for sibling assertions before committing.

Final dynamic criteria STOP point:
1. Classifier-sibling criterion: any change to diagnostic kind or blocker-code
   mapping must run the focused public-surface family filter and update every
   sibling assertion that names the old blocker.
2. Cargo-filter criterion: do not pass multiple positional test names to one
   `cargo test` command. Use one exact filter per command or a shared substring
   family filter.

## 2026-06-12 - Consume-final conversational activation fields

Task / slice:
- `wave-0-runtime-tests-boot-smoke-failure-classification`
- Commit: `52dea6363 restore consume final conversational activation fields`
- Goal: keep `taskflow consume final` conversational dispatch receipts from
  dropping activation fields when the selected route stores them under
  `runtime_assignment` instead of `default_route`.

Proof:
- `cargo +1.95.0 fmt --all -- --check`
- `git diff --check -- crates/vida/src/taskflow_consume.rs crates/vida/tests/boot_smoke.rs`
- `cargo +1.95.0 test -p vida conversational_dispatch_receipt_uses_runtime_assignment_activation_fallback -- --nocapture`
- `cargo +1.95.0 test -p vida --test boot_smoke taskflow_consume_final_selects_scope_discussion_role_for_spec_queries -- --nocapture --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke taskflow_consume_final_selects_pbi_discussion_role_for_backlog_queries -- --nocapture --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke taskflow_consume_final_selects_ -- --nocapture`
- Broad evidence after the runtime fallback and before the sibling assertion
  update: `252 passed`, `21 failed`; the target exact passed and the remaining
  failures stayed in adjacent consume-final, run-graph/status, stack-overflow,
  and projection clusters.

Observed model results:
- Executor: local orchestrator, 8/10. The failing JSON surface pointed directly
  at a missing `dispatch_receipt.activation_agent_type`; the correct fix was a
  small production fallback plus focused smoke assertions.
- Validator: local unit plus two exact public smokes and a two-test family
  filter, 9/10. No separate model validator was needed for this bounded receipt
  invariant.

Post-Task Self-Analysis:
- Worked: the dynamic criterion from the previous task caught the sibling PBI
  exact before commit, so the activation fallback invariant was fixed across
  both conversational modes in the same slice.
- Waste: the first unit proof needed formatting correction, and the broad run
  was used only as residual classification rather than acceptance.
- Risk: a pre-commit hook temporarily stashed unrelated dirty files; staging by
  explicit paths kept those hunks out of the commit, but this should remain a
  named post-commit check.
- Next change: after any fallback that populates previously-null receipt fields,
  run a family filter for adjacent assertions that intentionally expected null.
- Docs update: yes; the scorecard template now states that the dynamic criteria
  step is the final checklist item and must create session-derived criteria.
- workflow_score_10: 8/10.

Twenty criteria outcome:
1. Active bounded unit explicit: pass,
   `wave-0-runtime-tests-boot-smoke-failure-classification`.
2. Wave/parent closure distance: partial, two consume-final exacts improved; the
   parent broad boot-smoke task remains red.
3. Scope and non-goals stable: pass, only conversational receipt activation
   fields and directly affected smoke assertions changed.
4. Dirty worktree handled: pass, unrelated dirty files remained unstaged through
   the pre-commit stash/restore cycle.
5. Executor cheapest capable: pass, local orchestrator was sufficient for the
   small shared fallback.
6. Validator matched risk: pass, unit plus public smoke exacts and family filter.
7. Agent prompts: not applicable, no delegated executor prompt.
8. Agent handles: pass, no new handles used.
9. Telemetry: partial, token/cost unavailable; command proof recorded.
10. Avoidable commands: partial, broad run was noisy but useful for detecting the
    sibling stale null assertion.
11. Proof strength: pass, production unit and both public conversational modes
    covered the claimed behavior.
12. Public-surface proof: pass, `taskflow consume final` JSON public smoke paths
    covered.
13. Debug build: pass, cargo tests rebuilt `vida`.
14. TaskFlow state: partial, classification task remains active.
15. Staging by invariant: pass, only `taskflow_consume.rs` and the relevant
    `boot_smoke.rs` assertions were staged.
16. Publication authorization: pass, pushed to `main`.
17. Evaluation docs: pass, this STOP record is being written before the next
    runtime slice.
18. Parent/wave metrics: latest broad evidence after the fallback was
    `252 passed`, `21 failed`; wave remains open.
19. New defects/follow-ups: remaining deterministic adjacent failures include
    mixed feature delivery routing, plain bootstrap spec delegation text,
    run-graph/status recovery projection, and stack overflow.
20. Next routing rule: pass, route the next consume-final work by adjacent
    invariant family rather than by the first broad failure line only.

Final dynamic criteria STOP point:
1. Sibling-null-expectation criterion: when production starts populating a
   receipt field that was previously null, search or family-filter sibling tests
   for `.is_null()` expectations on that field before committing.
2. Pre-commit-stash criterion: after any commit hook reports unstaged-file
   stashing, immediately run `git status --short` and verify no unrelated hunk
   was staged, dropped, or accidentally normalized.
3. Final-dynamic-step criterion: the last STOP checklist item must create new
   criteria from the session segment since the previous task; do not treat
   already-written fixed criteria or prior dynamic criteria as satisfying this
   task's dynamic creation step.

## 2026-06-12 - Consume-final configured lane labels

Task / slice:
- `wave-0-runtime-tests-boot-smoke-failure-classification`
- Commit: `18c495abf label configured consume final lanes logically`
- Goal: make design-first consume-final orchestration surfaces label configured
  dev-team carrier ids as logical lane steps in JSON `active_cycle` and human
  TOON `delegated_lanes`.

Proof:
- `cargo +1.95.0 fmt --all -- --check`
- `git diff --check -- crates/vida/src/development_flow_orchestration.rs crates/vida/src/taskflow_consume.rs`
- `cargo +1.95.0 test -p vida dispatch_contract_uses_configured_dev_team_flow_lane_sequence -- --nocapture`
- `cargo +1.95.0 test -p vida --test boot_smoke taskflow_consume_final_routes_mixed_feature_delivery_requests_to_spec_first -- --nocapture --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke taskflow_consume_final_plain_prefers_bootstrap_spec_over_manual_design_steps -- --nocapture --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke taskflow_consume_final_ -- --nocapture`: `17 passed`, `1 failed`; residual failure remained
  `taskflow_consume_final_blocks_downstream_closure_when_docflow_verdict_blocks`
  with `left: String("senior") right: Null`.

Observed model results:
- Executor: local orchestrator, 8/10. Manual CLI replay showed carrier ids in
  `execution_lane_sequence` and generic `delegate_lane` labels in `active_cycle`,
  which identified the production owner precisely.
- Validator: configured-flow unit plus two public exact smokes, 9/10. The
  consume-final family filter was useful as residual clustering evidence but not
  acceptance because one pre-existing docflow-verdict case remains red.

Post-Task Self-Analysis:
- Worked: replaying the exact CLI JSON before editing prevented treating the
  mixed-feature assertion as stale; it exposed a real configured-carrier label
  gap.
- Waste: one manual replay used test timeout-wrapper args as if they were vida
  args, producing unusable `-k` errors.
- Risk: human TOON output and JSON orchestration labels had different audiences;
  changing only JSON would leave the operator-facing summary stale.
- Next change: when a configured-carrier id appears in an operator-facing lane
  list, inspect both JSON contract fields and compact TOON renderers in the same
  slice.
- Docs update: yes; this STOP record adds dynamic criteria for wrapper-arg replay
  and dual-surface label proof.
- workflow_score_10: 8/10.

Twenty criteria outcome:
1. Active bounded unit explicit: pass,
   `wave-0-runtime-tests-boot-smoke-failure-classification`.
2. Wave/parent closure distance: partial, two deterministic consume-final exacts
   improved; broad family still has one residual red.
3. Scope and non-goals stable: pass, logical lane labeling only.
4. Dirty worktree handled: pass, unrelated dirty files remained unstaged after
   the pre-commit stash/restore cycle.
5. Executor cheapest capable: pass, local fix from exact JSON/TOON evidence.
6. Validator matched risk: pass, unit plus public exacts and family residual run.
7. Agent prompts: not applicable.
8. Agent handles: pass, no new handles used.
9. Telemetry: partial, token/cost unavailable; proof commands recorded.
10. Avoidable commands: partial, one invalid manual replay with wrapper args.
11. Proof strength: pass for configured lane logical labels and compact TOON
    summary.
12. Public-surface proof: pass, JSON and default TOON consume-final surfaces
    covered.
13. Debug build: pass, cargo tests rebuilt `vida`.
14. TaskFlow state: partial, classification task remains active.
15. Staging by invariant: pass, only the orchestration builder and consume-final
    renderer were staged.
16. Publication authorization: pass, pushed to `main`.
17. Evaluation docs: pass, this STOP record is being written before the next
    runtime slice.
18. Parent/wave metrics: consume-final family is now `17 passed`, `1 failed` for
    the focused filter; full boot-smoke remains red.
19. New defects/follow-ups: next deterministic consume-final blocker is the
    docflow-verdict closure case with unexpected `selected_backend`.
20. Next routing rule: pass, continue within the consume-final family on the
    remaining deterministic exact before returning to broader run-graph/status
    clusters.

Final dynamic criteria STOP point:
1. Wrapper-arg replay criterion: when reproducing a boot-smoke helper manually,
   separate test harness timeout arguments from real `vida` CLI arguments before
   trusting the replay output.
2. Dual-surface label criterion: if a fix changes lane identity, verify both the
   machine JSON contract and the human compact output; carrier ids are allowed in
   deep contract fields but should not leak into operator summaries when logical
   lane labels are available.
3. Family-residual criterion: after a family filter leaves exactly one red case,
   record that case as the next candidate instead of treating the whole family as
   unclassified broad noise.

## 2026-06-12 - Consume-final docflow verdict downstream block

Task / slice:
- `wave-0-runtime-tests-boot-smoke-failure-classification`
- Commit: `2b96429cb block downstream dispatch on docflow verdict`
- Goal: prevent `taskflow consume final` from retaining a downstream dispatch
  target when the final handoff is blocked by DocFlow verdict evidence, while
  preserving conversational tracked-flow downstream targets.

Proof:
- `cargo +1.95.0 fmt --all -- --check`
- `git diff --check -- crates/vida/src/taskflow_consume.rs`
- `cargo +1.95.0 test -p vida --test boot_smoke taskflow_consume_final_blocks_downstream_closure_when_docflow_verdict_blocks -- --nocapture --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke taskflow_consume_final_selects_scope_discussion_role_for_spec_queries -- --nocapture --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke taskflow_consume_final_ -- --nocapture`: `18 passed`, `0 failed`.
- `cargo +1.95.0 test -p vida --test boot_smoke -- --nocapture`: attempted after commit, but the tool timed out at 120 seconds and no captured verdict log was available; the process finished after a follow-up wait, so this is not counted as pass/fail proof.

Observed model results:
- Executor: local orchestrator, 8/10. The failure showed the gate result already
  carried `docflow_verdict_block`, while the primary receipt blocker was
  `closure_admission_block`; the fix needed to use the actual DocFlow readiness
  signal, not only the selected blocker code.
- Validator: exact red, scope regression exact, and the full 18-test
  consume-final family, 9/10.

Post-Task Self-Analysis:
- Worked: adding the scope-discussion regression guard caught the first overbroad
  `docflow_ready=false` cleanup before commit.
- Waste: broad boot-smoke was launched before checking expected runtime length
  and timed out without a usable verdict.
- Risk: DocFlow readiness is also false for conversational tracked-flow cases,
  so a gate-order fix can accidentally erase lawful downstream planning targets.
- Next change: for gate-order fixes, run one positive preservation exact beside
  the negative blocking exact before family/broad proof.
- Docs update: yes; this STOP record adds dynamic criteria for paired gate proof
  and long-gate capture.
- workflow_score_10: 8/10.

Twenty criteria outcome:
1. Active bounded unit explicit: pass,
   `wave-0-runtime-tests-boot-smoke-failure-classification`.
2. Wave/parent closure distance: partial, consume-final family is green; full
   boot-smoke remains unverified after timeout.
3. Scope and non-goals stable: pass, downstream cleanup contract only.
4. Dirty worktree handled: pass, unrelated dirty files remained unstaged after
   pre-commit stash/restore.
5. Executor cheapest capable: pass, local exact-driven fix.
6. Validator matched risk: pass, negative exact, positive preservation exact,
   and full consume-final family.
7. Agent prompts: not applicable.
8. Agent handles: pass, no new handles used.
9. Telemetry: partial, token/cost unavailable; proof commands recorded.
10. Avoidable commands: partial, broad run should have been bounded or deferred
    until after a captured-output plan.
11. Proof strength: pass for consume-final family; broad suite proof not counted.
12. Public-surface proof: pass, public `taskflow consume final` JSON paths covered.
13. Debug build: pass, cargo tests rebuilt `vida`.
14. TaskFlow state: partial, classification task remains active.
15. Staging by invariant: pass, only `taskflow_consume.rs` staged.
16. Publication authorization: pass, pushed to `main`.
17. Evaluation docs: pass, this STOP record is being written before the next
    runtime slice.
18. Parent/wave metrics: focused consume-final family is `18 passed`, `0 failed`;
    full broad count was not captured.
19. New defects/follow-ups: broad proof needs a longer bounded runner or captured
    log path before it can be used as closure evidence.
20. Next routing rule: pass, move from consume-final family to the next
    deterministic cluster only after this STOP record and docs commit.

Final dynamic criteria STOP point:
1. Paired-gate-proof criterion: every gate-order fix must prove both the blocked
   negative case and at least one allowed positive case that could be accidentally
   erased by the new gate.
2. Broad-log-capture criterion: before launching a long broad suite, choose a
   command path that preserves a verdict beyond the tool timeout; otherwise
   classify it as exploratory and do not count it as proof.
3. Primary-vs-secondary-blocker criterion: when a surface has multiple blocker
   codes, do not key downstream behavior only from the selected primary blocker;
   inspect the underlying gate readiness that owns the behavior.

## 2026-06-12 - Run-graph flow-state smoke fixture alignment

Task / slice:
- `wave-0-runtime-tests-boot-smoke-failure-classification`
- Commit: `ea63f6418 align run graph flow state smoke fixtures`
- Goal: update run-graph flow-state smokes so they create the TaskFlow task
  authority required by the current stale-missing-task guard and assert the
  current open delegated cycle recovery contract.

Proof:
- `cargo +1.95.0 fmt --all -- --check`
- `git diff --check -- crates/vida/tests/boot_smoke.rs`
- `cargo +1.95.0 test -p vida --test boot_smoke taskflow_direct_run_surfaces_report_non_empty_bridged_flow_state -- --nocapture --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke taskflow_run_graph_bridge_syncs_non_empty_latest_flow_surfaces -- --nocapture --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke status_and_doctor_text_surfaces_report_non_empty_latest_flow_state -- --nocapture --exact`
- `cargo +1.95.0 test -p vida --test boot_smoke non_empty_latest_flow -- --nocapture`: `2 passed`, `0 failed`; the direct-run exact is not covered by that substring and was run separately.

Observed model results:
- Executor: local orchestrator, 7/10. The final fix was small, but the first
  fixture insertion hit the wrong adjacent test twice because the surrounding
  setup blocks were highly similar.
- Validator: three exact public smoke tests plus a focused substring filter,
  8/10. It caught both stale missing-task fixture shape and blocked open-cycle
  expectation drift.

Post-Task Self-Analysis:
- Worked: manual replay showed the distinction between missing TaskFlow task and
  open delegated cycle, so the final test assertions preserve the current
  fail-closed recovery contract.
- Waste: the first attempted residual (`status_init_and_graph_summary...`) was a
  timeout/output-size cluster and should have been classified separately before
  switching to a different exact.
- Waste: two patch attempts inserted the fixture into adjacent similar tests
  rather than the intended direct-run test.
- Risk: fixing only the first failing flow-state exact would have left adjacent
  JSON/text flow-state surfaces with inconsistent fixture authority.
- Next change: when multiple tests share nearly identical setup, patch by
  function-name context and verify the inserted line location with `rg` before
  running proof.
- Docs update: yes; this STOP record adds dynamic criteria for function-context
  patching and timeout-cluster deferral.
- workflow_score_10: 7/10.

Twenty criteria outcome:
1. Active bounded unit explicit: pass,
   `wave-0-runtime-tests-boot-smoke-failure-classification`.
2. Wave/parent closure distance: partial, three flow-state exacts improved; one
   run-graph dispatch-init timeout cluster remains open.
3. Scope and non-goals stable: partial, final scope became a three-test fixture
   batch after accidental adjacent insertions exposed the shared invariant.
4. Dirty worktree handled: pass, unrelated dirty files remained unstaged.
5. Executor cheapest capable: pass, local test fixture update.
6. Validator matched risk: pass, exacts across direct, latest JSON, and text
   status/doctor surfaces.
7. Agent prompts: not applicable.
8. Agent handles: pass, no new handles used.
9. Telemetry: partial, token/cost unavailable; proof commands recorded.
10. Avoidable commands: partial, one timeout exact and two misplaced patches.
11. Proof strength: pass for the flow-state fixture contract.
12. Public-surface proof: pass, run-graph, recovery, status, and doctor public
    surfaces covered through smoke tests.
13. Debug build: pass, cargo tests rebuilt `vida`.
14. TaskFlow state: partial, classification task remains active.
15. Staging by invariant: pass, only `boot_smoke.rs` staged.
16. Publication authorization: pass, pushed to `main`.
17. Evaluation docs: pass, this STOP record is being written before next work.
18. Parent/wave metrics: flow-state batch exacts are green; broad count not
    refreshed because the long broad command lacks a capture plan.
19. New defects/follow-ups: `status_init_and_graph_summary...` is a separate
    dispatch-init timeout/output-size cluster and should not be mixed with the
    flow-state fixture batch.
20. Next routing rule: pass, choose the next exact by deterministic fast failure
    unless deliberately entering the dispatch-init performance/output cluster.

Final dynamic criteria STOP point:
1. Function-context-patch criterion: when adjacent tests share identical setup
   blocks, patch using the function name plus local context and immediately
   verify insertion location with `rg` before running tests.
2. Timeout-cluster-deferral criterion: if an exact failure is a command timeout
   with large JSON output, classify it as a performance/output slice unless the
   active task is explicitly that cluster; do not blend it into a fast fixture
   assertion slice.
3. Open-delegated-cycle-exit criterion: recovery surfaces may return nonzero
   while still carrying valid bridged state. Tests whose purpose is state
   projection should assert both the blocking code and the projected state.

-----
artifact_path: process/agent-model-evaluation-log
artifact_type: process_doc
artifact_version: '1'
artifact_revision: 2026-06-11
schema_version: '1'
status: active
source_path: docs/process/agent-model-evaluation-log.md
created_at: 2026-06-11T00:00:00+03:00
updated_at: 2026-06-12T08:01:00+03:00
changelog_ref: agent-model-evaluation-log.changelog.jsonl
