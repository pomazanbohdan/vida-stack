# Problem-Party Protocol

Purpose: define a bounded multi-role discussion protocol for architecture, process, and conflict-heavy problem solving inside VIDA.

This protocol is optional and escalation-only. It is not the default path for routine implementation, research, or review work.

## Core Contract

1. Problem-party exists to improve decision quality when normal analysis/coach/verifier flow leaves a material conflict, ambiguity, or low-confidence architecture choice.
2. It must stay bounded by explicit board size, round count, and token budget.
3. It does not replace single-writer ownership, normal route receipts, or delegated verification.
4. It must output a structured decision artifact, not free-form chat residue.
5. Triggering problem-party is rule-based, not discretionary prose judgment.

## Allowed Triggers

Use problem-party only when at least one trigger row below evaluates to `yes` and no forbidden row blocks entry:

1. analysis/coach/verifier outputs conflict materially,
2. `merge_summary.conflict_flag=true` or `orchestrator_review_required=true`,
3. bug/spec/contract boundary is ambiguous after normal issue-contract flow,
4. framework/process remediation has multiple plausible systemic fixes,
5. the user explicitly requests multi-perspective discussion.

Forbidden as default when all are true:

1. the problem is routine and already decision-ready,
2. one bounded expert lane can answer it,
3. there is no material ambiguity or conflict,
4. the likely output would only restate an already accepted plan.

## Decision Matrix

### Entry Decision

Run `problem-party=small` when all are true:

1. at least one allowed trigger is true,
2. the active route is blocked on decision quality rather than missing raw evidence,
3. no single bounded expert lane is sufficient,
4. the problem can be framed as one bounded decision artifact.

Do not run problem-party when any are true:

1. the next correct action is simply "gather missing evidence",
2. route law, issue-contract law, approval law, or budget law already determines the next step,
3. a single verifier/coach/arbitration lane can resolve the issue without multi-role synthesis,
4. the user asked for direct execution and no material conflict exists.

Escalate from `small` to `large` only when all are true:

1. a completed small-board artifact exists,
2. the small-board artifact still leaves a material unresolved conflict,
3. the unresolved conflict crosses at least two of these dimensions:
   - architecture/runtime
   - quality/verification
   - delivery/cost
   - product/scope
   - security/safety
   - data/contracts
4. the next action still depends on a bounded decision artifact rather than raw evidence collection.

## Board Sizes

### Small Board

Use as the default problem-party board whenever the entry decision passes.

1. roles:
   - `architect`
   - `runtime_systems`
   - `quality_verification`
   - `delivery_cost`
2. default rounds: `1`
3. maximum rounds: `2`
4. target use:
   - architecture/process conflicts,
   - route ambiguity,
   - bounded framework design choices.

### Large Board

Use only when the escalation conditions above are satisfied or the user explicitly requests a broader multi-role board for one bounded decision.

1. roles:
   - `architect`
   - `runtime_systems`
   - `quality_verification`
   - `delivery_cost`
   - `product_scope`
   - `security_safety`
   - `sre_observability`
   - `data_contracts`
   - `dx_tooling`
   - `pm_process`
2. default rounds: `2`
3. maximum rounds: `3`
4. target use:
   - framework/platform design disputes,
   - major protocol changes,
   - cross-boundary product/framework ambiguity,
   - cases where the small board still leaves a material unresolved conflict.

## Output Artifact

Canonical runtime artifact:

1. `.vida/logs/problem-party/<task_id>.<topic>.json`

Required fields:

1. `task_id`
2. `topic`
3. `board_size`
4. `round_count`
5. `roles`
6. `problem_frame`
7. `constraints`
8. `options`
9. `conflict_points`
10. `decision`
11. `why_not_others`
12. `next_execution_step`
13. `confidence`
14. `budget_summary`

## Execution Rules

1. Start with the smallest board that can resolve the problem.
2. Escalate from `small` to `large` only when the small board remains materially unresolved.
3. Do not exceed the maximum rounds for the selected board.
4. Prefer one synthesis pass after each round over open-ended discussion.
5. Keep role prompts question-driven and bounded to one problem frame.
6. Preserve single-writer semantics: problem-party is a decision layer, not a mutation layer.
7. If compact/context compression is possible, persist the decision artifact before resuming the main TaskFlow.
8. When a problem-party decision receipt is written, runtime should update the run-graph so `problem_party` becomes a resumable node and the next writer step can become `ready` when the decision unblocks execution.

## Relationship To Existing VIDA Flow

1. Problem-party is stronger than a single bounded arbitration lane, but lighter than unconstrained open-ended team chat.
2. It may be entered only from these bounded upstream states:
   - issue-contract analysis that ended in bug/spec/contract ambiguity,
   - ensemble merge with `merge_summary.conflict_flag=true`,
   - verifier/coach path with `orchestrator_review_required=true`,
   - framework self-diagnosis or remediation analysis with multiple plausible systemic fixes.
3. It must not bypass:
   - route law,
   - budget blockers,
   - issue-contract gates,
   - delegated verification requirements.

## Canonical Helper

```bash
python3 problem-party.py render <task_id> "<topic>" --board small|large [--rounds N] [--problem-file PATH] [--output-dir DIR]
python3 problem-party.py synthesize <board_manifest.json> <role_notes.json> [--output PATH]
python3 problem-party.py receipt <task_id> <task_class> "<topic>" <decision_artifact.json>
```

## Anti-Patterns

1. Using problem-party for every non-trivial task.
2. Treating the role board as equal-authority voting on product truth.
3. Running large-board discussion without a bounded conflict trigger.
4. Leaving only prose/chat discussion without a decision artifact.
5. Using problem-party as a substitute for writer/coach/verifier lanes.

-----
artifact_path: config/runtime-instructions/problem-party.protocol
artifact_type: runtime_instruction
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/runtime-instructions.problem-party-protocol.md
created_at: 2026-03-07T21:28:00+02:00
updated_at: 2026-03-10T00:55:00+02:00
changelog_ref: runtime-instructions.problem-party-protocol.changelog.jsonl
