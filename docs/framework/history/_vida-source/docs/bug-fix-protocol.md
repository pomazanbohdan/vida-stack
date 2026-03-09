# Bug Fix Protocol (BFP)

Purpose: one unified `issue-as-contract` algorithm for fixing single or multiple reported errors, with mandatory equivalence gating, regression, and documentation/spec synchronization.

Scope:

1. Primary runtime for `/vida-bug-fix`.
2. Applies to bug batches from test/lint/runtime outputs.
3. Compatible with `br` SSOT + beads TODO execution.

## Command Layer Mapping

For `/vida-bug-fix`, BFP layers map to CLP as follows:

1. `CL1 Intake` -> `BFP-0 Intake` + `BFP-1 Impact & Priority`
2. `CL2 Reality And Inputs` -> `BFP-2 Issue Classification` + `BFP-3 Issue Contract` + `BFP-4 Reproduce & Validate`
3. `CL3 Contract And Decisions` -> `BFP-5 Equivalence Gate` + `BFP-6 Root Cause & Fix Plan`
4. `CL4 Materialization` -> `BFP-7 Implement`
5. `CL5 Gates And Handoff` -> `BFP-8 Verify & Regression` + `BFP-9 Documentation/Spec Sync` + `BFP-10 Closure`

Canonical layer source: `_vida/docs/command-layer-protocol.md`

## Input Contract

`/vida-bug-fix` accepts:

1. single issue description,
2. numbered list of issues,
3. test/log report snippet,
4. task or pool context (`br` task, milestone, feature scope).

## Unified Flow (BFP-0..10)

1. `BFP-0 Intake`:
   - normalize issue list (`FX-01..FX-N`),
   - map each issue to source evidence (test/log/crash/API).
2. `BFP-1 Impact & Priority`:
   - classify severity (`low/medium/high/critical`),
   - estimate blast radius,
   - mark fix order.
3. `BFP-2 Issue Classification`:
   - classify each issue as `defect_equivalent|defect_needs_contract_update|feature_delta|as_designed|not_a_bug|insufficient_evidence`.
4. `BFP-3 Issue Contract`:
   - build the canonical `issue_contract` artifact by `_vida/docs/issue-contract-protocol.md`,
   - separate `reported_scope` from `proven_scope` before any writer-ready decision,
   - if one issue contains both a primary executable slice and secondary unresolved symptoms, emit the `issue-split` artifact instead of silently widening the current bug fix,
   - do not send a writer lane raw bug text when the issue contract is still missing.
5. `BFP-4 Reproduce & Validate`:
   - reproduce each issue,
   - for server/API assumptions run live checks (`curl` or equivalent),
   - capture status/payload/error evidence.
6. `BFP-5 Equivalence Gate`:
   - if `issue_contract.status=writer_ready`, continue to fix planning,
   - for multi-symptom issues, each in-scope symptom must already have repro/red-test/live evidence or be explicitly excluded,
   - if `issue_contract.status=spec_delta_required`, materialize `spec_delta` and reconcile spec/product contract first,
   - if `issue_contract.status=issue_closed_no_fix`, close with rationale,
   - if `issue_contract.status=insufficient_evidence`, gather more evidence before implementation.
7. `BFP-6 Root Cause & Fix Plan`:
   - run root-cause-first reasoning (no hotfix),
   - build falsifiable hypothesis per issue.
   - define implementation sequence and dependencies,
   - for multi-issue pools, build the issue graph and classify `blocked|soft-blocked|parallel-investigation|single-writer` before choosing fix order,
   - define regression tests per issue.
8. `BFP-7 Implement`:
   - apply fixes in ordered chain,
   - add/adjust tests.
9. `BFP-8 Verify & Regression`:
   - verify original reproductions are resolved,
   - run regression checks to detect side effects.
10. `BFP-9 Documentation/Spec Sync`:
   - update impacted specs/contracts and operational docs immediately,
   - if the issue was non-equivalent, the pre-implementation spec reconciliation is mandatory, not optional.
11. `BFP-10 Closure`:
   - provide result matrix (fixed/partial/blocked),
   - confidence + residual risks + next actions.

## Mandatory Artifacts

1. `Issue Matrix`: id, severity, source, status.
2. `Issue Contract`: normalized equivalence decision and acceptance slice.
3. `Spec Delta`: mandatory when the issue is non-equivalent.
4. `Root Cause Notes`: issue -> cause -> evidence.
5. `Fix/Regression Matrix`: issue -> fix -> tests -> result.
6. `Doc Sync List`: updated spec/docs/contracts.

## Gates

1. No fix without reproducible evidence (or explicit reason why not reproducible).
2. No writer lane without `issue_contract` when the issue text is the execution spec.
3. No final success without regression evidence.
4. No closure without documentation/spec sync for affected behavior.
5. No hotfix-style unresolved workaround as final state.
6. If a root-cause fix requires a non-equivalent product/UX/contract choice,
   escalate to the user before implementation instead of silently selecting a branch.

## Batch Handling

For N issues:

1. prioritize by severity and dependency,
2. allow parallel investigation if scopes are independent,
3. serialize conflicting code scopes,
4. close each issue with explicit status (`fixed|partial|blocked`).
5. if the batch lacks an explicit dependency graph, implementation order is not yet valid.

## Transparency Rules

Status reports must include:

1. current task id/title/description,
2. current BFP step,
3. in-progress issue id(s),
4. next issue/step,
5. confidence and open risks.
