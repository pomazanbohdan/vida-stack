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

- `docs/product/spec/<feature>-design.md` for committed feature/change designs
- `docs/research/<topic>.md` for exploratory research before design closure

Active design docs:

- `docs/product/spec/clarify-spec-scope-design.md`
- `docs/product/spec/feature-specification-design.md`
- `docs/product/spec/flappy-bird-design.md`
- `docs/product/spec/api-constraints-specification-make-bounded-patch-design.md`
- `docs/product/spec/flappy-bird-flappy-bird-every-mechanism-design.md`
- `docs/product/spec/spec-proof-auto-flow-design.md`
- `docs/product/spec/release-1-shared-operator-envelope-closure-design.md`
- `docs/product/spec/clarify-enforce-immediate-project-agent-first-design.md`
- `docs/product/spec/clarify-enforce-immediate-continuation-shell-saf-design.md`
- `docs/product/spec/fix-release-admission-evidence-detection-artifac-design.md`
- `docs/product/spec/continuation-binding-fail-closed-hardening-design.md`
- `docs/product/spec/continuation-and-seeded-dispatch-bridge-design.md`
- `docs/product/spec/lawful-closure-continuation-rebinding-design.md`

Current promoted runtime-control specs:

- `docs/product/spec/autonomous-report-continuation-law.md`
- `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`

-----
artifact_path: product/spec/readme
artifact_type: product_spec
artifact_version: '1'
artifact_revision: 2026-04-05
schema_version: '1'
status: canonical
source_path: docs/product/spec/README.md
created_at: '2026-04-04T20:24:09+03:00'
updated_at: 2026-04-05T06:19:10.127047794Z
changelog_ref: README.changelog.jsonl
- `docs/product/spec/mempalace-against-vida-specs-project-design-design.md`
- `docs/product/spec/config-driven-host-system-runtime-keep-design.md`
- `docs/product/spec/specify-config-driven-host-system-inventory-design.md`
- `docs/product/spec/spec-compliant-exception-path-takeover-surface-design.md`
- `docs/product/spec/explicit-policy-selected-internal-backend-execut-design.md`
- `docs/product/spec/hybrid-host-executor-semantics-host-environment-design.md`
- `docs/product/spec/continue-post-main-carveout-with-next-design.md`
- `docs/product/spec/fix-continuation-reconciliation-resumed-executed-design.md`
- `docs/product/spec/continue-post-main-carveout-ready-view-design.md`
