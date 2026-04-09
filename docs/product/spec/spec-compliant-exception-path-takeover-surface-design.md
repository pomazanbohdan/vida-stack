# Spec Compliant Exception Path Takeover Surface Design

Purpose: restore a spec-compliant, receipt-governed operator path for local exception takeover so the runtime no longer deadlocks when specs require exception-path evidence but the product exposes no canonical surface that can record it.

## 1. Problem

Current project law says:

1. root-session local write work is forbidden by default,
2. normal write-producing work must stay on delegated lanes,
3. local takeover requires explicit exception-path receipt,
4. exception-path receipt is not sufficient while an open delegated cycle for the same packet still remains active,
5. canonical root `vida lane` is the operator surface that should expose lane status plus supersession or exception refs.

Current implementation drifts from that law in one critical way:

1. `vida lane` is still a fail-closed reserved surface,
2. `vida taskflow run-graph update` mutates `RunGraphStatus` only and cannot write dispatch-receipt evidence,
3. `RunGraphDispatchReceipt` already supports `exception_path_receipt_id`,
4. summary derivation currently collapses that field directly into active takeover semantics instead of separating recorded evidence from active authority,
5. no canonical operator mutation path exists to record the required takeover evidence.

This creates a Release 1 execution deadlock:

1. specs require exception-path receipt before local mutation,
2. runtime does not expose a lawful way to record that receipt,
3. root-session write remains blocked even when the blockage itself is implementation debt.

## 2. Goal

Implement the smallest canonical closure that makes exception-path takeover lawful, auditable, and machine-readable without weakening open-delegation law.

The bounded target is:

1. root `vida lane` becomes a real family-owned operator surface,
2. operator can inspect lane state through `vida lane show`,
3. operator can record bounded exception takeover through `vida lane exception-takeover`,
4. dispatch-receipt evidence remains the authoritative takeover source,
5. `vida status`, `vida doctor`, and resume/continue surfaces consume the same evidence consistently,
6. receipt recording, admissibility, and active takeover remain distinct machine-readable states.

## 3. Non-Goals

This change does not:

1. authorize generic local fallback,
2. bypass delegated-cycle law,
3. replace delegated execution with root-session coding as a normal path,
4. introduce memory-runtime behavior,
5. introduce vector retrieval, semantic search, or non-operator workflow changes.

## 4. Canonical Operator Surface

### 4.1 `vida lane show`

Root canonical inspection surface for one lane or run.

Must expose:

1. `surface`
2. `status`
3. `trace_id`
4. `workflow_class`
5. `risk_tier`
6. `artifact_refs`
7. `next_actions`
8. `blocker_codes`
9. `run_id`
10. `lane_id`
11. `runtime_role`
12. `lane_status`
13. `selected_backend`
14. `dispatch_status`
15. `supersedes_receipt_id`
16. `exception_path_receipt_id`

Primary source should be the latest or selected `RunGraphDispatchReceiptSummary`.

### 4.2 `vida lane exception-takeover`

Root canonical mutation surface for bounded local takeover evidence.

Expected shape:

1. input identifies one active `run_id`,
2. input records one bounded `receipt_id`,
3. optional structured reason fields may be accepted now or added later in backward-compatible form,
4. surface writes the exception-path evidence into the authoritative dispatch receipt,
5. response returns the updated lane envelope,
6. derived `lane_status` must report `lane_exception_recorded` while takeover is still blocked,
7. derived `lane_status` may report `lane_exception_takeover` only after takeover law is explicitly active.

## 5. Exception Receipt Contract

The operator mutation must satisfy the higher-precedence exception-path receipt contract already fixed in orchestration law.

Minimum fields:

1. `reason_class`
   - `agent_saturation`
   - `failed_lawful_reuse`
   - `documented_normal_lane_failure`
   - `higher_precedence_local_law`
2. `active_bounded_unit`
3. `owned_write_scope`
4. `why_delegated_or_rerouted_path_is_not_currently_lawful`
5. `why_local_write_is_the_smallest_safe_bounded_workaround`
6. `return_to_normal_posture_condition`
7. `verification_plan`

Release 1 minimum implementation rule:

1. `receipt_id` must be first-class and persisted now,
2. if richer structured fields are not yet modeled as first-class receipt storage, the initial operator surface may carry them in bounded metadata so long as the mutation remains auditable and machine-readable,
3. do not ship a string-only freeform mutation that loses receipt identity.

## 6. Open-Delegation Gate

Exception-path receipt is necessary but not sufficient while an open delegated cycle for the same packet remains active.

Therefore the operator surface must not silently permit takeover when the delegated cycle is still lawful.

Allowed outcomes:

1. if the delegated cycle remains open with no supersession, fail closed for local write but still persist the exception receipt as recorded evidence,
2. if explicit supersession or redirection is already recorded, takeover may become admissible,
3. blocker codes such as `internal_activation_view_only` or `configured_backend_dispatch_failed` are diagnosis evidence only and must not auto-activate local write by themselves,
4. if higher-precedence route law explicitly allows takeover, takeover may become admissible,
5. in all admissible cases, persist the exception receipt before the first local mutation,
6. active local write authority still requires an explicit active takeover state rather than implicit promotion from receipt existence alone.

## 7. State Model

Authoritative storage stays in `RunGraphDispatchReceipt`.

Implementation rule:

1. read the existing dispatch receipt for the selected `run_id`,
2. preserve existing dispatch lineage fields,
3. set `exception_path_receipt_id`,
4. optionally append structured exception metadata if the bounded implementation stores it now,
5. re-record the receipt through the authoritative state store,
6. derive `lane_exception_recorded` by default from exception evidence,
7. preserve `lane_exception_takeover` only when active takeover authority has been explicitly recorded,
8. never let receipt presence alone imply active local write.

Important constraint:

1. do not route this through `RunGraphStatus` only,
2. `run-graph update` is not sufficient because takeover evidence belongs to dispatch receipt authority.

## 8. Status And Doctor Propagation

Current `status` write-guard logic normalizes to `blocked_by_default` and does not surface exception evidence as a first-class operator truth.

This change must make that state observable.

Minimum required behavior:

1. `vida status --json` must expose whether exception-path evidence exists for the latest relevant lane,
2. `vida doctor --json` must stop acting as though no lawful takeover path exists once the receipt is recorded,
3. status and doctor must still fail closed when exception-path evidence is absent,
4. operator output must distinguish:
   - no exception evidence,
   - exception evidence recorded but delegated cycle still blocks takeover,
   - exception evidence admissible but not yet active,
   - exception evidence present and takeover lawfully active.

## 9. Consume / Recovery Continuity

`vida taskflow consume continue` and related resume logic already read `exception_path_receipt_id` when it exists.

This change must make that path usable end-to-end.

Required result:

1. when `exception_path_receipt_id` is persisted,
2. and delegated-cycle law is not yet satisfied, resume and continue logic must still accept `lane_exception_recorded` as lawful receipt evidence without confusing it with active local write,
3. and delegated-cycle law is satisfied plus takeover is explicitly activated, resume and continue logic accepts `lane_exception_takeover` as lawful active evidence instead of failing with missing exception-path evidence.

## 10. Code Map

Bounded implementation slice:

1. `crates/vida/src/root_command_router.rs`
   - replace fail-closed `vida lane` stub with family-owned routing
2. new `lane_surface` module under `crates/vida/src`
   - parse `show` and `exception-takeover`
   - render canonical operator envelope
   - perform receipt mutation
3. `crates/vida/src/cli.rs`
   - update `lane` help from reserved/fail-closed wording
4. `crates/vida/src/surface_render.rs`
   - help text parity for root surface
5. `crates/vida/src/status_surface_write_guard.rs`
   - surface exception evidence instead of blind blocked default
6. `crates/vida/src/status_surface.rs`
   - propagate new artifact refs if needed
7. `crates/vida/src/doctor_surface.rs`
   - remediation text and evidence visibility parity
8. existing state-store and run-graph summary modules
   - reuse, do not fork logic already present for `exception_path_receipt_id`

## 11. Proof

### Unit / Contract

1. `vida lane show` returns canonical root operator envelope,
2. `vida lane exception-takeover` persists `exception_path_receipt_id`,
3. receipt summary derives `lane_exception_recorded` until takeover activation is explicit,
4. status surface reports `receipt_recorded`, `admissible_not_active`, and `active` consistently,
5. doctor surface reports takeover posture consistently.

### Negative Proof

1. open delegated cycle without exception receipt stays blocked,
2. exception receipt without delegated-cycle clearance stays blocked,
3. exception receipt plus diagnosis blocker alone must not auto-activate local write,
4. root surface must not silently fall back to `taskflow run-graph update`.

### Runtime Proof

1. `vida lane show <run-id> --json`
2. `vida lane exception-takeover <run-id> --receipt-id <id> --json`
3. `vida status --json`
4. `vida doctor --json`
5. `vida taskflow consume continue --json`

## 12. Dependency And Sequencing

This design is the critical-path unblocker for:

1. finalizing bounded spec-first design documents when delegated execution is bridge-blocked,
2. lawful local repair of runtime implementation debt,
3. the downstream VIDA memory implementation track based on MemPalace and `memory-mcp-1file`.

Sequence:

1. close the takeover surface,
2. use it or equivalent lawful evidence path to finalize the bounded design docs,
3. then continue with the VIDA memory runtime track.

## 13. Result

Release 1 keeps delegation-first law intact, but removes the current deadlock by adding the missing operator mutation surface required by the existing specs.

-----
artifact_path: product/spec/spec-compliant-exception-path-takeover-surface-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-08
schema_version: 1
status: canonical
source_path: docs/product/spec/spec-compliant-exception-path-takeover-surface-design.md
created_at: 2026-04-08T21:24:52.231234213Z
updated_at: 2026-04-09T05:41:17.916939008Z
changelog_ref: spec-compliant-exception-path-takeover-surface-design.changelog.jsonl
