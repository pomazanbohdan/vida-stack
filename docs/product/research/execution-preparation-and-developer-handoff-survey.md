# Execution Preparation And Developer Handoff Survey

Purpose: ground the VIDA v1 `execution_preparation` stage in external agent-runtime patterns so the `solution_architect` lane and its handoff artifacts are based on proven orchestration practice rather than project-local invention.

## 1. Research Question

What external runtime patterns support a dedicated pre-execution architecture-preparation stage between planning and implementation, and what should VIDA adopt for `execution_preparation` and the developer handoff contract?

## 2. Core Result

The strongest current conclusion is:

1. multi-agent runtimes increasingly separate planning from execution,
2. specialized read-mostly subagents are used to gather context, constraints, and architecture direction before a mutation lane starts,
3. structured handoffs are treated as first-class runtime objects rather than as implicit chat recap,
4. execution-preparation should stay distinct from both product planning and direct implementation,
5. VIDA v1 should keep `solution_architect` as the default owner for that preparation stage.

## 3. External Research Signals

### 3.1 OpenAI

OpenAI's Agents SDK treats multi-agent decomposition and handoffs as first-class runtime behavior.

Implication for VIDA:

1. a manager or orchestrator can route work to a specialized pre-execution lane,
2. the handoff itself should be explicit and structured,
3. pre-execution specialization should be able to narrow what the next lane sees rather than forwarding raw chat context blindly.

Primary sources:

1. `OpenAI Agents SDK Multi-Agent`
   - https://openai.github.io/openai-agents-python/multi_agent/
2. `OpenAI Agents SDK Handoffs`
   - https://openai.github.io/openai-agents-python/handoffs/
3. `OpenAI Agents SDK Agents`
   - https://openai.github.io/openai-agents-python/agents/

### 3.2 Anthropic

Anthropic's Claude Code subagents model uses specialized subagents with separate context windows and bounded tool access.

Implication for VIDA:

1. execution preparation should be a separate bounded lane, not just "developer thinks a bit more first",
2. the preparation lane should be able to operate with a narrower, mostly read-only tool posture,
3. separate context for preparation reduces contamination of the implementation lane with irrelevant repo history.

Primary source:

1. `Claude Code Subagents`
   - https://docs.anthropic.com/en/docs/claude-code/sub-agents

### 3.3 Microsoft

Microsoft's orchestration guidance emphasizes sequential orchestration where distinct agents perform distinct stages such as analysis, writing, and review.

Implication for VIDA:

1. a dedicated preparation stage is a legitimate orchestration stage, not just an informal role preference,
2. stage outputs should feed the next stage intentionally,
3. pre-execution architecture analysis belongs in an explicit sequential flow before mutation.

Primary sources:

1. `Semantic Kernel Agent Orchestration`
   - https://learn.microsoft.com/en-us/semantic-kernel/frameworks/agent/agent-orchestration/
2. `Semantic Kernel Sequential Orchestration`
   - https://learn.microsoft.com/en-us/semantic-kernel/frameworks/agent/agent-orchestration/sequential
3. `Semantic Kernel Planning`
   - https://learn.microsoft.com/en-us/semantic-kernel/concepts/planning

### 3.4 LangGraph

LangGraph's plan-and-execute guidance treats planning as a separate runtime step and allows re-planning after execution discoveries.

Implication for VIDA:

1. planning and execution should remain distinct,
2. there is room for one additional bounded stage between them when execution needs architecture preparation,
3. re-preparation should be allowed when codebase drift or dependency findings supersede the earlier handoff.

Primary source:

1. `LangGraph Plan-And-Execute`
   - https://langchain-ai.github.io/langgraph/tutorials/plan-and-execute/plan-and-execute/

## 4. What VIDA Should Adopt

VIDA should adopt the following patterns now:

1. an explicit `execution_preparation` stage between planning and implementation,
2. a specialized `solution_architect` lane as the default owner,
3. a structured handoff packet rather than raw planning output,
4. bounded preparation context and bounded preparation tools,
5. re-preparation when material drift invalidates the previous handoff.

## 5. What VIDA Should Not Copy Directly

VIDA should not copy external frameworks as root law.

Specifically:

1. do not adopt provider-owned agent APIs as canonical VIDA contract,
2. do not treat vendor runtime objects as one-to-one VIDA state families,
3. do not collapse planning, preparation, and implementation into a generic "multi-agent" abstraction,
4. do not make implementation read arbitrary broad context just because a preparation lane exists.

## 6. Proposed VIDA-Specific Execution Preparation Shape

The strongest current VIDA-specific shape is:

1. `planning` produces bounded scope/spec/task intent,
2. `execution_preparation` studies specs, code, dependencies, and boundaries,
3. `execution_preparation` emits structured artifacts:
   - `architecture_preparation_report`
   - `developer_handoff_packet`
   - `change_boundary`
   - `dependency_impact_summary`
   - `spec_alignment_summary`
4. `implementation` begins from that handoff rather than from raw planning output,
5. `coach` and `verification` remain downstream quality gates rather than being merged into preparation.

## 7. Runtime Behavior Recommendations

The preparation stage should likely have the following runtime characteristics:

1. bounded read-mostly repository inspection,
2. explicit spec/protocol reads,
3. explicit dependency-surface discovery,
4. explicit handoff artifact materialization,
5. fail-closed execution when required preparation artifacts are absent or stale.

Inference:

1. this suggests the preparation lane should usually run with less mutation authority than the implementation lane,
2. its primary output is architecture alignment, not code changes.

## 8. Open Questions

The following still need closure inside VIDA product law:

1. the exact machine-readable schema for `architecture_preparation_report`,
2. the exact machine-readable schema for `developer_handoff_packet`,
3. the fast-path policy for skipping preparation on low-risk tasks,
4. the exact runtime query/status surfaces for preparation readiness,
5. whether the preparation lane is strictly read-only or can perform bounded preparatory mutations,
6. the exact stale-handoff invalidation triggers after scope or codebase drift.

## 9. Result

This research is strong enough to fix one architectural decision now:

1. `execution_preparation` is a valid first-class runtime stage for VIDA v1, not just a local workflow preference.

It is not yet strong enough to close:

1. the final artifact schema,
2. the exact runtime commands,
3. the exact bypass policy,
4. the exact mutation posture of the preparation lane.

-----
artifact_path: product/research/execution-preparation-and-developer-handoff-survey
artifact_type: product_research_doc
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/research/execution-preparation-and-developer-handoff-survey.md
created_at: '2026-03-13T00:10:00+02:00'
updated_at: '2026-03-13T00:10:00+02:00'
changelog_ref: execution-preparation-and-developer-handoff-survey.changelog.jsonl
