# Documentation Operation Protocol

Purpose: define the canonical agent instruction for working with VIDA documentation using only the currently green documentation layers and without depending on unfinished runtime-consumption behavior.

## Scope

This protocol applies when an agent:

1. reads/updates canonical markdown documentation,
2. changes footer metadata or sidecar changelog state,
3. needs a low-call documentation path,
4. must align documentation law before or during implementation work.

Current green scope: `Layer 1: Canonical Schema`, `Layer 2: Canonical Inventory`, `Layer 3: Canonical Validation`, `Layer 4: Canonical Mutation`, `Layer 5: Canonical Relations`, `Layer 6: Canonical Operator`, `Layer 7: Canonical Runtime Readiness`.

It must not assume that later layers are closed.

If the task is a project-wide migration of another project's documentation system toward Layer 7 closure, escalate to `instruction-contracts/work.documentation-layer7-migration-protocol`.

## Documentation-First Rule

1. When a new documentation-facing layer or rule changes, bring the canonical documentation into shape first.
2. Only after the documentation law is explicit may implementation or tooling behavior be changed.
3. If implementation behavior and documentation law diverge, fix the documentation first or realign the implementation before closure.

## Documentation Standard Precedence Rule

When writing or reshaping documentation artifacts, use the highest-authority formatting/structure rule that applies.

Precedence: active skill-specific artifact standard for the explicit skill/artifact -> explicit project-owned artifact-family standard -> promoted product-law requirements for canonical documentation/instruction artifacts -> bounded `vida docflow` default formatting/mutation behavior.

Rules:

1. `DocFlow` fallback defaults are fallback behavior only; they must not override an explicit project standard.
2. A skill-specific artifact contract may refine the shape of the document, but it must not weaken canonical metadata, lineage, validation, or deduplication law.
3. If a project standard and a skill-specific format conflict materially, use the higher-precedence skill format for the artifact body and preserve canonical metadata/footer/sidecar law around it.
4. If no higher-precedence artifact standard exists, use the canonical `DocFlow` documentation path.

Example: if a future business-analysis skill defines canonical `PBI` shape, use that body structure while preserving this protocol's metadata, changelog, and validation requirements.

## One-Touch Activation Rule

1. Documentation context activates this protocol immediately in one touch.
2. The orchestrator or worker must not wait for a second manual selection step once the active task is clearly about documentation, instruction canon, sidecar lineage, canonical maps, or documentation-layer tooling.
3. As soon as the task context is documentation-shaped, this protocol becomes active authority together with `instruction-contracts/bridge.instruction-activation-protocol`.
4. Presence in the protocol index is discovery evidence only; activation still comes from the context trigger defined in the activation protocol.

## Allowed Foundations

Allowed canonical foundations: `docs/product/spec/project-documentation-law.md`, `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`, `system-maps/framework.map`, `AGENTS.md`, `AGENTS.sidecar.md`, `vida docflow`.

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

Default orientation path: `vida docflow overview --profile active-canon` -> `vida docflow layer-status --layer <N>` for one-layer work -> `vida docflow doctor --layer <N>` for bounded layer audit -> targeted reads of changed canonical docs -> `vida docflow proofcheck --layer <N>` for one-layer closure or `vida docflow proofcheck --profile active-canon-strict` for cross-layer closure -> targeted deep reads only when proof reports a blocker.

Use richer views only when needed: `changelog`, `task-summary`, `summary`, `deps`, `deps-map`, `artifact-impact`, `task-impact`, `activation-check`, `protocol-coverage-check`, `readiness-check`, `readiness-write`, `report-check`.

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
4. prefer lawful DocFlow mutation paths over manual footer/changelog manipulation,
5. do not create parallel active documents that restate the same canonical rule, matrix, or artifact law when one canonical home already exists.

## Deduplication Rule

1. One canonical rule should have one canonical home.
2. If a higher-precedence artifact standard applies, align the existing canonical artifact to that standard instead of cloning the rule into a second active document.
3. Summaries, maps, and pointers may restate orientation-level information, but they must not become a second law-bearing source of truth.
4. If a documentation task reveals duplicated active law, the bounded task should reduce or remove that duplication when it is safe to do so within scope.

Thematic consolidation rule:

1. Documentation work should prefer coherent thematic artifacts over fragmented topic shards when the topic is materially one subject.
2. If related information is spread across several weakly connected documents, create or refresh a thematic consolidated artifact in the canonical owner layer.
3. Consolidation must reduce fragmentation without creating a second competing law source.
4. The goal is topic coherence, not document proliferation.

Protocol reuse and promotion rule:

1. Before creating a new canonical protocol, instruction contract, runtime instruction, or framework map, first search the existing canonical protocol/index/map surfaces for an already-owned home of the topic.
2. Minimum lookup path: `system-maps/protocol.index` -> `system-maps/framework.map` -> targeted search across `vida/config/instructions/**` for the active topic.
3. Create a new protocol-bearing artifact only when the topic is clearly a separate domain with its own bounded trigger, owner, and responsibility set.
4. "I want to say this more strongly" is not enough reason to create a new protocol if an existing canonical owner already covers the same topic.
5. When a new protocol is justified, move the related rule body into that new canonical home rather than leaving the same law half-owned across several older artifacts.
6. Maps, indexes, and adjacent protocols may point to the new home, but they must not retain a second competing body of the same law.
7. If the work reveals that an existing protocol already owns the topic but is underspecified, strengthen the existing protocol instead of cloning the topic into a sibling artifact.
8. If the work reveals that one topic is split across several weak protocol fragments without a justified trigger split, consolidate that topic back into one canonical owner when safe and bounded.

## Validation Rule

Before closure of documentation work: run `check` on changed scope/files; run `activation-check` when touching a canonical protocol, protocol index row, lane-entry routing, or activation wiring; run `protocol-coverage-check --profile active-canon` when touching canonical protocol inventory, protocol index rows, or protocol-bearing instruction artifacts; run `readiness-check --profile active-canon` when touching readiness law, projection parity, canonical bundles, compatibility classes, or boot-gate surfaces; run `doctor --profile active-canon-strict` when touching canonical docs/maps; prefer `proofcheck --layer <N>` for one-layer scope; prefer `proofcheck --profile active-canon-strict` for multi-layer active-canon scope; treat validation failure as blocking; keep success output quiet and failure output explicit.

## Map Registration Rule

When documentation work creates, promotes, renames, or materially reroutes a canonical project-visible document surface:

1. update the owning project map or index in the same bounded change,
2. if the change affects bootstrap-visible project documentation topology, update `AGENTS.sidecar.md` in the same bounded change,
3. if the change affects project-level canonical document entrypoints, update `docs/project-root-map.md` in the same bounded change,
4. treat missing map registration as blocking drift rather than optional follow-up cleanup.
5. use `vida docflow check-file`, `check`, `fastcheck`, or `readiness-check` as the operational proof surface for this rule; they must fail closed when required registration or bootstrap-visible sidecar pointers are missing.

## Reporting Prefix Verification Rule

When the proof target is the runtime reporting/log prefix rather than markdown footer law:

1. use `vida docflow report-check --path <file>`,
2. require the first non-empty line to start with `Thinking mode: <STC|PR-CoT|MAR|5-SOL|META>.`,
3. require the second non-empty line to start with `Requests:` or `Tasks:`,
4. require the third non-empty line to start with `Agents:`,
5. treat missing or malformed reporting-prefix shape as a blocking proof failure.

## Protocol Activation Verification Rule

1. A protocol does not become active merely because the file exists, because it appears in the protocol index, or because another document references it.
2. Treat protocol activation as a rule-evaluation problem:
   - determine the activation class and trigger from `instruction-contracts/bridge.instruction-activation-protocol`,
   - verify that the current lane, phase, route, or artifact flow satisfies that trigger,
   - only then treat the protocol as active authority for the current task.
3. When documentation work changes a canonical protocol or its routing/index wiring, verify the activation rule in the same work cycle.
4. If a documentation-context protocol lacks a valid activation binding, treat that as a bounded green-layer gap and correct it immediately when safe.
5. Use `vida docflow activation-check --root <dir> [files...]` as the bounded operational proof for activation coverage during documentation work.
6. Use `vida docflow protocol-coverage-check --root <dir> [files...]` as the bounded operational proof that canonical protocol-bearing artifacts are present in the protocol index and still have valid activation coverage.

## Immediate Gap Correction Rule

When documentation work or documentation-layer validation reveals a bounded defect inside an already-green layer:

1. correct the defect immediately in the same work cycle when the fix is safe and scope-bounded,
2. do not defer a green-layer usability, validation, mutation, or operator-surface defect merely because the main task can continue,
3. defer only when the required fix would materially widen scope beyond the active documentation task.

## Forbidden Dependencies

This protocol MUST NOT depend on runtime-owned latest resolution or ad hoc filesystem assumptions outside current canonical docs and DocFlow policy.

## Forbidden Behaviors

1. Do not invent undocumented documentation workflows.
2. Do not mutate footer metadata or sidecar history by hand when a lawful DocFlow path exists.
3. Do not justify documentation changes by future-layer intent alone.
4. Do not treat partial inventory or relation features as if they were already canonical authority.
5. Do not close documentation work without validation.
6. Do not restyle or reframe the repository `README.md` voice, presentation style, or narrative shape unless the user explicitly asks for a README stylistic rewrite; bounded factual updates are allowed, but stylistic transformation is forbidden by default.
7. Do not create a new protocol-bearing artifact before checking whether the topic already has a canonical owner.
8. Do not split one thematic law across several new protocol files unless the split is justified by distinct activation triggers or ownership boundaries.

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
9. `proofcheck --profile active-canon-strict` may be used as the one-command bounded closure proof for cross-layer active-canon work,
10. bootstrap-visible map registration is synchronized when the change created or rerouted a canonical project-visible document surface.

## Current Boundary Note

This protocol is intentionally bounded to the currently green documentation layers.

That means:

1. it is already safe to use for real documentation work now,
2. it already includes canonical runtime-readiness work when that work is bounded by promoted Layer 7 law,
3. those later layers may deepen this protocol later, but must not redefine the current lawful path retroactively.

-----
artifact_path: config/instructions/instruction-contracts/work.documentation-operation.protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/work.documentation-operation-protocol.md
created_at: '2026-03-10T04:10:00+02:00'
updated_at: 2026-07-03T12:12:00+03:00
changelog_ref: work.documentation-operation-protocol.changelog.jsonl
protocol_authoring_gate: enforced
protocol_compression_status: audit_passed
protocol_compression_algorithm: rfc2119-rewrite+list-compaction+protected-command-validation
protocol_compression_baseline_ref: 062a45c3d:vida/config/instructions/instruction-contracts/work.documentation-operation-protocol.md
protocol_compression_audit_at: 2026-07-03T12:12:00+03:00
protocol_compression_before_tokens: 3163
protocol_compression_after_tokens: 3113
protocol_compression_content_sha256: 4841269cb4f8e6dae59cf4ba6b598a377b5d4741cff4dc59841d6e705551ff91
