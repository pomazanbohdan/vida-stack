# VIDA Stack — META Refactoring Execution Plan

**Artifact purpose:** executable, agent-ready refactoring plan for decomposing `vida-stack` runtime code into maintainable shared libraries and ownership-specific modules while immediately incorporating the defect classes already found.

**Repository:** `pomazanbohdan/vida-stack`
**Canonical path:** `docs/product/spec/meta-refactor-runtime-boundary-source-plan.md`
**Target branch suggestion:** `refactor/meta-runtime-boundaries`
**Plan status:** execution-ready design; implementation must proceed wave-by-wave with tests after each move.
**Generated:** 2026-06-07

---

## 0. META algorithm execution receipt

### 0.1 Selected algorithm

The repository contains the canonical thinking algorithm in:

- `vida/config/instructions/references/algorithms.quick-reference.md`
- `vida/config/instructions/instruction-contracts/overlay.step-thinking-protocol.md`

This plan uses **META** because the task is:

- foundation architecture;
- framework-owned behavior change;
- fail-closed law risk;
- routing / authority chain risk;
- multi-error pool;
- explicit meta-analysis request.

### 0.2 Selected META block flow

Use the smallest lawful META block flow:

```text
SEL-01 override_check
SEL-03 route_bind
CTX-01 module_select
CTX-03 plan_seed
CTX-04 session_packet_bind
CTX-05 invariant_precedence_resolve
CRT-01/02/03 validation family
RFX-01/02/03 refinement family
OPT-01..05 options family
BUG-00..08 bug/error-search family
ENS-00 continuity admissibility
ENS-01 admissibility gate
ENS-02 cross-flow compare
ENS-03 weighted confidence
ENS-05 final synthesis
REP-01 evidence pack
REP-02 impact analysis
REP-03 execution proof
```

### 0.3 Task class and weights

Task class: `foundation_architecture` + `bug_root_cause` hybrid.

Use foundation architecture weights as primary because the output is a refactor plan:

```yaml
critique: 0.25
refinement: 0.30
options: 0.35
bug: 0.10
```

Bug blocks are still mandatory for all defect classes already found.

### 0.4 Admissibility gate

This plan is admissible only if all of these remain true during implementation:

```yaml
must_preserve:
  - public command names
  - public JSON envelope shape
  - release-1 operator contract parity
  - persisted state compatibility
  - receipt semantics
  - run-graph status/recovery visibility
  - explicit --state-dir authority
  - fail-closed host bridge behavior
  - compact TOON/default human output parity

must_not:
  - introduce parallel old/new execution paths without bounded removal task
  - let mutable request JSON become authority
  - let derived cache/snapshot override persisted state
  - move runtime truth into launcher rendering modules
  - use direct unbounded read_to_string in runtime surfaces
  - bypass TaskFlow authority through vida shell
```

---

## 1. Current architecture diagnosis

### 1.1 Core problem

The code currently concentrates too much runtime truth inside `crates/vida/src/**`.

The desired architecture already exists in project law:

```text
crates/vida/**:
  argument parsing
  subcommand routing
  text/json rendering

taskflow-*:
  lane lifecycle
  closure admission
  run graph
  delegated-cycle enforcement
  execution receipts

docflow-*:
  readiness proof
  validation outputs
  documentation mutation/validation law

shared contract crates:
  canonical artifact schemas
  error/blocker enums
  decision-table application I/O
```

The defect history confirms the problem: the same authority/path/session/routing mistakes appear across host bridge, lane completion, taskflow consume, status, doctor, docflow closeout, and task attempt surfaces.

### 1.2 Main anti-patterns to remove

| Anti-pattern | Why it blocks maintainability/debugging | Replacement |
|---|---|---|
| Runtime truth in `vida` surface files | shell owns decisions it should only route/render | TaskFlow use cases + shell adapters |
| Stringly typed blocker/status codes | parity gaps, typo risk, incomplete registry | typed enums in `taskflow-contracts` |
| Direct `serde_json::Value` domain logic | weak contracts, hard debugging | typed DTOs at boundaries |
| Direct file IO in surfaces | symlink/FIFO/out-of-root/DoS bugs | shared `runtime-path-policy` |
| Mutable request JSON as authority | host bridge integrity bypass | persisted receipt/result/packet authority chain |
| Derived cache/snapshot as authority | stale pass / forged final snapshot | state-store-backed validation |
| Large mixed-owner modules | hard to test and reason about | owner-specific child modules and facades |
| CLI render mixed with decision logic | JSON/human parity drift | `operator-output` shared renderer |

---

## 2. Target module and crate layout

### 2.1 New / promoted shared crates

Create these crates or modules. Prefer crates when the boundary must be reused across TaskFlow, DocFlow, and `vida`; otherwise create modules under `taskflow-core` first and promote later.

```text
crates/runtime-path-policy/
  src/lib.rs
  src/state_root.rs
  src/safe_path.rs
  src/bounded_json.rs
  src/atomic_write.rs
  src/symlink_policy.rs
  src/size_limits.rs

crates/operator-output/
  src/lib.rs
  src/command_text.rs
  src/envelope.rs
  src/artifact_refs.rs
  src/toon_report.rs
  src/next_actions.rs

crates/taskflow-host-bridge/
  src/lib.rs
  src/request.rs
  src/provenance.rs
  src/state_root.rs
  src/receipt_binding.rs
  src/completion.rs
  src/artifact_scope.rs
  src/path_policy.rs
  src/errors.rs

crates/taskflow-authority/
  src/lib.rs
  src/authority_chain.rs
  src/run_graph_authority.rs
  src/dispatch_receipt_authority.rs
  src/cache_admission.rs
  src/stale_guard.rs
  src/errors.rs
```

If adding too many crates at once is too disruptive, use this staging:

```text
Wave A internal modules:
  crates/vida/src/runtime_path_policy.rs
  crates/vida/src/operator_output.rs
  crates/vida/src/taskflow_host_bridge.rs
  crates/vida/src/taskflow_authority.rs

Wave B promote to crates:
  crates/runtime-path-policy
  crates/operator-output
  crates/taskflow-host-bridge
  crates/taskflow-authority
```

However, the preferred implementation is to create crates directly because the workspace already has family crates.

### 2.2 Final shell layout

`crates/vida` should end as:

```text
crates/vida/src/
  main.rs
  cli.rs
  root_command_router.rs
  shell_runtime_helpers.rs
  surface_render.rs
  service_client_cli.rs
  docflow_proxy.rs
  runtime_web_surface.rs       # shell adapter only, after runtime-service extraction
  release_surface.rs
```

Everything else should move below TaskFlow, DocFlow, runtime contracts, or shared libraries.

---

## 3. Global implementation rules for every wave

### 3.1 Mechanical extraction rule

For each function move:

```text
1. Copy the function unchanged into the new module.
2. Copy only the minimum required helper types/imports.
3. Compile.
4. Replace original function body with a wrapper that calls the new module.
5. Run the focused test.
6. Replace direct internal call sites with new module path.
7. Compile again.
8. Delete original wrapper only after all call sites have migrated.
9. Run public smoke tests for the affected surface.
10. Update this plan/checklist with the completed item.
```

### 3.2 Fix-before-extract rule

When the function is part of a known defect class, do **not** move the bug unchanged. Apply the safe contract at the new boundary first, then migrate callers.

Defect classes that must be fixed during extraction:

```yaml
host_bridge:
  - explicit state root must override inferred request root
  - missing/unreadable receipt must block
  - mutable request paths cannot be authority
  - request path reads must be regular-file/size-limited
  - implementation scope must come from immutable dispatch packet

routing:
  - human role labels must map to canonical task_class for backend admissibility
  - aliases cannot resolve to orchestrator for agent-init worker lanes

state_authority:
  - terminal closure needs clean completed receipt
  - recorded exception is not active takeover
  - projection cache requires session/worktree/operator-state freshness
  - final runtime-consumption snapshots cannot mint authority without persisted receipt

artifact_io:
  - no unbounded read_to_string
  - no out-of-root reads
  - no symlink/device/FIFO reads
  - max-size guard for JSON artifacts

external_process:
  - docflow git status must ignore repo-local config/hooks/fsmonitor and use timeout
```

### 3.3 Do-not-change public contracts

Do not rename these public commands during refactor:

```text
vida task ...
vida taskflow ...
vida consume ...
vida recovery ...
vida route ...
vida lane ...
vida approval ...
vida agent ...
vida agent-init
vida orchestrator-init
vida status
vida doctor
vida diagnostics
vida docflow ...
vida runtime web ...
```

Do not intentionally change these public artifacts without a separate schema task:

```text
RunGraphStatus
RunGraphDispatchReceipt
runtime-consumption final snapshots
host-tool-bridge request/result/receipt artifacts
release-1 operator envelopes
taskflow consume continue JSON
status/doctor JSON
DocFlow closeout JSON
```

---

## 4. Wave 0 — Baseline freeze and defect fixtures

### Goal

Freeze current behavior and create failing fixtures for all known bug classes before large movement.

### 4.1 Create a baseline tracking document

Create:

```text
docs/product/spec/meta-refactor-runtime-boundary-execution-plan.md
```

Record this plan and the executed proof commands after every wave.

### 4.2 Add test tags / modules for known defect classes

Create these test groups if not already present:

```text
crates/vida/tests/host_bridge_authority_smoke.rs
crates/vida/tests/runtime_authority_smoke.rs
crates/vida/tests/taskflow_routing_smoke.rs
crates/vida/tests/artifact_io_smoke.rs
crates/docflow-cli/tests/git_hardening_smoke.rs
```

If the project prefers existing smoke files, append tests to:

```text
crates/vida/tests/doctor_surface_contract_smoke.rs
crates/vida/tests/boot_smoke.rs
crates/vida/tests/task_smoke.rs
crates/docflow-cli/tests/cli_smoke.rs
```

### 4.3 Add red tests for P0/P1 defects

#### Test HB-001: explicit state-dir is authoritative

```text
Surface:
  vida agent host-bridge

Setup:
  trusted_state_root = temp/trusted/.vida/data/state
  attacker_state_root = temp/attacker/.vida/data/state
  request_path = attacker_state_root/host-tool-bridge/requests/request.json
  --state-dir trusted_state_root

Expected:
  status = blocked
  blocker_codes contains host_bridge_request_untrusted_path
  host_tool_calls = []
```

#### Test HB-002: missing receipt blocks pending host bridge

```text
Surface:
  vida agent host-bridge

Setup:
  pending host bridge request under valid state root
  no persisted RunGraphDispatchReceipt for run_id

Expected:
  status = blocked
  blocker_codes contains host_bridge_dispatch_receipt_missing
  host_tool_calls = []
```

#### Test HB-003: mutable request cannot redirect result/receipt path

```text
Surface:
  vida lane complete

Setup:
  persisted dispatch result points to result_path_A / receipt_path_A
  mutable request points to result_path_B / receipt_path_B

Expected:
  exit non-zero
  no write to B
  blocker/error mentions persisted dispatch receipt evidence mismatch
```

#### Test HB-004: retryable request path rejects FIFO/out-of-root/huge file

```text
Surface:
  vida lane complete or helper-level unit test

Setup:
  request_path points outside state root OR to FIFO OR to > max bytes file

Expected:
  command returns quickly
  status blocked/fail closed
  no read hang
```

#### Test HB-005: implementation scope comes from immutable packet

```text
Surface:
  vida lane complete

Setup:
  immutable dispatch packet owned_paths = ["allowed"]
  mutable request owned_paths = ["allowed", "secret"]
  implementation artifact changed_files = ["secret/outside.txt"]

Expected:
  status = blocked
  blocker_codes contains implementation_attempt_scope_guard_violation
  scope_validation.owned_paths = ["allowed"]
```

#### Test RT-001: terminal missing-task closure needs clean receipt

```text
Surface:
  vida status / vida doctor / helper unit

Setup:
  latest RunGraphStatus = terminal closure
  dispatch receipt is exception takeover OR mismatched OR missing

Expected:
  stale write guard remains active
  no root-local write authority
```

#### Test RT-002: recorded exception is not active takeover

```text
Surface:
  vida lane retire

Setup:
  RunGraphDispatchReceipt.lane_status = lane_exception_recorded
  exception_path_receipt_id present
  supersedes_receipt_id missing

Expected:
  retire rejected
  status remains blocked
  continuation binding preserved
```

#### Test RT-003: projection cache rejects sessionless stale state-bound pass

```text
Surface:
  vida status / taskflow graph-summary

Setup:
  cached projection has status pass and state marker only
  non-task operator state changed or session identity absent

Expected:
  cache rejected
  recompute required
```

#### Test RT-004: final runtime snapshot cannot mint receipt authority

```text
Surface:
  status/doctor fallback

Setup:
  forged final runtime-consumption snapshot with lane_exception_takeover and absolute paths
  no persisted dispatch receipt

Expected:
  latest_final_runtime_consumption_dispatch_receipt_summary returns None
```

#### Test ROUTE-001: dev-team human labels enforce task_class admissibility

```text
Surface:
  runtime dispatch / scheduler dispatch / agent-init

Setup:
  dispatch target = developer
  dispatch contract task_class = implementation
  backend is not admissible for implementation

Expected:
  blocked by backend admissibility
```

#### Test ROUTE-002: agent-init aliases cannot resolve to orchestrator

```text
Surface:
  vida agent-init

Setup:
  project dev_team role alias maps to runtime_role = orchestrator

Expected:
  blocked/rejected before selected_role accepted
```

#### Test IO-001: task attempt artifact reads require regular file and max size

```text
Surface:
  vida task attempt collect/consolidate

Setup:
  artifact ref = symlink/FIFO/huge file/out-of-root path

Expected:
  blocked with canonical artifact read blocker
```

#### Test DOC-001: docflow closeout does not execute repo-local git helpers

```text
Surface:
  vida docflow closeout --changed
  docflow-cli closeout --changed

Setup:
  repo-local core.fsmonitor helper writes sentinel file if executed

Expected:
  sentinel file absent
  closeout still reports changed markdown
```

### 4.4 Baseline proof commands

Run after tests are created:

```bash
cargo fmt --all -- --check
git diff --check
cargo test -p vida --test doctor_surface_contract_smoke -- --nocapture
cargo test -p vida --test boot_smoke -- --nocapture
cargo test -p vida --test task_smoke -- --nocapture
cargo test -p docflow-cli --test cli_smoke -- --nocapture
cargo test -p vida host_bridge -- --nocapture
cargo test -p vida status_surface -- --nocapture
cargo test -p vida taskflow_consume_resume -- --nocapture
```

If full batches are too slow, run focused tests first, then batch before merging each wave.

---

## 5. Wave 1 — `runtime-path-policy`

### Goal

Create a single safe IO/path boundary and remove direct unsafe reads/writes from authority-sensitive runtime paths.

### 5.1 Create crate

Add to workspace `Cargo.toml`:

```toml
"crates/runtime-path-policy",
```

Create:

```text
crates/runtime-path-policy/Cargo.toml
crates/runtime-path-policy/src/lib.rs
crates/runtime-path-policy/src/state_root.rs
crates/runtime-path-policy/src/safe_path.rs
crates/runtime-path-policy/src/bounded_json.rs
crates/runtime-path-policy/src/atomic_write.rs
crates/runtime-path-policy/src/symlink_policy.rs
crates/runtime-path-policy/src/size_limits.rs
```

### 5.2 Cargo.toml

```toml
[package]
name = "runtime-path-policy"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
```

Add dependency to `crates/vida/Cargo.toml`:

```toml
runtime-path-policy = { path = "../runtime-path-policy" }
```

Later add to `taskflow-host-bridge`, `taskflow-core`, and `docflow-cli` as needed.

### 5.3 Implement types

#### `state_root.rs`

```rust
#[derive(Debug, Clone)]
pub struct StateRoot {
    raw: PathBuf,
    canonical: PathBuf,
}

impl StateRoot {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PathPolicyError>;
    pub fn raw(&self) -> &Path;
    pub fn canonical(&self) -> &Path;
    pub fn contains_canonical(&self, path: &Path) -> bool;
}
```

#### `safe_path.rs`

```rust
#[derive(Debug, Clone)]
pub struct ExistingRegularFile {
    path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct NewStateOutputPath {
    path: PathBuf,
}

#[derive(Debug, Clone, Copy)]
pub enum ArtifactPathKind {
    HostBridgeRequest,
    HostBridgePacket,
    HostBridgeResult,
    HostBridgeReceipt,
    DispatchPacket,
    DispatchResult,
    RuntimeSnapshot,
    TaskAttemptArtifact,
    DocflowChangedPath,
    GenericJson,
}
```

Implement:

```rust
pub fn existing_regular_file_under_root(
    root: &StateRoot,
    raw_path: impl AsRef<Path>,
    kind: ArtifactPathKind,
) -> Result<ExistingRegularFile, PathPolicyError>;

pub fn new_output_path_under_root(
    root: &StateRoot,
    raw_path: impl AsRef<Path>,
    kind: ArtifactPathKind,
    replace_existing: bool,
) -> Result<NewStateOutputPath, PathPolicyError>;
```

Rules:

```text
- reject dot-segment traversal
- canonicalize root
- symlink_metadata before canonicalize
- reject symlink
- require metadata.is_file for existing reads
- reject directories, FIFOs, device files
- canonicalize file and ensure starts_with(root.canonical)
- for new output: create/validate parent, canonicalize parent, parent under root
- if replace_existing=false: fail when path exists
- if replace_existing=true: if path exists, it must be non-symlink regular file
```

#### `bounded_json.rs`

```rust
pub const DEFAULT_JSON_ARTIFACT_MAX_BYTES: u64 = 1024 * 1024;
pub const HOST_BRIDGE_REQUEST_MAX_BYTES: u64 = 256 * 1024;
pub const HOST_BRIDGE_RESULT_MAX_BYTES: u64 = 1024 * 1024;
pub const TASK_ATTEMPT_ARTIFACT_MAX_BYTES: u64 = 1024 * 1024;

pub fn read_json_file<T: DeserializeOwned>(
    file: &ExistingRegularFile,
    max_bytes: u64,
) -> Result<T, PathPolicyError>;

pub fn read_json_value_file(
    file: &ExistingRegularFile,
    max_bytes: u64,
) -> Result<serde_json::Value, PathPolicyError>;
```

Rules:

```text
- check metadata.len <= max_bytes
- open file, read exactly bounded bytes
- JSON parse errors include path and artifact kind
```

#### `atomic_write.rs`

```rust
pub fn write_json_new<T: Serialize>(
    path: &NewStateOutputPath,
    value: &T,
) -> Result<(), PathPolicyError>;

pub fn write_json_replace<T: Serialize>(
    path: &NewStateOutputPath,
    value: &T,
) -> Result<(), PathPolicyError>;
```

Use temp file + rename if feasible. If not, keep existing OpenOptions pattern but behind this API.

### 5.4 Move/replace functions from `agent_dispatch_surface.rs`

Move these functions into `runtime-path-policy` or replace their body with calls:

| Current function | Source | New owner | Action |
|---|---|---|---|
| `path_contains_dot_segment` | `crates/vida/src/agent_dispatch_surface.rs` | `runtime-path-policy::safe_path` | Replace with shared traversal checker |
| `canonical_state_artifact_path` | same | `runtime-path-policy::safe_path` | Replace with `existing_regular_file_under_root` / `new_output_path_under_root` |
| Direct `std::fs::read_to_string` inside `retryable_host_bridge_completion_request_for_state_root` | same | `runtime-path-policy::bounded_json` | Replace with bounded JSON read |
| Direct artifact write helpers | same | `runtime-path-policy::atomic_write` | Replace when used |

### 5.5 Move/replace functions from `lane_surface.rs`

Move or replace:

| Current function | New owner | Notes |
|---|---|---|
| `path_has_dot_segment` | `runtime-path-policy::safe_path` | delete duplicate after all call sites updated |
| `canonical_state_root` | `runtime-path-policy::StateRoot` | typed state root |
| `canonicalize_existing_state_path` | `runtime-path-policy::existing_regular_file_under_root` | require regular file |
| `validate_new_state_artifact_path` | `runtime-path-policy::new_output_path_under_root` | preserve replace behavior |
| `validate_state_artifact_path_for_host_bridge_write` | `runtime-path-policy::new_output_path_under_root` | move host bridge wrapper later |
| `write_json_artifact_new` | `runtime-path-policy::write_json_new` | keep wrapper for one wave |
| `write_json_artifact_replace_existing` | `runtime-path-policy::write_json_replace` | keep wrapper for one wave |
| `read_host_bridge_request` | `taskflow-host-bridge::request::read_request` using `runtime-path-policy` | do not keep raw read |

### 5.6 Replace direct unsafe reads

Search and replace by policy:

```bash
rg "read_to_string" crates/vida/src crates/docflow-cli/src crates/taskflow-* crates/docflow-* -n
rg "Path::exists|\\.exists\\(" crates/vida/src crates/docflow-cli/src crates/taskflow-* crates/docflow-* -n
rg "std::fs::write|OpenOptions" crates/vida/src crates/docflow-cli/src crates/taskflow-* crates/docflow-* -n
```

For each hit, classify:

```yaml
allowed_direct_io:
  - reading static embedded templates only
  - writing non-runtime temp test fixtures
  - non-authority docs commands after path review

must_use_runtime_path_policy:
  - state root files
  - runtime-consumption snapshots
  - host bridge request/result/receipt
  - dispatch packet/result
  - task attempt artifact refs
  - receipt packs
  - status/doctor reconciliation paths
```

### 5.7 Proof

```bash
cargo test -p runtime-path-policy -- --nocapture
cargo test -p vida host_bridge -- --nocapture
cargo test -p vida --test doctor_surface_contract_smoke -- --nocapture
cargo test -p vida --test task_smoke task_attempt -- --nocapture
cargo test -p docflow-cli --test cli_smoke closeout -- --nocapture
```

---

## 6. Wave 2 — `operator-output`

### Goal

Centralize human command rendering, release-1 operator envelope, artifact refs, next actions, and compact TOON.

### 6.1 Create crate

Add workspace member:

```toml
"crates/operator-output",
```

Create:

```text
crates/operator-output/Cargo.toml
crates/operator-output/src/lib.rs
crates/operator-output/src/command_text.rs
crates/operator-output/src/envelope.rs
crates/operator-output/src/artifact_refs.rs
crates/operator-output/src/toon_report.rs
crates/operator-output/src/next_actions.rs
```

Dependencies:

```toml
common-format-toon = { path = "../common-format-toon" }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
toon-format = { workspace = true }
```

### 6.2 Move from `crates/vida/src/operator_command_text.rs`

Move:

```rust
human_command
```

New path:

```rust
operator_output::command_text::human_command
```

Keep old wrapper for one wave:

```rust
pub(crate) fn human_command(command: &str) -> String {
    operator_output::command_text::human_command(command)
}
```

### 6.3 Move from `crates/vida/src/operator_contracts.rs`

Move:

```text
OperatorContractSpec
FinalizedOperatorSurfaceVerdict
canonical_pass_blocked_contract_status_str
finalize_operator_surface_verdict
build_release1_operator_output_payload
replace_release1_operator_output_artifact_refs
canonical_next_action_entries
normalize_blocker_codes
```

New path:

```rust
operator_output::envelope
```

Keep `operator_contracts.rs` as facade for one wave.

### 6.4 Move from `crates/vida/src/operator_toon_report.rs`

Move:

```text
OperatorToonField
print
render
structured value encoding helpers
```

New path:

```rust
operator_output::toon_report
```

Keep facade in `vida`:

```rust
pub(crate) use operator_output::toon_report::*;
```

### 6.5 Move shared status/doctor next actions

From:

```text
crates/vida/src/status_surface_signals.rs
crates/vida/src/status_surface_operator_contracts.rs
crates/vida/src/doctor_surface.rs
```

Extract catalog to:

```rust
operator_output::next_actions
```

Functions to move or create:

```text
human_recovery_status_command(run_id)
human_lane_show_command(run_id)
human_run_graph_status_command(run_id)
human_task_next_lawful_command()
human_taskflow_graph_summary_command()
human_protocol_binding_repair_command()
human_closed_run_reconcile_command()
human_dependency_graph_repair_command()
```

### 6.6 Replace imports

Search:

```bash
rg "operator_command_text::human_command|crate::operator_command_text::human_command|human_command\\(" crates/vida/src
rg "operator_contracts::|crate::operator_contracts" crates/vida/src
rg "operator_toon_report" crates/vida/src
```

Replace with `operator_output::*` paths, leaving wrappers only where call-site churn is too large.

### 6.7 Proof

```bash
cargo test -p operator-output -- --nocapture
cargo test -p vida operator_toon_report -- --nocapture
cargo test -p vida release1_operator_output_payload -- --nocapture
cargo test -p vida status_and_doctor_default_human_output_is_compact_toon_with_explicit_json_parity -- --nocapture
cargo test -p vida ready_command_in_human_next_action -- --nocapture
```

---

## 7. Wave 3 — `taskflow-host-bridge`

### Goal

Move host bridge authority, provenance, request validation, completion evidence, implementation artifact attachment, and scope validation out of shell surfaces.

### 7.1 Create crate

Add workspace member:

```toml
"crates/taskflow-host-bridge",
```

Cargo dependencies:

```toml
runtime-path-policy = { path = "../runtime-path-policy" }
operator-output = { path = "../operator-output" }
taskflow-contracts = { path = "../taskflow-contracts" }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
time = { workspace = true }
```

### 7.2 Public API

Create `src/lib.rs`:

```rust
pub mod request;
pub mod provenance;
pub mod receipt_binding;
pub mod completion;
pub mod artifact_scope;
pub mod errors;

pub use request::{
    HostBridgeRequest,
    HostBridgeRequestPath,
    read_host_bridge_request,
};

pub use provenance::{
    validate_host_bridge_request_provenance,
    HostBridgeProvenanceInput,
    HostBridgeProvenanceDecision,
};

pub use receipt_binding::{
    validate_dispatch_receipt_binding,
    DispatchReceiptBindingInput,
    DispatchReceiptBindingDecision,
};

pub use completion::{
    materialize_host_bridge_completion_evidence,
    HostBridgeCompletionInput,
    HostBridgeCompletionEvidence,
};

pub use artifact_scope::{
    attach_host_bridge_implementation_artifact,
    validate_implementation_artifact_scope,
};
```

### 7.3 Fix HB-001 before move: explicit state root authority

#### Current problematic source

`agent_dispatch_surface.rs::host_bridge_request_provenance_blockers` currently chooses inferred state root if request path is outside provided state root.

#### New function

```rust
pub fn resolve_host_bridge_state_root(
    request_path: &Path,
    provided_state_root: Option<&Path>,
) -> Result<StateRoot, HostBridgeError>
```

Rules:

```text
1. If provided_state_root exists:
   1.1 Open canonical StateRoot(provided).
   1.2 If request_path is not under provided root:
       return Err(host_bridge_request_untrusted_path)
   1.3 Return provided root.
2. Else infer root from request_path.
3. If no root inferred, use default proxy state root only at shell adapter boundary.
4. Never replace explicit root with inferred root.
```

#### Implementation checklist

```text
[ ] Add `resolve_host_bridge_state_root` in `taskflow-host-bridge/src/state_root.rs`.
[ ] Add test `explicit_state_root_wins_over_inferred_request_root`.
[ ] Replace body of `agent_dispatch_surface::host_bridge_request_provenance_blockers`.
[ ] Run HB-001.
```

### 7.4 Fix HB-002 before move: missing receipt always blocks

#### Move these functions from `agent_dispatch_surface.rs`

```text
host_bridge_request_provenance_blockers
host_bridge_request_provenance_blockers_for_state_root
append_host_bridge_dispatch_receipt_blockers
host_bridge_request_matches_reconciled_blocked_status
modern_pending_host_bridge_request
pending_host_bridge_request_for_state_root
retryable_host_bridge_completion_request_for_state_root
retryable_host_bridge_completion_request
```

#### New ownership

```text
taskflow-host-bridge/src/provenance.rs
```

#### Rewrite contract

Replace heuristic suppression with strict authority chain:

```rust
pub async fn validate_host_bridge_request_provenance(
    input: HostBridgeProvenanceInput<'_>,
) -> HostBridgeProvenanceDecision
```

Inputs:

```rust
pub struct HostBridgeProvenanceInput<'a> {
    pub state_root: &'a StateRoot,
    pub request_path: &'a Path,
    pub request: &'a HostBridgeRequest,
    pub store: &'a dyn RunGraphReceiptReader,
}
```

Rules:

```text
- state store open error => host_bridge_dispatch_receipt_missing
- run_graph_dispatch_receipt Err(_) => host_bridge_dispatch_receipt_missing
- run_graph_dispatch_receipt Ok(None) => host_bridge_dispatch_receipt_missing
- request_target != receipt.dispatch_target => host_bridge_dispatch_receipt_mismatch
- request.backend_id != receipt.selected_backend => host_bridge_dispatch_receipt_mismatch
- request.packet_path != receipt.dispatch_packet_path => host_bridge_dispatch_receipt_mismatch
- receipt.dispatch_status must be routed/executing/bridge_request_pending OR retryable blocked with persisted evidence
- retryable request may permit completion retry but not parent host tool calls without receipt
```

Checklist:

```text
[ ] Copy existing functions to `provenance.rs`.
[ ] Add typed `HostBridgeProvenanceDecision { blocker_codes, canonical_paths, receipt_summary }`.
[ ] Delete `host_bridge_request_matches_reconciled_blocked_status` bypass.
[ ] Keep compatibility wrapper in `agent_dispatch_surface`.
[ ] Run HB-002.
```

### 7.5 Move request helpers from `agent_dispatch_surface.rs`

Move to `taskflow-host-bridge/src/request.rs`:

| Current function | New function | Change |
|---|---|---|
| `host_bridge_request_string` | `HostBridgeRequest::string(field)` | typed wrapper |
| `host_bridge_required_string` | `HostBridgeRequest::required(field)` | return typed error |
| `legacy_internal_subagents_host_bridge_request` | `HostBridgeRequest::is_legacy_internal_subagents` | no behavior change |
| `effective_host_bridge_request` | `HostBridgeRequest::effective` | no behavior change |
| `host_bridge_request_implementation_artifacts` | `HostBridgeRequest::implementation_artifacts` | typed Vec |
| `host_bridge_record_component` | `record_component` | move to `request.rs` or `artifact_scope.rs` |

Implementation steps:

```text
[ ] Define `HostBridgeRequest { raw: serde_json::Value }`.
[ ] Add getters for run_id, dispatch_target, packet_path, backend_id, result_path, receipt_path.
[ ] Add `effective()` method for legacy adapter defaults.
[ ] Keep `serde_json::Value` export for JSON compatibility.
[ ] Replace call sites in `host_bridge_adapter_payload`.
```

### 7.6 Move host bridge payload builder

Move from `agent_dispatch_surface.rs`:

```text
host_bridge_operator_fields
host_bridge_adapter_payload
emit_host_bridge_payload
host_bridge_completion_lane_args
```

Target:

```text
taskflow-host-bridge/src/request.rs          # pure decision payload
crates/vida/src/agent_dispatch_surface.rs    # output rendering only
```

Split:

```rust
// taskflow-host-bridge
pub fn build_host_bridge_adapter_decision(input: HostBridgeAdapterInput) -> HostBridgeAdapterDecision;

// vida shell
fn emit_host_bridge_payload(decision: &HostBridgeAdapterDecision, json: bool) -> ExitCode
```

Rules:

```text
- `host_tool_calls` may be non-empty only when provenance decision has no blockers.
- next_actions must use `operator-output::human_command`.
- output JSON shape remains identical for one wave.
```

### 7.7 Move artifact attachment

Move from `agent_dispatch_surface.rs`:

```text
attach_host_bridge_implementation_artifacts
emit_host_bridge_attach_blocked
host_bridge_artifact_file
host_bridge_changed_files_from_artifact
host_bridge_normalized_implementation_artifact_path
write_host_bridge_normalized_implementation_artifact
push_unique_host_bridge_implementation_artifact
normalized_host_bridge_attempt_id
normalized_host_bridge_consolidation_receipt_id
write_host_bridge_request
```

Target:

```text
taskflow-host-bridge/src/artifact_scope.rs
```

Fix during move:

```text
- artifact input path must be existing regular file under allowed scope or explicitly allowed external source with size limit.
- normalized artifact path must be new output path under state root.
- request write must reject symlink and use atomic write.
- duplicate detection should use stable key: source_artifact_ref + attempt_id + task_id + stage_id.
```

Checklist:

```text
[ ] Create `AttachArtifactInput`.
[ ] Return `AttachArtifactDecision`.
[ ] Move functions one by one.
[ ] Replace direct `std::fs::write` with runtime-path-policy atomic write.
[ ] Replace direct `read_to_string` with bounded read.
[ ] Run HB-005 and IO-001.
```

### 7.8 Fix HB-003: completion paths from persisted receipt/result only

Move from `lane_surface.rs`:

```text
HostBridgeReceiptPaths
HostBridgeCompletionRequestContext
trusted_host_bridge_completion_request_context
host_bridge_request_object
host_bridge_request_paths_from_dispatch_result
validated_host_bridge_paths_from_receipt
materialize_host_bridge_completion_evidence
```

Target:

```text
taskflow-host-bridge/src/receipt_binding.rs
taskflow-host-bridge/src/completion.rs
```

Required rewrite:

```text
- Remove `request_paths_authoritative`.
- Add `packet_path` to `HostBridgeReceiptPaths`.
- `trusted_host_bridge_completion_request_context` must receive persisted `RunGraphDispatchReceipt`.
- Request status must be pending or blocked.
- request.backend_id must match status.selected_backend.
- request result/receipt paths must match persisted dispatch result paths.
- request packet_path must match persisted packet path if persisted packet path exists.
- packet JSON must have run_id and dispatch_target matching active run.
```

Step-by-step:

```text
[ ] Copy `HostBridgeReceiptPaths` to `receipt_binding.rs`.
[ ] Add field `packet_path: Option<PathBuf>`.
[ ] Copy `host_bridge_request_object`.
[ ] Copy `host_bridge_request_paths_from_dispatch_result`.
[ ] Extend it to parse optional `packet_path`.
[ ] Copy `validated_host_bridge_paths_from_receipt`.
[ ] Delete `request_paths_authoritative` parameter in new version.
[ ] Replace fallback branch for `artifact_kind == host_tool_bridge_result` so it does not trust mutable request unless persisted result explicitly has legacy host bridge result and all paths are canonical.
[ ] Copy `trusted_host_bridge_completion_request_context`.
[ ] Add `receipt: &RunGraphDispatchReceipt` parameter.
[ ] Validate request/result/receipt/packet binding through `validated_host_bridge_paths_from_receipt`.
[ ] Update `lane_surface` call site to pass `&receipt`.
[ ] Update `materialize_host_bridge_completion_evidence` signature: remove `request_paths_authoritative`.
[ ] Update all tests.
```

### 7.9 Fix HB-004: safe request read

Move from `lane_surface.rs`:

```text
read_host_bridge_request
host_bridge_request_has_retryable_completion_evidence
```

Target:

```text
taskflow-host-bridge/src/request.rs
taskflow-host-bridge/src/completion.rs
```

Rewrite:

```text
read_host_bridge_request(state_root, request_path):
  - use runtime-path-policy existing_regular_file_under_root
  - require ArtifactPathKind::HostBridgeRequest
  - max bytes HOST_BRIDGE_REQUEST_MAX_BYTES
  - parse JSON
```

For retryable completion evidence:

```text
- validate request_path before reading request
- validate receipt_path/result_path as existing regular files
- bounded JSON read for both
- no raw read_to_string
```

### 7.10 Fix HB-005: implementation scope from immutable dispatch packet

Move from `lane_surface.rs`:

```text
host_bridge_implementation_artifacts
host_bridge_request_artifacts_are_taskflow_authorized
host_bridge_implementation_scope_validation
host_bridge_scope_validation_blocker_codes
host_bridge_completion_requires_implementation_artifacts
taskflow_implementation_artifacts_for_host_bridge_request
```

Target:

```text
taskflow-host-bridge/src/artifact_scope.rs
```

Rewrite:

```text
- Add `host_bridge_dispatch_packet_json(state_root, persisted_receipt)`.
- Add `host_bridge_dispatch_packet_implementation_isolation(state_root, persisted_receipt)`.
- `host_bridge_implementation_scope_validation` must receive persisted receipt.
- Isolation/owned_paths must come from persisted dispatch packet first.
- If request has artifacts but no immutable isolation exists: blocked with implementation_artifact_contract_invalid or implementation_artifact_authority_missing.
- Request-level owned_paths can be evidence only, not authority.
```

### 7.11 Update `agent_dispatch_surface.rs`

After moves, `agent_dispatch_surface.rs` should contain only:

```text
run_agent(args)
parse command args
open state store
call taskflow_host_bridge use cases
emit JSON/TOON
```

Delete or wrapper-only:

```text
host_bridge_request_provenance_blockers
host_bridge_adapter_payload
attach_host_bridge_implementation_artifacts
```

### 7.12 Update `lane_surface.rs`

After moves, `lane_surface.rs` should delegate host bridge logic:

```rust
let host_bridge_evidence =
    taskflow_host_bridge::complete_host_bridge_lane(input).await?;
```

It should not directly parse host bridge request/result/receipt paths.

### 7.13 Proof

```bash
cargo test -p taskflow-host-bridge -- --nocapture
cargo test -p vida host_bridge -- --nocapture --test-threads=1
cargo test -p vida --test doctor_surface_contract_smoke host_bridge -- --nocapture
cargo test -p vida --test boot_smoke taskflow_consume_continue_routes_receipt_backed_ready_downstream_taskflow_packet -- --nocapture
cargo check -p vida
```

---

## 8. Wave 4 — `taskflow-authority`

### Goal

Centralize persisted authority decisions: run graph, dispatch receipt, snapshots, cache, stale guards, terminal closure, exception takeover, root-write permission.

### 8.1 Create crate

```text
crates/taskflow-authority/
```

Cargo dependencies:

```toml
taskflow-contracts = { path = "../taskflow-contracts" }
runtime-path-policy = { path = "../runtime-path-policy" }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
```

### 8.2 Public modules

```text
src/authority_chain.rs
src/terminal_closure.rs
src/stale_guard.rs
src/exception_takeover.rs
src/projection_cache.rs
src/final_snapshot.rs
src/continuation_binding.rs
src/errors.rs
```

### 8.3 Move terminal/stale predicates

From `status_surface.rs` and `lane_surface.rs`, move or create:

```text
terminal_missing_task_closure_has_clean_dispatch_receipt
latest_run_graph_task_stale_for_write_guard
missing_task_stale_blocked_run_can_retire
lane_takeover_state
runtime_binding_has_active_exception_takeover
exception_takeover_state_label
```

Target:

```text
taskflow-authority/src/terminal_closure.rs
taskflow-authority/src/stale_guard.rs
taskflow-authority/src/exception_takeover.rs
```

### 8.4 Fix stale terminal closure

New function:

```rust
pub fn terminal_missing_task_closure_has_clean_dispatch_receipt(
    status: &RunGraphStatus,
    receipt: Option<&RunGraphDispatchReceipt>,
) -> bool
```

Rules:

```text
pass only if:
  - status is terminal closure
  - receipt exists
  - receipt.run_id == status.run_id
  - receipt.dispatch_status == executed
  - receipt.lane_status == lane_completed
  - receipt.exception_path_receipt_id is None
  - receipt.dispatch_target matches status terminal target when available
```

### 8.5 Fix stale/retire exception behavior

New function:

```rust
pub fn can_retire_missing_task_stale_blocked_run(
    status: &RunGraphStatus,
    receipt: &RunGraphDispatchReceipt,
) -> RetireDecision
```

Rules:

```text
allow:
  - blocked dispatch with blocked/running lane status where old behavior explicitly allowed
  - prelaunch packet_ready closed task run when receipt is clean
  - active exception takeover only when:
      lane_status == lane_exception_takeover
      exception_path_receipt_id present
      supersedes_receipt_id present
      active bounded unit closed/proven
deny:
  - lane_exception_recorded without supersession
  - bridge_request_pending without completion proof
  - executed exception takeover without closed active unit
```

### 8.6 Move projection cache admissibility

From `status_surface.rs`:

```text
cached_status_projection_admissible
cached_status_projection_matches_current_session
cached_projection_is_state_bound_read_only_operator_fallback
```

Target:

```text
taskflow-authority/src/projection_cache.rs
```

Rewrite:

```text
- no sessionless fallback
- require session/worktree identity when cache was produced under session-bound context
- include non-task operator state markers:
    - dispatch receipts
    - continuation bindings
    - run-graph updates
    - runtime-consumption snapshots
- derived state marker does not override changed operator state
```

### 8.7 Move final snapshot fallback

From `runtime_consumption_state.rs` / status / doctor related code:

```text
latest_final_runtime_consumption_dispatch_receipt_summary
latest_final_runtime_consumption_snapshot_path
latest_recorded_final_runtime_consumption_snapshot_path
latest_terminal_consume_continue_snapshot_run_id
```

Target:

```text
taskflow-authority/src/final_snapshot.rs
```

Rewrite:

```text
latest_final_runtime_consumption_dispatch_receipt_summary(store, snapshot):
  - read latest persisted RunGraphStatus
  - snapshot.run_id must match latest status.run_id
  - read persisted dispatch receipt for run_id
  - build summary only from persisted receipt
  - return None if persisted receipt missing
  - never construct authority from raw final snapshot JSON
```

### 8.8 Update callers

Replace in:

```text
status_surface.rs
doctor_surface.rs
consume_final_operator_surface.rs
taskflow_consume_resume_projection.rs
taskflow_consume_resume_receipt.rs
lane_surface.rs
task_surface.rs
```

Pattern:

```rust
let decision = taskflow_authority::stale_guard::compute(...);
```

Shell surfaces render decision only.

### 8.9 Proof

```bash
cargo test -p taskflow-authority -- --nocapture
cargo test -p vida status_surface -- --nocapture
cargo test -p vida doctor_surface -- --nocapture
cargo test -p vida --test task_smoke terminal_exception_takeover_run_does_not_reemit_missing_task_retire_action -- --nocapture
cargo test -p vida --test doctor_surface_contract_smoke -- --nocapture
```

---

## 9. Wave 5 — Runtime contracts and typed blocker/status codes

### Goal

Replace stringly typed status/blocker/dispatch/lane codes with typed contracts.

### 9.1 Extend `taskflow-contracts`

Add modules:

```text
crates/taskflow-contracts/src/blocker_code.rs
crates/taskflow-contracts/src/lane_status.rs
crates/taskflow-contracts/src/dispatch_status.rs
crates/taskflow-contracts/src/run_graph_status.rs
crates/taskflow-contracts/src/artifact_kind.rs
crates/taskflow-contracts/src/runtime_role.rs
crates/taskflow-contracts/src/task_class.rs
```

### 9.2 Move from `release1_contracts.rs` / `operator_contracts.rs`

Move or duplicate temporarily:

```text
BlockerCode
LaneStatus
derive_lane_status
canonical_blocker_code_str
canonical_blocker_code_value_from_str
canonical_blocker_code_list
canonical_parametric_blocker_code_value
```

New owner:

```rust
taskflow_contracts::blocker_code
taskflow_contracts::lane_status
```

### 9.3 Migration rule

Do not mass-rewrite all strings at once. Use boundary functions first:

```rust
impl TryFrom<&str> for DispatchStatus
impl DispatchStatus {
    pub fn as_str(&self) -> &'static str;
    pub fn is_active_for_host_bridge(&self) -> bool;
}
```

### 9.4 Replace only authority-sensitive sites first

Replace strings in:

```text
taskflow-host-bridge
taskflow-authority
runtime_dispatch_execution
runtime_dispatch_state
lane_surface wrappers
status/doctor authority wrappers
```

### 9.5 Proof

```bash
cargo test -p taskflow-contracts -- --nocapture
cargo test -p vida blocker_code_registry_parity -- --nocapture
cargo test -p vida release1_operator_output_payload -- --nocapture
```

---

## 10. Wave 6 — Backend admissibility and agent-init role resolution

### Goal

Fix routing/role bugs and extract them into a canonical TaskFlow routing policy.

### 10.1 Create or extend module

Preferred target:

```text
crates/taskflow-core/src/routing/
  mod backend_admissibility.rs
  mod role_resolution.rs
  mod dispatch_alias.rs
```

If `taskflow-core` is not ready, create in `crates/vida/src/runtime_assignment_policy.rs` first, then promote.

### 10.2 Move backend admissibility logic

From:

```text
crates/vida/src/runtime_dispatch_state.rs
crates/vida/src/runtime_dispatch_execution.rs
crates/vida/src/runtime_contract_vocab.rs
crates/vida/src/runtime_assignment_policy.rs
```

Move/create:

```rust
pub fn backend_admissibility_key_for_task_class(task_class: TaskClass) -> BackendAdmissibilityKey;

pub fn backend_admissibility_key_for_dispatch_target(
    dispatch_target: &str,
    dispatch_contract_lane: Option<&DispatchContractLane>,
) -> BackendAdmissibilityKey;
```

Rules:

```text
- canonical task_class wins over human dispatch target label
- developer -> implementation if dispatch contract says task_class=implementation
- tester/test_authoring -> verification if dispatch contract says verification semantics
- architecture aliases use architecture strictness
- unknown labels default to fail-closed or conservative key
```

### 10.3 Move agent-init role resolution

From `init_surfaces.rs`:

```text
resolve_agent_init_explicit_role
agent_init_selected_role_allowed
dev_team role alias resolution helpers
flow-step runtime_role resolution helpers
legacy alias mapping helpers
```

Target:

```text
taskflow-core/src/routing/role_resolution.rs
```

Rules:

```text
- direct role cannot be orchestrator for agent-init worker lane
- dev_team.roles.runtime_role cannot resolve to orchestrator
- dev_team.flows.steps.runtime_role cannot resolve to orchestrator
- legacy alias mapping cannot resolve to orchestrator
- explicit orchestrator only allowed in orchestrator-init, never agent-init
```

### 10.4 Update callers

```text
init_surfaces.rs:
  call taskflow_core::routing::resolve_agent_init_role

runtime_dispatch_execution.rs:
  call taskflow_core::routing::backend_admissibility_key_for_dispatch_target

runtime_dispatch_state.rs:
  call same shared function
```

### 10.5 Proof

```bash
cargo test -p vida agent_init_explicit_role_rejects_dev_team_orchestrator_runtime_role_aliases -- --nocapture
cargo test -p vida backend_admissibility -- --nocapture
cargo test -p vida runtime_dispatch_execution -- --nocapture
cargo test -p vida --test boot_smoke -- --nocapture
```

---

## 11. Wave 7 — DocFlow external process hardening

### Goal

Move safe git process execution out of `docflow-cli/src/lib.rs` and prevent repo-local config/helper execution.

### 11.1 Create module

Option A:

```text
crates/docflow-core/src/git_status.rs
```

Option B:

```text
crates/docflow-cli/src/git_hardening.rs
```

Prefer A if reused beyond CLI.

### 11.2 Move from `docflow-cli/src/lib.rs`

Move:

```text
git_null_config_path
run_git_status_with_timeout
changed_markdown_paths
```

Target:

```rust
docflow_core::git_status
```

### 11.3 Rewrite contract

```rust
pub struct SafeGitStatusInput {
    pub root: PathBuf,
    pub pathspecs: Vec<String>,
    pub timeout: Duration,
}

pub fn changed_markdown_paths(input: SafeGitStatusInput) -> Result<Vec<String>, DocflowError>
```

Rules:

```text
- env GIT_CONFIG_NOSYSTEM=1
- env GIT_CONFIG_GLOBAL=/dev/null or NUL
- env_remove GIT_CONFIG_COUNT
- env_remove GIT_CONFIG_KEY_0
- env_remove GIT_CONFIG_VALUE_0
- env_remove GIT_CONFIG_PARAMETERS
- args: -c core.fsmonitor=false
- args: -c core.hooksPath=
- timeout default 10s
- no shell execution; use Command args only
```

### 11.4 Proof

```bash
cargo test -p docflow-core git_status -- --nocapture
cargo test -p docflow-cli closeout_changed_disables_repo_local_fsmonitor_helper -- --nocapture
cargo test -p docflow-cli --test cli_smoke closeout -- --nocapture
```

---

## 12. Wave 8 — TaskFlow run graph and continuation decomposition

### Goal

Decompose `taskflow_run_graph.rs`, `taskflow_proxy.rs`, and consume continuation logic into owner modules.

### 12.1 Target layout

```text
crates/taskflow-core/src/run_graph/
  mod model.rs
  mod status.rs
  mod recovery.rs
  mod closure.rs
  mod continuation.rs
  mod projections.rs
  mod stale.rs

crates/taskflow-core/src/consume/
  mod continue_use_case.rs
  mod resume_input.rs
  mod resume_state_machine.rs
  mod resume_reconciliation.rs
  mod resume_projection.rs
  mod resume_receipt.rs
  mod final_snapshot.rs

crates/taskflow-core/src/scheduling/
  mod graph_summary.rs
  mod next_lawful.rs
  mod scheduler_dispatch.rs
  mod actualize.rs
  mod route_explain.rs
```

### 12.2 Move from `taskflow_consume_resume.rs`

Already extracted boundaries exist:

```text
taskflow_consume_resume_output
taskflow_consume_resume_projection
taskflow_consume_resume_receipt
```

Next functions to move:

```text
resume input parsing helpers
run_id mismatch gates
dispatch packet contract error classification
stale missing-task retire guidance
deferred routed agent lane handoff logic
ready downstream routing logic
lock/state access blocker payload builder
consume continue orchestration decision builders
```

Exact mechanical plan:

```text
[ ] Open `crates/vida/src/taskflow_consume_resume.rs`.
[ ] Use `rg "^fn |^pub\\(crate\\) fn |^async fn |^pub\\(crate\\) async fn"`.
[ ] Group functions into:
    A. input resolution
    B. packet validation
    C. receipt state
    D. projection output
    E. state mutation orchestration
    F. rendering
[ ] Move group A to `taskflow-core/src/consume/resume_input.rs`.
[ ] Move group B to `taskflow-core/src/consume/packet_validation.rs`.
[ ] Move group C to `taskflow-core/src/consume/resume_receipt.rs`.
[ ] Keep group E as `continue_use_case.rs`.
[ ] Delete local output/render helpers that duplicate `operator-output`.
```

### 12.3 Move from `taskflow_proxy.rs`

Group and move:

```text
graph-summary builders -> taskflow-core/src/scheduling/graph_summary.rs
next-lawful decision policy -> taskflow-core/src/scheduling/next_lawful.rs
scheduler dispatch preview -> taskflow-core/src/scheduling/scheduler_dispatch.rs
route explain / validate routing -> taskflow-core/src/scheduling/route_explain.rs
proxy command parse/render -> taskflow-cli/src/proxy.rs
```

### 12.4 Move from `taskflow_run_graph.rs`

Group and move:

```text
run graph model/default constructors -> taskflow-contracts or taskflow-core/run_graph/model.rs
recovery summary classification -> taskflow-core/run_graph/recovery.rs
terminal closure predicates -> taskflow-authority
continuation binding summary -> taskflow-core/run_graph/continuation.rs
status rendering inputs -> taskflow-core/run_graph/projections.rs
```

### 12.5 Proof

```bash
cargo test -p vida taskflow_consume_resume -- --nocapture
cargo test -p vida taskflow_consume_resume_projection -- --nocapture
cargo test -p vida taskflow_consume_resume_receipt -- --nocapture
cargo test -p vida --test boot_smoke taskflow_consume_continue_default_output_is_compact_toon -- --nocapture
cargo test -p vida --test boot_smoke taskflow_consume_continue_routes_receipt_backed_ready_downstream_taskflow_packet -- --nocapture
cargo test -p vida task_next_lawful -- --nocapture
cargo test -p vida scheduler_dispatch -- --nocapture
```

---

## 13. Wave 9 — Task command use cases

### Goal

Turn `task_surface.rs` into a shell adapter. Move domain mutations and policies into TaskFlow.

### 13.1 Target modules

```text
crates/taskflow-core/src/task/
  mod create.rs
  mod update.rs
  mod note.rs
  mod block.rs
  mod verify.rs
  mod close.rs
  mod progress.rs
  mod closure_ready.rs
  mod dependencies.rs
  mod attempts.rs
  mod handoff.rs
  mod takeover.rs
  mod reconcile.rs
  mod split.rs
  mod spawn_blocker.rs
  mod import_export.rs
```

### 13.2 Function extraction groups

Use this command to list candidates:

```bash
rg "^fn |^pub\\(crate\\) fn |^async fn |^pub\\(crate\\) async fn" crates/vida/src/task_surface.rs
```

Move by group:

#### Group A — import/export/replace

Target:

```text
taskflow-core/src/task/import_export.rs
```

Move functions related to:

```text
import-jsonl validation
replace-jsonl application
export-jsonl generation
graph validation before import
blocked import operator envelope
```

Apply fix:

```text
operator envelope uses operator-output shared builder
retry commands use human_command without --json
```

#### Group B — progress/closure-ready

Target:

```text
taskflow-core/src/task/progress.rs
taskflow-core/src/task/closure_ready.rs
```

Move functions related to:

```text
direct child progress
parent closure candidacy
epic/non-epic child completion
proof status display
```

Preserve fix:

```text
any non-closed parent with all direct children closed-like is closure-ready, not only issue_type=epic
```

#### Group C — dependencies

Target:

```text
taskflow-core/src/task/dependencies.rs
```

Move functions related to:

```text
dep add
dep ensure
dep add-bulk
dep remove
dependency graph validation
critical path
blocked tasks
```

Preserve fix:

```text
task dep ensure output must render `vida task dep ensure`, not add-bulk
retry guidance must include task_id, depends_on_id, edge_type
```

#### Group D — attempt ledger

Target:

```text
taskflow-core/src/task/attempts.rs
```

Move functions related to:

```text
attempt dispatch
attempt status
attempt collect
attempt consolidate
attempt record
attempt transition
attempt summary
```

Apply IO fix:

```text
artifact refs must use runtime-path-policy existing_regular_file_under_root
max bytes = TASK_ATTEMPT_ARTIFACT_MAX_BYTES
no Path::exists + read_to_string
```

#### Group E — close/handoff/takeover

Target:

```text
taskflow-core/src/task/close.rs
taskflow-core/src/task/handoff.rs
taskflow-core/src/task/takeover.rs
```

Move functions related to:

```text
task close automation guard
handoff accept receipts
exception takeover status
proof status
closure gates
```

Preserve fixes:

```text
blocked automation must fail task close
proof command hints quote task ids
closed-run continuation projection parity
```

#### Group F — split/spawn/reconcile

Target:

```text
taskflow-core/src/task/split.rs
taskflow-core/src/task/spawn_blocker.rs
taskflow-core/src/task/reconcile.rs
```

Move functions related to:

```text
split child specs
spawn blocker preview/mutation
adaptive replan finding preview
reconcile closed runs
```

### 13.3 Shell adapter target

After moving groups, `task_surface.rs` should look like:

```rust
pub(crate) async fn run_task(args: TaskArgs) -> ExitCode {
    match args.command {
        TaskCommand::Create(args) => taskflow_cli::task::create(args).await,
        TaskCommand::Update(args) => taskflow_cli::task::update(args).await,
        ...
    }
}
```

No direct state transition logic remains in `vida`.

### 13.4 Proof

```bash
cargo test -p taskflow-core task -- --nocapture
cargo test -p vida --test task_smoke -- --nocapture
cargo test -p vida direct_child_progress_marks_non_epic_parent_ready_when_children_are_closed -- --nocapture
cargo test -p vida task_dependency_ensure_reports_ensure_surface_in_json_results -- --nocapture
cargo test -p vida append_task_notes_preserves_existing_notes -- --nocapture
```

---

## 14. Wave 10 — Init and project activation decomposition

### Goal

Separate bootstrap discovery, project activation, role resolution, config validation, materialization, and rendering.

### 14.1 Target layout

```text
crates/vida-init-core/
  src/lib.rs
  src/bootstrap_discovery.rs
  src/project_activation.rs
  src/launcher_snapshot.rs
  src/agent_role_resolution.rs
  src/template_materialization.rs
  src/config_validation.rs
  src/init_projection.rs

crates/vida/src/init_surfaces.rs
  shell adapter only
```

If creating a crate is too large, use:

```text
crates/vida/src/init/
  mod bootstrap_discovery.rs
  mod project_activation.rs
  mod launcher_snapshot.rs
  mod agent_role_resolution.rs
  mod template_materialization.rs
  mod config_validation.rs
  mod init_projection.rs
```

Then promote later.

### 14.2 Move from `init_surfaces.rs`

Group functions:

```text
orchestrator init view builders -> init_projection.rs
agent init role resolution -> agent_role_resolution.rs
project activator view builders -> project_activation.rs
launcher activation snapshot sync -> launcher_snapshot.rs
template/materialization -> template_materialization.rs
bootstrap source resolution -> bootstrap_discovery.rs
config validation -> config_validation.rs
```

### 14.3 Apply routing fix

In `agent_role_resolution.rs`:

```rust
fn selected_role_allowed_for_agent_init(role: RuntimeRole) -> bool {
    role != RuntimeRole::Orchestrator
}
```

Apply this check after every resolution branch:

```text
direct role
dev_team.roles.runtime_role
dev_team.flows.steps.runtime_role
legacy alias mapping
fallback role
```

### 14.4 Proof

```bash
cargo test -p vida init_surfaces::agent_init_surface_tests::agent_init_explicit_role_rejects_dev_team_orchestrator_runtime_role_aliases -- --nocapture
cargo test -p vida init_command_succeeds -- --nocapture
cargo test -p vida --test boot_smoke orchestrator_init -- --nocapture
cargo test -p vida --test boot_smoke agent_init -- --nocapture
```

---

## 15. Wave 11 — Root command router and shell cleanup

### Goal

Make `vida` shell routing explicit, small, and debug-friendly.

### 15.1 Keep in `vida`

Do not move yet:

```text
cli.rs
root_command_router.rs
main.rs
shell_runtime_helpers.rs
surface_render.rs
```

These are shell-owned and allowed.

### 15.2 Refactor root state binding into child module

Create:

```text
crates/vida/src/root_state_binding.rs
```

Move from `root_command_router.rs`:

```text
task_command_has_explicit_state_dir
task_command_needs_project_root
agent_command_needs_project_root
diagnostics_command_explicit_state_dir
orchestrator_session_command_explicit_state_dir
session_command_has_explicit_state_dir
session_command_needs_project_root
proxy_args_request_help_or_version
proxy_command_needs_project_root
command_needs_project_root_state_dir
RuntimeStateDirGuard
prepare_runtime_state_dir_for_parse
prepare_runtime_state_dir
command_explicit_state_dir
command_preserves_explicit_env_state_dir
command_preserves_parse_only_env_state_dir
bind_runtime_state_dir_for_project_bound_command
bind_runtime_state_dir_override_for_project_bound_command
bind_runtime_state_dir_to_current_project
normalize_runtime_state_dir_override
normalize_runtime_state_dir_env_for_parse
preserve_runtime_state_dir_env_for_project_bound_command
preserve_runtime_state_dir_env_for_parse_only
```

Leave in `root_command_router.rs`:

```text
run_root_command
command_label
```

### 15.3 Add trace envelope

Create:

```rust
pub struct RootCommandTrace {
    pub command_label: String,
    pub state_dir_source: StateDirSource,
    pub state_dir: Option<PathBuf>,
    pub project_root: Option<PathBuf>,
    pub session_id: Option<String>,
}
```

Emit only under debug/timing env, not default output.

### 15.4 Remove broad re-exports

In `main.rs`, replace:

```rust
pub(crate) use runtime_dispatch_state::*;
```

With explicit exports only while migrating:

```rust
pub(crate) use runtime_dispatch_state::{
    write_runtime_dispatch_result,
    runtime_lane_completion_summary_blocker_code,
    ...
};
```

Then remove entirely after TaskFlow crates expose required APIs.

### 15.5 Proof

```bash
cargo test -p vida uppercase_help_flag_is_normalized_for_windows_operator_habit -- --nocapture
cargo test -p vida init_command_succeeds -- --nocapture
cargo test -p vida --test boot_smoke -- --nocapture
cargo test -p vida --test doctor_surface_contract_smoke -- --nocapture
```

---

## 16. Wave 12 — Facade removal and architecture lint

### Goal

Remove legacy glue and enforce owner boundaries.

### 16.1 Remove transitional facades

Delete wrappers only after all call sites are migrated:

```text
crates/vida/src/operator_command_text.rs
crates/vida/src/operator_contracts.rs
crates/vida/src/operator_toon_report.rs
host bridge helper wrappers in agent_dispatch_surface.rs
host bridge helper wrappers in lane_surface.rs
runtime dispatch path helpers duplicated in runtime_dispatch_state.rs
```

If removal causes high churn, convert files to thin re-export modules for one release only and create removal task.

### 16.2 Add lint script

Create:

```text
scripts/check-runtime-boundaries.ps1
scripts/check-runtime-boundaries.sh
```

Checks:

```bash
# no direct read_to_string in runtime authority modules
rg "std::fs::read_to_string|read_to_string\\(" crates/vida/src crates/taskflow-* crates/docflow-* \
  -g '!**/tests/**' \
  -g '!crates/runtime-path-policy/**'

# no mutable request authority terms in shell
rg "request_paths_authoritative|modern_pending_host_bridge_request|reconciled_blocked_status" crates/vida/src

# no broad runtime_dispatch_state export
rg "pub\\(crate\\) use runtime_dispatch_state::\\*" crates/vida/src

# no direct string blocker codes outside contract/tests, allowlist generated/adapters
rg "\"host_bridge_|\"implementation_artifact_|\"stale_|\"blocked_dispatch" crates/vida/src crates/taskflow-* \
  -g '!**/tests/**' \
  -g '!crates/taskflow-contracts/**'
```

### 16.3 Add CI/check gate

Add to quality gate:

```bash
scripts/check-runtime-boundaries.ps1
cargo fmt --all -- --check
git diff --check
cargo test -p runtime-path-policy -- --nocapture
cargo test -p operator-output -- --nocapture
cargo test -p taskflow-host-bridge -- --nocapture
cargo test -p taskflow-authority -- --nocapture
cargo test -p vida --test boot_smoke -- --nocapture
cargo test -p vida --test doctor_surface_contract_smoke -- --nocapture
cargo test -p vida --test task_smoke -- --nocapture
cargo test -p docflow-cli --test cli_smoke -- --nocapture
```

---

## 17. Exact first implementation batch

This is the first concrete agent-executable batch. Do this before starting broader decomposition.

### Batch 1A — Runtime path policy crate

```text
[ ] Add workspace member `crates/runtime-path-policy`.
[ ] Add crate Cargo.toml.
[ ] Add `PathPolicyError`.
[ ] Add `StateRoot`.
[ ] Add `ArtifactPathKind`.
[ ] Add `existing_regular_file_under_root`.
[ ] Add `new_output_path_under_root`.
[ ] Add `read_json_value_file`.
[ ] Add `write_json_new`.
[ ] Add `write_json_replace`.
[ ] Add tests:
    [ ] rejects_dot_segments
    [ ] rejects_symlink_existing_file
    [ ] rejects_directory_existing_file
    [ ] rejects_out_of_root_existing_file
    [ ] rejects_oversized_json
    [ ] accepts_regular_json_under_root
[ ] Add dependency to `crates/vida/Cargo.toml`.
[ ] Run `cargo test -p runtime-path-policy -- --nocapture`.
```

### Batch 1B — Safe host bridge request read

```text
[ ] In `taskflow-host-bridge` or temporary `crates/vida/src/host_bridge_request_policy.rs`, implement:
    [ ] read_host_bridge_request_under_root(state_root, request_path)
[ ] Replace `lane_surface::read_host_bridge_request` body with wrapper.
[ ] Replace `agent_dispatch_surface` direct request read call sites if any.
[ ] Update `host_bridge_request_has_retryable_completion_evidence`:
    [ ] validate request_path before read
    [ ] validate receipt_path before read
    [ ] validate result_path before read
    [ ] use bounded JSON read
[ ] Add HB-004 test.
[ ] Run focused host bridge tests.
```

### Batch 1C — Explicit state root authority fix

```text
[ ] In `agent_dispatch_surface::host_bridge_request_provenance_blockers`:
    [ ] replace `(Some(_provided), Some(inferred)) => inferred`
    [ ] with `(Some(provided), Some(_inferred)) => provided`
    [ ] if request path not under provided root, do not use inferred; return untrusted path blocker.
[ ] Add HB-001 test.
[ ] Run focused test.
```

### Batch 1D — Missing receipt always blocks

```text
[ ] In `host_bridge_request_provenance_blockers_for_state_root`:
    [ ] store open Err => push host_bridge_dispatch_receipt_missing
    [ ] remove pending heuristics suppression
[ ] In `append_host_bridge_dispatch_receipt_blockers`:
    [ ] Err(_) => push host_bridge_dispatch_receipt_missing
    [ ] Ok(None) => push host_bridge_dispatch_receipt_missing
    [ ] remove reconciled blocked target mismatch early return
[ ] Delete or deprecate:
    [ ] host_bridge_request_matches_reconciled_blocked_status
    [ ] modern_pending_host_bridge_request
    [ ] pending_host_bridge_request_for_state_root
[ ] Add HB-002 test.
[ ] Run focused test.
```

### Batch 1E — Completion paths from persisted receipt

```text
[ ] In `HostBridgeReceiptPaths`, add:
    [ ] packet_path: Option<PathBuf>
[ ] In `host_bridge_request_paths_from_dispatch_result`:
    [ ] parse optional packet_path
[ ] In `trusted_host_bridge_completion_request_context`:
    [ ] add receipt parameter
    [ ] canonicalize request path under state root
    [ ] require request.status pending/blocked
    [ ] require request.backend_id == status.selected_backend
    [ ] call validated_host_bridge_paths_from_receipt
    [ ] validate result/receipt path equality
    [ ] validate packet_path equality if persisted packet path exists
    [ ] read packet via bounded safe read
    [ ] require packet.run_id == run_id
    [ ] require packet.dispatch_target == request.dispatch_target
[ ] In `validated_host_bridge_paths_from_receipt`:
    [ ] remove `request_paths_authoritative`
    [ ] remove branch that trusts mutable request result_path/receipt_path
[ ] In `materialize_host_bridge_completion_evidence`:
    [ ] remove `request_paths_authoritative`
    [ ] use persisted packet path for packet_path field when available
[ ] Update call site in `run_lane`.
[ ] Add HB-003 test.
```

### Batch 1F — Immutable implementation scope

```text
[ ] Add helper `host_bridge_dispatch_packet_json(state_root, persisted_receipt)`.
[ ] Add helper `host_bridge_dispatch_packet_implementation_isolation(state_root, persisted_receipt)`.
[ ] Change `host_bridge_implementation_scope_validation` signature:
    old: (request, artifacts, authority)
    new: (state_root, request, persisted_receipt, artifacts, authority)
[ ] Make owned_paths source:
    [ ] persisted dispatch packet implementation_isolation.owned_paths
    [ ] never request.owned_paths
[ ] If request has implementation_artifacts but persisted packet has no implementation_isolation:
    [ ] block with implementation_artifact_contract_invalid
[ ] Add HB-005 test.
```

### Batch 1G — Proof batch

```bash
cargo fmt --all -- --check
git diff --check
cargo test -p runtime-path-policy -- --nocapture
cargo test -p vida host_bridge -- --nocapture --test-threads=1
cargo test -p vida --test doctor_surface_contract_smoke -- --nocapture
cargo check -p vida
```

---

## 18. Function move register

This register is intentionally explicit. Agent should take one row at a time.

### 18.1 `agent_dispatch_surface.rs`

| # | Function / item | Destination | Fix now? | Notes |
|---:|---|---|---|---|
| A001 | `host_bridge_record_component` | `taskflow-host-bridge::request` | no | used for stable filenames |
| A002 | `host_bridge_changed_files_from_artifact` | `taskflow-host-bridge::artifact_scope` | yes | normalize changed files and validate scope |
| A003 | `host_bridge_request_implementation_artifacts` | `taskflow-host-bridge::request` | no | typed accessor |
| A004 | `push_unique_host_bridge_implementation_artifact` | `taskflow-host-bridge::artifact_scope` | yes | key should include task/stage |
| A005 | `host_bridge_normalized_implementation_artifact_path` | `taskflow-host-bridge::artifact_scope` | yes | use runtime-path-policy output path |
| A006 | `write_host_bridge_normalized_implementation_artifact` | `runtime-path-policy::atomic_write` wrapper | yes | no symlink write-through |
| A007 | `path_contains_dot_segment` | `runtime-path-policy::safe_path` | yes | delete duplicate |
| A008 | `canonical_state_artifact_path` | `runtime-path-policy::safe_path` | yes | replace all call sites |
| A009 | `host_bridge_request_string` | `taskflow-host-bridge::request` | no | typed getter |
| A010 | `host_bridge_request_provenance_blockers` | `taskflow-host-bridge::provenance` | yes | explicit state root wins |
| A011 | `infer_host_bridge_state_root_from_request_path` | `taskflow-host-bridge::state_root` | no | use only when no explicit root |
| A012 | `host_bridge_request_path_is_under_state_root` | `runtime-path-policy::StateRoot` | yes | canonical root check |
| A013 | `host_bridge_request_provenance_blockers_for_state_root` | `taskflow-host-bridge::provenance` | yes | missing receipt always blocks |
| A014 | `append_host_bridge_dispatch_receipt_blockers` | `taskflow-host-bridge::receipt_binding` | yes | no heuristic suppression |
| A015 | `host_bridge_request_matches_reconciled_blocked_status` | delete | yes | unsafe bypass |
| A016 | `modern_pending_host_bridge_request` | delete | yes | request not authority |
| A017 | `pending_host_bridge_request_for_state_root` | delete | yes | request not authority |
| A018 | `host_bridge_artifact_has_retryable_completion_blocker` | `taskflow-host-bridge::completion` | yes | safe bounded artifact read |
| A019 | `retryable_host_bridge_completion_request_for_state_root` | `taskflow-host-bridge::completion` | yes | safe read receipt/result |
| A020 | `retryable_host_bridge_completion_request` | `taskflow-host-bridge::completion` | yes | no inferred root over explicit root |
| A021 | `host_bridge_operator_fields` | `operator-output::envelope` or bridge output builder | no | preserve envelope shape |
| A022 | `legacy_internal_subagents_host_bridge_request` | `taskflow-host-bridge::request` | no | preserve legacy effective adapter |
| A023 | `effective_host_bridge_request` | `taskflow-host-bridge::request` | no | typed effective request |
| A024 | `host_bridge_adapter_payload` | `taskflow-host-bridge::request` decision builder | yes | host_tool_calls only after provenance pass |
| A025 | `emit_host_bridge_payload` | keep in `vida` shell | no | rendering only |
| A026 | `host_bridge_completion_lane_args` | `taskflow-host-bridge::completion` or shell adapter | no | command string via operator-output |
| A027 | `attach_host_bridge_implementation_artifacts` | `taskflow-host-bridge::artifact_scope` | yes | safe IO + receipt binding |
| A028 | `emit_host_bridge_attach_blocked` | `operator-output` / shell render | no | output only |
| A029 | `host_bridge_artifact_file` | `runtime-path-policy::bounded_json` | yes | regular file + size limit |
| A030 | `write_host_bridge_request` | `runtime-path-policy::atomic_write` | yes | atomic, no symlink |

### 18.2 `lane_surface.rs`

| # | Function / item | Destination | Fix now? | Notes |
|---:|---|---|---|---|
| L001 | `HostBridgeReceiptPaths` | `taskflow-host-bridge::receipt_binding` | yes | add `packet_path` |
| L002 | `HostBridgeCompletionRequestContext` | `taskflow-host-bridge::completion` | yes | include validated packet context |
| L003 | `read_host_bridge_request` | `taskflow-host-bridge::request` | yes | safe bounded read |
| L004 | `host_bridge_path_string` | `taskflow-host-bridge::request` | no | typed getter |
| L005 | `trusted_host_bridge_completion_request_context` | `taskflow-host-bridge::completion` | yes | require persisted receipt |
| L006 | `path_has_dot_segment` | `runtime-path-policy::safe_path` | yes | delete duplicate |
| L007 | `canonical_state_root` | `runtime-path-policy::StateRoot` | yes | typed state root |
| L008 | `canonicalize_existing_state_path` | `runtime-path-policy::safe_path` | yes | require regular file |
| L009 | `validate_new_state_artifact_path` | `runtime-path-policy::safe_path` | yes | output path |
| L010 | `validate_state_artifact_path_for_host_bridge_write` | `runtime-path-policy::safe_path` | yes | no symlink overwrite |
| L011 | `host_bridge_request_object` | `taskflow-host-bridge::receipt_binding` | no | persisted result parsing |
| L012 | `host_bridge_request_paths_from_dispatch_result` | `taskflow-host-bridge::receipt_binding` | yes | parse packet path |
| L013 | `validated_host_bridge_paths_from_receipt` | `taskflow-host-bridge::receipt_binding` | yes | remove request authority |
| L014 | `write_json_artifact_new` | `runtime-path-policy::atomic_write` | yes | wrapper temporarily |
| L015 | `write_json_artifact_replace_existing` | `runtime-path-policy::atomic_write` | yes | wrapper temporarily |
| L016 | `host_bridge_implementation_artifacts` | `taskflow-host-bridge::artifact_scope` | yes | request artifacts need authority |
| L017 | `host_bridge_request_artifacts_are_taskflow_authorized` | `taskflow-host-bridge::artifact_scope` | yes | stable authority key |
| L018 | `host_bridge_implementation_scope_validation` | `taskflow-host-bridge::artifact_scope` | yes | immutable packet scope |
| L019 | `host_bridge_scope_validation_blocker_codes` | `taskflow-host-bridge::artifact_scope` | no | typed blocker later |
| L020 | `host_bridge_completion_requires_implementation_artifacts` | `taskflow-host-bridge::artifact_scope` | no | move constant policy |
| L021 | `taskflow_implementation_artifacts_for_host_bridge_request` | `taskflow-host-bridge::artifact_scope` | yes | request task_id must match run/task |
| L022 | `host_bridge_completion_retryable_blocker` | `taskflow-contracts::blocker_code` | no | wrapper temporarily |
| L023 | `host_bridge_artifact_has_retryable_completion_blocker` | `taskflow-host-bridge::completion` | yes | safe read |
| L024 | `host_bridge_request_has_retryable_completion_evidence` | `taskflow-host-bridge::completion` | yes | safe request read |
| L025 | `host_bridge_completion_request_required` | `taskflow-host-bridge::completion` | yes | use typed dispatch status |
| L026 | `materialize_host_bridge_completion_evidence` | `taskflow-host-bridge::completion` | yes | persisted paths only |
| L027 | `missing_task_stale_blocked_run_can_retire` | `taskflow-authority::stale_guard` | yes | recorded exception not active |
| L028 | `lane_mutation_status_guard` | `taskflow-authority` or `taskflow-core::lane_lifecycle` | no | later wave |
| L029 | `lane_takeover_state` | `taskflow-authority::exception_takeover` | yes | central active takeover |
| L030 | `run_lane` host bridge branch | keep shell adapter | yes | call extracted use case |

### 18.3 `status_surface.rs` / `doctor_surface.rs`

| # | Function / item | Destination | Fix now? |
|---:|---|---|---|
| S001 | `cached_status_projection_admissible` | `taskflow-authority::projection_cache` | yes |
| S002 | `cached_status_projection_matches_current_session` | `taskflow-authority::projection_cache` | yes |
| S003 | `cached_projection_is_state_bound_read_only_operator_fallback` | delete | yes |
| S004 | `terminal_missing_task_closure_has_clean_dispatch_receipt` | `taskflow-authority::terminal_closure` | yes |
| S005 | `latest_run_graph_task_stale_for_write_guard` | `taskflow-authority::stale_guard` | yes |
| S006 | status/doctor next action constants | `operator-output::next_actions` | no |
| S007 | retrieval trust / protocol binding operator contracts | `taskflow-authority` + `operator-output` | yes |
| S008 | status/doctor TOON rendering | `operator-output::toon_report` | no |

### 18.4 `runtime_consumption_state.rs`

| # | Function / item | Destination | Fix now? |
|---:|---|---|---|
| RCS001 | `latest_final_runtime_consumption_dispatch_receipt_summary` | `taskflow-authority::final_snapshot` | yes |
| RCS002 | `latest_final_runtime_consumption_snapshot_path` | `taskflow-authority::final_snapshot` | no |
| RCS003 | `latest_recorded_final_runtime_consumption_snapshot_path` | `taskflow-authority::final_snapshot` | no |
| RCS004 | `latest_terminal_consume_continue_snapshot_run_id` | `taskflow-authority::final_snapshot` | yes |
| RCS005 | retrieval trust signal helpers | `taskflow-authority` | yes |

### 18.5 `init_surfaces.rs`

| # | Function / item | Destination | Fix now? |
|---:|---|---|---|
| I001 | `resolve_agent_init_explicit_role` | `taskflow-core::routing::role_resolution` | yes |
| I002 | `agent_init_selected_role_allowed` | `taskflow-core::routing::role_resolution` | yes |
| I003 | dev_team role alias lookup | `taskflow-core::routing::role_resolution` | yes |
| I004 | flow-step runtime_role lookup | `taskflow-core::routing::role_resolution` | yes |
| I005 | orchestrator init projection builders | `vida-init-core::init_projection` | no |
| I006 | project activation builders | `vida-init-core::project_activation` | no |
| I007 | bootstrap source root resolution | `vida-init-core::bootstrap_discovery` | no |
| I008 | template materialization | `vida-init-core::template_materialization` | no |

### 18.6 `runtime_dispatch_state.rs` / `runtime_dispatch_execution.rs`

| # | Function / item | Destination | Fix now? |
|---:|---|---|---|
| D001 | backend admissibility key helpers | `taskflow-core::routing::backend_admissibility` | yes |
| D002 | dispatch target alias normalization | `taskflow-core::routing` | yes |
| D003 | host bridge request/result receipt helpers | `taskflow-host-bridge` | yes |
| D004 | path policy helpers | `runtime-path-policy` | yes |
| D005 | result evidence helpers | existing `runtime_dispatch_result_evidence` then TaskFlow | no |
| D006 | lane completion summary blocker classifier | existing `runtime_dispatch_lane_completion` then TaskFlow | no |
| D007 | packet text rendering | `operator-output` / taskflow packet text | no |
| D008 | receipt projection helpers | `taskflow-authority` / TaskFlow | yes |

### 18.7 `docflow-cli/src/lib.rs`

| # | Function / item | Destination | Fix now? |
|---:|---|---|---|
| DOC001 | `git_null_config_path` | `docflow-core::git_status` | yes |
| DOC002 | `run_git_status_with_timeout` | `docflow-core::git_status` | yes |
| DOC003 | `changed_markdown_paths` | `docflow-core::git_status` | yes |
| DOC004 | closeout verdict builders | already `closeout_verdict` | no |
| DOC005 | closeout rendering | CLI shell | no |

---

## 19. Option evaluation used for final architecture

### Option 1 — Mechanical split inside `crates/vida`

Pros:
- Lowest immediate churn.
- Fastest compile feedback.

Cons:
- Keeps ownership wrong.
- Does not prevent future runtime truth drift into shell.
- Shared libraries remain unavailable to TaskFlow/DocFlow.

Decision:
- Use only as temporary staging when crate extraction is blocked.

### Option 2 — Extract shared crates first

Pros:
- Best long-term ownership.
- Prevents duplicated path/authority/operator-output bugs.
- Enables focused tests.

Cons:
- More workspace/Cargo churn.
- Requires careful facade compatibility.

Decision:
- Chosen for safety primitives: `runtime-path-policy`, `operator-output`, `taskflow-host-bridge`, `taskflow-authority`.

### Option 3 — Move all TaskFlow runtime into `taskflow-core` immediately

Pros:
- Matches ownership law.

Cons:
- Too large and risky.
- High call-site churn.
- Hard to isolate bugfixes.

Decision:
- Not chosen as first move. Do after safety/host-bridge/authority crates stabilize.

### Option 4 — Keep current layout and only patch bugs

Pros:
- Fastest near-term patching.

Cons:
- Repeated defect class will recur.
- Debugging remains difficult.
- Violates project goals.

Decision:
- Not admissible as final architecture.

### Final hybrid

```text
1. Shared safety crates first.
2. Host bridge extraction second.
3. Authority/stale/cache extraction third.
4. TaskFlow domain use cases fourth.
5. vida shell cleanup last.
```

Confidence: 88%
Residual risk: Cargo/workspace churn and test runtime cost. Mitigation: facade-first moves and focused proofs per wave.

---

## 20. Final acceptance checklist

The refactor is complete only when all are true:

```text
[ ] `crates/vida` has no runtime authority decisions except routing/rendering.
[ ] No `pub(crate) use runtime_dispatch_state::*`.
[ ] Host bridge logic lives outside `agent_dispatch_surface.rs` and `lane_surface.rs`.
[ ] Explicit state root always wins.
[ ] Missing receipt always blocks host bridge.
[ ] Mutable request JSON is never authority for result/receipt/packet/scope.
[ ] Artifact reads are regular-file + under-root + size-limited.
[ ] Runtime snapshots cannot mint receipt authority.
[ ] Projection cache is session/worktree/operator-state aware.
[ ] Agent-init cannot resolve worker aliases to orchestrator.
[ ] Backend admissibility uses canonical task_class.
[ ] DocFlow closeout cannot execute repo-local Git helpers.
[ ] Operator JSON/human/TOON output uses one shared renderer.
[ ] Blocker/status codes use typed contracts at authority boundaries.
[ ] Each moved boundary has focused tests.
[ ] Public smoke tests pass.
[ ] Architecture lint passes.
```

---

## 21. Suggested commit sequence

Use small commits:

```text
01 test: add host bridge authority regression fixtures
02 feat: add runtime-path-policy crate
03 fix: route host bridge request reads through bounded path policy
04 fix: keep explicit host bridge state root authoritative
05 fix: fail closed missing host bridge dispatch receipts
06 fix: bind host bridge completion paths to persisted receipt evidence
07 fix: validate implementation artifacts against immutable dispatch packet scope
08 feat: add operator-output crate and move human command rendering
09 feat: move release-1 operator envelope to operator-output
10 feat: add taskflow-authority crate
11 fix: terminal closure stale guard requires clean receipt
12 fix: recorded exception receipt is not active takeover authority
13 fix: validate runtime-consumption final snapshot fallback against persisted receipt
14 fix: harden projection cache admission
15 fix: backend admissibility uses canonical task_class
16 fix: agent-init rejects orchestrator role aliases
17 fix: docflow changed closeout isolates git config and timeout
18 refactor: move host bridge decision builders to taskflow-host-bridge
19 refactor: move taskflow consume resume policies to taskflow-core consume modules
20 refactor: move task lifecycle use cases below shell
21 refactor: split root state binding module
22 chore: remove legacy facades and broad re-exports
23 test: add runtime boundary lint and full smoke proof
```

---

## 22. Agent execution notes

For every checklist item:

```text
1. Create a small branch or commit.
2. Move exactly one function group.
3. Keep wrapper in old location until compile is green.
4. Run the smallest focused test.
5. Run adjacent public smoke.
6. Only then delete old helper.
7. Record proof command in docs/product/spec/meta-refactor-runtime-boundary-execution-plan.md.
```

If a moved function needs more than three unrelated dependencies, stop and create an input/output DTO instead of importing half of the old module.

If a dependency cycle appears, do not add reverse dependency to `vida`. Move the shared type down into `taskflow-contracts`, `vida-contracts`, or `runtime-path-policy`.

If tests require old behavior that violates authority/fail-closed rules, update the test to the new canonical behavior and document the defect fix.

---

## 23. Minimal next action for implementation agent

Start here:

```text
1. Create `crates/runtime-path-policy`.
2. Add safe path and bounded JSON APIs.
3. Add tests for symlink/FIFO/out-of-root/oversized JSON.
4. Patch `lane_surface::read_host_bridge_request`.
5. Patch `agent_dispatch_surface::canonical_state_artifact_path` call sites through wrappers.
6. Add HB-001/HB-004 tests.
7. Run focused host bridge tests.
```

Do not start broad TaskFlow decomposition until this IO/authority baseline is green.

-----
artifact_path: product/spec/meta-refactor-runtime-boundary-source-plan
artifact_type: document
artifact_version: 1
artifact_revision: 2026-06-12
schema_version: 1
status: canonical
source_path: docs/product/spec/meta-refactor-runtime-boundary-source-plan.md
created_at: 2026-06-12T10:55:45+03:00
updated_at: 2026-06-12T10:55:45+03:00
changelog_ref: meta-refactor-runtime-boundary-source-plan.changelog.jsonl
