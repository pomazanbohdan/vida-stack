# Subagent Onboarding Protocol

Purpose: define the canonical lifecycle for adding, probing, promoting, degrading, cooling down, and recovering external CLI subagents in VIDA.

## Scope

This protocol applies to cli subagent onboarding inside:

1. `vida.config.yaml`
2. `_vida/templates/vida.config.yaml.template`
3. `_vida/scripts/subagent-system.py`
4. `_vida/scripts/subagent-dispatch.py`
5. `_vida/scripts/subagent-eval-pack.py`

## Lifecycle

Canonical lifecycle:

1. `declared`
   - cli subagent exists in overlay/template,
   - `detect_command` and `dispatch` contract are present.
2. `detected`
   - runtime confirms the CLI exists locally.
3. `probed`
   - cli subagent passes a bounded headless smoke check.
4. `probation`
   - cli subagent may be used in bounded read-only lanes while history is still shallow.
5. `promoted`
   - cli subagent has real successful runs in the target lane and may join regular fanout.
6. `degraded`
   - cli subagent remains usable but should be downweighted or withheld from critical fanout.
7. `cooldown`
   - cli subagent is temporarily excluded from routing until `cooldown_until`.
8. `recovered`
   - cli subagent re-enters via probe or successful post-cooldown run.
9. `retired`
   - cli subagent is intentionally removed from active routing, typically by overlay disable or manual retirement.

## Minimum Onboarding Contract

When adding a new cli subagent:

1. declare the cli subagent in `vida.config.yaml`,
2. mirror the cli subagent in `_vida/templates/vida.config.yaml.template`,
3. declare realistic runtime limits:
   - `max_runtime_seconds`
   - `min_output_bytes`
4. declare `dispatch` fields that actually work in headless mode,
5. add probe settings when supported:
   - `probe_static_args`
   - `probe_prompt`
   - `probe_expect_substring`
   - `probe_timeout_seconds`
6. validate config before use,
7. run subagent probe before promoting the cli subagent into critical fanout.

## Probe Contract

Canonical probe command:

```bash
python3 _vida/scripts/subagent-system.py probe <subagent>
```

Probe requirements:

1. bounded runtime only,
2. no project mutation,
3. explicit success expectation when possible,
4. probe updates subagent availability but does not fabricate quality success,
5. failed probe must not silently leave cli subagent in `active` state.

Recovery helpers:

```bash
python3 _vida/scripts/subagent-system.py recover <subagent>
python3 _vida/scripts/subagent-system.py recover-pending
```

Recovery rules:

1. healthy cli subagents should return `noop` instead of re-probing unnecessarily,
2. cooldown-blocked cli subagents should report `blocked` until cooldown expires,
3. degraded or probe-required cli subagents should re-enter only through bounded recovery/probe flow,
4. recovery must update availability state without inventing a quality win.

## Promotion Rules

Promotion into regular fanout should require:

1. successful probe,
2. at least one real successful run in the target lane when practical,
3. no active cooldown,
4. no persistent `interactive_blocked` or `auth_invalid` reason,
5. acceptable chatter/failure behavior for the lane.
6. lane-specific probation must resolve into `promoted` before the cli subagent joins critical fanout for that lane.

## Degradation And Cooldown

Canonical temporary failure reasons:

1. `daily_quota_exhausted`
2. `rate_limited`
3. `auth_invalid`
4. `interactive_blocked`
5. `runtime_unstable`

Rules:

1. quota or rate-limit failures should suppress the cli subagent until `cooldown_until`,
2. interactive/headless failures should prefer degrade + probe-before-return,
3. runtime instability should not be treated as a quality success,
4. subagent quality score and subagent availability must remain separate signals.
5. `auth_invalid` and `interactive_blocked` should be treated as routing blockers until repaired and recovered.

## Lane Fitness

Cli subagents are not universally good or bad.

Lane fitness should be evaluated independently for:

1. `analysis`
2. `review`
3. `meta_analysis`
4. `verification`
5. `implementation` when applicable

Runtime should prefer task-class success history over global reputation when both exist.

## Lifecycle Runtime Expectations

Minimum runtime/operator surface should expose:

1. global `lifecycle_stage` for each cli subagent,
2. lane-specific lifecycle stage for task classes with real history,
3. probation thresholds before a cli subagent becomes fully promoted,
4. retirement visibility for manually disabled cli subagents,
5. routing suppression for retired cli subagents.

## Anti-Patterns

1. promoting a cli subagent into critical fanout from `detect_command` alone,
2. treating probe success as equivalent to deep lane success,
3. keeping quota-exhausted subagents in active routing,
4. mixing planning chatter with evidence-bearing progress,
5. storing onboarding policy only in changelog or chat instead of canonical protocol/docs.
