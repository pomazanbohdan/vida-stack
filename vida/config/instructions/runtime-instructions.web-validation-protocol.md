# Web Validation Protocol (WVP)

Purpose: one canonical protocol for internet research and reality validation.

Scope:

1. Mandatory for all VIDA flows when external assumptions may affect decisions.
2. Mandatory read during boot (`AGENTS.md` LEAN/STANDARD/FULL BOOT).
3. Used by `vida/config/instructions/instruction-contracts.thinking-protocol.md` and `vida/config/instructions/runtime-instructions.spec-contract-protocol.md` as single source.

## Core Contract

Never finalize a decision that depends on external facts without validation evidence.

External facts include:

1. package/dependency versions and compatibility,
2. platform behavior (Android/iOS/Flutter/toolchain),
3. API behavior and schema assumptions,
4. security/auth/crypto practices,
5. migration/deprecation guidance,
6. standards/compliance requirements.

## Mandatory Triggers

Run web validation when at least one trigger is true:

1. unknown build/test/lint/runtime error,
2. selecting or upgrading a dependency,
3. API integration or parser contract decisions,
4. platform-specific issue (Android/iOS/configuration),
5. security/auth/token/crypto/session decisions,
6. migration/deprecation/replacement decisions,
7. architecture decision with external best-practice claim.

## Source Quality Policy

Source hierarchy (highest priority first):

1. official documentation/specification/changelog,
2. official repository docs/release notes,
3. vendor-maintained integration guides,
4. secondary explainers/tutorials (supporting only).

Minimum evidence:

1. regular topics: at least 2 independent agreeing sources,
2. security/architecture/compliance: at least 3 sources,
3. at least one primary source whenever available,
4. evidence should be recent and version-compatible.

## Validation Workflow

1. `WVP-0 Trigger Check`
   - identify which trigger(s) fired and what must be validated.
2. `WVP-1 Query Plan`
   - define 2-4 focused queries and expected output fields.
3. `WVP-2 Evidence Collection`
   - collect URLs and extract key facts for each trigger.
4. `WVP-3 Cross-Source Reconciliation`
   - mark each fact as `agreed`, `conflicting`, or `unknown`.
5. `WVP-4 Live Reality Validation` (when API/server exists)
   - run live requests (`curl` or equivalent), capture status, payload, and error body.
6. `WVP-5 Decision Binding`
   - bind decisions/spec text only to validated facts.
7. `WVP-6 Log Evidence`
   - store concise evidence in task logs and user report.

## API Reality Validation (Mandatory for server/API assumptions)

Use real requests before closing assumptions.

Checklist:

1. endpoint/method verified,
2. auth mode verified,
3. request payload shape verified,
4. success response shape verified,
5. error response/body verified,
6. mismatchs documented as `conflict` and reflected in spec.

## Evidence Format (Operational)

When a WVP trigger fired, include a compact evidence block in TaskFlow logs (`block-end` evidence or `reflect` evidence):

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

For API tasks, include one live snippet summary:

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

`quality-health-check.sh` treats these markers as canonical WVP evidence for runtime validation.

## Confidence Impact

Confidence must be downgraded when evidence is weak:

1. no primary source,
2. conflicting sources unresolved,
3. stale docs for current version,
4. no live API check despite API assumption,
5. claim copied from secondary source only.

Recommended bands:

1. `>= 85`: ready,
2. `70..84`: conditional (explicit risks),
3. `< 70`: not ready.

## Integration Map

1. `vida/config/instructions/instruction-contracts.thinking-protocol.md#section-web-search`: router-level trigger map and algorithm integration.
2. `vida/config/instructions/runtime-instructions.spec-contract-protocol.md`: SCP gates and weighted readiness model.
3. `vida/config/instructions/command-instructions.implement-execution-protocol.md`: execution-time validation before code decisions.

## Fail Conditions

Stop and request clarification/evidence if:

1. trigger fired but no reliable sources found,
2. source conflict changes expected behavior,
3. live API contradicts specification,
4. security claim has no primary source support.

-----
artifact_path: config/runtime-instructions/web-validation.protocol
artifact_type: runtime_instruction
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/runtime-instructions.web-validation-protocol.md
created_at: 2026-03-06T22:42:30+02:00
updated_at: 2026-03-10T00:55:00+02:00
changelog_ref: runtime-instructions.web-validation-protocol.changelog.jsonl
