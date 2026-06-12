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
the mapped generic runtime owner protocols.

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
   performs only publication authorized by a current explicit operator
   instruction, and closes TaskFlow.

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
4. `verification`,
5. `changed_files` or reviewed scope,
6. `gaps`,
7. residual risks,
8. `tokens_used`,
9. `steps_taken`,
10. `tool_calls_used`.

If exact token usage is not exposed by the host runtime, the attempt must write
`tokens_used: not_exposed_by_host`. It must not estimate tokens unless the
runtime provides an explicit estimate field. Step and tool-call counts must be
reported from the attempt's own action log.

For cheap executor lanes selected as `gpt-5.4-mini`, use the highest available
reasoning effort by default. The consolidator still treats the attempt as
untrusted until a stronger validator or root orchestrator verifies source
fidelity, public-surface proof, and false-green risk.

Use the cheapest model that is reliable for the packet class, not the cheapest
model unconditionally:

1. `gpt-5.4-mini` is the default for narrow read-only decomposition and exact
   test-only edits with explicit proof commands.
2. `gpt-5.5-low` is the default rework executor after a mini false-green,
   timeout, shutdown, or validator rejection, and for bounded source/runtime
   implementation.
3. `gpt-5.5-medium` is the default validator for authority-sensitive runtime,
   TaskFlow, DocFlow, host-bridge, path-policy, receipt, release, and public
   operator-surface changes.
4. A mini result can be accepted as partial evidence, but cannot close an
   authority-sensitive task without stronger validation and root proof.
5. Repeated timeout or shutdown from the same carrier class is a process
   failure signal; the next attempt must either narrow the packet or escalate
   the executor model.

Runtime-authority task defaults from the 2026-06-11 wave-0 evaluation:

1. Start with `gpt-5.4-mini` at the highest available reasoning effort for
   narrow decomposition, source-sync documentation, or one exact regression-test
   packet.
2. Use `gpt-5.5-low` as the first rework executor when mini output is partial,
   times out, or misses acceptance but the scope remains bounded.
3. Use `gpt-5.5-medium` as the default validator for authority predicates,
   receipt semantics, TaskFlow/DocFlow state, host bridge, projection cache,
   release closure, and public operator JSON.
4. Add a second cheap mini validator only for one named risk. Add triple
   validation only for shared authority predicates, validator disagreement, or
   wave/epic/release closure.
5. Split broad validation prompts by concrete risk owner instead of asking one
   validator to review the entire repository or epic.
6. The orchestrator assigns an `agent_score_10` after synthesis and records it
   in the model evaluation log or task note before the next agent selection.

Three-step stage loop:

1. Executor attempt:
   - one bounded cheap executor owns one write scope,
   - the attempt returns a patch plus its focused proof bundle,
   - the orchestrator waits long enough for the cheap lane before classifying
     timeout.
2. Bundled validation:
   - the orchestrator runs one compact proof bundle,
   - one stronger validator checks the diff, proof, false-green risk, and closure
     readiness,
   - a rejection becomes one exact rework packet with the validator's blocking
     finding, not a new broad discovery cycle.
3. Consolidation and publication:
   - accepted work is recorded in TaskFlow,
   - scoped files are committed and pushed,
   - PR state and the agent model evaluation log are updated before the next
     implementation task starts.

Use broader ensembles only when the three-step loop produces conflicting
evidence, repeated validator rejection, dirty-file overlap, missing public proof,
or an architectural ownership question.

Operational launch playbook:

1. Mini first pass:
   - use `gpt-5.4-mini` with highest available reasoning only when the packet
     has one bounded task id, explicit scope, one expected artifact, one proof
     target, and a timeout.
2. Rework executor:
   - use `gpt-5.5-low` when mini returns partial work, no artifact, no
     telemetry, timeout/shutdown, or validator-rejected output, while the scope
     is still bounded enough for one patch.
3. Closure validator:
   - use `gpt-5.5-medium` for authority-sensitive runtime, TaskFlow, DocFlow,
     host-bridge, path-policy, receipt, release, PR integration, and public
     operator-surface decisions.
4. Triple validation:
   - use it only for shared authority predicates, validator disagreement,
     medium-high residual risk, or wave/epic/release closure.
5. Cleanup:
   - after each completed or failed attempt, close/delete the host handle before
     launching a replacement attempt for the same stage.
   - do not close an active mini attempt after only a short wait timeout when it
     can still be resumed or asked for a compact partial report; first wait one
     longer interval or send a final-report request.
6. Publication:
   - after accepted implementation evidence, TaskFlow update, debug build,
     commit, and authorized push remain orchestrator duties, not attempt duties.
   - after a wave closes, the system `vida` binary must be release-installed and
     PATH-smoked before the wave is operationally closed.

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
artifact_revision: 2026-06-12
schema_version: '1'
status: proposed
source_path: docs/process/multi-agent-stage-ensemble-protocol.md
created_at: 2026-06-03T14:40:00+03:00
updated_at: 2026-06-12T00:00:00+03:00
changelog_ref: multi-agent-stage-ensemble-protocol.changelog.jsonl
