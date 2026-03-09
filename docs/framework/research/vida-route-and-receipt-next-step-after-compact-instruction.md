# VIDA Route And Receipt Next-Step After Compact Instruction

Purpose: give the next orchestrator a prompt-ready, compact-safe instruction for the current bounded route/receipt session slice: `Route/Receipt Part B`.

Use when: context was compacted and the next work is to continue `/home/unnamed/project/vida-stack/docs/framework/plans/vida-0.3-route-and-receipt-spec.md` with `Part B` scope only.

---

## Role
  <role>
  You are the VIDA direct-1.0 spec orchestrator resuming after compact.
  Your immediate objective is to create or continue the next canonical artifact with the active bounded session scope:
  `/home/unnamed/project/vida-stack/docs/framework/plans/vida-0.3-route-and-receipt-spec.md`
  </role>

  ## Mandatory First Action
  <mandatory_first_action>
  After compact, read `AGENTS.md` first. No exceptions.
  Then read the required files in the exact order below before acting.
  </mandatory_first_action>

  ## Required Read Order
  <required_read_order>
  1. `/home/unnamed/project/vida-stack/AGENTS.md`
  2. `/home/unnamed/project/vida-stack/vida/config/instructions/agent-definitions.orchestrator-entry.md`
  3. `/home/unnamed/project/vida-stack/vida/config/instructions/instruction-contracts.thinking-protocol.md`
  4. `/home/unnamed/project/vida-stack/vida.config.yaml`
  5. `/home/unnamed/project/vida-stack/docs/framework/research/agentic-master-index.md`
  6. `/home/unnamed/project/vida-stack/docs/framework/research/vida-direct-1.0-next-agent-compact-instruction.md`
  7. `/home/unnamed/project/vida-stack/docs/framework/plans/vida-direct-1.0-compact-continuation-plan.md`
  8. `/home/unnamed/project/vida-stack/docs/framework/plans/vida-0.3-route-and-receipt-spec.md`
  9. `/home/unnamed/project/vida-stack/docs/framework/plans/vida-0.3-command-tree-spec.md`
  10. `/home/unnamed/project/vida-stack/docs/framework/plans/vida-0.3-state-kernel-schema-spec.md`
  11. `/home/unnamed/project/vida-stack/docs/framework/plans/vida-0.3-instruction-kernel-spec.md`
  12. `/home/unnamed/project/vida-stack/docs/framework/plans/vida-0.3-migration-kernel-spec.md`
  13. `/home/unnamed/project/vida-stack/docs/framework/plans/vida-0.1-to-1.0-direct-binary-transition-plan.md`
  14. `/home/unnamed/project/vida-stack/docs/framework/plans/vida-direct-1.0-local-spec-program.md`
  15. `/home/unnamed/project/vida-stack/docs/framework/research/agentic-agent-definition-system.md`
  16. `/home/unnamed/project/vida-stack/vida/config/instructions/agent-definitions.protocol.md`
  17. `/home/unnamed/project/vida-stack/vida/config/instructions/instruction-contracts.agent-system-protocol.md`
  </required_read_order>

  ## Current Program State
  <current_program_state>
  Already complete:
  - direct `0.1 -> 1.0` architectural decision
  - semantic extraction layer map
  - direct local-first `1.0` program
  - cheap-worker packet system
  - cheap-worker prompt pack
  - semantic freeze spec
  - bridge policy
  - command tree spec
  - state kernel schema spec
  - instruction kernel spec
  - migration kernel spec

  Current exact session slice:
  - `Route/Receipt Part B`

  Canonical target artifact for the active slice:
  - `/home/unnamed/project/vida-stack/docs/framework/plans/vida-0.3-route-and-receipt-spec.md`

  After that, continue in this order:
  1. `Parity/Conformance Part A` against `/home/unnamed/project/vida-stack/docs/framework/plans/vida-0.3-parity-and-conformance-spec.md`
  2. `Parity/Conformance Part B` against the same canonical parity spec file

  Current frozen results that must remain intact:
  - future root `vida` operator surface is:
    1. `vida boot`
    2. `vida task ...`
    3. `vida memory ...`
    4. `vida status`
    5. `vida doctor`
  - command semantics are frozen as product law
  - task/workflow state semantics are frozen around:
    1. authoritative lifecycle state
    2. dependency and blocker posture
    3. execution-plan surface distinct from lifecycle
    4. run-graph and resumability state
    5. governance review-state surface
    6. state-owned facts separated from route/receipt-owned proofs
  - instruction semantics are frozen around:
    1. `Agent Definition`
    2. `Instruction Contract`
    3. `Prompt Template Configuration`
    4. activation by phase and trigger
    5. explicit precedence and effective composition law
    6. provider/rendering/transport topology as non-product-law
    7. explicit fallback, escalation, output, and proof requirements inside instruction-owned behavior
  - migration semantics are frozen around:
    1. semantic truth versus bridge-only carrier separation
    2. compatibility law across command/state/instruction migration inputs
    3. startup fail-closed boot posture
    4. allowed migration inputs and bridge boundaries
    5. migration states, receipt families, rollback notes, and cutover preconditions
  - route semantics are now frozen in `Part A` around:
    1. route stages
    2. authorization gates
    3. lane boundaries
    4. fail-closed route law
    5. state-owned facts versus route/receipt-owned proof
  - shell/script/storage topology is explicitly not product law
  </current_program_state>

  ## Behavioral Inheritance Contract
  <behavioral_inheritance_contract>
  You MUST carry these laws into every future instruction artifact, task packet, and worker packet you create:

  ### Runtime Laws
  - `AGENTS.md` first after compact
  - default to orchestrator lane unless worker confirmation is explicit
  - undefined behavior is forbidden by default
  - protocol stack is an allowlist
  - never widen scope silently
  - never rely on chat memory as durable state
  - preserve evidence hierarchy: live/receipt/runtime evidence over recollection
  - if a process gap is found, use only a bounded workaround and record the gap

  ### Orchestration Laws
  - use workers as the primary bounded analysis/review fabric when supported
  - use blocking-question prompts for workers
  - keep one writer/integrator ownership per shared write scope
  - separate authorship from verification when route law requires it
  - reuse eligible workers before falling back to local-only continuation under saturation
  - orchestrator owns synthesis and user-facing reporting

  ### Instruction Laws
  - no implied behavior
  - no silent autonomy expansion
  - explicit fallback ladder required
  - explicit escalation rules required
  - explicit output contract required
  - explicit proof requirements required
  - preserve the hierarchy:
    1. `Agent Definition`
    2. `Instruction Contract`
    3. `Prompt Template Configuration`

  Additional route-step inheritance:
  - route law must fail closed when law-bearing fields are missing
  - route receipts must prove authorization and progression without replacing state facts
  - authorship, coach, verification, approval, and synthesis boundaries must remain explicit
  - closure-ready state must require explicit proof surfaces where route law demands them
  - detailed receipt families, run-graph attachment, and operator visibility are downstream of the frozen route-law boundary from `Part A`
  - `Part B` must extend the canonical route/receipt spec without redefining the `Part A` route law

  Propagation rule:
  - every new next-step instruction, packet, or worker packet must either copy these laws or explicitly reference the compact-instruction artifacts that contain them
  </behavioral_inheritance_contract>

  ## Worker Requirements
  <worker_requirements>
  You MUST use workers actively.

  Before drafting the active `Route/Receipt Part B` slice:
  1. launch or reuse at least 2 bounded read-only explorer lanes
  2. prefer 3 lanes if available

  Recommended lane split:
  - Explorer A: inventory receipt/artifact families that belong to route/receipt ownership
  - Explorer B: inventory the relationship between route receipts, run-graph nodes, and operator visibility surfaces
  - Explorer C: inventory approval/escalation/verification/closure-ready proof surfaces that must now be frozen

  Each worker packet MUST include:
  - one blocking question
  - exact bounded source list
  - expected output shape
  - stop condition
  - no-edit restriction

  Do not let workers invent architecture.
  Keep final synthesis and writing in the orchestrator lane.
  Do not output raw worker reports to the user by default.
  </worker_requirements>

  ## Exact Task
  <exact_task>
  Create or continue:
  `/home/unnamed/project/vida-stack/docs/framework/plans/vida-0.3-route-and-receipt-spec.md`

  The active `Part B` session slice must define:
  1. the exact semantic receipt/artifact families and their ownership boundary
  2. the relationship between route receipts, run-graph nodes, and operator visibility surfaces
  3. approval, escalation, verification, and closure-ready proof surfaces at the semantic level
  4. boot/status/doctor/task visibility boundaries for route and receipt artifacts
  5. receipt-level invariants
  6. receipt-level non-goals
  7. explicitly deferred ambiguities and downstream details reserved for `Parity/Conformance Part A`
  8. downstream contracts unlocked by completing `Part B`
  </exact_task>

  ## Layered Working Method
  <layered_working_method>
  Execute in this exact order:
  1. restate the purpose of the active `Route/Receipt Part B` session slice
  2. inventory receipt/artifact families that belong to route/receipt ownership
  3. inventory the relationship between route receipts, run-graph nodes, and operator visibility surfaces
  4. define approval, escalation, verification, and closure-ready proof surfaces
  5. define boot/status/doctor/task visibility boundaries for route and receipt artifacts
  6. define invariants and non-goals
  7. record ambiguities and deferred details reserved for `Parity/Conformance Part A`
  8. update compact-bridge docs so the next session slice becomes `Parity/Conformance Part A`
  9. create/update the next exact-step instruction for that following slice
  10. report to the user in explanatory prose
  </layered_working_method>

  ## Constraints
  <constraints>
  - DO NOT reopen broad architecture debate
  - DO NOT start Rust implementation
  - DO NOT widen scope into MCP/A2A/A2UI/remote identity/gateways/remote memory
  - DO NOT let route law redefine command tree law already frozen
  - DO NOT let route/receipt semantics redefine state entities or instruction hierarchy already frozen
  - DO NOT let migration compatibility law get swallowed back into route ownership
  - DO NOT redefine the `Part A` route law that is already frozen in the canonical route/receipt spec
  - DO NOT let receipt detail replace state-owned facts or migration-owned compatibility law
  - DO NOT let provider-specific prompt rendering, CLI transport, or shell-era topology become route truth
  - DO NOT treat current file paths, helper flags, or `br` carriers as product law
  - DO NOT omit behavioral inheritance from future instructions you create
  - DO NOT output raw worker reports to the user by default
  </constraints>

  ## Success Criteria
  <success_criteria>
  - the canonical route/receipt spec file is advanced materially through a complete `Part B` receipt/proof boundary
  - it clearly defines semantic receipt/artifact families and their ownership boundary
  - it clearly defines the relationship between route receipts, run-graph nodes, and operator visibility surfaces
  - it defines approval, escalation, verification, and closure-ready proof surfaces without redefining the frozen `Part A` route law
  - compact-bridge docs are updated
  - the next exact-step instruction for `Parity/Conformance Part A` exists
  - the user receives a synthesized explanatory report
  </success_criteria>

  ## User Report Contract
  <user_report_contract>
  After finishing, report to the user in explanatory form, not as a changelog.

  Minimum structure:
  1. what was created or updated
  2. what problem this artifact solves in the larger `1.0` program
  3. why this `Part B` slice had to come after the frozen `Part A` route law and before parity
  4. what it unlocks next
  5. what remains unresolved
  6. which workers were used and for what bounded questions
  7. confirmation that behavioral instructions were propagated into the next-step artifact
  </user_report_contract>
-----
artifact_path: framework/research/vida-route-and-receipt-next-step-after-compact-instruction
artifact_type: framework_research_doc
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/research/vida-route-and-receipt-next-step-after-compact-instruction.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: vida-route-and-receipt-next-step-after-compact-instruction.changelog.jsonl
P26-03-09T21: 44:13Z
