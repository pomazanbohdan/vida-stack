# Algorithms Quick Reference

Purpose: compress the algorithm descriptions without losing their essence by keeping explicit triggers, quality gates, and escalation rules.

## Unified Matrix

| Algorithm | When To Use | Mandatory Steps (minimum) | Quality Gate | Escalation |
|---|---|---|---|---|
| STC | Low complexity, local tasks | Step -> check -> localize -> rollback -> retry (<=3) | No unresolved step errors remain | After 3 retries -> PR-CoT |
| PR-CoT | Medium complexity, independent perspectives needed | Pass1 (4 perspectives) -> consensus -> Pass2 (revision) | Critical contradictions are closed | >=2 issues after Pass2 -> MAR |
| MAR | Complex tasks with regression risk | 3 rounds (Actor/Evaluator/Critic/Reflector) | Score >= 8/10 | <8 after 3 rounds -> META |
| 5-SOL | Choice between alternatives / design directions | R1: 5 options -> packet -> R2: 5 new options -> hybrid | Comparative choice with explicit trade-offs | Low agreement/confidence -> META |
| META | High risk/uncertainty, security/auth, explicit meta-analysis | PR-CoT + MAR + 5-SOL + synthesis | Confidence >= 80% with evidence | If <80% after loop -> cautious synthesis/user decision |
| Bug Reasoning | Bugs/incidents | classify -> trace -> hypothesis -> verify -> resolve | Root cause confirmed, not the symptom | High severity / wide blast radius -> MAR/5-SOL/META |
| Web-Search Gate | Unstable external knowledge | detect trigger -> find sources -> reconcile | >=2 sources (>=3 for sec/arch) | If sources conflict -> escalate algorithm |

## Algorithm Cards

### STC
- When: baseline mode for simple tasks.
- Input: a clear local objective.
- Steps: generate a step, verify it, localize the first error, roll back to a clean prefix, retry.
- Success: the task is solved without logical gaps.
- Escalation: after 3 failed retries.

### PR-CoT
- When: medium complexity with a need for independent validation.
- Input: a task with multiple aspects (logic/data/architecture/alternatives).
- Steps: 4 perspectives -> consensus packet -> revision by each perspective.
- Success: aligned decision with no critical conflicts.
- Escalation: unresolved critical contradictions.

### MAR
- When: complex non-trivial decisions.
- Input: a task with a high impact radius.
- Steps: 3 role rounds + accumulated lessons learned.
- Success: score >= 8/10.
- Escalation: score < 8 after 3 rounds.

### 5-SOL
- When: a justified choice between directions is needed.
- Input: a task with alternatives and trade-offs.
- Steps: 5 R1 options, 5 new R2 options, final hybrid choice.
- Success: choice with transparent pros/cons.
- Escalation: low agreement between rounds.

### META
- When: high-stakes decisions, security/auth, or an explicit meta-analysis request.
- Input: a complex task with a high cost of error.
- Steps: run PR-CoT, MAR, and 5-SOL separately -> synthesize -> confidence gate.
- Success: confidence >= 80% with evidence artifacts.
- Escalation: if confidence remains low.

### Bug Reasoning
- When: bugs, incidents, regressions.
- Input: a reproducible error.
- Steps: classification -> root-cause trace -> falsifiable hypothesis -> verification.
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
