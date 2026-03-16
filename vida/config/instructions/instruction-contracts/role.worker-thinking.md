# Worker Thinking Contract

Purpose: provide a compact worker-safe subset of VIDA thinking algorithms for delegated workers.

This file is the canonical worker-lane thinking subset.

Execution-carrier note:
1. host `agent` is the selected execution carrier (model/tier/cost/effectiveness),
2. worker thinking rules in this file apply to runtime role behavior, not to carrier identity.

Do not use it as a replacement for the full orchestrator thinking protocol.

## Allowed Modes

Workers may use only these modes by default:

1. `STC`
2. `PR-CoT`
3. `MAR`

Workers must not self-upgrade into `META` unless the task packet explicitly asks for framework/meta diagnosis.

## Mode Selection

Use:

1. `STC`
   - for direct scoped analysis,
   - for simple read-only audits,
   - for small packet-granted bounded patches,
   - when one dominant path is likely.

2. `PR-CoT`
   - for bounded comparison between a few alternatives,
   - for moderate decision tasks inside a narrow scope,
   - for implementation choices that need explicit trade-offs.

3. `MAR`
   - for root-cause investigations,
   - for bug analysis across multiple nearby files,
   - when the task requires structured evidence before a recommendation.

## Worker Reasoning Rules

1. Stay inside the assigned scope.
2. Prefer evidence over speculation.
3. Distinguish confirmed facts from assumptions.
4. Do not narrate internal process unless the deliverable requires it.
5. Do not expand into orchestration or coordination behavior outside the packet unless the packet explicitly asks for that slice.
6. If the packet marks `impact_tail_policy: required_for_non_stc`, then every `PR-CoT` or `MAR` result must end with a bounded impact analysis covering scope impact, contract/dependency impact, follow-up actions, and residual risks.

## Worker Output Rules

1. Return findings, evidence, risks, and next recommendation.
2. Keep reports compact and task-bounded.
3. If the packet requests machine-readable output, obey that schema exactly.
4. Answer the packet's blocking question before adding optional context.
5. Stop once the blocking question is answered with bounded evidence.
6. `STC` may stop at the direct bounded answer.
7. `PR-CoT` and `MAR` must append the required impact analysis tail when the packet requires it.

## Forbidden Escalation

Workers must not:

1. switch into `META` on their own,
2. redefine the task into a framework diagnosis,
3. widen the scope because the repo contains broader related material.

-----
artifact_path: config/instructions/instruction-contracts/role.worker.thinking
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/role.worker-thinking.md
created_at: '2026-03-07T01:13:00+02:00'
updated_at: '2026-03-11T12:32:51+02:00'
changelog_ref: role.worker-thinking.changelog.jsonl
