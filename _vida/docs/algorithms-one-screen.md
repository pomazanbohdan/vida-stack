# Algorithms One-Screen

| Algorithm | When | Minimum Actions | Gate | Escalation |
|---|---|---|---|---|
| STC | Basic/local task | step -> check -> localize -> retry (<=3) | No unresolved step error remains | -> PR-CoT |
| PR-CoT | Medium complexity | 4 perspectives -> consensus -> revision | No unresolved critical findings remain | -> MAR |
| MAR | Complex task | 3 role rounds + knowledge carry-over | weighted rubric score >= 8/10 and no critical residual risk | -> META |
| 5-SOL | Choice between alternatives | R1: 5 options -> weighted ledger -> R2: 5 new options -> legal hybrid/top option | Admissible final decision with score+confidence >= 80 or explicit cautious band | -> META |
| META | High-stakes / meta-analysis | domain packet -> choose blocks -> admissibility gate -> family-weighted confidence -> synthesis | admissible and confidence >= 80% | repair loop -> best admissible option/user input |
| Bug Reasoning | Bug/incident | classify -> trace -> root-cause receipt -> hypothesis -> route | Root cause confirmed | -> route by severity map |
| Web Search Gate | External unstable information | trigger -> sources -> reconcile | >=2 sources (>=3 sec/arch) | escalate algorithm level |

Keep the essence intact: `triggers + quality gates + escalation`.

Scoring surfaces:
- `selector_score` = routing only (`11-55`)
- `algorithm_raw_score` = local algorithm score (`PR-CoT issues`, `MAR 1-10`, `5-SOL 1-5 + option %`)
- `normalized_signal` = handoff/META input after gates
