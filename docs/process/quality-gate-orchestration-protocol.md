# Quality-Gate Orchestration Protocol

Status: canonical

## Purpose

Define a single-pass quality workflow that establishes a baseline, evaluates
ordered gates, records defects once, repairs in a bounded batch, and reruns the
same evidence set. The protocol applies to code, configuration, scripts,
documentation, CI, release, and operator-surface changes.

This is a process contract. It does not replace domain-specific test, release,
security, or documentation law; those laws supply the checks selected by the
gates below.

## Core flow

Run one coherent pass in this order:

`baseline → gates → defect ledger → batch repair → rerun`

1. **Baseline:** freeze the change scope, record the starting revision and
   environment, and run the cheapest admissible discovery checks once.
2. **Gates:** execute G0–G12 in order. A gate may consume prior artifacts but
   must not silently broaden the file, tool, or environment scope.
3. **Defect ledger:** normalize every finding into one row before changing
   anything. Deduplicate by invariant and owning surface, not by message text.
4. **Batch repair:** repair all ledger rows in the same bounded ownership area;
   serialize overlapping writes and keep independent read-only checks parallel.
5. **Rerun:** repeat the failed gate and all downstream gates using the same
   selectors and environment. A changed selector is a new baseline.

No gate is considered green from an unrecorded manual inspection. Preserve raw
stdout/stderr or an artifact reference whenever the default output is compacted.

## Gate sequence G0–G12

| gate | name | required evidence | fail/hold condition |
| --- | --- | --- | --- |
| G0 | Scope and ownership | changed-path set, owner, acceptance target, risk tier | scope or owner is ambiguous |
| G1 | Baseline reproducibility | clean starting result, revision, tool versions, environment id | baseline cannot be reproduced or is stale |
| G2 | Input and contract integrity | parser/schema/link/front-matter checks relevant to the target | malformed input or contract drift |
| G3 | Static quality | formatter, linter, type/compiler, spelling or markdown checks selected by tool matrix | deterministic static error |
| G4 | Unit/fixture proof | focused tests for changed pure behavior and fixtures | focused test failure or missing fixture proof |
| G5 | Integration/public surface | CLI/API/UI/docflow/operator surface behavior and default/explicit modes | public contract mismatch |
| G6 | Negative and boundary proof | blocked, empty, malformed, permission, timeout, and boundary cases as applicable | fail-open behavior or untested high-risk boundary |
| G7 | Cross-surface consistency | parity across callers, formats, platforms, generated outputs, and persisted snapshots | duplicated or contradictory truth |
| G8 | Security and data safety | secret scan, authorization/data-handling checks, migration/rollback evidence when applicable | security/data risk unresolved |
| G9 | Performance and operability | timing envelope, output-size/selector check, timeout and artifact paths | unexplained slow/noisy/unbounded operation |
| G10 | Change hygiene | diff review, generated-file status, link/reference closure, no unrelated churn | dirty scope, broken references, or accidental artifacts |
| G11 | Release/CI admission | applicable CI, package, install, deployment, or compatibility proof | required admission check absent or red |
| G12 | Closure and rerun parity | ledger resolved, same selectors rerun, final evidence bundle and residuals | unresolved row, changed proof shape, or missing handoff |

G0–G2 are always required. G3–G10 are selected by risk and surface. G11 is
required for release-affecting changes. G12 is required for every closure.

## Risk thresholds and admission

Assign the highest applicable tier before G2:

| tier | trigger | minimum gates | rerun rule |
| --- | --- | --- | --- |
| R0 routine | docs, comments, isolated non-behavioral text | G0–G3, G10, G12 | rerun changed-file checks |
| R1 behavioral | one module, script, or public surface | G0–G7, G9–G10, G12 | rerun failed gate + downstream gates |
| R2 elevated | auth, persistence, migration, cross-module, CI/release, or user-visible workflow | G0–G10, G12 | rerun full selected set with negative proof |
| R3 critical | security boundary, data loss, installer/runtime replacement, or multi-platform release | G0–G12 | independent review and full admission rerun required |

Escalate one tier when evidence is missing, the same defect recurs, or the
repair changes an owning boundary. A green result cannot lower a tier after the
fact.

## Defect ledger

Use one row per invariant/surface pair:

| field | required value |
| --- | --- |
| id | stable `QG-###` identifier |
| gate | first gate that exposed it |
| invariant | behavior that must remain true |
| surface | file/module/command/format/platform |
| severity | blocker, high, medium, low |
| evidence | command, artifact, fixture, or exact observation |
| verdict | one of the matrix semantics below |
| owner | repair owner and bounded path set |
| repair | intended change, not a speculative workaround |
| rerun | exact gate/selector to repeat |
| residual | accepted follow-up or `none` |

Ledger rules: deduplicate equivalent symptoms; link related rows; never close a
row by deleting its proof; classify infrastructure separately from product
behavior; keep unresolved rows blocking at G12 unless explicitly accepted by
the owning release/risk authority.

## Verdict matrix: Z/O/M/B/I/E/S+R/P/C

The compact verdict alphabet is intentionally explicit so “not green” states
cannot be mistaken for failure or success:

| code | meaning | closure treatment |
| --- | --- | --- |
| Z | zero/clean: check ran and found no issue | pass; retain evidence |
| O | observed: issue or behavior directly reproduced | ledger row required |
| M | missing: required check/evidence was not run or cannot be located | blocker; do not infer pass |
| B | blocked: check could not run because a prerequisite is unavailable | blocker unless infra row is accepted and alternate proof exists |
| I | inconclusive: result is ambiguous, flaky, truncated, or contradictory | rerun or replace proof; never pass by averaging |
| E | equivalent: alternate proof demonstrates the same invariant with equal or stronger strength | acceptable only with cited equivalence and owner approval |
| S+R | success plus regression: target check passes and a related regression is exposed | target is not closed; repair regression and rerun downstream |
| P | partial: bounded subset passes but required coverage is incomplete | remains open; list uncovered selectors |
| C | conditional: passes only under named environment, flag, or assumption | acceptable only when condition is explicit, reproducible, and in scope |

### No-evidence, equivalent, and infrastructure semantics

- **No evidence** is `M`, not `Z`. A claim, screenshot without context, or
  second-hand report is not evidence.
- **Equivalent evidence** is `E` only when the alternate check exercises the
  same invariant, boundary, and failure semantics; a cheaper check that omits a
  boundary is `P` or `M`.
- **Infrastructure failure** is `B` when the product check did not execute
  (tool missing, runner unavailable, dependency outage, permission failure).
  Record the infra cause separately, preserve the raw error, and use an
  approved equivalent only when its strength is documented. Do not relabel a
  product assertion failure as infrastructure.
- `I` applies when the cause cannot yet be separated. Resolve it to `O`, `B`,
  `E`, or `Z` before closure.

## Tool matrix

Select the smallest tool that proves the gate, then retain a stronger tool for
the admission tier when required:

| concern | preferred tool/check | artifact or selector |
| --- | --- | --- |
| scope/diff | Git status/diff, changed-path allowlist | revision, path list, diff check |
| markdown/docs | markdown parser, link checker, front-matter/schema validator | file/anchor selector + raw report |
| Rust/source | project formatter/linter/compiler and focused test runner | package/filter, stdout/stderr artifacts |
| scripts | syntax/check/help/dry-run mode | script path + exit code |
| CLI/operator | default compact output + explicit machine mode + `--help` | selector, JSON artifact, option text |
| fixtures/snapshots | focused fixture/golden/integration test | fixture id and expected diff |
| security | secret scanner, dependency/license/advisory check, auth/data test | finding id and sanitized output |
| performance | timed command, bounded output, timeout probe | duration, bytes/lines, artifact refs |
| CI/release | required workflow, package/install/smoke/compatibility gate | run URL/id, package checksum, logs |
| review | independent diff/ledger review for R2/R3 | reviewer, timestamp, disposition |

Tool substitution is valid only as `E` with an equivalence note. A tool that
cannot preserve raw failure details produces `I` or `B`, never `Z`.

## Closure checklist

Before declaring the quality pass complete:

1. G0–G2 and every risk-selected gate have a result and artifact reference.
2. Every non-`Z` row is resolved, accepted by the correct authority, or carried
   as an explicit blocking follow-up.
3. Batch repairs changed only owned paths and did not weaken a gate.
4. The rerun used the baseline selectors and reached G12.
5. The final bundle names residual risk, conditional assumptions, and any
   infrastructure follow-up.

## Metadata

Artifact family: project process protocol. Loading posture: task-selected for
quality, test, release, documentation, and operator-surface work. Canonical
owner: `docs/process/quality-gate-orchestration-protocol.md`.

-----
artifact_path: process/quality-gate-orchestration-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-08-12'
schema_version: '1'
status: canonical
source_path: docs/process/quality-gate-orchestration-protocol.md
created_at: '2026-08-12T00:00:00+03:00'
updated_at: '2026-08-12T00:00:00+03:00'
