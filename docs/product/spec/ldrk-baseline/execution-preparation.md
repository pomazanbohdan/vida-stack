# LDRK Baseline Execution Preparation

Status: generated execution-preparation artifact for TaskFlow task `ldr-001`.

## architecture_preparation_report

Target implementation area: `scripts/architecture` baseline tooling and `docs/product/spec/ldrk-baseline` generated inventory artifacts.

Relevant architecture context: LDRK moves direct run graph, lane, dispatch, host-bridge, continuation, claim, and task-record mutation toward `VidaCommandEnvelope`, deterministic completion algebra, and a redb-backed `OperationalJournal` for operational records.

Important invariants: generated artifacts must be reproducible on unchanged sources; domain crates must not depend on redb/SurrealDB/Restate storage types; baseline work is measurement only and does not cut over authority.

Integration/dependency concerns: `rg` and Python are sufficient for the baseline; `tokei`/`scc` are optional and recorded in `baseline.json` rather than required.

Expected implementation shape: one deterministic Python script scans owned runtime source roots and emits JSON plus markdown artifacts.

## developer_handoff_packet

Prepared task target: implement and maintain `scripts/architecture/ldrk_baseline_inventory.py` and generated artifacts under `docs/product/spec/ldrk-baseline`.

Intended implementation direction: keep the scanner dependency-free, lexical, deterministic, and explicit about known limitations.

Bounded next steps for developer lane: refine parser precision only when a later task needs more exact command metadata; do not introduce runtime authority changes in `ldr-001`.

Required proofs/tests/checks: run the inventory twice, compare stable hashes, inspect the host-bridge drift-map section, run TaskFlow graph validation before task closure.

Preparation findings: baseline sha256 `e1ca2f418df416208f64fcc89bb244d6cd351865c47e435820325ea576203df7`; targeted production LOC `161388`; direct mutation candidates `1621`; production outcome classifier candidates `260`; status helper false positives `409`; cfg(test) classifier candidates `276`; cfg(test) status helper candidates `767`.

## change_boundary

May change: `scripts/architecture/**`, `docs/product/spec/ldrk-baseline/**`, and spec map/catalog pointers when needed.

Must not change: `.vida` runtime state by hand, TaskFlow/DocFlow authority stores, production runtime command behavior, or dependency manifests.

Reuse rather than rewrite: existing VIDA runtime surfaces, TaskFlow records, DocFlow specs, and release packaging scripts.

Escalate before mutation: any production Rust runtime authority change, storage dependency change, command contract change, or generated runtime snapshot change.

## dependency_impact_summary

Relevant dependencies: Python standard library, `rg` if available, optional `tokei`/`scc` availability recorded as metadata.

Likely coupling points: Rust CLI definitions, runtime state-store modules, host-bridge and lane receipt paths, TaskFlow artifact registry.

Migration or compatibility risks: lexical counts are baseline indicators, not semantic proof; later cutover tasks must add contract/integration tests.

Outward impact to preserve: generated baseline artifacts must remain byte-stable and safe to regenerate locally without mutating runtime state.

## spec_alignment_summary

Governing specs/protocols: LDRK epic notes, execution preparation handoff model, canonical inventory law, runtime readiness law, and TaskFlow runtime binding model.

Required alignment: baseline metrics must support the LDRK code-reduction and drift-reduction gates without moving authority or introducing dual-write behavior.

Open questions: precise semantic command tree extraction and exact mutation ownership can be improved in later LDRK implementation slices; they do not block this baseline artifact.

-----
artifact_path: product/spec/ldrk-baseline/execution-preparation
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-06-22
schema_version: 1
status: generated
source_path: docs/product/spec/ldrk-baseline/execution-preparation.md
created_at: 2026-06-22T00:00:00Z
updated_at: 2026-06-22T00:00:00Z
changelog_ref: ldrk-baseline/execution-preparation.changelog.jsonl
