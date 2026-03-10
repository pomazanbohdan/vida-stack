# Documentation Operation Protocol

Purpose: define the canonical agent instruction for working with VIDA documentation using only the currently green documentation layers and without depending on unfinished runtime-consumption behavior.

## Scope

This protocol applies when an agent:

1. reads or updates canonical markdown documentation,
2. changes footer metadata or sidecar changelog state,
3. needs a low-call operational path for documentation work,
4. must keep documentation law aligned before or during implementation work.

This protocol is limited to the currently documentation-green layers:

1. `Layer 1: Canonical Schema`
2. `Layer 2: Canonical Inventory`
3. `Layer 3: Canonical Validation`
4. `Layer 4: Canonical Mutation`
5. `Layer 5: Canonical Relations`
6. `Layer 6: Canonical Operator`
7. `Layer 7: Canonical Runtime Readiness`

It must not assume that later layers are closed.

If the task is not one bounded documentation operation but a project-wide migration of another project's documentation system toward Layer 7 closure, escalate to:

1. `vida/config/instructions/instruction-contracts.documentation-layer7-migration-protocol.md`

## Documentation-First Rule

1. When a new documentation-facing layer or rule changes, bring the canonical documentation into shape first.
2. Only after the documentation law is explicit may implementation or tooling behavior be changed.
3. If implementation behavior and documentation law diverge, fix the documentation first or realign the implementation before closure.

## Documentation Standard Precedence Rule

When writing or reshaping documentation artifacts, use the highest-authority formatting and structure rule that applies.

Precedence order:

1. an active skill-specific artifact standard when the current task explicitly uses a skill and that skill defines the format of the artifact being written,
2. an explicit project-owned documentation standard for that artifact family when such a standard is already documented,
3. promoted product-law requirements for canonical documentation and instruction artifacts,
4. the bounded default formatting and mutation behavior provided by `codex-v0/codex.py`.

Rules:

1. `codex` defaults are fallback behavior only; they must not override an explicit project standard.
2. A skill-specific artifact contract may refine the shape of the document, but it must not weaken canonical metadata, lineage, validation, or deduplication law.
3. If a project standard and a skill-specific format conflict materially, use the higher-precedence skill format for the artifact body and preserve canonical metadata/footer/sidecar law around it.
4. If no higher-precedence artifact standard exists, use the canonical `codex` documentation path.

Example rule:

1. if a future business-analysis skill defines the canonical `PBI` document shape, use that `PBI` structure for the body of the document while still preserving the canonical metadata, changelog, and validation requirements from this protocol.

## One-Touch Activation Rule

1. Documentation context activates this protocol immediately in one touch.
2. The orchestrator or worker must not wait for a second manual selection step once the active task is clearly about documentation, instruction canon, sidecar lineage, canonical maps, or documentation-layer tooling.
3. As soon as the task context is documentation-shaped, this protocol becomes active authority together with `vida/config/instructions/instruction-contracts.instruction-activation-protocol.md`.
4. Presence in the protocol index is discovery evidence only; activation still comes from the context trigger defined in the activation protocol.

## Allowed Foundations

Use only the following canonical foundations:

1. `docs/product/spec/project-documentation-system.md`
2. `docs/product/spec/canonical-documentation-and-inventory-layers.md`
3. `vida/config/instructions/system-maps.framework-map-protocol.md`
4. `AGENTS.md`
5. `AGENTS.sidecar.md`
6. `codex-v0/codex.py`

Evidence rule:

1. For documentation ownership, bootstrap, canonical-map, or documentation-model analysis, use canonical instruction/spec/map artifacts as the primary authority.
2. Do not use operational artifacts such as `*.changelog.jsonl`, `*.current.jsonl`, registry snapshots, readiness snapshots, or other generated status files as the primary basis for conclusions about documentation ownership or canonical policy.
3. Those operational artifacts may be used only as secondary corroboration after the conclusion is already grounded in canonical documentation.

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
2. `python3 codex-v0/codex.py layer-status --layer <N>` when the work is bounded to one canonical layer
3. `python3 codex-v0/codex.py doctor --layer <N>` when a bounded layer audit is needed
4. targeted reads of the canonical documents being changed
5. `python3 codex-v0/codex.py proofcheck --layer <N>` for bounded one-layer closure or `python3 codex-v0/codex.py proofcheck --profile active-canon-strict` for cross-layer closure
6. targeted deep reads only when the proof surface reports a blocker

Use richer history or status views only when needed:

1. `changelog`
2. `task-summary`
3. `summary`
4. `deps`
5. `deps-map`
6. `artifact-impact`
7. `task-impact`
8. `activation-check`
9. `protocol-coverage-check`
10. `readiness-check`
11. `readiness-write`

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
4. prefer lawful codex mutation paths over manual footer/changelog manipulation,
5. do not create parallel active documents that restate the same canonical rule, matrix, or artifact law when one canonical home already exists.

## Deduplication Rule

1. One canonical rule should have one canonical home.
2. If a higher-precedence artifact standard applies, align the existing canonical artifact to that standard instead of cloning the rule into a second active document.
3. Summaries, maps, and pointers may restate orientation-level information, but they must not become a second law-bearing source of truth.
4. If a documentation task reveals duplicated active law, the bounded task should reduce or remove that duplication when it is safe to do so within scope.

## Validation Rule

Before closure of documentation work:

1. run `check` on the changed scope or changed files,
2. run `activation-check` when the change touches a canonical protocol, protocol index row, lane-entry routing, or activation wiring,
3. run `protocol-coverage-check --profile active-canon` when the change touches canonical protocol inventory, protocol index rows, or protocol-bearing instruction artifacts,
4. run `readiness-check --profile active-canon` when the change touches readiness law, projection parity, canonical bundles, compatibility classes, or boot-gate surfaces,
5. run `doctor --profile active-canon-strict` when the change affects canonical docs or maps,
6. prefer `proofcheck --layer <N>` when the changed scope is tightly bounded to one canonical layer,
7. prefer `proofcheck --profile active-canon-strict` when the changed scope spans multiple active-canon layers,
8. treat validation failure as blocking,
9. keep success output quiet and failure output explicit.

## Protocol Activation Verification Rule

1. A protocol does not become active merely because the file exists, because it appears in the protocol index, or because another document references it.
2. Treat protocol activation as a rule-evaluation problem:
   - determine the activation class and trigger from `vida/config/instructions/instruction-contracts.instruction-activation-protocol.md`,
   - verify that the current lane, phase, route, or artifact flow satisfies that trigger,
   - only then treat the protocol as active authority for the current task.
3. When documentation work changes a canonical protocol or its routing/index wiring, verify the activation rule in the same work cycle.
4. If a documentation-context protocol lacks a valid activation binding, treat that as a bounded green-layer gap and correct it immediately when safe.
5. Use `python3 codex-v0/codex.py activation-check --root <dir> [files...]` as the bounded operational proof for activation coverage during documentation work.
6. Use `python3 codex-v0/codex.py protocol-coverage-check --root <dir> [files...]` as the bounded operational proof that canonical protocol-bearing artifacts are present in the protocol index and still have valid activation coverage.

## Immediate Gap Correction Rule

When documentation work or documentation-layer validation reveals a bounded defect inside an already-green layer:

1. correct the defect immediately in the same work cycle when the fix is safe and scope-bounded,
2. do not defer a green-layer usability, validation, mutation, or operator-surface defect merely because the main task can continue,
3. defer only when the required fix would materially widen scope beyond the active documentation task.

## Forbidden Dependencies

This protocol must not depend on:

1. runtime-owned latest resolution,
2. ad hoc filesystem assumptions outside current canonical docs and codex policy.

## Forbidden Behaviors

1. Do not invent undocumented documentation workflows.
2. Do not mutate footer metadata or sidecar history by hand when a lawful codex path exists.
3. Do not justify documentation changes by future-layer intent alone.
4. Do not treat partial inventory or relation features as if they were already canonical authority.
5. Do not close documentation work without validation.
6. Do not restyle or reframe the repository `README.md` voice, presentation style, or narrative shape unless the user explicitly asks for a README stylistic rewrite; bounded factual updates are allowed, but stylistic transformation is forbidden by default.

## Closure Rule

Documentation work is closed only when:

1. the canonical documentation reflects the intended rule or structure,
2. metadata and sidecar lineage are synchronized,
3. `check` passes,
4. `activation-check` passes when protocol activation coverage changed,
5. `protocol-coverage-check` passes when canonical protocol coverage changed,
6. `readiness-check` passes when readiness surfaces changed,
7. `doctor --profile active-canon-strict` passes when the change touches canonical maps, specs, or active instruction canon,
8. `proofcheck --layer <N>` may be used as the one-command bounded closure proof for one canonical layer,
9. `proofcheck --profile active-canon-strict` may be used as the one-command bounded closure proof for cross-layer active-canon work.

## Current Boundary Note

This protocol is intentionally bounded to the currently green documentation layers.

That means:

1. it is already safe to use for real documentation work now,
2. it already includes canonical runtime-readiness work when that work is bounded by promoted Layer 7 law,
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
updated_at: '2026-03-10T04:25:26+02:00'
changelog_ref: instruction-contracts.documentation-operation-protocol.changelog.jsonl
