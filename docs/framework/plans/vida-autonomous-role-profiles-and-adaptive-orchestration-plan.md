# VIDA Autonomous Role Profiles And Adaptive Orchestration Implementation Plan

> **For Agent:** Choose execution skill based on task dependencies:
> - Sequential/dependent tasks -> `worker-driven-development`
> - 3+ independent tasks -> `dispatching-parallel-agents`

**Goal:** Implement a complete VIDA-native system for role profiles, agent definitions, instruction contracts, adaptive agent counts, consensus orchestration, research-refresh execution, and OWASP-aware verification without introducing external tool integrations beyond web search.

**Architecture:** Build the system in layers. First establish canonical source refresh and the role/profile/instruction-definition contract. Then add routing, adaptive scaling, consensus, task packets, verification burden, OWASP mapping, and evaluation/trace surfaces. Only after the control plane is explicit should the project roll out task slicing, pilot runs, and epic-scale adoption.

**Tech Stack:** Markdown protocols, instruction contracts, YAML templates/config, Python runtime helpers, shell wrappers, local execution receipts, web-search-based research refresh, `docs/framework/history/_vida-source/tests/*` proof surfaces, `br` task state, TaskFlow telemetry.

**Parallelizable:** PARTIALLY. Foundation tasks are sequential. After the profile/routing contract is stable, verification/eval/security work can split into bounded parallel slices.

**Total Tasks:** 18 tasks, expected to become one new epic with multiple child tasks and at least one proving wave.

**Out Of Scope For This Plan:**
- no MCP/A2A or other external integration implementation
- no remote tool marketplace or connector runtime
- no vendor-specific external tool orchestration beyond web search refresh
- no UI work unless needed for operator visibility of the new local runtime state

**Execution Principle:**
- every child task must be independently executable from its packet
- every child task must start with a research refresh gate
- every child task must end with an explicit proof artifact
- no task may assume chat memory from prior tasks
- no runtime behavior may rely on implied agent logic; instruction behavior must be explicit, versioned, and fail-closed

---

## Source Refresh Baseline

Every implementation task in this plan must refresh and record source deltas before mutation whenever the task depends on externally changing guidance.

### Canonical source families

1. `OpenAI`
   - prompting, evals, agent safety, harness engineering, agent runtime design
2. `Anthropic`
   - context engineering, multi-agent research system, auditing, red teaming
3. `Google`
   - agent scaling science, design-pattern selection, A2A/A2UI references, orchestration studies
4. `Microsoft`
   - personas, orchestration patterns, governance, AI security, approval patterns
5. `OWASP`
   - GenAI/LLM/agentic security, ASVS, MASVS, MASTG, SAMM

### Research Refresh Gate (RRG)

Every task packet that touches any source-sensitive behavior must begin with:

1. Load the current source registry artifact.
2. Re-run targeted web searches against primary or strong sources only.
3. Record one of:
   - `no_material_change`
   - `material_change_requires_task_delta`
4. If material change exists:
   - update the source registry,
   - update the task packet or spec delta,
   - only then continue implementation.

### Minimum refresh questions

1. Has guidance changed for role/persona/profile design?
2. Has guidance changed for adaptive multi-agent scaling?
3. Has guidance changed for consensus/verification patterns?
4. Has guidance changed for OWASP GenAI / agentic security?
5. Has guidance changed for evals / tracing / context engineering?

### Required source artifacts to create during implementation

- `docs/framework/history/research/2026-03-08-agentic-role-profile-source-registry.md`
- `docs/framework/history/research/2026-03-08-agentic-role-profile-source-delta-log.md`

---

## Epic Structure Recommendation

Create one new epic for implementation and slice it into these waves:

1. `Wave A - Sources, Profiles, Taxonomy`
2. `Wave B - Routing, Scoring, Adaptive Scaling`
3. `Wave C - Consensus, Task Packets, Context Handoffs`
4. `Wave D - Verification Burden, OWASP, Security Gates`
5. `Wave E - Evals, Traces, Operator Visibility`
6. `Wave F - Pilot And Rollout`

---

## Unified Task Reinforcement Bundle

This bundle applies to **every one of Tasks 1-18 below**. When any task from this plan is converted into an epic child task, spec packet, or TaskFlow execution packet, the child artifact must include this full reinforcement layer.

Purpose:
- prevent hidden assumptions
- prevent research-to-implementation loss
- make each task independently executable
- make verification and rollback explicit
- keep future slicing deterministic

### Canonical reinforcement fields

1. `Research-derived decisions`
   - which concrete design/runtime decisions are taken directly from the cited web sources
2. `Invalidation triggers`
   - which external source changes force task re-review
3. `Local proof obligations`
   - which local docs/tests/receipts/traces must exist because of the research-backed decision
4. `Fallback if research shifts`
   - how the task reacts when refreshed guidance conflicts with the current plan
5. `Assumptions register`
   - what the task assumes about codebase, docs, protocols, and runtime state
6. `Scope boundary`
   - exact in-scope work
7. `Non-goals`
   - explicit out-of-scope items
8. `Dependency map`
   - upstream tasks, protocols, artifacts, and decisions required before execution
9. `Failure modes / anti-patterns`
   - known ways this task can fail or degrade quality
10. `Risk register`
   - task-local risks, blast radius, and mitigation posture
11. `Change impact surface`
   - files, protocols, routes, tests, and operator surfaces likely to change
12. `Interface contract`
   - what artifact/schema/receipt this task must hand off downstream
13. `Definition of done by artifact`
   - exact deliverables proving the task is complete
14. `Verification recipe`
   - exact verification path, not only generic “review it”
15. `Rollback / reversal plan`
   - how to safely revert or disable the change if later tasks expose flaws
16. `Escalation map`
   - when the task must reopen spec, security, approval, or verifier flow
17. `Ownership map`
   - owner, reviewer, verifier, security owner, advisory roles
18. `Data / security classification`
   - relevant data sensitivity and OWASP/security surfaces
19. `Traceability links`
   - source -> decision -> artifact -> proof -> downstream dependency
20. `Examples / golden samples`
   - one concrete sample of the target deliverable or result shape
21. `Route rationale`
   - why this task should be sequential, parallel, single-lane, coach-gated, etc.
22. `Cost and latency budget`
   - acceptable orchestration and verification overhead
23. `Freshness SLA`
   - when the source basis must be refreshed again
24. `Terminology normalization`
   - exact terms that must be used consistently
25. `Open questions`
   - remaining uncertainties that do not block the current task but must stay visible

### Required fill quality

Every child task cut from this plan must satisfy all of the following:

1. no field may be left implicit if it changes execution or verification
2. no field may defer critical assumptions to chat memory
3. `Research-derived decisions`, `Invalidation triggers`, and `Local proof obligations` are mandatory whenever web-backed guidance is cited
4. `Rollback / reversal plan` is mandatory for any task that changes protocols, routing, scoring, or security posture
5. `Data / security classification` is mandatory for any task touching role permissions, routing law, memory, verification, or OWASP mapping
6. `Open questions` may remain unresolved only if they do not block safe execution

### Compression rule

When converting these tasks into child tasks:

- keep the full reinforcement bundle,
- but allow terse field values,
- except for `Research-derived decisions`, `Invalidation triggers`, `Local proof obligations`, `Verification recipe`, and `Escalation map`, which must remain explicit.

### Default bundle values by task family

Use these defaults unless the child task packet records a justified override:

- `docs/protocol tasks`
  - route rationale: single-lane authoring plus independent review
  - cost budget: low
  - freshness SLA: refresh on every major external-guidance touch
- `routing/scoring tasks`
  - route rationale: single-lane authoring plus verifier review and trace proof
  - cost budget: medium
  - rollback: feature-flag or route-law rollback if possible
- `security/OWASP tasks`
  - route rationale: authoring plus independent security review
  - cost budget: medium-high
  - freshness SLA: shortest among all task families
- `eval/trace tasks`
  - route rationale: authoring plus independent proof validation
  - cost budget: medium
- `pilot/rollout tasks`
  - route rationale: orchestrated, measured, and fail-safe
  - cost budget: highest acceptable among plan tasks

### Reinforcement bundle usage note

The sections already present in Tasks 1-18 cover:
- `Web source basis`
- `Current source takeaways`
- `Research Refresh Gate`
- `Files`
- `Steps`
- `Acceptance`
- `Verification`

The bundle above is additive. It strengthens those sections and must be merged into every future child task created from this plan.

---

## Task 1: Create the source registry and refresh protocol

**Why first:** The rest of the plan depends on refreshed external guidance. This task prevents knowledge drift.

**Files:**
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/history/research/2026-03-08-agentic-role-profile-source-registry.md`
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/history/research/2026-03-08-agentic-role-profile-source-delta-log.md`
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/research-refresh-protocol.md`
- Modify: `/home/unnamed/project/mobile-odoo/docs/process/agent-system.md`

**Autonomous packet must include:**
- source families
- current reviewed dates
- allowed source tiers
- material-change rules

**Research Refresh Gate:**
1. Re-run searches for OpenAI, Anthropic, Google, Microsoft, OWASP guidance.
2. Capture exact source links and last-reviewed dates.
3. Record which parts are stable and which are volatile.

**Web source basis:**
- OpenAI, *A practical guide to building agents* — https://openai.com/business/guides-and-resources/a-practical-guide-to-building-ai-agents/
- Anthropic, *Effective context engineering for AI agents* — https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents
- OWASP, *GenAI Security Project* — https://genai.owasp.org/

**Current source takeaways:**
- External guidance changes quickly enough that refresh must be a blocking gate, not a nice-to-have note.
- Context/memory/handoff quality is a first-class design concern, so source deltas must be recorded as artifacts instead of chat recollection.
- Security guidance for agentic systems is evolving fast enough that OWASP-backed deltas can materially change downstream task design.

**Steps:**
1. Draft the registry template with source family, scope, trust tier, last checked date, and refresh trigger.
2. Draft the delta log template with `query`, `source`, `materiality`, `impacted module`, and `required follow-up`.
3. Write the canonical `research-refresh-protocol.md`.
4. Link the protocol from the project agent-system doc.

**Acceptance:**
- registry exists
- delta log exists
- refresh protocol defines blocking vs non-blocking changes
- future tasks can refresh sources without chat history

**Verification:**
- read the three artifacts and confirm each required field exists
- verify the project doc points to the new refresh protocol

---

## Task 2: Define the canonical role taxonomy

**Files:**
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/role-taxonomy-protocol.md`
- Modify: `/home/unnamed/project/mobile-odoo/docs/process/agent-system.md`

**Autonomous packet must include:**
- list of current and target roles
- role ownership classes
- definition/execution/assurance/governance split

**Research Refresh Gate:**
1. Refresh sources on personas, agent loops, orchestration patterns, and governance.
2. Record any new role families or collapse rules.

**Web source basis:**
- Microsoft, *AI personas* — https://learn.microsoft.com/en-us/azure/well-architected/ai/personas
- Microsoft, *Single-agent versus multiple-agent systems* — https://learn.microsoft.com/en-us/azure/cloud-adoption-framework/ai-agents/single-agent-multiple-agents
- Azure Architecture Center, *AI agent design patterns* — https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns

**Current source takeaways:**
- Personas/roles should stay few, explicit, and purpose-bound instead of exploding into dozens of fuzzy identities.
- Multi-agent separation should follow work shape and governance needs, not aesthetics.
- Role taxonomy must distinguish discussion personas from runtime-execution roles and independent assurance roles.

**Steps:**
1. Define canonical top-level role classes.
2. Define mandatory roles vs optional specialist roles.
3. Define which roles are profile-only and which require runtime lanes.
4. Define role ownership language for future task packets.

**Acceptance:**
- every role belongs to one class
- role names are non-overlapping
- governance and assurance roles are separated from execution

**Verification:**
- audit the taxonomy for duplicate authority
- verify each role has an explicit purpose and exit artifact

---

## Task 3: Define the role profile card schema

**Files:**
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/role-profile-protocol.md`
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/history/_vida-source/templates/role-profile-card.yaml`
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/history/_vida-source/tests/test_role_profile_schema.py`

**Autonomous packet must include:**
- required profile fields
- mandatory invariants
- disallowed ambiguity

**Research Refresh Gate:**
1. Refresh persona/profile guidance from OpenAI, Microsoft, Google.
2. Refresh any new guidance on tone, authority, permissions, or role safety.

**Web source basis:**
- OpenAI, *Prompting guide* — https://platform.openai.com/docs/guides/prompting
- Microsoft, *AI personas* — https://learn.microsoft.com/en-us/azure/well-architected/ai/personas
- Google, *Prompting strategies* — https://ai.google.dev/gemini-api/docs/prompting-strategies

**Current source takeaways:**
- Profile design should separate role identity from task-specific instructions.
- Tone and cognitive stance matter, but they must not silently grant authority or tools.
- Good profiles are compact, explicit, and evaluable rather than narrative-heavy.
- Role logic should eventually be promoted into agent definitions, instruction contracts, and prompt template configurations rather than living only in prose.

**Required profile fields:**
- `role_id`
- `role_class`
- `mission`
- `scope_boundary`
- `thinking_tone_axes`
- `reasoning_policy`
- `evidence_standard`
- `permissions_policy`
- `handoff_targets`
- `forbidden_substitutions`
- `output_contract`
- `verification_burden`
- `escalation_rules`
- `owasp_surfaces`
- `instruction_contract_ref`
- `prompt_template_config_ref`

**Steps:**
1. Define the schema in prose and YAML.
2. Define validation rules for required fields.
3. Create at least one example card for `coach`.
4. Create example cards for `writer`, `verifier`, and `ops-validator`.
5. Define how a role profile is rendered into an instruction contract and prompt template configuration without moving canonical logic into provider-specific config.

**Acceptance:**
- profile cards are machine-actionable
- tone is separated from permissions and authority
- schema is testable
- profile cards are explicitly upstream of instruction contracts rather than treated as the full runtime logic

**Verification:**
- schema test passes
- example profile cards validate

---

## Task 4: Define thinking-tone axes and cognitive stance vocabulary

**Files:**
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/thinking-tone-axes.md`
- Modify: `/home/unnamed/project/mobile-odoo/docs/framework/role-profile-protocol.md`

**Autonomous packet must include:**
- the tone axes list
- allowed values
- mapping guidance

**Research Refresh Gate:**
1. Refresh sources on prompting/persona guidance.
2. Refresh context on critique, uncertainty, persistence, and risk posture patterns.

**Web source basis:**
- Google, *Prompting strategies* — https://ai.google.dev/gemini-api/docs/prompting-strategies
- OpenAI, *Prompting guide* — https://platform.openai.com/docs/guides/prompting
- Microsoft, *Design best practices* — https://learn.microsoft.com/en-us/copilot/microsoft-365/employee-self-service/design-best-practices

**Current source takeaways:**
- Tone should be parameterized as stable behavioral axes instead of ad hoc prose.
- Critique, persistence, ambiguity handling, and verbosity should be steerable without redefining the role itself.
- Adaptive tone is useful, but only inside a bounded role identity and output contract.

**Tone axes to define:**
- `skepticism`
- `precision`
- `creativity`
- `persistence`
- `risk_tolerance`
- `verbosity`
- `warmth`
- `abstraction_level`

**Steps:**
1. Define each axis with allowed values and intent.
2. Define what each axis must not influence.
3. Provide example axis settings for key roles.
4. Add guidance for when a role should switch execution mode instead of changing identity.

**Acceptance:**
- tone axes are reusable across roles
- axes do not silently alter permissions
- profile cards can reference this vocabulary instead of ad hoc prose

**Verification:**
- example roles can be described unambiguously with the axes

---

## Task 5: Define the task classification model

**Files:**
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/task-classification-protocol.md`
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/history/_vida-source/scripts/task-classify.py`
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/history/_vida-source/tests/test_task_classification.py`

**Autonomous packet must include:**
- task classes
- risk dimensions
- mutation/read distinction
- ambiguity rules

**Research Refresh Gate:**
1. Refresh current guidance on single-agent vs multi-agent selection.
2. Refresh scaling guidance on parallelizable vs sequential tasks.

**Web source basis:**
- Google Research, *Towards a science of scaling agent systems* — https://research.google/blog/towards-a-science-of-scaling-agent-systems-when-and-why-agent-systems-work/
- Google Cloud, *Choose design patterns for agentic AI systems* — https://cloud.google.com/architecture/choose-design-pattern-agentic-ai-system
- Azure Architecture Center, *AI agent design patterns* — https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns

**Current source takeaways:**
- Task shape matters more than ideology: parallelizable tasks and sequential tasks benefit from different orchestration patterns.
- Classification must detect coupling, decomposability, and risk before any routing decision is made.
- Pattern selection is not merely architectural preference; it changes cost, latency, and correctness.

**Task classes must include at minimum:**
- `research`
- `spec`
- `bugfix`
- `feature`
- `refactor`
- `migration`
- `incident`
- `release`
- `docs/process/framework`

**Steps:**
1. Define classification inputs.
2. Define the output contract.
3. Define low/medium/high risk posture.
4. Implement a deterministic classifier helper.

**Acceptance:**
- one request maps to one primary task class
- risk posture is explicit
- ambiguous requests fail closed into clarification/spec flow

**Verification:**
- fixture tests for representative requests

---

## Task 6: Define the task score model

**Files:**
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/task-score-protocol.md`
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/history/_vida-source/scripts/task-score.py`
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/history/_vida-source/tests/test_task_score.py`

**Autonomous packet must include:**
- scoring inputs
- weighting model
- thresholds

**Research Refresh Gate:**
1. Refresh adaptive scaling guidance.
2. Refresh any new evidence about task decomposability or coordination costs.

**Web source basis:**
- Google Research, *Towards a science of scaling agent systems* — https://research.google/blog/towards-a-science-of-scaling-agent-systems-when-and-why-agent-systems-work/
- OpenAI, *Evaluation best practices* — https://platform.openai.com/docs/guides/evaluation-best-practices
- Anthropic, *How we built our multi-agent research system* — https://www.anthropic.com/engineering/built-multi-agent-research-system

**Current source takeaways:**
- Adaptive scaling should be evidence-driven rather than a fixed minimum-agent ideology.
- Prior effectiveness is a meaningful signal and should influence future routing and proof burden.
- Coordination cost must be modeled explicitly, otherwise the score will over-recommend fan-out.

**Scoring dimensions should include:**
- complexity
- risk
- coupling
- external volatility
- verification burden
- prior run effectiveness

**Steps:**
1. Define numeric inputs and thresholds.
2. Define how prior-run effectiveness alters the score.
3. Define when poor previous outcomes should increase verification instead of fan-out.
4. Implement test fixtures covering edge cases.

**Acceptance:**
- score explains why a task stayed single-agent or scaled out
- prior effectiveness is a first-class input

**Verification:**
- score fixtures produce stable, interpretable outputs

---

## Task 7: Define the adaptive agent-count policy

**Files:**
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/adaptive-agent-count-protocol.md`
- Modify: `/home/unnamed/project/mobile-odoo/vida/config/instructions/instruction-contracts.agent-system-protocol.md`
- Modify: `/home/unnamed/project/mobile-odoo/vida.config.yaml`
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/history/_vida-source/tests/test_adaptive_agent_count.py`

**Autonomous packet must include:**
- score thresholds
- escalation triggers
- de-escalation triggers
- minimum and maximum agent counts by task class

**Research Refresh Gate:**
1. Refresh sources on dynamic scaling and multi-agent effectiveness.
2. Refresh consensus vs arbiter guidance.

**Web source basis:**
- Google Research, *Towards a science of scaling agent systems* — https://research.google/blog/towards-a-science-of-scaling-agent-systems-when-and-why-agent-systems-work/
- Anthropic, *How we built our multi-agent research system* — https://www.anthropic.com/engineering/built-multi-agent-research-system
- Microsoft, *Single-agent versus multiple-agent systems* — https://learn.microsoft.com/en-us/azure/cloud-adoption-framework/ai-agents/single-agent-multiple-agents

**Current source takeaways:**
- Dynamic agent count is superior to a fixed floor when task score and prior effectiveness are available.
- More agents are not automatically better; sequential or tightly coupled tasks can degrade under excess fan-out.
- Arbiter paths must exist because disagreement sometimes indicates ambiguity, not the need for more lanes.

**Policy requirements:**
- single agent remains legal for low-score or tightly coupled work
- dynamic fan-out is based on score and prior effectiveness
- failed or contradictory outputs may trigger arbiter mode rather than more fan-out

**Steps:**
1. Define per-task-class min/max agent counts.
2. Define escalation rules after weak prior results.
3. Define disagreement handling and escalation ceilings.
4. Define when to choose `single -> dual -> triad -> quorum -> arbiter`.

**Acceptance:**
- no global hardcoded minimum agent count
- routing is explainable
- prior performance can increase or reduce agent count

**Verification:**
- test cases for low-score, high-score, sequential, and conflict-heavy tasks

---

## Task 8: Define the consensus and escalation policy

**Files:**
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/consensus-orchestration-protocol.md`
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/history/_vida-source/scripts/consensus-merge.py`
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/history/_vida-source/tests/test_consensus_merge.py`

**Autonomous packet must include:**
- allowed consensus modes
- conflict semantics
- escalation semantics

**Research Refresh Gate:**
1. Refresh consensus, debate, adjudicator, and verifier-pattern guidance.
2. Refresh any new warnings about consensus failure modes.

**Web source basis:**
- Azure Architecture Center, *AI agent design patterns* — https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns
- Wang et al., *Mixture-of-Agents* — https://arxiv.org/abs/2406.04692
- Google Research, *Improving multi-agent debate with sparse communication topology* — https://research.google/pubs/improving-multi-agent-debate-with-sparse-communication-topology/

**Current source takeaways:**
- Consensus quality depends on independence and merge design, not just on agent count.
- Debate and ensemble patterns can improve results, but only when the merge layer detects real disagreement and conflict shape.
- Verification and adjudication remain distinct from the consensus mechanism itself.

**Consensus modes to support:**
- `single_pass`
- `unanimous`
- `majority`
- `weighted_majority`
- `verifier_veto`
- `arbiter_tiebreak`

**Steps:**
1. Define which task classes use which consensus mode.
2. Define what counts as material disagreement.
3. Define when consensus is advisory vs blocking.
4. Implement the merge helper and tests.

**Acceptance:**
- consensus never silently replaces independent verification
- escalation paths are explicit

**Verification:**
- merge tests cover agreement, disagreement, veto, and tie-break

---

## Task 9: Define the task packet and autonomous handoff contract

**Files:**
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/autonomous-task-packet-protocol.md`
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/history/_vida-source/templates/autonomous-task-packet.md`
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/history/_vida-source/tests/test_task_packet_contract.py`

**Autonomous packet must include:**
- objective
- exact scope
- role profile reference
- source registry reference
- research refresh status
- inputs
- outputs
- proof burden
- stop conditions

**Research Refresh Gate:**
1. Refresh context engineering and handoff best practices.
2. Refresh any new guidance on artifact-driven execution.

**Web source basis:**
- Anthropic, *Effective context engineering for AI agents* — https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents
- OpenAI, *A practical guide to building agents* — https://openai.com/business/guides-and-resources/a-practical-guide-to-building-ai-agents/
- Azure Architecture Center, *AI agent design patterns* — https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns

**Current source takeaways:**
- Execution quality depends heavily on what enters the context window and what is externalized as artifacts.
- Handoffs must carry structured state instead of relying on long transcript inheritance.
- Autonomous packets must be complete enough to survive context resets and independent execution.

**Steps:**
1. Define the minimal packet schema.
2. Define packet variants for research, docs, framework, and implementation work.
3. Define what information is forbidden to omit.
4. Create tests or validators for packet completeness.

**Acceptance:**
- any child task can run from its packet without chat memory
- packets explicitly carry source-refresh state

**Verification:**
- validator rejects incomplete packets

---

## Task 10: Define context compaction and summary-handoff rules

**Files:**
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/context-handoff-protocol.md`
- Modify: `/home/unnamed/project/mobile-odoo/docs/framework/research-refresh-protocol.md`
- Modify: `/home/unnamed/project/mobile-odoo/docs/framework/autonomous-task-packet-protocol.md`

**Autonomous packet must include:**
- handoff artifact format
- summary requirements
- stale-summary detection rule

**Research Refresh Gate:**
1. Refresh context engineering guidance.
2. Refresh long-context and compaction guidance.

**Web source basis:**
- Anthropic, *Effective context engineering for AI agents* — https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents
- Anthropic, *Context management* — https://www.anthropic.com/news/context-management
- OpenAI, *Harness engineering* — https://openai.com/index/harness-engineering/

**Current source takeaways:**
- Long-horizon execution degrades when summaries are informal or stale.
- Compaction should preserve task-critical and source-refresh-critical facts in explicit artifacts.
- Good handoff summaries are operational documents, not chat recaps.

**Steps:**
1. Define the canonical handoff summary.
2. Define which facts must always survive compaction.
3. Define how research refresh status survives compaction.
4. Define when summary is stale and requires rebuild.

**Acceptance:**
- tasks no longer depend on transcript continuity
- source refresh evidence survives compaction

**Verification:**
- example handoff can restore execution context correctly

---

## Task 11: Define the role compatibility and forbidden substitution matrix

**Files:**
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/role-compatibility-matrix.md`
- Modify: `/home/unnamed/project/mobile-odoo/docs/framework/role-taxonomy-protocol.md`

**Autonomous packet must include:**
- roles under analysis
- risk policy
- substitution rules

**Research Refresh Gate:**
1. Refresh role separation guidance.
2. Refresh independent verification and approval guidance.

**Web source basis:**
- OpenAI, *A practical guide to building agents* — https://openai.com/business/guides-and-resources/a-practical-guide-to-building-ai-agents/
- Azure Architecture Center, *AI agent design patterns* — https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns
- Microsoft, *Single-agent versus multiple-agent systems* — https://learn.microsoft.com/en-us/azure/cloud-adoption-framework/ai-agents/single-agent-multiple-agents

**Current source takeaways:**
- Role separation is most valuable at authorship, review, verification, and approval boundaries.
- Some role collapse is acceptable for small work, but only when the proof burden is also lower.
- Compatibility rules must be explicit or the system will silently drift into unsafe shortcuts.

**Matrix must state explicitly:**
- which roles may collapse on low-risk tasks
- which roles must never collapse
- which role may advise but not approve
- which role may critique but not close

**Steps:**
1. Build the compatibility matrix.
2. Build the forbidden substitution matrix.
3. Add examples for low/medium/high-risk tasks.
4. Link the matrix from role profile docs.

**Acceptance:**
- `coach != verifier`
- `writer != reviewer` on non-trivial work
- security roles stay independent when required

**Verification:**
- manual review of representative task routes against the matrix

---

## Task 12: Define the verification burden matrix

**Files:**
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/verification-burden-matrix.md`
- Modify: `/home/unnamed/project/mobile-odoo/vida/config/instructions/command-instructions.implement-execution-protocol.md`

**Autonomous packet must include:**
- task class
- risk score
- required proofs

**Research Refresh Gate:**
1. Refresh evaluator-optimizer, verifier, and review guidance.
2. Refresh security verification sources where applicable.

**Web source basis:**
- OpenAI, *Evaluation best practices* — https://platform.openai.com/docs/guides/evaluation-best-practices
- OpenAI, *Safety in building agents* — https://platform.openai.com/docs/guides/agent-builder-safety
- Azure Architecture Center, *AI agent design patterns* — https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns

**Current source takeaways:**
- Verification should scale with task risk, external volatility, and prior result quality.
- Separate proof burden from implementation burden so the system can raise verification without rewriting architecture.
- Tool and action safety must be reflected in verification, not only in planning.

**Matrix must define:**
- required proofs by task class and score band
- which checks are local vs live
- which roles must sign off

**Steps:**
1. Define proof requirements for each class and risk level.
2. Define verification burden changes when prior results were weak.
3. Define closure-ready conditions.
4. Link the matrix to role profiles and routing docs.

**Acceptance:**
- the same task score can produce higher proof burden after weak prior outcomes
- closure cannot happen without required evidence

**Verification:**
- review matrix against representative scenarios

---

## Task 13: Define the OWASP security spine

**Files:**
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/owasp-agent-security-protocol.md`
- Create: `/home/unnamed/project/mobile-odoo/docs/process/owasp-vida-mapping.md`
- Modify: `/home/unnamed/project/mobile-odoo/docs/framework/verification-burden-matrix.md`
- Modify: `/home/unnamed/project/mobile-odoo/docs/framework/role-profile-protocol.md`

**Autonomous packet must include:**
- OWASP source set
- current security scope
- mapping targets

**Research Refresh Gate:**
1. Refresh OWASP GenAI, LLM, agentic application, ASVS, MASVS, MASTG, and SAMM material.
2. Record which controls affect role profiles, runtime gates, and mobile/backend verification.

**Web source basis:**
- OWASP, *GenAI Security Project* — https://genai.owasp.org/
- OWASP, *LLM Top 10* — https://genai.owasp.org/llm-top-10/
- OWASP, *ASVS* — https://owasp.org/www-project-application-security-verification-standard/
- OWASP, *MASVS* — https://mas.owasp.org/MASVS/
- OWASP, *MASTG* — https://mas.owasp.org/MASTG/
- OWASP, *SAMM* — https://owaspsamm.org/

**Current source takeaways:**
- OWASP needs to be mapped as a control matrix, not cited as background reading.
- Agentic risks, backend risks, and mobile risks must be separated but linked under one security spine.
- Security review cannot be collapsed into generic review or coach feedback.

**Mapping targets:**
- `OWASP GenAI` -> prompt/tool/memory/agentic risks
- `ASVS` -> backend/API/security verification
- `MASVS/MASTG` -> mobile verification
- `SAMM` -> SDLC/governance maturity

**Steps:**
1. Build a control matrix from OWASP sources to VIDA modules.
2. Define which roles own which OWASP surfaces.
3. Define when OWASP security gates are mandatory.
4. Add verification rules for security-sensitive tasks.

**Acceptance:**
- OWASP is a first-class security spine
- security is not collapsed into generic review

**Verification:**
- audit the matrix for coverage across agentic, backend, and mobile surfaces

---

## Task 14: Define observability and trace schema

**Files:**
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/adaptive-orchestration-trace-schema.md`
- Modify: `/home/unnamed/project/mobile-odoo/vida/config/instructions/runtime-instructions.run-graph-protocol.md`
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/history/_vida-source/tests/test_trace_schema.py`

**Autonomous packet must include:**
- required trace fields
- retention expectations
- route explanation requirements

**Research Refresh Gate:**
1. Refresh observability, harness, and trace-grading guidance.
2. Refresh governance guidance on audit trails where relevant.

**Web source basis:**
- OpenAI, *Harness engineering* — https://openai.com/index/harness-engineering/
- OpenAI, *Introducing AgentKit* — https://openai.com/index/introducing-agentkit/
- Microsoft, *Integrate, manage, and operate AI agents* — https://learn.microsoft.com/en-us/azure/cloud-adoption-framework/ai-agents/integrate-manage-operate

**Current source takeaways:**
- Agent systems need graph-level traces, not only final-output logs.
- Route and verification decisions should be inspectable from runtime artifacts.
- Operator surfaces are most useful when they expose why the system changed path, not only that it did.

**Trace fields must include:**
- task class
- task score
- prior effectiveness score
- chosen agent count
- chosen roles
- consensus mode
- blockers
- rework loops
- verification outcome
- source-refresh status

**Steps:**
1. Define the schema.
2. Define when fields are mandatory.
3. Align the schema with run-graph concepts.
4. Add tests or validators.

**Acceptance:**
- orchestration choices are explainable from the trace
- source refresh status is visible in traces

**Verification:**
- trace validator passes on fixture manifests

---

## Task 15: Define the eval and benchmark package

**Files:**
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/history/research/2026-03-08-agentic-role-profile-eval-plan.md`
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/agentic-eval-protocol.md`
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/history/_vida-source/tests/test_eval_protocol_contract.py`

**Autonomous packet must include:**
- benchmark questions
- grader criteria
- task families
- metrics

**Research Refresh Gate:**
1. Refresh eval and benchmark guidance from OpenAI and Google.
2. Refresh any new agent-auditing or grading patterns.

**Web source basis:**
- OpenAI, *Evaluation best practices* — https://platform.openai.com/docs/guides/evaluation-best-practices
- OpenAI, *PaperBench* — https://openai.com/research/paperbench/
- Google Research, *DS-STAR* — https://research.google/blog/ds-star-a-state-of-the-art-versatile-data-science-agent/

**Current source takeaways:**
- Adaptive orchestration needs benchmark and grader coverage, not just anecdotal wins.
- Baseline comparisons against simpler routes are required to justify extra coordination cost.
- Evals should measure route quality, proof quality, and cost-quality tradeoffs together.

**Metrics to cover:**
- task success
- cost per resolved task
- rework rate
- false-green rate
- consensus quality
- verifier overturn rate
- route regret
- security gate hit rate

**Steps:**
1. Define the benchmark set.
2. Define grader inputs and outputs.
3. Define route-selection evaluation.
4. Define ablations comparing single vs multi-agent paths.

**Acceptance:**
- adaptive orchestration can be measured against simpler baselines
- quality, cost, and safety tradeoffs are visible

**Verification:**
- eval protocol doc and test fixtures are internally coherent

---

## Task 16: Define rollout and migration order

**Files:**
- Create: `/home/unnamed/project/mobile-odoo/docs/process/agent-role-profile-rollout.md`
- Modify: `/home/unnamed/project/mobile-odoo/docs/process/agent-system.md`

**Autonomous packet must include:**
- migration sequence
- risk boundaries
- rollback triggers

**Research Refresh Gate:**
1. Refresh rollout, governance, and safety guidance where relevant.
2. Refresh any new guidance on agent approvals and intervention.

**Web source basis:**
- Microsoft, *Integrate, manage, and operate AI agents* — https://learn.microsoft.com/en-us/azure/cloud-adoption-framework/ai-agents/integrate-manage-operate
- Microsoft, *Project design and production environment strategy* — https://learn.microsoft.com/en-us/microsoft-copilot-studio/guidance/project-design-production-environment-strategy
- Azure Well-Architected, *Responsible AI in Azure Workloads* — https://learn.microsoft.com/en-us/azure/well-architected/ai/responsible-ai

**Current source takeaways:**
- Large agentic changes should roll out in controlled waves with governance, not big-bang replacements.
- Environment strategy and approval boundaries should be designed before pilot expansion.
- Responsible AI guidance maps naturally to pause, rollback, and human-approval gates.

**Steps:**
1. Define foundation-first rollout.
2. Define pilot-only routes.
3. Define feature flags or route flags if needed.
4. Define rollback conditions for the new control plane.

**Acceptance:**
- rollout can pause after any wave
- migration does not require a big-bang cutover

**Verification:**
- review rollout dependencies for hidden coupling

---

## Task 17: Define the proving wave

**Files:**
- Create: `/home/unnamed/project/mobile-odoo/docs/process/agent-role-profile-proving-wave.md`
- Modify: `/home/unnamed/project/mobile-odoo/docs/process/agent-role-profile-rollout.md`

**Autonomous packet must include:**
- pilot task selection rules
- measurement rules
- failure containment rules

**Research Refresh Gate:**
1. Refresh current guidance on proving, auditing, and runtime monitoring.
2. Refresh any new guidance on autonomous coding safety and evaluation.

**Web source basis:**
- OpenAI, *Harness engineering* — https://openai.com/index/harness-engineering/
- Google Research, *Towards a science of scaling agent systems* — https://research.google/blog/towards-a-science-of-scaling-agent-systems-when-and-why-agent-systems-work/
- Anthropic, *How we built our multi-agent research system* — https://www.anthropic.com/engineering/built-multi-agent-research-system

**Current source takeaways:**
- Proving waves should be treated as measurement exercises, not symbolic demos.
- Pilot tasks should be selected to expose differences between single-agent and adaptive multi-agent routes.
- Monitoring and auditing need to be part of the proving design from the start.

**Steps:**
1. Define pilot task selection criteria.
2. Define score bands for single/dual/triad routes.
3. Define proving metrics.
4. Define fail-stop rules if the pilot regresses quality or safety.

**Acceptance:**
- at least one proving wave can validate the new design on real tasks
- proving results can decide whether to expand or halt

**Verification:**
- pilot design is measurable and reversible

---

## Task 18: Prepare epic-slicing and TaskFlow formation guidance

**Files:**
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/history/research/2026-03-08-agentic-master-index.md`
- Create: `/home/unnamed/project/mobile-odoo/docs/framework/history/research/2026-03-08-agentic-epic-slicing-agent-instruction.md`
- Modify: `/home/unnamed/project/mobile-odoo/docs/framework/history/plans/2026-03-08-vida-autonomous-role-profiles-and-adaptive-orchestration-plan.md`

**Autonomous packet must include:**
- target epic waves
- child task boundaries
- dependency order
- acceptance bundle

**Research Refresh Gate:**
1. No external refresh required unless plan-to-epic slicing relies on changed routing/security guidance.
2. If yes, update the source delta log before slicing.

**Web source basis:**
- OpenAI, *A practical guide to building agents* — https://openai.com/business/guides-and-resources/a-practical-guide-to-building-ai-agents/
- Anthropic, *Effective context engineering for AI agents* — https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents
- Azure Architecture Center, *AI agent design patterns* — https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns

**Current source takeaways:**
- Good slicing preserves execution context, boundaries, and proof burden rather than only splitting by topic.
- Child tasks should carry enough artifact context to survive independent execution and compaction.
- Epic slicing should respect orchestration dependency order, not just topical grouping.

**Steps:**
1. Create a compact-safe master index that records `current scope`, `future scope`, `missing but referenced`, and the canonical reading order.
2. Create a prompt-style instruction for the next agent that defines how to slice the epic without losing research layers.
3. Convert this plan into epic wave candidates.
4. Define one child task packet per task above.
5. Define sequential vs parallel boundaries.
6. Define mandatory proof bundle per child task.

**Acceptance:**
- the plan can be sliced into a new epic without losing research findings
- each child task is independently executable
- the next agent has one canonical compact bridge and one prompt-ready slicing instruction

**Verification:**
- review the master index and slicing instruction against this plan and confirm no task depends on hidden chat context

---

## Execution Order

### Sequential foundation

1. Task 1
2. Task 2
3. Task 3
4. Task 4
5. Task 5
6. Task 6
7. Task 7
8. Task 8
9. Task 9
10. Task 10
11. Task 11
12. Task 12
13. Task 13

### Parallel-safe layer after foundation

14. Task 14
15. Task 15
16. Task 16

### Final preparation and proving

17. Task 17
18. Task 18

---

## Per-Task Autonomous Execution Template

Every epic child cut from this plan should contain:

1. `Goal`
2. `Files`
3. `Research Refresh Gate`
4. `Input Artifacts`
5. `Output Artifacts`
6. `Role Profile`
7. `Task Class`
8. `Task Score Inputs`
9. `Allowed Agent Count Range`
10. `Consensus Mode`
11. `Verification Burden`
12. `OWASP Surfaces`
13. `Stop Conditions`
14. `Acceptance`
15. `Verification`
16. `Research-derived decisions`
17. `Invalidation triggers`
18. `Local proof obligations`
19. `Fallback if research shifts`
20. `Assumptions register`
21. `Scope boundary`
22. `Non-goals`
23. `Dependency map`
24. `Failure modes / anti-patterns`
25. `Risk register`
26. `Change impact surface`
27. `Interface contract`
28. `Definition of done by artifact`
29. `Verification recipe`
30. `Rollback / reversal plan`
31. `Escalation map`
32. `Ownership map`
33. `Data / security classification`
34. `Traceability links`
35. `Examples / golden samples`
36. `Route rationale`
37. `Cost and latency budget`
38. `Freshness SLA`
39. `Terminology normalization`
40. `Open questions`

### Template intent

This template is intentionally over-complete. When slicing the new epic:

1. keep all 40 fields in the child-task template,
2. allow compact answers where risk is low,
3. never drop the reinforcement fields that affect research validity, proof burden, rollback, escalation, or security posture.

---

## Golden Sample Child-Task Packet

This is a filled example for future slicing. Use it as the canonical sample when converting Tasks 1-18 into real child tasks.

### Sample: Task 7 child packet

**Task Title:** Define the adaptive agent-count policy

1. `Goal`
   - Define and implement the policy that chooses single-agent, dual-lane, triad, quorum, or arbiter routing based on task score and prior effectiveness rather than a fixed agent minimum.
2. `Files`
   - Create: `/home/unnamed/project/mobile-odoo/docs/framework/adaptive-agent-count-protocol.md`
   - Modify: `/home/unnamed/project/mobile-odoo/vida/config/instructions/instruction-contracts.agent-system-protocol.md`
   - Modify: `/home/unnamed/project/mobile-odoo/vida.config.yaml`
   - Create: `/home/unnamed/project/mobile-odoo/docs/framework/history/_vida-source/tests/test_adaptive_agent_count.py`
3. `Research Refresh Gate`
   - Refresh Google scaling research, Anthropic multi-agent guidance, and Microsoft single-vs-multi-agent guidance.
   - Update the source delta log if routing rules or anti-pattern warnings changed materially.
4. `Input Artifacts`
   - `docs/framework/history/research/2026-03-08-agentic-role-profile-source-registry.md`
   - `docs/framework/history/research/2026-03-08-agentic-role-profile-source-delta-log.md`
   - `docs/framework/task-classification-protocol.md`
   - `docs/framework/task-score-protocol.md`
   - `docs/framework/role-taxonomy-protocol.md`
5. `Output Artifacts`
   - protocol doc
   - config changes
   - tests
   - one example route matrix or table
6. `Role Profile`
   - primary author: `framework-architect-writer`
   - independent verifier: `route-policy-verifier`
7. `Task Class`
   - `framework-routing`
8. `Task Score Inputs`
   - complexity: high
   - risk: medium-high
   - coupling: medium
   - external volatility: medium-high
   - prior effectiveness: neutral unless new route evidence exists
9. `Allowed Agent Count Range`
   - authoring lane: `1`
   - analysis/review lanes during execution of this task: `1-3`
   - runtime output of the task may define broader future ranges by task class
10. `Consensus Mode`
   - authoring: `single_pass`
   - independent review: `verifier_veto`
11. `Verification Burden`
   - high
12. `OWASP Surfaces`
   - excessive agency
   - unsafe autonomy expansion
   - verification bypass risk
13. `Stop Conditions`
   - stop if refreshed research materially contradicts the plan premise
   - stop if scoring cannot justify at least one lawful single-agent path
   - stop if the policy would silently bypass independent verification
14. `Acceptance`
   - no global hard minimum agent count remains
   - policy explains when one agent is lawful and preferred
   - policy explains when disagreement should trigger arbiter mode instead of more fan-out
15. `Verification`
   - review the protocol text
   - inspect the config diff
   - run the adaptive-agent-count tests
   - verify at least one low-score and one high-score route case
16. `Research-derived decisions`
   - use adaptive scaling driven by task shape and prior result quality
   - keep single-agent legal for tightly coupled or low-score work
   - distinguish fan-out escalation from arbiter escalation
17. `Invalidation triggers`
   - new Google evidence against current scaling heuristics
   - new Anthropic guidance showing better escalation rules
   - new runtime law that changes verification independence requirements
18. `Local proof obligations`
   - executable tests for score bands
   - doc-level route matrix
   - config-level policy exposure
19. `Fallback if research shifts`
   - reduce to a conservative policy: `single or dual only`
   - keep arbiter escalation but freeze higher fan-out until reevaluated
20. `Assumptions register`
   - task classification and task score protocols exist or are implemented first
   - config can expose route-law fields clearly
   - write ownership stays singular
21. `Scope boundary`
   - routing law, route policy, config exposure, and tests
22. `Non-goals`
   - no external tool integration
   - no UI/operator dashboard work
   - no implementation of new remote providers
23. `Dependency map`
   - depends on Tasks 1, 2, 5, and 6 from this plan
24. `Failure modes / anti-patterns`
   - “more agents = better” bias
   - hidden reintroduction of a hard minimum agent floor
   - using consensus as a substitute for verification
   - scoring logic that ignores prior performance
25. `Risk register`
   - route instability
   - excess orchestration cost
   - unsafe escalation rules
26. `Change impact surface`
   - protocol docs
   - route config
   - route tests
   - future proving-wave metrics
27. `Interface contract`
   - emit a stable route-law artifact that downstream verifier and rollout tasks can consume
28. `Definition of done by artifact`
   - protocol exists
   - config exposes policy
   - tests pass
   - example route table exists
29. `Verification recipe`
   - compare protocol rules against config fields
   - run tests
   - confirm single-agent, dual-lane, and arbiter cases exist
30. `Rollback / reversal plan`
   - revert to the last static route policy and disable adaptive count in config
31. `Escalation map`
   - if research changes materially: reopen spec for the route law
   - if security posture widens autonomy: require security review
   - if tests cannot prove safe single-agent routing: narrow scope and pause rollout
32. `Ownership map`
   - owner: framework architect/writer
   - reviewer: orchestrator doc reviewer
   - verifier: independent route-policy verifier
   - security owner: OWASP/security reviewer if autonomy expands
33. `Data / security classification`
   - framework control-plane logic
   - no direct PII
   - indirect security impact because routing changes execution authority
34. `Traceability links`
   - sources -> Task 7 plan entry -> protocol -> config -> tests -> proving wave
35. `Examples / golden samples`
   - one low-score example chooses single-agent
   - one medium-score example chooses dual-lane
   - one conflict-heavy example chooses arbiter mode
36. `Route rationale`
   - author sequentially because protocol/config/test surfaces are shared
   - verify independently because routing policy is safety-relevant
37. `Cost and latency budget`
   - documentation and tests may add moderate overhead
   - runtime policy must justify fan-out cost via score and prior effectiveness
38. `Freshness SLA`
   - refresh before implementation starts
   - refresh again before rollout or if a major scaling source changes
39. `Terminology normalization`
   - use `adaptive_agent_count`, `prior_effectiveness`, `arbiter`, `fan_out`, `verification_independence`
40. `Open questions`
   - exact threshold values
   - how strongly cost should influence the score
   - whether some task classes should cap at triad by default

---

## Non-Negotiable Invariants

1. No implementation task may proceed without research refresh when external guidance is relevant.
2. No role profile may mix tone, permissions, and approval authority into one implicit blob.
3. No consensus result may replace independent verification.
4. No security-sensitive task may bypass the OWASP mapping layer.
5. No child task may depend on transcript memory.
6. No rollout may happen without traces and evals.
7. No route law may remain implicit once implemented.

---

## Suggested First Child Tasks For The New Epic

If the epic is formed immediately after this plan, start with:

1. `Create source registry and refresh protocol`
2. `Define role taxonomy`
3. `Define role profile card schema`
4. `Define task classification and score model`
5. `Define adaptive agent-count policy`

These five tasks establish the contract that everything else depends on.

---

## Checkpoint: Ready For Epic Formation

**Created:** 2026-03-08
**Total:** 18 tasks
**Status:** Ready for slicing into a new framework epic

**Summary:**
- Tasks 1-4 establish the source, taxonomy, and profile foundation.
- Tasks 5-10 establish routing, scoring, scaling, consensus, and autonomous handoffs.
- Tasks 11-13 establish compatibility, proof burden, and OWASP security spine.
- Tasks 14-18 establish traces, evals, rollout, proving, and epic slicing.

**Next:** Create the new epic and convert each task above into a tracked child task with explicit TaskFlow blocks and proof surfaces.
-----
artifact_path: framework/plans/vida-autonomous-role-profiles-and-adaptive-orchestration-plan
artifact_type: plan
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/plans/vida-autonomous-role-profiles-and-adaptive-orchestration-plan.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: vida-autonomous-role-profiles-and-adaptive-orchestration-plan.changelog.jsonl
P26-03-09T21: 44:13Z
