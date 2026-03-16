# Functional Matrix Protocol

Status: active product law

Purpose: define the canonical protocol for designing, updating, and proving functional and capability matrices so layered runtime maps remain operationally useful, code-linked, and ownership-safe.

## 1. Scope

This protocol defines:

1. when VIDA should use a functional matrix,
2. the required structure of a canonical matrix,
3. the minimum row schema for each layer or capability slice,
4. the required status split between law, implementation, and proof,
5. seam and owner requirements,
6. update triggers and closure rules.

This protocol does not define:

1. the domain semantics of any one runtime family,
2. the detailed law of `TaskFlow`, `DocFlow`, or protocol binding by itself,
3. process-only status notes that are not part of canonical product law.

## 2. Why Functional Matrices Exist

Functional matrices exist to make layered capability maps operational rather than decorative.

Without one canonical matrix protocol:

1. matrices drift into narrative summaries,
2. status claims mix law closure with implementation closure,
3. layer names stop mapping cleanly to code surfaces,
4. seams between sibling runtime families become implicit,
5. program planning and proof collection lose one bounded audit surface.

Compact rule:

1. matrix rows must steer implementation,
2. matrix rows must support audit,
3. matrix rows must not become abstract architecture theater.

## 3. When A Matrix Is Required

Use a canonical functional matrix when all are true:

1. the capability space is layered or staged,
2. each layer must deliver standalone value,
3. ownership boundaries must remain explicit,
4. implementation will be spread across multiple waves, crates, or donor bridges,
5. later closure depends on proving earlier capability levels.

Typical matrix classes include:

1. runtime capability layers,
2. documentation/inventory capability layers,
3. protocol-binding capability matrices,
4. future retrieval, observability, sync, or gateway capability ladders when they become layered enough to require one canonical progression map.

## 4. Canonical Matrix Outputs

Every canonical functional matrix must provide all of:

1. one top-level status matrix,
2. one expanded section per layer or capability slice,
3. one explicit ownership map,
4. one explicit proof posture,
5. one explicit dependency rule between rows,
6. one explicit current-gap picture.

## 5. Required Matrix Header

Every canonical functional matrix must begin with:

1. `Title`
2. `Status`
3. `Purpose`
4. one top-level matrix table
5. one matrix reading rule
6. one current alignment or compliance snapshot
7. scope and layering rule before expanded row sections begin

## 6. Required Top-Level Matrix Columns

The top-level matrix must expose at least:

1. `layer` or `capability slice`
2. `layer name`
3. `core value`
4. `required implementation`
5. `builds on`
6. `must not depend on`
7. `standalone value`
8. `detail section`

Recommended additional top-level columns when useful:

1. `owner`
2. `law status`
3. `implementation status`
4. `proof status`
5. `main current gap`

## 7. Mandatory Status Split

Every matrix row must keep three different status classes explicit:

1. `law status`
   - whether canonical owner law is closed enough
2. `implementation status`
   - whether the active implementation surface exists and is usable
3. `proof status`
   - whether bounded evidence is green enough to trust the claim

Status rule:

1. one merged status is allowed only in the compact top-level table when the expanded row sections still break the three classes apart,
2. no row may claim closure using only canonical prose when implementation is missing,
3. no row may claim implementation closure without bounded proof.

## 8. Mandatory Row Schema

Every expanded matrix row must define at least:

1. `Purpose`
2. `Owns`
3. `Must Not Own`
4. `Inputs`
5. `Outputs`
6. `Owner Docs`
7. `Owner Code Surface`
8. `Operator Surface`
9. `Proof Surface`
10. `Failure Mode`
11. `Current Gap`
12. `Standalone Value`

### 8.1 Owner Docs

Must list:

1. the canonical owner spec,
2. any adjacent owner spec required for the seam,
3. any higher-precedence law the row may not override.

### 8.2 Owner Code Surface

Must list the current intended implementation home such as:

1. crates,
2. binaries,
3. commands,
4. runtime maps,
5. bounded bridge surfaces when the layer is still donor-backed.

### 8.3 Operator Surface

Must list how the layer becomes inspectable to the operator:

1. command families,
2. status paths,
3. generated artifacts,
4. bounded reports or receipts.

### 8.4 Proof Surface

Must list concrete evidence such as:

1. command proofs,
2. tests,
3. receipts,
4. readiness or doctor surfaces,
5. parity fixtures.

### 8.5 Failure Mode

Must define:

1. how the row fails closed,
2. what the blocking symptom is,
3. which bounded remediation path restores lawful progress.

## 9. Seam Requirement

If a matrix participates in a sibling-runtime or cross-family seam, the matrix must expose:

1. seam trigger,
2. seam input contract,
3. seam output contract,
4. upstream owner,
5. downstream owner,
6. forbidden ownership transfer,
7. closure owner.

Seam rule:

1. seam ownership must never be inferred from narrative,
2. if two matrices meet at one seam, both must name the seam in compatible terms,
3. final closure authority must be explicit even when downstream evidence is required.

## 10. Dependency Rule

Every matrix must make row dependencies explicit.

Required dependency statements:

1. what each row builds on,
2. what each row must not depend on,
3. which shortcuts are forbidden,
4. which later-layer assumptions must not leak backward.

Forbidden pattern:

1. a row may not claim closure by reading evidence from a later unfinished row as if that later row were already active law.

## 11. Layer Closure Rule

A matrix row may be treated as a real layer only when it is capability-complete within its own boundary.

Required layer-closure conditions:

1. the row delivers one bounded, coherent capability bundle rather than a partial fragment,
2. the row has standalone operational value before any later row exists,
3. the row can be inspected through its own bounded operator surface,
4. the row can be trusted through its own bounded proof surface,
5. the row fails closed when its owned prerequisites or proofs are missing,
6. the row does not need a later layer to explain, complete, or legitimize its own owned function.

Closure rule:

1. if a row still requires manual completion from a later row, it is not yet layer-closed,
2. if a row has narrative value but no bounded operator or proof surface, it is not yet layer-closed,
3. if a row owns scattered partial behaviors that do not form one coherent capability, it is not yet a lawful layer.

## 12. Bridge And Migration Rule

When a matrix row is still donor-backed or bridge-backed, the row must say so explicitly.

Minimum migration posture classes:

1. `native_closed`
2. `native_partial`
3. `bridge_backed`
4. `target_only`

Rule:

1. bridge-backed rows may use donor behavior as evidence or continuity support,
2. bridge-backed rows must not hide the fact that native closure is still pending,
3. the matrix must name the bridge surface rather than gesturing vaguely at legacy behavior.

## 13. Update Triggers

A canonical functional matrix must be updated whenever any of the following changes:

1. layer ownership changes,
2. a new owner spec absorbs or takes over row detail,
3. implementation moves to new crates or binaries,
4. operator command surfaces change materially,
5. proof surfaces change materially,
6. seam contracts change,
7. a row changes from bridge-backed to native or vice versa,
8. a row changes closure status.

## 14. Review Rule

Matrix review must ask all of:

1. does each row still map to one real owner,
2. does each row still map to one real code surface,
3. does each row still have bounded proof,
4. are seam contracts still symmetric,
5. are any status claims overstated,
6. is any process detail pretending to be product law.

## 15. Anti-Patterns

The following weaken a functional matrix and are forbidden:

1. one merged status that hides law/implementation/proof differences,
2. a row that becomes useful only after a later layer exists,
3. a row that claims closure without its own operator surface,
4. a row that claims closure without its own proof surface,
5. a row that collects adjacent concerns without forming one coherent capability bundle.
2. rows without owner docs,
3. rows without code mapping,
4. rows without proof,
5. rows that smuggle later-layer authority into earlier layers,
6. seam descriptions that do not name closure owner,
7. donor references without explicit bridge posture,
8. narrative summaries that replace inspectable row contracts.

## 15. Relationship To Current Canon

This protocol currently governs and should strengthen at least:

1. `docs/product/spec/canonical-runtime-layer-matrix.md`
2. `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`
3. capability matrices required by `docs/product/spec/taskflow-protocol-runtime-binding-model.md`
4. future layered capability maps for runtime families or cross-runtime seams

Related authoring references:

1. `docs/product/spec/canonical-layer-documentation-template.md`
2. `docs/product/spec/release-1-plan.md`

Interpretation rule:

1. matrix-specific semantics remain owned by their domain specs,
2. this document owns the protocol for how such matrices must be constructed and maintained.

## 16. Completion Proof

This protocol is working when all are true:

1. new canonical matrices can be authored from one repeatable structure,
2. existing matrices can distinguish law, implementation, and proof cleanly,
3. every matrix row can be traced to owner docs and owner code surfaces,
4. seam rows can no longer drift into implicit ownership,
5. matrix status can guide implementation and review, not just architecture discussion.

## 17. Current Rule

1. functional matrices are canonical control instruments, not decorative architecture summaries,
2. every meaningful row must be owner-linked, code-linked, and proof-linked,
3. law status, implementation status, and proof status must remain explicitly separable,
4. seams between `TaskFlow`, `DocFlow`, and future runtime families must be named and auditable.

-----
artifact_path: product/spec/functional-matrix-protocol
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/product/spec/functional-matrix-protocol.md
created_at: '2026-03-13T08:54:35+02:00'
updated_at: 2026-03-16T08:38:28.77831166Z
changelog_ref: functional-matrix-protocol.changelog.jsonl
