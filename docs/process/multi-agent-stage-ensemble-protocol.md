# Multi-Agent Stage Ensemble Protocol

Status: proposed follow-up project process architecture

Purpose: define the project-side architecture for running multiple independent
agent or model attempts for a single TaskFlow task stage, then consolidating the
results through one authoritative validator before TaskFlow state is updated.

## Context

The current runtime can dispatch development-team lanes and can select internal
or external carriers from configuration. The missing layer is finer grained than
a task and broader than a single carrier: one task stage may need several
independent attempts from different internal agents, external CLI carriers, or
model profiles before the main orchestrator accepts a canonical result.

This protocol records the follow-up architecture requested after the active
codebase refactoring epic. It is project process guidance until promoted into
generic runtime owners through `docs/process/generic-runtime-protocol-promotion-plan.md`.

## External Architecture Inputs

The design is aligned with these industry patterns:

1. Orchestrator-worker multi-agent systems: a lead agent decomposes work and
   delegates independent subagents in parallel, then compiles the result.
2. Agent task/artifact protocols: remote agents operate on stateful tasks and
   return immutable or append-only artifacts rather than directly owning the
   caller's canonical state.
3. Async task execution: long-running work needs streaming, polling, or
   notification-style status transitions independent from the caller session.
4. Handoffs and guardrails: routing to specialists and validation of inputs,
   outputs, and tool results must be explicit and schema-backed.

## Core Model

TaskFlow must grow a stage-level ensemble layer:

```text
Task -> Stage -> Attempt(s) -> Artifact(s) -> ConsolidationReceipt -> StageResult
```

The canonical TaskFlow task remains the authority. Individual agents never
directly close a task, replace notes, or mutate canonical implementation state.
They produce attempts. The orchestrator or a configured consolidator validates
those attempts and records the accepted stage result.

## Stage Policy

Each task stage may define an attempt policy:

```yaml
task_stage_policies:
  analysis:
    strategy: independent_attempts
    attempts:
      - backend: vibe_cli
        model_profile: deep
      - backend: qwen_cli
        model_profile: critic
    consolidator:
      backend: primary_orchestrator
      action: validate_and_record_stage_result
```

Required policy fields:

1. `stage_id`: `analysis`, `design`, `implementation`, `review`, `proof`, or a
   project-defined extension.
2. `strategy`: `single`, `independent_attempts`, `competitive_patch`, or
   `review_panel`.
3. `attempts`: ordered or unordered carrier/model profile assignments.
4. `isolation`: `read_only`, `patch_artifact`, `isolated_worktree`, or
   `proof_only`.
5. `consolidator`: the authoritative validator for the stage.
6. `freshness_boundary`: task revision, git revision, runtime binding, and
   optional packet hash that the attempt must match.

## Task Research Intake Modes

Every task research stage must model the two external advisory modes separately:

1. `external_readonly_complete`: an external carrier completes the analysis,
   specification, review, or proof-diagnosis slice and returns a structured
   report. This mode is read-only and may only become canonical after root
   validation and a consolidation receipt.
2. `external_patch_proposal`: an external carrier prepares a patch proposal,
   proposed diff or file plan, verification commands, and rollback notes. The
   root orchestrator applies or rejects the proposal, runs proof, commits,
   performs only explicitly authorized publication, and closes TaskFlow.

The research intake record must include both mode decisions for the task. A mode
may be `submitted`, `running`, `produced`, `accepted`, `rejected`, `stale`,
`failed`, or `not_run`. `not_run` requires a reason, such as no independent
question, no lawful owned path, unavailable carrier, duplicate expected output,
or a task risk tier too low for an extra external attempt. Complex,
high-ambiguity, or high-risk tasks should run both modes in parallel before the
consolidator records the canonical stage result.

## Attempt Ledger

Runtime state should persist a `TaskAttempt` record:

```json
{
  "attempt_id": "task-id:analysis:vibe_cli:001",
  "task_id": "task-id",
  "stage_id": "analysis",
  "todo_id": "optional-todo-id",
  "backend": "vibe_cli",
  "model_profile": "deep",
  "mode": "read_only",
  "base_git_rev": "HEAD",
  "allowed_paths": ["crates/vida/src/**"],
  "forbidden_mutations": ["task_close", "task_notes_replace", "main_worktree_patch"],
  "status": "running"
}
```

Allowed statuses:

1. `submitted`
2. `running`
3. `produced`
4. `validating`
5. `accepted`
6. `partially_accepted`
7. `rejected`
8. `stale`
9. `failed`
10. `consumed`

## Attempt Final Report Contract

Every agent attempt, whether accepted or rejected, must return a final report
that includes:

1. changed files or read-only scope reviewed,
2. production change summary or explicit `production_changed: false`,
3. tests/proofs run with pass/fail/not-run status,
4. residual risks,
5. `tokens_used`,
6. `steps_taken`,
7. `tool_calls_used`.

If exact token usage is not exposed by the host runtime, the attempt must write
`tokens_used: not_exposed_by_host`. It must not estimate tokens unless the
runtime provides an explicit estimate field. Step and tool-call counts must be
reported from the attempt's own action log.

For cheap executor lanes selected as `gpt-5.4-mini`, use the highest available
reasoning effort by default. The consolidator still treats the attempt as
untrusted until a stronger validator or root orchestrator verifies source
fidelity, public-surface proof, and false-green risk.

## Attempt Artifact Contract

Every attempt must return structured output:

```json
{
  "schema_version": "stage-attempt-v1",
  "attempt_id": "task-id:analysis:vibe_cli:001",
  "task_id": "task-id",
  "stage_id": "analysis",
  "observed_facts": [],
  "hypotheses": [],
  "related_files": [],
  "changed_files": [],
  "patch_ref": null,
  "proof_commands": [],
  "proof_results": [],
  "risks": [],
  "confidence": "medium",
  "notes_append_candidate": "",
  "limitations": []
}
```

Facts and hypotheses must stay separate. A consolidator may only treat a fact as
canonical when it is supported by repo evidence, command output, runtime state,
or an accepted proof artifact.

## Consolidation Receipt

The consolidator writes one `ConsolidationReceipt` per stage:

```json
{
  "receipt_id": "task-id:analysis:consolidation:001",
  "task_id": "task-id",
  "stage_id": "analysis",
  "attempt_ids": [],
  "accepted_attempt_ids": [],
  "rejected_attempt_ids": [],
  "conflicts": [],
  "canonical_findings": [],
  "canonical_decision": "",
  "required_next_stage": "design",
  "notes_append": "",
  "proof": []
}
```

The receipt, not the individual attempt, is the only artifact allowed to update
canonical task notes or stage state.

## Command Surface

Minimum runtime commands:

```powershell
vida task attempt dispatch <task-id> --stage analysis --policy configured --json
vida task attempt dispatch <task-id> --stage implementation --backend internal_codex --model-profile architect --json
vida task attempt status <task-id> --stage analysis --json
vida task attempt collect <task-id> --stage analysis --json
vida task attempt consolidate <task-id> --stage analysis --json
vida task stage status <task-id> --json
vida task note append <task-id> --from-consolidation <receipt-id> --json
```

The existing `task update --notes` surface must not be used for attempt results.
Attempt and consolidation updates require append-only note support.

## Isolation Rules

1. Analysis, design, review, and proof attempts default to `read_only`.
2. Implementation attempts must use `patch_artifact` or `isolated_worktree`.
3. External CLI carriers must not write to the canonical worktree unless the
   runtime has granted an explicit bounded write scope for that attempt.
4. TaskFlow mutations are reserved for the consolidator or root orchestrator.
5. A stale attempt cannot be consumed automatically; it must be rebased or
   superseded.

## Operator Summary

Task progress and graph summary surfaces should expose:

1. active stage,
2. configured attempt count,
3. running attempts,
4. produced attempts,
5. accepted/rejected/stale attempt counts,
6. latest consolidation receipt,
7. next command.

Example:

```text
stage=analysis attempts=2/2 produced=2 accepted=1 rejected=1 consolidation=accepted next=vida task stage advance <task-id>
```

## Follow-Up Epic Boundary

This protocol should become a new follow-up epic after the active architecture
refactor quality epic. It should not block the current epic unless the current
runtime cannot complete its required orchestration tasks without the attempt
ledger.

The first implementation slice should be append-only notes because every later
attempt and consolidation workflow depends on safe task-note mutation.

## Proof Expectations

1. Unit tests for attempt ledger state transitions.
2. CLI smoke tests for dispatch, status, collect, and consolidate.
3. Tests proving external attempts cannot mutate canonical task state directly.
4. Tests proving stale attempts are rejected before note append.
5. Tests proving implementation attempts can be represented as patch artifacts
   without touching the canonical worktree.

-----
artifact_path: process/multi-agent-stage-ensemble-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: 2026-06-03
schema_version: '1'
status: proposed
source_path: docs/process/multi-agent-stage-ensemble-protocol.md
created_at: 2026-06-03T14:40:00+03:00
updated_at: 2026-06-03T18:30:00+03:00
changelog_ref: multi-agent-stage-ensemble-protocol.changelog.jsonl
