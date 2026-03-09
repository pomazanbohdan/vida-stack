# Agentic Task Archetype Library

**Purpose:** Provide default shapes for common child tasks so future slicing stays consistent.

---

## Archetypes

### Research archetype

- default route: single lane + independent review
- proof: document + source refresh + review
- security classification: low unless data/security topic

### Protocol / docs archetype

- default route: single author + independent reviewer
- proof: artifact completeness + terminology alignment
- anti-pattern to avoid: over-parallelized shared editing

### Routing / scoring archetype

- default route: single author + verifier
- proof: protocol + tests + route examples
- anti-pattern to avoid: intuition-based thresholds

### Security mapping archetype

- default route: author + security reviewer
- proof: control matrix + explicit ownership
- anti-pattern to avoid: flattening security into generic review

### Eval / trace archetype

- default route: author + proof validator
- proof: schema + sample traces/metrics
- anti-pattern to avoid: metrics without definitions

### Pilot / rollout archetype

- default route: measured pilot with stop rules
- proof: metrics, rollback note, escalation path
- anti-pattern to avoid: production-scale adoption without proving

