# Agentic Parameter Registry

**Purpose:** Capture the canonical value sets, enumerations, score bands, and reusable lists discussed during research so they stop living in chat history and can be referenced by future plans, protocols, config, and tests.

**Scope:** Documentation-only registry for local VIDA work. No external integrations are assumed beyond web search refresh.

**Usage rule:**
- treat this file as the documentation SSOT for research-derived parameter sets until the values are promoted into framework-owned protocols, templates, config, or tests
- when a value here becomes executable runtime law, the runtime surface becomes the stronger source and this file should point to it

**Refresh rule:**
- update this registry whenever research materially changes a parameter family
- record the corresponding source delta in the source delta log

---

## 1. Source Families

### Canonical values

- `OpenAI`
- `Anthropic`
- `Google`
- `Microsoft`
- `OWASP`

### Use

- cite these as the primary research families in source registry and delta log artifacts

---

## 2. Role Classes

### Canonical values

- `defining`
- `producing`
- `proving`
- `governing`

### Notes

- use role classes to keep profiles grouped by system function, not by job-title aesthetics

---

## 3. Canonical Roles

### Defining

- `request-intake`
- `researcher`
- `domain-analyst`
- `spec-writer`
- `spec-critic`
- `planner`
- `dependency-mapper`

### Producing

- `writer`
- `refactorer`
- `migration-owner`
- `process-designer`

### Proving

- `coach`
- `tester`
- `verifier`
- `reviewer`
- `ops-validator`
- `security-reviewer`

### Governing

- `orchestrator`
- `release-coordinator`
- `retrospective-analyst`
- `framework-diagnostician`

---

## 4. Thinking Tone Axes

### Canonical values

- `skepticism`: `low | medium | high`
- `precision`: `low | medium | high`
- `creativity`: `low | medium | high`
- `persistence`: `low | medium | high`
- `risk_tolerance`: `low | medium | high`
- `verbosity`: `low | medium | high`
- `warmth`: `low | medium | high`
- `abstraction_level`: `low | medium | high`

### Notes

- use axes to tune cognitive stance without changing permissions or authority

---

## 5. Execution Modes

### Canonical values

- `light`
- `standard`
- `strict`
- `incident`
- `release`
- `read_only`
- `write_enabled`

### Notes

- keep identity stable and vary execution mode when the role posture changes

---

## 6. Task Classes

### Canonical values

- `research`
- `spec`
- `bugfix`
- `feature`
- `refactor`
- `migration`
- `incident`
- `release`
- `docs`
- `process`
- `framework`
- `framework-routing`

### Notes

- use exactly one primary task class per task packet

---

## 7. Risk Levels

### Canonical values

- `low`
- `medium`
- `high`
- `critical`

### Suggested interpretation

- `low`: bounded local change, limited blast radius
- `medium`: shared logic or meaningful regression surface
- `high`: security, routing, wide-scope workflow, or release impact
- `critical`: production safety, destructive migration, or severe incident handling

---

## 8. Complexity Levels

### Canonical values

- `low`
- `medium`
- `high`

### Notes

- complexity is not the same as risk; a task can be low-complexity and still high-risk

---

## 9. Coupling Levels

### Canonical values

- `low`
- `medium`
- `high`

### Notes

- high coupling is a strong signal against blind fan-out

---

## 10. External Volatility Levels

### Canonical values

- `low`
- `medium`
- `high`

### Notes

- use this for research refresh pressure and proof burden

---

## 11. Prior Effectiveness Bands

### Canonical values

- `poor`
- `weak`
- `neutral`
- `good`
- `strong`

### Notes

- use prior effectiveness to adjust route choice, proof burden, and escalation

---

## 12. Agent Count Modes

### Canonical values

- `single`
- `dual`
- `triad`
- `quorum`
- `arbiter`

### Suggested interpretation

- `single`: one primary lane
- `dual`: producer + challenger or producer + verifier
- `triad`: producer + coach/checker + verifier
- `quorum`: 3+ advisory/consensus lanes with merge policy
- `arbiter`: explicit tie-break or adjudication lane

---

## 13. Consensus Modes

### Canonical values

- `single_pass`
- `unanimous`
- `majority`
- `weighted_majority`
- `verifier_veto`
- `arbiter_tiebreak`

### Notes

- consensus mode is not a substitute for verification independence

---

## 14. Verification Burden Levels

### Canonical values

- `light`
- `standard`
- `high`
- `strict`

### Notes

- increase verification burden when risk, volatility, or prior ineffective results rise

---

## 15. Approval Gate Levels

### Canonical values

- `none`
- `policy_gate_required`
- `senior_review_required`
- `human_gate_required`

### Notes

- use this family when routes need explicit approval control

---

## 16. Route Rationales

### Canonical values

- `single_lane_authoring`
- `single_lane_authoring_plus_independent_review`
- `single_lane_authoring_plus_verifier`
- `sequential_shared-surface`
- `parallel_independent-slices`
- `coach_gated`
- `security_gated`
- `measured_pilot`

### Notes

- route rationale should explain why a task is not being parallelized, not only why it is

---

## 17. Freshness SLA Buckets

### Canonical values

- `refresh_on_start`
- `refresh_before_rollout`
- `refresh_on_material_source_change`
- `refresh_on_security_source_change`
- `refresh_on_major_prompting_change`

### Notes

- tasks with volatile guidance should use more than one freshness trigger

---

## 18. OWASP Surfaces

### Canonical values

- `prompt_injection`
- `unsafe_tool_use`
- `excessive_agency`
- `insecure_memory_handling`
- `sensitive_data_exposure`
- `backend_api_verification`
- `mobile_auth_storage_network`
- `sdlc_maturity`

### Mapping hints

- `prompt_injection`, `unsafe_tool_use`, `excessive_agency`, `insecure_memory_handling`, `sensitive_data_exposure` -> OWASP GenAI / LLM / agentic guidance
- `backend_api_verification` -> ASVS
- `mobile_auth_storage_network` -> MASVS / MASTG
- `sdlc_maturity` -> SAMM

---

## 19. Trace Fields

### Canonical values

- `task_class`
- `task_score`
- `risk_level`
- `prior_effectiveness`
- `agent_count_mode`
- `consensus_mode`
- `verification_burden`
- `selected_roles`
- `research_refresh_status`
- `blockers`
- `rework_loops`
- `verification_outcome`

---

## 20. Eval Metrics

### Canonical values

- `task_success_rate`
- `cost_per_resolved_task`
- `rework_rate`
- `false_green_rate`
- `consensus_quality`
- `verifier_overturn_rate`
- `route_regret`
- `security_gate_hit_rate`

---

## 21. Documentation Reinforcement Fields

### Canonical values

- `Research-derived decisions`
- `Invalidation triggers`
- `Local proof obligations`
- `Fallback if research shifts`
- `Assumptions register`
- `Scope boundary`
- `Non-goals`
- `Dependency map`
- `Failure modes / anti-patterns`
- `Risk register`
- `Change impact surface`
- `Interface contract`
- `Definition of done by artifact`
- `Verification recipe`
- `Rollback / reversal plan`
- `Escalation map`
- `Ownership map`
- `Data / security classification`
- `Traceability links`
- `Examples / golden samples`
- `Route rationale`
- `Cost and latency budget`
- `Freshness SLA`
- `Terminology normalization`
- `Open questions`

### Notes

- this family should be applied to every future child task sliced from the master plan

---

## 22. Terminology Normalization Seeds

### Canonical values

- `role`
- `profile`
- `execution_mode`
- `task_class`
- `task_score`
- `agent_count_mode`
- `consensus_mode`
- `verification_burden`
- `approval_gate_level`
- `research_refresh`
- `proof_obligation`
- `escalation`

### Notes

- keep these names stable across docs, config, and tests

---

## 23. Registry Mutation Rules

1. Add a new value only if an existing value cannot describe the case without ambiguity.
2. When a new value is added, update:
   - the source delta log
   - affected plan/protocol docs
   - any tests or config that use the enumeration
3. If a value becomes executable runtime law, add a pointer to the stronger runtime source.
4. Do not allow synonyms to accumulate in parallel.

---

## 24. Immediate next use

Use this registry as the input source when:

- slicing the master plan into a new epic
- building role-profile templates
- defining task score and routing protocols
- defining adaptive agent-count policy
- defining verification burden and OWASP mapping
-----
artifact_path: framework/research/agentic-parameter-registry
artifact_type: framework_research_doc
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/research/agentic-parameter-registry.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: agentic-parameter-registry.changelog.jsonl
P26-03-09T21: 44:13Z
