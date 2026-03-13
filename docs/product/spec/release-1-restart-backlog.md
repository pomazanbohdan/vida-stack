# Release 1 Restart Backlog

Status: active product execution backlog

Purpose: define the first canonical restart backlog for `Release 1` so resumed development can proceed through bounded closure units derived from the Release-1 capability matrix, seam map, restart plan, and implementation-reality pass.

## 1. Scope

This backlog covers:

1. the first restart-era bounded work units,
2. the critical-path order for those units,
3. owner/runtime-family placement,
4. closure-class separation,
5. restart priority.

This backlog does not cover:

1. every future task in Release 1,
2. Release-2 work,
3. micro-level coding subtasks inside one bounded closure unit.

## 2. Backlog Rule

Every backlog item must map to:

1. one `Release slice`,
2. one `owner`,
3. one `layer` or `seam segment`,
4. one `closure class`,
5. one bounded code surface,
6. one bounded proof target.

Compact rule:

1. no mixed “work on Release 1 generally” items,
2. no tasks that need a later slice to justify their own closure,
3. no broad implementation continuation outside this bounded shape.

## 3. Priority Bands

Priority classes:

1. `P0`
   - critical-path blocker for Release-1 closure or restart control
2. `P1`
   - high-value slice closure needed before the next release step
3. `P2`
   - useful but not on the immediate critical path

## 4. First Restart Backlog

| ID | Priority | Release slice | Owner | Layer / seam segment | Closure class | Action posture | Bounded code surface | Bounded proof target | Closure criterion |
|---|---|---|---|---|---|---|---|---|---|
| R1-B01 | P0 | Slice 1 | `TaskFlow` | Layer 3 | implementation | `refactor / carve out` | launcher-owned execution logic in `crates/vida/src/main.rs` that belongs with tracked execution substrate | existing task/runtime smoke plus bounded TaskFlow execution proofs stay green after carve-out | tracked execution ownership is less launcher-concentrated without regression |
| R1-B02 | P0 | Slice 1 | `TaskFlow` | Layer 4 | implementation | `refactor / carve out` | launcher-owned routing/query/help logic in `crates/vida/src/main.rs` | route/query/operator proofs remain green and map cleanly to TaskFlow owner surface | lane/routing behavior is bounded under clearer TaskFlow ownership |
| R1-B03 | P0 | Slice 1 | `TaskFlow` | Layer 7 | implementation | `refactor / carve out` | run-graph/recovery ownership concentrated across `crates/vida/src/main.rs` and `crates/vida/src/state_store.rs` | run-graph/recovery/checkpoint/gate proofs remain green | recovery behavior stays intact while native ownership becomes cleaner |
| R1-B04 | P0 | Slice 5 | `seam` | Segment 1 | implementation | `extend` | `TaskFlow` direct-consumption activation path into bounded `DocFlow` branch | `consume final` path still green with explicit activation evidence | seam activation contract is explicit, narrow, and not shell-accidental |
| R1-B05 | P0 | Slice 5 | `seam` | Segment 2 | proof | `extend` | `DocFlow` readiness/proof return consumed by `TaskFlow` | seam-specific contract tests and end-to-end proof for readiness/proof return | `TaskFlow` consumes explicit `DocFlow` verdicts with no hidden state shortcut |
| R1-B06 | P0 | Slice 5 | `seam` | Segment 3 | proof | `extend` | final closure admission path and Wave-5 hardening evidence | bounded closure proof showing fail-closed admission to Release-1 closure | Release-1 closure admission is explicit, blockable, and proof-backed |
| R1-B07 | P1 | Slice 2 | `TaskFlow` | Layer 5 | implementation | `refactor / extend` | handoff/context-governance behavior still living as launcher-heavy logic | bounded handoff/recovery/operator proofs remain green | Layer-5 ownership is clearer and less shell-concentrated |
| R1-B08 | P1 | Slice 2 | `TaskFlow` | Layer 6 | implementation | `refactor / extend` | verification/approval gate logic partly living in launcher orchestration | doctor/verification/closure guardrails remain green | gate behavior is cleaner and better aligned with TaskFlow layer law |
| R1-B09 | P1 | Slice 2 | `TaskFlow` | Layer 2 | implementation | `extend` | DB-first activation/configurator integration through runtime state and launcher/state-store boundary | activation/config sync proofs and bounded status surfaces | activation state becomes a more stable primary runtime path |
| R1-B10 | P1 | Slice 3 | `TaskFlow` | Layers 8-9 | implementation | `extend` | strict compiled control/bundle and protocol-binding consumption path | bundle/readiness/consumption proofs remain green | runtime consumes stricter compiled control without broad raw-law fallback |
| R1-B11 | P1 | Slice 3 | `TaskFlow` | Layer 2 + protocol binding | proof | `extend` | protocol-binding authority on DB-first runtime spine | protocol-binding status/check/sync proofs plus compiled payload import evidence | binding path is explicit and trusted as runtime authority |
| R1-B12 | P1 | Slice 2 | `DocFlow` | Layer 4 | refactor | `refactor / carve out` | mutation-heavy shell concentration inside `crates/docflow-cli/src/lib.rs` | all in-process mutation proofs remain green | mutation semantics stay intact while shell density is reduced |
| R1-B13 | P1 | Slice 5 | `DocFlow` | Layer 7 | implementation | `extend` | readiness ownership toward seam-specific runtime consumption | readiness/proofcheck/profile proofs plus seam-facing tests | readiness outputs are shaped cleanly for TaskFlow consumption |
| R1-B14 | P2 | Slice 4 | `DocFlow` | Layers 5-6 | implementation | `extend` | relation/operator rendering surfaces | artifact-impact/task-impact/operator proofs remain green | DocFlow operator loop becomes stronger without blocking critical path |
| R1-B15 | P2 | Slice 4 | `TaskFlow` | Layers 3-6 | implementation | `extend` | planning/execution/status/approval loop surfaces in launcher and runtime state | planning/status/task graph/operator proofs | Slice-4 loop closes further once critical path is more stable |

## 5. Ordered Restart Sequence

The recommended first execution order is:

1. `R1-B01`
2. `R1-B02`
3. `R1-B03`
4. `R1-B04`
5. `R1-B05`
6. `R1-B06`
7. `R1-B07`
8. `R1-B08`
9. `R1-B09`
10. `R1-B10`
11. `R1-B11`
12. `R1-B12`
13. `R1-B13`

Sequencing rule:

1. complete the first P0 shell-concentration carve-outs before broadening Slice-4 feature work,
2. keep seam-hardening on the critical path,
3. allow P2 loop enrichment only after the critical path is visibly moving.

## 6. Action Split

Current bounded backlog mix:

1. `refactor / carve out`
   - `R1-B01`
   - `R1-B02`
   - `R1-B03`
   - `R1-B07`
   - `R1-B08`
   - `R1-B12`
2. `extend`
   - `R1-B04`
   - `R1-B05`
   - `R1-B06`
   - `R1-B09`
   - `R1-B10`
   - `R1-B11`
   - `R1-B13`
   - `R1-B14`
   - `R1-B15`

Backlog interpretation:

1. the restart is mostly carve-out plus extension,
2. it is not primarily rewrite,
3. this matches the implementation-reality pass.

## 7. Backlog Update Rule

This backlog must be refreshed whenever:

1. one item closes,
2. one item splits into smaller lawful closure units,
3. one seam blocker changes priority,
4. ownership concentration changes materially,
5. the implementation-reality pass is revised materially.

## 8. Closure Rule

The first restart backlog is valid only when:

1. every item still maps to one bounded closure unit,
2. every P0 item remains on the actual critical path,
3. no P2 item silently jumps ahead of unresolved P0 seam or shell-concentration blockers,
4. backlog execution can update the Release-1 matrices and seam map row-by-row.

-----
artifact_path: product/spec/release-1-restart-backlog
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/product/spec/release-1-restart-backlog.md
created_at: '2026-03-13T14:30:00+02:00'
updated_at: '2026-03-13T09:56:50+02:00'
changelog_ref: release-1-restart-backlog.changelog.jsonl
