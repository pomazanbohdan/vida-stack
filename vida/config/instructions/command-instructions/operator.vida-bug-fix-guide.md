# /vida-bug-fix — Unified Fix Command

Purpose: one command for single or batch bug fixing with root-cause workflow, regression validation, and spec/doc synchronization.

Primary protocol: `bug-fix-protocol.md` (BFP).

## Protocol Layers

This command maps layers as:

1. `CL1 Intake` -> `BFP-0 Intake` + `BFP-1 Impact`
2. `CL2 Reality And Inputs` -> `BFP-2 Reproduce` + `BFP-3 Root Cause`
3. `CL3 Contract And Decisions` -> `BFP-4 Plan`
4. `CL4 Materialization` -> `BFP-5 Implement`
5. `CL5 Gates And Handoff` -> `BFP-6 Verify` + `BFP-7 Sync` + `BFP-8 Close`

Canonical source: `command-layer-protocol.md`

Fix boundary:

1. `/vida-bug-fix` owns the root-cause fix chain for the normalized issue set.
2. `CL4` is ordered by blast radius and dependency, not by ad hoc retries.
3. `CL5` cannot close the command without regression evidence and doc/spec sync.

## Runtime Contract

1. SSOT for task state: `br` + beads logs.
2. No old phase-command delegation (`/vida-bug-fix-*`).
3. Root-cause-first only; no hotfix as final solution.
4. For server/API issues, live validation is mandatory.

## Inputs

Supported invocation patterns:

1. `/vida-bug-fix "single issue"`
2. `/vida-bug-fix "1) issue A 2) issue B ..."`
3. `/vida-bug-fix "<test/lint/runtime report excerpt>"`

Optional context:

1. related `TaskFlow` task,
2. feature/spec scope,
3. recent test pipeline output.

## Execution Flow (BFP)

1. `BFP-0 Intake` — normalize issue set (`FX-01..FX-N`).
2. `BFP-1 Impact` — severity + blast radius + order.
3. `BFP-2 Reproduce` — reproduce and collect evidence.
4. `BFP-3 Root Cause` — hypothesis and proof path per issue.
5. `BFP-4 Plan` — implementation + regression plan.
6. `BFP-5 Implement` — fix chain execution.
7. `BFP-6 Verify` — regression and side-effect checks.
8. `BFP-7 Sync` — update affected specs/contracts and operational docs.
9. `BFP-8 Close` — result matrix + confidence + risks.

## Required Outputs

1. `Issue Matrix` (id/severity/source/status).
2. `Root Cause Notes`.
3. `Fix/Regression Matrix`.
4. `Documentation/Spec Sync List`.

## Gate Conditions

1. No “fixed” status without reproducible evidence.
2. No “done” without regression checks.
3. No closure without documentation/spec sync for changed behavior.

## Interaction Rules

1. If decision_required mode: user confirms key strategy choices.
2. If autonomous mode: execute end-to-end with checkpoints.
3. Always report IDs + short descriptions for task and active TaskFlows.

## Related

1. `bug-fix-protocol.md`
2. `step-thinking-protocol.md` (bug reasoning)
3. `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`
4. `runtime.task-state-telemetry-protocol.md`

-----
artifact_path: config/command-instructions/vida.bug-fix
artifact_type: command_instruction
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/command-instructions/operator.vida-bug-fix-guide.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-11T13:25:40+02:00'
changelog_ref: operator.vida-bug-fix-guide.changelog.jsonl
