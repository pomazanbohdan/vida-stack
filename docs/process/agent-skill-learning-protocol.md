# Agent Skill Learning Protocol

Status: active runtime protocol

Protocol id: `agent.skill-learning`

Canonical surface: `vida skill learn`

Implementation status: protocol-only until the runtime command family is implemented.

Purpose: define the neutral VIDA runtime protocol for turning agent execution evidence into validated project skills without training model weights or accepting unvalidated prompt drift.

## Scope

This protocol defines how VIDA projects:

1. capture skill-learning evidence from coding sessions,
2. distinguish useful patterns from accidental local fixes,
3. stage bounded skill edits,
4. validate those edits against held-out evidence,
5. reject unsafe or overfit edits,
6. activate accepted skill changes under TaskFlow and DocFlow authority,
7. run a protocol-only subset when a project does not use the VIDA runtime.

This protocol does not define:

1. model fine-tuning,
2. automatic mutation of active `SKILL.md` files,
3. a replacement for TaskFlow, DocFlow, or agent-extension registries,
4. project-specific skill content,
5. provider-specific model prompts,
6. permission to harvest private transcripts without an explicit configured policy.

## External References

This protocol keeps direct source links to the original systems it adapts:

1. SkillOpt project page: https://microsoft.github.io/SkillOpt/
2. SkillOpt repository: https://github.com/microsoft/SkillOpt
3. SkillOpt paper: https://arxiv.org/abs/2605.23904
4. SkillOpt-Sleep preview: https://github.com/microsoft/SkillOpt/blob/main/docs/sleep/README.md
5. SkillLens project page: https://microsoft.github.io/SkillLens/
6. SkillLens repository: https://github.com/microsoft/SkillLens
7. SkillLens paper: https://arxiv.org/abs/2605.23899

## Core Rule

Skill learning is a validation-gated artifact lifecycle.

The lifecycle is:

```text
experience -> event -> example -> patch proposal -> validation run -> decision -> activation
```

An active skill changes only when:

1. the source experience is recorded,
2. the reusable pattern is explicit,
3. the candidate edit is bounded,
4. the candidate is validated on evidence outside the source examples,
5. rejected patterns are recorded,
6. activation is authorized by the current project write rules.

Positive coding experience is not enough by itself. A useful skill must also say when the pattern applies, what failure it prevents, what evidence proves it, and when it must not be used.

## Runtime Authorities

TaskFlow owns:

1. active bounded unit,
2. DB-backed step requirement before active skill mutation,
3. task close evidence,
4. Post-Task Self-Analysis evidence,
5. graph validation after runtime mutations,
6. follow-up tasks for missing validation infrastructure.

DocFlow owns:

1. project-visible documentation checks,
2. documentation closeout evidence,
3. process-map registration,
4. readiness checks when activation wiring changes.

Agent-extension registries own:

1. skill ids,
2. skill compatibility rules,
3. profile and flow bindings,
4. runtime projections for project-local skill activation.

Runtime assignment owns:

1. extractor carrier selection,
2. optimizer carrier selection,
3. target carrier selection,
4. validator carrier selection,
5. cost and reasoning tier policy.

The skill-learning runtime must not fabricate receipts, silently bypass active write guards, or treat generated text as authoritative before validation.

## Manual Bypass Rule

When the current VIDA runtime cannot represent the required multi-task or command-family mutation, an operator may explicitly authorize protocol-only manual implementation.

Under that bypass:

1. do not create fake TaskFlow receipts,
2. do not mutate runtime state by hand,
3. keep work to bounded static docs or code edits,
4. record the missing runtime surface as an implementation follow-up,
5. preserve the distinction between protocol adoption and implemented runtime command support,
6. run file-level and DocFlow checks that do not depend on the missing runtime surface.

Protocol adoption is complete only for the text contract. Runtime support remains incomplete until `vida skill learn` exists and passes public-surface proof.

## SkillLens Layer: Diagnose Utility

The runtime evaluates skill usefulness across three stages.

### Experience Generation

Allowed source experiences include:

1. task close evidence,
2. Post-Task Self-Analysis fields,
3. dynamic criteria,
4. agent return classifications,
5. command timing findings,
6. proof failures,
7. successful proof bundles,
8. GitHub PR or review findings,
9. CI failure clusters,
10. user corrections,
11. repeated runtime blockers,
12. manually supplied positive and negative examples.

Each source experience must record:

1. source ref,
2. task or session ref,
3. observed behavior,
4. expected behavior,
5. proof signal,
6. candidate skill ids,
7. privacy class,
8. replay permission.

### Skill Extraction

A candidate skill pattern must include:

1. problem class,
2. trigger conditions,
3. success mechanism,
4. failure mechanism,
5. required evidence,
6. forbidden shortcuts,
7. verification target,
8. non-applicability cases,
9. source examples,
10. held-out examples or a reason they do not yet exist.

### Skill Consumption

Runtime must evaluate the consumed skill, not just the extracted text.

Validation must detect:

1. positive transfer,
2. neutral transfer,
3. negative transfer,
4. target-carrier mismatch,
5. overfitting to source examples,
6. context-budget harm,
7. unsafe narrowing or broadening of existing instructions.

## Skill Quality Rubric

Every proposal is scored before validation.

Minimum dimensions:

1. `failure_mechanism_encoding`
   - The skill explains why prior failures happened.
2. `actionable_specificity`
   - The skill tells the agent what to do differently in observable steps.
3. `high_risk_action_blacklist`
   - The skill names actions that must not be taken.

Additional dimensions:

1. `trigger_precision`
   - Activation conditions are narrow enough.
2. `proof_alignment`
   - The skill names proof that closes the behavior.
3. `negative_transfer_risk`
   - The skill is unlikely to degrade adjacent tasks.
4. `compression_quality`
   - The skill is compact enough to load without overwhelming task context.
5. `authority_alignment`
   - The skill does not override TaskFlow, DocFlow, runtime, or user authority.

The first three dimensions are mandatory for any proposal that modifies an active skill.

## SkillOpt Layer: Bounded Text Optimization

The runtime treats the skill artifact as trainable external state.

Trainable state can include:

1. `.agents/skills/<skill>/SKILL.md`,
2. `.agents/skills/<skill>/references/**`,
3. `.agents/skills/<skill>/scripts/**`,
4. project skill registry rows,
5. runtime projection metadata.

The model is not trained.

## Patch Contract

Every proposal uses structured operations:

1. `add`,
2. `delete`,
3. `replace`.

Each operation records:

1. target skill id,
2. target path,
3. base hash,
4. affected section,
5. operation kind,
6. before text,
7. after text,
8. source event ids,
9. expected behavior change,
10. rollback note.

The patch proposal must be reproducible from recorded event ids and rejected-buffer context.

## Textual Learning Rate

The runtime enforces an edit budget.

Default budget:

1. at most one new section,
2. at most three bullet-level edits,
3. at most 800 changed characters,
4. no full-file rewrite,
5. no unrelated skill folder edits,
6. no removal of safety, verification, privacy, or authority instructions unless a stronger safety cleanup gate is explicitly in scope.

A larger budget is allowed only for a bounded migration task that names:

1. target skill ids,
2. owned paths,
3. compatibility constraints,
4. validation pool,
5. rollback plan.

## Validation Gate

A candidate becomes active only when validation passes.

Required validation classes:

1. schema validation,
2. skill quality rubric,
3. source-example sanity check,
4. held-out task validation,
5. regression guard,
6. documentation or skill-folder check,
7. TaskFlow graph validation when runtime state mutates,
8. dirty-scope check before commit.

Held-out validation can use:

1. replayable mini tasks,
2. focused tests,
3. static checks,
4. PR review recurrence checks,
5. bug reproduction commands,
6. benchmark fixtures,
7. human-reviewed acceptance when executable proof is impossible.

The held-out gate must not use only the exact experience that generated the patch.

## Rejected Buffer

Rejected proposals are retained as negative feedback.

Each rejected entry records:

1. proposal id,
2. rejected patch summary,
3. source event ids,
4. failed rubric dimensions,
5. failed validation commands,
6. negative-transfer evidence,
7. reviewer or runtime reason,
8. future suppression rule.

Proposal generation must consult relevant rejected entries before producing a new patch.

## Slow Update

A slow update consolidates several accepted small edits into a cleaner skill shape.

It is allowed only when:

1. multiple accepted rules point to one stable invariant,
2. held-out validation remains green,
3. the compacted skill is shorter or clearer,
4. rejected-buffer review shows no repeated harmful direction,
5. process maps and activation registries remain aligned.

Slow update is never a shortcut around validation.

## Runtime Data Model

### `SkillLearningEvent`

Fields:

1. `event_id`,
2. `source_kind`,
3. `source_ref`,
4. `task_id`,
5. `skill_ids`,
6. `carrier_profile`,
7. `outcome`,
8. `observed_pattern`,
9. `evidence_refs`,
10. `proof_refs`,
11. `risk_tags`,
12. `privacy_class`,
13. `created_at`.

Allowed `source_kind` values:

1. `task_close`,
2. `self_analysis`,
3. `dynamic_criterion`,
4. `agent_return`,
5. `user_correction`,
6. `github_review`,
7. `ci_failure`,
8. `command_timing`,
9. `runtime_diagnostic`,
10. `manual_seed`.

### `SkillLearningExample`

Fields:

1. `example_id`,
2. `event_id`,
3. `example_kind`,
4. `task_prompt_or_summary`,
5. `expected_behavior`,
6. `proof_command`,
7. `scoring_method`,
8. `split`,
9. `privacy_class`,
10. `replay_allowed`.

Allowed `split` values:

1. `train`,
2. `selection`,
3. `held_out`,
4. `rejected_reference`.

### `SkillPatchProposal`

Fields:

1. `proposal_id`,
2. `target_skill_id`,
3. `target_path`,
4. `base_hash`,
5. `operations`,
6. `edit_budget`,
7. `source_event_ids`,
8. `rubric_scores`,
9. `expected_delta`,
10. `created_by`,
11. `status`.

Allowed `status` values:

1. `draft`,
2. `ready_for_validation`,
3. `validation_running`,
4. `accepted`,
5. `rejected`,
6. `deferred`,
7. `superseded`.

### `SkillValidationRun`

Fields:

1. `validation_id`,
2. `proposal_id`,
3. `selection_examples`,
4. `held_out_examples`,
5. `commands`,
6. `results`,
7. `delta_summary`,
8. `regressions`,
9. `verdict`,
10. `validated_at`.

Allowed `verdict` values:

1. `pass`,
2. `fail`,
3. `inconclusive`,
4. `blocked`.

### `SkillLearningDecision`

Fields:

1. `decision_id`,
2. `proposal_id`,
3. `decision`,
4. `reason`,
5. `accepted_path`,
6. `rejected_buffer_ref`,
7. `taskflow_ref`,
8. `docflow_ref`,
9. `commit_ref`,
10. `decided_at`.

Allowed `decision` values:

1. `accept`,
2. `reject`,
3. `defer`,
4. `request_rework`.

## Storage Layout

Runtime-owned state should live under the authoritative VIDA state store.

Suggested logical store:

```text
.vida/data/state/skill-learning/
  events.jsonl
  examples.jsonl
  proposals.jsonl
  validations.jsonl
  decisions.jsonl
  rejected-buffer.jsonl
```

Human-readable staged artifacts can live under:

```text
.vida/artifacts/skill-learning/
  proposals/<proposal-id>.patch.md
  validations/<validation-id>.md
  decisions/<decision-id>.md
```

Project-local active skills remain:

```text
.agents/skills/<skill>/SKILL.md
.agents/skills/<skill>/references/
.agents/skills/<skill>/scripts/
```

Protocol-only projects may use:

```text
.agents/skills/<skill>/learning/events.md
.agents/skills/<skill>/learning/proposals/
.agents/skills/<skill>/learning/rejected.md
.agents/skills/<skill>/learning/validations/
.agents/skills/<skill>/CHANGELOG.md
```

## Command Surface

The canonical command family is:

```text
vida skill learn
```

Until implemented, this command family is a protocol target, not a working runtime surface.

### `vida skill learn collect`

Purpose: collect skill-learning events from existing runtime evidence.

Examples:

```powershell
vida skill learn collect --task <task-id>
vida skill learn collect --since 2026-07-01
vida skill learn collect --source self-analysis --json
```

No skill files are edited.

### `vida skill learn examples`

Purpose: create or inspect replayable examples from collected events.

Examples:

```powershell
vida skill learn examples --skill <skill-id>
vida skill learn examples --proposal <proposal-id> --split held_out
```

### `vida skill learn propose`

Purpose: generate a bounded patch proposal.

Examples:

```powershell
vida skill learn propose --skill <skill-id> --from-events <event-id>
vida skill learn propose --skill <skill-id> --since 2026-07-01 --budget small
```

Rules:

1. proposal is staged,
2. active `SKILL.md` is not edited,
3. rejected buffer is consulted,
4. output includes operations and validation plan,
5. proposal is reproducible from event ids.

### `vida skill learn validate`

Purpose: run the selection and held-out validation gate.

Examples:

```powershell
vida skill learn validate --proposal <proposal-id>
vida skill learn validate --proposal <proposal-id> --held-out-only
```

Rules:

1. apply patch in an isolated candidate copy,
2. run schema and rubric checks,
3. run declared proof commands,
4. compare candidate against base skill,
5. report regressions explicitly.

### `vida skill learn accept`

Purpose: apply a validated proposal to the active skill.

Examples:

```powershell
vida skill learn accept --proposal <proposal-id>
```

Rules:

1. validation verdict must be `pass`,
2. a DB-backed step must exist before file mutation unless an explicit manual-bypass rule applies,
3. edit must match the validated patch base hash,
4. DocFlow or skill-folder validation must pass after write,
5. TaskFlow graph validation must pass when runtime state mutates,
6. decision record is written.

### `vida skill learn reject`

Purpose: reject a proposal and update the rejected buffer.

Examples:

```powershell
vida skill learn reject --proposal <proposal-id> --reason "negative transfer on held-out runtime tasks"
```

Rules:

1. rejected reason is mandatory,
2. failed validation refs are stored,
3. rejected buffer is updated,
4. future proposal prompts include relevant rejected entries.

### `vida skill learn status`

Purpose: summarize the skill-learning backlog.

Examples:

```powershell
vida skill learn status
vida skill learn status --skill <skill-id>
vida skill learn status --json
```

Default output should be compact TOON/plain.

Required fields:

1. candidate events,
2. open proposals,
3. blocked validations,
4. rejected patterns,
5. accepted last update,
6. next recommended action.

### `vida skill learn sleep`

Purpose: run an offline consolidation cycle inspired by local agent sleep workflows.

Examples:

```powershell
vida skill learn sleep --since 2026-07-01 --stage-only
vida skill learn sleep --skill vida-runtime-development --recall 20 --dream-rollouts 3
```

Rules:

1. default is stage-only,
2. no active skill file changes without explicit accept,
3. privacy policy filters transcript inputs,
4. recall selects similar past examples,
5. synthetic variants are optional and labeled,
6. held-out gate remains mandatory.

## Runtime Flow

### Normal Post-Task Flow

After task close:

1. read TaskFlow close evidence,
2. read Post-Task Self-Analysis,
3. collect events,
4. classify whether events are skill-worthy,
5. record `no_skill_update_reason` when not skill-worthy,
6. create examples when skill-worthy,
7. propose a patch only when enough examples exist,
8. validate the candidate,
9. stage accept or reject decision,
10. show next action to the orchestrator.

### Offline Consolidation Flow

On scheduled consolidation:

1. harvest opted-in session summaries,
2. deduplicate events,
3. cluster recurring tasks or failure modes,
4. retrieve similar historical examples,
5. propose bounded patches,
6. validate against held-out pool,
7. reject harmful patches,
8. stage accepted candidates for operator review,
9. update status output.

### Manual Seed Flow

For a project without enough runtime history:

1. operator writes positive and negative examples,
2. runtime or protocol-only process imports them as manual seed events,
3. proposal generator creates a small skill patch,
4. validation uses project tests or review rubric,
5. operator accepts only after proof.

## Protocol-Only Subset

Projects without VIDA runtime can still use the lifecycle.

Minimum files:

```text
.agents/skills/<skill>/SKILL.md
.agents/skills/<skill>/learning/events.md
.agents/skills/<skill>/learning/proposals/
.agents/skills/<skill>/learning/rejected.md
.agents/skills/<skill>/learning/validations/
.agents/skills/<skill>/CHANGELOG.md
```

Minimum rules:

1. write positive and negative examples,
2. create candidate pattern as a proposal first,
3. apply the skill quality rubric,
4. enforce a small edit budget,
5. validate on tasks other than the source examples,
6. move only validated text into active `SKILL.md`,
7. append rejected proposals to `rejected.md`,
8. keep a changelog entry for accepted changes.

Protocol-only adoption gives process discipline without claiming runtime enforcement.

## Privacy And Redaction

Before storing an event or example, runtime must classify it as:

1. `public`,
2. `project_internal`,
3. `sensitive`,
4. `secret_blocked`.

`secret_blocked` examples cannot be stored.

Sensitive examples may be stored only after redaction and only when replay does not expose credentials, customer data, private keys, tokens, personal contact data, or payment data.

## Scoring

Validation scoring can combine:

1. binary test pass,
2. exact output match,
3. static guard pass,
4. runtime CLI contract pass,
5. judge rubric,
6. human review,
7. negative-transfer check,
8. token or context-size budget.

Default decision rule:

```text
accept if candidate improves or preserves held-out success and reduces a recorded risk without introducing regressions
reject if candidate fails held-out, increases regressions, weakens safety, or exceeds edit budget
defer if evidence is useful but validation signal is insufficient
```

## Initial Implementation Plan

### Phase 1: Read-Only Skill Learning Index

Add:

1. `SkillLearningEvent` contracts,
2. `vida skill learn collect`,
3. `vida skill learn status`,
4. extraction from TaskFlow close, self-analysis, and user-correction evidence,
5. compact TOON and JSON output.

Proof:

1. event classification unit tests,
2. CLI integration for `status`,
3. no file mutation.

### Phase 2: Proposal Staging

Add:

1. proposal contract,
2. rejected buffer contract,
3. `vida skill learn propose`,
4. staged patch artifact output,
5. edit-budget enforcement.

Proof:

1. proposal schema tests,
2. budget rejection tests,
3. rejected-buffer suppression tests.

### Phase 3: Validation Gate

Add:

1. candidate skill sandbox,
2. validation runner,
3. held-out example registry,
4. `vida skill learn validate`,
5. regression report.

Proof:

1. pass and fail validation fixtures,
2. negative-transfer fixture,
3. source-only validation rejection.

### Phase 4: Controlled Activation

Add:

1. `vida skill learn accept`,
2. `vida skill learn reject`,
3. active file mutation behind DB-backed step,
4. DocFlow or skill-folder check,
5. TaskFlow graph proof.

Proof:

1. accepted patch applies only when base hash matches,
2. rejected proposal updates buffer,
3. active skill is unchanged on validation failure.

### Phase 5: Sleep Mode

Add:

1. configured transcript harvesting,
2. recall over past examples,
3. optional dream rollouts,
4. scheduled stage-only run,
5. operator review summary.

Proof:

1. opt-in policy enforcement,
2. redaction tests,
3. stage-only default behavior,
4. no active skill mutation during sleep.

## Recommended First Runtime Task

The first implementation task should be read-only:

```text
task_id: runtime-skill-learning-readonly-index
title: Add read-only skill learning event index and status surface
owned_paths:
  - crates/vida/src/skill_learning_surface.rs
  - crates/vida/src/cli.rs
  - crates/vida/src/root_command_router.rs
  - crates/vida/src/state_store_skill_learning.rs
  - crates/vida/tests/*
  - docs/process/agent-skill-learning-protocol.md
definition_of_done:
  - vida skill learn status shows compact TOON output
  - vida skill learn status --json shows machine-readable event/proposal counts
  - vida skill learn collect --task <task-id> stores read-only events
  - active SKILL.md files are never modified in phase 1
  - validation tests cover event classification and no-mutation guarantees
```

## Completion Criteria

This protocol is text-adopted when:

1. the canonical file exists,
2. project routing maps point to it,
3. original source links remain in the file,
4. DocFlow/file checks pass or documented manual-bypass proof exists.

The runtime feature is implemented only when:

1. `vida skill learn status` exists,
2. `vida skill learn collect` exists,
3. read-only storage is proven,
4. validation-gated proposal workflow is implemented,
5. accept/reject behavior is tested through public CLI surfaces.

-----
artifact_path: process/agent-skill-learning-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-07-01'
schema_version: '1'
status: canonical
source_path: docs/process/agent-skill-learning-protocol.md
created_at: 2026-07-01T00:00:00+03:00
updated_at: 2026-07-01T00:00:00+03:00
changelog_ref: agent-skill-learning-protocol.changelog.jsonl
