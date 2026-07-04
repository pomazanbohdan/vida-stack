# GitHub PR Processing Protocol

## Purpose

Define the project-owned operating protocol for processing open GitHub pull requests in `vida-stack` only after a current explicit operator instruction authorizes the specific PR-processing batch or PR-processing pattern that may merge, close, manually integrate, clean branches, and return to `main`.

## Trigger

Use this protocol when the operator explicitly authorizes a `vida-stack` PR-processing batch, a named PR, or a repeatable PR-processing pattern that may inspect, merge, manually integrate, comment on, close, clean branches, publish `main`, or perform post-merge service operations.

## Scope

This protocol applies to project-local PR triage and batch processing for the active `vida-stack` repository.

Remote-mutation authorization gate:

1. wave closure, task closure, or a clean local commit is not authorization by itself to run this PR-processing protocol, push `main`, close PRs, comment on PRs, merge PRs, or delete remote branches,
2. before inventory expands into any remote-mutating PR-processing batch, the operator must have given a current affirmative request naming that specific PR batch, publication scope, or repeatable publication pattern,
3. if that authorization is absent or stale relative to the target batch, record PR-processing as an available follow-up and stop before any GitHub mutation.

It covers:

1. open PR inventory,
2. TaskFlow intake for each PR as a pull-request/work item,
3. validation of each PR's intended functional fix,
4. merge versus close decisions,
5. manual integration of useful fixes from stale, duplicate, conflicting, or failing PRs,
6. branch cleanup,
7. return-to-main and push closure.

It does not replace GitHub issue triage, release publication, framework bootstrap law, TaskFlow lane law, or DocFlow document law.

## Authority

1. This document owns the project-local GitHub PR-processing workflow for `vida-stack`.
2. `docs/process/zombie-d-test-writing-protocol.md` owns the required ZOMBIE-D matrix and closure gate for PR-related test proof.
3. `AGENTS.md` and `AGENTS.sidecar.md` own runtime bootstrap, TaskFlow step-before-write, root write, and final-report law.
4. GitHub remote mutation authority still requires a current explicit operator instruction for the target PR batch.
5. TaskFlow remains the state authority for PR work items, process tasks, proof evidence, and closure.

## Inputs

Each PR-processing batch must identify:

1. explicit operator authorization scope,
2. target repository, remote, and base branch,
3. open PR inventory with number, title, URL, branch, base, mergeability, checks, and changed files,
4. TaskFlow PR work item ids,
5. intended fix or disposition for each PR,
6. related tests and proof commands for each processed PR,
7. ZOMBIE-D category coverage for PR-related tests before service operations,
8. branch cleanup and GitHub terminal actions to perform after validation.

## Outputs

A completed PR-processing batch must produce:

1. integrated or intentionally rejected PR dispositions,
2. commits pushed to `main`,
3. ZOMBIE-D validation evidence for PR-related tests,
4. PR comments, merges, closes, or branch cleanup actions,
5. TaskFlow proof and closure evidence,
6. final branch, worktree, and open-PR state.

## Rules

### Required Operating Sequence

1. Restore project context before mutation.
   - Read the project bootstrap/sidecar context when the session has drifted.
   - Inspect `git status --short --branch`.
   - Resolve the current remote and ensure the target base is `main` unless the user explicitly names another base.
   - Confirm a current explicit operator instruction authorizes the specific remote-mutating PR-processing batch before any push, merge, PR comment, PR close, or remote branch deletion.

2. Inventory open PRs.
   - Use `gh pr list --state open --json number,title,headRefName,baseRefName,mergeable,statusCheckRollup,url`.
   - Capture duplicate titles, duplicate commits, conflicting branches, failing checks, pending checks, and stale branches.
   - Do not merge a PR solely because it is open or mergeable.

3. Create or update TaskFlow PR work items.
   - PRs are TaskFlow pull-request/work items, not defect tasks by default.
   - Record PR number, title, URL, branch, base, changed-file summary, checks, mergeability, intended fix, current classification, priority, dependency/conflict hints, and next action.
   - Re-evaluate priority against active runtime goals and current defect batches before processing order is fixed.

4. Revalidate each PR's intent.
   - Read PR title/body/files/commits.
   - Compare the patch against current `origin/main`.
   - Classify the PR as one of:
     - directly mergeable and still useful,
     - useful but stale/conflicting and requiring manual integration,
     - duplicate of another PR,
     - obsolete because current `main` already contains the behavior,
     - invalid or failing without a useful functional change.

5. Inspect failing or pending checks before deciding.
   - For failed CI, inspect available job logs.
   - If logs are not yet available but the failure can be reproduced locally, run the relevant local check.
   - When a PR has a useful fix but also a compile/test defect, integrate the useful fix manually and repair the defect in the integration branch rather than merging the failing branch.

6. Integrate useful changes on a fresh branch from current main.
   - Fetch and prune first.
   - Create a temporary integration branch from `origin/main`.
   - Cherry-pick or manually port exactly one copy of each unique useful fix.
   - Resolve conflicts against current `main`; preserve current main behavior unless the PR's fix intentionally replaces it.
   - If multiple PRs overlap, keep the logically strongest combined behavior and avoid duplicate commits.

7. Verify before publishing.
   - Run formatting for touched languages when applicable.
   - Run at least one compile-level check for changed code.
   - Run targeted tests for the functional behavior being integrated.
   - If a full test suite is impractical, record the bounded proof that was actually run and any residual risk.

8. Publish through main.
   - Switch back to `main`.
   - Fast-forward or merge the verified integration branch into `main`.
   - Push `main` to `origin`.
   - Do not start service operations yet.

9. Validate PR-related tests against ZOMBIE-D before service operations.
   - After all PR changes in the authorized batch are merged, manually integrated, or intentionally rejected on `main`, identify the tests that prove each processed PR's intended behavior.
   - Validate those PR-related tests against `docs/process/zombie-d-test-writing-protocol.md` before branch cleanup, release/install, final diagnostics, TaskFlow closeout, PR closure comments, or other service operations.
   - The validation must confirm that the in-scope test batch has a filled Z/O/M/B/I/E/S matrix or an explicit non-applicable reason for each missing category.
   - If the PR changed runtime, TaskFlow, DocFlow, lane, run-graph, recovery, receipt, CLI, JSON, TOON/plain, fixture, snapshot, or operator behavior, the PR-related tests must include the public-surface, fail-closed blocker, `next_actions`, `artifact_refs`, and persisted-state coverage required by the ZOMBIE-D protocol.
   - If the existing tests are not ZOMBIE-D-compliant, stop before service operations, either update the tests in the same integration batch or create a blocking TaskFlow follow-up that names the uncovered ZOMBIE-D categories and why the PR batch cannot be operationally closed yet.
   - Record the ZOMBIE-D validation result in the PR-processing report and TaskFlow evidence, including the PR numbers, related test names or commands, covered categories, uncovered categories, and any follow-up ids.

10. Close processed PRs.
   - For PRs manually integrated, close with a comment naming the integration commit or commit range and state that the useful behavior is now present on `main`.
   - For duplicate PRs, close as duplicates after confirming the duplicate behavior is integrated or intentionally rejected.
   - For invalid PRs, close with the validation reason.
   - Every processed PR must end in exactly one GitHub terminal action: merge when the PR is current, non-duplicate, green, and still the accepted integration path; otherwise close it after its creation reason has been analyzed and either integrated, superseded, or rejected.
   - Closing comments are mandatory and automatic. The comment must name the reason for closure, the disposition of the PR's intended fix, any integration commit or replacement task when applicable, and the check or validation evidence that blocked merge when the PR was not mergeable.
   - Delete remote head branches automatically for closed PRs when they are project-owned cleanup branches. Leave a branch only when GitHub permissions deny deletion or when the branch is not project-owned; record that exception in the closure comment/report.

11. Final sanity checks.
   - Fetch with prune.
   - Confirm `git status --short --branch` shows clean `main` tracking `origin/main`.
   - Confirm `gh pr list --state open` is empty or contains only PRs intentionally left open.
   - Run the appropriate VIDA runtime diagnostic when the batch closes a runtime/process slice.

### Decision Rules

1. Prefer direct merge only when the PR is current, non-duplicate, green enough for project policy, and still expresses the intended fix cleanly.
2. Prefer manual integration when the PR is useful but stale, conflicting, duplicated, or failing due to a repairable integration defect.
3. Close without integration only when the intended behavior is already present, invalid, obsolete, or has no logical functional value after revalidation.
4. Never leave useful behavior stranded in a closed PR. If closing a useful PR, first integrate or recreate its functional fix on `main`.
5. Never leave project-owned cleanup branches behind after their PRs are closed and their useful changes are integrated or rejected.
6. Never leave a processed PR open merely because it was reviewed. After processing, automatically merge it or close it with the required comment and branch cleanup.
7. Never treat a PR-processing batch as service-ready until the PR-related proof tests have been checked against the ZOMBIE-D protocol or the batch has a blocking TaskFlow follow-up for each uncovered category.

## Forbidden

1. Do not expand PR inventory into remote mutation without current explicit operator authorization for the target batch.
2. Do not merge a PR solely because it is open or mergeable.
3. Do not close a useful PR before its useful behavior is integrated, superseded by a named commit, or rejected with evidence.
4. Do not start service operations after PR integration until PR-related tests have ZOMBIE-D validation evidence or blocking follow-up ids.
5. Do not delete non-project-owned branches or branches whose cleanup authority is unclear.
6. Do not treat chat-only notes as closure evidence when TaskFlow evidence is required.

## Escalation

1. If PR checks fail and logs are available, inspect logs before deciding merge, manual integration, or rejection.
2. If PR-related tests are missing ZOMBIE-D coverage, stop before service operations and update tests or create blocking TaskFlow follow-ups.
3. If GitHub permissions deny comment, close, merge, push, or branch deletion, record the exact blocker in the report and TaskFlow evidence.
4. If runtime diagnostics conflict after the batch, classify the conflict as a VIDA runtime blocker or follow-up before closing the process task.
5. If authorization scope is stale or ambiguous, stop before remote mutation and ask for a fresh operator instruction.

## Validation

### Proof Contract

A completed PR-processing batch must report:

1. processed PR numbers,
2. merge/manual-integration/close classification,
3. commits pushed to `main`,
4. checks/tests run,
5. ZOMBIE-D validation for PR-related tests, including covered and uncovered categories,
6. closed PRs and deleted branches,
7. TaskFlow item ids updated or closed,
8. final branch and worktree state.

Validation commands are selected by changed surface. Minimum process validation for a documentation-only update to this protocol is:

1. `vida docflow check-file --path docs/process/github-pr-processing-protocol.md --json`
2. `vida task validate-graph --json`
3. `git diff --check`

## Token Budget

No fixed token budget is declared for this protocol. Acceptance depends on preserving operational authority, required anchors, PR-processing steps, ZOMBIE-D validation gates, and DocFlow validation. If this document becomes an always-loaded bootstrap artifact, compress it under `docs/product/spec/protocol-authoring-and-token-economy-law.md`.

## Metadata

-----
artifact_path: process/github-pr-processing-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-07-04'
schema_version: '1'
status: canonical
source_path: docs/process/github-pr-processing-protocol.md
created_at: '2026-05-16T00:00:00+03:00'
updated_at: 2026-07-04T00:00:00+03:00
changelog_ref: github-pr-processing-protocol.changelog.jsonl
