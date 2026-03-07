# Issue Contract Protocol (ICP)

Purpose: normalize raw issues, bug reports, and regression notes into a machine-checked execution contract before any writer lane starts.

Scope:

1. Canonical bridge for `bug-as-spec` style requests.
2. Applies when the incoming issue text is the main spec input or when analysis must decide whether a reported bug is an equivalent fix versus a spec/product-contract delta.
3. Works with `_vida/docs/bug-fix-protocol.md`, `_vida/docs/implement-execution-protocol.md`, and `_vida/docs/subagent-system-protocol.md`.

## Core Principle

Do not treat raw bug text as writer-ready by default.

Normalize it into `issue_contract` first, then decide whether the writer may proceed.

## Intake Classes

The analysis lane must classify each issue as one of:

1. `defect_equivalent`
2. `defect_needs_contract_update`
3. `feature_delta`
4. `as_designed`
5. `not_a_bug`
6. `insufficient_evidence`

## Required Artifact

Canonical runtime artifact:

1. `.vida/logs/issue-contracts/<task_id>.json`

Minimum fields:

1. `classification`
2. `equivalence_assessment`
3. `reported_behavior`
4. `expected_behavior`
5. `scope_in`
6. `scope_out`
7. `acceptance_checks`
8. `spec_sync_targets`
9. `wvp_required`
10. `wvp_status`
11. `status`
12. `resolution_path`

## Status Mapping

`issue_contract.status` must normalize to one of:

1. `writer_ready`
   - equivalent fix; writer may proceed.
2. `spec_delta_required`
   - behavior/spec/product contract must be reconciled before the writer starts.
3. `issue_closed_no_fix`
   - `as_designed` or `not_a_bug`; no writer lane.
4. `insufficient_evidence`
   - more evidence, reproduction, or research is required before any writer lane.

## Equivalence Gate

Before writer authorization:

1. analysis receipt must exist when the route requires analysis,
2. `issue_contract` must exist,
3. `issue_contract.status` must be `writer_ready`.

If any item fails, writer authorization is blocked.

## WVP / Internet Validation

If the issue classification or expected behavior depends on external facts:

1. run `_vida/docs/web-validation-protocol.md`,
2. mark `wvp_required=yes`,
3. record `wvp_status`,
4. do not mark `writer_ready` while WVP is still `conflicting` or `unknown`.

## Prompt Rendering Rule

For issue-driven implementation:

1. analysis produces `issue_contract`,
2. writer prompt must be rendered from the original request plus normalized `issue_contract`,
3. rework handoffs must stay rooted in the original request/contract plus coach delta, not prior writer context.

## Coach And Verifier

1. Default coach policy remains two independent cheaper coaches when eligible.
2. Coach may return the implementation for rework, but it does not rewrite the original issue contract.
3. If coach discovers a non-equivalent product/spec change, the route must fail closed and reopen spec reconciliation instead of silently continuing.

## Spec Sync

Spec synchronization happens in two distinct moments:

1. pre-implementation:
   - when `issue_contract.status=spec_delta_required`
2. post-fix:
   - when a completed equivalent fix changes or clarifies the documented contract

## Fail Conditions

Stop execution if any is true:

1. missing `issue_contract`,
2. stale `issue_contract` versus current route receipt,
3. conflicting equivalence decision,
4. `spec_delta_required` without spec reconciliation,
5. `insufficient_evidence` without new evidence path.
