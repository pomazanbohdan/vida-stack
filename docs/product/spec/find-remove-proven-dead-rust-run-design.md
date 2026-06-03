# Find Remove Proven Dead Rust Run Design

Purpose:
Define the bounded dead-code removal pass for the architecture refactor epic. The work removes only Rust code that is proven unreachable from CLI command surfaces, runtime dispatch paths, TaskFlow/DocFlow proxies, tests, generated packet normalization, and release/install entrypoints.

## Bounded Scope

This slice is discovery-first and conservative. It may remove private Rust functions, modules, or tests only when every candidate has direct usage evidence showing no references from:

1. `crates/vida/src/cli.rs` and command-family routing.
2. TaskFlow, DocFlow, lane, recovery, status, and dispatch runtime surfaces.
3. Runtime packet builders, proxy/adapters, generated packet normalization, and installed launcher proof paths.
4. Existing integration and smoke tests that exercise operator-visible behavior.

This slice must not rename public command surfaces, remove compatibility aliases, or simplify help/output behavior. Those are owned by the command help and output-interface tasks in the parent architecture refactor epic.

## Architectural Decision

Use a two-tier dead-code classifier:

1. Candidate discovery: collect compiler warnings, `rg` reference evidence, module visibility, test references, and command/runtime entrypoint reachability.
2. Removal admission: remove only candidates that are private, have zero non-definition references, and are outside compatibility, proxy, and generated-packet repair paths.

If a symbol is unused but plausibly reserved for a command, option, runtime packet field, compatibility migration, or future release contract, keep it and record the rationale instead of removing it.

## Protected Entrypoints

The implementation packet must treat these as protected unless a later bounded design explicitly authorizes changes:

1. Root `vida` command routing and all `--help` surfaces.
2. `vida task`, `vida taskflow`, `vida docflow`, `vida lane`, `vida status`, `vida doctor`, `vida orchestrator-init`, and `vida agent-init`.
3. Runtime dispatch packet creation, packet repair/normalization, result synthesis, exception takeover, lane supersede/complete, and continuation binding.
4. Release install/build proof paths and Windows launcher/runtime path projection.

## Proof Plan

The developer lane must produce:

1. A candidate table with symbol/module, reference evidence, risk classification, and keep/remove decision.
2. `rg` or compiler evidence for each removed symbol showing no remaining references.
3. Focused Rust tests for any behavior-adjacent removal.
4. `cargo fmt -p vida --check`.
5. At minimum `cargo test -p vida` for the affected modules, plus `cargo build -p vida --release` before release/install continuation.

## Candidate Table

This table is the admission gate for follow-up dead-code removal packets. It records current analysis evidence only; it does not authorize source deletion by itself.

| Candidate area | Evidence source | Reachability / risk | Decision | Follow-up proof |
| --- | --- | --- | --- | --- |
| `crates/vida/src/release_contract_adapters.rs` adapter helpers | `cargo check -p vida` with warnings completed cleanly; subagent analysis reported `blocker_code`, `canonical_blocker_codes`, and `release_contract_status` have callers. | Release-1 contract adapters are compatibility-sensitive and can be reached by runtime bundle, taskflow consume bundle, and operator contract tests. | Keep called helpers. Audit only private helpers with fresh zero-reference evidence: `blocker_code_str`, `boot_compatibility_is_backward_compatible`. | Exact `rg` checks per helper plus focused release contract tests before any removal. |
| `state_store_run_graph_state.rs`, `state_store_run_graph_summary.rs`, `state_store_instruction_bundle.rs` suppression clusters | Subagent analysis found dead-code suppressions concentrated in protected state-store/runtime files. | High risk: these files back run-graph recovery, continuation binding, and installed runtime status surfaces. | Keep by default. Split into one cluster audit packet at a time. | For each symbol: zero non-definition references, no packet/runtime reachability, and targeted run-graph/status tests. |
| Dependency-level unused code | `cargo machete` was unavailable on this host. | Tooling evidence is absent; Cargo dependency changes can affect build features and release packaging. | No dependency removals in this packet. | Separate tooling-approved packet to install or run an approved dependency dead-code tool. |
| Command/help/output entrypoints | This design explicitly protects root command routing, TaskFlow/DocFlow/lane/status/doctor/init/help surfaces. | Very high risk: entrypoints may be indirectly reached by CLI aliases, tests, generated packets, or release/install proof. | Keep. Do not remove in dead-code packets. | Covered by command help/output interface tasks, not this work-pool. |

## Rejected Approaches

1. Bulk removal from compiler warnings only, rejected because runtime command and packet surfaces include indirect reachability.
2. Removing public command or option aliases in this slice, rejected because help/output parity has separate acceptance targets.
3. Splitting oversized modules in the same implementation packet, rejected because that belongs to `architecture-refactor-oversized-module-split` and would mix risk domains.

-----
artifact_path: product/spec/find-remove-proven-dead-rust-run-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-06-02
schema_version: 1
status: canonical
source_path: docs/product/spec/find-remove-proven-dead-rust-run-design.md
created_at: 2026-06-02T23:16:52.9321129Z
updated_at: 2026-06-02T23:18:32.2756038Z
changelog_ref: find-remove-proven-dead-rust-run-design.changelog.jsonl
