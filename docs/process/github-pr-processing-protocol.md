# GitHub PR Processing Protocol

Purpose: define the project-owned operating protocol for processing open GitHub pull requests in `vida-stack` when the operator is asked to merge, close, manually integrate, clean branches, and return to `main`.

## Scope

This protocol applies to project-local PR triage and batch processing for the active `vida-stack` repository.

It covers:

1. open PR inventory,
2. validation of each PR's intended functional fix,
3. merge versus close decisions,
4. manual integration of useful fixes from stale, duplicate, conflicting, or failing PRs,
5. branch cleanup,
6. return-to-main and push closure.

It does not replace GitHub issue triage, release publication, framework bootstrap law, TaskFlow lane law, or DocFlow document law.

## Required Operating Sequence

1. Restore project context before mutation.
   - Read the project bootstrap/sidecar context when the session has drifted.
   - Inspect `git status --short --branch`.
   - Resolve the current remote and ensure the target base is `main` unless the user explicitly names another base.

2. Inventory open PRs.
   - Use `gh pr list --state open --json number,title,headRefName,baseRefName,mergeable,statusCheckRollup,url`.
   - Capture duplicate titles, duplicate commits, conflicting branches, failing checks, pending checks, and stale branches.
   - Do not merge a PR solely because it is open or mergeable.

3. Revalidate each PR's intent.
   - Read PR title/body/files/commits.
   - Compare the patch against current `origin/main`.
   - Classify the PR as one of:
     - directly mergeable and still useful,
     - useful but stale/conflicting and requiring manual integration,
     - duplicate of another PR,
     - obsolete because current `main` already contains the behavior,
     - invalid or failing without a useful functional change.

4. Inspect failing or pending checks before deciding.
   - For failed CI, inspect available job logs.
   - If logs are not yet available but the failure can be reproduced locally, run the relevant local check.
   - When a PR has a useful fix but also a compile/test defect, integrate the useful fix manually and repair the defect in the integration branch rather than merging the failing branch.

5. Integrate useful changes on a fresh branch from current main.
   - Fetch and prune first.
   - Create a temporary integration branch from `origin/main`.
   - Cherry-pick or manually port exactly one copy of each unique useful fix.
   - Resolve conflicts against current `main`; preserve current main behavior unless the PR's fix intentionally replaces it.
   - If multiple PRs overlap, keep the logically strongest combined behavior and avoid duplicate commits.

6. Verify before publishing.
   - Run formatting for touched languages when applicable.
   - Run at least one compile-level check for changed code.
   - Run targeted tests for the functional behavior being integrated.
   - If a full test suite is impractical, record the bounded proof that was actually run and any residual risk.

7. Publish through main.
   - Switch back to `main`.
   - Fast-forward or merge the verified integration branch into `main`.
   - Push `main` to `origin`.
   - Delete the temporary local integration branch after it is fully contained in `main`.

8. Close processed PRs.
   - For PRs manually integrated, close with a comment naming the integration commit or commit range and state that the useful behavior is now present on `main`.
   - For duplicate PRs, close as duplicates after confirming the duplicate behavior is integrated or intentionally rejected.
   - For invalid PRs, close with the validation reason.
   - Every processed PR must end in exactly one GitHub terminal action: merge when the PR is current, non-duplicate, green, and still the accepted integration path; otherwise close it after its creation reason has been analyzed and either integrated, superseded, or rejected.
   - Closing comments are mandatory and automatic. The comment must name the reason for closure, the disposition of the PR's intended fix, any integration commit or replacement task when applicable, and the check or validation evidence that blocked merge when the PR was not mergeable.
   - Delete remote head branches automatically for closed PRs when they are project-owned cleanup branches. Leave a branch only when GitHub permissions deny deletion or when the branch is not project-owned; record that exception in the closure comment/report.

9. Final sanity checks.
   - Fetch with prune.
   - Confirm `git status --short --branch` shows clean `main` tracking `origin/main`.
   - Confirm `gh pr list --state open` is empty or contains only PRs intentionally left open.
   - Run the appropriate VIDA runtime diagnostic when the batch closes a runtime/process slice.

## Decision Rules

1. Prefer direct merge only when the PR is current, non-duplicate, green enough for project policy, and still expresses the intended fix cleanly.
2. Prefer manual integration when the PR is useful but stale, conflicting, duplicated, or failing due to a repairable integration defect.
3. Close without integration only when the intended behavior is already present, invalid, obsolete, or has no logical functional value after revalidation.
4. Never leave useful behavior stranded in a closed PR. If closing a useful PR, first integrate or recreate its functional fix on `main`.
5. Never leave project-owned cleanup branches behind after their PRs are closed and their useful changes are integrated or rejected.
6. Never leave a processed PR open merely because it was reviewed. After processing, automatically merge it or close it with the required comment and branch cleanup.

## Proof Contract

A completed PR-processing batch must report:

1. processed PR numbers,
2. merge/manual-integration/close classification,
3. commits pushed to `main`,
4. checks/tests run,
5. closed PRs and deleted branches,
6. final branch and worktree state.

-----
artifact_path: process/github-pr-processing-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-05-16'
schema_version: '1'
status: canonical
source_path: docs/process/github-pr-processing-protocol.md
created_at: '2026-05-16T00:00:00+03:00'
updated_at: 2026-05-16T14:06:57.5397037Z
changelog_ref: github-pr-processing-protocol.changelog.jsonl
