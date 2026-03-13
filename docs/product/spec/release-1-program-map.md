# Release 1 Program Map

Status: active product execution map

Purpose: define the canonical Release 1 navigation map across architecture, execution program, runtime tracks, and proof surfaces without duplicating owner law.

## 1. Why This Map Exists

Release 1 is now owned across several different artifact classes:

1. top-level architecture law,
2. execution-program sequencing,
3. runtime-family modernization tracks,
4. supporting owner specs,
5. proof and implementation-condition surfaces.

Without one bounded program map:

1. `VERSION-PLAN.md` becomes overloaded with inner program detail,
2. `compiled-autonomous-delivery-runtime-architecture.md` becomes overloaded with working-program navigation,
3. `current-spec-map.md` remains correct but too broad to act as the day-to-day Release-1 entrypoint,
4. track owners can drift because people start from different documents.

## 2. Program Rule

This map is:

1. the practical Release-1 navigation entrypoint,
2. the pointer layer above the execution program and runtime-track owners,
3. the place where current Release-1 working surfaces are grouped explicitly.

This map is not:

1. the full version ladder,
2. the full product-spec registry,
3. the owner of runtime-family law already owned elsewhere.

## 3. Release-1 Document Classes

Release 1 currently uses four document classes.

### 3.1 Architecture Anchor

Top-level architecture direction is owned by:

1. `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`

This anchor defines:

1. mission,
2. runtime planes,
3. TaskFlow/DocFlow seam,
4. architectural invariants,
5. implementation order.

### 3.2 Execution Program

The execution program is owned by:

1. `docs/product/spec/release-1-wave-plan.md`

This owner defines:

1. wave ordering,
2. bounded scope per wave,
3. donor/proof base,
4. wave closure requirements.

### 3.3 Runtime-Track Owners

The active Release-1 runtime tracks are owned by:

1. `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`
2. `docs/product/spec/docflow-v1-runtime-modernization-plan.md`

Track rule:

1. these tracks remain separate owner plans,
2. Release 1 must not merge them into one generic runtime blob,
3. cross-track coordination happens through the architecture anchor and execution program rather than by collapsing their ownership.

### 3.3.1 Release Capability Matrix

The cross-track Release-1 capability projection is owned by:

1. `docs/product/spec/release-1-capability-matrix.md`

Matrix rule:

1. this matrix combines Release-1 product requirements with `TaskFlow` and `DocFlow` capability coverage,
2. it does not replace the runtime-track owners,
3. it is the bounded control surface for judging whether Release-1 closure is coherent across both runtime families.

### 3.3.2 Release Seam Map

The critical Release-1 closure seam is owned by:

1. `docs/product/spec/release-1-seam-map.md`

Seam-map rule:

1. this map isolates the final `TaskFlow -> DocFlow -> Release 1 closure` path,
2. it is the narrowest control surface for final hardening and closure blockers,
3. it must remain aligned with the wider release-capability matrix and both runtime-family matrices.

### 3.3.3 Release Restart Plan

The controlled-development restart owner is:

1. `docs/product/spec/release-1-restart-plan.md`

Restart-plan rule:

1. this plan defines how active development resumes over the existing codebase,
2. it converts Release-1 work into matrix-first bounded closure units,
3. it must be used before broad implementation continuation when the delivery model has drifted from the canonical control surfaces.

### 3.3.4 Implementation Reality Pass

The current reality-backed restart input is:

1. `docs/product/spec/release-1-implementation-reality-pass.md`

Reality-pass rule:

1. this report estimates current readiness by slice, layer, and seam,
2. it is the canonical basis for restart backlog rebuilding,
3. it must be refreshed when ownership concentration or proof posture changes materially.

### 3.3.5 Restart Backlog

The first bounded restart backlog is owned by:

1. `docs/product/spec/release-1-restart-backlog.md`

Backlog rule:

1. this backlog converts the restart plan and reality pass into ordered closure units,
2. it is the practical execution queue for the first restart cycle,
3. it must stay aligned with the release matrix, seam map, and implementation-reality pass.

### 3.4 Supporting Owner Specs

Release-1 supporting detail is owned by adjacent specs such as:

1. `docs/product/spec/compiled-runtime-bundle-contract.md`
2. `docs/product/spec/project-activation-and-configurator-model.md`
3. `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
4. `docs/product/spec/bootstrap-carriers-and-project-activator-model.md`
5. `docs/product/spec/execution-preparation-and-developer-handoff-model.md`
6. `docs/product/spec/runtime-paths-and-derived-cache-model.md`
7. `docs/product/spec/operational-state-and-synchronization-model.md`
8. `docs/product/spec/extensibility-and-output-template-model.md`

### 3.5 Proof And Readiness Surfaces

Proof and current implementation conditions are anchored by:

1. `docs/process/vida1-development-conditions.md`
2. current Rust workspace surfaces under `crates/taskflow-*`, `crates/docflow-*`, and `crates/vida/**`

Proof rule:

1. the program map points to proof surfaces,
2. it does not replace them with summary claims.

## 4. Recommended Reading Path

The recommended Release-1 reading path is:

1. start here,
2. read `compiled-autonomous-delivery-runtime-architecture.md` for top-level direction,
3. read `release-1-wave-plan.md` for execution sequencing,
4. read the relevant runtime-track owner:
   - `taskflow-v1-runtime-modernization-plan.md`
   - `docflow-v1-runtime-modernization-plan.md`
5. read the supporting owner spec for the currently active concern,
6. confirm current conditions in `docs/process/vida1-development-conditions.md`.

## 5. Program Routing Pointers

Use the following routing rules:

1. version-scope question:
   - start with `VERSION-PLAN.md`
2. Release-1 execution question:
   - start with this map, then `release-1-wave-plan.md`
3. top-level architecture/boundary question:
   - start with `compiled-autonomous-delivery-runtime-architecture.md`
4. TaskFlow implementation question:
   - start with `taskflow-v1-runtime-modernization-plan.md`
5. DocFlow implementation question:
   - start with `docflow-v1-runtime-modernization-plan.md`
6. Release-1 cross-track closure question:
   - start with `release-1-capability-matrix.md`
7. Release-1 seam or hardening blocker question:
   - start with `release-1-seam-map.md`
8. Release-1 restart or controlled-resumption question:
   - start with `release-1-restart-plan.md`
9. Release-1 implementation-readiness or rewrite-pressure question:
   - start with `release-1-implementation-reality-pass.md`
10. Release-1 restart backlog or execution ordering question:
   - start with `release-1-restart-backlog.md`
11. current closure/proof question:
   - start with `docs/process/vida1-development-conditions.md`
12. full registry/canon-discovery question:
   - start with `docs/product/spec/current-spec-map.md`

## 6. Current Rule

1. Release 1 keeps one architecture anchor, one execution-program owner, one release-capability matrix, one release seam map, one restart plan, one implementation-reality report, one restart backlog, two runtime-track owners, and several supporting owner specs.
2. `VERSION-PLAN.md` stays above the inner Release-1 program.
3. `current-spec-map.md` stays the full canonical registry rather than the day-to-day Release-1 working entrypoint.
4. this map is the bounded Release-1 navigation entrypoint until a later runtime-native program surface replaces it.

-----
artifact_path: product/spec/release-1-program-map
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/product/spec/release-1-program-map.md
created_at: '2026-03-13T08:39:49+02:00'
updated_at: '2026-03-13T09:56:50+02:00'
changelog_ref: release-1-program-map.changelog.jsonl
