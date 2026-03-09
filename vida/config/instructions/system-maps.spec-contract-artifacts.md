# Spec Contract Artifacts (Templates)

Purpose: standard templates for SCP outputs in `/vida-spec` flow.

## 1) Decision Log Template

```markdown
## Decision Log

- Scope: <scope>
- Session: <date/time>

### D-01 <category>
- Options considered: <A/B/C>
- Selected: <choice>
- Why: <reason>
- Trade-off accepted: <trade-off>

### D-02 <category>
...

### Open decisions
- <item>
```

## 2) API Reality Matrix Template

```markdown
## API Reality Matrix

| Area | Expected | Actual | Status | Evidence |
|---|---|---|---|---|
| Auth | ... | ... | confirmed/conflict/unknown | curl #1 |
| Endpoint X | ... | ... | ... | curl #2 |
| Error body | ... | ... | ... | curl #3 |

Notes:
- <risk or mismatch>
```

## 3) Design Contract Template

```markdown
## Design Contract

### User Flows
- Flow A: <steps>

### State Map
- loading: <behavior>
- empty: <behavior>
- error: <behavior>
- retry: <behavior>

### Components
- Component X: props/state/events
```

## 4) Technical Contract Template

```markdown
## Technical Contract

### Interfaces
- Interface A: input/output/errors

### Data Contracts
- DTO X: fields/types/constraints

### Observability
- Logs: <structured points>
- Metrics: <key counters/timers>
```

## 5) Confidence Scorecard Template

```markdown
## Confidence Scorecard

- user_alignment: <0..100>
- api_reality: <0..100>
- evidence_quality: <0..100>
- architecture_fit: <0..100>
- delivery_readiness: <0..100>

- final_score: <value>
- band: ready | conditional | not_ready
- downgrade_factors: <list>
```

## 6) WVP Evidence Template

```markdown
## Web Validation Evidence (WVP)

### Trigger
- trigger: api | package | security | migration | platform | error
- why_triggered: <short reason>

### Sources
- primary: <url>
- secondary: <url>
- additional (if security/architecture): <url>

### Reconciliation
- agreement: agreed | conflicting | partial
- key_facts:
  - <fact #1>
  - <fact #2>

### Live Validation (if API exists)
- method/url: <GET ...>
- status: <code>
- response_shape: <keys>
- error_shape: <keys>

### Decision Impact
- changed_in_spec: <what changed>
- remaining_risks: <if any>
```

## 7) Draft Execution-Spec Template

```markdown
## Draft Execution-Spec

- scope_in:
  - <what may change>
- scope_out:
  - <what must not change>
- acceptance_checks:
  - <check #1>
  - <check #2>
- assumptions:
  - <assumption #1>
- open_decisions:
  - <decision or none>
- recommended_next_path: /vida-form-task | /vida-bug-fix
```

Canonical helper:

```bash
python3 draft-execution-spec.py write <task_id> <input.json> [--output PATH]
python3 draft-execution-spec.py validate <task_id> [--path PATH]
python3 draft-execution-spec.py status <task_id> [--path PATH]
```

-----
artifact_path: config/system-maps/spec-contract.artifacts
artifact_type: system_map
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/system-maps.spec-contract-artifacts.md
created_at: 2026-03-06T22:42:30+02:00
updated_at: 2026-03-10T00:55:00+02:00
changelog_ref: system-maps.spec-contract-artifacts.changelog.jsonl
