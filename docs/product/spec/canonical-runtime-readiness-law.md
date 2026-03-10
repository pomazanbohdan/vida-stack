# VIDA Canonical Runtime Readiness Law

Status: active product law

Purpose: define the canonical readiness gate between canonical inventory/operator layers and runtime consumption so VIDA can determine whether the current canon is safe for runtime use without silently inferring readiness from presence alone.

## 1. Scope

This spec defines:

1. readiness inputs,
2. readiness verdict classes,
3. tuple, compatibility, bundle, projection, and gate requirements,
4. fail-closed blocking rules,
5. the current bounded operational proof surface.

This spec does not define:

1. runtime consumption behavior,
2. live route progression,
3. actual migration execution logic.

## 2. Canonical Readiness Purpose

The readiness layer exists to answer:

1. whether the current canonical artifact set is complete enough for runtime use,
2. whether version-bearing inputs are resolved,
3. whether projections and bundles are valid enough to trust,
4. whether boot-gate prerequisites are present and non-blocking,
5. whether runtime consumption must fail closed.

## 3. Canonical Readiness Inputs

The minimum readiness input set is:

1. canonical inventory rows,
2. explicit version tuples from canonical markdown and machine-readable artifacts,
3. compatibility-class declarations,
4. canonical bundle artifacts,
5. explicit projection bindings,
6. boot-gate and migration gate artifacts.

Current canonical sources:

1. `docs/product/spec/canonical-inventory-law.md`
2. `docs/product/spec/instruction-artifact-model.md`
3. `vida/config/instructions/projection_manifest.yaml`
4. `vida/config/instructions/bundles/default_runtime.yaml`
5. `vida/config/migration/compatibility_classes.yaml`
6. `vida/config/migration/boot_gates.yaml`
7. `vida/config/codex-registry.current.jsonl`
8. `vida/config/codex-readiness.current.jsonl` as the current materialized readiness report path

## 4. Source-Version Tuple Rule

At minimum, readiness must see:

1. `artifact_version`
2. `artifact_revision`

Rules:

1. version-bearing canonical artifacts must expose a complete source-version tuple,
2. unresolved tuples are blocking,
3. tuple presence must be explicit and must not be inferred only from timestamps or filenames.

## 5. Compatibility Class Rule

Rules:

1. machine-readable canonical artifacts that participate in runtime law must declare `compatibility_class`,
2. the declared class must be supported by the canonical compatibility-class catalog,
3. unknown compatibility classes are blocking,
4. compatibility class presence does not itself authorize runtime consumption; it only satisfies one readiness input.

## 6. Bundle Completeness Rule

Rules:

1. the canonical runtime bundle must exist,
2. it must expose explicit bundle ordering,
3. required bundle families must be present in the declared order,
4. missing canonical bundle families are blocking,
5. bundle presence does not replace activation or projection checks.

## 7. Projection Binding And Freshness Rule

Rules:

1. when a canonical markdown artifact declares `projection_ref`, the target projection must exist,
2. the target projection must expose `version`, `revision`, and `compatibility_class`,
3. projection version and revision must match the authoritative markdown tuple,
4. a mismatched or missing declared projection is blocking,
5. readiness must not silently treat an undeclared projection as valid just because a similar file exists nearby.

## 8. Boot-Gate Rule

Readiness must include the canonical boot-gate artifact set.

At minimum:

1. compatibility catalog present,
2. instruction catalog present,
3. route catalog present,
4. receipt taxonomy present,
5. proof taxonomy present,
6. boot-gate definitions present,
7. required non-blocking gate outcomes are representable by canonical law.

Rules:

1. missing required gate artifacts are blocking,
2. fail-closed gate rules from canonical migration law remain binding,
3. readiness may report only bounded gate presence and gate-law sufficiency until runtime execution exists.

## 9. Readiness Verdict Classes

The canonical readiness verdict classes are:

1. `ready`
2. `blocked`
3. `insufficient_evidence`

Rules:

1. `ready` means all required readiness inputs are satisfied,
2. `blocked` means a required input is missing, unresolved, incompatible, or mismatched,
3. `insufficient_evidence` means canonical law requires a proof surface that is not yet present,
4. any non-`ready` verdict is fail-closed for runtime consumption.

## 10. Blocking Reason Rule

Readiness output must report explicit blocking reasons.

The minimum blocker families are:

1. `missing_version_tuple`
2. `missing_projection_target`
3. `projection_tuple_mismatch`
4. `missing_compatibility_class`
5. `unsupported_compatibility_class`
6. `missing_bundle`
7. `bundle_family_gap`
8. `missing_boot_gate_artifact`
9. `insufficient_gate_evidence`

## 11. Current Bounded Operational Proof

The current transitional bounded proof surface is:

1. `python3 codex-v0/codex.py readiness-check --profile active-canon`
2. `python3 codex-v0/codex.py readiness-write --profile active-canon --canonical`

Current scope of that proof:

1. version tuple completeness for canonical markdown,
2. explicit projection binding existence and tuple parity where declared,
3. compatibility-class presence and support for machine-readable runtime-law artifacts,
4. canonical bundle presence and family completeness,
5. canonical boot-gate artifact presence.

Materialized-readiness rule:

1. the transitional canonical readiness report path is `vida/config/codex-readiness.current.jsonl`,
2. the readiness artifact must contain only current blocking rows for the selected canonical scope,
3. an empty materialized readiness artifact means the selected scope is currently `ready` under bounded Layer 7 proof,
4. the materialized readiness artifact is evidence only and does not by itself authorize runtime consumption.

## 12. Completion Proof For Layer 7

Layer 7 is closed when all of the following are true:

1. one promoted canonical readiness spec exists,
2. tuple, compatibility, bundle, projection, and gate rules are explicit,
3. readiness verdict classes are explicit,
4. fail-closed blocking reasons are explicit,
5. a bounded readiness-check exists and follows this law.

## 13. Standalone Value

This layer gives VIDA a pre-runtime gate that can say whether the canonical system is safe for runtime consumption before runtime execution begins.

## 14. Source Absorption

This spec absorbs and concentrates readiness law previously scattered across:

1. `docs/framework/plans/vida-0.3-instruction-kernel-spec.md`
2. `docs/framework/plans/vida-0.3-migration-kernel-spec.md`
3. `docs/product/spec/instruction-artifact-model.md`
4. `docs/framework/research/canonical-runtime-readiness-external-patterns.md`

-----
artifact_path: product/spec/canonical-runtime-readiness-law
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: docs/product/spec/canonical-runtime-readiness-law.md
created_at: '2026-03-10T05:05:00+02:00'
updated_at: '2026-03-10T04:07:10+02:00'
changelog_ref: canonical-runtime-readiness-law.changelog.jsonl
