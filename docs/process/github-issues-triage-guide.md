# GitHub Issues Triage Guide

Purpose: define the project-owned process for classifying, tagging, and operating GitHub Issues in `pomazanbohdan/vida-stack` after the 2026-05-06 issue-label normalization pass.

## Scope

This guide governs project issue triage in the upstream VIDA stack repository:

1. GitHub repository: `pomazanbohdan/vida-stack`
2. issue labels used for operational classification,
3. issue status and priority labels,
4. VIDA diagnostic issue publication and duplicate handling,
5. future issue-form defaults when repository issue templates are added.

This guide does not replace:

1. `docs/product/spec/github-public-repository-law.md` for GitHub-native public repository law,
2. `docs/process/release-formatting-protocol.md` for GitHub release pages,
3. TaskFlow as the authoritative internal backlog and execution store.

## Operating Model

GitHub Issues are the public/upstream tracking surface for VIDA runtime, process, repository, and diagnostic follow-ups that must be visible outside the local TaskFlow state.

TaskFlow remains the execution authority. A GitHub issue does not grant root-lane write permission, close a TaskFlow task, or replace a VIDA receipt. It provides public context, stable discussion, searchability, and cross-project defect aggregation.

## Repository Identity Rule

Runtime and framework findings must be published to the upstream VIDA stack issue tracker even when evidence is collected in a downstream VIDA-initialized product project.

Rules:

1. current downstream project identity is evidence context only,
2. upstream runtime/framework publication target is `pomazanbohdan/vida-stack`,
3. do not infer the issue tracker from the current product repository `origin`,
4. sanitize downstream project paths, tokens, private config, and unrelated dirty-file content before issue publication,
5. search existing open issues before creating a new issue.

## Label Taxonomy

The project uses prefixed labels for deterministic filtering.

### Type Labels

Use exactly one primary `type:*` label for normal issues:

1. `type: defect` - observed broken behavior, regression, or runtime inconsistency.
2. `type: feature` - new capability or functional enhancement.
3. `type: task` - implementation, maintenance, or operational task.
4. `type: docs` - documentation, generated instructions, or scaffold text.
5. `type: refactor` - internal restructure without intended behavior change.

GitHub default labels such as `bug`, `enhancement`, and `documentation` may remain for public familiarity, but the prefixed `type:*` labels are the project-operational classification source.

### Area Labels

Use one or more `area:*` labels to show ownership and impact:

1. `area: runtime`
2. `area: taskflow`
3. `area: docflow`
4. `area: agent-dispatch`
5. `area: bootstrap`
6. `area: github`

Add additional `area:*` labels only when a repeated project-owned area needs stable filtering.

### Priority Labels

Use exactly one priority label when severity is known:

1. `priority: p0` - critical; breaks releases/security or blocks all work.
2. `priority: p1` - high; blocks important development or recovery path.
3. `priority: p2` - medium; important but has workaround or bounded impact.
4. `priority: p3` - low; cleanup or polish.

Prefer under-classifying to `p2` while evidence is incomplete. Promote to `p1` or `p0` only when impact is specific.

### Status Labels

Use exactly one active status label:

1. `status: triage` - needs classification, owner, or implementation direction.
2. `status: needs-design` - needs architecture/design decision before implementation.
3. `status: ready` - clear enough to schedule or implement.
4. `status: in-progress` - actively being worked.
5. `status: blocked` - cannot proceed until a dependency or external blocker clears.
6. `status: needs-info` - needs more evidence, reproduction details, or clarification.

When status changes, remove the previous `status:*` label rather than stacking multiple lifecycle states.

### Source Labels

Use `source: diagnostic` when an issue is created by, or materially extended by, VIDA runtime self-diagnostic evidence.

This label means the issue body or comments should preserve command evidence, observed behavior, expected behavior, impact, and sanitized reproduction context.

## Current Issue Classification

The 2026-05-06 pass applied the taxonomy to the current open issues:

1. `#114` - `type: defect`, `area: runtime`, `area: taskflow`, `area: docflow`, `area: agent-dispatch`, `priority: p1`, `status: triage`, `source: diagnostic`.
2. `#115` - `type: docs`, `area: bootstrap`, `area: runtime`, `area: github`, `priority: p2`, `status: ready`, `source: diagnostic`.
3. `#116` - `type: feature`, `area: runtime`, `area: taskflow`, `area: agent-dispatch`, `priority: p1`, `status: needs-design`, `source: diagnostic`.

Existing `codex` and `aardvark` labels are preserved because they are already used on pull requests. They are not the canonical issue triage taxonomy.

## Triage Procedure

For each new or updated issue:

1. Search existing open issues for duplicate or related evidence.
2. Decide whether the issue belongs in upstream `pomazanbohdan/vida-stack` or only in a downstream product project.
3. Apply one `type:*` label.
4. Apply all relevant `area:*` labels.
5. Apply one `priority:*` label when impact is clear.
6. Apply one `status:*` label.
7. Add `source: diagnostic` when runtime diagnostics are the source.
8. Link related issues in the body or a comment when one issue is a design follow-up for another.

Duplicate policy:

1. If the new evidence fully matches an existing issue and adds no new details, comment with date, project context, and reproduction workflow instead of creating a duplicate.
2. If the evidence adds a new error shape, command surface, environment, or reproduction path, comment on the existing issue with the new details.
3. Create a new issue only when no existing issue covers the same underlying defect or improvement.

## Command Reference

Common read commands:

```powershell
gh issue list --repo pomazanbohdan/vida-stack --state open --json number,title,labels
gh label list --repo pomazanbohdan/vida-stack --limit 200
gh issue view <number> --repo pomazanbohdan/vida-stack --json number,title,body,labels,comments
```

Common mutation commands:

```powershell
gh label create "type: defect" --repo pomazanbohdan/vida-stack --color d73a4a --description "Observed broken behavior, regression, or runtime inconsistency" --force
gh issue edit <number> --repo pomazanbohdan/vida-stack --add-label "type: defect,priority: p1,status: triage"
gh issue edit <number> --repo pomazanbohdan/vida-stack --remove-label "status: triage" --add-label "status: ready"
```

Use the GitHub connector when structured issue mutation is available in the active host. Use `gh` for label creation and other repository metadata operations not covered by the connector.

## Issue Forms And Issue Types

Issue forms may be added later under `.github/ISSUE_TEMPLATE/**` to prefill labels and required fields.

Rules:

1. labels referenced by issue forms must already exist in the repository,
2. issue forms may set default labels and assignees,
3. issue forms can set GitHub issue type only where that GitHub feature is available,
4. GitHub issue types are organization-level; for this personal-account repository, prefixed labels remain the practical project taxonomy,
5. when issue forms are introduced, update this guide and `docs/product/spec/github-public-repository-law.md` if repository intake law changes.

## External Baseline

This guide follows the current GitHub model where:

1. labels categorize issues, pull requests, and discussions within a repository,
2. issue forms can define fields plus default labels,
3. issue types are managed at organization level and are separate from repository labels.

Reference URLs:

1. `https://docs.github.com/en/issues/using-labels-and-milestones-to-track-work/managing-labels`
2. `https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-issue-forms`
3. `https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/managing-issue-types-in-an-organization`

-----
artifact_path: process/github-issues-triage-guide
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-05-06'
schema_version: '1'
status: canonical
source_path: docs/process/github-issues-triage-guide.md
created_at: '2026-05-06T11:52:00+03:00'
updated_at: '2026-05-06T11:52:00+03:00'
changelog_ref: github-issues-triage-guide.changelog.jsonl
