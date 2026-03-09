# VIDA State Kernel Next-Step After Compact Instruction

Purpose: give the next orchestrator an exact prompt-ready instruction for the post-command-tree artifact step: `state kernel schema spec`.

Use when: context was compacted and the next artifact to create is `docs/framework/history/plans/2026-03-08-vida-0.3-state-kernel-schema-spec.md`.

---

## Exact Objective

Create:

1. `/home/unnamed/project/mobile-odoo/docs/framework/history/plans/2026-03-08-vida-0.3-state-kernel-schema-spec.md`

The artifact must define:

1. authoritative state entities for direct `1.0`,
2. task/workflow lifecycle vocabulary and mutation law,
3. dependency, blocker, review, approval, and readiness state surfaces that belong in state,
4. run-graph and resumability state that belongs in state,
5. the boundary between state-owned facts and route/receipt-owned proofs,
6. the boundary between state semantics and discardable `0.1` storage topology,
7. state-level invariants,
8. state-level non-goals,
9. open ambiguities,
10. downstream contracts unlocked by the spec.

---

## Required Read Order

Read these in order before acting:

1. `/home/unnamed/project/mobile-odoo/AGENTS.md`
2. `/home/unnamed/project/mobile-odoo/vida/config/instructions/agent-definitions.orchestrator-entry.md`
3. `/home/unnamed/project/mobile-odoo/vida/config/instructions/instruction-contracts.thinking-protocol.md`
4. `/home/unnamed/project/mobile-odoo/vida.config.yaml`
5. `/home/unnamed/project/mobile-odoo/docs/framework/history/research/2026-03-08-agentic-master-index.md`
6. `/home/unnamed/project/mobile-odoo/docs/framework/history/research/2026-03-08-vida-direct-1.0-next-agent-compact-instruction.md`
7. `/home/unnamed/project/mobile-odoo/docs/framework/history/plans/2026-03-08-vida-0.3-command-tree-spec.md`
8. `/home/unnamed/project/mobile-odoo/docs/framework/history/plans/2026-03-08-vida-0.1-to-1.0-direct-binary-transition-plan.md`
9. `/home/unnamed/project/mobile-odoo/docs/framework/history/plans/2026-03-08-vida-semantic-extraction-layer-map.md`
10. `/home/unnamed/project/mobile-odoo/docs/framework/history/plans/2026-03-08-vida-0.2-semantic-freeze-spec.md`
11. `/home/unnamed/project/mobile-odoo/docs/framework/history/plans/2026-03-08-vida-0.2-bridge-policy.md`
12. `/home/unnamed/project/mobile-odoo/docs/framework/history/plans/2026-03-08-vida-direct-1.0-local-spec-program.md`
13. `/home/unnamed/project/mobile-odoo/vida/config/instructions/runtime-instructions.beads-protocol.md`
14. `/home/unnamed/project/mobile-odoo/vida/config/instructions/runtime-instructions.taskflow-protocol.md`
15. `/home/unnamed/project/mobile-odoo/vida/config/instructions/runtime-instructions.task-state-reconciliation-protocol.md`
16. `/home/unnamed/project/mobile-odoo/vida/config/instructions/runtime-instructions.run-graph-protocol.md`
17. `/home/unnamed/project/mobile-odoo/vida/config/instructions/runtime-instructions.human-approval-protocol.md`
18. `/home/unnamed/project/mobile-odoo/vida/config/instructions/instruction-contracts.agent-system-protocol.md`

Optional but preferred:

1. `/home/unnamed/project/mobile-odoo/vida/config/instructions/runtime-instructions.framework-memory-protocol.md`
2. `/home/unnamed/project/mobile-odoo/vida/config/instructions/diagnostic-instructions.silent-framework-diagnosis-protocol.md`

---

## Behavioral Inheritance Contract

The following laws MUST be carried forward into:

1. every future compact instruction,
2. every next-step instruction,
3. every child task packet,
4. every worker packet,
5. every worker prompt,
6. every state-kernel-adjacent implementation packet.

Mandatory carried behavior laws:

1. `AGENTS.md` is always the first read after compact.
2. Default lane is orchestrator unless worker-lane confirmation is explicit.
3. Undefined behavior is forbidden by default.
4. Protocols act as an allowlist, not guidance.
5. Never widen scope silently.
6. Never rely on chat memory as durable state.
7. Use workers as the primary bounded analysis/review fabric when supported.
8. Reuse eligible workers before falling back to local-only continuation under saturation.
9. Keep one writer/integrator ownership per shared write scope.
10. Separate authorship from verification when route law requires it.
11. No implied behavior in instructions.
12. `Instruction Contract` is the canonical logic source.
13. `Prompt Template Configuration` must not become the logic source.
14. Every prompt/packet must contain explicit fallback and escalation rules.
15. User-facing reports must be synthesized by the orchestrator, not raw worker dumps.
16. Evidence hierarchy must prefer live/receipt/runtime evidence over chat recollection.
17. If a process gap is found, use only a bounded workaround and record the gap; do not invent a permanent path.
18. Preserve the hierarchy:
    - `Agent Definition`
    - `Instruction Contract`
    - `Prompt Template Configuration`

Propagation rule:

1. any future instruction artifact created in this program must include either:
   - a `Behavioral Inheritance Contract` section carrying these laws, or
   - an explicit reference to this file plus the general compact instruction file.

---

## Worker Requirements For This Step

Before drafting the state kernel spec:

1. launch or reuse at least `2` bounded explorer lanes,
2. prefer `3` lanes because the artifact touches state, resumability, and route-adjacent boundaries,
3. keep all lanes read-only,
4. keep final synthesis and writing local.

Recommended blocking questions:

1. `Explorer A`: Which current `0.1` state vocabularies, transitions, blockers, and readiness classes are semantic and must survive?
2. `Explorer B`: Which current state/storage/file/layout surfaces are topology and must not be frozen into the future state kernel?
3. `Explorer C`: Which run-graph, resumability, review, approval, and route-adjacent facts belong in state versus later route/receipt law?

Each worker packet must carry:

1. one blocking question,
2. exact source list,
3. expected output shape,
4. no-edit restriction,
5. stop condition.

---

## Layered Working Method For This Step

Execute in this exact order:

1. restate the purpose of `state kernel schema spec`,
2. inventory current semantic state vocabularies,
3. inventory discardable storage/topology surfaces,
4. separate state-owned facts from route/receipt-owned proofs,
5. define entities, statuses, transitions, dependencies, blockers, review/approval state, and resumability state,
6. define dependencies on command, instruction, route/receipt, and migration kernels,
7. define state-level invariants and non-goals,
8. record open ambiguities,
9. update compact bridge so the next artifact becomes `instruction kernel spec`,
10. create/update the next exact-step instruction for that following artifact,
11. report to the user in explanatory prose.

---

## Output Requirements

After writing the artifact, report to the user:

1. what was created,
2. why `state kernel schema spec` had to come immediately after the command tree,
3. what state-model problem it solves in the larger `1.0` program,
4. what remains unresolved,
5. that the next exact artifact is `instruction kernel spec`,
6. which workers were used and for what bounded questions,
7. that behavioral inheritance was propagated into the next-step artifact.

---

## Hard Constraints

1. Do not reopen broad architecture debate.
2. Do not start Rust implementation.
3. Do not silently widen into external future-scope domains.
4. Do not let current `br`/JSONL/file layout become future product law.
5. Do not let receipt payloads swallow state facts that should remain authoritative state.
6. Do not let state schema redefine route/approval/verification proof law that belongs later.
7. Do not omit behavioral inheritance from future instructions created during this step.
-----
artifact_path: framework/research/vida-state-kernel-next-step-after-compact-instruction
artifact_type: framework_research_doc
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/research/vida-state-kernel-next-step-after-compact-instruction.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: vida-state-kernel-next-step-after-compact-instruction.changelog.jsonl
P26-03-09T21: 44:13Z
