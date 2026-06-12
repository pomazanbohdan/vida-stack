# Agent-Mode Test-First Delivery Flow Model

Status: active product model

Purpose: define the project target where the root session acts as orchestrator and write-producing development is executed through configured VIDA agent roles, carriers, and model profiles.

## Summary

- Model: make agent-mode delivery the default operating model for runtime defect repair and ordinary bounded development.
- Owner layer: `project process | runtime orchestration`
- Runtime surface: `taskflow | agent-init | lane | dispatch | status`
- Status: active product model

## Context

Current recovery work exposed two related problems:

1. runtime execution surfaces can be blocked or view-only, causing the root session to fall back into local implementation too easily,
2. test-first defect repair needs a stronger lane model where analysis, test authoring, implementation, coaching, review, and proof are separated but still cost-controlled.

The project already distinguishes carrier tier/model/cost from runtime role in `vida.config.yaml`. The missing process contract is the exact target flow and the TaskFlow actualization discipline that must happen after each lane result.

## Goals

1. Keep the root session as orchestrator: shape, route, synthesize, update TaskFlow, and decide fallback or escalation.
2. Execute write-producing work through configured VIDA agent roles and carriers whenever the canonical runtime path is available.
3. Use a middle-tier `test_author` / `autotester` lane for regression-test authoring because test quality is part of the defect contract.
4. Use the cheapest eligible developer lane for bounded implementation by default.
5. Use coach, reviewer/verifier, prover, and architect lanes only at their configured gates.
6. Track cost/effectiveness and update routing decisions without hardcoding agent ids, model refs, or host CLI assumptions.
7. Keep TaskFlow current across parent/child layer, priority, dependencies, proof targets, execution semantics, and sequential/parallel posture after every new evidence event.

## Non-Goals

1. Do not replace framework-owned role law.
2. Do not hardcode concrete model refs or carrier names in runtime code.
3. Do not turn host-local subagent APIs into the canonical legality gate.
4. Do not delegate epic or milestone-shaped work.

## Target Lane Chain

For normal write-producing runtime/operator defect work:

1. `orchestrator_synthesis`
   - binds active bounded unit,
   - actualizes TaskFlow,
   - selects configured lane sequence.
2. `analyst`
   - middle-tier by default,
   - reads specs/code/runtime evidence,
   - returns bounded problem statement, owned paths, acceptance contract, and stop rules.
3. `test_author` / `autotester`
   - middle-tier by default,
   - writes or specifies the failing regression test,
   - must prove the test fails for the observed field-level defect, not fixture weakness.
4. `coach_test_gate`
   - validates the regression test against the spec and runtime evidence before implementation.
5. `developer`
   - cheapest eligible write carrier by default,
   - implements only inside the bounded write scope.
6. `duplication_reviewer`
   - checks reuse of existing contracts/helpers and rejects duplicated or unwired logic.
7. `coach_implementation_gate`
   - checks spec conformance and bounded acceptance criteria after implementation.
8. `verifier` / `prover`
   - independently proves closure and release readiness.
9. `orchestrator_synthesis`
   - records results, reroutes/fallbacks if needed, closes or updates the task, and binds the next lawful item.

## TaskFlow Actualization Contract

After every lane result, new defect, blocked dispatch, proof result, diagnostic, or priority change, the orchestrator must update or verify:

1. active task id and parent/child layer,
2. status and close/blocker state,
3. priority and sibling order,
4. dependency edges and blocker relationships,
5. owned/read-only paths,
6. acceptance and proof targets,
7. notes or linked artifacts containing lane evidence,
8. execution semantics and conflict domain,
9. sequential-only versus parallel-safe posture,
10. next lawful lane and stop condition.

If any of these fields are stale or missing, the next action is TaskFlow actualization before the next write-producing lane starts.

## Cost And Effectiveness Contract

Routing should optimize for cost without hiding quality failures:

1. prefer configured low-cost implementation lanes for bounded code changes,
2. use middle-tier lanes for analysis, test authoring, and coach work,
3. use senior/high lanes for independent verification and release proof,
4. use architect/xhigh lanes only for architecture conflict, cross-scope boundary failure, or repeated incoherent closure,
5. record selected role, resolved carrier tier/model profile, task class, duration, normalized cost units when available, outcome, rework count, fallback hops, and proof result.

Promotion is evidence-driven: missing handoff fields, repeated compile/test failure, scope drift, or unresolved design conflict may promote the lane. Demotion is also evidence-driven: routine bounded implementation that does not need high reasoning should return to the cheapest eligible carrier.

## Runtime Blocker Handling

If canonical `vida agent-init` execution is blocked by runtime defects:

1. record the blocker in TaskFlow,
2. use configured host/advisory agents only as bounded evidence or draft carriers,
3. do not treat advisory output as a write receipt,
4. keep repair work scoped to restoring canonical VIDA agent-mode execution,
5. return to canonical delegation as soon as receipts and lane execution are available.

## Required Implementation Work

Full functioning requires these bounded runtime/process slices:

1. Configure a first-class `test_author` / `autotester` lane in the development-team flow, with middle-tier default carrier selection and a handoff contract before implementation.
2. Add or validate a coach test gate that reviews new regression tests before developer implementation when the task is test-first.
3. Ensure `vida agent-init` and related dispatch surfaces can execute or route each configured lane with receipt-backed evidence, not only activation/view output.
4. Persist lane effectiveness telemetry: resolved role, carrier tier, model profile, task class, cost units, duration, outcome, rework count, fallback hops, and proof result.
5. Add TaskFlow actualization support so task status, priority, dependencies, parent/child layer, owned paths, proof targets, execution semantics, and sequential/parallel posture can be refreshed after every new evidence event with minimal command churn.
6. Ensure scheduling/continuation surfaces re-evaluate priority, dependencies, and parallel admissibility after new defects, handoffs, diagnostics, or proof results.
7. Keep all role/carrier/model decisions derived from `vida.config.yaml` and active agent-extension registries; tests may use synthetic names but runtime behavior must not hardcode concrete model refs.
8. Add post-push PR intake to diagnostics: open PRs must be represented as TaskFlow pull-request/work items, prioritized, processed through `docs/process/github-pr-processing-protocol.md`, merged only when current and valid, or closed with comment/intent disposition after valid changes are integrated, superseded, or rejected.
9. Add PR closure cleanup: after merge or close, delete project-owned PR branches automatically when safe and record any cleanup exception in the PR comment and TaskFlow notes.
10. Add a global-goal self-diagnostic gate that compares each happy-path cycle against the target operating model: root orchestrates, configured agents execute, cheapest eligible roles are selected, TaskFlow stays current, diagnostics materialize new goals/tasks, and new global-goal gaps become analyst-reviewed TaskFlow tasks before implementation routing.
11. Add a release-impact version gate for materially advancing task pools: classify patch versus minor, create or update release TaskFlow work, tag the selected version, verify README currency, and verify CI/release artifacts and public release body for that exact version before closure.

## Acceptance Targets

1. `AGENTS.sidecar.md` names the agent-mode delivery target and TaskFlow actualization invariant.
2. `docs/process/team-development-and-orchestration-protocol.md` defines the analyst/test_author/coach/developer/reviewer/verifier/prover chain.
3. `docs/process/project-orchestrator-operating-protocol.md` makes TaskFlow actualization part of the normal loop.
4. The active TaskFlow backlog contains a bounded task for implementing runtime support gaps discovered by this design.
5. A happy-path diagnostic can show pass/fail evidence for root orchestration, delegated execution, cost-aware role selection, TaskFlow actualization, and diagnostic-driven task creation.
6. A release-impact diagnostic can show the patch/minor decision, selected version, tag, GitHub Actions run, GitHub release, manifest version, binary version with build timestamp, public release body, commit ledger, README revision evidence, and installable artifact evidence for the closed task pool.

## Proof Targets

```text
vida docflow check --root . AGENTS.sidecar.md docs/process/team-development-and-orchestration-protocol.md docs/process/project-orchestrator-operating-protocol.md docs/product/spec/agent-mode-test-first-delivery-flow-model.md docs/product/spec/current-spec-map.md docs/product/spec/README.md
```

-----
artifact_path: product/spec/agent-mode-test-first-delivery-flow-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-05-21'
schema_version: '1'
status: canonical
source_path: docs/product/spec/agent-mode-test-first-delivery-flow-model.md
created_at: '2026-05-21T21:10:00+03:00'
updated_at: '2026-05-21T21:10:00+03:00'
changelog_ref: agent-mode-test-first-delivery-flow-model.changelog.jsonl
