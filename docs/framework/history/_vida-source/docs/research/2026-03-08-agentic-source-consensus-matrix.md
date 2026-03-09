# Agentic Source Consensus Matrix

**Purpose:** Record the strongest agreements across source families so VIDA can treat them as the most stable external truths in this research bundle.

---

## Consensus Areas

| Topic | OpenAI | Anthropic | Google | Microsoft | OWASP | Result |
|---|---|---|---|---|---|---|
| fixed minimum agent counts are weak default policy | yes | implied yes | yes | yes | n/a | strong consensus |
| single-agent is still valid for some tasks | yes | yes | yes | yes | n/a | strong consensus |
| task shape matters for route selection | yes | yes | yes | yes | n/a | strong consensus |
| artifact-driven handoff beats chat-memory dependence | yes | yes | implied yes | implied yes | n/a | strong consensus |
| consensus is not a proof substitute | yes | yes | implied yes | yes | n/a | strong consensus |
| security and governance must be first-class | yes | implied yes | implied yes | yes | yes | strong consensus |
| volatile guidance requires refresh or validation | yes | yes | implied yes | yes | yes | strong consensus |
| evals and measurement are necessary | yes | implied yes | yes | implied yes | n/a | strong consensus |
| profiles/personas must stay explicit and bounded | yes | implied yes | yes | yes | n/a | strong consensus |

---

## Interpretation Rule

Use this matrix when deciding whether a principle can be elevated toward an invariant.

Threshold:

1. if 3+ strong source families agree and no major family materially disagrees, treat the result as `candidate invariant`
2. if agreement is partial or conditional, route it to the heuristic library instead

