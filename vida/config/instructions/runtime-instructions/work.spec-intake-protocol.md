# Spec Intake Protocol (SIP)

Purpose: normalize raw research findings, issue or release signals, and user scope negotiation into a compact machine-actionable intake artifact before SCP, ICP, or FTP consumes them.

Scope:

1. Canonical intake layer before `runtime-instructions/work.spec-contract-protocol`.
2. Canonical intake layer before `runtime-instructions/bridge.issue-contract-protocol` when the incoming issue/release signal is still ambiguous or scope-bearing.
3. Canonical intake layer for mixed research + user clarification flows that need a compact negotiation artifact before task formation.

## Core Principle

Do not route raw research notes, raw issue text, raw release notes, or raw chat clarification directly into downstream spec/task formation when a compact normalized intake artifact can remove ambiguity first.

Normalize first, then route.

Research-to-spec progression rule:

1. When a bounded decision depends on fresh research, the lawful sequence is:
   - complete the bounded research pass,
   - update the living research artifact,
   - derive explicit requirements from that research,
   - normalize those requirements into this intake/spec layer,
   - only then continue into practical validation, SCP, ICP, FTP, or implementation-facing work.
2. Do not treat raw findings as a substitute for requirements.
3. Do not treat requirements as sufficient without an updated intake/spec artifact when downstream work still depends on negotiated scope or contract shape.
4. When related findings are spread across multiple artifacts or passes, consolidate them into a thematic research/spec surface before downstream practical continuation.

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

1. `ready_for_scp` -> `runtime-instructions/work.spec-contract-protocol`
2. `ready_for_issue_contract` -> `runtime-instructions/bridge.issue-contract-protocol`
3. `needs_user_negotiation` -> stay in spec-intake / SCP discovery until open decisions are resolved
4. `needs_spec_delta` -> `runtime-instructions/work.spec-delta-protocol`
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

## Research Sufficiency Rule

When `research_findings` materially influence scope, contract, or recommendation:

1. record whether the research pass was comprehensive enough for the current decision,
2. record what evidence classes were covered:
   - research docs,
   - spec docs,
   - code/runtime evidence,
   - web validation,
   - adjacent protocol or process surfaces,
3. record what remains open or conflicting,
4. if those gaps are material, route stays incomplete and more research is required before downstream formation.
5. downstream formation is lawful only when the active intake can point to `100% decision-ready confidence` for the bounded decision it is trying to support.
6. if unresolved material research questions still exist, keep the intake incomplete rather than promoting it by optimism or momentum.

Autonomous continuation rule:

1. If intake remains incomplete because material research gaps still exist, continue the next required research/validation pass automatically rather than handing off prematurely.
2. Do not promote the intake to downstream formation while required research is merely deferred by convenience.
3. Do not promote the intake to practical validation or implementation-facing work until the intake reflects the latest research-backed requirements for the bounded question.

Requirements rule:

1. If research findings materially affect scope, acceptance, routing, or design, the intake must include explicit requirement statements or requirement-ready acceptance checks derived from those findings.
2. Missing requirement formation keeps the intake incomplete even when evidence collection itself is strong.
3. Missing spec/intake refresh after requirement changes keeps the route incomplete even when the previous intake had been valid earlier.

Consolidation rule:

1. Intake/spec formation must point back to a coherent thematic source, not to a pile of disconnected notes.
2. If the bounded topic has become fragmented across multiple research artifacts, create or refresh a consolidated topic artifact before marking the intake ready.
3. Fragmented evidence without consolidation keeps the intake incomplete when that fragmentation materially weakens decision clarity.

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
python3 spec-intake.py write <task_id> <input.json> [--output PATH]
python3 spec-intake.py validate <task_id> [--path PATH]
python3 spec-intake.py status <task_id> [--path PATH]
```

-----
artifact_path: config/runtime-instructions/spec-intake.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.spec-intake-protocol.md
created_at: '2026-03-08T02:15:22+02:00'
updated_at: '2026-03-11T13:04:07+02:00'
changelog_ref: work.spec-intake-protocol.changelog.jsonl
