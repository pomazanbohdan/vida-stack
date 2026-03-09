# Binary Foundation Staged Asset

This folder is the staged Binary Foundation transfer asset.

Apply it in the final development environment only after the next agent confirms:

1. `Binary Foundation` is the active lawful wave,
2. no newer continuation artifact supersedes it,
3. root/runtime mutation is allowed there.

Suggested application:

1. copy `Cargo.toml` to the final environment root,
2. copy `crates/vida/` into the final environment workspace,
3. optionally merge `Makefile.binary-foundation.template` into the final environment runbook or root Makefile,
4. run proof:
   - `cargo test`
   - `cargo run -- --help`
   - `cargo run -- boot`
   - `cargo run -- boot --help`
   - `cargo run -- task --help`
   - `cargo run -- memory --help`
   - `cargo run -- status --help`
   - `cargo run -- doctor --help`

