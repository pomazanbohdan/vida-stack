# Pack Handoff Protocol

Purpose: define the canonical boundary and admissibility rules for handoff between VIDA packs so pack routing does not silently own cross-pack transition law.

## Scope

This protocol applies when work moves between packs in the active routed sequence:

1. `research-pack -> spec-pack`
2. `spec-pack -> work-pool-pack`
3. `work-pool-pack -> dev-pack`
4. `work-pool-pack -> bug-pool-pack`
5. `dev-pack -> reflection-pack`
6. `bug-pool-pack -> reflection-pack`

This protocol does not replace the deeper owners of spec, form-task, implement, bug-fix, or framework self-analysis law.

## Core Contract

Each pack handoff must produce:

1. one explicit current pack owner,
2. one explicit next pack target,
3. a bounded output packet that the next owner may lawfully consume,
4. an admissibility verdict for the handoff,
5. blocker visibility when the next pack may not start yet.
6. stable tracked-pack identity when the same `work-pool-pack` or `dev-pack` task is resumed later in the same tracked flow.

## Handoff Matrix

1. `research-pack -> spec-pack`
   - required outputs:
     - bounded findings
     - explicit scope candidates or constraints
     - external validation evidence when triggered
   - next owner:
     - `runtime-instructions/work.spec-contract-protocol`
2. `spec-pack -> work-pool-pack`
   - required outputs:
     - approved spec scope
     - AC / contract decisions
     - readiness context for task materialization
   - next owner:
     - `command-instructions/planning.form-task-protocol`
3. `work-pool-pack -> dev-pack`
   - required outputs:
     - ready queue in the DB-backed task runtime
     - explicit launch decision
     - dependency state
     - reuse of the already materialized tracked task id when the `work-pool-pack` or downstream `dev-pack` task was created earlier in the same tracked flow
   - next owner:
     - `command-instructions/execution.implement-execution-protocol`
4. `work-pool-pack -> bug-pool-pack`
   - required outputs:
     - narrowed executable bug slice or issue-contract-ready input
     - readiness state for bug execution
   - next owners:
     - `command-instructions/execution.bug-fix-protocol`
     - `runtime-instructions/bridge.issue-contract-protocol` when normalization is still required
5. `dev-pack -> reflection-pack`
   - required outputs:
     - completed implementation evidence
     - verification status
     - drift or synchronization trigger when present
   - next owners:
     - `instruction-contracts/work.documentation-operation-protocol`
     - `diagnostic-instructions/analysis.framework-self-analysis-protocol` only when tracked framework self-analysis/remediation is the actual target rather than ordinary documentation or task-pool reconciliation
6. `bug-pool-pack -> reflection-pack`
   - required outputs:
     - fix evidence or closure verdict
     - regression results
     - any contract/documentation drift requiring reconciliation
   - next owners:
     - `instruction-contracts/work.documentation-operation-protocol`
     - `diagnostic-instructions/analysis.framework-self-analysis-protocol` only when the routed outcome is tracked framework self-analysis/remediation instead of ordinary reflection drift handling

## Admissibility Rule

A next pack may start only when all are true:

1. current pack produced its required outputs,
2. no blocker in the current owner forbids the transition,
3. the next pack's mandatory input contract is satisfied,
4. user approval exists when the next pack requires explicit launch or approval,
5. required web / live-validation evidence exists when external-fact triggers were active.
6. if the tracked task id already exists, the handoff uses reuse/ensure semantics instead of retrying duplicate raw creation for the same id.

If any item fails, the handoff must stop with an explicit blocker or waiting verdict rather than silently continuing.

## Change-Impact Boundary

If handoff is invalidated by scope, AC, decision, or dependency drift:

1. do not continue into the next pack by inertia,
2. route reconciliation through the current canonical owners:
   - `instruction-contracts/core.orchestration-protocol`
   - `command-instructions/planning.form-task-protocol`
   - `command-instructions/execution.implement-execution-protocol`
3. resume pack progression only after the affected owner restores an admissible handoff state.

## Output Contract

Each handoff should be expressible as:

1. `from_pack`
2. `to_pack`
3. `required_outputs_present`
4. `blocking_codes`
5. `approval_state`
6. `admissibility_verdict`
7. `next_owner`

## Fail-Closed Rule

1. Do not treat pack order alone as proof of lawful handoff.
2. Do not treat wrapper completion as canonical handoff evidence.
3. Do not start the next pack when required outputs or approvals are still missing.
4. Do not rerun raw `vida task create <same-task-id> ...` for an already materialized tracked `work-pool-pack` or `dev-pack`; reuse the existing tracked task and continue shaping from that id.

-----
artifact_path: config/runtime-instructions/pack-handoff.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.pack-handoff-protocol.md
created_at: '2026-03-11T00:00:00+02:00'
updated_at: '2026-03-11T13:03:32+02:00'
changelog_ref: work.pack-handoff-protocol.changelog.jsonl
