# Agent Governance And Policy Hardening Survey

Purpose: summarize current official guidance from OpenAI, Anthropic, and Microsoft on human-in-the-loop, approval gates, tool/runtime boundaries, verification, and operational governance for agent systems, and map those findings to the next hardening pass for VIDA policy surfaces.

## 1. Research Question

What policy patterns appear across major official agent-platform guidance, and which of those patterns should shape the next hardening pass for VIDA approval, verification, closure, and execution-boundary policy?

## 2. Research Scope

This survey focuses on official sources from:

1. OpenAI
2. Anthropic
3. Microsoft

It does not treat community blog posts or third-party framework tutorials as primary evidence.

## 3. Converging Findings Across Vendors

Across all three vendors, several patterns converge strongly.

### 3.1 Human Oversight Is Risk-Triggered, Not Constant

The consistent direction is:

1. not every agent step requires manual approval,
2. consequential, high-risk, or external-impact actions do require explicit human oversight,
3. human approval should be a structured operational step, not an improvised chat habit.

Implication for VIDA:

1. `human in the loop` should remain explicit governance state,
2. approval should be policy-triggered by risk/action class,
3. approval receipts should stay first-class and fail closed.

### 3.2 Tool Boundaries Must Be Explicit

The vendors consistently treat tool execution as a bounded control problem rather than a generic model side effect.

Common themes:

1. explicit tool schemas and tool contracts,
2. least-privilege tool access,
3. approval or gating for sensitive tools or sensitive argument classes,
4. clear separation between untrusted model input and executable action.

Implication for VIDA:

1. `approval_policy` alone is not enough,
2. VIDA should introduce a separate execution-boundary policy surface for tool/action classes.

### 3.3 Sandboxing And Runtime Containment Matter

Anthropic and Microsoft are especially explicit here, while OpenAI guidance also points toward controlled execution environments and safe tool orchestration.

Common themes:

1. sandboxed execution,
2. permission-bounded runtime environments,
3. control hooks or middleware points around tool invocation,
4. no silent trust in generated commands or external-action requests.

Implication for VIDA:

1. runtime execution policy must be explicit,
2. dangerous operations should be classed and gated,
3. policy should distinguish read-only, mutating, external, and high-impact actions.

### 3.4 Verification And Evaluation Must Be Operational

The cross-vendor direction is not “judge only the final answer”.

The emphasis is on:

1. workflow-level quality,
2. trace and execution inspection,
3. tool-use correctness,
4. explicit evaluators or graders,
5. observable auditability.

Implication for VIDA:

1. `verification_policy` should become more risk-aware and evidence-aware,
2. trace/eval expectations should be explicit where the work is consequential,
3. closure should depend on evidence class coverage, not only on nominal task status.

### 3.5 Approval, Verification, And Closure Are Distinct

The official guidance does not collapse:

1. authorization,
2. technical validation,
3. final acceptance/closure.

Implication for VIDA:

1. our current split is correct,
2. policies should harden the distinction rather than blur it,
3. closure must continue to depend on both verification and approval where policy requires both.

## 4. Vendor Notes

### 4.1 OpenAI

OpenAI guidance most clearly reinforces:

1. agent safety as a build-time and runtime concern,
2. explicit safeguards around tool/action use,
3. structured safety boundaries rather than ad hoc natural-language conventions,
4. trace grading and evaluation as ongoing control surfaces.

Most relevant VIDA takeaways:

1. keep approval and safety boundaries executable,
2. prefer structured contracts and traces over prose-only safety instructions,
3. use evaluation/trace surfaces to confirm policy behavior rather than trusting prompts alone.

### 4.2 Anthropic

Anthropic guidance most clearly reinforces:

1. permission-based architecture,
2. sandboxing and bounded shells,
3. tool-specific control hooks,
4. explicit caution around command/tool execution and long-lived agent autonomy.

Most relevant VIDA takeaways:

1. split execution-boundary policy from approval policy,
2. classify commands/tools by risk and capability,
3. let runtime policy guard tool execution directly rather than relying only on upstream orchestration judgment.

### 4.3 Microsoft

Microsoft guidance most clearly reinforces:

1. human-in-the-loop as a structured workflow surface,
2. approval as a pause-and-resume lifecycle rather than a chat convention,
3. transparency, auditability, and intelligibility for agent actions,
4. tool approval and HITL workflows as first-class platform concepts.

Most relevant VIDA takeaways:

1. approval should be modeled as resumable workflow state,
2. operator questions and answers should be durable operational events,
3. tool approval and human approval can share lifecycle ideas but should stay semantically distinct when needed.

## 5. Recommended VIDA Policy Stack

The strongest policy stack for VIDA now looks like:

1. `execution_boundary_policy.yaml`
2. `approval_policy.yaml`
3. `verification_policy.yaml`
4. `closure_policy.yaml`
5. later, optional `evaluation_policy.yaml` if trace/eval rules need their own owner surface

### 5.1 Execution Boundary Policy

This missing policy family should define:

1. allowed action classes,
2. denied action classes,
3. approval-required action classes,
4. sandbox and permission posture,
5. external-system boundaries,
6. argument-level escalation triggers for high-impact tools.

### 5.2 Approval Policy Hardening

`approval_policy.yaml` should be extended with:

1. explicit risk bands,
2. approval-required action classes,
3. reapproval triggers on drift,
4. batch versus per-action approval posture,
5. approver classes by risk band,
6. dual-control or escalated approval for critical actions.

### 5.3 Verification Policy Hardening

`verification_policy.yaml` should be extended with:

1. risk-aware verifier requirements,
2. action-class-specific evidence requirements,
3. trace/eval expectations where work is consequential,
4. explicit external-source validation expectations for research-driven work,
5. clearer no-verifier fallback law.

### 5.4 Closure Policy Hardening

`closure_policy.yaml` should remain a pure closure gate and define:

1. closure-ready prerequisites,
2. stale approval blockers,
3. stale import/cache/runtime-state blockers,
4. reopen conditions,
5. required receipt classes and evidence categories.

Rule:

1. closure policy should not carry unrelated bridge overlays longer than necessary,
2. closure policy should consume approval and verification state, not absorb their whole semantics.

## 6. Concrete Gaps In Current VIDA Policy Surfaces

Based on the current repo state, the main gaps are:

1. `approval_policy.yaml` is still too thin and mostly lifecycle-oriented,
2. `verification_policy.yaml` is structurally good but not yet strongly risk-banded,
3. `closure_policy.yaml` still carries bridge-era overlay/config concerns and should be narrowed,
4. there is no dedicated execution-boundary policy surface yet.

## 7. Recommended Next Hardening Pass

The next bounded policy-hardening pass should:

1. create `execution_boundary_policy.yaml`,
2. extend `approval_policy.yaml` with risk bands and reapproval logic,
3. extend `verification_policy.yaml` with risk-aware and evidence-aware gates,
4. narrow `closure_policy.yaml` to closure-only semantics,
5. align all four policy families with the `.vida/**` runtime-placement direction instead of root bridge files.

## 8. Sources

Official references used for this survey:

1. OpenAI Agent Builder Safety
   - https://developers.openai.com/api/docs/guides/agent-builder-safety
2. OpenAI Safety Best Practices
   - https://platform.openai.com/docs/guides/safety-best-practices
3. OpenAI Trace Grading
   - https://platform.openai.com/docs/guides/trace-grading
4. Anthropic Tool Use
   - https://docs.anthropic.com/en/docs/agents-and-tools/tool-use/implement-tool-use
5. Anthropic Bash Tool
   - https://docs.claude.com/en/docs/agents-and-tools/tool-use/bash-tool
6. Anthropic Sandboxing
   - https://docs.claude.com/en/docs/claude-code/sandboxing
7. Anthropic Hooks Guide
   - https://docs.claude.com/en/docs/claude-code/hooks-guide
8. Microsoft Agent Framework Tool Approval
   - https://learn.microsoft.com/en-us/agent-framework/agents/tools/tool-approval
9. Microsoft Agent Framework Human In The Loop
   - https://learn.microsoft.com/en-us/agent-framework/workflows/human-in-the-loop
10. Microsoft Azure AI Agents Transparency Note
   - https://learn.microsoft.com/en-us/azure/foundry/responsible-ai/agents/transparency-note
11. Microsoft Azure OpenAI Transparency Note
   - https://learn.microsoft.com/en-us/azure/foundry/responsible-ai/openai/transparency-note

-----
artifact_path: product/research/agent-governance-and-policy-hardening-survey
artifact_type: product_research_doc
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/research/agent-governance-and-policy-hardening-survey.md
created_at: '2026-03-12T20:30:00+02:00'
updated_at: '2026-03-12T20:30:00+02:00'
changelog_ref: agent-governance-and-policy-hardening-survey.changelog.jsonl
