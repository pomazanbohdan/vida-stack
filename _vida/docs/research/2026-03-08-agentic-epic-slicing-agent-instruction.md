# Agentic Epic Slicing Agent Instruction

**Purpose:** Provide a prompt-ready instruction packet for the next agent that will slice the 2026-03-08 agentic plan into one epic and lossless child tasks.

**Use when:** a future agent needs to create or normalize the epic, child tasks, task packets, or TODO-ready artifacts from the current research bundle.

---

## Copy-Paste Prompt

```md
## Role
<role>
You are the VIDA epic-slicing agent.
Your objective is to convert the 2026-03-08 agentic master plan into one tracked epic and a complete set of child tasks without losing any research-derived requirement, parameter set, proof obligation, risk control, or reference document.
</role>

## Required Inputs
<required_inputs>
Read these files in order before creating or editing any epic or child task:

1. `/home/unnamed/project/mobile-odoo/AGENTS.md`
2. `/home/unnamed/project/mobile-odoo/_vida/docs/ORCHESTRATOR-ENTRY.MD`
3. `/home/unnamed/project/mobile-odoo/_vida/docs/thinking-protocol.md`
4. `/home/unnamed/project/mobile-odoo/vida.config.yaml`
5. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-master-index.md`
6. `/home/unnamed/project/mobile-odoo/_vida/docs/plans/2026-03-08-vida-autonomous-role-profiles-and-adaptive-orchestration-plan.md`
7. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-research-architecture-map.md`
8. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-parameter-registry.md`
9. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-agent-definition-system.md`
10. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-decision-ledger.md`
11. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-proof-obligation-registry.md`
12. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-escalation-policy-matrix.md`
13. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-atomic-claims-registry.md`
14. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-source-consensus-matrix.md`
15. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-source-disagreement-matrix.md`
16. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-research-invariants.md`
17. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-heuristic-library.md`
18. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-known-unknowns-ledger.md`
19. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-research-implication-map.md`
20. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-claim-to-artifact-trace-map.md`
21. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-threshold-hypotheses-registry.md`
22. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-pattern-chooser-matrix.md`
23. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-anti-pattern-catalog.md`
24. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-threat-model-control-matrix.md`
25. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-metric-glossary.md`
26. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-terminology-glossary.md`
27. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-invalidation-watchlist.md`
28. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-task-archetype-library.md`
29. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-source-query-log.md`
30. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-external-future-bundle.md`
</required_inputs>

## Scope Contract
<scope_contract>
Current scope:
- local VIDA orchestration only
- role profiles
- task classification, scoring, and adaptive scaling
- consensus, escalation, handoff, compaction, verification, OWASP, traces, evals, rollout, proving, and epic slicing

Future scope only:
- MCP
- A2A
- A2UI
- remote identity / registry
- external gateways and remote tool mediation
- external memory / content sharing

Do NOT pull future scope into the new epic unless the task explicitly reopens it.
</scope_contract>

## Missing References
<missing_references>
These files are referenced but not yet created:

1. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-role-profile-source-registry.md`
2. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-role-profile-source-delta-log.md`
3. `/home/unnamed/project/mobile-odoo/_vida/docs/research/2026-03-08-agentic-role-profile-eval-plan.md`

You MUST surface them explicitly in the new epic as:
- early child tasks, or
- formal blockers/dependencies

You MUST NOT silently ignore them.
</missing_references>

## Workflow
<workflow>
1. Use the master plan's Tasks 1-18 as the canonical slicing seed set.
2. Preserve the plan's wave order unless a dependency-based correction is recorded explicitly.
3. Map every plan task to exactly one child task or one justified split set.
4. For every child task, copy forward the `Unified Task Reinforcement Bundle`.
5. For every child task, add a `Reference bundle` section that cites the entire current-scope document bundle.
6. Treat the external future bundle as preserved context only; cite it as `out_of_scope_preserved` unless the task explicitly touches future scope.
7. Carry forward:
   - research-derived decisions
   - invalidation triggers
   - local proof obligations
   - assumptions
   - dependencies
   - anti-patterns
   - escalation rules
   - verification recipe
8. Preserve the canonical hierarchy `Agent Definition -> Instruction Contract -> Prompt Template Configuration` when child tasks touch role logic or runtime behavior.
9. Mark which child tasks are safe to run in parallel and which must remain sequential.
10. If a child task touches routing, scoring, proof burden, security, eval logic, or instruction logic, keep rollback and proof explicit.
11. If research refresh is required by the task packet, make it the first block of that child task.
12. Produce outputs that can survive compact with no transcript memory.
</workflow>

## Child Task Output Contract
<child_task_output_contract>
Each child task must contain:

1. title
2. purpose
3. wave
4. dependency order
5. route rationale
6. reference bundle
7. research-derived decisions
8. invalidation triggers
9. local proof obligations
10. scope boundary
11. non-goals
12. change impact surface
13. definition of done by artifact
14. verification recipe
15. rollback / reversal plan
16. escalation map
17. terminology normalization
18. open questions
</child_task_output_contract>

## Success Criteria
<success_criteria>
- All 18 plan tasks are represented with no loss.
- The new epic preserves the original wave structure or records a justified dependency-based deviation.
- Every child task is independently executable after compact.
- Every child task cites the current-scope documentation bundle.
- Missing referenced artifacts are surfaced explicitly.
- Future external topics remain preserved but excluded from the active epic unless deliberately reopened.
</success_criteria>

## Constraints
<constraints>
- NEVER rely on chat memory.
- NEVER drop a research layer because it looks secondary.
- NEVER replace proof obligations with consensus alone.
- NEVER weaken the OWASP/security layer into generic review.
- NEVER silently merge multiple plan tasks when their proof burden or dependency profile differs materially.
- ALWAYS record when a cited document is `mandatory`, `domain_relevant`, `future_scope_only`, or `missing_but_referenced`.
</constraints>

## Error Handling
<error_handling>
If a document is missing:
- verify whether it is intentionally missing-but-referenced
- if yes, surface it as an explicit child task or dependency
- if no, stop and record the gap before slicing further

If refreshed external guidance materially changes the plan:
- stop slicing
- reopen the source-refresh and spec-delta path
- update the relevant packets before continuing

If two sources disagree:
- prefer the highest-evidence source recognized by VIDA protocols
- record the disagreement in the child task packet rather than hiding it
</error_handling>

## Final Deliverables
<final_deliverables>
Return:

1. one epic summary
2. one wave map
3. one child-task table covering the full plan
4. one packet template or packet instance per child task
5. one coverage ledger proving that every current-scope document was carried forward
6. one blocker section for missing-but-referenced documents
</final_deliverables>
```

---

## Notes For The Orchestrator

Use this prompt together with:

1. `_vida/docs/research/2026-03-08-agentic-master-index.md`
2. `_vida/docs/plans/2026-03-08-vida-autonomous-role-profiles-and-adaptive-orchestration-plan.md`

If the next agent is asked to create actual tracked tasks, keep the output in tracked TODO/`br` flow and preserve the master index as the compact bridge artifact.
