# Autonomous Report Continuation Law

Status: active product law

Purpose: define how VIDA treats intermediate reports as lawful progress boundaries without turning them into unnecessary stop points when the next authorized step is already known.

## 1. Core Rule

`continue_after_reports` is an autonomous-execution policy surface.

It exists to preserve two things at once:

1. intermediate visibility through lawful reports,
2. uninterrupted continuation through the next already-authorized step.

Rule:

1. a report is informational by default, not terminal by default,
2. if the next lawful step is already determined and no blocker exists, runtime should continue automatically after the report,
3. this continuation must remain bounded by existing research, spec, approval, verification, and route law.
4. explicit user intent to discuss the report itself turns that report boundary into a lawful pause point.
5. reports whose purpose is pre-execution validation or implementation validation are gating reports, not auto-continuation reports.

## 2. Canonical Config Surface

Project overlay surface:

1. `vida.config.yaml`
2. `autonomous_execution.continue_after_reports`
3. `autonomous_execution.validation_report_required_before_implementation`

Recommended default:

1. `true`

Meaning:

1. intermediate lawful reports should auto-advance into the next already-authorized step,
2. report boundaries remain visible,
3. report boundaries do not become implicit wait states unless another gate requires it,
4. explicit user requests to discuss the report override automatic continuation for that boundary.

Execution-entry meaning:

1. `validation_report_required_before_implementation=true` inserts a mandatory validation report before each implementation-bearing slice.
2. spec-ready transition into work-pool/dev shaping and post-validation continuation are currently runtime-defined behaviors, not live `autonomous_execution` overlay toggles.

## 3. Lawful Report Stages

Reports may exist at these stages:

1. intake / framing,
2. evidence / reality validation,
3. contract / requirement / decision formation,
4. artifact materialization summary,
5. gate / handoff / closure verdict.

Interpretation rule:

1. reports in stages 1 through 4 are normally intermediate,
2. a pre-execution validation report is gating even if it appears before material execution rather than at final closure,
3. stage 5 may be terminal only when the relevant closure criteria are actually satisfied,
4. otherwise stage 5 is also a handoff boundary rather than a stop boundary.

## 4. Research And Spec Boundary

This policy must not bypass the canonical sequence:

1. research,
2. research artifact update,
3. requirement formation,
4. spec or intake formation,
5. only then practical validation or implementation-facing continuation.

Therefore:

1. `continue_after_reports=true` never authorizes skipping artifact updates,
2. it never authorizes skipping requirement formation,
3. it never authorizes skipping spec/intake refresh,
4. it only removes unnecessary stopping after a lawful intermediate report.

Implementation-entry rule:

1. spec readiness does not bypass the validation gate when `validation_report_required_before_implementation=true`,
2. the lawful execution-entry order is:
   - spec ready,
   - runtime reaches the implementation-entry handoff,
   - validation report generated,
   - validation accepted,
   - implementation starts,
3. after implementation validation passes, runtime resumes the next lawful bounded continuation according to the compiled lane chain.

## 5. TaskFlow Routing Effect

In tracked flow:

1. TaskFlow may emit intermediate summaries, status artifacts, evidence reports, and handoff reports,
2. those reports must preserve the next lawful step,
3. when `continue_after_reports=true`, tracked flow should continue into that next step automatically if:
   - no blocker exists,
   - no approval gate requires pause,
   - no validation gate requires pause,
   - no unresolved material research/spec gap remains,
   - the active bounded unit is explicit rather than inferred heuristically,
   - the user did not explicitly ask to pause,
   - the user did not explicitly ask to discuss the current report.

## 6. Stop Conditions

Automatic continuation after a report is forbidden when any of these are true:

1. the next step would materially widen scope,
2. the next step requires user approval or governance review,
3. the next step requires paid, privileged, or user-owned systems,
4. the current report is a pre-execution validation report or implementation-validation report,
5. material research or validation gaps remain,
6. the intake/spec/contract is still incomplete,
7. the user explicitly requested a pause after the current report,
8. the user explicitly requested discussion of the current report before continuation.
9. the runtime cannot prove one uniquely bound active bounded unit for the continuation step.

## 7. Framework Alignment

This surface aligns with current VIDA law:

1. orchestrator law allows autonomous follow-through when the next lawful step is already determined,
2. command-layer law already distinguishes intermediate reports from closure gates,
3. research law already requires automatic continuation while material gaps remain,
4. TaskFlow law already says progress reporting must not interrupt lawful continuation by itself during continuous autonomous execution.
5. execution-entry validation remains a distinct gate even inside autonomous continuation mode.

So this spec does not create a new execution shortcut.
It normalizes one explicit policy surface across:

1. overlay,
2. command-layer behavior,
3. research/spec progression,
4. TaskFlow routing.

## 8. Completion Proof

This policy is considered wired when:

1. `vida.config.yaml` contains `autonomous_execution.continue_after_reports`,
2. `vida.config.yaml` contains the live validation-gate key `autonomous_execution.validation_report_required_before_implementation`,
3. the project overlay protocol documents only the autonomous-execution keys that runtime actually consumes,
4. command-layer/research/TaskFlow protocols state that non-gating reports do not stop lawful continuation by default,
5. user-facing reporting still remains bounded by approval, verification, and closure law.

-----
artifact_path: product/spec/autonomous-report-continuation-law
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/autonomous-report-continuation-law.md
created_at: '2026-03-10T19:14:51+02:00'
updated_at: '2026-03-12T07:48:27+02:00'
changelog_ref: autonomous-report-continuation-law.changelog.jsonl
