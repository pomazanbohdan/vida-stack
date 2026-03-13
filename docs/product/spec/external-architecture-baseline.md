# External Architecture Baseline

Status: active product reference law

Purpose: define the canonical external vendor architecture baseline that informs VIDA orchestration, guardrails, runtime state, and subagent coordination decisions.

## 1. Why This Baseline Exists

VIDA should keep explicit alignment with current external architecture baselines where those baselines strengthen:

1. orchestration boundaries,
2. guardrail placement,
3. runtime-state ownership,
4. handoff and subagent composition.

This document preserves those alignments separately from the top-level architecture anchor so the anchor can stay focused on VIDA-owned structure.

## 2. OpenAI

Alignment:

1. behavior, tools, and guardrails sit at agent/runtime boundaries,
2. orchestration owns routing and handoffs,
3. tracing and execution state belong in runtime surfaces.

Official references:

1. `https://openai.github.io/openai-agents-python/handoffs/`
2. `https://openai.github.io/openai-agents-js/guides/guardrails/`
3. `https://openai.github.io/openai-agents-js/guides/running-agents`

## 3. Anthropic

Alignment:

1. subagents own scoped prompts and bounded expertise,
2. role wording tends toward behavior contract,
3. upper-layer orchestration should therefore prefer lane/coordination semantics over role-behavior ownership.

Official references:

1. `https://docs.anthropic.com/en/docs/claude-code/sub-agents`
2. `https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices#give-claude-a-role`

## 4. Microsoft

Alignment:

1. orchestration is a coordination-pattern layer,
2. execution/runtime machinery stays below orchestration,
3. agent specialization and plugin/tool execution are explicit rather than implicit.

Official references:

1. `https://learn.microsoft.com/en-us/semantic-kernel/frameworks/agent/agent-architecture`
2. `https://learn.microsoft.com/en-us/semantic-kernel/frameworks/agent/agent-orchestration/`

## 5. Current Rule

1. external baselines may inform VIDA architecture,
2. they do not replace VIDA-owned product law,
3. alignment remains explicit and inspectable rather than implicit in discussion history.

-----
artifact_path: product/spec/external-architecture-baseline
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/product/spec/external-architecture-baseline.md
created_at: '2026-03-13T08:39:49+02:00'
updated_at: '2026-03-13T08:47:25+02:00'
changelog_ref: external-architecture-baseline.changelog.jsonl
