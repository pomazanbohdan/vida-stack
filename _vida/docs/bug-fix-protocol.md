# Bug Fix Protocol (BFP)

Purpose: one unified algorithm for fixing single or multiple errors after task/pool development, with mandatory regression and documentation/spec synchronization.

Scope:

1. Primary runtime for `/vida-bug-fix`.
2. Applies to bug batches from test/lint/runtime outputs.
3. Compatible with `br` SSOT + beads TODO execution.

## Command Layer Mapping

For `/vida-bug-fix`, BFP layers map to CLP as follows:

1. `CL1 Intake` -> `BFP-0 Intake` + `BFP-1 Impact & Priority`
2. `CL2 Reality And Inputs` -> `BFP-2 Reproduce & Validate` + `BFP-3 Root Cause`
3. `CL3 Contract And Decisions` -> `BFP-4 Fix Plan`
4. `CL4 Materialization` -> `BFP-5 Implement`
5. `CL5 Gates And Handoff` -> `BFP-6 Verify & Regression` + `BFP-7 Documentation/Spec Sync` + `BFP-8 Closure`

Canonical layer source: `_vida/docs/command-layer-protocol.md`

## Input Contract

`/vida-bug-fix` accepts:

1. single issue description,
2. numbered list of issues,
3. test/log report snippet,
4. task or pool context (`br` task, milestone, feature scope).

## Unified Flow (BFP-0..8)

1. `BFP-0 Intake`:
   - normalize issue list (`FX-01..FX-N`),
   - map each issue to source evidence (test/log/crash/API).
2. `BFP-1 Impact & Priority`:
   - classify severity (`low/medium/high/critical`),
   - estimate blast radius,
   - mark fix order.
3. `BFP-2 Reproduce & Validate`:
   - reproduce each issue,
   - for server/API assumptions run live checks (`curl` or equivalent),
   - capture status/payload/error evidence.
4. `BFP-3 Root Cause`:
   - run root-cause-first reasoning (no hotfix),
   - build falsifiable hypothesis per issue.
5. `BFP-4 Fix Plan`:
   - define implementation sequence and dependencies,
   - define regression tests per issue.
6. `BFP-5 Implement`:
   - apply fixes in ordered chain,
   - add/adjust tests.
7. `BFP-6 Verify & Regression`:
   - verify original reproductions are resolved,
   - run regression checks to detect side effects.
8. `BFP-7 Documentation/Spec Sync`:
   - update impacted specs/contracts and operational docs immediately.
9. `BFP-8 Closure`:
   - provide result matrix (fixed/partial/blocked),
   - confidence + residual risks + next actions.

## Mandatory Artifacts

1. `Issue Matrix`: id, severity, source, status.
2. `Root Cause Notes`: issue -> cause -> evidence.
3. `Fix/Regression Matrix`: issue -> fix -> tests -> result.
4. `Doc Sync List`: updated spec/docs/contracts.

## Gates

1. No fix without reproducible evidence (or explicit reason why not reproducible).
2. No final success without regression evidence.
3. No closure without documentation/spec sync for affected behavior.
4. No hotfix-style unresolved workaround as final state.
5. If a root-cause fix requires a non-equivalent product/UX/contract choice,
   escalate to the user before implementation instead of silently selecting a branch.

## Batch Handling

For N issues:

1. prioritize by severity and dependency,
2. allow parallel investigation if scopes are independent,
3. serialize conflicting code scopes,
4. close each issue with explicit status (`fixed|partial|blocked`).

## Transparency Rules

Status reports must include:

1. current task id/title/description,
2. current BFP step,
3. in-progress issue id(s),
4. next issue/step,
5. confidence and open risks.
