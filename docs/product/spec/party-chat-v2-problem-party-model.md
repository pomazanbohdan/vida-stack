# Party Chat Problem-Party Model

Status: active product law

Purpose: define how the Airtable-originated Party Chat council model is saved into the VIDA project canon and mapped onto the current `problem_party` runtime helper, project roles, project skills, project profiles, and project flow sets.

## 1. Source Of Truth

The immediate upstream source for this document is the Airtable `Vida` base, `Table 1`, records:

1. `Party Chat v2 Spec — Part 1/4`
2. `Party Chat v2 Spec — Part 2/4`
3. `Party Chat v2 Spec — Part 3/4`
4. `Party Chat v2 Spec — Part 4/4`

This Airtable source defines Party Chat as:

1. a governance protocol for multi-expert reasoning,
2. a compact council rather than unconstrained roleplay,
3. a runtime with explicit framing, critique, synthesis, verification, and action output,
4. a system that prefers the smallest useful council over maximum panel size,
5. a module that can integrate into Vida Stack as a decision, verification, and execution-planning layer.

## 2. Canonical Runtime Mapping

Within VIDA, Party Chat maps to the bounded `problem_party` runtime surface.

Mapping rules:

1. `problem_party` remains a bounded escalation-only decision layer.
2. Party Chat provides the richer board model, seat model, and role bundle used by that layer.
3. Party Chat does not replace writer, coach, verifier, approval, or closure law.
4. Party Chat must output structured artifacts, not free-form transcript residue.
5. Runtime integration must use project-owned extension registries rather than hardcoded ad hoc role names alone.

## 3. Council Model

### 3.1 Core Runtime Roles

The Party Chat v2 integration recognizes these project roles:

1. `party_chat_facilitator`
2. `party_chat_architect`
3. `party_chat_runtime_systems`
4. `party_chat_quality_verification`
5. `party_chat_delivery_cost`
6. `party_chat_product_scope`
7. `party_chat_system_analyst`
8. `party_chat_product_designer`
9. `party_chat_security_safety`
10. `party_chat_sre_observability`
11. `party_chat_qa_tester`
12. `party_chat_accessibility_ux`
13. `party_chat_data_contracts`
14. `party_chat_dx_tooling`
15. `party_chat_pm_process`
16. `party_chat_release_manager`

### 3.2 Board Sizes

Default board presets:

1. `small`
   - `party_chat_architect`
   - `party_chat_runtime_systems`
   - `party_chat_quality_verification`
   - `party_chat_delivery_cost`
2. `large`
   - `party_chat_architect`
   - `party_chat_runtime_systems`
   - `party_chat_quality_verification`
   - `party_chat_delivery_cost`
   - `party_chat_product_scope`
   - `party_chat_security_safety`
   - `party_chat_sre_observability`
   - `party_chat_data_contracts`
   - `party_chat_dx_tooling`
   - `party_chat_pm_process`
3. `modern_full`
   - `party_chat_system_analyst`
   - `party_chat_product_designer`
   - `party_chat_architect`
   - `party_chat_runtime_systems`
   - `party_chat_quality_verification`
   - `party_chat_qa_tester`
   - `party_chat_delivery_cost`
   - `party_chat_product_scope`
   - `party_chat_security_safety`
   - `party_chat_sre_observability`
   - `party_chat_accessibility_ux`
   - `party_chat_data_contracts`
   - `party_chat_dx_tooling`
   - `party_chat_pm_process`
   - `party_chat_release_manager`

The facilitator remains a separate orchestration posture and is attached to the manifest independently of the board role list.

## 4. Role And Authority Mapping

Each Party Chat project role must derive from a lawful framework base role:

1. facilitation and framing roles derive from `business_analyst`,
2. delivery/scope/process roles derive from `pm`,
3. verification-heavy roles derive from `verifier`,
4. architecture/runtime/data/dx specialist roles derive from `worker`,
5. review-heavy observability, accessibility, and conformance roles may derive from `coach`.

Project roles may narrow posture, but they must not grant stronger authority than their base role.

## 5. Skill Model

Party Chat integration uses project skills to express bounded council capability payloads.

Minimum skill surfaces:

1. council framing and synthesis,
2. architecture reasoning,
3. runtime systems reasoning,
4. verification and critique,
5. delivery and cost trade-off analysis,
6. optional specialty payloads for product scope, security, observability, data contracts, DX, and process.

## 6. Profile Model

Each Party Chat project role should resolve through one project profile so the runtime can attach:

1. the resolved role reference,
2. the bounded skill bundle,
3. the stable stance used during board rendering and synthesis.

The runtime helper should prefer profiles rather than raw role IDs whenever the matching profile exists.

## 7. Runtime Helper Contract

The `problem-party` helper must support:

1. manifest rendering for `small` and `large` boards,
2. role/profile resolution from project agent-extension registries,
3. inspectable dispatch and session planning,
4. structured synthesis output with decision / verification / execution packets,
5. bounded execute flow that writes session artifacts and final receipt,
6. receipt writing that updates `run_graph` for `problem_party`,
7. deterministic artifact paths under `.vida/logs/problem-party/`.

Required manifest payload shape:

1. `task_id`
2. `topic`
3. `board_size`
4. `round_count`
5. `facilitator`
6. `roles`
7. `problem_frame`
8. `constraints`
9. `options`
10. `conflict_points`
11. `decision`
12. `why_not_others`
13. `next_execution_step`
14. `confidence`
15. `budget_summary`
16. `role_bindings`
17. `profile_bindings`
18. `binding_validation`
19. `status`

## 8. Runtime Behavior Rules

1. start from the smallest lawful board,
2. keep facilitator separate from the board expert list,
3. resolve project profiles first and fall back to raw project roles only when the profile is absent,
4. emit structured JSON artifacts that survive compact,
5. mark `problem_party` as completed in the run graph when a receipt is written,
6. if the decision explicitly unblocks implementation, mark `writer` as `ready`.
7. if config or role/profile/flow resolution is invalid, fail closed with `status=invalid_manifest`.
8. if `execution_mode=single_agent`, runtime must require non-empty `single_agent.backend` and `single_agent.model`.
9. if `hard_cap_agents` cannot satisfy `min_experts` with a facilitator reserved, runtime must fail closed rather than silently shrinking to an invalid council.

## 9. Conversation-Mode Relation

Party Chat is not a replacement for scope/PBI conversational modes.

Rules:

1. ordinary scope work still routes through `scope_discussion`,
2. ordinary backlog/PBI work still routes through `pbi_discussion`,
3. Party Chat is a higher-intensity council surface for bounded conflict, architecture, verification, or cross-domain decision work,
4. its tracked handoff remains `problem_party` artifacts plus the normal TaskFlow route, not a separate ungoverned chat lane.

## 10. Framework Alignment

Party Chat is a project-level integration, but it is lawful only because the current VIDA framework canon already provides compatible bounded orchestration surfaces.

Framework alignment points:

1. bounded escalation instead of default free-form discussion:
   - `runtime-instructions/work.problem-party-protocol.md` already defines `problem_party` as optional, escalation-only, structured, and bounded by board size, round count, and token budget,
   - Party Chat uses this exact slot instead of inventing a second ungoverned discussion lane.
2. orchestration law already prefers structured conflict handling:
   - `instruction-contracts/core.orchestration-protocol.md` already says materially conflict-heavy but bounded problems should prefer `problem-party-protocol.md` over ad hoc debate and require a structured decision artifact before resuming the main flow,
   - Party Chat strengthens that artifact rather than bypassing it.
3. project-owned roles are already lawful under framework validation:
   - `runtime-instructions/work.project-agent-extension-protocol.md` already allows project roles, skills, profiles, and flow sets when they resolve through `vida.config.yaml` and pass fail-closed validation,
   - Party Chat therefore uses project council roles as validated extensions, not as hidden prompt-only authority.
4. lane-class selection already supports bounded role routing before deeper flow handoff:
   - `runtime-instructions/work.agent-lane-selection-protocol.md` already supports fixed or auto lane-class selection and bounded conversational role modes with lawful handoff,
   - Party Chat remains downstream of that law and does not replace ordinary scope or PBI discussion.
5. framework-owned safety boundaries remain intact:
   - Party Chat may enrich role composition and model routing,
   - but it still must not bypass route law, verification gates, approval gates, or single-writer ownership.

Framework-alignment rule:

1. Party Chat is lawful only as a stronger bounded projection of the existing `problem_party` framework slot,
2. it must remain inspectable through receipts, manifests, dispatch plans, and run-graph state,
3. it must not be used to invent a parallel authority system outside current VIDA routing and gate law.

## 11. Completion Proof

This integration is considered wired when:

1. the spec is saved in `docs/product/spec/**`,
2. Party Chat project roles, skills, profiles, and flow sets exist under `docs/process/agent-extensions/**`,
3. `vida.config.yaml` enables those project registries,
4. the active TaskFlow runtime family exposes a `problem-party` command,
5. the helper consumes the project registries when rendering board manifests,
6. runtime tests prove manifest rendering and receipt writing.

## 12. External Alignment

The current Party Chat integration is also aligned with current public multi-agent guidance and research rather than depending only on internal project preference.

Alignment points:

1. orchestrator plus specialized subagents:
   - Anthropic describes a lead-agent plus specialized subagent pattern for complex multi-agent work,
   - this supports Party Chat's facilitator plus role-seat model rather than a single flat prompt,
   - source: https://www.anthropic.com/engineering/multi-agent-research-system
2. lawful support for both single-agent and multi-agent execution:
   - OpenAI's current agent tooling explicitly supports orchestrating both single-agent and multi-agent workflows,
   - this supports the Party Chat requirement that one project setting may collapse the council to one agent while another may dispatch role seats across multiple agents,
   - source: https://openai.com/index/new-tools-for-building-agents/
3. runtime registry and metadata-based agent resolution:
   - Microsoft's multi-agent reference architecture describes dynamic agent discovery based on capabilities and metadata,
   - this supports Party Chat's use of validated framework roles, project roles, project profiles, and role-to-model bindings as a runtime resolution surface,
   - source: https://microsoft.github.io/multi-agent-reference-architecture/docs/reference-architecture/Patterns.html
4. task-adaptive topology over static one-model routing:
   - recent multi-agent research argues that orchestration topology and adaptive coordination can dominate raw single-model choice,
   - this supports Party Chat's board-size presets, single-agent vs multi-agent execution mode, and future DEI/topology evolution,
   - source: https://arxiv.org/abs/2602.16873

External-alignment rule:

1. Party Chat should remain config-gated and bounded under VIDA law,
2. external alignment justifies the runtime shape,
3. external alignment does not authorize bypassing framework review, verification, approval, or closure gates.

-----
artifact_path: product/spec/party-chat-v2-problem-party-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: docs/product/spec/party-chat-v2-problem-party-model.md
created_at: '2026-03-10T18:05:00+02:00'
updated_at: '2026-03-12T07:58:34+02:00'
changelog_ref: party-chat-v2-problem-party-model.changelog.jsonl
