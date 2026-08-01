# Rhai Quality-Gate Rollout Closeout Runbook

Status: canonical operator procedure for rollout admission and recovery.

## Purpose and authority

Use this runbook to admit, canary, activate, rollback, quarantine, and resume
the `rhai.runtime.quality-gate` policy family. The authority split is fixed:

- Rust validates schemas, ABI, dependencies, limits, capabilities, effects,
  profile unions, persistence, receipts, fallback, rollback, and final verdicts.
- Rhai is deterministic and non-authoritative; it may recommend additive
  profiles only and cannot pass evidence, grant effects, or mutate pointers/pins.
- YAML is reviewed declaration input; the DB is authoritative for immutable
  policy identity, lifecycle, active/last-known-good pointers, run pins, and
  receipts.

Canonical companion documents:

- [`versioned-rhai-policy-runtime-design.md`](../product/spec/versioned-rhai-policy-runtime-design.md)
- [`versioned-rhai-policy-runtime-authority-adr.md`](../product/decisions/versioned-rhai-policy-runtime-authority-adr.md)
- [`zombie-d-test-writing-protocol.md`](../process/zombie-d-test-writing-protocol.md)

## Admission thresholds

Use the exact `(policy_id, version, content_digest)` tuple for every artifact.
Promotion is blocked unless all thresholds pass:

| Gate | Threshold |
|---|---|
| Compatibility | 100% schema/ABI/dependency/digest/limit checks; persisted pins resolve exactly |
| Replay | 100% deterministic replay agreement for verdict, effective profiles, and proof digest |
| Shadow parity | 100% Rust final-verdict agreement; additive differences are enumerated and do not change Rust verdicts |
| Safety/health | 0 forbidden effects/capabilities, timeouts, panics, invalid outputs, or raw-secret/raw-evidence emissions |
| Receipts | 100% evaluation and lifecycle transitions have durable receipts |
| Profiles | all eight registered profiles remain in `Rust_required ∪ explicit_profiles ∪ Rhai_additions` |

The canary runs the complete bounded corpus and, when available, at least 100
production-like evaluations. Canary output never authorizes effects or changes
the active pointer. The only valid quality-profile sequence is
`off -> shadow -> additive_canary -> active`.

## Procedure

1. **Prepare** — verify the immutable tuple, manifest, dependency closure,
   limits, Rust ABI, and a receipt-backed candidate record.
2. **Shadow** — evaluate beside the Rust path; compare final verdicts and
   effective profiles; store bounded receipts containing IDs/digests/mode,
   duration, agreement/diff, blocker/fallback codes only.
3. **Canary** — run the threshold corpus; keep Rhai recommendations additive;
   require Rust validation and final verdict for every case.
4. **Activate** — after DocFlow/readiness, graph, focused E2E, formatting/check,
   and cross-document parity evidence is attached, let Rust atomically change
   the DB active pointer and persist the activation receipt.
5. **Observe** — monitor divergence, evaluator errors/timeouts, invalid output,
   profile-union changes, promotion/activation rejection, failover, rollback,
   quarantine, and pinned-resume blocks. Never persist raw context, secrets,
   credentials, or arbitrary Rhai output.

## Rollback, quarantine, and last-known-good

On any threshold breach, freeze promotion and preserve active, last-known-good,
and existing run-pin state. Quarantine the failed tuple, capture its receipt and
blocker code, then ask the Rust host to atomically restore the valid
last-known-good bundle. If it is unavailable or incompatible, Rust may use only
the immutable baseline. If neither validates, keep the operation blocked and
escalate. Rollback never rewrites run pins and never selects the latest file,
latest DB row, an unpinned version, or an ad hoc script.

## Compatibility and pinned resume

Resume resolves the exact persisted policy tuple; it does not follow the active
pointer silently. Missing, stale, incompatible, or digest-mismatched pins are
fail-closed blockers. Pin repair requires an explicit Rust/DB transaction and a
durable receipt. Re-run replay, parity, profile-union, and receipt checks before
returning a recovered version to `shadow` or `promotable`.

## Release and operator closeout

Attach evidence for DocFlow/readiness, `vida task validate-graph --json`, the
focused quality-gate E2E matrix, formatting/check gates, and parity across the
three canonical docs plus this runbook. Missing or blocked evidence keeps the
TaskFlow item open. System installation is a separate canonical
release-install gate after this evidence bundle; installation success alone is
not policy or Rust-authority proof.

-----
artifact_path: runbooks/rhai-quality-gate-rollout-closeout
artifact_type: runbook
artifact_version: "1"
schema_version: "1"
status: canonical
source_path: docs/runbooks/rhai-quality-gate-rollout-closeout.md
