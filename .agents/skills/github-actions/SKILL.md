---
name: github-actions
description: Design, audit, repair, and document GitHub Actions workflows using current official GitHub documentation. Use when working with .github/workflows YAML, CI/CD pipelines, workflow triggers, jobs, matrices, permissions, caching, artifacts, reusable workflows, runners, GITHUB_TOKEN, secrets, OIDC, artifact attestations, workflow security hardening, or GitHub Actions troubleshooting.
---

# GitHub Actions

Use this skill to produce GitHub Actions work that is current, secure, and verifiable. Do not answer from stale memory when workflow syntax, permissions, runner behavior, security controls, or action versions matter.

## Core Workflow

1. Classify the task:
   - `design`: create a new workflow or reusable workflow.
   - `audit`: review an existing workflow for correctness, security, cost, speed, and maintainability.
   - `repair`: fix a failing or risky workflow.
   - `spec`: document workflow behavior and quality gates.
2. Inspect local workflows first when a repository is available:
   - `.github/workflows/*.yml`
   - `.github/workflows/*.yaml`
   - action files such as `action.yml` or `action.yaml`
   - CI helper scripts invoked by workflows
3. Load the minimum reference needed:
   - `references/official-docs.md` for canonical GitHub Docs links and current rules.
   - `references/workflow-patterns.md` for workflow templates, CI/CD stages, publish/deploy patterns, audit checks, and security patterns.
4. Validate against official GitHub Docs before making claims about:
   - workflow syntax, events, contexts, expressions, variables, and limits
   - permissions and `GITHUB_TOKEN`
   - `pull_request_target`, forked PRs, and untrusted input
   - reusable workflow limitations
   - dependency caching, artifacts, OIDC, and attestations
   - runner labels, hosted runner images, larger runners, self-hosted runners, and ARC
5. Prefer small, explicit workflows:
   - state `permissions` at workflow or job level
   - use `concurrency` for PR/build/deploy flows that should not overlap
   - use matrices only when the cross-product is intentional
   - split long shell blocks into checked-in scripts when they need tests or reuse
   - use reusable workflows for organization-wide policy and repeated CI structure
6. Validate the result:
   - run `actionlint` when available
   - run `gh workflow list` or `gh workflow view` when GitHub CLI and auth are available
   - run local commands invoked by the workflow when feasible
   - for fixes, inspect the failing GitHub Actions logs before changing YAML

## Security Defaults

- Set least-privilege `permissions`; use `contents: read` as the default baseline unless the job needs more.
- Avoid long-lived cloud secrets; prefer OIDC with cloud-side trust conditions.
- Treat PR titles, branch names, issue bodies, commit messages, and other event payload data as untrusted input.
- Avoid `pull_request_target` unless the workflow is intentionally designed for untrusted forks and does not check out or run attacker-controlled code with elevated token/secrets.
- Pin third-party actions to a full commit SHA for hardened workflows. For GitHub-owned actions, major-version pins can be acceptable for routine CI, but security-sensitive workflows should still evaluate SHA pinning.
- Never print secrets or transformed secrets. Do not assume masking catches every derived value.
- Use artifact attestations for release binaries, packages, and container images when provenance matters.

## Design Output Contract

When creating or rewriting a workflow, return:

- the workflow file path
- triggers and why each one is present
- job graph and dependencies
- required permissions by job
- cache/artifact strategy
- secret/OIDC/environment assumptions
- validation commands that were run or should be run
- official docs consulted when behavior is non-obvious

## Audit Output Contract

Lead with findings ordered by severity. For each finding include:

- file and line when available
- risk or failure mode
- specific GitHub Actions rule or documentation basis
- minimal fix
- validation step

If no issues are found, state the remaining residual risk, especially untested GitHub-side behavior or unavailable logs.

## Spec Output Contract

For workflow specifications, describe behavior rather than YAML trivia:

- purpose and trigger contract
- job dependency matrix
- input/output artifacts and outputs
- permissions, secrets, environments, and OIDC trust assumptions
- cache invalidation and artifact retention expectations
- publish/deploy gates, rollback assumptions, and notification hooks
- failure handling and rerun expectations
- quality gates and observability

## Reference Notes

This skill was grounded on official GitHub Docs and selected unique ecosystem patterns on 2026-05-22. Re-check official docs for exact current limits, runner images, action versions, and plan-gated features before high-stakes changes.
