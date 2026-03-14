# Framework Source Lineage Index

Purpose: preserve the principal framework-formation source lineage after the historical `docs/framework/plans/**` and `docs/framework/research/**` documents are removed from the active tree.

Status rule:

1. This file is provenance only.
2. It is not framework law, not product law, and not an executable runtime surface.
3. Settled semantics now live in promoted canonical homes such as `vida/config/instructions/**`, `docs/product/spec/**`, and `vida/config/**`.
4. Historical mutation lineage remains in Git history and surviving sidecar changelog evidence for active canonical artifacts.

Removal rule:

1. The archived framework-formation documents were used to shape the current framework.
2. They are removed because the framework now carries promoted canonical owners for the settled semantics they introduced.
3. This index preserves the principal deleted source paths and the main canonical surfaces that absorbed them.

## Principal Archived Source Set

| Archived source path | Primary promoted/canonical homes |
| --- | --- |
| `docs/framework/plans/vida-0.1-to-1.0-direct-binary-transition-plan.md` | `system-maps/framework.map.md`, `docs/product/spec/current-spec-map.md`, `docs/product/spec/canonical-runtime-layer-matrix.md` |
| `docs/framework/plans/vida-0.2-semantic-freeze-spec.md` | `docs/product/spec/partial-development-kernel-model.md`, `vida/config/migration/migration_paths.yaml` |
| `docs/framework/plans/vida-semantic-extraction-layer-map.md` | `docs/product/spec/partial-development-kernel-model.md`, `vida/config/migration/migration_paths.yaml` |
| `docs/framework/plans/vida-0.3-parity-and-conformance-spec.md` | `docs/product/spec/partial-development-kernel-model.md`, `vida/config/migration/migration_paths.yaml` |
| `docs/framework/plans/vida-0.3-state-kernel-schema-spec.md` | `docs/product/spec/partial-development-kernel-model.md`, `docs/product/spec/canonical-machine-map.md`, `vida/config/machines/**`, `vida/config/policies/closure_policy.yaml` |
| `docs/framework/plans/vida-0.3-route-and-receipt-spec.md` | `docs/product/spec/partial-development-kernel-model.md`, `docs/product/spec/receipt-and-proof-law.md`, `docs/product/spec/canonical-runtime-layer-matrix.md`, `vida/config/routes/route_catalog.yaml`, `vida/config/machines/**` |
| `docs/framework/plans/vida-0.3-instruction-kernel-spec.md` | `docs/product/spec/instruction-artifact-model.md`, `docs/product/spec/canonical-runtime-readiness-law.md`, `instruction_catalog.yaml` |
| `docs/framework/plans/vida-0.3-migration-kernel-spec.md` | `docs/product/spec/canonical-runtime-readiness-law.md`, `vida/config/machines/boot_migration_gate.yaml`, `runtime-instructions/runtime.export-protocol.md` |
| `docs/framework/plans/vida-0.3-command-tree-spec.md` | `system-maps/framework.map.md`, `system-maps/protocol.index.md` |
| `docs/framework/plans/vida-0.3-db-first-runtime-spec.md` | `docs/product/spec/canonical-runtime-layer-matrix.md`, `runtime-instructions/runtime.direct-runtime-consumption-protocol.md`, `vida/config/machines/boot_migration_gate.yaml` |
| `docs/framework/plans/vida-0.3-storage-kernel-spec.md` | `system-maps/protocol.index.md`, `runtime-instructions/runtime.export-protocol.md` |
| `docs/framework/plans/vida-0.3-instruction-memory-and-sidecar-spec.md` | `system-maps/protocol.index.md`, `runtime-instructions/runtime.export-protocol.md` |
| `docs/framework/research/agentic-agent-definition-system.md` | `agent-definitions/model.agent-definitions-contract.md`, `agent-definitions/role.role-profile-contract.md`, `docs/product/spec/agent-role-skill-profile-flow-model.md` |
| `docs/framework/research/agentic-terminology-glossary.md` | `agent-definitions/model.agent-definitions-contract.md`, `system-maps/framework.map.md` |
| `docs/framework/research/agentic-cheap-worker-packet-system.md` | `runtime-instructions/lane.agent-handoff-context-protocol.md`, `system-maps/protocol.index.md` |
| `docs/framework/research/agentic-proof-obligation-registry.md` | `runtime-instructions/work.verification-lane-protocol.md`, `system-maps/protocol.index.md` |
| `docs/framework/research/canonical-runtime-readiness-external-patterns.md` | `docs/product/spec/canonical-runtime-readiness-law.md`, `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`, `codex-v0/codex.py` |
| `docs/framework/research/vida-1.0-agent-runtime-external-alignment.md` | `docs/product/spec/canonical-runtime-layer-matrix.md`, `runtime-instructions/lane.agent-handoff-context-protocol.md`, `runtime-instructions/recovery.checkpoint-replay-recovery-protocol.md`, `system-maps/observability.map.md` |

## Use Rule

1. When a canonical artifact needs historical provenance, cite this index instead of reviving deleted framework-formation documents.
2. When a canonical artifact needs current law, cite the promoted owner surface instead of this index.
3. Do not recreate `docs/framework/plans/**` or `docs/framework/research/**` as parallel active canon.

## Validation Target

1. After cleanup, no active canonical artifact should require the deleted framework-formation documents to resolve current law.
2. Any surviving reference to deleted paths outside this index is drift and should be corrected.

-----
artifact_path: process/framework-source-lineage-index
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/process/framework-source-lineage-index.md
created_at: '2026-03-11T00:00:00+02:00'
updated_at: '2026-03-12T07:58:34+02:00'
changelog_ref: framework-source-lineage-index.changelog.jsonl
