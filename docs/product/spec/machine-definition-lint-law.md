# VIDA Machine Definition Lint Law

Status: draft `v1` bounded validation artifact

Revision: `2026-03-09`

Purpose: define future static validation rules for `vida/config/machines/*` so invalid machine graphs fail before runtime mutation.

## 1. Scope

This artifact defines future lint rules for:

1. state graph reachability,
2. terminal/final-state correctness,
3. event/alias conflicts,
4. overlapping transition ambiguity,
5. checkpoint/gateway graph sanity.

It does not define:

1. runtime event handling,
2. vendor DSL validation,
3. speculative optimization passes,
4. automatic graph repair.

## 2. Candidate Lint Classes

### 2.1 Structural Validity

Machines should fail lint when:

1. there is no unique initial posture,
2. states or statuses are unreachable from the initial posture,
3. a non-terminal posture has no lawful outgoing transition when one is required by the machine family,
4. a declared terminal/final posture has outgoing transitions,
5. a machine with terminal states leaves non-terminal states with no path to any terminal state.

### 2.2 Transition Ambiguity

Machines should fail lint when:

1. the same event/alias combination creates ambiguous competing transitions for the same source posture,
2. `from_any` creates illegal shadowing over a more specific transition where precedence would be non-obvious,
3. back-edges bypass canonical guards or receipts,
4. event aliases collide across incompatible semantics.

### 2.3 Route And Gateway Sanity

Machines should fail lint when:

1. gateway-like postures are encoded into the wrong machine ownership surface,
2. checkpoint-required transitions cannot produce a lawful resume target,
3. route-stage machines redefine frozen stage vocabulary,
4. verification/approval/coach distinctions collapse into one state axis.

## 3. Severity Bands

Lint outputs should separate:

1. `error` for invalid graphs that must fail closed,
2. `warning` for risky but currently legal patterns,
3. `advisory` for maintainability or clarity concerns.

Examples:

1. unreachable state -> `error`
2. non-terminal dead end -> `error`
3. alias near-collision that is still distinguishable -> `warning`
4. excessive `from_any` usage -> `advisory`

## 4. Evidence And Output Shape

Future lint output should include:

1. `machine`
2. `rule_id`
3. `severity`
4. `message`
5. `state_or_transition_ref`
6. `suggested_fix_class`

## 5. Invariants

1. lint validates product-law machine definitions, not runtime vendor adapters
2. lint must fail closed on graph-invalid machine law
3. lint may recommend refactors but must not mutate machine specs automatically
4. lint must preserve frozen `task_lifecycle` and `route_progression` vocabularies

-----
artifact_path: product/spec/machine-definition-lint-law
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/product/spec/machine-definition-lint-law.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-09T20:28:59+02:00
changelog_ref: machine-definition-lint-law.changelog.jsonl
