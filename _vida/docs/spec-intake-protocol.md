# Spec Intake Protocol (SIP)

Purpose: normalize raw research findings, issue or release signals, and user scope negotiation into a compact machine-actionable intake artifact before SCP, ICP, or FTP consumes them.

Scope:

1. Canonical intake layer before `_vida/docs/spec-contract-protocol.md`.
2. Canonical intake layer before `_vida/docs/issue-contract-protocol.md` when the incoming issue/release signal is still ambiguous or scope-bearing.
3. Canonical intake layer for mixed research + user clarification flows that need a compact negotiation artifact before task formation.

## Core Principle

Do not route raw research notes, raw issue text, raw release notes, or raw chat clarification directly into downstream spec/task formation when a compact normalized intake artifact can remove ambiguity first.

Normalize first, then route.

## Canonical Artifact

1. `.vida/logs/spec-intake/<task_id>.json`

Minimum fields:

1. `task_id`
2. `intake_class`
3. `source_inputs`
4. `problem_statement`
5. `requested_outcome`
6. `research_findings`
7. `issue_signals`
8. `release_signals`
9. `assumptions`
10. `proposed_scope_in`
11. `proposed_scope_out`
12. `open_decisions`
13. `acceptance_checks`
14. `recommended_contract_path`
15. `status`

## Intake Classes

Normalize each intake into one of:

1. `research`
2. `issue`
3. `release_signal`
4. `user_negotiation`
5. `mixed`

## Status Mapping

`spec_intake.status` must normalize to one of:

1. `ready_for_scp`
   - enough normalized context exists to enter full SCP.
2. `ready_for_issue_contract`
   - the intake is issue-like and narrow enough to continue into ICP.
3. `needs_user_negotiation`
   - scope, assumptions, or acceptance still require explicit user clarification.
4. `needs_spec_delta`
   - non-equivalent change is already visible and must route into spec-delta reconciliation.
5. `insufficient_intake`
   - not enough normalized evidence to continue safely.

## Routing Rule

1. `ready_for_scp` -> `_vida/docs/spec-contract-protocol.md`
2. `ready_for_issue_contract` -> `_vida/docs/issue-contract-protocol.md`
3. `needs_user_negotiation` -> stay in spec-intake / SCP discovery until open decisions are resolved
4. `needs_spec_delta` -> `_vida/docs/spec-delta-protocol.md`
5. `insufficient_intake` -> gather more research/evidence before forming tasks or writer-ready contracts

## Normalization Rules

1. `problem_statement` must describe the user-visible or system-visible problem, not the presumed solution.
2. `requested_outcome` must describe the intended result in one bounded statement.
3. `research_findings` should contain only facts, alternatives, and evidence-backed recommendations that materially affect scope or contract.
4. `issue_signals` should capture reported symptoms, regressions, or bug-like claims without silently promoting them to equivalent fixes.
5. `release_signals` should capture release-driven or milestone-driven pressure that may imply contract change, deprecation, or priority shift.
6. `assumptions` must be explicit; implicit assumptions are invalid intake state.
7. `proposed_scope_in` and `proposed_scope_out` must be present before SCP/FTP negotiation or ICP narrowing continues.
8. `open_decisions` must stay empty before task materialization or writer authorization; if non-empty, the route is still negotiation-bound.

## User Negotiation Rule

When the intake depends on user clarification:

1. capture the proposed scope and assumptions first,
2. turn unresolved questions into `open_decisions`,
3. do not widen scope silently while waiting for clarification,
4. after clarification, rewrite the same intake artifact instead of leaving the resolution only in chat.

## Release / Issue Rule

When incoming data looks like a bug or release note but implies behavioral change:

1. keep that signal in `issue_signals` or `release_signals`,
2. do not mark `ready_for_issue_contract` unless the intake is already narrow and equivalent enough for ICP,
3. otherwise mark `needs_spec_delta` or `needs_user_negotiation`.

## Fail Conditions

Stop downstream formation if any are true:

1. missing intake artifact where SIP is required,
2. `status=needs_user_negotiation` with unresolved `open_decisions`,
3. `status=needs_spec_delta` but no explicit delta-reconciliation path selected,
4. `status=insufficient_intake`,
5. non-empty `proposed_scope_in` is missing while task/spec formation continues.

## Commands

```bash
python3 _vida/scripts/spec-intake.py write <task_id> <input.json> [--output PATH]
python3 _vida/scripts/spec-intake.py validate <task_id> [--path PATH]
python3 _vida/scripts/spec-intake.py status <task_id> [--path PATH]
```
