# Documentation Operation Protocol

Purpose: define the canonical agent instruction for working with VIDA documentation using only the currently green documentation layers and without depending on unfinished inventory, relation, readiness, or runtime-consumption behavior.

## Scope

This protocol applies when an agent:

1. reads or updates canonical markdown documentation,
2. changes footer metadata or sidecar changelog state,
3. needs a low-call operational path for documentation work,
4. must keep documentation law aligned before or during implementation work.

This protocol is limited to the currently documentation-green layers:

1. `Layer 1: Canonical Schema`
2. `Layer 3: Canonical Validation`
3. `Layer 4: Canonical Mutation`
4. `Layer 6: Canonical Operator`

It must not assume that later layers are closed.

## Documentation-First Rule

1. When a new documentation-facing layer or rule changes, bring the canonical documentation into shape first.
2. Only after the documentation law is explicit may implementation or tooling behavior be changed.
3. If implementation behavior and documentation law diverge, fix the documentation first or realign the implementation before closure.

## Allowed Foundations

Use only the following canonical foundations:

1. `docs/product/spec/project-documentation-system.md`
2. `docs/product/spec/canonical-documentation-and-inventory-layers.md`
3. `vida/config/instructions/system-maps.framework-map-protocol.md`
4. `AGENTS.md`
5. `AGENTS.sidecar.md`
6. `codex-v0/codex.py`

## Required Working Mode

When working on documentation, the agent must operate in this order:

1. read the relevant canonical docs first,
2. use operator views to understand current state,
3. make bounded edits,
4. finalize metadata/changelog lawfully,
5. run validation before closure.

## Low-Call Operator Path

Default documentation orientation path:

1. `python3 codex-v0/codex.py overview --profile active-canon`
2. targeted reads of the canonical documents being changed
3. `python3 codex-v0/codex.py check --root <dir> [files...]`
4. `python3 codex-v0/codex.py doctor --profile active-canon-strict`

Use richer history or status views only when needed:

1. `changelog`
2. `task-summary`
3. `summary`

## Lawful Mutation Path

If one documentation change is a single logical edit:

1. edit the file,
2. run one lawful finalization step,
3. validate.

If one documentation change requires multiple diff operations:

1. perform the diff edits first,
2. run exactly one `finalize-edit` afterward,
3. do not create one changelog event per low-level diff step.

## Required Metadata and Lineage Rule

For canonical markdown artifacts:

1. keep the canonical markdown body as the latest active revision only,
2. keep lineage in sibling `*.changelog.jsonl`,
3. keep footer metadata machine-readable,
4. prefer lawful codex mutation paths over manual footer/changelog manipulation.

## Validation Rule

Before closure of documentation work:

1. run `check` on the changed scope or changed files,
2. run `doctor --profile active-canon-strict` when the change affects canonical docs or maps,
3. treat validation failure as blocking,
4. keep success output quiet and failure output explicit.

## Forbidden Dependencies

This protocol must not depend on:

1. unfinished canonical inventory closure as an authority source,
2. relation or impact graph completion,
3. runtime-readiness verdicts,
4. runtime-owned latest resolution,
5. ad hoc filesystem assumptions outside current canonical docs and codex policy.

## Forbidden Behaviors

1. Do not invent undocumented documentation workflows.
2. Do not mutate footer metadata or sidecar history by hand when a lawful codex path exists.
3. Do not justify documentation changes by future-layer intent alone.
4. Do not treat partial inventory or relation features as if they were already canonical authority.
5. Do not close documentation work without validation.

## Closure Rule

Documentation work is closed only when:

1. the canonical documentation reflects the intended rule or structure,
2. metadata and sidecar lineage are synchronized,
3. `check` passes,
4. `doctor --profile active-canon-strict` passes when the change touches canonical maps, specs, or active instruction canon.

## Current Boundary Note

This protocol is intentionally bounded to the currently green documentation layers.

That means:

1. it is already safe to use for real documentation work now,
2. it does not wait for full inventory-law promotion, relation-law promotion, or runtime-readiness closure,
3. those later layers may deepen this protocol later, but must not redefine the current lawful path retroactively.

-----
artifact_path: config/instructions/instruction-contracts/documentation-operation.protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts.documentation-operation-protocol.md
created_at: '2026-03-10T04:10:00+02:00'
updated_at: '2026-03-10T04:10:00+02:00'
changelog_ref: instruction-contracts.documentation-operation-protocol.changelog.jsonl
