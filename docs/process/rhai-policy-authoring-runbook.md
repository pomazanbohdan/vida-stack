# Rhai Policy Authoring Runbook

Status: canonical for versioned Rhai policy authoring and rollout operations.

## Purpose

Use this runbook to author, validate, stage, promote, activate, recover, and
resume versioned Rhai policies without transferring runtime authority out of
the Rust host or losing durable policy identity.

Canonical references:

- `docs/product/spec/versioned-rhai-policy-runtime-design.md`
- `docs/product/decisions/versioned-rhai-policy-runtime-authority-adr.md`

## Authority and authoring inputs

Authoring starts with all of the following inputs:

1. A reviewed YAML manifest containing the policy ID, version, content digest,
   dependencies, resource limits, ABI declaration, and rollout intent.
2. A versioned Rhai bundle containing bounded, deterministic, pure decision
   functions behind the allowlisted Rust ABI.
3. The Rust host contract, including schema validation, compilation admission,
   capability allowlists, time/memory/input/instruction limits, effect checks,
   transaction boundaries, and final verdict rules.
4. The DB registry, which stores immutable policy identity, lifecycle state,
   active and last-known-good pointers, run pins, and receipts.
5. Representative validated input snapshots and deterministic replay cases.

Authority is divided as follows:

- Rust owns parsing, validation, compilation, capabilities, effects,
  persistence, transactions, and final authorization.
- Rhai returns typed decisions only; it cannot perform I/O, mutate DB state,
  select its own version, grant capabilities, or bypass Rust checks.
- YAML is reviewed declaration and rollout input; publishing YAML creates
  candidate metadata but never changes the active pointer.
- The DB is authoritative for selected immutable versions, lifecycle state,
  pointers, pins, and promotion/failover/rollback receipts.

## Policy catalog

The runtime supports exactly these six policy IDs:

| Policy ID | Responsibility | Shadow behavior | Active behavior |
|---|---|---|---|
| `rhai.runtime.authority` | authority and claim decision | compare only | Rust validates and enforces |
| `rhai.runtime.lifecycle` | candidate and state-transition rules | observe transitions | Rust commits allowed transitions |
| `rhai.runtime.failover` | unavailable/error fallback choice | simulate fallback | Rust selects last-known-good or baseline |
| `rhai.runtime.promotion` | promotion eligibility verdict | calculate gate result | Rust admits only passed gates |
| `rhai.runtime.rollback` | rollback and quarantine recommendation | simulate recovery | Rust atomically changes the pointer |
| `rhai.runtime.pinned-resume` | resume compatibility decision | compare against pin | Rust requires the persisted pin |

Policy identity is the exact tuple `(policy_id, version, content_digest)`.
A digest mismatch is invalid and is never interpreted as a new version.

## Authoring and validation procedure

1. Select one of the six policy IDs and create a new immutable version. Do not
   overwrite an existing version or select a policy by filename order.
2. Review the YAML manifest against the Rust ABI, dependency closure, declared
   limits, rollout intent, and policy-specific input/output schema.
3. Compile the Rhai bundle through the Rust host. Verify deterministic,
   bounded execution and reject unknown IDs, duplicate versions, invalid YAML,
   digest mismatches, unsupported ABI, missing dependencies, excessive limits,
   nondeterministic inputs, and forbidden capabilities.
4. Replay representative snapshots and record policy ID, version, digest, mode,
   input snapshot identity, typed verdict, and receipt ID for every evaluation.
5. Keep the new version in `candidate` until validation and receipts are
   complete. Authoring must not mutate the active pointer, run pins, or runtime
   state.

## Shadow rollout

Policy versions follow the explicit lifecycle:

`candidate -> shadow -> promotable -> active -> retired|rolled_back|quarantined`.

1. Register the immutable candidate in the DB and transition it to `shadow`
   through the Rust-owned transaction boundary.
2. Run shadow evaluation beside the current Rust/active decision path.
3. Treat shadow output as comparison data only; it cannot authorize effects,
   state transitions, capability grants, or pointer changes.
4. Record divergence, evaluator failures, resource use, and complete shadow
   receipts. Do not mark the version `promotable` until the manifest,
   compilation, dependencies, resource profile, and shadow evidence pass.

## Promotion gates

Promote one policy/version at a time only after all gates are evidenced:

- valid reviewed manifest and immutable content digest;
- successful Rust-host compilation and bounded evaluation;
- closed and valid dependency graph;
- deterministic replay with stable verdicts;
- no forbidden capability, effect, ownership, or ABI violation;
- acceptable shadow parity and no unexplained divergence;
- complete evaluation and transition receipts;
- explicit operator/runtime promotion approval.

Rust admits `promotable -> active` only through an atomic DB transaction. The
promotion receipt must identify the policy tuple, gate results, approver or
runtime gate, and resulting pointer state.

## Active activation

Activation changes the DB active pointer atomically only after the activation
receipt is durable. The active policy remains subject to Rust validation,
capability checks, ownership checks, transaction boundaries, resource limits,
and receipt requirements. New runs use the active bundle only after that
receipt is durable; existing runs retain their persisted pins.

## Failover and rollback

### Failover

On unavailable, timed-out, panicking, invalid, or divergent policy evaluation,
Rust selects the DB last-known-good version. If that version is unavailable or
invalid, Rust may select only the immutable Rust baseline. If neither is
valid, block the operation and preserve the failure evidence. Record the
fallback reason, selected tuple, and receipt.

### Rollback

Rollback is a Rust-owned atomic pointer change to the prior valid version. It
must record the reason, failed tuple, restored tuple, and rollback receipt;
quarantine the failed version; and leave existing run pins unchanged. Never
rewrite pins to make rollback appear successful.

## Pinned resume

Resume resolves the exact persisted `(policy_id, version, content_digest)` for
the run. It must not follow the current active pointer silently. A missing,
stale, incompatible, unavailable, or digest-mismatched pin fails closed and
requires explicit recovery evidence. Recovery may repair the pin only through
the Rust/DB authority boundary with a durable receipt; an operator must not
substitute the latest YAML, latest DB row, or an ad hoc script.

## Receipt requirements

Persist append-only evidence for every evaluation and state transition. At
minimum, a receipt identifies:

- policy ID, version, and content digest;
- `shadow` or `active` mode;
- input snapshot and run-pin identity where applicable;
- typed verdict, gate result, fallback or rollback reason;
- prior and resulting lifecycle/pointer state;
- timestamp/correlation and receipt ID;
- Rust host validation outcome and any fail-closed blocker.

Receipts are required for candidate registration, shadow evaluation, promotion,
activation, failover, rollback, quarantine, and pinned-resume repair.

## Incident handling

1. Stop effect authorization immediately on evaluator error, timeout, panic,
   parity drift, missing receipt, invalid digest, DB unavailability, or failed
   promotion/activation gate.
2. Preserve the active pointer and all existing run pins while collecting the
   failing policy tuple, input snapshot, verdict, error, and receipt evidence.
3. Use the last-known-good DB version, then the immutable Rust baseline, only
   through Rust failover logic. If neither passes validation, keep the operation
   blocked.
4. Quarantine the failed candidate/version before any rollback or retry that
   could re-authorize it.
5. Roll back atomically when the prior version is valid; otherwise escalate as
   a fail-closed availability incident and require explicit recovery approval.
6. After recovery, replay the pinned-resume and promotion evidence before
   returning a policy to `shadow` or `promotable`.

## Fail-closed stop conditions

Stop authoring, rollout, activation, or resume when any of these conditions is
present:

- unknown policy ID, duplicate version, digest mismatch, or invalid manifest;
- missing dependency, unsupported ABI, excessive resource declaration, or
  nondeterministic input;
- Rhai attempts I/O, persistence, capability grant, pointer/pin mutation, or
  an effect outside the Rust-owned ABI;
- compilation, evaluation, deterministic replay, shadow parity, or resource
  validation fails;
- promotion, activation, failover, rollback, or pin repair lacks a durable
  receipt or atomic DB transaction;
- evaluator timeout/panic, unavailable DB, missing last-known-good/baseline,
  stale or incompatible pin, or unexplained divergence occurs;
- requested recovery would select the latest file/row, an unpinned version, or
  an ad hoc script.

Do not resume or activate by guessing. Expose the exact blocker and recovery
state, preserve pins and receipts, and wait for explicit Rust/DB-authority
recovery.

## Scope boundary

This runbook does not implement the Rhai engine, Rust host, ABI, DB migration,
YAML schema change, TaskFlow mutation, runtime-state mutation, or new policy
capabilities. Those changes require their own bounded design, proof, and
authority decision.

-----
artifact_path: process/rhai-policy-authoring-runbook
artifact_type: process_runbook
artifact_version: "1"
schema_version: "1"
status: canonical
source_path: docs/process/rhai-policy-authoring-runbook.md
