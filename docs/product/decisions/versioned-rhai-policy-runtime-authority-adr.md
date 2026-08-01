# Versioned Rhai Policy Runtime Authority ADR

Status: accepted for authority and compatibility guarantees.

## Context

The versioned Rhai policy runtime needs one durable authority model for policy
selection, evaluation, lifecycle transitions, rollout, recovery, and resumed
work. The runtime spans four layers with different responsibilities:

- Rust is the trusted runtime boundary for parsing, validation, ABI and host
  API allowlisting, capability checks, resource limits, effects, persistence,
  transactions, and final operator-visible verdicts.
- Rhai is a bounded, deterministic decision language. A policy returns a typed
  decision but cannot perform I/O, mutate the database, select its own version,
  or bypass Rust enforcement.
- YAML contains reviewed declarations, dependencies, limits, and rollout
  intent. YAML publication may create a candidate but cannot select the active
  runtime version.
- The DB is authoritative for immutable policy identity, lifecycle state, the
  active and last-known-good pointers, run pins, and promotion/failover/
  rollback receipts.

The canonical design is
[`docs/product/spec/versioned-rhai-policy-runtime-design.md`](../spec/versioned-rhai-policy-runtime-design.md).

## Decision

Adopt `(policy_id, version, content_digest)` as the immutable policy identity.
Every evaluation and lifecycle transition records the policy identity, mode,
input snapshot, verdict, and receipt ID. Rust owns every state-changing
decision and effect boundary.

The runtime supports six core Rhai policy IDs plus the dedicated
`rhai.runtime.quality-gate` family:

| Policy ID | Responsibility | Shadow mode | Active mode |
|---|---|---|---|
| `rhai.runtime.authority` | authority and claim decision | compare only | Rust validates and enforces |
| `rhai.runtime.lifecycle` | candidate and state-transition rules | observe transitions | Rust commits allowed transitions |
| `rhai.runtime.failover` | unavailable/error fallback choice | simulate fallback | Rust selects last-known-good or baseline |
| `rhai.runtime.promotion` | promotion eligibility verdict | calculate gate result | Rust admits only passed gates |
| `rhai.runtime.rollback` | rollback and quarantine recommendation | simulate recovery | Rust atomically changes the pointer |
| `rhai.runtime.pinned-resume` | resume compatibility decision | compare against pin | Rust requires the persisted pin |
| `rhai.runtime.quality-gate` | additive quality profile recommendation | compare-only or additive recommendation | Rust owns required profiles and final verdict |

`shadow` and `active` are explicit modes; shadow output never authorizes an
effect, and active output contributes only through Rust-owned validation,
capability, ownership, transaction, and receipt checks.

### Lifecycle and recovery

Policy versions progress only through the explicit lifecycle:

`candidate -> shadow -> promotable -> active -> retired|rolled_back|quarantined`.

Admission to `promotable` requires a valid manifest, compiled digest,
dependency closure, bounded resource profile, and durable shadow receipts.
Promotion additionally requires clean compilation/evaluation, deterministic
replay, no forbidden capability, acceptable shadow parity, complete receipts,
and explicit operator/runtime gate evidence.

Activation is an atomic DB pointer change after the activation receipt is
durable. Failover selects the DB last-known-good version or an immutable Rust
baseline. If neither is valid, the operation blocks. Rollback atomically
restores the prior pointer, records reason and receipt, quarantines the failed
version, and leaves existing run pins unchanged.

Resumed work resolves the exact persisted `(policy_id, version, content_digest)`
pin. It never silently follows the current active version. A missing,
incompatible, stale, or digest-mismatched pin fails closed and requires
explicit recovery.

### Hard boundaries

- Unknown IDs, duplicate versions, digest mismatches, invalid YAML, unsupported
  ABI, missing dependencies, excessive resource declarations, and
  nondeterministic inputs are rejected.
- Rhai cannot grant capabilities, bypass ownership, write state, perform I/O,
  mutate active pointers or pins, or introduce effects outside Rust-controlled
  APIs.
- YAML is declaration and review input, never the runtime selection authority.
- DB promotion, activation, rollback, and pin writes require an atomic
  Rust-owned transaction and a durable receipt.
- Evaluator timeout, panic, unavailable DB, missing receipt, parity drift, or a
  failed gate blocks the operation and exposes the recovery state.
- The runtime never falls back to the latest file, latest DB row, an unpinned
  policy, or an ad hoc script.

### Quality-gate authority extension

The quality-gate family is identified only by `rhai.runtime.quality-gate` and
uses `QualityGateContextV1` and `QualityGateDecisionV1`. The context carries the
task, policy pin, profile, mode, input digest, capability snapshot, limits, and
receipt identity. The decision carries a deterministic digest, typed
recommendation (`no_change`, `additive_profile`, or `block`), additive profiles,
blockers, and evidence references.

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

| Profile ID | Rhai may do | Rust must do |
|---|---|---|
| `contract` | recommend additive checks | own schema and final verdict |
| `security` | recommend additive checks | enforce mandatory security checks |
| `a11y` | recommend additive checks | enforce required accessibility checks |
| `visual` | recommend additive checks | validate artifacts and thresholds |
| `performance` | recommend additive checks | enforce budgets and evidence |
| `resilience` | recommend additive checks | enforce failure/recovery checks |
| `property` | recommend additive checks | validate generated cases and evidence |
| `observability` | recommend additive checks | require receipt and telemetry evidence |

Quality-profile rollout is nested under the policy lifecycle and is distinct
from `candidate -> shadow -> promotable -> active`. Its rollout is monotonic:
`off -> shadow -> additive_canary -> active`. Rhai is
never authoritative in any mode: Rust validates IDs, types, dependencies,
limits, effective profiles, capabilities, evidence, persistence, receipts,
fallback, rollback, pinning, and the final verdict. Evaluation, promotion,
activation, failover, rollback, and resume each require a receipt. Missing or
incompatible pinned bundles fail closed with `policy_pinned_bundle_missing`;
fallback is only the receipt-backed last-known-good bundle or immutable Rust
baseline. Existing run pins are immutable and rollback does not rewrite them.

Quality-gate fail-closed triggers are unknown policy/profile IDs, schema/type
mismatch, oversized context or values, evaluator timeout, sandbox error, invalid
Rhai output, missing/incompatible pin, or receipt failure; Rust falls back only
to last-known-good or immutable baseline. Shadow receipts contain policy
identity, input/output digests, duration, agreement/diff, and error/fallback
codes only; never raw context, secrets, or arbitrary Rhai output.

The canonical cross-document matrix is:

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

## Rejected Alternatives

| Alternative | Rejection reason |
|---|---|
| YAML-only selection | It cannot provide durable selection, immutable identity, pinning, or transactional recovery. |
| Rhai self-authorized effects | It would bypass Rust capability, ownership, persistence, and fail-closed enforcement. |
| Direct activation of the latest version | It is non-reproducible and provides no shadow evidence or promotion gate. |
| Keep active state only in files or memory | It cannot provide durable pointers, receipts, crash recovery, or pinned resume. |
| Let rollback rewrite run pins | It would change the meaning of already-started work and break deterministic resume. |

## Trade-offs

- The Rust host and DB boundary are larger and stricter, but authority,
  enforcement, receipts, and recovery remain auditable and deterministic.
- Shadow evaluation consumes capacity and delays promotion, but exposes
  divergence before a policy can authorize any effect.
- Immutable versions and content digests require registry/storage discipline,
  but make replay, rollback, and pinned resume reproducible.
- Explicit fail-closed blocking can reduce availability during evaluator or DB
  faults, but avoids unsafe fallback or authority ambiguity.

## Consequences

- New policy behavior must fit the Rust-owned ABI and one of the six registered
  policy IDs; new capabilities require a Rust change.
- Implementations must persist lifecycle transitions, active and
  last-known-good pointers, pins, and receipts in the DB transaction boundary.
- Promotion, failover, rollback, and pinned-resume proofs must cover replay,
  crash/retry, parity drift, unavailable dependencies, and missing receipts.
- Existing runs remain pinned; only new runs use the active bundle after the
  activation receipt is durable.
- Operator surfaces must expose policy ID/version/digest, mode, pin, verdict,
  fallback reason, and receipt ID without arbitrary script output or secrets.

## Non-Goals

- Implementing the Rhai engine, Rust host, or policy ABI.
- Migrating the DB or changing DB schemas in this ADR.
- Changing YAML declarations, TaskFlow, runtime state, or active pointers as
  part of documenting this decision.
- Allowing Rhai to add capabilities, effects, persistence, or authority beyond
  the Rust boundary.
- Defining the operational authoring, incident-response, or repair runbook;
  those procedures belong in `docs/process/rhai-policy-authoring-runbook.md`.

## Canonical Design

The detailed requirements, component model, state model, rollout strategy, and
proof matrix remain canonical in
`docs/product/spec/versioned-rhai-policy-runtime-design.md`.

-----
artifact_path: product/decisions/versioned-rhai-policy-runtime-authority-adr
artifact_type: architecture_decision_record
artifact_version: "1"
schema_version: "1"
status: canonical
source_path: docs/product/decisions/versioned-rhai-policy-runtime-authority-adr.md
