# Agentic External Future Bundle

**Purpose:** Preserve the research bundle for the external integration layers intentionally excluded from the current main plan, so future work can pick them up without reconstructing the research from scratch.

**Current scope status:** explicitly out of scope for the main plan. This file is a future-facing preservation artifact only.

---

## 1. Excluded External Domains

### Canonical future domains

- `MCP / Model Context Protocol`
- `A2A / Agent-to-Agent interoperability`
- `A2UI / agent-driven interface protocols`
- `external tool / connector ecosystems`
- `agent identity / registry`
- `gateway / traffic mediation`
- `cross-provider remote agent invocation`
- `external memory and consent-governed content sharing`

---

## 2. Source Families

- `OpenAI`
- `Anthropic`
- `Google`
- `Microsoft`
- `OWASP`
- `MCP specification`

---

## 3. Web Source Basis

- MCP specification — https://modelcontextprotocol.io/specification/2024-11-05/index
- MCP versioning — https://modelcontextprotocol.io/specification/versioning
- Google A2A — https://developers.googleblog.com/a2a-a-new-era-of-agent-interoperability/
- Google A2UI — https://developers.googleblog.com/introducing-a2ui-an-open-project-for-agent-driven-interfaces/
- Anthropic subagents — https://docs.anthropic.com/en/docs/claude-code/sub-agents
- Anthropic Agent SDK — https://www.anthropic.com/engineering/building-agents-with-the-claude-agent-sdk/
- Microsoft governance/security across organization — https://learn.microsoft.com/en-us/azure/cloud-adoption-framework/ai-agents/governance-security-across-organization

---

## 4. Current Source Takeaways

- external integration layers require stronger consent, versioning, tool scoping, and audit than purely local orchestration
- interoperability is becoming a first-class area, but local runtime law should stay stable before external federation is attempted
- external future work needs its own security, identity, and governance bundle rather than piggybacking on local-only assumptions

---

## 5. Parameter Sets

### External domain families

- `protocol_family`: `mcp | a2a | a2ui | custom_remote_agent`
- `consent_model`: `implicit_none | user_approval | admin_approval | policy_gated`
- `tool_scope`: `read_only | bounded_write | privileged_write | remote_action`
- `interop_mode`: `local_only | bridge | federated`
- `remote_identity_assurance`: `low | medium | high`
- `gateway_policy`: `none | rate_limited | policy_enforced | full_mediation`
- `data_sharing_model`: `none | explicit_artifact | session_scoped | policy_scoped`

---

## 6. Research-Derived Claims

- external integrations need explicit capability and version contracts
- consent and least-privilege become stricter when crossing local runtime boundaries
- interoperability should be modeled via task/artifact/status exchange rather than loose transcript exchange
- remote agent trust should be governed like identity/security infrastructure, not just as a prompt concern

---

## 7. Source Consensus

- versioning and capability discovery matter
- consent and governance matter
- tool scoping and isolation matter
- auditability matters

---

## 8. Source Disagreements / Open Areas

- exact interop standard choice for future external work
- how much federation should be exposed by default
- how strongly local route law should constrain remote providers
- whether a single gateway layer or multiple domain-specific gateways is better

---

## 9. Research Invariants For This Future Bundle

1. external integrations must not weaken local safety invariants
2. remote tool access requires stricter consent and policy than local docs-only work
3. versioning and capability mismatch must fail closed
4. identity and audit must be explicit for remote or federated lanes

---

## 10. Heuristics

- keep external topics archived as future bundles until local orchestration law is stable
- promote local-first proofs before introducing remote provider complexity
- require explicit consent and policy modeling before any external tool ecosystem work

---

## 11. Anti-Patterns

- importing external protocols before the local core is stable
- treating remote lanes like local subagents with identical trust
- using interoperability terminology without versioning or identity rules
- flattening consent into generic approval

---

## 12. Proof Obligations

Future external-epic work should not be considered complete without:

- version/compatibility contract
- consent model
- trust/identity model
- tool scoping policy
- failure and rollback behavior
- audit surface

---

## 13. Threat / Control View

| Threat | Required control |
|---|---|
| capability mismatch | version negotiation and fail-closed behavior |
| remote privilege sprawl | least-privilege scopes and consent |
| untracked remote action | audit logs and traceability |
| unsafe cross-agent data flow | explicit sharing model and policy gates |
| injection through remote prompts/resources | stricter validation and trust boundaries |

---

## 14. Escalation Rules

- if any future external design weakens local safety invariants -> reopen security review
- if protocol versioning is unclear -> block implementation
- if remote identity or audit is undefined -> block implementation
- if consent model is ambiguous -> route to governance/policy design first

---

## 15. Metric Candidates

- remote action audit completeness
- cross-provider compatibility failure rate
- consent denial / approval rate
- gateway policy violation rate
- remote route regret

---

## 16. Terminology Seeds

- `protocol_family`
- `interop_mode`
- `consent_model`
- `gateway_policy`
- `remote_identity_assurance`
- `data_sharing_model`

---

## 17. Future Task Archetypes

- `mcp-integration-spec`
- `a2a-federation-research`
- `gateway-policy-design`
- `remote-identity-and-consent-model`
- `external-tool-scope-audit`

---

## 18. Claim-To-Artifact Trace Seeds

- interoperability claims -> future interop protocol docs
- consent/governance claims -> future security/governance docs
- versioning claims -> future compatibility tests and policy docs

---

## 19. Threshold Hypotheses

- keep `interop_mode=local_only` as default until future external epics explicitly change it
- require `policy_enforced` or stronger gateway policy before remote action execution
- require at least `medium` remote identity assurance before non-read-only remote lanes

---

## 20. Known Unknowns

- exact MCP adoption scope for VIDA
- whether A2A belongs in framework core or future project overlays only
- what external identity model should be canonical
- how much of future external policy should live in config vs protocol docs

---

## 21. Future Migration Guidance

If a future epic picks up any of these domains:

1. start from this bundle, not from scratch
2. split the epic by `identity`, `capability/versioning`, `consent/governance`, and `runtime enforcement`
3. preserve local safety invariants while expanding scope

