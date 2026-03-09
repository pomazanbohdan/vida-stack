# Agentic Research Implication Map

**Purpose:** Translate the research corpus into architecture implications so future work can see not just what sources said, but what those findings mean for VIDA design.

---

## Implications By Domain

| Domain | Research implication | Primary downstream surfaces |
|---|---|---|
| agent definitions | role logic must be modeled as a versioned agent-definition system rather than loose prompt prose | agent-definition protocol, instruction contracts, prompt template configs, conformance evals |
| role profiles | profiles must separate identity, tone, permissions, and authority | role-profile protocol, templates, examples |
| routing | route selection must depend on task shape, risk, and prior effectiveness | task classification, task scoring, adaptive routing |
| scaling | fixed minimum agent count is weaker than adaptive orchestration | task score, pattern chooser, threshold hypotheses |
| consensus | consensus is a merge mechanism, not a proof mechanism | consensus protocol, verifier design, proof registry |
| verification | proof burden must scale independently of implementation burden | proof registry, verification matrix, health gates |
| security | OWASP controls must be mapped into task design and gates, not appended as notes | threat matrix, OWASP mapping, escalation matrix |
| handoff/context | artifact-driven packets and summaries are required for resilience | task packets, context handoff docs, compaction |
| evals | orchestration needs explicit benchmarking and route-regret measurement | metric glossary, eval protocol, pilots |
| governance | route changes and approval conditions need inspectable rationale | trace schema, rollout docs, escalation policy |
| future integrations | external layers should remain preserved as future bundles until explicitly brought into scope | external future bundle, future epics |

---

## Interpretation Rule

When a future task references research:

1. cite the underlying claim or source family,
2. cite at least one architecture implication from this map,
3. point to the downstream artifact family that should absorb the implication.
