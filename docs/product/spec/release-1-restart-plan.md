# Release 1 Restart Plan

Status: active product execution plan

Purpose: define the canonical restart plan for `Release 1` so active `TaskFlow` and `DocFlow` development can resume over the already existing codebase through controlled protocols, matrix-first decomposition, and bounded closure units instead of ad hoc feature drift.

## 1. Problem Statement

Active Rust development for both `TaskFlow` and `DocFlow` has already produced real implementation value.

However, the earlier execution posture became inefficient because:

1. work advanced before one explicit release control model existed,
2. `TaskFlow`, `DocFlow`, and Release-1 product requirements were not always managed from the same bounded control surface,
3. code slices, architecture decisions, and proof surfaces could drift apart,
4. mixed work items made it hard to tell whether a change was closing:
   - law,
   - implementation,
   - proof,
   - or only local code movement.

Restart rule:

1. the restart must preserve existing code and proofs,
2. it must replace the old execution posture,
3. it must not treat the current codebase as disposable or start-from-zero material.

## 2. Restart Objective

The restart objective is:

1. keep the existing codebase,
2. stop uncontrolled widening,
3. reframe work through the promoted Release-1 matrices and seam map,
4. rebuild the active backlog as bounded closure units,
5. resume implementation only after reality, gaps, and critical path are explicit.

Compact rule:

1. do not restart the codebase,
2. restart the control model around the codebase.

## 3. Canonical Inputs

Primary restart owners:

1. `docs/product/spec/release-1-capability-matrix.md`
2. `docs/product/spec/release-1-seam-map.md`
3. `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`
4. `docs/product/spec/docflow-v1-runtime-modernization-plan.md`
5. `docs/product/spec/canonical-runtime-layer-matrix.md`
6. `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`
7. `docs/process/vida1-development-conditions.md`

Restart-input rule:

1. the matrices define what must exist,
2. the modernization plans define ownership and target shape,
3. the development-conditions ledger defines what is already proven,
4. the restart plan exists to reconnect these into one controlled execution model.

## 4. Restart Boundary Rule

During the restart, the following are temporarily frozen as uncontrolled work:

1. new capability directions not mapped to a Release-1 slice,
2. mixed tasks that span `TaskFlow`, `DocFlow`, and shell concerns without explicit seam ownership,
3. architectural rewrites that are not grounded in a matrix gap or seam blocker,
4. speculative Release-2 capability disguised as Release-1 work,
5. refactors that do not move one matrix status or one proof boundary.

Allowed work during restart:

1. implementation-reality mapping,
2. gap mapping,
3. seam clarification,
4. bounded bug fixes needed to keep already-proven paths stable,
5. backlog rebuilding into closure units.

## 5. Restart Model

The restart model is:

1. `matrix-first`
2. `reality-pass second`
3. `gap-map third`
4. `bounded backlog fourth`
5. `vertical closure execution fifth`

Execution rule:

1. no new implementation slice should begin unless it can be located in:
   - one Release-1 slice,
   - one runtime family owner,
   - one layer or seam segment,
   - one closure class.

## 6. Phase 1: Implementation Reality Pass

### 6.1 Purpose

Build one factual map of what already exists in code and proofs.

### 6.2 Required Output

The reality pass must produce one table in this shape:

1. `Release slice`
2. `Runtime family`
3. `Layer or seam segment`
4. `Owner crate/module/file`
5. `Law status`
6. `Implementation status`
7. `Proof status`
8. `Strongest evidence`
9. `Main current gap`

### 6.3 Reality Rule

1. the reality pass must describe the actual codebase,
2. it must not restate desired architecture as if it already existed,
3. already-proven surfaces from `vida1-development-conditions.md` must be counted as real assets, not rediscovered later.

## 7. Phase 2: Gap Map

### 7.1 Purpose

Convert the reality pass into one bounded gap model for Release 1.

### 7.2 Gap Types

Each gap must be classified as exactly one of:

1. `law gap`
2. `implementation gap`
3. `proof gap`
4. `seam gap`
5. `bridge cleanup gap`

### 7.3 Gap Rule

1. no gap may be recorded as a vague “needs more work” statement,
2. each gap must map to one owner and one closure target,
3. if a gap is outside Release 1, it must be deferred explicitly rather than left mixed in the active queue.

## 8. Phase 3: New Decomposition Model

### 8.1 Canonical Work Unit

The new unit of work is:

1. one `Release slice`,
2. one `Runtime family owner`:
   - `TaskFlow`
   - `DocFlow`
   - `seam`
3. one `Layer` or `seam segment`,
4. one `Closure class`:
   - `law`
   - `implementation`
   - `proof`

### 8.2 Canonical Task Shape

Every restart-era work item must define:

1. target Release slice,
2. target owner,
3. target layer or seam segment,
4. target closure class,
5. bounded code surface,
6. bounded proof surface,
7. closure criterion,
8. matrix row or seam segment that should change after closure.

### 8.3 Forbidden Decomposition Pattern

The following are forbidden restart-era task shapes:

1. “finish TaskFlow”
2. “continue DocFlow”
3. “work on Release 1 generally”
4. one task that changes several runtime-family owners without one explicit seam scope
5. one task that mixes architecture, code, and proof across unrelated slices.

## 9. Phase 4: Critical Path Rule

The Release-1 critical path must be managed explicitly.

Current critical path:

1. Slice 1 `Operational Spine` native closure,
2. Slice 2 `Project Activation Surface` DB-first closure,
3. Slice 3 `Compiled Runtime Bundles` strict control-bundle closure,
4. Slice 5 seam path:
   - `TaskFlow` Layer 9 activation
   - `DocFlow` readiness/proof return
   - Release-1 closure admission

Critical-path rule:

1. work that does not move the critical path is allowed only when:
   - it preserves already-proven behavior,
   - or it removes a blocker for a critical-path slice,
2. otherwise it should wait.

## 10. Phase 5: Vertical Closure Execution

Once the new backlog exists, implementation resumes only through vertical slices.

Each vertical slice must follow:

1. target one bounded closure unit,
2. align or update law if needed,
3. implement bounded code change,
4. run bounded proof,
5. update the affected matrix or seam status.

Closure rule:

1. code change without proof is not closed,
2. proof without matrix update is not closed,
3. matrix update without real code/proof movement is not closed unless the task was explicitly law-only.

## 11. Existing Codebase Preservation Rule

The existing codebase is an input asset, not restart waste.

Preserve and reuse:

1. already-green Rust crates and tests,
2. already-proven launcher-owned surfaces under `crates/vida/**`,
3. already-proven `DocFlow` in-process commands and package tests,
4. already-proven `TaskFlow` state, recovery, planning, and direct-consumption slices,
5. donor-parity fixtures and bridge behaviors that still carry lawful continuity.

Do not preserve as permanent architecture:

1. bridge-only execution paths whose ownership now belongs in native Rust crates,
2. monolithic donor modules that prevent clear runtime-family ownership,
3. ambiguous mixed shell logic when the restart model gives it a bounded owner.

## 12. Restart Order

The restart should execute in this order:

1. complete the implementation reality pass,
2. produce the Release-1 gap map,
3. rebuild the backlog into canonical closure units,
4. isolate the seam-critical backlog,
5. resume critical-path implementation,
6. allow secondary slices only after they do not compete with critical-path closure.

## 13. First Mandatory Restart Outputs

The restart is considered initialized only when all of the following exist:

1. one implementation-reality table for Release 1,
2. one Release-1 gap map,
3. one rebuilt backlog using the canonical work-unit shape,
4. one explicit critical-path view,
5. one explicit seam-blocker view for `release-1-seam-map.md`.

## 14. Closure Rule

The restart phase is closed only when:

1. active work is no longer organized around ad hoc mixed tasks,
2. every active task maps to one bounded closure unit,
3. the Release-1 critical path is explicit,
4. the `TaskFlow -> DocFlow -> Release 1 closure` seam is managed as a separate blocker surface,
5. implementation can resume without losing matrix control.

-----
artifact_path: product/spec/release-1-restart-plan
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/product/spec/release-1-restart-plan.md
created_at: '2026-03-13T13:55:00+02:00'
updated_at: '2026-03-13T09:28:49+02:00'
changelog_ref: release-1-restart-plan.changelog.jsonl
