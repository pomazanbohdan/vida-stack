# Algorithms Quick Reference

Purpose: compress the algorithm descriptions without losing their essence by keeping explicit triggers, quality gates, and escalation rules.

## Unified Matrix

| Algorithm | When To Use | Mandatory Steps (minimum) | Quality Gate | Escalation |
|---|---|---|---|---|
| STC | Selector score `<=12`; low complexity, local tasks | Step -> check -> localize -> rollback -> retry (<=3) | No unresolved step errors remain | After 3 retries -> PR-CoT; protocol/route ambiguity -> META |
| PR-CoT | Selector score `13-22`; medium complexity, independent perspectives needed | Pass1 (4 perspectives) -> consensus -> Pass2 (revision) | No unresolved critical findings remain | Unresolved critical or >=2 issues after Pass2 -> MAR |
| MAR | Selector score `23-32`; complex tasks with regression risk | 3 rounds (Actor/Evaluator/Critic/Reflector) | Weighted rubric score >= 8/10 and no unresolved critical residual risk | <8 after 3 rounds -> META |
| 5-SOL | Selector score `33-42`; choice between alternatives / design directions | R1: 5 options -> weighted option ledger -> R2: 5 new options -> legal hybrid/top option | Admissible choice with weighted option score + confidence >= 80, or explicit cautious band | Low score/confidence or legality pressure -> META |
| META | Selector score `>42`; high risk/uncertainty, security/auth, explicit meta-analysis | Select domain packet -> choose blocks -> admissibility gate -> family-weighted confidence -> synthesize | Admissible result with confidence >= 80% and proof receipts | If <80% after repair loop -> best admissible option/user decision |
| Bug Reasoning | Bugs/incidents | classify -> root-cause trace -> root-cause receipt -> falsifiable hypothesis -> verification | Root cause confirmed, not the symptom | High severity / wide blast radius -> route by severity map |
| Web-Search Gate | Unstable external knowledge | detect trigger -> find sources -> reconcile | >=2 sources (>=3 for sec/arch) | If sources conflict -> escalate algorithm |

## Unified Scoring Contract

- `selector_score` is routing-only, stays on the `11-55` scale, and uses `C×2 + R×3 + S×3 + N×2 + F×1`.
- Default bands: `STC <=12`, `PR-CoT 13-22`, `MAR 23-32`, `5-SOL 33-42`, `META >42`.
- `PR-CoT` exports a gate result plus `validation_signal` from issue severity.
- `MAR` keeps the local `1-10` weighted rubric score and exports `refinement_signal`.
- `5-SOL` keeps local `1-5` category scoring, exports `best option %`, `agreement %`, and `options_signal`.
- `META` uses only normalized signals after admissibility gates and weights them by task class.

## Routing Escalators

- Route directly to `META` when protocol conflict, execution gate mismatch, fail-closed law risk, or framework-owned behavior change is present.
- Route directly to `META` when tracked writer execution has `no_eligible_analysis_lane`, `no_eligible_verifier`, or `no_eligible_coach` and a policy decision is required.
- Keep the score-selected route only when the task is mainly local implementation without governance ambiguity.
- If `STC` is later proven to be a misclassification by review/gate/root-cause evidence, do not reuse `STC` for the same task class in the current pass.
- A confirmed `STC` misfire promotes the next route to at least `PR-CoT`, and to `META` for protocol/fail-closed/framework-routing cases.

## Algorithm Cards

### STC
- When: baseline mode for simple tasks with selector score `<=12`.
- Input: a clear local objective.
- Steps: generate a step, verify it, localize the first error, roll back to a clean prefix, retry.
- Success: the task is solved without logical gaps.
- Escalation: after 3 failed retries, or immediately for protocol/route ambiguity.

### PR-CoT
- When: selector score `13-22`, medium complexity with a need for independent validation.
- Input: a task with multiple aspects (logic/data/architecture/alternatives).
- Steps: 4 perspectives -> consensus packet -> revision by each perspective.
- Success: aligned decision with no unresolved critical findings.
- Export: `validation_signal` from critical/major/minor issue weights.
- Escalation: unresolved critical findings or >=2 issues.

### MAR
- When: selector score `23-32`, complex non-trivial decisions.
- Input: a task with a high impact radius.
- Steps: 3 role rounds + accumulated lessons learned.
- Success: weighted rubric score >= 8/10 with no unresolved critical residual risk.
- Rubric weights: correctness `0.35`, completeness `0.25`, alignment `0.25`, simplicity `0.15`.
- Escalation: score < 8 after 3 rounds.

### 5-SOL
- When: selector score `33-42`, a justified choice between directions is needed.
- Input: a task with alternatives and trade-offs.
- Steps: 5 R1 options, weighted option ledger, 5 new R2 options, legal hybrid or explicit top single option.
- Success: admissible choice with transparent pros/cons, weighted option score, and confidence >= 80% or explicit cautious band.
- Weighting: 2 core categories = `0.25` each; supporting categories share the remaining `0.50`.
- Escalation: low score/confidence between rounds or failed legality.

### META
- When: selector score `>42`, high-stakes decisions, security/auth, framework-owned behavior changes, protocol conflicts, fail-closed law risk, tracked writer routing gaps, or an explicit meta-analysis request.
- Input: a complex task with a high cost of error.
- Steps: select a domain packet, assemble the smallest lawful block flow, run admissibility gate, apply family weights, synthesize.
- Success: admissible result with confidence >= 80% and proof artifacts.
- Family weights: task-class dependent, with validation heavier for security/schema work and options heavier for architecture/tech-stack work.
- Escalation: if confidence remains low.

### Bug Reasoning
- When: bugs, incidents, regressions.
- Input: a reproducible error.
- Steps: classification -> root-cause trace -> root-cause receipt -> falsifiable hypothesis -> verification.
- Success: root cause fixed, not just the symptom.
- Escalation: wide blast radius or non-reproducibility.

### Web-Search Gate
- When: external knowledge may be stale.
- Input: dependency/API/security/runtime questions.
- Steps: check trigger -> collect sources -> reconcile versions/dates.
- Success: sources are aligned and current.
- Escalation: conflicting or insufficient sources.

## Matrix: This Specific Question Through All Algorithms

Question: "How can the algorithm descriptions be optimized without losing their essence?"

| Algorithm | Result For This Question | Strength | Limitation |
|---|---|---|---|
| STC | Two-level format (Card + Deep Spec) | Fast and practical | Less alternative validation |
| PR-CoT | Added consensus format and unified card fields | Balance between speed and quality | Requires more time |
| MAR | Added quality gates and scoring | Best for standard stability | Heavier process |
| 5-SOL | Compared 5 documentation formats and chose a hybrid | Transparent trade-offs | Excessive for simple tasks |
| META | Combined standard + governance | Maximum reliability | Highest time cost |

Synthesis (recommended):
1. Keep `Quick Reference` (this file) as the operational layer.
2. Keep `Deep Spec` in `_vida/docs/thinking-protocol.md` as canonical.
3. Preserve: triggers, quality gates, escalation rules.
4. Add a smoke gate: if the quality gate fails -> automatically escalate to the next algorithm.
