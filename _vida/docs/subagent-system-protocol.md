# Subagent System Protocol (SSP)

Purpose: define one generic, portable protocol for subagent-system initialization, routing, fallback, and learning.

Cost-priority default:

1. free external read-only subagents should be the default first pass for eligible non-trivial read-heavy work,
2. bridge fallback should be explicit and deterministic,
3. internal subagents should remain the senior lane under orchestrator control.

## Scope

This protocol governs the system level above single-dispatch prompts:

1. activation,
2. subagent capability detection,
3. mode selection,
4. routing,
5. success/failure scoring,
6. escalation, promotion, and demotion.

Single-dispatch prompt contract stays in `_vida/docs/subagents.md`.

Worker-lane entry contract stays in `_vida/docs/SUBAGENT-ENTRY.MD`.

## Modes

Supported system modes:

1. `native`
   - use internal subagents only.
2. `hybrid`
   - use internal and external subagents according to routing policy.
3. `disabled`
   - do not use subagents.

## Initialization Flow

Canonical runtime flow:

1. read `vida.config.yaml` through `_vida/docs/project-overlay-protocol.md`,
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

## Entry Separation

Hard rule:

1. `AGENTS.md` is the orchestrator-only entry contract,
2. external and delegated workers must use `_vida/docs/SUBAGENT-ENTRY.MD`,
3. do not proxy full orchestrator boot/governance language into worker prompts unless the task explicitly audits the framework layer,
4. worker prompts should optimize for bounded evidence delivery, not meta-orchestration narration.

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
9. optional `fanout_subagents` for orchestrator-managed read-only ensemble dispatch,
10. optional `fanout_min_results`,
11. optional `merge_policy`,
12. optional `dispatch_required`,
13. optional `external_first_required`,
14. optional `bridge_fallback_subagent`,
15. optional `internal_escalation_trigger`,
16. optional `max_runtime_seconds`,
17. optional `min_output_bytes`,
18. optional progress/timeout policy metadata.

Ensemble rule:

1. On eligible read-only classes, `fanout_*` metadata is the default execution plan unless route metadata explicitly says otherwise.
2. Eligible classes are research, analysis, meta-analysis, review, and verification unless project overlay narrows them.
3. Keep writer ownership single-lane under the orchestrator even when read-only fanout is active.
4. `bridge_fallback_subagent` is the canonical next hop after free external subagents and before internal escalation.
5. Internal subagents are the senior lane for arbitration, architecture, and mutation-owning work; they are not the default cheap first pass for eligible read-only classes.

## Ensemble Merge Semantics

When route metadata includes read-only ensemble fanout, the orchestrator owns merge behavior.

Minimum merge contract:

1. dispatch only the declared `fanout_subagents`,
2. stop only after at least `fanout_min_results` valid returns or an explicit subagent-exhausted state,
3. deduplicate materially identical findings before synthesis,
4. separate exact consensus from normalized semantic consensus before synthesis,
5. separate `consensus`, `unique_findings`, and `open_conflicts`,
6. emit a tie-break signal when semantic conflict remains decision-relevant,
5. keep raw subagent disagreement out of the final answer unless it remains decision-relevant.

`merge_policy=consensus_with_conflict_flag` means:

1. prefer points independently supported by multiple subagents,
2. surface unresolved conflicts explicitly instead of averaging them away,
3. inject an internal review or architecture lane only when:
   - the conflict affects the decision path,
   - evidence quality differs materially across subagents,
   - `fanout_min_results` is not met,
   - or the orchestrator confidence remains below the active task threshold,
4. keep the orchestrator as the final synthesizer even after tie-break review.

Efficiency rule:

1. do not exceed project `max_parallel_agents`,
2. do not re-run equivalent subagents when current consensus is already sufficient,
3. escalate only on unresolved decision-critical conflict, not on stylistic variance.

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
7. subagent-availability stability.
6. per-domain usefulness.

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

Project overlay may override these values.

## Runtime Commands

Canonical helpers:

```bash
python3 _vida/scripts/subagent-system.py init [task_id]
python3 _vida/scripts/subagent-system.py status
python3 _vida/scripts/subagent-system.py subagents
python3 _vida/scripts/subagent-system.py route <task_class>
python3 _vida/scripts/subagent-system.py probe <subagent>
python3 _vida/scripts/subagent-system.py recover <subagent>
python3 _vida/scripts/subagent-system.py recover-pending
python3 _vida/scripts/subagent-system.py record <subagent> <success|failure> <task_class> [quality_score] [latency_ms] [note]
python3 _vida/scripts/subagent-system.py scorecard [subagent]
python3 _vida/scripts/subagent-dispatch.py subagent <task_id> <task_class> <subagent> <prompt_file> <output_file> [workdir]
python3 _vida/scripts/subagent-dispatch.py ensemble <task_id> <task_class> <prompt_file> <output_dir> [workdir]
python3 _vida/scripts/subagent-eval-pack.py run <task_id>
```

## Verification

Minimum proof:

1. initialization writes snapshot,
2. routing returns an eligible subagent or explicit disabled reason,
3. repeated failures change subagent state to `demoted`,
4. repeated successes can promote a subagent to `preferred`,
5. task-close evaluation refreshes `.vida/state/subagent-strategy.json` when subagent runs were used.
