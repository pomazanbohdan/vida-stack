# Official GitHub Actions Docs Map

Validated on 2026-05-22 against `docs.github.com`.

Use this file as a navigation map. Prefer the official page for exact syntax, current limits, plan availability, and examples.

## Core Workflow Model

- Workflows overview: https://docs.github.com/en/actions/concepts/workflows-and-actions/workflows
  - Workflows live under `.github/workflows`.
  - A run is triggered by events, manual dispatch, schedules, or repository dispatch.
  - A workflow has jobs, and each job has steps that run scripts or actions.
- Workflow syntax: https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax
  - Canonical source for `on`, `permissions`, `env`, `defaults`, `concurrency`, `jobs`, `strategy.matrix`, containers, services, and reusable workflow calls.
  - Branch/path filters combine with AND semantics when both are present.
  - Skipped workflows can leave required checks pending.
  - Order matters for positive and negative glob patterns.
- Events that trigger workflows: https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows
  - Check event-specific constraints before using less common triggers.
  - Some events require the workflow file to exist on the default branch.
- Expressions: https://docs.github.com/en/actions/reference/workflows-and-actions/expressions
  - Use expression functions for conditionals and JSON handling.
  - Avoid interpolating untrusted event data into shell commands.
- Contexts: https://docs.github.com/en/actions/reference/workflows-and-actions/contexts
  - Context availability differs by workflow key.
  - Treat event payload fields as untrusted when they can be controlled by users.
- Variables: https://docs.github.com/en/actions/reference/workflows-and-actions/variables
  - Default `GITHUB_*` and `RUNNER_*` variables are runner-provided.
  - Configuration variables differ from environment variables written during a run.
- Workflow commands: https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands
  - Use environment files such as `GITHUB_ENV`, `GITHUB_OUTPUT`, and `GITHUB_PATH` instead of deprecated command forms.

## Reuse And Structure

- Reusable workflow concepts and reference: https://docs.github.com/en/actions/concepts/workflows-and-actions/reusing-workflow-configurations
- Reuse workflows how-to: https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows
  - A reusable workflow must be directly under `.github/workflows`; subdirectories are not supported.
  - It must include `on: workflow_call`.
  - A caller invokes it at the job level with `jobs.<job_id>.uses`, not as a step.
  - Cross-repository reusable workflows should use a SHA, tag, or branch; a commit SHA is safest for stability and security.
  - Caller `GITHUB_TOKEN` permissions can be downgraded by the called workflow, not elevated.
  - Avoid using the same concurrency group in caller and called workflows when `cancel-in-progress` is true.
- Matrix jobs: https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/run-job-variations
  - Use `include`, `exclude`, `fail-fast`, `continue-on-error`, and `max-parallel` intentionally.
  - Avoid accidental combinatorial explosion.

## Data, Caching, And Artifacts

- Dependency caching: https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching
  - Cache dependencies and build intermediates, not final release artifacts.
  - Use restore keys deliberately; broad restore keys can use stale caches.
  - Do not put secrets in cache paths or cache keys.
- Store and share data with workflow artifacts: https://docs.github.com/en/actions/tutorials/store-and-share-data
  - Use artifacts to pass build outputs between jobs or retain debugging evidence.
  - Set retention intentionally for cost and compliance.
- Workflow artifacts concept: https://docs.github.com/en/actions/concepts/workflows-and-actions/workflow-artifacts
  - Artifacts are run-scoped outputs, distinct from dependency caches.
- Publishing Docker images: https://docs.github.com/actions/guides/publishing-docker-images
  - Use when building and pushing container images to GitHub Packages, Docker Hub, or another registry.
  - Check current Docker action versions and permissions before copying examples.

## Runners

- GitHub-hosted runners: https://docs.github.com/en/actions/concepts/runners/github-hosted-runners
  - GitHub provides hosted runner machines with managed images.
  - Verify current labels and images before relying on installed tools.
- Larger runners: https://docs.github.com/en/actions/concepts/runners/larger-runners
  - Use for higher CPU/RAM, static IP needs, GPU, or private networking requirements when plan allows.
- Self-hosted runners: https://docs.github.com/en/actions/concepts/runners/self-hosted-runners
  - Use labels and runner groups to control placement.
  - Treat runner compromise and persistence as part of the threat model.
- Actions Runner Controller: https://docs.github.com/en/actions/concepts/runners/actions-runner-controller
  - Use for Kubernetes-managed ephemeral runner scale sets when operationally justified.

## Security

- Security concepts: https://docs.github.com/en/actions/concepts/security
- Secure use reference: https://docs.github.com/en/actions/reference/security/secure-use
  - Use least privilege for secrets and `GITHUB_TOKEN`.
  - Prefer actions over inline scripts when this reduces shell-injection exposure.
  - Pin third-party actions to a full commit SHA for maximum supply-chain hardening.
  - Audit third-party actions before use.
- `GITHUB_TOKEN`: https://docs.github.com/en/actions/concepts/security/github_token
  - The token is automatically created for each workflow run.
  - Set explicit permissions and avoid relying on repository defaults.
- Secrets: https://docs.github.com/en/actions/concepts/security/secrets
  - Secret masking is not a substitute for avoiding exposure.
  - Prefer environment protection and OIDC for deployments.
- OpenID Connect reference: https://docs.github.com/en/actions/reference/security/oidc
  - Use `id-token: write` only in jobs that request an OIDC token.
  - Constrain cloud trust policies with repository, ref, environment, and other token claims.
  - Use `github/actions-oidc-debugger` only as a temporary diagnostic aid.
- Artifact attestations concept: https://docs.github.com/en/actions/concepts/security/artifact-attestations
- Use artifact attestations: https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations
  - Generate provenance for release binaries, packages, and container images.
  - Attestations use GitHub's OIDC-backed provenance model and Sigstore.
  - Private/internal availability can depend on GitHub plan.
- Script injections: https://docs.github.com/en/actions/concepts/security/script-injections
  - Never place untrusted context values directly inside shell scripts.
- Compromised runners: https://docs.github.com/en/actions/concepts/security/compromised-runners
  - Self-hosted and long-lived runners need isolation, cleanup, and secret exposure controls.
- Upload SARIF to code scanning: https://docs.github.com/en/code-security/code-scanning/integrating-with-code-scanning/uploading-a-sarif-file-to-github
  - Use for third-party SAST, dependency, secret, or container scanners that emit SARIF.
  - Requires `security-events: write`; private/internal repository support can depend on GitHub Code Security availability.
- SARIF upload troubleshooting: https://docs.github.com/en/code-security/reference/code-scanning/sarif-files/troubleshoot-sarif-uploads
  - Check file size, validity, feature availability, and default CodeQL setup conflicts when uploads fail.

## Deployments

- Deployment environments: https://docs.github.com/en/actions/concepts/workflows-and-actions/deployment-environments
  - Use environments for `staging`, `production`, or other deployment targets.
  - Environments can require approval, restrict branches/tags, gate jobs with protection rules, and scope secrets.
- Deployments and environments reference: https://docs.github.com/en/actions/reference/workflows-and-actions/deployments-and-environments
  - Required reviewers, wait timers, branch/tag restrictions, and custom protection rules have plan and repository visibility constraints.
- Reviewing deployments: https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/review-deployments
  - Use when a job waits for required environment review.
- Deploying with GitHub Actions: https://docs.github.com/en/actions/concepts/use-cases/deploying-with-github-actions
  - Use for deployment trigger, environment, concurrency, history, monitoring, and custom protection rule guidance.

## Operations And Troubleshooting

- Monitor workflows: https://docs.github.com/en/actions/how-tos/monitor-workflows
- Workflow run logs: https://docs.github.com/en/actions/how-tos/monitor-workflows/use-workflow-run-logs
- Enable debug logging: https://docs.github.com/en/actions/how-tos/monitor-workflows/enable-debug-logging
- Manage workflow runs: https://docs.github.com/en/actions/how-tos/manage-workflow-runs
- Limits: https://docs.github.com/en/actions/reference/limits

## Docs-First Rule

Browse or open the relevant official page before making exact claims about:

- current runner images and included tools
- workflow syntax edge cases
- event-specific availability
- security-sensitive behavior
- plan-gated features
- current GitHub-owned action major versions
- limits, retention, storage, billing, or concurrency behavior
