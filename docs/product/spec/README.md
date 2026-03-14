# Product Spec Guide

Use this directory for bounded product-facing feature/change design documents and linked ADRs.

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
