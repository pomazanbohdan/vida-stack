# Execution Approval And Interrupt Resume Survey

Purpose: summarize what OpenAI, Anthropic, Microsoft, and LangGraph say about execution pauses, approval-gated interruption, resumable workflow state, and replay-safe continuation, then map those findings to the Stage-4 VIDA runtime contract.

## 1. Research Question

What concrete runtime semantics should VIDA adopt for `execution / approval / interrupt-resume` so pause-and-continue behavior is durable, inspectable, and lawful rather than a chat-level convention?

## 2. Research Scope

This survey uses only official or primary product/runtime documentation from:

1. OpenAI
2. Anthropic
3. Microsoft
4. LangGraph

Rule:

1. these sources are runtime-pattern evidence only,
2. they do not replace VIDA core law,
3. they do help define the right Stage-4 operator/runtime contract.

## 3. Cross-Source Convergence

Across the four source families, several runtime patterns converge strongly.

### 3.1 Pause Is Explicit Runtime State

The shared direction is:

1. execution does not merely "wait in the chat",
2. a pause is represented as explicit waiting state,
3. the system must record why execution stopped and what it is waiting for.

VIDA implication:

1. `approval`, `manual_intervention`, and adjacent waits must remain first-class runtime state,
2. a paused task cannot be represented only by narrative text,
3. Stage 4 must keep waiting state queryable and receipt-backed.

### 3.2 Resume Is Continuation, Not Restart By Guesswork

The shared direction is:

1. continuation happens from previously saved state,
2. resume should target a bounded continuation point,
3. the system must not rely on broad transcript scanning to infer where to continue.

VIDA implication:

1. resume needs explicit handles, ids, or equivalent bounded targeting,
2. broad-scan resume is not lawful target behavior,
3. a resume path must remain deterministic enough for operator inspection.

### 3.3 Approval Is A Blocking Gate, Not A Chat Habit

The shared direction is:

1. approval for consequential actions is a structured gate,
2. approval pauses execution until a lawful signal arrives,
3. approval and execution should share one stateful lifecycle rather than living as disconnected UI events.

VIDA implication:

1. approval must stay route-bound and fail-closed,
2. approval receipts alone are not enough unless they are tied to paused execution lineage,
3. Stage 4 should continue to separate approval from verification while keeping both in one inspectable runtime flow.

### 3.4 Waiting And Resume Must Be Durable

The shared direction is:

1. pause/resume state must survive process/session boundaries,
2. durable state must outlive a single in-memory turn,
3. operator-visible continuation requires durable execution lineage.

VIDA implication:

1. project-local DB truth must remain the authority for waiting and resumption state,
2. waits and resumes cannot depend on volatile chat memory,
3. checkpoint lineage and waiting lineage should stay aligned.

### 3.5 Replay-Safe Side Effects Matter

The shared direction is:

1. resume and replay must not blindly repeat dangerous external effects,
2. non-repeatable actions need explicit treatment,
3. durable execution implies side-effect discipline.

VIDA implication:

1. replay-safe and non-replay-safe actions must be distinguished explicitly,
2. Stage 4 cannot treat every resumed step as harmless recomputation,
3. execution-boundary policy must eventually enforce replay-safe behavior.

## 4. Source Notes

### 4.1 OpenAI

OpenAI agent guidance reinforces:

1. human approval can interrupt a run,
2. the run surfaces interruption state explicitly,
3. continuation resumes from stored run state rather than from a fresh guess,
4. approval handling must work even when nested tool or handoff behavior is involved.

VIDA takeaway:

1. approval interruption should be modeled as runtime state,
2. waits should be inspectable and resumable through a bounded continuation path,
3. approval belongs in execution lifecycle, not outside it.

### 4.2 Anthropic

Anthropic guidance is less explicit about one unified pause/resume object, but still reinforces:

1. permissioned execution boundaries,
2. sandboxed and bounded tool execution,
3. sensitive actions requiring explicit approval or permission posture,
4. long-running execution needing safe operational control.

VIDA takeaway:

1. approval boundaries should remain permission-bearing, not purely conversational,
2. interrupt/resume design must stay aligned with execution-boundary and sandbox policy,
3. pause/resume should not weaken least-privilege execution law.

### 4.3 Microsoft

Microsoft guidance reinforces:

1. approval as a pause-for-input workflow state,
2. continuation through the same workflow/session context,
3. explicit operator decision handling rather than silent continuation,
4. human-in-the-loop as a structured product surface.

VIDA takeaway:

1. operator answers should be treated as durable runtime events,
2. resume should bind to prior workflow state,
3. approval waits should be visible and queryable as product state.

### 4.4 LangGraph

LangGraph most clearly reinforces:

1. explicit `interrupt` semantics,
2. durable continuation through saved state and execution identity,
3. resume with a bounded `resume` command instead of broad reinterpretation,
4. node restart on resume requiring idempotent or replay-safe side-effect handling.

VIDA takeaway:

1. interrupt/resume should be treated as a first-class runtime primitive,
2. checkpoint and resume identity must remain explicit,
3. replay-safe side-effect discipline must be part of the future runtime contract.

## 5. Recommended VIDA Borrow Decisions

### 5.1 Adopt Now

These should be treated as active Stage-4 direction now:

1. explicit waiting state for approval/manual intervention/external waits,
2. durable pause state in project-local DB truth,
3. deterministic resume targeting rather than broad chat scanning,
4. approval as a blocking execution gate,
5. operator-query visibility into what execution is waiting on,
6. alignment between waiting state and checkpoint lineage.

### 5.2 Adopt As Forward Direction

These should be accepted as next-step direction but not overstated as already closed:

1. one-time resume-handle consumption and duplicate-resume protection,
2. richer resume signal classes such as `approve`, `reject`, `edit`, `rework`, `external_signal`,
3. stronger split between replay-safe and non-replay-safe actions,
4. resume correlation targeting and trigger indexing,
5. fuller fork/replay debugging over paused execution history.

### 5.3 Reject As-Is

These should not become root VIDA law:

1. provider-owned run or thread identity as the canonical runtime identity,
2. framework-specific interrupt APIs as product-law contracts,
3. UI-native approval widgets as the authority for approval state,
4. transcript-only continuation as the pause/resume mechanism.

## 6. Immediate Consequences For Stage 4

This survey narrows the remaining Stage-4 contract questions to:

1. what the authoritative pause object is,
2. what the exact resume-handle lifecycle is,
3. how approval, verification, coach, and manual intervention waiting states differ operationally,
4. which bounded query surfaces expose waiting execution state,
5. how checkpoint lineage and replay-safe side effects interact with continuation.

## 7. Sources

Primary references used for this survey:

1. OpenAI Agents Python: Human In The Loop
   - https://openai.github.io/openai-agents-python/human_in_the_loop/
2. OpenAI Agents JS: Human In The Loop
   - https://openai.github.io/openai-agents-js/guides/human-in-the-loop/
3. Anthropic Tool Use Implementation
   - https://docs.anthropic.com/en/docs/agents-and-tools/tool-use/implement-tool-use
4. Anthropic Claude Code Security
   - https://docs.claude.com/en/docs/claude-code/security
5. Anthropic Claude Code Sandboxing
   - https://docs.claude.com/en/docs/claude-code/sandboxing
6. Microsoft Agent Framework Tool Approval
   - https://learn.microsoft.com/en-us/agent-framework/agents/tools/tool-approval
7. Microsoft Agent Framework Function Tools Approvals Tutorial
   - https://learn.microsoft.com/en-us/agent-framework/tutorials/agents/function-tools-approvals
8. Microsoft Agent Framework Human In The Loop Workflows
   - https://learn.microsoft.com/en-us/agent-framework/workflows/human-in-the-loop
9. LangGraph Human In The Loop
   - https://docs.langchain.com/oss/python/langgraph/human-in-the-loop
10. LangGraph Durable Execution
   - https://docs.langchain.com/oss/python/langgraph/durable-execution

-----
artifact_path: product/research/execution-approval-and-interrupt-resume-survey
artifact_type: product_research_doc
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/research/execution-approval-and-interrupt-resume-survey.md
created_at: '2026-03-12T23:20:00+02:00'
updated_at: '2026-03-12T23:20:00+02:00'
changelog_ref: execution-approval-and-interrupt-resume-survey.changelog.jsonl
