# Web Validation Protocol (WVP)

Purpose: one canonical protocol for internet research and reality validation.

Scope: mandatory for VIDA flows when external assumptions may affect decisions; part of the orchestrator lean-execution boot read set through `entry.orchestrator-entry.md`, `bootstrap.orchestrator-boot-flow.md`, and `bridge.instruction-activation-protocol.md`; worker lanes use it only when the active packet or activating companion protocol requires bounded external validation; used by `instruction-contracts/overlay.step-thinking-protocol` and `runtime-instructions/work.spec-contract-protocol` as SSOT for step/spec external validation.

## Core Contract

Never finalize a decision that depends on external facts without validation evidence.

Boundary rule:

1. WVP is a validation layer, not a substitute for research artifacts, requirement formation, or specification/intake formation.
2. When a practical validation step depends on earlier research, that research must already be reflected in the current research artifact and the downstream requirement/spec surfaces before WVP becomes the closure-ready validation step.

External facts include package/dependency versions and compatibility; platform behavior (Android/iOS/Flutter/toolchain); API behavior and schema assumptions; security/auth/crypto practices; migration/deprecation guidance; standards/compliance requirements.

## Mandatory Triggers

Run web validation when any trigger is true: unknown build/test/lint/runtime error; selecting/upgrading dependency; API integration or parser contract decision; platform-specific issue (Android/iOS/configuration); security/auth/token/crypto/session decision; migration/deprecation/replacement decision; architecture decision with external best-practice claim.

## Source Quality Policy

Source hierarchy, highest priority first: official documentation/specification/changelog; official repository docs/release notes; vendor-maintained integration guides; secondary explainers/tutorials (supporting only).

Minimum evidence: regular topics need at least 2 independent agreeing sources; security/architecture/compliance need at least 3; at least one primary source whenever available; evidence should be recent and version-compatible.

## Validation Workflow

1. `WVP-0 Trigger Check`
   - identify fired trigger(s) and validation target.
2. `WVP-1 Query Plan`
   - define 2-4 focused queries and output fields.
3. `WVP-2 Evidence Collection`
   - collect URLs and key facts per trigger.
4. `WVP-3 Cross-Source Reconciliation`
   - mark each fact as `agreed`, `conflicting`, or `unknown`.
5. `WVP-4 Live Reality Validation` (when API/server exists)
   - run live requests (`curl` or equivalent), capture status, payload, error body.
6. `WVP-5 Decision Binding`
   - bind decisions/spec text only to validated facts.
7. `WVP-6 Log Evidence`
   - store concise evidence in task logs and user report.

## Completeness Rule

Web validation is incomplete until it states what was checked, what was not checked, what remains unknown/conflicting, and whether remaining gaps are material to the decision.

Fail-closed rule: if material gaps remain, continue validation before closure; do not treat one search result or one agreeing source as full validation; when alternatives or best-practice claims are involved, check competing candidates rather than only the preferred option; validation is closure-ready only when no unresolved material validation questions remain for the active decision; required target is `100% decision-ready confidence`, not partial comfort from a small evidence sample.

Autonomous continuation rule: when WVP is active and evidence remains materially incomplete, continue with the next required validation pass automatically. Do not stop after one source sweep if additional primary sources, competing candidates, or live checks are still required. Pause only when the next validation step would widen scope materially, needs user credentials/paid access/privileged systems, or the user explicitly asked to stop after the current pass.

Research-ordering rule: do not use web validation as a shortcut around unfinished upstream research synthesis. If the active question still lacks updated research artifact, explicit requirements, or refreshed intake/spec, pause closure and return upstream unless the web check is itself the missing research step. Practical validation is lawful only after the bounded question has been translated into updated research state, explicit requirements, and updated spec/intake or equivalent contract artifact.

## API Reality Validation (Mandatory for server/API assumptions)

Use real requests before closing assumptions.

Checklist: endpoint/method verified; auth mode verified; request payload shape verified; success response shape verified; error response/body verified; mismatches documented as `conflict` and reflected in spec.

## Evidence Format (Operational)

When a WVP trigger fired, include compact evidence in TaskFlow logs (`block-end` evidence or `reflect` evidence):

```text
WVP:
- trigger: <api|package|security|migration|platform|error>
- sources:
  - <url1>
  - <url2>
- agreement: <agreed|conflicting|partial>
- live_check: <n/a|curl ok|curl mismatch>
- decision_impact: <what changed in spec/plan/implementation>
```

For API tasks, include one live summary:

```text
LIVE:
- method: <GET|POST|...>
- url: <endpoint>
- status: <code>
- response_shape: <keys/contract>
- error_shape: <keys/contract>
```

Structured marker shortcut:

```bash
bash wvp-evidence.sh record <task_id> <trigger> <agreement> <live_check> <decision_impact> [sources_csv]
bash wvp-evidence.sh not-required <task_id> <reason>
```

`quality-health-check.sh` treats these markers as canonical WVP runtime evidence.

## Confidence Impact

Confidence must be downgraded when evidence is weak: no primary source; conflicting sources unresolved; stale docs for current version; no live API check despite API assumption; claim copied from secondary source only.

Recommended bands: `>= 85` ready; `70..84` conditional with explicit risks; `< 70` not ready.

## Integration Map

1. `instruction-contracts/overlay.step-thinking-protocol#section-web-search`: router-level trigger map and algorithm integration.
2. `runtime-instructions/work.spec-contract-protocol`: SCP gates and weighted readiness model.
3. `command-instructions/execution.implement-execution-protocol`: execution-time validation before code decisions.

## Fail Conditions

Stop and request clarification/evidence if trigger fired but no reliable sources were found, source conflict changes expected behavior, live API contradicts specification, or security claim has no primary source support.

-----
artifact_path: config/runtime-instructions/web-validation.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.web-validation-protocol.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: 2026-07-03T14:40:00+03:00
changelog_ref: work.web-validation-protocol.changelog.jsonl
protocol_authoring_gate: enforced
protocol_compression_status: audit_passed
protocol_compression_algorithm: evidence-list-compaction+trigger-atom-preserve-exact+gate-preserve-exact
protocol_compression_baseline_ref: 3aefbd5b8:vida/config/instructions/runtime-instructions/work.web-validation-protocol.md
protocol_compression_audit_at: 2026-07-03T14:40:00+03:00
protocol_compression_before_tokens: 1631
protocol_compression_after_tokens: 1621
protocol_compression_content_sha256: 6ce79390da5d366589e35959d791da9e80ba975b38d222075e04b9bc57dbbca5
