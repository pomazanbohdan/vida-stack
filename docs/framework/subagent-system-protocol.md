# Subagent System Protocol (SSP)

Purpose: define one generic, portable protocol for subagent-system initialization, routing, fallback, and learning.

## Hard-Law Encoding

Mandatory routing policy is runtime law.

1. If route metadata marks `external_first_required=yes`, any direct internal bypass outside lawful escalation is invalid.
2. If route metadata marks `analysis_required=yes`, writer dispatch is invalid until the declared analysis phase completes and the analysis receipt exists or the runtime records an explicit blocker.
3. If route metadata marks `dispatch_required=fanout_then_synthesize`, single-lane dispatch is invalid unless the runtime is executing the declared bridge fallback or bounded arbitration path.
4. If route metadata marks `independent_verification_required=yes`, synthesis-ready state is invalid until a verifier plan and verifier artifact exist or the runtime records an explicit `no_eligible_verifier` blocker.
5. If route metadata marks `fanout_min_results > 0`, the fanout phase is incomplete until that threshold is met or the runtime records `subagent_exhausted`.
6. If route metadata marks `coach_required=yes` on a write-producing route, closure-ready state is invalid until a coach artifact exists with either `coach_approved` or a structured `return_for_rework` blocker.
7. Recommendations are permitted only for ranking/routing heuristics after all mandatory route laws are satisfied.

Cost-priority default:

1. free external read-only subagents should be the default first pass for eligible non-trivial read-heavy work,
2. for write-producing routes in `hybrid`, free external zero-budget analysis lanes should be the default first pass before any writer or bridge dispatch,
3. bridge fallback should be explicit and deterministic,
4. internal subagents should remain the senior lane under orchestrator control.

Debug-escalation default:

1. when repeated technical failures trigger `docs/framework/debug-escalation-protocol.md`, the default first parallel lane should include a bounded external catch/review agent when one is eligible,
2. this diagnostic lane is independent evidence, not writer ownership,
3. repeated local debugging without eligible external review should require an explicit blocker or exhaustion receipt.

## Scope

This protocol governs the system level above single-dispatch prompts:

1. activation,
2. subagent capability detection,
3. mode selection,
4. routing,
5. success/failure scoring,
6. escalation, promotion, and demotion.

Single-dispatch prompt contract stays in `docs/framework/subagents.md`.

Worker-lane entry contract stays in `docs/framework/SUBAGENT-ENTRY.MD`.

## Modes

Supported system modes:

1. `native`
   - use internal subagents only.
2. `hybrid`
   - use internal and external subagents according to routing policy.
3. `disabled`
   - do not use subagents.

Mode-synced execution rule:
1. `native`
   - internal subagents are the first eligible analysis/review lane and the first authorized development-support orchestration lane.
2. `hybrid`
   - external-first routing remains the default for eligible read-only work and the default first hop for development orchestration whenever route policy requires subagent-first execution.
3. `disabled`
   - no subagent-first requirement; the orchestrator may execute locally.

## Initialization Flow

Canonical runtime flow:

1. read `vida.config.yaml` through `docs/framework/project-overlay-protocol.md`,
2. check `protocol_activation.agent_system`,
3. detect configured subagents,
4. determine `requested_mode`,
5. compute `effective_mode`,
6. write runtime snapshot,
7. expose route/status helpers for the current session.

## Subagent Backend Classes

Framework subagent backend classes are generic:

1. `internal`
   - runtime-managed internal subagents.
2. `external_cli`
   - external CLI-driven agents/models.
3. `external_review`
   - independent validation/review lane.

Project docs/config may bind concrete subagents to these classes.

## State Ownership

Hard rule:

1. orchestrator owns `br` task state,
2. orchestrator owns TODO lifecycle,
3. orchestrator owns build/close/integration transitions,
4. subagents may only return artifacts/results unless explicitly granted bounded repo-write scope.
5. read-only lanes must not mutate framework-owned or project-owned workspace trees; runtime should fail closed on unauthorized writes under `AGENTS.md`, `docs/framework/history/_vida-source/*`, project docs/scripts/config roots, or application source trees.

## Entry Separation

Hard rule:

1. `AGENTS.md` is the orchestrator-only entry contract,
2. external and delegated workers must use `docs/framework/SUBAGENT-ENTRY.MD`,
3. do not proxy full orchestrator boot/governance language into worker prompts unless the task explicitly audits the framework layer,
4. worker prompts should optimize for bounded evidence delivery, not meta-orchestration narration.
5. worker prompts should carry explicit worker-lane confirmation markers so runtime role does not depend on repository-global instruction inheritance.

## Routing Contract

Routing input:

1. task class,
2. activated mode,
3. configured subagent order,
4. subagent availability,
5. subagent score state,
6. optional project overlay model policy for the chosen subagent,
7. optional project overlay profile policy for the chosen subagent,
8. route-level write and verification policy,
9. optional read-only fanout subagent set,
10. optional fanout minimum-result threshold,
11. optional merge policy for consensus/conflict handling.

Routing output:

1. chosen subagent,
2. selected model (when the subagent backend supports model selection),
3. selected profile (when the subagent backend supports profile selection),
4. reason,
5. effective score,
6. fallback subagents,
7. effective write scope,
8. verification gate,
9. effective route-law metadata:
   - `dispatch_required`
   - `external_first_required`
   - `bridge_fallback_subagent`
   - `internal_escalation_trigger`
   - `max_runtime_seconds`
   - `min_output_bytes`
10. effective independent-verification metadata:
   - `verification_route_task_class`
   - `independent_verification_required`
   - `verification_plan`
11. effective coach-review metadata for post-write routes:
   - `coach_required`
   - `coach_route_task_class`
   - `coach_plan`
12. effective analysis-phase metadata:
   - `analysis_required`
   - `analysis_route_task_class`
   - `analysis_plan`
   - `analysis_receipt_required`
   - `analysis_zero_budget_required`
13. effective dispatch-policy metadata:
   - `dispatch_policy.local_execution_allowed`
   - `dispatch_policy.local_execution_preferred`
   - `dispatch_policy.cli_dispatch_required_if_delegating`
   - `dispatch_policy.direct_internal_bypass_forbidden`
   - `dispatch_policy.internal_route_authorized`
   - `dispatch_policy.internal_escalation_allowed`
   - `dispatch_policy.allowed_internal_reasons`
   - `dispatch_policy.required_dispatch_path`
14. effective external-validation metadata:
   - `web_search_required`
15. optional ensemble/advisory metadata:
   - `fanout_subagents`
   - `fanout_min_results`
   - `merge_policy`
   - progress/timeout policy metadata
16. optional deterministic-route and FinOps metadata:
   - `route_graph`
   - `route_budget`

Effective-route law rule:

1. Config-level route declarations may omit law-bearing keys only when runtime defaulting/derivation will materialize them in the emitted route receipt.
2. Route receipts and effective routing output must expose explicit values for all law-bearing fields used by authorization, gating, or blocking logic.
3. Law-bearing fields include:
   - `dispatch_required`
   - `external_first_required`
   - `analysis_required`
   - `analysis_route_task_class`
   - `analysis_receipt_required`
   - `independent_verification_required`
   - `verification_route_task_class`
   - `coach_required`
   - `coach_route_task_class`
   - `dispatch_policy.*`
   - `web_search_required`
4. Omission semantics are therefore config-only, not runtime-authorization semantics.
5. If runtime cannot derive an explicit effective value for a law-bearing field before writer/verification/closure authorization, the route is protocol-invalid and must fail closed.

Ensemble rule:

1. On eligible read-only classes, `fanout_*` metadata is the default execution plan unless route metadata explicitly says otherwise.
2. Eligible classes are research, analysis, meta-analysis, review, and verification unless project overlay narrows them.
2.1. `read_only_prep` is the canonical prep-only route class for issue-contract drafting, bounded research, regression-plan prep, and other dependent-task preparation that must remain non-writing.
3. Keep writer ownership single-lane under the orchestrator even when read-only fanout is active.
4. `bridge_fallback_subagent` is the canonical next hop after free external subagents and before internal escalation.
5. Internal subagents are the senior lane for arbitration, architecture, and mutation-owning work; they are not the default cheap first pass for eligible read-only classes.
6. When `independent_verification_required=yes`, the runtime must choose a distinct eligible cli subagent or verification ensemble for validation before orchestrator synthesis whenever such a verifier exists; fallback to same-lane verification is lawful only under the explicit verifier-fallback rules in this protocol.
7. When mode is not `disabled`, eligible non-trivial read-heavy analysis should go to subagent lanes first; the orchestrator is the synthesizer and mutation owner, not the default primary analyst.
8. When mode is not `disabled`, development execution should be orchestrator-managed through the routed subagent system; local orchestrator-first development is not the default path.
8.1. When route metadata marks `web_search_required=yes`, the runtime must filter out subagents that do not both expose `capability_band=web_search` and declare dispatch-level web-search wiring.
8.2. `provider_configured` counts as operator-trusted wiring metadata at config time, but web-required execution should still require a bounded live web-search probe before the lane is treated as ready.
9. For write-producing routes in `hybrid`, the canonical path is `analysis -> writer -> coach -> verification` when `coach_required=yes`; otherwise it remains `analysis -> writer -> verification`. The writer lane is not authorized before the analysis receipt exists.
9.1. When the implementation request is issue-driven, the analysis phase must also emit a valid `issue_contract`; writer authorization is invalid until that artifact is `writer_ready`.
10. Raw subagent returns belong to the evidence layer; the default user-facing output is the orchestrator's synthesized conclusion.
11. Runtime artifacts must expose a machine-readable route receipt with the selected law-bearing fields before synthesis is considered valid.
12. Generic assistant defaults that would otherwise jump directly into local implementation are subordinate to this route contract while the subagent system is active.
13. A run is not mutation-authorized until the route receipt or lawful escalation receipt makes writer ownership and local-execution authorization explicit.
14. Undocumented execution behavior is forbidden by default while this system is active; route policy acts as an allowlist, not a set of suggestions.
15. "Optional" route fields in config/examples do not authorize missing values in the effective route receipt when those fields participate in law, gating, escalation, or closure decisions.

## Internal Escalation Boundary

Internal-subagent availability does not automatically authorize direct internal use.

Required distinctions:

1. `internal_primary`
   - route-selected primary use, valid in `native` and in task classes that explicitly authorize internal as primary.
2. `internal_escalation`
   - route-authorized escalation after external/bridge steps or other declared policy conditions.
3. `internal_bypass`
   - direct internal invocation outside the declared route authorization boundary.

Hybrid-mode rule:

1. when `external_first_required=yes`, direct internal bypass is forbidden when `dispatch_policy.direct_internal_bypass_forbidden=yes`,
2. internal escalation remains allowed when route metadata exposes `internal_escalation_allowed=yes`,
3. lawful internal escalation must carry a concise escalation receipt in run artifacts/logs.
4. a run that violates items 1-3 is protocol-invalid and should fail fast instead of degrading to advisory reporting.
5. lawful local-orchestrator mutation under active subagent mode must also carry a concise escalation receipt; otherwise the run remains routing-incomplete and must stay in orchestration mode.

## Independent Verification Contract

Independent verification is a first-class runtime artifact, not an ad hoc orchestrator habit.

Minimum contract:

1. eligible non-trivial work should separate authorship and verification when route policy requires it,
2. verification should be selected from a dedicated verification route class when possible,
3. the verifier should differ from the author/fanout lane when another eligible verifier exists,
4. fallback to the same cli subagent as verifier is allowed only when no other eligible verifier remains,
5. route output should expose the selected verifier plan so operator tooling and proving-wave scripts do not guess,
6. authored-result quality and verifier quality should both influence scorecards over time,
7. the orchestrator should synthesize and escalate; it should not be the default primary analyst and primary verifier for eligible lanes.
8. when `required=yes`, missing verification is a blocking state, not a soft warning.
9. when the active review target requires policy, senior, or human approval, technical verification alone is insufficient; runtime must apply the human-approval gate before closure-ready synthesis.

## Coach Review Contract

Coach review is the post-write formative gate for implementation routes.

Minimum contract:

1. `coach` runs after the writer phase and before final independent verification on routes that declare `coach_required=yes`,
2. `coach` checks the implementation against the original prompt/spec and either approves it for the final verifier or returns it for rework,
3. `coach` is not the final independent verifier and must not silently replace that role when `independent_verification_required=yes`,
4. the default coach path should use two independent cheaper coach lanes when the route exposes enough eligible coaches; runtime should treat coach as an ensemble/quorum artifact, not a single review opinion,
5. the safe default merge policy is `unanimous_approve_rework_bias`: approve only when the required coach quorum approves; any valid `return_for_rework` vote blocks advancement and must be merged into one fresh-start rework handoff,
6. `coach` output must be structured enough for runtime to distinguish `coach_approved` from `return_for_rework`,
7. each coach lane must judge readiness for final independent verification from its own lane only; pending parallel coach lanes are not blockers and must not force `merge_ready=no`,
8. environment/tool absence alone is not a coach blocker unless it proves a concrete implementation gap; lane-local tool limits should stay in verification notes/results, not silently flip approval into rework,
9. runtime may use an ordered feedback-extraction chain (`stdout json -> stderr json -> error/status json -> text fallbacks`) to recover coach evidence, but text-only fallback must never create a synthetic approval verdict,
10. structured rework handoffs must carry feedback provenance (`feedback_source` + `feedback_sources`) so the next writer pass can trace the origin of the coach delta,
11. `max_coach_passes` caps repeated coach-review loops,
12. `return_for_rework` must emit a structured rework-handoff artifact rooted in the original prompt/spec plus coach delta,
13. the next writer pass must consume that rework-handoff as a fresh-start packet instead of continuing prior writer context by default,
14. contradictory coach finality payloads are protocol-invalid and must normalize to an explicit `invalid_coach_payload*` failure state instead of being silently coerced,
15. missing coach artifacts on a coach-required route are blocking, not advisory.

Suggested route split:

1. analysis/research/meta-analysis -> `verification_ensemble`,
2. write-producing bounded lanes -> `review_ensemble`,
3. review/verification lanes themselves may skip a second independent verifier unless project overlay asks for it.

## Ensemble Merge Semantics

When route metadata includes read-only ensemble fanout, the orchestrator owns merge behavior.

Minimum merge contract:

1. dispatch only the declared `fanout_subagents`,
2. stop only after at least `fanout_min_results` valid returns or an explicit subagent-exhausted state,
3. deduplicate materially identical findings before synthesis,
4. separate exact consensus from normalized semantic consensus before synthesis,
5. separate `consensus`, `unique_findings`, and `open_conflicts`,
6. emit a tie-break signal when semantic conflict remains decision-relevant,
7. keep raw subagent disagreement out of the final answer unless it remains decision-relevant,
8. keep raw subagent report bodies out of the default final answer unless the user explicitly asks to inspect them.
9. a merge summary that does not satisfy items 1-2 is not synthesis-ready.

`merge_policy=consensus_with_conflict_flag` means:

1. prefer points independently supported by multiple subagents,
2. surface unresolved conflicts explicitly instead of averaging them away,
3. inject an internal review or architecture lane only when:
   - the conflict affects the decision path,
   - evidence quality differs materially across subagents,
   - `fanout_min_results` is not met,
   - or the orchestrator confidence remains below the active task threshold,
4. keep the orchestrator as the final synthesizer even after tie-break review.

Reporting boundary:

1. Subagent responses are synthesis inputs, not default deliverables.
2. The orchestrator should answer the user in its own voice using merged findings and cited evidence.
3. If the user asks to see a subagent report, provide it explicitly as an inspection artifact rather than mixing it into the default final answer.

Efficiency rule:

1. do not exceed project `max_parallel_agents`,
2. do not re-run equivalent subagents when current consensus is already sufficient,
3. escalate only on unresolved decision-critical conflict, not on stylistic variance.

## Deterministic Route Graph

Routing output should expose a compact deterministic route artifact so orchestration is inspectable without replaying the whole decision.

Minimum contract:

1. `route_graph.graph_strategy` should describe the route family, for example `deterministic_then_escalate`,
2. `route_graph.deterministic_first` should indicate whether the route is intended to stay on deterministic workflow edges until evidence forces escalation,
3. `route_graph.nodes` should identify the primary dispatch lane, bridge fallback, internal escalation, optional coach lane, verification lane, and orchestrator synthesis node when relevant,
4. `route_graph.edges` should describe the escalation/coach/verification conditions,
5. `route_graph.planned_path` should give operators one compact intended execution path.

## Task-Level FinOps Budget

Routing and dispatch should expose a bounded budget object for each task class rather than relying only on raw timeouts.

Minimum contract:

1. `route_budget.budget_policy` should declare how aggressive cost minimization is for the route,
2. `route_budget.max_budget_units` should cap the intended route cost using normalized subagent cost units,
3. `route_budget.max_cli_subagent_calls` should cap total cli-subagent dispatches for the route,
4. `route_budget.max_coach_passes` should cap coach reruns on coach-required routes,
5. `route_budget.max_verification_passes` should cap verification reruns/extra verification lanes,
5. `route_budget.max_fallback_hops` should cap bridge/internal escalation depth,
6. `route_budget.max_total_runtime_seconds` should cap the whole route, not only individual subagents,
7. route selection should prefer lower-cost eligible cli subagents when quality signals are near-equivalent,
8. route selection may still escalate above budget when policy explicitly requires bridge/internal escalation for safety or verification.

Budget-observability minimum:

1. run artifacts should distinguish cheap-lane attempts, bridge fallback, lawful internal escalation, and policy bypass,
2. lawful internal escalation should record a structured escalation trigger/receipt,
3. diagnostics should expose budget violations and internal-escalation receipts separately from normal provider failures.

## Bounded Arbitration Lane

When `merge_summary.tie_break_reason=semantic_conflict_without_majority`, the orchestrator may run one bounded arbitration lane.

Arbitration contract:

1. run at most one additional subagent,
2. prefer an unused eligible read-only subagent; if none exists, allow one deterministic rerun of the best remaining supported subagent,
3. pass only the original prompt plus the conflicting semantic clusters,
4. require the arbitrator to choose one existing cluster or return `no_decision`,
5. do not trigger a second fanout wave or unbounded retries,
6. keep final synthesis ownership under the orchestrator.

Manifest contract:

1. record an `arbitration` object with trigger, selected subagent, reuse status, execution status, and parsed decision,
2. record `post_arbitration_merge_summary` separately from the original `merge_summary`,
3. allow `post_arbitration_merge_summary.consensus_mode` to resolve to `arbitrated` or `inconclusive`,
4. preserve the original read-only fanout results as the canonical base evidence instead of counting the arbitrator as a normal fanout vote.

Runtime expectation:

1. route output should expose `max_parallel_agents` and `state_owner`,
2. ensemble dispatch manifest should record `requested_fanout_subagents`, effective `fanout_subagents`, `subagent_exhausted`, and `merge_summary`,
3. `merge_summary` should expose at least `agreements`, `semantic_agreements`, `unique_findings`, `open_conflicts`, `consensus_mode`, and whether orchestrator review is required,
4. `merge_summary` should distinguish `exact_consensus`, `semantic_consensus`, and `semantic_majority`,
5. `merge_summary` should emit `tie_break_recommended` and `tie_break_reason` when the orchestrator needs an additional arbitration lane or human decision,
6. bounded arbitration runs should emit `arbitration` and `post_arbitration_merge_summary` in the manifest when arbitration was attempted,
7. route output should expose whether external-first dispatch is required, which bridge fallback is canonical, and what condition authorizes internal escalation,
8. subagent run artifacts should distinguish command success from `merge_ready`,
9. subagent run artifacts should distinguish `merge_ready` from `useful_progress`,
10. dynamic scorecards should remain visible by task class and inferred domain.
11. subagent run artifacts should distinguish low-value planning chatter from evidence-bearing analysis.
12. ensemble manifest should expose `active_subagents` and `active_count` while fanout is still running so operator visibility does not wait for the first completed result.
13. ensemble dispatch should expose a bounded lease/ownership artifact for the active orchestration lane.

## Progress-Aware Runtime

Timeout policy must not be based only on a single wall-clock limit.

Minimum runtime contract:

1. track `useful_progress` separately from `merge_ready`,
2. capture `time_to_first_useful_output_ms` when possible,
3. allow at most one bounded runtime extension for subagents that are still making useful progress,
4. deny repeated unbounded extensions,
5. terminate subagents that are idle or stuck even if they previously produced low-value chatter,
6. prefer subagent-specific runtime budgets over one global timeout.

Progress taxonomy:

1. `boot_progress`
   - startup/sandbox/session initialization only,
2. `read_progress`
   - file reads/searches without findings yet,
3. `useful_progress`
   - evidence-bearing findings or structured analytical movement,
4. `merge_ready`
   - output strong enough to participate in final ensemble synthesis.
5. `chatter_only`
   - verbose planning/process narration without findings, evidence, or synthesis-ready content.

Phase-aware timeout policy:

1. `startup_timeout_seconds`
   - maximum time with no output at all after process launch,
2. `no_output_timeout_seconds`
   - maximum idle/no-progress time before useful progress appears,
3. `progress_idle_timeout_seconds`
   - maximum idle time after useful progress has already appeared,
4. `max_runtime_extension_seconds`
   - one bounded extension cap when useful progress is still active near the wall-clock limit.

Parity rule:

1. phase-aware timeout handling must apply to both ensemble fanout lanes and single-run fallback/single dispatch lanes,
2. single-run fallback must not silently collapse back to one coarse wall-clock timeout.

Default timeout behavior:

1. cut startup-stalled cli subagents before the full runtime budget is burned,
2. cut no-output/no-progress cli subagents before bridge fallback is delayed unnecessarily,
3. allow one bounded extension only for evidence-bearing runs,
4. emit distinct failure reasons for:
   - `startup_timeout`
   - `no_output_timeout`
   - `stalled_after_progress`
   - `runtime_unstable`

Behavioral guard:

1. repeated `chatter_only` without any useful progress should degrade the cli subagent for critical lanes,
2. degraded-on-chatter subagents should require probe or cooldown before rejoining core fanout,
3. chatter penalties in scorecards are not sufficient by themselves for repeated non-evidence behavior.

## Subagent Availability And Recovery

Subagent quality and subagent availability are separate signals.

Minimum availability contract:

1. runtime should track subagent availability independently from score,
2. canonical availability states:
   - `active`
   - `degraded`
   - `quota_exhausted`
   - `disabled_manual`
3. temporary subagent suppression should use `cooldown_until`,
4. subagents that hit daily quota or temporary rate limits should be automatically excluded from routing until cooldown expires,
5. subagents may return through a bounded `probe` path before rejoining critical fanout lanes,
6. probe success should restore `subagent_state=active`,
7. probe failure should update availability state without fabricating a quality success.
8. repeated `chatter_only` behavior may trigger temporary degraded state even when the cli subagent command itself exits successfully.
9. `auth_invalid` and `interactive_blocked` should suppress the cli subagent from routing until explicit repair and successful recovery.
10. routing should expose `suppressed_subagents` with reasons when availability rules filter candidates out.
11. operator status should expose actionable remediation hints, not only raw degraded state.
12. operator status should expose recovery history and task-class readiness, not only global score.
13. operator status should expose lifecycle stage (`detected|probed|probation|promoted|degraded|cooldown|recovered|retired`) per cli subagent.

Recovery-aware routing rule:

1. successful recent recovery may soften a prior demotion only when the cli subagent is currently `active`,
2. repeated failed recoveries should reduce routing confidence and may keep prior demotion in force.

Failure-reason examples:

1. `daily_quota_exhausted`
2. `rate_limited`
3. `auth_invalid`
4. `interactive_blocked`
5. `runtime_unstable`

## Learning Contract

Every subagent backend has:

1. global score,
2. per-task-class score,
3. consecutive failure counter,
4. success count,
5. failure count,
6. state (`preferred|normal|demoted`).

Scorecards should evolve toward:

1. merge-ready rate,
2. useful-progress rate,
3. time-to-first-useful-output,
4. timeout-after-progress rate,
5. fallback dependence rate,
6. chatter-only rate,
7. subagent-availability stability,
8. recovery attempts/successes,
9. per-domain usefulness.
10. authored-result verification pass/fail history,
11. verifier success/catch history.

## Execution Memory And Long-Horizon Routing

Scorecards are not sufficient as the only memory layer.

Minimum execution-memory contract:

1. persist a strategy snapshot derived from accumulated subagent runs,
2. carry forward memory hints per task class:
   - `preferred_subagents`
   - `avoid_subagents`
   - `retry_useful_subagents`
   - `failure_prone_subagents`
3. track prompt-family effectiveness where practical so repeated prompt patterns can be compared over time,
4. track recurring failure signatures per subagent beyond one latest failure reason,
5. allow routing to consume bounded memory adjustments from strategy state, not only raw scorecards,
6. keep memory-derived routing influence bounded so recent short-horizon evidence can still override stale history.

Runtime expectation:

1. strategy state should be written to `.vida/state/subagent-strategy.json`,
2. `subagent-eval-pack.py` should refresh this state after task-close or explicit evaluation runs,
3. `route` output should expose when long-horizon memory changed the selected ordering or score.

Review-state distinction:

1. per-subagent run review state should describe the review gate reached by that one run:
   - `review_passed`
   - `policy_gate_required`
   - `senior_review_required`
   - `human_gate_required`
2. manifest/task-level review state may advance further to:
   - `promotion_ready`
   once ensemble synthesis is decision-ready and no further review gate remains for the active risk class.
3. when the manifest/task-level review state remains `policy_gate_required`, `senior_review_required`, or `human_gate_required`, closure-ready state still requires a matching approval receipt under `docs/framework/human-approval-protocol.md`.

Lane-aware promotion/demotion rule:

1. demotion may apply globally or per task class,
2. a cli subagent demoted for the active task class should be suppressed from that lane even if its global score is still acceptable,
3. globally demoted cli subagents should stay out of core fanout unless they are:
   - the explicit bridge fallback,
   - the internal senior lane,
   - or later re-promoted by evidence.
4. probationary cli subagents may participate only in bounded low-risk lanes until lane-specific evidence promotes them.
5. promoted state should be tracked independently per task class when practical.

## Lease / Ownership Runtime

Minimum lease contract:

1. parallel cli-subagent orchestration should acquire one bounded lease for the active ensemble lane,
2. the lease should carry:
   - resource type
   - resource id
   - holder
   - expiry
   - fencing token
3. a conflicting active lease should block a second overlapping ensemble on the same resource,
4. successful close should release the lease and record release status in the manifest,
5. operator tooling should expose active leases as part of runtime diagnostics,
6. lease conflicts should be written to lease history so recent orchestration contention is visible to the operator.
7. runtime should support lease renewal for longer active orchestrations instead of assuming one fixed acquire/release window,
8. runtime should expose cleanup/expiry semantics so stale released/expired leases do not accumulate indefinitely.
9. reusable read-only subagent pooling may be layered on top of the same lease ledger; pool borrow/release must reuse the canonical lease system instead of inventing a second ownership store.

Saturation fallback rule:

1. if subagent-first execution is active and the orchestrator cannot allocate a new delegated lane because agent/thread capacity is saturated, the next action is not immediate local-only continuation.
2. the orchestrator must first try one of:
   - reuse an existing eligible agent/thread,
   - release or close an idle agent/thread and retry bounded delegation,
   - or record an explicit saturation blocker when neither reuse nor cleanup is possible.
3. local-only continuation after saturation is lawful only after one of those reuse/recovery paths has been attempted and the outcome is inspectable.

## Operator Visibility

Operator surfaces should expose more than raw health.

Minimum operator summary:

1. current subagent availability and remediation hints,
2. preferred and eligible task classes,
3. recent recoveries with status and timestamp,
4. unstable timeout classes by cli subagent:
   - `startup_timeout_count`
   - `no_output_timeout_count`
   - `stalled_after_progress_count`
5. review-target map by task class:
   - `risk_class`
   - `target_review_state`
   - `target_manifest_review_state`
   - `verification_gate`
   - `write_scope`
6. recent lease-conflict summary from the lease ledger.
7. one-shot diagnosis surface that aggregates alerts, remediation, timeout instability, and lane-readiness without requiring manual JSON synthesis.
8. lease summary should distinguish `active`, `released`, and `expired`, and group recent conflicts by resource when possible.

## Escalation And Adaptation

Failure path:

1. subagent failure increments `consecutive_failures`,
2. score decreases,
3. after threshold, subagent is demoted for affected task classes,
4. router prefers the next eligible subagent,
5. runtime records the downgrade decision.

Success path:

1. repeated good outcomes increase score,
2. score crossing promotion threshold marks subagent as `preferred`,
3. router may choose it more often for the relevant task classes.

## Minimum Thresholds

Current portable defaults:

1. `consecutive_failure_limit = 5`
2. `promotion_score = 80`
3. `demotion_score = 35`
4. `probation_success_runs = 3`
5. `probation_task_runs = 1`
6. `retirement_failure_limit = 12`

Project overlay may override these values.

## Runtime Commands

Canonical helpers:

```bash
vida-v0 system snapshot [task_id]
vida-v0 system detect
vida-v0 system mode
vida-v0 system budget-summary [task_class]
vida-v0 registry build
vida-v0 registry check <task_class> <subagent>
vida-v0 lease acquire <resource_type> <resource_id> <holder> [--ttl-seconds N]
vida-v0 lease renew <resource_type> <resource_id> <holder> [--ttl-seconds N]
vida-v0 lease release <resource_type> <resource_id> <holder>
vida-v0 lease list
vida-v0 pool borrow <task_class> <holder> [--ttl-seconds N]
vida-v0 pool release <subagent> <holder>
vida-v0 pool status
vida-v0 prepare-execution <task_id> <writer_task_class> <prompt_file> <output_dir> [workdir]
```

Runtime notes:

1. Read-only external CLI dispatches may borrow/release `subagent_pool` leases automatically; the helper remains the explicit operator surface.
2. Provider-configured web-search lanes must fail closed to fallback when the live web probe does not clear.

## Verification

Minimum proof:

1. initialization writes snapshot,
2. routing returns an eligible subagent or explicit disabled reason,
3. repeated failures change subagent state to `demoted`,
4. repeated successes can promote a subagent to `preferred`,
5. task-close evaluation refreshes `.vida/state/subagent-strategy.json` when subagent runs were used.
