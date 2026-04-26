# Execution Preparation And Developer Handoff Model

Status: active product law

Purpose: define the canonical `Execution Preparation Layer` in VIDA v1 so code-shaped and architecture-sensitive work passes through one explicit preparation stage before developer execution, reducing architectural drift across different implementation tasks and agents.

## 1. Problem

Planning and execution are already distinguished, but that is not yet enough to keep the codebase coherent across many tasks.

Without one explicit preparation stage:

1. different developer lanes may read the same task differently,
2. architectural constraints may remain implicit or buried in chat,
3. code reuse points and forbidden change surfaces may be rediscovered inconsistently,
4. implementation can drift away from specs, runtime boundaries, or dependency constraints even when planning was lawful.

## 2. Goal

VIDA v1 should introduce one canonical stage between planning and implementation:

1. `execution_preparation`

This stage exists to:

1. read the bounded task or PBI,
2. study the governing specs and protocols,
3. inspect the relevant codebase and dependency surface,
4. determine what may and may not change,
5. prepare one bounded architecture-preparation handoff for developer execution.

Compact rule:

1. planning does not hand raw tasks directly to developer execution,
2. code-shaped execution starts from a prepared handoff packet,
3. the prepared handoff packet reduces cross-task architectural drift.

## 3. Stage Placement

The canonical v1 runtime flow becomes:

1. `intake`
2. `planning`
3. `execution_preparation`
4. `implementation`
5. `coach`
6. `verification`
7. `approval / closure`

Interpretation rule:

1. `execution_preparation` is not part of raw intake,
2. it is not the same as implementation,
3. it is not a substitute for downstream coach or verifier gates.

## 4. Default Lane Owner

The default lane owner for this stage is:

1. `solution_architect`

Role rule:

1. `business_analyst` shapes scope and requirements,
2. `pm` shapes delivery cut and launch readiness,
3. `solution_architect` prepares bounded implementation architecture and constraints,
4. `worker` executes implementation from the prepared handoff.

## 5. When This Stage Is Required

`execution_preparation` is required by default for:

1. code-shaped implementation tasks,
2. architecture-sensitive changes,
3. tasks that touch multiple modules or dependency surfaces,
4. tasks that rely on governing specs or runtime/protocol constraints,
5. tasks where silent codebase drift would be costly.

It may be bypassed only when:

1. the task class is explicitly marked `small` or `low-risk`,
2. the active policy allows a fast path,
3. the bypass is explicit rather than silent.

## 6. Required Inputs

The `execution_preparation` stage must read and reconcile at least:

1. the bounded task or PBI,
2. the governing specs and protocols,
3. the relevant codebase surface,
4. the relevant dependency and integration surface,
5. current project/runtime constraints,
6. known acceptance and verification direction.

Rule:

1. the stage must not act only from chat wording if stronger evidence sources exist.

## 7. Required Outputs

The stage must produce at least:

1. `architecture_preparation_report`
2. `developer_handoff_packet`
3. `change_boundary`
4. `dependency_impact_summary`
5. `spec_alignment_summary`

### 7.1 `architecture_preparation_report`

Must capture:

1. target implementation area,
2. relevant architecture context,
3. important invariants,
4. integration/dependency concerns,
5. expected implementation shape.

### 7.2 `developer_handoff_packet`

Must capture:

1. the prepared task target,
2. the intended implementation direction,
3. bounded next steps for the developer lane,
4. required proofs/tests/checks,
5. explicit references to the preparation findings.

### 7.3 `change_boundary`

Must capture:

1. what may be changed,
2. what must not be changed,
3. what should be reused rather than rewritten,
4. what surfaces require escalation before mutation.

### 7.4 `dependency_impact_summary`

Must capture:

1. relevant dependencies,
2. likely coupling points,
3. migration or compatibility risks,
4. outward impact that the implementation must preserve.

### 7.5 `spec_alignment_summary`

Must capture:

1. governing specs/protocols,
2. what they require,
3. where implementation must remain aligned,
4. which open questions block execution if unresolved.

## 8. Developer Lane Rule

The developer or worker lane must not begin code-shaped implementation from raw planning output when `execution_preparation` is required.

Instead the developer lane should begin from:

1. a lawful `developer_handoff_packet`,
2. the matching `architecture_preparation_report`,
3. the current execution plan and route authorization.

Forbidden pattern:

1. direct raw `spec-pack -> worker` routing for non-fast-path code work.

## 9. Drift-Reduction Rule

This stage exists specifically to reduce codebase divergence.

That means:

1. repeated tasks in the same codebase should be prepared against one consistent architectural reading,
2. developer lanes should receive bounded architectural constraints instead of reinventing local architecture every time,
3. the same module/dependency boundaries should be preserved across separate tasks unless a later lawful preparation stage explicitly changes them.

## 10. Relation To Teams And Coordination

The common implementation-preparation chain is:

1. `business_analyst -> pm -> solution_architect -> worker -> coach -> verifier`

Team rule:

1. this is the default prepared-implementation chain for architecture-sensitive work,
2. projects may specialize it,
3. they must not silently remove the preparation stage where policy/task class requires it.

## 11. Runtime Queryability

The runtime should be able to answer at least:

1. whether `execution_preparation` is required for the current task,
2. whether it has completed,
3. which preparation artifacts exist,
4. what developer handoff is active,
5. whether execution is blocked by missing preparation.

These answers must be queryable through bounded runtime/status surfaces rather than only through chat recap.

Current operator query surface:

1. `vida taskflow artifacts list --json` reports the execution-preparation registry, missing/materialized posture, source snapshot pointer, and Release-1 operator contract,
2. `vida taskflow artifacts show <artifact-id> --json` reports one artifact entry and fails closed when the requested id is outside the current registry,
3. blocked query output must use canonical `blocker_codes` and shared `operator_contracts` rather than a local prose-only failure.

## 12. Fail-Closed Rule

Execution must fail closed when:

1. the task class requires `execution_preparation`,
2. no lawful preparation artifacts exist,
3. preparation findings are stale after material scope or codebase drift,
4. the handoff packet conflicts with governing specs, boundaries, or route authorization.

Allowed fallback:

1. rerun `execution_preparation`,
2. reopen planning/spec alignment if required.

Forbidden fallback:

1. silent direct continuation into developer execution.

## 13. Relation To Other Specs

This model complements:

1. `agent-role-skill-profile-flow-model.md`
   - role ownership, including `solution_architect`
2. `team-coordination-model.md`
   - coordination chains and team composition
3. `user-facing-runtime-flow-and-operating-loop-model.md`
   - operator-facing placement of `execution_preparation`
4. `compiled-autonomous-delivery-runtime-architecture.md`
   - top-level v1 runtime architecture
5. `release-1-plan.md`
   - active Release-1 execution sequencing and owner plan for introducing this stage
6. `docs/product/research/execution-preparation-and-developer-handoff-survey.md`
   - external grounding for the dedicated pre-execution architecture-preparation stage and structured developer handoff

## 14. Completion Proof

This model is closed enough when:

1. `execution_preparation` is recognized as a first-class runtime stage,
2. `solution_architect` is the default owner for that stage,
3. code-shaped work can require architecture-preparation artifacts before execution,
4. developer execution begins from a bounded handoff rather than from raw planning output,
5. the system can explain when and why preparation is required.

-----
artifact_path: product/spec/execution-preparation-and-developer-handoff-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/execution-preparation-and-developer-handoff-model.md
created_at: '2026-03-12T23:59:59+02:00'
updated_at: 2026-04-26T14:58:34.964682781Z
changelog_ref: execution-preparation-and-developer-handoff-model.changelog.jsonl
