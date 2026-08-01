# Versioned Rhai Policy Runtime Design Document

Status: `draft`

## Summary
- Feature / change: Versioned, receipt-bearing Rhai policy evaluation with explicit Rust, YAML, and DB authority boundaries.
- Owner layer: `runtime-family`
- Runtime surface: `other`
- Status: `draft; design-only micro-step for rhai-policy-runtime-authority-contract-20260727`

## Current Context
- Rust is the trusted runtime and must retain control of parsing, validation, capabilities, effects, persistence, and operator-visible verdicts.
- Rhai is a bounded decision language; YAML supplies reviewed declarations and limits; the DB stores the canonical selected version, lifecycle state, pins, and receipts.
- The missing contract is an explicit versioned lifecycle for shadow evaluation, activation, failover, promotion, rollback, and resumed work.

## Goal
- Define one authority model for six core Rhai policy IDs plus the `rhai.runtime.quality-gate` family, with explicit modes and version/digest identity.
- Make policy changes observable, promotable only through proof gates, atomically reversible, and safe for pinned task/run resume.
- Out of scope: implementing the Rhai engine, changing Rust code, migrating the DB, changing YAML, or changing TaskFlow/runtime state in this design step.

## Requirements

### Functional Requirements
- Every evaluation names policy ID, version, content digest, mode, input snapshot, verdict, and receipt ID.
- `shadow` evaluates without authorizing effects; `active` may contribute only through the Rust-owned enforcement boundary.
- A resumed run uses its persisted policy pin; it never silently follows the current active version.

### Non-Functional Requirements
- Deterministic, bounded execution with time, memory, input-size, and instruction limits.
- Stable diagnostics for missing, invalid, stale, divergent, or unavailable policy versions.
- No policy may grant capabilities, bypass ownership, or write state outside Rust-controlled APIs.

## Ownership And Canonical Surfaces
- Project docs / specs affected: this design; later ADR and operations runbook linked below.
- Framework protocols affected: runtime readiness, fail-closed execution, receipts, and continuation/pinned-resume contracts.
- Runtime families affected: Rust policy host, Rhai evaluator, YAML configuration loader, and DB-backed policy registry.
- Config / receipts / runtime surfaces affected: versioned policy manifest, DB active-pointer/pin records, shadow comparison receipts, promotion/rollback receipts.

## Design Decisions

### 1. Rust is the enforcement authority
Will implement / choose:
- Rust owns schema validation, compilation admission, host API allowlisting, capability checks, side effects, transactions, timeouts, and final authorization.
- Rhai returns a typed decision; it cannot perform I/O, mutate DB state, select its own version, or bypass Rust checks.
- YAML is declarative input and review material, not runtime authority; the DB is authoritative for the selected immutable version and lifecycle state.
- Trade-off: more Rust boundary code in exchange for deterministic fail-closed behavior and auditable promotion.
- Alternatives considered: YAML-only or script-self-authorized execution; rejected because neither provides durable selection, pinning, or enforcement.
- ADR link if this must become a durable decision record: `docs/product/decisions/versioned-rhai-policy-runtime-authority-adr.md` (later; not part of this micro-step).

### 2. Policy versions use shadow-first promotion
Will implement / choose:
- Each policy version progresses `candidate -> shadow -> promotable -> active -> retired|rolled_back|quarantined`.
- Shadow and active are mutually explicit per policy ID; no implicit mode conversion is allowed.
- Trade-offs: shadow consumes evaluation capacity and delays activation, but exposes divergence before effects are authorized.
- Alternatives considered: direct activation and latest-YAML selection; rejected as unsafe and non-reproducible.
- ADR link if needed: `docs/product/decisions/versioned-rhai-policy-runtime-authority-adr.md` (later).

### 3. Quality-gate policy family

The quality-gate family uses the single policy ID `rhai.runtime.quality-gate` and
typed, versioned inputs and outputs:

```text
QualityGateContextV1 {
  schema_version, task_id, policy_id, policy_version, content_digest,
  profile_id, mode, baseline_verdict, inputs_digest, capability_snapshot,
  limits, pin, receipt_id
}
QualityGateDecisionV1 {
  schema_version, decision_id, policy_id, policy_version, content_digest,
  profile_id, recommendation, additive_profiles, blockers, evidence_refs,
  receipt_id, deterministic_digest
}
```

Typed-field contract: `schema_version` is `u16` and equals `1`; task, policy,
profile, mode, recommendation, blocker, and evidence IDs are bounded enums or
non-empty UTF-8 strings (maximum 128 bytes); versions and limits are bounded
unsigned integers; digests are exactly 64 lowercase hexadecimal characters;
capability snapshots and limits are bounded typed maps; pins are optional but
immutable when present; `receipt_id` is a non-empty bounded identifier.
Unknown fields, enum values, oversized strings/maps, invalid digest shape, or
type mismatch fail closed before evaluation. Rust computes and validates
`effective_profiles = Rust_required ∪ explicit_profiles ∪ Rhai_additions`;
Rhai additions are additive-only and may never remove `Rust_required` or
`explicit_profiles`.

`recommendation` is restricted to `no_change`, `additive_profile`, or `block`.
Rhai may recommend additive profiles and rationale only; it cannot mark evidence
passed, remove a required profile, grant capabilities, or produce an effect.

| Profile ID | Minimum owner | Rhai responsibility | Rust responsibility |
|---|---|---|---|
| `contract` | Rust | recommend additive checks | validate schema and final verdict |
| `security` | Rust | recommend additive checks | enforce mandatory security checks |
| `a11y` | Rust | recommend additive checks | enforce required accessibility checks |
| `visual` | Rust | recommend additive checks | validate artifacts and thresholds |
| `performance` | Rust | recommend additive checks | enforce budgets and evidence |
| `resilience` | Rust | recommend additive checks | enforce failure/recovery checks |
| `property` | Rust | recommend additive checks | validate generated cases and evidence |
| `observability` | Rust | recommend additive checks | require receipt and telemetry evidence |

The quality-profile rollout is nested under the policy lifecycle and is distinct
from `candidate -> shadow -> promotable -> active`; its only legal sequence is
`off -> shadow -> additive_canary -> active`, and direct jumps are invalid.
`off` is Rust-only, `shadow` compares without authority,
`additive_canary` accepts only Rust-validated additions, and `active` still
requires the Rust final verdict. Every evaluation, promotion, activation,
failover, rollback, and resume writes a receipt. A missing or incompatible pin
fails closed with `policy_pinned_bundle_missing`; fallback is limited to the
receipt-backed last-known-good bundle or an immutable Rust baseline. Run pins
are immutable and rollback never rewrites them.

Quality-gate fail-closed triggers are unknown policy/profile IDs, schema/type
mismatch, oversized context or values, evaluator timeout, sandbox error, invalid
Rhai output, missing/incompatible pin, or receipt failure; Rust falls back only
to last-known-good or immutable baseline. Shadow receipts contain policy
identity, input/output digests, duration, agreement/diff, and error/fallback
codes only; never raw context, secrets, or arbitrary Rhai output.

The cross-document authority matrix is canonical and must remain identical in
the ADR and ZOMBIE-D protocol:

| Policy ID | Reviewed additive profiles | Rust-required authority |
|---|---|---|
| `rhai.runtime.authority` | `contract`, `security` | registry, schema, capability and final verdict |
| `rhai.runtime.lifecycle` | `contract`, `resilience`, `observability` | state transitions, persistence and receipts |
| `rhai.runtime.failover` | `resilience`, `observability` | last-known-good/baseline selection and blocking |
| `rhai.runtime.promotion` | `contract`, `performance`, `property`, `observability` | gate admission and activation |
| `rhai.runtime.rollback` | `resilience`, `security`, `observability` | atomic pointer change and quarantine |
| `rhai.runtime.pinned-resume` | `contract`, `resilience`, `property`, `observability` | immutable pin resolution and compatibility |
| `rhai.runtime.quality-gate` | `contract`, `security`, `a11y`, `visual`, `performance`, `resilience`, `property`, `observability` | quality profile registry, additive decisions and final verdict |

These are reviewed additive defaults; the Rust-required profile baseline is
always retained even when the manifest declares a narrower additive set.

## Technical Design

### Core Components
- Rust policy host: validates manifests, compiles/evaluates Rhai, enforces limits/capabilities, and emits verdicts/receipts.
- Rhai policy bundle: versioned pure decision functions behind an allowlisted ABI.
- YAML manifest: human-reviewed policy source, declared dependencies, limits, and rollout intent.
- DB registry: immutable versions/digests, lifecycle state, active pointer, last-known-good pointer, run pins, and promotion/rollback receipts.

### Data / State Model
- Policy identity is `(policy_id, version, content_digest)`; a digest mismatch is invalid, not a new interpretation.
- Six IDs and modes:

| Policy ID | Responsibility | Shadow mode | Active mode |
|---|---|---|---|
| `rhai.runtime.authority` | authority/claim decision | compare only | Rust validates and enforces |
| `rhai.runtime.lifecycle` | candidate and state-transition rules | observe transitions | Rust commits allowed transitions |
| `rhai.runtime.failover` | unavailable/error fallback choice | simulate fallback | Rust selects last-known-good/baseline |
| `rhai.runtime.promotion` | promotion eligibility verdict | calculate gate result | Rust admits only passed gates |
| `rhai.runtime.rollback` | rollback/quarantine recommendation | simulate recovery | Rust atomically changes pointer |
| `rhai.runtime.pinned-resume` | resume compatibility decision | compare against pin | Rust requires the persisted pin |

- Lifecycle requires valid manifest, compiled digest, dependency closure, bounded resource profile, and shadow receipt before `promotable`.
- Promotion requires clean compile/evaluation, deterministic replay, no forbidden capability, acceptable shadow parity, complete receipts, and explicit operator/runtime gate evidence.
- Failover uses the last-known-good DB version or immutable Rust baseline only; if neither is valid, the operation blocks.
- Rollback atomically restores the prior pointer, records the reason and receipt, quarantines the failed version, and leaves pins unchanged.
- Pinned resume resolves the exact `(policy_id, version, digest)` stored with the run; missing or incompatible pins fail closed and require explicit recovery.

### Integration Points
- Rust host calls Rhai only with validated snapshots and a fixed ABI, then re-checks every returned action.
- YAML publication creates candidate metadata; it does not change the active DB pointer.
- DB transactions serialize promotion, activation, rollback, and pin writes; receipts make each transition replayable.
- Later ADR and runbook must link to this design and preserve the same IDs, states, and gate vocabulary.

### Bounded File Set
- TaskFlow-bounded docs:
  - `docs/product/spec/versioned-rhai-policy-runtime-design.md`
  - `docs/product/decisions/versioned-rhai-policy-runtime-authority-adr.md`
  - `docs/process/rhai-policy-authoring-runbook.md`
- All three quality-gate authority documents are in this bounded step; the authoring runbook is not changed here.
- No Rust, Rhai, YAML, DB, index, changelog, TaskFlow, or runtime file is in scope for this docs step.

## Fail-Closed Constraints
- Reject unknown IDs, duplicate versions, digest mismatches, invalid YAML, unsupported ABI, missing dependencies, excessive resource declarations, and nondeterministic inputs.
- Shadow output cannot authorize effects; active output cannot bypass Rust validation, capability checks, ownership, transaction boundaries, or receipt requirements.
- Never select the latest file, latest DB row, unpinned policy, or ad hoc script as fallback.
- On evaluator timeout, panic, unavailable DB, missing receipt, parity drift, or failed gate, block and expose the exact recovery state.
- Do not mutate active pointers, pins, or runtime state without an atomic Rust-owned transaction and receipt.

## Implementation Plan

### Phase 1
- Define typed Rust host ABI, manifest schema, six IDs, immutable version/digest records, and shadow receipts.
- Proof target: parser/validator and deterministic shadow evaluation fixtures.

### Phase 2
- Add DB lifecycle transitions, active-pointer transactions, promotion gates, failover, rollback, and pinned-resume resolution.
- Proof target: replay, crash/retry, parity, rollback, and pinned-resume integration matrix.

### Phase 3
- Add operator surfaces, metrics, runbook procedures, staged rollout, and compatibility cleanup.
- Proof target: end-to-end fail-closed behavior across every lifecycle and mode transition.

## Validation / Proof
- Unit tests: manifest/ID validation, digest identity, limits, typed verdicts, and state-machine transitions.
- Integration tests: YAML-to-candidate import, DB promotion/rollback transactions, shadow/active parity, failover, and pinned resume.
- Runtime checks: timeout/panic/DB-loss blocking, capability denial, receipt completeness, and deterministic replay.
- Canonical checks:
  - `activation-check`
  - `protocol-coverage-check`
  - `check`
  - `doctor`

## Observability
- Log policy ID/version/digest, mode, run pin, gate verdict, fallback reason, and receipt ID without logging secrets or arbitrary script output.
- Count shadow divergence, active denials, evaluator failures, promotion failures, failovers, rollbacks, and pinned-resume blocks.
- Persist append-only transition receipts and retain the prior last-known-good pointer.

## Rollout Strategy
- Register and validate all six policies in `shadow`; compare against the current Rust decision path.
- Promote one policy/version at a time after replay, parity, resource, security, and operator gates pass.
- Activate by an atomic DB pointer change; monitor receipts and divergence; rollback to last-known-good on any failed invariant.
- Existing runs remain pinned; new runs use the active bundle only after the activation receipt is durable.

## Future Considerations
- Follow-up ADR at `docs/product/decisions/versioned-rhai-policy-runtime-authority-adr.md` to formalize authority and compatibility guarantees.
- Follow-up runbook at `docs/process/rhai-policy-authoring-runbook.md` for promotion, rollback, incident response, and pinned-resume repair.
- Known limitation: policy behavior remains bounded by the Rust ABI and cannot express new capabilities without a Rust change.

## References
- `docs/product/spec/templates/feature-design-document.template.md`
- `docs/product/spec/feature-design-and-adr-model.md`
- `docs/product/spec/canonical-runtime-layer-matrix.md`
- `docs/product/spec/canonical-runtime-readiness-law.md`
- `docs/product/research/db-authority-and-migration-runtime-research.md`
- Later ADR: `docs/product/decisions/versioned-rhai-policy-runtime-authority-adr.md`
- Later runbook: `docs/process/rhai-policy-authoring-runbook.md`

-----
artifact_path: product/spec/versioned-rhai-policy-runtime-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-07-27'
schema_version: '1'
status: canonical
source_path: docs/product/spec/versioned-rhai-policy-runtime-design.md
created_at: '2026-07-27T00:00:00+03:00'
updated_at: '2026-07-27T00:00:00+03:00'
changelog_ref: versioned-rhai-policy-runtime-design.changelog.jsonl
