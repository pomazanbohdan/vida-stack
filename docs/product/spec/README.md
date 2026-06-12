# Product Spec Guide

Use this directory for bounded product-facing feature/change design documents and linked ADRs.

Top-level execution model:

1. `agent` is execution carrier (model/tier/cost/effectiveness), not runtime role identity.
2. `role` remains explicit runtime activation state.
3. Runtime binds admissible carrier to role/task-class and selects by capability/admissibility -> score guard -> cheapest eligible.

Default rule:

1. If a request asks for research, detailed specifications, implementation planning, and then code, create or update one bounded design document before implementation.
2. Start from the local template at `docs/product/spec/templates/feature-design-document.template.md`.
3. Open one feature epic and one spec-pack task in `vida taskflow` before normal implementation work begins.
4. Use `vida docflow init`, `vida docflow finalize-edit`, and `vida docflow check` to keep the document canonical.
5. Close the spec-pack task only after the design artifact is finalized and validated, then hand off through the next TaskFlow packet.
6. When one major decision needs durable standalone recording, add a linked ADR instead of overloading the design document.

Suggested homes:

- `docs/product/spec/<feature>-contract.md` or `docs/product/spec/<feature>-spec.md` for committed product/runtime contracts and specifications
- `docs/product/research/<topic>-survey.md` for exploratory research before design closure

Active design docs:

- `docs/product/spec/project-agent-first-delegation-contract.md`
- `docs/product/spec/release-admission-evidence-detection-contract.md`
- `docs/product/spec/continuation-binding-fail-closed-contract.md`
- `docs/product/spec/continuation-seeded-dispatch-bridge-contract.md`
- `docs/product/spec/lawful-closure-continuation-rebinding-contract.md`
- `docs/product/spec/authoritative-state-lock-recovery-contract.md`
- `docs/product/spec/taskflow-execution-semantics-scheduler-contract.md`
- `docs/product/spec/specification-lane-scope-hardening-contract.md`
- `docs/product/spec/fail-closed-resume-closure-truth-contract.md`
- `docs/product/spec/selector-precedence-bounded-repair-contract.md`
- `docs/product/spec/retrieval-identity-memory-governance-contract.md`
- `docs/product/spec/lane-supersede-shared-truth-envelope-contract.md`
- `docs/product/spec/implementation-backend-admissibility-selection-truth-contract.md`
- `docs/product/spec/unified-hybrid-runtime-selection-policy-contract.md`
- `docs/product/spec/taskflow-happy-path-test-catalog-contract.md`
- `docs/product/spec/runtime-web-restart-current-repo-command-contract.md`
- `docs/product/spec/spec-compliant-exception-path-takeover-surface-contract.md`
- `docs/product/spec/dead-code-removal-admission-contract.md`

Current promoted runtime-control specs:

- `docs/product/spec/agent-role-skill-profile-flow-model.md`
- `docs/product/spec/compiled-runtime-bundle-contract.md`
- `docs/product/spec/autonomous-report-continuation-law.md`
- `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
- `docs/product/spec/checkpoint-commit-and-replay-model.md`
- `docs/product/spec/multi-orchestrator-session-ownership-claims-contract.md`
- `docs/product/spec/config-driven-host-system-runtime-contract.md`
- `docs/product/spec/internal-backend-executor-route-policy-contract.md`
- `docs/product/spec/hybrid-host-executor-semantics-model.md`
- `docs/product/spec/codex-host-agent-boundary-and-cli-bridge-contract.md`
- `docs/product/spec/host-agent-bridge-adapter-contract.md`
- `docs/product/spec/carrier-model-profile-selection-runtime-model.md`
- `docs/product/spec/test-first-runtime-defect-remediation-model.md`
- `docs/product/spec/agent-mode-test-first-delivery-flow-model.md`
- `docs/product/spec/fast-high-signal-pre-commit-contract.md`
- `docs/product/spec/mempalace-vida-memory-implementation-model.md`
- `docs/product/spec/vida-coder-service-mode-executor-contract.md`
- `docs/product/spec/codex-app-agent-lifecycle-cleanup-contract.md`
- `docs/product/spec/model-provider-price-catalog-lifecycle-contract.md`
- `docs/product/spec/operator-output-envelope-and-bounded-rendering-contract.md`
- `docs/product/spec/production-observability-and-operator-baselines-contract.md`
- `docs/product/spec/prompt-lifecycle-evaluation-and-safety-baseline-contract.md`

-----
artifact_path: product/spec/readme
artifact_type: product_spec
artifact_version: '1'
artifact_revision: 2026-04-13
schema_version: '1'
status: canonical
source_path: docs/product/spec/README.md
created_at: '2026-04-04T20:24:09+03:00'
updated_at: 2026-06-04T01:02:48.1588214Z
changelog_ref: README.changelog.jsonl
