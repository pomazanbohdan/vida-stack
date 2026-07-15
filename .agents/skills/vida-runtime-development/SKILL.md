---
name: vida-runtime-development
description: "VIDA runtime development workflow for vida-stack. Use when working on TaskFlow, DocFlow, runtime diagnostics, operator command surfaces, run-graph/lane/dispatch state, issue-driven runtime defects, command-efficiency follow-ups, or session closeout/proof planning in C:/project/vida-stack."
---

# VIDA Runtime Development

Use this skill to keep VIDA runtime work evidence-first, TaskFlow-backed, and low-friction.

## Start Sequence

1. Read `AGENTS.md` and `AGENTS.sidecar.md` after any compact or re-entry.
2. Run `vida orchestrator-init --json` before selecting work.
3. State:
   - `active_bounded_unit`
   - `why_this_unit`
   - `sequential_vs_parallel_posture`
4. If a write is needed, create a DB-backed `step` first.
5. For runtime defects, load `docs/process/project-error-search-runtime-diagnostics-protocol.md`.
6. For command/output/timing friction, load `docs/process/command-timing-and-gate-optimization-protocol.md`.
7. For session-level runbook details, load `references/session-runtime-runbook.md`.

## Code Search and Optional Graph Memory

Use the canonical search chain for discovery and impact analysis:

1. Run `ccc index` when the project index is absent or stale, then use `ccc search` for semantic discovery and ranked concepts.
2. Run `ctx_compose` before `ctx_search`; use `ctx_search` for filesystem-aware confirmation, bounded context, and impact context.
3. Use `rg` for exact symbols, strings, paths, `rg --files`, and high-recall fallback or confirmation.
4. Keep `codebase-memory-mcp` optional for fresh graph, call, and architecture context; use `lean-ctx` for current filesystem truth, uncommitted edits, and command/build/test output. Never let an indexed or search projection outrank current worktree evidence.
5. Search/discovery output is navigation and impact evidence, not proof. Proof requires focused tests, runtime/CLI commands, or recorded artifacts.

## Runtime Evidence Order

Prefer the smallest authoritative surface that answers the question:

1. `vida task show <task-id> --json` for known task metadata.
2. `vida task tree <task-id> --json` for parent/child closure shape.
3. `vida task validate-graph --json` after TaskFlow mutations.
4. `vida status --json` and `vida doctor --json` for projection parity.
5. `vida orchestrator-init --json` for active bounded-unit and continuation binding.
6. Run-graph, lane, recovery, and dispatch surfaces only when the defect needs that evidence.

Do not treat derived summaries, stale cache, lane preview, or advisory text as stronger than authoritative TaskFlow state, receipt-backed evidence, and current runtime status.

## Write Discipline

Before every project mutation:

1. Create a `step` under the active task or current epic.
2. State `STEP`, `STOP`, and `IF_BLOCKED`.
3. Keep writes sequential when they mutate the same TaskFlow graph, docs map, skill folder, or runtime state.
4. Close the step only after validation passes.
5. If `task close` rejects a valid close because of literal words in the reason, record that as operator-surface evidence and retry with neutral wording only for the same bounded close step.
6. For runtime defects, prefer architectural contract fixes over local symptom patches. Move duplicated or fragile behavior into a named helper or contract boundary when the defect affects more than one surface, receipt, JSON field family, or operator workflow.
7. Do not preserve a narrow workaround if it leaves the same invariant implicit elsewhere. Make the invariant explicit, reusable, and covered by public-surface tests.
8. Before implementing a registered runtime defect, perform a freshness audit against the current worktree/runtime. Old issue comments, TaskFlow notes, PR reviews, and agent reports are leads, not authority. Classify each symptom as `actual_now`, `partially_fixed`, `superseded`, `merged_into_broader_invariant`, or `stale_not_reproduced`.
9. If the freshness audit shows multiple registered defects are manifestations of one shared boundary problem, stop local symptom fixing and write the shared architectural decision first: canonical owner, invariant, affected surfaces, stale reports to update/close later, and proof suite.
10. Do not close or implement a stale registered defect as written. Update the task note with the live evidence and either re-scope it to the current invariant or mark it blocked/stale according to TaskFlow policy.
11. For architectural fixes, run a shared/deduplication research gate before editing. Inspect whether status, doctor, diagnostics, graph-summary, DocFlow, lane, consume surfaces, renderers, packet repair, command-output schemas, CLI help/options, persisted-state adapters, fixture builders, or integration harnesses compute the same blocker/action/verdict separately. This gate is mandatory even when the reported defect names one file or one command. If duplication exists, introduce, extend, or reuse a shared contract/helper/verdict/renderer/schema/harness module instead of copying another branch.
12. Record the shared-boundary decision before write-producing work. Name the decision as `introduce`, `extend`, `reuse`, or `not_applicable`; name the accepted shared seam, files/functions to decompose, affected callers, command/help/output implications, rejected alternatives, and the reason if no shared boundary is introduced.
13. Include test architecture in the design decision. Identify which existing tests must be rewritten, deleted, or consolidated around the shared invariant, which remain as compatibility checks, and which new integration tests prove the public surfaces converge. Include default TOON output, explicit JSON output, help/options, persisted-state fixtures, and cross-surface parity when the changed command family exposes them.
14. Treat the architectural fix as incomplete until adjacent duplicated branches, stale local renderers, stale option/help text, old fixture builders, obsolete snapshots, and tests that preserve old duplicate behavior are rewritten, removed, or explicitly retained as compatibility proof. A test rewrite is part of the fix when the old test would force duplicated production code to remain.

## Command Efficiency

After each coherent runtime work pool, check whether the session required avoidable operations:

- full backlog scans instead of filtered task search,
- raw reruns because compact output hid key fields,
- large default JSON/log output where a compact summary plus artifact refs would have been enough,
- host/tool truncation, compression watchdog timeouts, transport closure, or megabyte runtime artifacts,
- client-side JSON unwrapping because stable field selectors were missing,
- repeated `status`, `doctor`, `task tree`, and GitHub reads for one proof bundle,
- noisy output where a child summary would have been enough.

If yes, create or update an operator-efficiency TaskFlow item under the current runtime/quality epic.

Adequate runtime output is the smallest output that preserves proof and next-action clarity. Prefer selected fields, compact default views, bounded head/tail/range views, and artifact-backed raw logs. When two command shapes prove the same fact, the lower model-visible token/byte output is better. Treat oversized default output, missing selectors, raw host-bridge prompts in normal output, and >1 MiB artifacts required for normal completion as runtime defects, not cosmetic issues.

For parallel task packs, run one explicit finalize sequence:

1. Attach structured proof evidence to every task before `vida task close`; close reason text is not proof authority.
2. Close pack tasks sequentially because TaskFlow graph mutation is a shared runtime domain.
3. Run `vida task reconcile-closed-runs --limit 25 --json`, then `vida orchestrator-init --json`.
4. Treat a remaining `closed_task_active_run_projection_mismatch` as a runtime blocker or follow-up, not as a clean pack close.
5. Record broad test deltas as `inside_pack`, `adjacent_regression`, or `outside_pack_residual` before closing tasks.
6. Run the runtime self-diagnostic once for the whole closed pack before selecting unrelated work.
7. Actualize project-skill creation or updates through `docs/process/agent-skill-learning-protocol.md`: collect close, self-analysis, diagnostic, and user-correction events; record `no_skill_update_reason` when no skill update is needed; otherwise stage or validate the bounded proposal.
8. Actualize TaskFlow last, after skill-learning actualization and immediately before next-work analysis.
9. Create or update runtime follow-ups for repeated command-output loss, oversized output/token waste, repo-root temp leakage, missing pack-level proof/close commands, or shell PATH drift.

## Agent Research Ring

For complex runtime defects, use an adaptive agent research ring instead of a fixed number of research loops.

1. Agent research is the default for multi-surface runtime defects, external-project live defect reports, PR/security findings, and any blocker that repeats after a local patch. Start agents before local implementation unless the next local command is the only critical-path proof.
2. Start with 2-3 parallel `middle`/medium read-only agents when the defect spans multiple runtime surfaces. Split the prompts by evidence owner:
   - owner/code path and task mapping,
   - state/config/runtime boundary,
   - failing proof and closure tests.
3. If thread limits, provider limits, or host-tool capability gaps prevent the full ring, launch the maximum available agents immediately, state the reduced lane count, and continue with non-overlapping local validation only. Do not silently collapse the ring into root-only investigation.
4. Follow with one inverse coach/reviewer agent whose job is to challenge the findings, look for false-green fixes, missing dependencies, and scope creep.
5. Use an architect/synthesis agent only when the first reports conflict or the defect crosses architectural ownership boundaries.
6. Stop research when the ring proves:
   - root cause or a bounded root-cause tree,
   - owning files/functions,
   - fix-now versus follow-up boundary,
   - failing proof and closure proof suite,
   - inverse coach found no blocking gap.
7. Prefer 1 explorer + 1 coach for simple defects, 3 explorers + 1 coach for runtime surface mismatches, and 3 explorers + 1 inverse coach + optional architect for architectural defects.
8. Use large fixed loops, such as 10 rounds, only when the defect decomposes into multiple independent sub-defects or the agent reports materially disagree.
9. Keep prompts narrow. Ask each agent for concrete file/function references, defect ownership, proof gaps, and explicit non-goals. Do not ask every agent to rediscover the entire codebase.
10. Root/orchestrator remains the consolidator. Do not apply the first minimal patch proposal if it only fixes a fixture or local symptom while preserving a broader runtime mismatch.
11. Before closing a high-risk runtime defect, run at least one coach/verifier pass over the implemented patch and proof plan. If no coach lane is available, record that capacity blocker in the TaskFlow note and keep closure conservative.
12. After each agent result arrives, immediately classify it as accepted evidence, partial evidence, conflict, content failure, process failure, irrelevant, or stale. Once classified, close or delete the completed agent handle in the same orchestration step. If the host supports both close and delete/remove, prefer the strongest cleanup operation that does not discard the result artifact or transcript already captured by the orchestrator. Do not launch the next agent, start local implementation, or continue the research ring while a classified completed handle remains open. Stale completed handles are a capacity defect in long sessions, because they can exhaust the host thread limit and prevent the next research ring.

## Proof And Closure

Minimal proof for this skill's work:

1. `vida task validate-graph --json`
2. Focused DocFlow check when docs changed.
3. Relevant runtime parity command, such as `vida status --json` plus `vida doctor --json`.
4. `git status --short` before reporting closure.
5. Any GitHub issue or PR state from live `gh` or connector evidence when GitHub surfaces changed.

## Test Coverage Standard

1. Runtime defects that block another project, affect operator JSON, change dispatch/run-graph/lane/TaskFlow state, or repair a security/actionability bug require integration or smoke coverage through the public CLI surface. Unit tests alone are not sufficient.
2. Target 100% coverage of the changed behavior path: success path, fail-closed blocker, machine-readable `blocker_codes`, `next_actions`, and `artifact_refs` needed for the operator to act.
3. For parity defects, test every affected public surface together, such as `status`, `doctor`, `consume continue`, `dispatch-next`, and `run-graph status`, instead of testing only the helper that computes the flag.
4. For packet/state repair defects, add at least one fixture that uses persisted runtime state and invokes the same command family an operator or agent would run.
5. If an integration test is impossible in the current slice, record the exact blocker in the TaskFlow note, keep the task open unless the user explicitly accepts the risk, and create a follow-up before closing related parent work.
6. Keep unit tests for small classifiers/helpers, but treat them as supporting proof below the integration/smoke test.
7. When writing integration tests for a file or public surface, write the whole planned batch first. Do not run full suites after each small assertion. Use focused tests only while shaping the batch, then run broader/full suites only after the complete file/surface batch is written.
8. A test batch should be research-shaped before execution: identify the public scenarios, success/fail-closed paths, snapshot or persisted-state parity, and operator JSON fields that belong to the same file/surface, then implement those tests together.
9. If a focused test exposes a production defect during batch writing, fix the production defect and continue completing the remaining batch before broad/full verification.
10. Prefer smaller integration tests with varied fixtures over one huge scenario. Cover multiple meaningful variants such as ready path, blocked DocFlow, blocked closure admission, dispatch packet preview blocked, persisted snapshot parity, and downstream dispatch gating.
11. When a defect is architectural, tests should prove the invariant at the contract boundary and through every affected public CLI surface family in the bounded scope. Avoid tests that only encode the immediate symptom line-by-line.
12. For shared/deduplication fixes, rewrite or consolidate existing tests that duplicate old behavior, including tests in the currently touched file, adjacent smoke/integration fixtures, persisted-state fixtures, generated snapshots, CLI help/output assertions, and JSON parity checks. The proof suite must include a shared-helper/unit contract test plus public CLI integration tests for every affected surface family.
13. For architectural file decomposition, write tests against the new shared boundary and the public command surface, not only the extracted private function. Remove old tests that require a duplicated or legacy path to keep passing, and add varied integration fixtures for each public behavior that moved to the shared boundary.
14. For command-output changes, integration coverage must prove the default human output is compact TOON/plain, the explicit `--json` path remains machine-readable, and `--help` documents both modes.
15. Human-facing `next_actions`, remediation hints, and default command suggestions must prefer default commands without `--json`. Keep `--json` in option/help text and JSON-specific machine workflows only.
16. For runtime architectural remediation, target 100% coverage of the changed behavior path: success, fail-closed blocker, default compact output, explicit JSON output, help/options, next-action guidance, persisted-state fixture, and cross-surface parity.
17. For runtime CLI surfaces and shared runtime contracts, matrix/table-driven integration tests are the default. The matrix must cover help, default compact TOON/plain, explicit JSON, success, blocked/fail-closed states, shared operator-contract fields, `blocker_codes`, `next_actions`, `artifact_refs`, and state-dir override when supported. Point tests are regression add-ons, not the primary proof.
18. When a root cause is found, immediately audit the existing test suite for completeness and consistency before closing or narrowing the fix. Check whether tests cover the whole behavioral class, every affected public surface, stale duplicate tests, contradictory assertions, fixture gaps, and the shared invariant that should have failed. Update, consolidate, or delete inconsistent tests as part of the same bounded fix.
19. After fixing the root cause and any adjacent defects inside a task, run a read-only agent pattern sweep before task closure. This is a closure gate, not an optional review. The agent must search for the same error pattern across related files, shared helpers, command surfaces, tests, fixtures, persisted-state snapshots, operator outputs, and next-action text. Capture and classify the result in the task note; if the pattern exists outside the current bounded fix, create or update a follow-up TaskFlow task with acceptance criteria before closing the current task. If no wider pattern is found, record explicit negative evidence before closure.

Do not close GitHub issues, epics, or TaskFlow parents until the relevant children and proof targets are current.

-----
artifact_path: .agents/skills/vida-runtime-development
artifact_type: agent_skill
artifact_version: '1'
artifact_revision: '2026-07-01'
schema_version: '1'
status: canonical
source_path: .agents/skills/vida-runtime-development/SKILL.md
created_at: '2026-07-01T00:00:00+03:00'
updated_at: 2026-07-01T18:14:00+03:00
changelog_ref: vida-runtime-development.changelog.jsonl
