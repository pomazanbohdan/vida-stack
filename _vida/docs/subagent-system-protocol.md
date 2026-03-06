# Subagent System Protocol (SSP)

Purpose: define one generic, portable protocol for subagent-system initialization, routing, fallback, and learning.

Cost-priority default:

1. free external read-only providers should be the default first pass for eligible non-trivial read-heavy work,
2. bridge fallback should be explicit and deterministic,
3. internal providers should remain the senior lane under orchestrator control.

## Scope

This protocol governs the system level above single-dispatch prompts:

1. activation,
2. provider capability detection,
3. mode selection,
4. routing,
5. success/failure scoring,
6. escalation, promotion, and demotion.

Single-dispatch prompt contract stays in `_vida/docs/subagents.md`.

## Modes

Supported system modes:

1. `native`
   - use internal subagents only.
2. `hybrid`
   - use internal and external providers according to routing policy.
3. `disabled`
   - do not use subagents.

## Initialization Flow

Canonical runtime flow:

1. read `vida.config.yaml` through `_vida/docs/project-overlay-protocol.md`,
2. check `protocol_activation.agent_system`,
3. detect configured providers,
4. determine `requested_mode`,
5. compute `effective_mode`,
6. write runtime snapshot,
7. expose route/status helpers for the current session.

## Provider Classes

Framework provider classes are generic:

1. `internal`
   - runtime-managed internal subagents.
2. `external_cli`
   - external CLI-driven agents/models.
3. `external_review`
   - independent validation/review lane.

Project docs/config may bind concrete providers to these classes.

## State Ownership

Hard rule:

1. orchestrator owns `br` task state,
2. orchestrator owns TODO lifecycle,
3. orchestrator owns build/close/integration transitions,
4. providers may only return artifacts/results unless explicitly granted bounded repo-write scope.

## Routing Contract

Routing input:

1. task class,
2. activated mode,
3. configured provider order,
4. provider availability,
5. provider score state,
6. optional project overlay model policy for the chosen provider,
7. optional project overlay profile policy for the chosen provider,
8. route-level write and verification policy,
9. optional read-only fanout provider set,
10. optional fanout minimum-result threshold,
11. optional merge policy for consensus/conflict handling.

Routing output:

1. chosen provider,
2. selected model (when the provider supports model selection),
3. selected profile (when the provider supports profile selection),
4. reason,
5. effective score,
6. fallback chain,
7. effective write scope,
8. verification gate,
9. optional `fanout_providers` for orchestrator-managed read-only ensemble dispatch,
10. optional `fanout_min_results`,
11. optional `merge_policy`,
12. optional `dispatch_required`,
13. optional `external_first_required`,
14. optional `bridge_fallback_provider`,
15. optional `internal_escalation_trigger`,
16. optional `max_runtime_seconds`,
17. optional `min_output_bytes`.

Ensemble rule:

1. On eligible read-only classes, `fanout_*` metadata is the default execution plan unless route metadata explicitly says otherwise.
2. Eligible classes are research, analysis, meta-analysis, review, and verification unless project overlay narrows them.
3. Keep writer ownership single-lane under the orchestrator even when read-only fanout is active.
4. `bridge_fallback_provider` is the canonical next hop after free external providers and before internal escalation.
5. Internal providers are the senior lane for arbitration, architecture, and mutation-owning work; they are not the default cheap first pass for eligible read-only classes.

## Ensemble Merge Semantics

When route metadata includes read-only ensemble fanout, the orchestrator owns merge behavior.

Minimum merge contract:

1. dispatch only the declared `fanout_providers`,
2. stop only after at least `fanout_min_results` valid returns or an explicit provider-exhausted state,
3. deduplicate materially identical findings before synthesis,
4. separate exact consensus from normalized semantic consensus before synthesis,
5. separate `consensus`, `unique_findings`, and `open_conflicts`,
6. emit a tie-break signal when semantic conflict remains decision-relevant,
5. keep raw provider disagreement out of the final answer unless it remains decision-relevant.

`merge_policy=consensus_with_conflict_flag` means:

1. prefer points independently supported by multiple providers,
2. surface unresolved conflicts explicitly instead of averaging them away,
3. inject an internal review or architecture lane only when:
   - the conflict affects the decision path,
   - evidence quality differs materially across providers,
   - `fanout_min_results` is not met,
   - or the orchestrator confidence remains below the active task threshold,
4. keep the orchestrator as the final synthesizer even after tie-break review.

Efficiency rule:

1. do not exceed project `max_parallel_agents`,
2. do not re-run equivalent providers when current consensus is already sufficient,
3. escalate only on unresolved decision-critical conflict, not on stylistic variance.

## Bounded Arbitration Lane

When `merge_summary.tie_break_reason=semantic_conflict_without_majority`, the orchestrator may run one bounded arbitration lane.

Arbitration contract:

1. run at most one additional provider,
2. prefer an unused eligible read-only provider; if none exists, allow one deterministic rerun of the best remaining supported provider,
3. pass only the original prompt plus the conflicting semantic clusters,
4. require the arbitrator to choose one existing cluster or return `no_decision`,
5. do not trigger a second fanout wave or unbounded retries,
6. keep final synthesis ownership under the orchestrator.

Manifest contract:

1. record an `arbitration` object with trigger, selected provider, reuse status, execution status, and parsed decision,
2. record `post_arbitration_merge_summary` separately from the original `merge_summary`,
3. allow `post_arbitration_merge_summary.consensus_mode` to resolve to `arbitrated` or `inconclusive`,
4. preserve the original read-only fanout results as the canonical base evidence instead of counting the arbitrator as a normal fanout vote.

Runtime expectation:

1. route output should expose `max_parallel_agents` and `state_owner`,
2. ensemble dispatch manifest should record `requested_fanout_providers`, effective `fanout_providers`, `provider_exhausted`, and `merge_summary`,
3. `merge_summary` should expose at least `agreements`, `semantic_agreements`, `unique_findings`, `open_conflicts`, `consensus_mode`, and whether orchestrator review is required,
4. `merge_summary` should distinguish `exact_consensus`, `semantic_consensus`, and `semantic_majority`,
5. `merge_summary` should emit `tie_break_recommended` and `tie_break_reason` when the orchestrator needs an additional arbitration lane or human decision,
6. bounded arbitration runs should emit `arbitration` and `post_arbitration_merge_summary` in the manifest when arbitration was attempted,
7. route output should expose whether external-first dispatch is required, which bridge fallback is canonical, and what condition authorizes internal escalation,
8. provider run artifacts should distinguish command success from `merge_ready`,
9. dynamic scorecards should remain visible by task class and inferred domain.

## Learning Contract

Every provider has:

1. global score,
2. per-task-class score,
3. consecutive failure counter,
4. success count,
5. failure count,
6. state (`preferred|normal|demoted`).

## Escalation And Adaptation

Failure path:

1. provider failure increments `consecutive_failures`,
2. score decreases,
3. after threshold, provider is demoted for affected task classes,
4. router prefers the next eligible provider,
5. runtime records the downgrade decision.

Success path:

1. repeated good outcomes increase score,
2. score crossing promotion threshold marks provider as `preferred`,
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
python3 _vida/scripts/subagent-system.py route <task_class>
python3 _vida/scripts/subagent-system.py record <provider> <success|failure> <task_class> [quality_score] [latency_ms] [note]
python3 _vida/scripts/subagent-system.py scorecard [provider]
python3 _vida/scripts/subagent-dispatch.py provider <task_id> <task_class> <provider> <prompt_file> <output_file> [workdir]
python3 _vida/scripts/subagent-dispatch.py ensemble <task_id> <task_class> <prompt_file> <output_dir> [workdir]
python3 _vida/scripts/subagent-eval-pack.py run <task_id>
```

## Verification

Minimum proof:

1. initialization writes snapshot,
2. routing returns an eligible provider or explicit disabled reason,
3. repeated failures change provider state to `demoted`,
4. repeated successes can promote a provider to `preferred`,
5. task-close evaluation refreshes `.vida/state/subagent-strategy.json` when provider runs were used.
