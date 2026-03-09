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
-----
artifact_path: framework/research/agentic-task-archetype-library
artifact_type: framework_research_doc
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/research/agentic-task-archetype-library.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: agentic-task-archetype-library.changelog.jsonl
P26-03-09T21: 44:13Z
