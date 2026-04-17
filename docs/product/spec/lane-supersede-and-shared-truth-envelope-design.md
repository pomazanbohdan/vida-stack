# Lane Supersede And Shared Truth Envelope Design

Status: approved

## Summary
- Feature / change: make `vida lane supersede` the explicit supersession mutation and unify one shared lane-envelope truth derivation across `show`, `exception-takeover`, and `supersede`
- Owner layer: `mixed`
- Runtime surface: `launcher | taskflow recovery adjacency`
- Status: `approved`

## Current Context
- `vida lane` already exists as a root family-owned operator surface.
- `RunGraphDispatchReceipt` already carries both `exception_path_receipt_id` and `supersedes_receipt_id`.
- `release1_contracts.rs` already distinguishes:
  - receipt recorded versus active takeover admissibility through `exception_takeover_state(...)`
  - canonical lane-status mapping through `derive_lane_status(...)`
- The remaining drift is in the lane surface projection layer:
  - `show` derives blocked/pass truth through one helper,
  - `exception-takeover` inlines a separate activation branch,
  - `supersede` inlines another activation branch,
  - recovery/help surfaces do not point operators to the full recovery-critical lane command set.

## Goal
- Make `exception-takeover` record exception-path evidence without silently promoting it to active local-write authority.
- Make `supersede` the explicit lawful supersession evidence path that can promote an already recorded exception receipt into `lane_exception_takeover`.
- Make `show`, `exception-takeover`, and `supersede` all emit one canonical lane envelope with the same blocked/pass, blocker-code, and next-action logic.
- Keep recovery/help discoverability aligned so operators can discover `show`, `exception-takeover`, and `supersede` from recovery-adjacent help.

## Requirements

### Functional Requirements
- `vida lane show` must emit one canonical envelope whose `blocker_codes` and `next_actions` come from a shared derivation path.
- `vida lane exception-takeover <run-id> --receipt-id <id>` must persist `exception_path_receipt_id` and default to `lane_exception_recorded` unless explicit supersession evidence is already present.
- `vida lane supersede <run-id> --receipt-id <id>` must persist `supersedes_receipt_id`.
- When exception evidence already exists, `vida lane supersede` must promote the lane to `lane_exception_takeover`.
- When exception evidence exists but supersession evidence does not, the shared lane envelope must fail closed and tell the operator that supersession is still required.
- Open delegated-cycle denial must continue to surface `open_delegated_cycle`.
- Recovery-adjacent help must mention the recovery-critical lane commands so the operator does not have to guess the lane subcommands.

### Non-Functional Requirements
- Keep the change bounded to lane-surface projection/mutation law, adjacent help/discoverability, and directly affected tests/docs.
- Reuse `release1_contracts.rs` and existing state-store summaries instead of inventing a second lane-truth model.
- Emit only registry-backed blocker codes.

## Design Decisions

### 1. Receipt recording and active takeover remain separate states
Will implement / choose:
- `exception-takeover` records the exception receipt.
- `supersede` records supersession evidence.
- Active local-write authority is reflected only when explicit supersession evidence is present.
- Why:
  - This preserves the already established `receipt_recorded` versus `admissible_not_active` versus `active` split used by the write-guard/status surfaces.
- Trade-offs:
  - Operators need one more explicit mutation step before local takeover becomes active.

### 2. Shared lane truth lives in one projection helper
Will implement / choose:
- One lane-truth helper will derive:
  - `status`
  - `blocker_codes`
  - `next_actions`
- The same helper will be used by `show`, `exception-takeover`, and `supersede`.
- Why:
  - The runtime already had the right evidence fields; drift existed in how each command projected them.

### 3. Missing supersession after exception receipt is a first-class blocker
Will implement / choose:
- Use canonical blocker `supersession_without_receipt` when exception evidence exists, delegated-cycle law no longer blocks admissibility, but explicit supersession evidence has not yet been recorded.
- Why:
  - That state is neither `open_delegated_cycle` nor active takeover; it needs a truthful machine-readable blocker.

## Technical Design

### Core Components
- `crates/vida/src/lane_surface.rs`
  - add one shared lane-truth derivation helper
  - update mutation handlers to reuse that helper
  - stop `exception-takeover` from auto-promoting to active takeover on admissibility alone
- `crates/vida/src/taskflow_layer4.rs`
  - expose lane-command discoverability from recovery help
- tests in `crates/vida/src/lane_surface.rs`
  - pin recorded-vs-superseded truth and shared envelope output

### Shared Truth Rules
1. `lane_exception_takeover` with active takeover evidence is `pass`.
2. `lane_superseded` is `pass`.
3. `lane_exception_recorded` is `blocked`.
4. `lane_exception_recorded` plus open delegated cycle emits `open_delegated_cycle`.
5. `lane_exception_recorded` plus admissible-but-not-active takeover emits `supersession_without_receipt` and points the operator to `vida lane supersede`.
6. Ordinary blocked/failed dispatch truth continues to surface canonical receipt/downstream blocker codes.

### Bounded File Set
- `docs/product/spec/lane-supersede-and-shared-truth-envelope-design.md`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/current-spec-provenance-map.md`
- `docs/product/spec/README.md`
- `crates/vida/src/lane_surface.rs`
- `crates/vida/src/taskflow_layer4.rs`

## Validation / Proof
- Unit / contract:
  - targeted lane-surface tests for:
    - recorded exception receipt remains blocked
    - admissible-but-not-active state points to `supersede`
    - `supersede` activates exception takeover when exception evidence already exists
    - `show` emits registry-backed blocker codes from the shared helper
- Runtime checks:
  - `vida lane show <run-id> --json`
  - `vida lane exception-takeover <run-id> --receipt-id <id> --json`
  - `vida lane supersede <run-id> --receipt-id <id> --json`
  - `vida taskflow help recovery`

## References
- `docs/product/spec/spec-compliant-exception-path-takeover-surface-design.md`
- `docs/product/spec/release-1-operator-surface-contract.md`
- `docs/product/spec/release-1-error-and-exception-taxonomy.md`
- `docs/product/spec/release-1-runtime-enum-and-code-contracts.md`

-----
artifact_path: product/spec/lane-supersede-and-shared-truth-envelope-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: 2026-04-17
schema_version: '1'
status: canonical
source_path: docs/product/spec/lane-supersede-and-shared-truth-envelope-design.md
created_at: '2026-04-17T18:20:00+03:00'
updated_at: 2026-04-17T18:20:00+03:00
changelog_ref: lane-supersede-and-shared-truth-envelope-design.changelog.jsonl
