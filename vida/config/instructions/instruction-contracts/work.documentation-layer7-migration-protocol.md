# Documentation Layer 7 Migration Protocol

Purpose: define the canonical agent protocol for migrating the documentation of any project toward Layer 7 closure using the VIDA documentation layers without depending on Layer 8 runtime-consumption behavior.

## Scope

This protocol applies when the active task is to:

1. migrate a project's documentation system toward the VIDA Layer 1 through Layer 7 model,
2. normalize canonical homes, metadata, lineage, inventory, validation, relations, operator views, and readiness law,
3. reduce duplication and scattered authority while preserving project-specific standards.

This protocol does not define:

1. runtime-consumption integration,
2. project code/runtime migration beyond the documentation and information-system layer,
3. permission to widen scope into Layer 8 behavior.

## Activation Rule

Activate this protocol immediately when the task context is any of:

1. "migrate documentation to Layer 7",
2. "bring project documentation to VIDA canonical layers",
3. "normalize docs/instructions/inventory/readiness for another project",
4. any equivalent task where the main goal is documentation-system migration rather than one local document edit.

This protocol is a triggered-domain extension of:

1. `vida/config/instructions/instruction-contracts/work.documentation-operation-protocol.md`
2. `vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md`

## Target State

The migration target is a documentation system where:

1. Layer 1 through Layer 7 are explicitly modeled,
2. each closed layer is independently useful,
3. each next layer depends only on already-closed lower layers,
4. active canonical law has one canonical home,
5. metadata, lineage, inventory, validation, relations, operator ergonomics, and readiness all have bounded proofs,
6. Layer 8 remains out of scope unless runtime-consumption law is explicitly requested and available.

## Precedence Rule

Documentation migration must respect formatting and artifact-shape authority in this order:

1. active skill-specific artifact format,
2. explicit project-owned documentation standard,
3. promoted product-law documentation contract,
4. bounded `DocFlow` fallback behavior.

Migration must therefore:

1. normalize the system to VIDA layers without erasing higher-precedence project or skill-specific artifact formats,
2. preserve canonical metadata, lineage, validation, and deduplication law even when body format varies by project.

## Migration Sequence

The migration sequence is outer-to-inner and layer-ordered.

### Phase A: Establish Authority And Map

1. identify the active documentation surface,
2. identify current maps, indexes, specs, process docs, and instruction docs,
3. identify which files are law-bearing and which are only summaries, pointers, or history,
4. identify project-specific standards that must outrank `DocFlow` defaults.

### Phase B: Close Layer 1

1. normalize vocabulary,
2. define artifact classes, statuses, owners, layers, and metadata/footer contract,
3. ensure no active canonical artifact depends on implicit identity only.

### Phase C: Close Layer 2

1. define the active inventory scope,
2. define one canonical registry structure and path,
3. ensure markdown and machine-readable law are both inventory-visible,
4. make source/projection linkage inspectable.

### Phase D: Close Layer 3

1. define validation gates,
2. define warning/error posture,
3. define explicit exception policy only where needed,
4. ensure bounded proof exists for consistency.

### Phase E: Close Layer 4

1. define lawful mutation paths,
2. normalize metadata/changelog update rules,
3. define link migration and identity-safe moves/renames,
4. prevent manual drift in footer and sidecar surfaces.

### Phase F: Close Layer 5

1. define relation surfaces and edge taxonomy,
2. define direct/reverse relations,
3. define artifact impact and task impact,
4. ensure relation validation is bounded and canonical.

### Phase G: Close Layer 6

1. define low-call operator views,
2. define bounded overview, history, impact, and state reads,
3. ensure one-command orientation exists for active canon.

### Phase H: Close Layer 7

1. define readiness inputs,
2. define tuple, projection, bundle, compatibility, and gate rules,
3. define fail-closed blocker classes,
4. define bounded readiness proof,
5. materialize the current readiness report if the project needs shared readiness evidence.

## Deduplication Rule

During migration:

1. one canonical rule must end with one canonical home,
2. matrix docs may summarize a layer but must not duplicate its law,
3. plans and research may remain evidence or input but must not continue as the active canonical home once a promoted law exists,
4. if a duplicated active law is discovered, reduce that duplication in the same bounded migration slice when safe.

## Project-Safe Boundary Rule

When applied to another project:

1. do not silently force VIDA repository structure onto the target project,
2. migrate semantic layers first, filesystem layout second,
3. preserve project-owned naming and body format where compatible with canonical metadata and lineage law,
4. only normalize physical paths when the project explicitly wants that migration.

## Required Proof Path

The bounded proof path for documentation migration to Layer 7 is:

1. `python3 codex-v0/codex.py layer-status --layer <N>` for the current target layer,
2. `python3 codex-v0/codex.py doctor --layer <N>` for bounded layer validation,
3. `python3 codex-v0/codex.py proofcheck --layer <N>` for bounded layer closure,
4. `python3 codex-v0/codex.py proofcheck --profile active-canon-strict` when the slice spans multiple layers,
5. `python3 codex-v0/codex.py readiness-check --profile active-canon` and `readiness-write --canonical` when Layer 7 is touched.

## Closure Rule

Documentation migration to Layer 7 is closed only when:

1. the target migration slice has one canonical law-bearing home,
2. duplicated active authority is removed or reduced within scope,
3. the current target layer is explicit and self-contained,
4. bounded doctor/proof commands pass for the touched layer or layer set,
5. readiness proof passes when Layer 7 surfaces changed,
6. no Layer 8 behavior is assumed or claimed without runtime authority.

## Standalone Value

This protocol lets an agent migrate the documentation system of another project into a Layer 1 through Layer 7 shape that is already operational and useful before runtime-consumption integration exists.

-----
artifact_path: config/instructions/instruction-contracts/work.documentation-layer7-migration.protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/work.documentation-layer7-migration-protocol.md
created_at: '2026-03-10T04:23:08+02:00'
updated_at: '2026-03-11T12:33:10+02:00'
changelog_ref: work.documentation-layer7-migration-protocol.changelog.jsonl
