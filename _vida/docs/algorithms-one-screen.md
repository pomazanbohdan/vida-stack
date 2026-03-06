# Algorithms One-Screen

| Algorithm | When | Minimum Actions | Gate | Escalation |
|---|---|---|---|---|
| STC | Basic/local task | step -> check -> fix -> retry (<=3) | No logical gaps remain | -> PR-CoT |
| PR-CoT | Medium complexity | 4 perspectives -> consensus -> revision | No critical conflicts remain | -> MAR |
| MAR | Complex task | 3 role rounds | score >= 8/10 | -> META |
| 5-SOL | Choice between alternatives | R1: 5 options -> R2: 5 new options -> hybrid | Clear trade-off choice | -> META |
| META | High-stakes / meta-analysis | PR-CoT + MAR + 5-SOL + synthesis | confidence >= 80% | cautious decision/user input |
| Bug Reasoning | Bug/incident | classify -> trace -> hypothesis -> verify | Root cause confirmed | -> MAR/5-SOL/META |
| Web Search Gate | External unstable information | trigger -> sources -> reconcile | >=2 sources (>=3 sec/arch) | escalate algorithm level |

Keep the essence intact: `triggers + quality gates + escalation`.
