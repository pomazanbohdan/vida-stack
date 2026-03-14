# Runtime Family Map: DocFlow

Purpose: define the bounded `DocFlow` runtime family surface used for documentation-system operations and documentation-aware repository tooling.

## Runtime Identity

1. runtime family: `docflow`
2. current compatibility payload: project-local `.codex/**` plus the bounded `vida docflow` launcher contract
3. current role: bounded documentation/operator runtime surface
4. framework relationship: independently usable runtime family under the unified VIDA framework map

## Canonical Surfaces

1. canonical launcher contract:
   - `vida docflow`
2. executable/tool entrypoint:
   - `vida docflow`
3. compatibility payload:
   - project-local `.codex/**` when the selected host CLI is `codex`
4. bounded config/policy surfaces:
   - `vida/config/docflow/docsys_policy.yaml`
   - current canonical artifacts under `vida/config/docflow-*.current.jsonl`
5. governing framework protocol:
   - `instruction-contracts/work.documentation-operation-protocol`

## Activation Triggers

Read this map when:

1. the task is documentation-shaped,
2. documentation operator commands, proof checks, or inventory checks are needed,
3. documentation runtime readiness or documentation tooling ownership must be resolved,
4. a task asks about the `DocFlow` runtime family directly.
5. `taskflow` has entered its final runtime-consumption layer and needs the bounded documentation/inventory/readiness surface that the runtime must consume.
6. project activation is pending and a bounded documentation/config readiness surface is needed without entering TaskFlow.

## Routing

1. Documentation operation law:
   - continue to `instruction-contracts/work.documentation-operation-protocol`
2. Repository documentation commands:
   - continue to `docs/process/documentation-tooling-map.md`
3. Low-call operator/tooling path:
   - continue to `vida docflow`
   - use `vida docflow report-check --path <file>` when the runtime proof target is the required reporting-block shape rather than markdown artifact validation
4. Final runtime-consumption documentation law:
   - continue to `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`
   - continue to `docs/product/spec/canonical-runtime-layer-matrix.md`
   - use this branch when `taskflow` needs the final documentation/inventory/runtime-readiness surfaces that it must consume as runtime authority.
5. Pending activation companion surface:
   - stay in `vida project-activator` for bounded activation mutation,
   - use `vida docflow` for documentation/readiness inspection,
   - do not enter `vida taskflow` or any non-canonical external TaskFlow runtime while activation is still pending.

## Boundary Rule

1. `DocFlow` is a runtime/tooling surface, not the owner of framework-wide truth.
2. Documentation law remains in framework and project canonical docs.
3. `vida docflow` is the canonical launcher bridge for the current donor/runtime surface.
4. project-local `.codex/**` is compatibility payload only and must route through the same `vida docflow` boundary.
5. host CLI activation does not make `.codex/**` the owner of documentation law.
6. launcher contract is mode-stable:
   - repo/dev binary mode = in-process Rust `docflow` shell,
   - installed mode = the same in-process Rust `docflow` shell exposed through `vida docflow`.
7. `DocFlow` validation is allowed to fail closed on missing project-doc map registration and missing `AGENTS.sidecar.md` bootstrap pointers when canonical project-visible documentation surfaces are created or rerouted.
8. broader DocFlow command law still belongs to the donor/runtime family and future `docflow-rs` surface.
9. When activated from the final `taskflow` runtime layer, `DocFlow` resolves canonical inventory, readiness, bundle, and documentation-consumption evidence, but it does not replace `taskflow` as the primary runtime-consumption authority.

-----
artifact_path: config/system-maps/runtime-family.docflow
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/runtime-family.docflow-map.md
created_at: '2026-03-10T08:45:00+02:00'
updated_at: 2026-03-14T12:41:58.831031562Z
changelog_ref: runtime-family.docflow-map.changelog.jsonl
