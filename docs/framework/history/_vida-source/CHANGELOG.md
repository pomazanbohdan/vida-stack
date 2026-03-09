# Changelog

Rules:

1. Newest entries must always be added at the top.
2. Each entry must start with a full timestamp in `YYYY-MM-DD HH:MM` format.
3. Record only significant framework changes.
4. Group updates under fixed headings when applicable: `Added`, `Changed`, `Fixed`, `Protocol`.
5. Keep this file limited to VIDA framework/runtime changes, not project feature work.

## 2026-03-08 18:30

Changed:

1. Recalibrated the canonical selector bands in [thinking-protocol.md](/home/unnamed/project/vida-stack/_vida/docs/thinking-protocol.md) to `STC <=12`, `PR-CoT 13-22`, `MAR 23-32`, `5-SOL 33-42`, and `META >42`, so medium/high-governance work escalates earlier without making `META` the default for simple execution.
2. Added explicit `META` routing overrides in [thinking-protocol.md](/home/unnamed/project/vida-stack/_vida/docs/thinking-protocol.md) for framework-owned behavior changes, protocol conflicts, execution gate mismatch, fail-closed law risk, and tracked writer `no_eligible_*` routing gaps.
3. Added canonical `RISK_ESCALATORS` and `RETROSPECTIVE_ESCALATION` rules in [thinking-protocol.md](/home/unnamed/project/vida-stack/_vida/docs/thinking-protocol.md), including confirmed `STC` misfire handling so the same task class does not silently repeat under too-weak routing.
4. Synced the operational summary in [algorithms-quick-reference.md](/home/unnamed/project/vida-stack/_vida/docs/algorithms-quick-reference.md) to the new selector bands and escalation rules.
5. `execution-auth-gate.py` now treats explicit `no_eligible_verifier` as a lawful verification-ready state and adds a framework-only structured execution-auth override path for tracked `no_eligible_analysis_lane` cases instead of forcing unsynchronized local writer fallback.
6. Added focused regression coverage in [test_execution_auth_gate.py](/home/unnamed/project/vida-stack/_vida/tests/test_execution_auth_gate.py) and clarified the fail-closed exception in [implement-execution-protocol.md](/home/unnamed/project/vida-stack/_vida/docs/implement-execution-protocol.md).
7. Tightened `execution-auth-gate.py` so structured execution-auth overrides are fail-fast and limited to framework-labeled `no_eligible_analysis_lane` cases, and synced the compact matrix in [algorithms-one-screen.md](/home/unnamed/project/vida-stack/_vida/docs/algorithms-one-screen.md).

## 2026-03-08 02:11

Fixed:

1. [quality-health-check.sh](/home/unnamed/project/mobile-odoo/_vida/scripts/quality-health-check.sh) now runs execution-auth validation only for real `dev-pack` implementation context, not for reflection/docs tasks that merely share writer-like block ids or retain stray implementation receipts.
2. Added regression coverage in [test_beads_runtime.py](/home/unnamed/project/mobile-odoo/_vida/tests/test_beads_runtime.py) so this health-check classification path stays pinned to execution context instead of block-id or receipt-only heuristics.

Changed:

1. Closed the stale `mobile-1j1*` coach-pipeline backlog after reconciling it against implemented runtime surfaces, later proving waves, and template/runtime sync work already landed elsewhere in VIDA.

## 2026-03-08 00:41

Added:

1. Added [instruction-activation-protocol.md](/home/unnamed/project/mobile-odoo/_vida/docs/instruction-activation-protocol.md) as the canonical framework surface for phase-based instruction loading, trigger-only protocol activation, naming guidance, and instruction-layer decomposition rules.

Changed:

1. [AGENTS.md](/home/unnamed/project/mobile-odoo/AGENTS.md) now points to the instruction activation protocol instead of silently treating bootstrap text as the place to restate deeper activation policy.
2. [ORCHESTRATOR-ENTRY.MD](/home/unnamed/project/mobile-odoo/_vida/docs/ORCHESTRATOR-ENTRY.MD) now treats instruction loading as a triggered decision bound to the new activation protocol instead of broadening the boot read-set by implication.
3. [protocol-index.md](/home/unnamed/project/mobile-odoo/_vida/docs/protocol-index.md) now indexes instruction activation/decomposition as a canonical framework domain.

## 2026-03-08 00:35

Fixed:

1. `beads-workflow.sh` now probes bootstrap context hydration with `VIDA_CONTEXT_HYDRATE_ALLOW_MISSING=1`, so the first context-capsule bootstrap no longer emits a false `context_hydration_failed` before the capsule exists.
2. Added regression coverage in [test_beads_runtime.py](/home/unnamed/project/mobile-odoo/_vida/tests/test_beads_runtime.py) for the pending-capable bootstrap hydration contract.
3. `framework-operator-status.py` now reconciles closed framework bugs out of the rendered silent-diagnosis backlog, keeping the operator surface consistent with `vida-silent-diagnosis.py status`.
4. Added task-state reconciliation via [task-state-reconciliation-protocol.md](/home/unnamed/project/mobile-odoo/_vida/docs/task-state-reconciliation-protocol.md) and [task-state-reconcile.py](/home/unnamed/project/mobile-odoo/_vida/scripts/task-state-reconcile.py), and surfaced the derived classification in health/operator/boot flows.

## 2026-03-07 21:34

Added:

1. Added local trace-grading protocol and helper via [trace-eval-protocol.md](/home/unnamed/project/mobile-odoo/_vida/docs/trace-eval-protocol.md) and [trace-eval.py](/home/unnamed/project/mobile-odoo/_vida/scripts/trace-eval.py).
2. Added focused regression coverage for trace grading and dataset export in [test_trace_eval.py](/home/unnamed/project/mobile-odoo/_vida/tests/test_trace_eval.py).
3. Added typed capability registry protocol and helper via [capability-registry-protocol.md](/home/unnamed/project/mobile-odoo/_vida/docs/capability-registry-protocol.md) and [capability-registry.py](/home/unnamed/project/mobile-odoo/_vida/scripts/capability-registry.py).

Changed:

1. `subagent-eval-pack.py` now emits a compact `trace_eval` summary alongside the existing `eval_pack`, so post-task strategy refresh can bind to first-class local trace grading instead of only task-close metrics.
2. `protocol-index.md` and `quality-health-check.sh` now treat local trace grading as a canonical framework surface.
3. `subagent-eval-pack.py` now also emits a compact `trace_dataset` export reference in `subagent-review-<task_id>.json`, and `pipelines.md` now treats trace eval + dataset artifacts as part of the normal telemetry improvement loop.
4. `subagent-system.py` now applies a typed capability gate before candidate scoring, so capability-incompatible lanes are suppressed instead of merely being down-ranked heuristically.

## 2026-03-07 21:12

Added:

1. Added durable run-graph protocol and helper via [run-graph-protocol.md](/home/unnamed/project/mobile-odoo/_vida/docs/run-graph-protocol.md) and [run-graph.py](/home/unnamed/project/mobile-odoo/_vida/scripts/run-graph.py) so routed execution now has a canonical node-level resumability ledger under `.vida/state/run-graphs/`.

Changed:

1. `subagent-dispatch.py` now updates the run-graph ledger for `analysis`, `writer`, `coach`, `verifier`, `approval`, and `synthesis` transitions instead of keeping routed progress implicit in manifests alone.
2. `vida-boot-snapshot.py` now surfaces compact run-graph resume hints for active tasks, so compact recovery can see the next resumable node without broad state inspection.
3. `framework-operator-status.py` now summarizes active run-graphs and their next resumable nodes as part of the operator surface.
4. `quality-health-check.sh` now treats the durable run-graph protocol/helper as required framework files.

Fixed:

1. Routed execution after compact or lane failure no longer depends only on TODO or manifest inference to find the next resumable orchestration stage.

## 2026-03-07 21:12

Changed:

1. `execution-auth-gate.py` and `subagent-dispatch.py` no longer treat `draft_execution_spec` as a direct implementation authorization substitute for a missing `issue_contract`; draft execution-spec now remains a pre-launch review artifact instead of an execution bypass.
2. Runtime validators in `subagent-dispatch.py` now reuse the canonical helper schemas from `spec-intake.py`, `spec-delta.py`, and `draft-execution-spec.py` instead of checking only a shallow field subset.
3. `writer_prompt_text` no longer injects stale `draft_execution_spec` content into issue-contract-driven writer prompts, removing a contradiction path between pre-launch spec review artifacts and normalized issue contracts.
4. `spec-intake-protocol.md`, `spec-delta-protocol.md`, `issue-contract-protocol.md`, `implement-execution-protocol.md`, and `spec-contract-artifacts.md` now define deterministic next-hop semantics for spec intake, spec delta, issue contract readiness, and draft execution-spec handoff.

Fixed:

1. Added regression coverage to prove that missing `issue_contract` stays blocked even when a `draft_execution_spec` exists, while helper-level schema regressions for `spec_intake`, `spec_delta`, and `draft_execution_spec` now fail the focused suite.

## 2026-03-07 20:48

Added:

1. Added [spec-intake-protocol.md](/home/unnamed/project/mobile-odoo/_vida/docs/spec-intake-protocol.md) and [spec-intake.py](/home/unnamed/project/mobile-odoo/_vida/scripts/spec-intake.py) as the new compact normalization layer for mixed research, issue/release signals, and user-scope negotiation before SCP/ICP/FTP.
2. Added focused tests for the new intake helper in [test_spec_intake.py](/home/unnamed/project/mobile-odoo/_vida/tests/test_spec_intake.py).
3. Added [spec-delta-protocol.md](/home/unnamed/project/mobile-odoo/_vida/docs/spec-delta-protocol.md), [spec-delta.py](/home/unnamed/project/mobile-odoo/_vida/scripts/spec-delta.py), and focused tests in [test_spec_delta.py](/home/unnamed/project/mobile-odoo/_vida/tests/test_spec_delta.py) for explicit non-equivalent contract reconciliation.

Changed:

1. `spec-contract-protocol.md`, `issue-contract-protocol.md`, `form-task-protocol.md`, and `implement-execution-protocol.md` now require normalized `spec_intake` when raw upstream inputs are still mixed or scope-bearing instead of letting downstream formation start from raw text.
2. `protocol-index.md` now maps spec-intake normalization as a canonical framework protocol surface.
3. `bug-fix-protocol.md`, `issue-contract-protocol.md`, and `form-task-protocol.md` now require explicit `spec_delta` materialization instead of treating non-equivalent reconciliation as an implicit side note.
4. `spec-contract-protocol.md`, `form-task-protocol.md`, `spec-contract-artifacts.md`, and `/vida-spec` now require a compact draft execution-spec so user review and task formation happen from a bounded execution contract instead of broad prose.
5. Added [draft-execution-spec.py](/home/unnamed/project/mobile-odoo/_vida/scripts/draft-execution-spec.py) and focused tests in [test_draft_execution_spec.py](/home/unnamed/project/mobile-odoo/_vida/tests/test_draft_execution_spec.py) so the draft execution-spec contract has a canonical helper surface instead of living only in prose.

## 2026-03-07 20:42

Added:

1. Added [future.md](/home/unnamed/project/mobile-odoo/_vida/docs/future.md) to separate out-of-environment platform roadmap items from locally remaining non-Rust VIDA work.

Changed:

1. `protocol-index.md` now maps `future.md` explicitly as a non-canonical framework reference surface so future alignment notes do not become stray undocumented docs.

## 2026-03-07 20:33

Changed:

1. `project-overlay-protocol.md` and `AGENTS.md` now define the canonical no-overlay execution rule explicitly: without `vida.config.yaml`, VIDA may use only framework-owned wrappers/commands and must not infer a host-project runbook.
2. `subagent-system-protocol.md` now defines explicit omission semantics for law-bearing route fields: config may omit them before derivation, but effective route receipts must materialize them before authorization or the route fails closed.
3. `problem-party-protocol.md` now uses an explicit entry/escalation decision matrix instead of heuristic trigger language.
4. `human-approval-protocol.md` now declares canonical runtime review-state names and marks older aliases as legacy/reference-only terminology.
5. `protocol-index.md` now indexes `tooling.md`, clarifies the no-overlay command path for project operations, and adds an explicit completeness rule for framework-owned protocol mapping.
6. `ORCHESTRATOR-ENTRY.MD`, `subagent-system-protocol.md`, and `silent-framework-diagnosis-protocol.md` now use stricter algorithmic wording in previously heuristic law-bearing sections.

## 2026-03-07 19:45

Added:

1. Added first-class human approval receipt helper via [human-approval-gate.py](/home/unnamed/project/mobile-odoo/_vida/scripts/human-approval-gate.py).
2. Added canonical approval governance protocol via [human-approval-protocol.md](/home/unnamed/project/mobile-odoo/_vida/docs/human-approval-protocol.md).
3. Added framework memory ledger via [framework-memory.py](/home/unnamed/project/mobile-odoo/_vida/scripts/framework-memory.py) and [framework-memory-protocol.md](/home/unnamed/project/mobile-odoo/_vida/docs/framework-memory-protocol.md).
4. Added document lifecycle ledger and validator via [doc-lifecycle.py](/home/unnamed/project/mobile-odoo/_vida/scripts/doc-lifecycle.py) and [document-lifecycle-protocol.md](/home/unnamed/project/mobile-odoo/_vida/docs/document-lifecycle-protocol.md).
5. Added aggregated operator visibility surface via [framework-operator-status.py](/home/unnamed/project/mobile-odoo/_vida/scripts/framework-operator-status.py).

Changed:

1. `subagent-dispatch.py` now fails closed when a route reaches `policy_gate_required`, `senior_review_required`, or `human_gate_required` without a matching approval receipt.
2. `implement-execution-protocol.md` and `subagent-system-protocol.md` now treat human approval as a distinct post-verification closure gate rather than an implied status label.
3. `vida-silent-diagnosis.py` now records durable anomaly memory from framework bug capture and session reflection.
4. `project-overlay-protocol.md` now points framework-owned doc lifecycle/freshness metadata to the dedicated framework protocol/state instead of leaving it implicit.
5. `problem-party.py` now emits route-visible receipts and `subagent-system.py`/`vida-config.py` now carry route metadata for problem-party activation.

Protocol:

1. `protocol-index.md` now maps the human approval lifecycle, framework memory ledger, and document lifecycle/freshness as canonical framework protocols.

## 2026-03-07 19:22

Added:

1. Added bounded `problem-party` protocol and helper for optional multi-role conflict discussion via [problem-party-protocol.md](/home/unnamed/project/mobile-odoo/_vida/docs/problem-party-protocol.md) and [problem-party.py](/home/unnamed/project/mobile-odoo/_vida/scripts/problem-party.py).

Changed:

1. `orchestration-protocol.md` now recognizes `problem_party` as a bounded escalation lens for conflict-heavy but still scoped decisions.
2. `todo-protocol.md` now requires the board artifact path to be recorded in block evidence when a task enters problem-party discussion mode.

Protocol:

1. `protocol-index.md` now indexes problem-party as a canonical framework protocol instead of leaving multi-role discussion as ad hoc behavior.

## 2026-03-07 19:02

Added:

1. Added automatic follow-up task creation for unresolved secondary slices emitted by `issue-split` artifacts.
2. Added config-driven live web-search probe support, including `subagent-system.py web-probe <subagent>`.
3. Added declarative framework wave task-state reconciliation through [framework-task-sync.py](/home/unnamed/project/mobile-odoo/_vida/scripts/framework-task-sync.py).

Changed:

1. Runtime dispatch now borrows/releases `subagent_pool` leases automatically for eligible read-only external CLI lanes.
2. CLI model-cache discovery is now config-driven through `dispatch.models_cache_path` instead of a framework hardcode.

Fixed:

1. Provider-configured web-search lanes no longer stay trust-only during live web-required execution; they must pass a bounded probe or fail closed.
2. Framework wave task-state reconciliation now uses the JSONL-first `br --no-db` path instead of depending on the malformed SQLite database.

Protocol:

1. `issue-contract-protocol.md`, `subagent-system-protocol.md`, `project-overlay-protocol.md`, and `protocol-index.md` now document follow-up issue-split tasks, automatic pool lease integration, live provider-configured web probes, and framework wave state sync.

## 2026-03-07 18:48

Changed:

1. `todo-protocol.md` now treats silent framework diagnosis as a canonical deferred follow-up path inside tracked execution instead of leaving that behavior split across overlay/runtime/FSAP rules.

Fixed:

1. Framework gap capture during active non-framework work now has explicit TODO-level rules for workaround logging, task-boundary handoff, compact-safe persistence, and invalid closure when the gap exists only in chat memory.

Protocol:

1. `silent-framework-diagnosis-protocol.md` now points to `todo-protocol.md` as the execution-layer contract when tracked TODO/`br` flow is active.
2. `protocol-index.md` now cross-links silent diagnosis and TODO planning so deferred framework follow-up behavior is discoverable from either entry point.

## 2026-03-07 18:40

Added:

1. Added queue-backed single-writer task-state mutation path through [br-mutation-queue.py](/home/unnamed/project/mobile-odoo/_vida/scripts/br-mutation-queue.py) and wired mutating `br`/`beads_mutate` calls through the canonical serialized runtime path.
2. Added silent VIDA framework self-diagnosis mode with root overlay support, boot visibility, deferred framework bug capture, and session reflection via [vida-silent-diagnosis.py](/home/unnamed/project/mobile-odoo/_vida/scripts/vida-silent-diagnosis.py).
3. Added reusable leased subagent pool helper in [subagent-pool.py](/home/unnamed/project/mobile-odoo/_vida/scripts/subagent-pool.py).
4. Added reusable product/framework proving-pack templates in [proving-pack.py](/home/unnamed/project/mobile-odoo/_vida/scripts/proving-pack.py).
5. Added mixed-issue split artifacts under `.vida/logs/issue-splits/<task_id>.json` so one bug can preserve a primary executable slice and a secondary unresolved slice without widening the writer lane.

Changed:

1. Cheap analysis/review/coach lanes now fail closed when they return preamble-only output, missing machine-readable payloads, or other low-signal results instead of being treated as successful progress.
2. Runtime scoring and routing now persist and penalize low-fitness cheap-lane behavior, so repeated preamble-only or machine-readable-missing outputs demote lanes instead of letting them stay preferred.
3. Boot snapshots now surface active silent framework diagnosis configuration directly from the overlay.
4. Issue-driven execution now preserves mixed-symptom follow-up scope through explicit issue-split artifacts while keeping writer authorization bound only to the primary proven slice.

Fixed:

1. Cheap lanes that exit with prose-only partial output no longer create false-positive success paths in `prepare-execution` or `coach-review`.
2. Concurrent task-state mutations now serialize through one canonical writer path instead of relying on multiple independent mutating entry points under subagent-heavy execution.
3. Framework self-diagnosis/debug mode is now synchronized between project overlay, template, schema validation, boot/runtime helpers, and protocol docs instead of existing only as fragmented behavior.

Protocol:

1. `beads-protocol.md` now defines queue-backed single-writer serialization as the canonical mutation rule for mutating task-state commands.
2. `framework-self-analysis-protocol.md`, `silent-framework-diagnosis-protocol.md`, `project-overlay-protocol.md`, and `ORCHESTRATOR-ENTRY.MD` now formalize silent framework diagnosis as a background capture protocol for an agentic engineering platform, with quality and token efficiency treated as equal-weight goals.
3. `issue-contract-protocol.md`, `bug-fix-protocol.md`, and `implement-execution-protocol.md` now explicitly document mixed-issue split handling so unresolved secondary symptoms are preserved as follow-up work instead of silently re-entering the current writer scope.

## 2026-03-07 11:23

Added:

1. Route outputs now expose canonical `route_law_summary` so hard routing requirements and blocking conditions are visible before dispatch.
2. Canonical subagent run payloads and ensemble manifests now include machine-readable `route_receipt` artifacts with dispatch, fanout, fallback, verification, and budget law context.
3. Ensemble execution now persists a dedicated `verification` result block so independent verifier routing is inspectable as part of the canonical runtime artifact.

Changed:

1. `framework-self-analysis-protocol.md` now enforces a hard-law doctrine: mandatory framework behavior must be encoded as runtime law, verifier gates, blocker codes, or explicit option matrices instead of advisory prose.
2. `orchestration-protocol.md`, `implement-execution-protocol.md`, `subagent-system-protocol.md`, `subagents.md`, and `bug-fix-protocol.md` now treat external-first dispatch, fanout minimums, independent verification, and pool-graph analysis as invalid-to-bypass requirements rather than recommendations.
3. `/vida-implement` execution now formally requires top-level pool dependency graph analysis before selecting a writer lane for epic/wave/multi-task execution.
4. Ensemble completion now distinguishes `decision_ready` from `synthesis_ready`, so routed flows cannot report completion before required independent verification finishes.

Fixed:

1. Illegal `single` dispatch on routes that require `fanout_then_synthesize` now fails fast with a policy-violation error instead of allowing silent internal/manual bypass.
2. Routed analysis/meta-analysis/review paths now fail closed when they try to skip mandatory external-first or verification requirements.
3. Verification-required ensemble runs no longer appear complete after primary merge consensus alone; they stay blocked or `verification_pending` until the verifier path clears synthesis.

Protocol:

1. VIDA framework self-diagnosis now explicitly classifies missing runtime enforcement for a mandatory rule as a framework defect, not as acceptable operator discipline.
2. Multi-issue bug-fix planning now requires an explicit dependency graph with `blocked|soft-blocked|parallel-investigation|single-writer` classification before implementation order is valid.
3. Routed subagent orchestration now treats manual/direct invocation outside canonical dispatch runtime as protocol-invalid unless a lawful fallback or escalation receipt exists.

## 2026-03-07 11:00

Changed:

1. `thinking-protocol.md` now requires a mandatory `Impact Analysis Checklist` before output for every non-`STC` algorithm, so `PR-CoT`, `MAR`, `5-SOL`, and `META` must carry analysis through downstream scope, contract, operational, follow-up, and residual-risk review.
2. User-facing framework report contracts no longer include explicit subagent/process sections by default; subagent participation stays an internal execution mechanism unless the user explicitly asks to inspect it.

Protocol:

1. `SUBAGENT-ENTRY.MD`, `SUBAGENT-THINKING.MD`, `subagents.md`, `subagent-prompt-templates.md`, and `render-subagent-prompt.sh` now synchronize `impact_tail_policy: required_for_non_stc` and bounded `impact_analysis` return fields for worker lanes.
2. Orchestrator reporting contracts now explicitly keep subagent execution hidden from default visual output while still allowing internal evidence synthesis.

## 2026-03-07 10:53

Changed:

1. `ORCHESTRATOR-ENTRY.MD` now makes orchestration-first execution mandatory for development `execution_flow` when `protocol_activation.agent_system=true` and the effective subagent mode is not `disabled`.
2. `orchestration-protocol.md` now routes development execution through the active subagent system before local implementation and preserves explicit mode distinctions for `native`, `hybrid`, and `disabled`.

Protocol:

1. `subagent-system-protocol.md` now explicitly states that development execution is orchestrator-managed through routed subagent lanes by default when subagent mode is active.
2. Hybrid-mode development is now documented as route-policy-first orchestration with external-first dispatch, bridge fallback, and lawful internal escalation instead of an implicit local-first path.

## 2026-03-07 08:43

Added:

1. Introduced `status_diagnostic` as a dedicated low-cost routing class for compact state/status diagnostics with `local_or_external_first` semantics.
2. Added explicit dispatch-policy metadata to route outputs: `local_execution_allowed`, `local_execution_preferred`, `cli_dispatch_required_if_delegating`, `direct_internal_bypass_forbidden`, `internal_escalation_allowed`, `allowed_internal_reasons`, and `required_dispatch_path`.
3. Added budget-policy telemetry to canonical subagent run logs, including `selected_cost_class`, `selected_budget_units`, `cheap_lane_attempted`, `bridge_fallback_used`, `internal_escalation_used`, `policy_bypass`, `budget_violation`, `cost_escalation_trigger`, and `internal_escalation_receipt`.

Changed:

1. Route budgets now expose explicit cost classes alongside normalized budget units so operator tooling can distinguish `free`, `cheap`, `paid`, and `expensive` lanes.
2. Subagent diagnosis/operator status now surfaces budget-policy summaries in addition to provider health, route graphs, and review targets.
3. The canonical overlay template now mirrors the live budget-policy routing fields from the active project config.

Fixed:

1. `vida-config.py` schema validation now accepts the new routing policy fields used for local-first cheap diagnostics and internal-escalation authorization.
2. `route_budget_limits()` now treats `max_budget_units: 0` as an intentional hard cap instead of silently replacing it with a fallback default.
3. `quality-health-check.sh` now warns when task-scoped canonical subagent runs show routing bypass, budget violations, or internal escalations without receipts.

Protocol:

1. `subagent-system-protocol.md` now explicitly distinguishes `internal_primary`, lawful `internal_escalation`, and forbidden `internal_bypass` for hybrid budget-aware routing.

## 2026-03-07 08:25

Added:

1. `_vida/scripts/vida-boot-snapshot.py` as a compact dev-boot status surface that renders top-level active work, `ready_head`, `decision_required`, and open/in-progress `parent-child` subtask trees.
2. Boot receipts now persist a sibling `boot-snapshot` artifact for dev-oriented boots alongside the existing boot packet.

Changed:

1. `boot-profile.sh` now generates compact boot snapshots during dev-oriented boot runs and treats them as first-class receipt-linked runtime artifacts.
2. `boot-packet.py` now exposes runtime hints for compact snapshot access so boot consumers can prefer bounded task-state reads over broad queue discovery.
3. Lean/dev boot guidance in `ORCHESTRATOR-ENTRY.MD`, `orchestration-protocol.md`, `framework-self-analysis-protocol.md`, `boot-packet-protocol.md`, and `/vida-status` now prefers the compact boot snapshot before wider `br` or repo discovery for development-related context questions.

Fixed:

1. Compact subtask trees now resolve through targeted `br show <id>` reads instead of relying on `br list --json` payloads that omit `parent` metadata for child issues.
2. `boot-profile.sh verify-receipt` now ignores archived `.boot-packet.json` and `.boot-snapshot.json` files when selecting the latest canonical receipt.

Protocol:

1. Development-related `answer_only` and `execution_flow` boot paths now formally prefer one bounded task-state snapshot over broad discovery whenever that snapshot is sufficient to answer the request.

## 2026-03-07 07:50

Added:

1. `_vida/docs/ORCHESTRATOR-ENTRY.MD` as the canonical L0 orchestrator contract replacing the old monolithic `AGENTS.md` body.
2. Explicit worker-lane confirmation markers and blocking-question packet fields in canonical rendered subagent prompts.

Changed:

1. `AGENTS.md` now acts as a bootstrap router that selects orchestrator vs worker entry instead of serving as the full contract for every lane.
2. Orchestration now classifies requests as `answer_only`, `artifact_flow`, `execution_flow`, or `mixed` before engaging `br`, TODO, or pack machinery.
3. The framework now treats subagent execution as the default analysis/review fabric in supported modes while keeping final synthesis, mutation ownership, and user-facing reporting under the orchestrator.
4. Protocol index and framework map now reflect the bootstrap split, worker entry topology, request-intent gate, and log-read budget as canonical framework structure.

Fixed:

1. Worker prompts now carry a complete machine-readable return contract with `question_answered`, `answer`, `evidence_refs`, and `recommended_next_action` instead of a partial summary schema.
2. User-facing reporting now defaults to orchestrator-synthesized conclusions rather than relaying raw subagent reports or fragments.

Protocol:

1. `orchestration-protocol.md`, `subagent-system-protocol.md`, `subagents.md`, `pipelines.md`, and `tooling.md` now codify the hard log-read budget, question-driven worker packets, and the rule that broad `.vida/logs`/`.vida/state`/`.beads` reads require explicit escalation.
2. `subagent-system-protocol.md` and related worker docs now formalize the mode-synced `native|hybrid|disabled` subagent-first behavior and the orchestrator-only ownership of final user-facing answers.

## 2026-03-07 07:25

Added:

1. Route outputs now expose canonical `target_review_state` and `target_manifest_review_state` so cli-subagent review intent is visible before dispatch.
2. Lease diagnostics now retain recent history with explicit conflict, acquire, and release events for operator inspection.

Changed:

1. Routing now applies timeout-instability penalties using `startup_timeout_count`, `no_output_timeout_count`, and `stalled_after_progress_count` in addition to timeout-after-progress signals.
2. Runtime payload loading now canonicalizes legacy note/domain strings during state and scorecard reads so live operator surfaces stay framework-generic.
3. `subagent-eval-pack.py` now reuses canonical review-target helpers from `subagent-system.py` instead of maintaining duplicated review-state mapping logic.

Fixed:

1. Single-run cli-subagent dispatch now has phase-aware timeout parity with ensemble execution instead of one coarse wall-clock timeout.
2. Lease acquisition no longer treats already released leases as active blockers; overlapping active ensembles still fail closed and are recorded as conflicts.
3. Live state/review artifacts no longer surface legacy `provider_state=` or old domain tags such as `odoo_api`, `flutter_ui`, and `riverpod_state`.
4. Verified end-to-end runtime proofs now include:
   - single-run parity proof `r32`
   - lease-conflict proof `r32`
   - clean post-conflict ensemble reruns `r33` and `r34`

Protocol:

1. `subagent-system-protocol.md` now explicitly requires phase-aware timeout parity for single-run lanes, recovery-aware routing softening rules, lease-conflict history visibility, and richer operator timeout/recovery summaries.

## 2026-03-07 06:40

Added:

1. Phase-aware cli-subagent timeout controls were added to runtime/config surface: `startup_timeout_seconds`, `no_output_timeout_seconds`, `progress_idle_timeout_seconds`, and `max_runtime_extension_seconds`.
2. Operator status now exposes recovery history fields and lane-readiness visibility through `eligible_task_classes`, `recovery_attempt_count`, `recovery_success_count`, and last recovery status/timestamp.

Changed:

1. `subagent-dispatch.py` now enforces phase-aware timeout behavior instead of relying only on a single wall-clock budget.
2. `subagent-system.py` now refreshes live runtime/config state before routing, so route outputs reflect current overlay knobs instead of stale init snapshots.
3. Analysis routing now suppresses task-class-demoted cli subagents from core fanout while keeping explicit bridge and internal lanes available.
4. `vida.config.yaml` and `_vida/templates/vida.config.yaml.template` now include per-cli-subagent timeout profiles aligned with observed runtime behavior, including a longer profile for `gemini_cli`.

Fixed:

1. Config validation in `vida-config.py` now accepts the new dispatch timeout knobs as canonical schema.
2. Live ensemble manifests now carry timeout-policy metadata and expose phase-aware runtime behavior as part of the canonical run artifact.
3. `r30` proved the new runtime path end-to-end with `gemini_cli`, `kilo_cli`, and `qwen_cli` reaching semantic consensus and `review_state=promotion_ready` without bridge fallback.

Protocol:

1. `subagent-system-protocol.md` now defines phase-aware timeout policy, recovery-history visibility, and lane-aware demotion semantics as part of the canonical cli-subagent runtime contract.

## 2026-03-07 06:05

Added:

1. `subagent-system.py` now exposes recovery helpers: `recover <subagent>` and `recover-pending`.
2. Ensemble manifests now expose live `active_subagents` and `active_count` during running fanout.

Changed:

1. Runtime vocabulary was pushed further toward canonical `cli subagent` terminology across dispatch, routing, evaluation, and operator status surfaces.
2. Worker gating now relies on structured evidence signals instead of a coarse byte-size fallback for `useful_progress` and `merge_ready`.
3. Operator status now exposes `preferred_task_classes` so lane-fit can be seen without inspecting separate route calls.
4. `quality-health-check.sh` now reads the canonical `.vida/logs/subagent-runs.jsonl` run log and surfaces `cli subagent` health state directly.

Fixed:

1. Routing now hydrates fresh scorecards from `SCORECARD_PATH` instead of relying on stale `INIT_PATH` runtime snapshots.
2. `auth_invalid` and `interactive_blocked` remediation semantics now consistently suppress routing and require bounded recovery/probe flow.
3. Health output now shows degraded/cooldown/probe-required cli subagents by name.
4. Runtime availability state migration now canonicalizes old `provider_state` payloads to `subagent_state`.

Protocol:

1. `subagent-system-protocol.md` now reflects recovery commands, suppressed-subagent visibility, and live ensemble manifest expectations.
2. `subagent-onboarding-protocol.md` now documents recovery flow and routing-block semantics for broken cli subagents.

## 2026-03-07 01:41

Changed:

1. `_vida/templates/vida.config.yaml.template` now mirrors the canonical VIDA provider stack instead of a generic single-provider example.
2. The template now includes practical runtime settings for real CLI subagents: provider tiers, `max_runtime_seconds`, `min_output_bytes`, bridge fallback, and external-first routing metadata.

Fixed:

1. The template now embeds provider-specific timeout environment settings where they are known to be operationally useful, including `OPENCODE_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS` for `kilo_cli` and `opencode_cli`.

Protocol:

1. The default overlay template is now aligned with the real subagent runtime contract, so new projects inherit working provider configuration instead of abstract placeholders.

## 2026-03-07 01:35

Added:

1. `_vida/docs/SUBAGENT-THINKING.MD` as the worker-lane thinking subset limited to `STC`, `PR-CoT`, and `MAR`.

Changed:

1. Worker-lane prompts now inject both entry and thinking contracts through `_vida/scripts/render-subagent-prompt.sh`.
2. Semantic merge now uses similarity-based clustering instead of near-full-text grouping.
3. Scorecards now track runtime maturity metrics including `useful_progress_rate`, `avg_time_to_first_useful_output_ms`, and `timeout_after_progress_count`.

Fixed:

1. Manifest fallback state no longer reports premature `provider_exhausted=true` during active fallback execution.
2. Semantic consensus with strong agreement now resolves more cleanly without unnecessary open conflicts or arbitration.

Protocol:

1. Worker reasoning is now explicitly separated from orchestrator reasoning.
2. Framework docs/scripts were de-projectized to remove host-specific identity, stack, and domain assumptions from canonical runtime policy.

## 2026-03-07 00:15

Added:

1. `_vida/CHANGELOG.md` as the canonical framework change log.

Changed:

1. `_vida/templates/vida.config.yaml.template` to reflect the real agent-system shape:
   `senior_internal`, `external_free`, `cost_priority`, `dispatch.env`, runtime budget fields, and fanout metadata examples.
2. `_vida/docs/protocol-index.md` to link the framework change log.

## 2026-03-06 23:55

Added:

1. `_vida/docs/SUBAGENT-ENTRY.MD` as the worker-lane entry contract.

Changed:

1. `_vida/docs/subagents.md` to separate orchestrator entry from worker entry.
2. `_vida/docs/subagent-prompt-templates.md` so external workers receive bounded worker semantics instead of orchestrator identity.
3. `_vida/scripts/render-subagent-prompt.sh` to inject `Worker Entry Contract` into canonical rendered prompts.

## 2026-03-06 23:20

Changed:

1. Hardened subagent dispatch runtime with managed subprocess polling.
2. Added manifest `phase` visibility for `fanout_running`, `fallback_running`, `merge_evaluating`, `arbitration_running`, and completion states.

Fixed:

1. Added timed termination, early-stop, and unreachable-stop behavior for ensemble fanout.
2. Reduced unnecessary arbitration churn through stronger merge handling.

## 2026-03-06 22:40

Changed:

1. Prioritized free external providers as the default first-pass lane for eligible read-only work.
2. Formalized `gpt-5.1-codex-mini` as the canonical bridge fallback.
3. Moved internal subagents into the senior arbitration / architecture / mutation-owning lane.

Protocol:

1. Extended routing outputs with explicit orchestration hierarchy metadata.

## 2026-03-06 22:10

Added:

1. Source-backed merge weighting.
2. `dispatch.env` support for provider-specific runtime environment variables.

Changed:

1. Started progress-aware runtime behavior with `useful_progress` tracking.

Protocol:

1. Updated subagent-system protocol to distinguish worker-entry, useful-progress, and merge-ready runtime states.
# 2026-03-07

- Added context governance ledger and protocol to classify local/runtime/web-validated sources with freshness and provenance metadata.
- Wired `prepare-execution` to emit context governance summaries and persist them into `.vida/state/context-governance.json`.
- Extended framework operator status and health checks to include context governance surfaces.
- Promoted `problem-party` into the run-graph model: receipts now mark `problem_party` completed and can advance `writer` to `ready`.
## 2026-03-07

1. Fixed [quality-health-check.sh](/home/unnamed/project/mobile-odoo/_vida/scripts/quality-health-check.sh) so execution-auth validation runs only for real `dev-pack` implementation context, not for reflection/docs tasks that merely share block ids or stray implementation receipts.
1. Expanded [framework-operator-status.py](/home/unnamed/project/mobile-odoo/_vida/scripts/framework-operator-status.py) to surface route rationale, estimated route cost classes, silent-diagnosis backlog, recent framework reflections, anomaly clusters, and suspicious run-graph artifacts.
2. Updated [framework-memory-protocol.md](/home/unnamed/project/mobile-odoo/_vida/docs/framework-memory-protocol.md), [context-governance-protocol.md](/home/unnamed/project/mobile-odoo/_vida/docs/context-governance-protocol.md), and [run-graph-protocol.md](/home/unnamed/project/mobile-odoo/_vida/docs/run-graph-protocol.md) to align canonical ledgers with the richer operator surface.
3. Added env-driven run-graph state-dir resolution in [run-graph.py](/home/unnamed/project/mobile-odoo/_vida/scripts/run-graph.py) and switched focused tests to explicit isolated overrides so run-graph tests no longer pollute real `.vida/state/run-graphs`.
4. Hardened [framework-wave-start.sh](/home/unnamed/project/mobile-odoo/_vida/scripts/framework-wave-start.sh) and [quality-health-check.sh](/home/unnamed/project/mobile-odoo/_vida/scripts/quality-health-check.sh) with JSONL-safe task label fallback when `br show` is degraded by malformed SQLite state.
5. Hardened [beads-workflow.sh](/home/unnamed/project/mobile-odoo/_vida/scripts/beads-workflow.sh) with named optional tail flags for `block-end` and `block-finish`, plus a `parse-block-tail` diagnostic command, so telemetry fields no longer depend on brittle positional shifting.
6. Tightened subagent-first orchestration law in [AGENTS.md](/home/unnamed/project/mobile-odoo/AGENTS.md), [ORCHESTRATOR-ENTRY.MD](/home/unnamed/project/mobile-odoo/_vida/docs/ORCHESTRATOR-ENTRY.MD), and [subagent-system-protocol.md](/home/unnamed/project/mobile-odoo/_vida/docs/subagent-system-protocol.md): agent/thread saturation now requires `reuse existing eligible agents first` before local-only continuation.
