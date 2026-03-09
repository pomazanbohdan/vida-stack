# VIDA Binary Foundation Staging Bundle

Purpose: keep the first Binary Foundation scaffold in a transferable folder inside `_vida` so it can be applied in the final development environment without mutating this repository root.

Contents:

1. `Cargo.toml`
2. `crates/vida/Cargo.toml`
3. `crates/vida/src/main.rs`
4. `crates/vida/src/temp_state.rs`
5. `crates/vida/tests/boot_smoke.rs`
6. `Makefile.binary-foundation.template`

Expected external-environment application:

1. copy `Cargo.toml` to repo root,
2. copy `crates/vida/*` into the external repo workspace,
3. optionally merge `Makefile.binary-foundation.template` into the external environment runbook or Makefile,
4. run `cargo test`,
5. run:
   - `cargo run -- --help`
   - `cargo run -- boot`
   - `cargo run -- boot --help`
   - `cargo run -- task --help`
   - `cargo run -- memory --help`
   - `cargo run -- status --help`
   - `cargo run -- doctor --help`
