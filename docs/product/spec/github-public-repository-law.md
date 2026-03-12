# VIDA GitHub Public Repository Law

Status: active product law

Purpose: define the canonical GitHub-native repository and community surface required for a VIDA public repository so public discoverability, contribution posture, security reporting, ownership, and release packaging remain explicit without leaking deep product law into root/community carriers.

## Scope

This law governs:

1. GitHub-recognized root/community files,
2. `.github/**` repository-community surfaces,
3. issue and pull request template surfaces,
4. release/tag posture for public distribution,
5. the boundary between public repository entrypoints and deep product canon in `docs/**`.

This law does not govern:

1. framework instruction filenames under `vida/config/**`,
2. deep product-law filenames under `docs/**`,
3. project-specific implementation or release content beyond the repository/public-surface contract.

## Public Repository Core Rule

A VIDA public repository must stay GitHub-native at the outer surface.

That means:

1. GitHub-recognized files keep their standard names,
2. community/discoverability files stay in GitHub-recognized locations,
3. deep product law continues to live in `docs/**`,
4. root/community files act as entrypoints, guidance, ownership, and disclosure surfaces rather than as the full owner of deep product canon.

## GitHub Location Precedence Rule

GitHub recognizes community-health and related repository files in this order:

1. `.github/`
2. repository root
3. `docs/`

Rules:

1. repository-specific files may be placed in any GitHub-recognized location that the platform supports,
2. if the same file exists in more than one recognized location, GitHub uses the higher-precedence location,
3. repository-specific files override organization/account defaults provided by a public `.github` repository,
4. repository policy must not invent alternate filenames for GitHub-recognized community surfaces.

## Required Public Repository Surfaces

### Repository Entry And Legal Surface

Required:

1. `README.md`
2. one repository license file at root

Rule:

1. `README.md` is the primary public entrypoint,
2. the license file must remain repository-local; organization defaults do not replace it.

### Contributor And Community Surface

Required:

1. `CONTRIBUTING.md`
2. `CODE_OF_CONDUCT.md`
3. `SUPPORT.md`

Recommended:

1. `GOVERNANCE.md` when project governance is non-trivial,
2. `FUNDING.yml` when sponsor visibility is desired.

### Security Surface

Required:

1. `SECURITY.md`

Recommended GitHub security baseline for public repositories:

1. Dependabot alerts
2. secret scanning
3. push protection
4. code scanning
5. private vulnerability reporting

### Ownership And Review Surface

Required:

1. `CODEOWNERS`

Rules:

1. `CODEOWNERS` may live in `.github/`, root, or `docs/`,
2. the active file is selected by GitHub in that order,
3. the file must exist on the base branch for review requests to work,
4. listed users or teams must have write access,
5. repository-specific ownership rules should prefer `.github/CODEOWNERS` unless a root placement is explicitly justified.

### Intake Surface

Required:

1. issue templates or issue forms

Conditionally required:

1. one pull request template, but only when the repository accepts public pull requests as an active contribution path

Rules:

1. issue templates belong under `.github/ISSUE_TEMPLATE/**`,
2. repository-local issue templates override organization defaults when the repository carries its own `.github/ISSUE_TEMPLATE` content,
3. if public PR intake is disabled by repository policy, the repository may omit a pull request template until that intake path is enabled,
4. when PR intake becomes active, pull request templates may live in `.github/`, root, or `docs/`, but repository policy should prefer `.github/` for stable public workflow entry.

### Release Surface

Required when the repository distributes runnable software or binaries:

1. Git tags
2. GitHub releases

Rules:

1. releases package software for public consumption,
2. releases must stay anchored to tags,
3. binary files, release notes, and public distribution links belong in the release surface rather than being scattered across deep docs.

## Repository-Owned Placement Rule

For this project family, prefer:

1. root for:
   - `README.md`
   - license file
   - `CONTRIBUTING.md`
   - `CODE_OF_CONDUCT.md`
   - `SECURITY.md`
   - `SUPPORT.md`
2. `.github/` for:
   - `CODEOWNERS`
   - issue templates
   - pull request templates
   - workflow/community-health defaults
   - other GitHub-native automation surfaces
3. `docs/**` for:
   - deep product canon
   - process maps
   - research and memory lanes

Fallback rule:

1. if organization-wide defaults are being intentionally inherited, repository-local copies are optional,
2. once the repository needs repo-specific behavior, the repo must carry its own file rather than relying on silent inherited defaults.

## Community Profile Rule

Public repositories should remain community-profile complete enough for contributors to understand:

1. what the project is,
2. how to contribute,
3. how behavior is moderated,
4. how to report security issues,
5. how to get help,
6. who owns which code paths,
7. how public releases are published.

## Boundary Rule

1. root/community files are public-entry and governance carriers,
2. deep product law remains owned by `docs/product/spec/**`,
3. process/runbook material remains owned by `docs/process/**`,
4. GitHub-native repository surfaces must point into deep canon where needed, but they must not become a second full product spec tree.

## External Validation Baseline

This law is grounded in the official GitHub documentation for:

1. README discoverability and precedence
2. repository best practices and security features for public repositories
3. contributor guidelines
4. code of conduct
5. security policy
6. support resources
7. code owners
8. issue and pull request templates
9. releases
10. default community health files

Reference URLs:

1. `https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes`
2. `https://docs.github.com/en/repositories/creating-and-managing-repositories/best-practices-for-repositories`
3. `https://docs.github.com/en/communities/setting-up-your-project-for-healthy-contributions/setting-guidelines-for-repository-contributors`
4. `https://docs.github.com/en/communities/setting-up-your-project-for-healthy-contributions/adding-a-code-of-conduct-to-your-project`
5. `https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/configure-vulnerability-reporting/adding-a-security-policy-to-your-repository`
6. `https://docs.github.com/en/communities/setting-up-your-project-for-healthy-contributions/adding-support-resources-to-your-project`
7. `https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners`
8. `https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/about-issue-and-pull-request-templates`
9. `https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases`
10. `https://docs.github.com/en/communities/setting-up-your-project-for-healthy-contributions/creating-a-default-community-health-file`

-----
artifact_path: product/spec/github-public-repository-law
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/github-public-repository-law.md
created_at: '2026-03-12T09:00:00+02:00'
updated_at: '2026-03-12T08:04:26+02:00'
changelog_ref: github-public-repository-law.changelog.jsonl
