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

-----
artifact_path: product/spec/readme
artifact_type: product_spec
artifact_version: '1'
artifact_revision: 2026-04-05
schema_version: '1'
status: canonical
source_path: docs/product/spec/README.md
created_at: '2026-04-04T20:24:09+03:00'
updated_at: 2026-04-05T05:20:01.303332154Z
changelog_ref: README.changelog.jsonl
