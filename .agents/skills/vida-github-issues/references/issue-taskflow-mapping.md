# Issue To TaskFlow Mapping

Use this reference when mapping GitHub Issues to TaskFlow.

## Required Evidence

1. GitHub issue number, title, state, labels, URL, and latest meaningful comment.
2. Matching TaskFlow task id or reason no task exists.
3. Parent epic or current active epic.
4. Proof target and current proof status.
5. Duplicate/stale/superseded decision when closing.

## Current Project Defaults

Repository:

```text
pomazanbohdan/vida-stack
```

Default open issue read:

```powershell
gh issue list --repo pomazanbohdan/vida-stack --state open --limit 200 --json number,title,state,url,labels,updatedAt,createdAt,body
```

Default validation:

```powershell
vida task validate-graph --json
gh issue list --repo pomazanbohdan/vida-stack --state open --limit 50 --json number,title,state,url
```

## Labels

Prefer the prefixed taxonomy from `docs/process/github-issues-triage-guide.md`:

- `type:*`
- `area:*`
- `priority:*`
- `status:*`
- `source: diagnostic`

TaskFlow labels should include `github:<number>` for issue-linked work.
