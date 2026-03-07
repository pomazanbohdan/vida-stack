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
2. `.vida/logs/issue-splits/<task_id>.json` when a mixed issue contains both an executable primary slice and unresolved secondary symptoms
3. follow-up task linkage in the issue-split artifact when the unresolved slice should become separately tracked work

Minimum fields:

1. `classification`
2. `equivalence_assessment`
3. `reported_behavior`
4. `expected_behavior`
5. `reported_scope`
6. `proven_scope`
7. `symptoms` for multi-symptom issues
8. `scope_out`
9. `acceptance_checks`
10. `spec_sync_targets`
11. `wvp_required`
12. `wvp_status`
13. `status`
14. `resolution_path`

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
3. `issue_contract.status` must be `writer_ready`,
4. `issue_contract.proven_scope` must be non-empty.

If any item fails, writer authorization is blocked.

## Scope Split Rule

Each `issue_contract` must separate:

1. `reported_scope`
   - the full symptom surface from intake or analysis,
2. `proven_scope`
   - the narrowed executable surface supported by current evidence.

Writer authorization binds only to `proven_scope`.

Unproven claims must stay in `reported_scope` until they are either:

1. reproduced/proven,
2. moved into a later slice,
3. routed to spec/issue reconciliation.

## Symptom Evidence Rule

For multi-symptom issues:

1. each symptom must be represented in `issue_contract.symptoms`,
2. each in-scope symptom must have `evidence_status` in `reproduced|red_test|live_evidence`,
3. symptoms without evidence must be explicitly marked `disposition=out_of_scope` before writer authorization,
4. otherwise `writer_ready` is invalid and must fail closed.

## Mixed-Issue Split Rule

If one issue contains:

1. a proven in-scope executable slice, and
2. secondary unresolved or explicitly out-of-scope symptoms,

the runtime should emit an `issue-split` artifact that preserves:

1. the primary executable slice the writer may implement now,
2. the secondary unresolved slice that should become follow-up work instead of silently re-expanding the current fix.
3. follow-up task identity when runtime materializes the unresolved slice into tracked work.

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
