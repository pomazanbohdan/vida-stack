# Algorithms One-Screen

| Algorithm | When | Minimum Actions | Gate | Escalation |
|---|---|---|---|---|
| STC | Selector score `<=12`; basic/local task | step -> check -> localize -> retry (<=3) | No unresolved step error remains | -> PR-CoT; protocol/route ambiguity -> META |
| PR-CoT | Selector score `13-22`; medium complexity | 4 perspectives -> consensus -> revision | No unresolved critical findings remain | -> MAR |
| MAR | Selector score `23-32`; complex task | 3 role rounds + knowledge carry-over | weighted rubric score >= 8/10 and no critical residual risk | -> META |
| 5-SOL | Selector score `33-42`; choice between alternatives | R1: 5 options -> weighted ledger -> R2: 5 new options -> legal hybrid/top option | Admissible final decision with score+confidence >= 80 or explicit cautious band | -> META |
| META | Selector score `>42`; high-stakes / meta-analysis | domain packet -> choose blocks -> admissibility gate -> family-weighted confidence -> synthesis | admissible and confidence >= 80% | repair loop -> best admissible option/user input |
| Bug Reasoning | Bug/incident | classify -> trace -> root-cause receipt -> hypothesis -> route | Root cause confirmed | -> route by severity map |
| Web Search Gate | External unstable information | trigger -> sources -> reconcile | >=2 sources (>=3 sec/arch) | escalate algorithm level |

Keep the essence intact: `triggers + quality gates + escalation`.

Scoring surfaces:
- `selector_score` = routing only (`11-55`)
- `algorithm_raw_score` = local algorithm score (`PR-CoT issues`, `MAR 1-10`, `5-SOL 1-5 + option %`)
- `normalized_signal` = handoff/META input after gates

Routing escalators:
- Route directly to `META` for framework-owned behavior change, protocol conflict, execution gate mismatch, fail-closed law risk, or tracked writer `no_eligible_*` routing gaps.
- Keep the score-selected route only for mainly local implementation without governance ambiguity.
- If `STC` is later proven to be a misclassification by review/gate/root-cause evidence, do not reuse `STC` for the same task class in the current pass; promote to at least `PR-CoT`, or to `META` for protocol/fail-closed/framework-routing cases.
