# VIDA 0.2 Semantic Freeze Spec

Purpose: define the exact `0.1` runtime semantics that `1.0` must preserve during the direct binary rewrite.

Status: canonical freeze artifact for direct `1.0` planning and downstream kernel specs.

Date: 2026-03-08

---

## 1. Core Freeze Decision

`0.1` remains the behavioral oracle.

`1.0` must preserve:

1. runtime law,
2. canonical vocabularies,
3. state transitions,
4. receipts and gate semantics,
5. resumability and proof behavior.

`1.0` must not preserve mechanically:

1. shell/Python helper boundaries,
2. `br` storage topology,
3. `.beads/issues.jsonl`,
4. `.vida/state/*` and `.vida/logs/*` path layout,
5. wrapper-specific CLI glue,
6. current provider-specific dispatch transport.

Compact rule:

`freeze semantics, not topology`

---

## 2. Scope Boundary

This freeze covers the semantically valuable layers already identified in:

1. `docs/framework/history/plans/2026-03-08-vida-semantic-extraction-layer-map.md`
2. `docs/framework/history/plans/2026-03-08-vida-0.1-to-1.0-direct-binary-transition-plan.md`
3. `docs/framework/history/plans/2026-03-08-vida-direct-1.0-local-spec-program.md`

In scope:

1. bootstrap router and instruction precedence,
2. activation classes and trigger matrix,
3. request intent and tracked-flow boundary,
4. command semantics,
5. lifecycle/state/review/approval vocabulary,
6. route/authorization/verification/approval law,
7. instruction runtime law,
8. worker packet and handoff law,
9. run-graph and resumability semantics,
10. compact/context-capsule semantics,
11. framework memory and self-diagnosis semantics,
12. context governance and web validation semantics,
13. observability, proof, and closure gates,
14. bridge/migration-relevant behavior.

Out of scope:

1. `MCP`,
2. `A2A`,
3. `A2UI`,
4. remote identity/registry,
5. gateways and external tool mediation,
6. remote memory/content sharing.

Those remain preserved only in:

1. `docs/framework/history/research/2026-03-08-agentic-external-future-bundle.md`

---

## 3. Semantic Preservation Law

`1.0` must preserve these product-level laws exactly:

1. execution behavior is allowlisted and fail-closed by default,
2. undefined behavior is forbidden,
3. task lifecycle state and execution telemetry are separate surfaces,
4. one canonical mutation path owns state transitions,
5. writer ownership is singular,
6. route metadata governs authorization, fallback, and closure,
7. receipts are first-class runtime artifacts, not optional logs,
8. compact/resume must work without chat memory,
9. durable lessons and anomalies belong in memory, not only in chat or transient logs,
10. external reality checks must be validated before decisions that depend on them,
11. proof-before-close is mandatory when route law requires it.

---

## 4. Frozen Semantic Domains

### 4.1 Bootstrap And Activation

Freeze:

1. lane resolution,
2. hard-stop boot rule,
3. instruction precedence root,
4. activation classes:
   - `always_on`
   - `lane_entry`
   - `triggered_domain`
   - `closure_reflection`
5. explicit trigger matrix for additional protocol activation.

Do not freeze:

1. markdown reread ritual as runtime mechanism,
2. broad markdown loading behavior,
3. duplicated policy bodies across docs.

### 4.2 Request Intent And Tracked-Flow Boundary

Freeze:

1. `answer_only`
2. `artifact_flow`
3. `execution_flow`
4. `mixed`
5. the tracked-flow boundary:
   - task flow is required for mutation, formal artifact production, or traceable multi-step execution
   - task flow is forbidden by default for pure `answer_only`

Do not freeze:

1. current pack-start shell commands,
2. current TaskFlow helper names.

### 4.3 Command Semantics

Freeze:

1. canonical operator actions,
2. compact output expectations,
3. blocker/error semantics,
4. the command-layer model:
   - `CL1 Intake`
   - `CL2 Reality And Inputs`
   - `CL3 Contract And Decisions`
   - `CL4 Materialization`
   - `CL5 Gates And Handoff`

Do not freeze:

1. `/vida-*` markdown file layout,
2. shell wrapper paths,
3. concrete `br` or script examples embedded in current command docs.

### 4.4 State, Lifecycle, Review, And Approval Vocabulary

Freeze:

1. task lifecycle states:
   - `open`
   - `in_progress`
   - `closed`
   - `deferred`
2. TaskFlow step states:
   - `todo`
   - `doing`
   - `done`
   - `blocked`
3. TaskFlow block end-result surface:
   - `done`
   - `partial`
   - `failed`
4. TSRP reconciliation classes:
   - `active`
   - `blocked`
   - `done_ready_to_close`
   - `stale_in_progress`
   - `open_but_satisfied`
   - `drift_detected`
   - `invalid_state`
   - `closed`
5. run-graph node set:
   - `analysis`
   - `writer`
   - `coach`
   - `problem_party`
   - `verifier`
   - `approval`
   - `synthesis`
6. run-graph node statuses:
   - `pending`
   - `ready`
   - `running`
   - `completed`
   - `blocked`
   - `failed`
   - `skipped`
7. review/approval governance states that already exist as semantic surfaces:
   - `review_passed`
   - `policy_gate_required`
   - `senior_review_required`
   - `human_gate_required`
   - `promotion_ready`
8. approval decisions:
   - `approved`
   - `rejected`

Do not freeze:

1. current JSONL/file-carrier storage implementation,
2. current log file paths,
3. current script command names used to mutate or inspect those states.

Storage decision note:

1. direct `1.0` authoritative storage is later fixed by the storage-kernel line to embedded `SurrealDB` on `kv-surrealkv`,
2. therefore legacy `SQLite` or JSONL carriers are not product-law candidates here.

### 4.5 Route, Authorization, Verification, And Approval Law

Freeze:

1. route-law fields are runtime law, not recommendations,
2. `analysis_required` blocks writer authorization until satisfied,
3. `analysis_receipt_required` blocks writer authorization until receipt exists,
4. `coach_required` inserts coach after writer and before final verifier,
5. `independent_verification_required` blocks closure-ready state until verifier evidence exists or explicit blocker exists,
6. `external_first_required` forbids silent internal bypass,
7. `dispatch_required` forbids silent local/manual substitution,
8. `web_search_required` filters execution to web-capable lanes and requires bounded live validation,
9. lawful local mutation under active routed execution requires a route-authorized local path or explicit escalation receipt,
10. approval-required routes must pass human approval before closure-ready synthesis,
11. omission of law-bearing values in effective route state is protocol-invalid and must fail closed.

### 4.6 Instruction Runtime Law

Freeze:

1. `Agent Definition` as the umbrella runtime object,
2. `Instruction Contract` as the canonical behavior source,
3. `Prompt Template Configuration` as the render/config layer only,
4. no-implied-behavior law,
5. explicit fallback and escalation contract,
6. versioned instruction behavior,
7. project/framework ownership split for instruction sources.

Do not freeze:

1. provider-specific prompt rendering as source of truth,
2. chat-memory-dependent behavior,
3. any single provider’s dispatch format.

### 4.7 Worker Packet And Handoff Semantics

Freeze:

1. bounded packet requirement,
2. explicit scope and stop condition,
3. required proof contract,
4. exact forbidden moves,
5. no-chat-memory continuation rule,
6. deterministic handoff packet as the unit of delegated execution.

Do not freeze:

1. current packet-rendering helper implementation,
2. current prompt file layout as the only runtime transport.

### 4.8 Run-Graph, Compact, And Resumability Semantics

Freeze:

1. run-graph is the node-level resumability ledger for one routed run,
2. run-graph is not a second task-state engine,
3. compact may happen at any time,
4. compact `pre` writes a context capsule,
5. compact `post` must hydrate before execution continues,
6. hydration failure is blocking,
7. `resume_hint` is a first-class resumability surface,
8. context capsule preserves:
   - `epic_id`
   - `epic_goal`
   - `task_id`
   - `task_role_in_epic`
   - `done`
   - `next`
   - `constraints`
   - `open_risks`
   - `acceptance_slice`

Do not freeze:

1. current `.vida/state/run-graphs/*.json` layout,
2. current context-capsule file path,
3. current shell helper names.

### 4.9 Framework Memory And Self-Diagnosis

Freeze:

1. framework memory is durable state, not chat memory,
2. memory kinds:
   - `lesson`
   - `correction`
   - `anomaly`
3. silent self-diagnosis is always-on when configured,
4. diagnosis does not hijack the current task by default,
5. diagnosis captures now and defers systematic fix until task boundary,
6. session reflection may capture new framework gaps,
7. instruction-layer clarity and instruction/runtime consistency are valid diagnosis targets.

Do not freeze:

1. current file-backed ledger implementation,
2. current capture helper command names.

### 4.10 Context Governance And Web Validation

Freeze:

1. context must be classified before use as execution evidence,
2. governed context fields:
   - `source_class`
   - `path`
   - `freshness`
   - `provenance`
   - `role_scope`
3. source classes:
   - `local_repo`
   - `local_runtime`
   - `overlay_declared`
   - `web_validated`
   - `external_connector`
4. web validation is mandatory when external volatility can change decisions,
5. live request/payload validation outranks local inference,
6. external facts without adequate validation must downgrade confidence or block completion.

Do not freeze:

1. current CLI/browser tooling for WVP,
2. current evidence marker formatting as storage topology.

### 4.11 Observability, Proof, And Closure Gates

Freeze:

1. operator/status surfaces must summarize canonical runtime artifacts rather than infer them ad hoc,
2. quality-health and verify gates are real closure conditions,
3. contradictions in execution proof can block finish,
4. stale or done-but-open work requires reconciliation before closure,
5. parity and regression expectations are first-class rewrite constraints.

Do not freeze:

1. current log file layout,
2. current shell-based gate script names,
3. current per-file telemetry organization.

---

## 5. Canonical Artifact Families To Preserve Semantically

These artifact families are semantically canonical even when their storage layout changes in `1.0`.

Must preserve:

1. route receipt,
2. lawful escalation receipt,
3. analysis receipt,
4. coach artifact,
5. rework handoff artifact,
6. verifier plan and verifier artifact,
7. approval receipt,
8. boot/verify receipt,
9. context capsule,
10. run-graph ledger,
11. framework-memory ledger,
12. context-governance ledger,
13. WVP evidence block,
14. execution verification/health evidence.

Semantic rule:

1. these artifacts are product behavior,
2. their current file paths are not.

---

## 6. Explicit Non-Semantic Topology To Discard

`1.0` must not treat the following as product law:

1. `bash docs/framework/history/_vida-source/scripts/...` as the canonical operator surface,
2. the shell/Python split,
3. `br --no-db` workarounds,
4. `.beads/issues.jsonl` as the long-term backend,
5. queue/log path quirks,
6. provider-specific route assembly,
7. current CLI worker transport as the only orchestration transport,
8. current file path layout under `.vida/state/*` and `.vida/logs/*`.

---

## 7. Freeze Outputs Required Before Broad Binary Coding

This semantic freeze is considered usable only when the following are treated as frozen downstream inputs:

1. canonical vocabulary sets,
2. canonical gate laws,
3. canonical artifact families,
4. semantic/non-semantic split,
5. unresolved ambiguities list.

Downstream specs unlocked by this document:

1. `docs/framework/history/plans/2026-03-08-vida-0.2-bridge-policy.md`
2. `docs/framework/history/plans/2026-03-08-vida-0.3-command-tree-spec.md`
3. `docs/framework/history/plans/2026-03-08-vida-0.3-state-kernel-schema-spec.md`
4. `docs/framework/history/plans/2026-03-08-vida-0.3-instruction-kernel-spec.md`
5. `docs/framework/history/plans/2026-03-08-vida-0.3-migration-kernel-spec.md`
6. `docs/framework/history/plans/2026-03-08-vida-0.3-route-and-receipt-spec.md`
7. `docs/framework/history/plans/2026-03-08-vida-0.3-parity-and-conformance-spec.md`

---

## 8. Open Ambiguities That Must Be Resolved Downstream

These are not blockers for semantic freeze, but they are blockers for later strict binary schemas:

1. `route receipt` is law-bearing but still lacks one centralized schema and canonical artifact contract,
2. `boot verify receipt` is referenced by multiple protocols but lacks one centralized schema,
3. lifecycle state, review state, approval state, and closure-ready state do not yet exist as one unified cross-surface state machine,
4. run-graph node metadata shape remains illustrative rather than fully specified,
5. framework-memory entry schema is underspecified beyond kind and summary counts,
6. `role_scope` vocabulary for context governance is not yet normalized into a strict enum,
7. observability/proof artifacts remain semantically fragmented across several ledgers and logs,
8. some current command-layer examples still leak `0.1` topology into semantic descriptions.

---

## 9. Source Basis

Primary local source basis used for this freeze:

1. `AGENTS.md`
2. `vida/config/instructions/agent-definitions.orchestrator-entry.md`
3. `vida/config/instructions/instruction-contracts.instruction-activation-protocol.md`
4. `vida/config/instructions/runtime-instructions.project-overlay-protocol.md`
5. `vida/config/instructions/command-instructions.command-layer-protocol.md`
6. `vida/config/instructions/instruction-contracts.orchestration-protocol.md`
7. `vida/config/instructions/system-maps.protocol-index.md`
8. `vida/config/instructions/runtime-instructions.beads-protocol.md`
9. `vida/config/instructions/runtime-instructions.taskflow-protocol.md`
10. `vida/config/instructions/runtime-instructions.task-state-reconciliation-protocol.md`
11. `vida/config/instructions/runtime-instructions.human-approval-protocol.md`
12. `vida/config/instructions/instruction-contracts.agent-system-protocol.md`
13. `vida/config/instructions/runtime-instructions.run-graph-protocol.md`
14. `vida/config/instructions/runtime-instructions.framework-memory-protocol.md`
15. `vida/config/instructions/diagnostic-instructions.silent-framework-diagnosis-protocol.md`
16. `vida/config/instructions/runtime-instructions.web-validation-protocol.md`
17. `vida/config/instructions/runtime-instructions.context-governance-protocol.md`
18. `vida/config/instructions/agent-definitions.protocol.md`
19. `docs/framework/templates/instruction-contract.yaml`
20. `docs/framework/templates/prompt-template-config.yaml`
21. `docs/framework/history/plans/2026-03-08-vida-semantic-extraction-layer-map.md`
22. `docs/framework/history/plans/2026-03-08-vida-0.1-to-1.0-direct-binary-transition-plan.md`
23. `docs/framework/history/plans/2026-03-08-vida-direct-1.0-local-spec-program.md`
24. `docs/framework/history/research/2026-03-08-agentic-agent-definition-system.md`
25. `docs/framework/history/research/2026-03-08-agentic-proof-obligation-registry.md`
26. `docs/framework/history/research/2026-03-08-agentic-metric-glossary.md`

Current source takeaways:

1. `0.1` already contains strong runtime law and canonical gate behavior.
2. The strongest stable surfaces are laws, vocabularies, receipts, and resumability semantics.
3. The most volatile or accidental surfaces are shell/Python transport, storage layout, and provider-specific dispatch plumbing.
4. `1.0` should therefore compile these semantics into binary-owned kernels instead of porting helper topology.

---

## 10. Final Rule

If a `0.1` behavior cannot be described as:

1. a law,
2. a vocabulary element,
3. a transition,
4. a receipt/artifact family,
5. a resumability invariant,
6. a proof or authorization rule,

then it should not be frozen as a `1.0` semantic requirement.

-----
artifact_path: framework/plans/vida-0.2-semantic-freeze-spec
artifact_type: plan
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/plans/vida-0.2-semantic-freeze-spec.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:30:56+02:00
changelog_ref: vida-0.2-semantic-freeze-spec.changelog.jsonl
P26-03-09T21: 44:13Z
