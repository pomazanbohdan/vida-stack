# GitHub Actions Workflow Patterns

These patterns synthesize unique useful ideas from current GitHub Docs and public skill ecosystem results: docs-first answers, production templates, specification-first workflow design, and CI/CD security gates.

## CI/CD Stage Taxonomy

Use this taxonomy when designing or auditing a pipeline:

| Stage | Purpose | Typical proof |
| --- | --- | --- |
| Build | Produce reproducible binaries, packages, images, or static assets | lockfile install, compiler/build command, artifact upload |
| Test | Catch regressions before merge or deploy | unit, integration, end-to-end, compatibility matrix |
| Security | Detect vulnerable code, dependencies, secrets, or containers | CodeQL, third-party SARIF, dependency review, container scan |
| Package | Build once and promote the same artifact | artifact download, image digest, SBOM/provenance |
| Deploy staging | Exercise deployment automation safely | environment-scoped secrets, smoke test, deployment record |
| Deploy production | Release with explicit gates | protected environment, approval, concurrency, rollback note |

Prefer "build once, deploy many" for release flows: publish or deploy the exact artifact that passed tests, rather than rebuilding separately in each environment.

## Design Checklist

1. State the workflow purpose in one sentence.
2. Choose the smallest trigger set:
   - `pull_request` for PR validation.
   - `push` to protected branches for merge validation.
   - tags or releases for publishing.
   - `workflow_dispatch` for operator-controlled runs.
   - `workflow_call` for reusable workflows.
3. Add branch/path filters only when skipped required checks will not block merges.
4. Add `permissions` explicitly.
5. Use `concurrency` for PR validation and deployments that should cancel or serialize.
6. Split jobs by trust boundary:
   - untrusted code build/test
   - privileged publishing
   - deployment with environment approval
   - provenance/attestation
7. Define cache keys from lockfiles and tool versions.
8. Upload artifacts for cross-job handoff or debugging; do not use artifacts as a dependency cache.
9. Add local validation commands to the workflow comments or PR notes, not as noisy YAML comments unless they help maintainers.

## Minimal PR CI

Use for fast validation on every PR. Adapt tool commands to the repository.

```yaml
name: CI

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]

permissions:
  contents: read

concurrency:
  group: ci-${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

jobs:
  test:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v6
      - name: Install dependencies
        run: npm ci
      - name: Lint
        run: npm run lint
      - name: Test
        run: npm test
```

Validation notes:

- Verify the current major version for setup actions before committing.
- Add tool setup actions only when the hosted runner image does not already provide the required tool version.
- Keep job permissions at `contents: read` unless the job writes checks, packages, attestations, pages, issues, or deployments.

## Matrix CI

Use when the project must prove compatibility across versions or operating systems.

```yaml
strategy:
  fail-fast: false
  max-parallel: 4
  matrix:
    os: [ubuntu-latest, windows-latest]
    node-version: [20, 22]
    include:
      - os: ubuntu-latest
        node-version: 24
        experimental: true

continue-on-error: ${{ matrix.experimental == true }}
runs-on: ${{ matrix.os }}
```

Avoid broad matrices when one smoke job plus a smaller compatibility matrix would prove the same risk.

## Reusable Workflow

Reusable workflow:

```yaml
name: Reusable Test

on:
  workflow_call:
    inputs:
      node-version:
        required: true
        type: string

permissions:
  contents: read

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: actions/setup-node@v6
        with:
          node-version: ${{ inputs.node-version }}
          cache: npm
      - run: npm ci
      - run: npm test
```

Caller:

```yaml
jobs:
  test:
    uses: ./.github/workflows/reusable-test.yml
    with:
      node-version: "22"
    permissions:
      contents: read
```

Rules:

- Call reusable workflows at the job level, not within steps.
- Keep called workflows directly under `.github/workflows`.
- Use a commit SHA for cross-repository reusable workflows when stability and security matter.
- Do not assume a called workflow can elevate token permissions beyond the caller.

## Secure Publish With OIDC And Attestation

Use for release assets, packages, or container images.

Required design properties:

- publish only from protected branches, tags, or releases
- use an environment for deployment approvals when humans must gate production
- set `id-token: write` only on the publish/provenance job
- constrain cloud trust policy to repository, ref, and environment claims
- generate provenance with GitHub artifact attestations when consumers need build integrity
- upload or publish the exact asset that was attested

Skeleton:

```yaml
permissions:
  contents: read

jobs:
  build:
    runs-on: ubuntu-latest
    permissions:
      contents: read
    steps:
      - uses: actions/checkout@v4
      - run: ./scripts/build-release.sh
      - uses: actions/upload-artifact@v4
        with:
          name: release-assets
          path: dist/

  publish:
    needs: build
    runs-on: ubuntu-latest
    environment: production
    permissions:
      contents: read
      id-token: write
      attestations: write
    steps:
      - uses: actions/download-artifact@v4
        with:
          name: release-assets
          path: dist/
      - name: Publish
        run: ./scripts/publish-release.sh dist/
      - name: Attest build provenance
        uses: actions/attest-build-provenance@v3
        with:
          subject-path: "dist/*"
```

Before using this skeleton, verify the current action major versions and plan availability for attestations.

## Container Image Publish

Use when the workflow builds and pushes images to GHCR, Docker Hub, or a cloud registry.

```yaml
name: Publish Image

on:
  push:
    tags: ["v*"]

permissions:
  contents: read
  packages: write
  id-token: write
  attestations: write

env:
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository }}

jobs:
  image:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: docker/login-action@v4
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - id: meta
        uses: docker/metadata-action@v6
        with:
          images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
      - id: build
        uses: docker/build-push-action@v7
        with:
          context: .
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max
      - uses: actions/attest-build-provenance@v3
        with:
          subject-name: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
          subject-digest: ${{ steps.build.outputs.digest }}
          push-to-registry: true
```

Before shipping this pattern, verify the build step exposes the digest output under the selected action version. If not, bind the build step with `id: build` and use the documented digest output or attest the built artifact path instead.

## Security Scan And SARIF Upload

Use for third-party scanners that emit SARIF, such as container, dependency, secret, or SAST tools.

```yaml
name: Security Scan

on:
  pull_request:
  push:
    branches: [main]
  schedule:
    - cron: "30 3 * * 1"

permissions:
  contents: read
  security-events: write

jobs:
  scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - name: Run scanner
        run: ./scripts/security-scan.sh --format sarif --output results.sarif
      - uses: github/codeql-action/upload-sarif@v4
        with:
          sarif_file: results.sarif
```

Rules:

- Keep scanner setup in a script when it is reused locally or has nontrivial flags.
- Upload only valid SARIF; failing uploads usually need SARIF validation, file-size reduction, feature availability checks, or CodeQL setup conflict checks.
- Mark security scanners `allow_failure: false` unless the team has an explicit temporary exception.
- Split secret scanning, dependency scanning, SAST, DAST, and container scanning when they have different runtime, permissions, or failure policy.

## Kubernetes Or Deployment Target

Use this only after deciding where credentials come from and how production is gated.

```yaml
jobs:
  deploy:
    runs-on: ubuntu-latest
    environment:
      name: production
      url: ${{ vars.PRODUCTION_URL }}
    concurrency:
      group: deploy-production
      cancel-in-progress: false
    permissions:
      contents: read
      id-token: write
    steps:
      - uses: actions/checkout@v6
      - name: Authenticate
        run: ./scripts/authenticate-deploy-target.sh
      - name: Deploy
        run: ./scripts/deploy.sh production
      - name: Smoke test
        run: ./scripts/smoke-test.sh "${{ vars.PRODUCTION_URL }}"
```

Rules:

- Prefer OIDC over stored cloud keys.
- Use GitHub environments for production approvals and environment-scoped secrets.
- Serialize production deploys with `concurrency`.
- Keep deployment commands in scripts so rollback and local rehearsal are possible.
- Record deployment URLs and smoke-test evidence when the environment is user-facing.

## Failure Notifications

Add notification hooks only after the workflow has useful failure messages. Prefer GitHub-native annotations, job summaries, check runs, or issues before adding chat notifications. If using Slack, Teams, PagerDuty, or another service, keep tokens in environment-scoped secrets and notify only on actionable failures.

```yaml
- name: Notify failure
  if: failure()
  run: ./scripts/notify-ci-failure.sh
```

## Workflow Specification Template

Use this for `spec` mode or when a workflow is important enough to maintain as product infrastructure.

| Section | Required content |
| --- | --- |
| Purpose | One sentence describing the workflow outcome |
| Trigger contract | Events, branch/tag/path filters, manual inputs, schedule |
| Job graph | Jobs, `needs`, parallelism, matrix axes |
| Permissions | Workflow/job permissions and why each write scope exists |
| Inputs and outputs | `workflow_call` inputs, job outputs, artifacts, packages, images |
| Secrets and trust | Secrets, variables, environments, OIDC claims, untrusted inputs |
| Caching | Cache keys, restore keys, invalidation, non-cacheable paths |
| Artifacts | Names, contents, retention, provenance/attestation requirements |
| Quality gates | Required checks, scanners, tests, deployment approvals |
| Failure handling | Retry/rerun policy, cancellation, rollback, notification |
| Operations | Logs, debug mode, dashboards, owners, change-management notes |

## Audit Checklist

Correctness:

- Workflow files are in `.github/workflows` and use `.yml` or `.yaml`.
- Triggers match the intended source refs and events.
- Branch/path filters do not leave required checks permanently pending.
- `needs` expresses the actual job dependency graph.
- Outputs use `GITHUB_OUTPUT`, not deprecated command forms.
- Matrix settings are bounded with `fail-fast`, `continue-on-error`, and `max-parallel` where appropriate.

Security:

- `permissions` is explicit and least-privilege.
- Jobs that do not need secrets cannot access secrets.
- Third-party SARIF upload jobs use `security-events: write` and no broader token permissions than needed.
- `pull_request_target` is absent or justified with a safe design.
- Untrusted context values are not interpolated directly into shell scripts.
- Third-party actions are reviewed and pinned appropriately.
- OIDC trust policy is scoped; `id-token: write` is not global unless every job needs it.
- Self-hosted runners are not used for untrusted fork code unless isolated and ephemeral.
- Release jobs use environments, protected branches/tags, and attestations where needed.
- Production deploys have environment approvals or documented automated protection rules.

Performance and cost:

- Dependency caches are keyed by lockfiles and tool versions.
- Artifacts have intentional retention.
- Long shell scripts are moved into repository scripts for local reproduction.
- Concurrency cancels superseded PR runs and serializes deploys.
- Matrix size is proportional to risk.
- Container builds use registry/build caches intentionally and do not cache secrets.

Maintainability:

- Repeated CI logic is moved into reusable workflows.
- Workflow names and job names are stable enough for branch protection rules.
- Required checks are not renamed casually.
- Workflow failure messages point to local scripts or logs a maintainer can reproduce.
- Important workflows have a short spec covering trigger, permissions, artifacts, and gates.

## Common Failure Modes

- YAML parses but GitHub rejects an expression because the context is unavailable at that key.
- A skipped workflow remains pending because it is required by branch protection.
- `pull_request_target` checks out untrusted code and runs it with elevated token or secrets.
- A called reusable workflow expects permissions that the caller did not grant.
- Cache restore keys are too broad and restore stale or incompatible dependencies.
- A job writes to packages, pages, deployments, checks, or attestations without matching `permissions`.
- A shell command embeds PR title/body/branch data and becomes injection-prone.
- A self-hosted runner preserves workspace state or credentials between jobs.
