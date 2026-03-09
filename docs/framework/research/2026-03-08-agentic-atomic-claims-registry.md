# Agentic Atomic Claims Registry

**Purpose:** Capture the smallest reusable research claims from the orchestration corpus so the system can track what is known, how strongly it is supported, and which documents consume it.

---

## Claims

| ID | Claim | Source families | Confidence | Notes | Consumed by |
|---|---|---|---|---|---|
| C01 | More agents are not always better. | Google, Microsoft, Anthropic | high | sequential or tightly coupled tasks can degrade under excess fan-out | pattern chooser, heuristics, task score |
| C02 | Single-agent execution remains valid for bounded or tightly coupled work. | Google, Microsoft, OpenAI | high | low-score or narrow shared-scope work should not be forced into multi-agent form | decision ledger, adaptive routing |
| C03 | Adaptive scaling is better than a fixed minimum agent rule when task shape and prior effectiveness are known. | Google, Anthropic, OpenAI | medium-high | thresholds remain open, direction is stable | decision ledger, task score |
| C04 | Consensus does not equal proof. | OpenAI, Anthropic, Microsoft | high | agreement must not replace verifier independence | invariants, proof registry |
| C05 | Role identity should be separated from permissions and approval authority. | Microsoft, OpenAI, Google | high | profiles must not smuggle permissions through tone/persona | role profile work |
| C06 | Artifact-driven handoff is more reliable than transcript inheritance for long-horizon tasks. | Anthropic, OpenAI | high | context drift is a real failure mode | task packets, handoff protocols |
| C07 | Research refresh is required for source-sensitive tasks because guidance is volatile. | OpenAI, Anthropic, OWASP | high | especially true for agent safety, scaling, and OWASP material | refresh protocol |
| C08 | OWASP should be treated as a control spine, not a citation appendix. | OWASP, Microsoft, OpenAI | high | map controls into tasks, reviews, and gates | threat matrix, proof registry |
| C09 | Verification burden should scale with risk, volatility, and prior weak results. | OpenAI, Microsoft | medium-high | exact thresholds remain open | proof registry, metric glossary |
| C10 | Route rationale and traceability are needed for trustworthy orchestration. | OpenAI, Microsoft | medium-high | path changes must be inspectable | trace schema, metrics |
| C11 | Personas should be few, explicit, and purpose-bound rather than numerous and fuzzy. | Microsoft, OpenAI | high | role sprawl hurts manageability | taxonomy, profiles |
| C12 | Disagreement can indicate ambiguity, not just missing perspectives. | Google, Anthropic | medium-high | sometimes arbiter/reframe is better than more fan-out | disagreement matrix, heuristics |
| C13 | Security-sensitive tasks need independent security handling, not generic review only. | OWASP, Microsoft | high | applies to agentic, backend, and mobile surfaces | threat matrix, escalation matrix |
| C14 | Benchmarks and evals are required to justify orchestration complexity. | OpenAI, Google | high | anecdotal wins are insufficient | metrics, proof registry |
| C15 | Stable terminology is necessary because semantic drift breaks routing and verification. | Microsoft, OpenAI | medium | terminology errors become execution errors in multi-doc systems | glossary, templates |

---

## Use Rule

When a new document, task packet, or protocol cites research:

1. prefer claim IDs over ad hoc paraphrase
2. if no claim fits, add a new one here first
3. route disagreements through the disagreement matrix instead of silently forcing them into consensus

