# Agentic Decision Ledger

**Purpose:** Record the concrete architecture decisions taken from research so future work can distinguish between raw takeaways and deliberate design choices.

---

## Decision Table

| ID | Decision | Why | Source families | Impacts | Invalidation triggers |
|---|---|---|---|---|---|
| D01 | Use `research refresh` as a blocking gate for source-sensitive tasks | external guidance changes fast enough to affect design and safety | OpenAI, Anthropic, OWASP | refresh protocol, task packets | major source updates on prompting, scaling, OWASP |
| D02 | Separate `role identity` from `execution mode` | prevents mixing persona, authority, and permissions | Microsoft, OpenAI, Google | role profiles, templates | new role-design guidance that collapses these successfully |
| D03 | Keep `single-agent` lawful for low-score or tightly coupled work | fixed agent minimum harms efficiency and can degrade sequential work | Google, Microsoft | adaptive routing, scoring | new scaling evidence showing different default |
| D04 | Use `adaptive agent count` based on task shape and prior effectiveness | dynamic scaling is stronger than rigid fan-out | Google, Anthropic, OpenAI | task scoring, routing policy | better cost/quality evidence with another strategy |
| D05 | Treat `consensus` as advisory or route-gating, not as proof by itself | agreement does not equal truth or safety | Azure, Anthropic, OpenAI | consensus protocol, verifier design | strong evidence that another proof mechanism supersedes verifier separation |
| D06 | Keep `coach` before verifier and never instead of verifier | formative critique and independent proof are distinct | internal VIDA research synthesis, OpenAI/Azure patterns | execution chain, verification law | future framework law explicitly changes coach semantics |
| D07 | Make `artifact-driven handoff` the default | chat memory is fragile and context windows drift | Anthropic, OpenAI | task packets, handoff summaries | strong contrary evidence favoring transcript inheritance |
| D08 | Use `OWASP` as a security spine, not background reading | agentic, backend, mobile, and SDLC risks need one mapped control layer | OWASP | security protocols, verification, approval | new security framework fully replaces this mapping |
| D09 | Promote only high-value documented values into runtime law | not every research finding belongs in config immediately | OpenAI, Microsoft | promotion workflow | evidence that direct config-first is safer |
| D10 | Make `proof burden` adaptive to task risk, volatility, and prior result quality | implementation burden and proof burden should not be conflated | OpenAI, Azure | proof registry, verification matrix | new eval evidence contradicting adaptive proof logic |
| D11 | Use `traceable route rationale` for orchestration changes | route changes must be inspectable, not magical | OpenAI, Microsoft | trace schema, operator surfaces | alternative trace mechanism adopted |
| D12 | Use `task archetypes` to reduce slicing ambiguity | repeated task families should not be reinvented | Microsoft, Anthropic | epic slicing, packet templates | future project-specific diversity makes archetypes too coarse |
| D13 | Treat `Agent Definition` as the umbrella object over role profile, instruction contract, tool policy, and rendering config | agent logic should not live only in loose prompts or provider config | Microsoft, OpenAI, Anthropic | agent-definition protocol, role profiles, templates, evals | stronger official definition model supersedes the current synthesis |
| D14 | Keep `Instruction Contract` as canonical logic and `Prompt Template Configuration` as rendering layer | prevents provider-specific prompt config from becoming the hidden source of truth | Microsoft, Anthropic, OpenAI | templates, runtime packets, tests | a stronger unified schema replaces the split |
| D15 | Default to deterministic, allowlisted, fail-closed behavior for role logic | undefined behavior and silent autonomy expansion are unacceptable in VIDA | OpenAI, Google | execution law, fallback rules, guardrails, approvals | new framework doctrine explicitly authorizes broader improvisation |

---

## How To Use

When drafting a new protocol or child task:

1. cite the decision IDs that justify the work
2. check invalidation triggers
3. if invalidated, return to source refresh and record a delta
