# VIDA bounded Kani proofs

This is a standalone verification crate so normal workspace builds do not
require Kani. On a Kani-enabled Linux toolchain run:

```bash
CARGO_UNSTABLE_IGNORE_RUST_VERSION=1 \
  cargo kani --ignore-rust-version \
  --manifest-path verification/kani/Cargo.toml
```

The harness is bounded and targets the pure TaskFlow transition/version
invariant. It does not claim full runtime or concurrency correctness.

The repository toolchain remains pinned to Rust 1.97.1. Kani 0.67 bundles its
own Rust 1.93 nightly, so the semantic gate passes the compatibility override
above: Cargo metadata receives `CARGO_UNSTABLE_IGNORE_RUST_VERSION=1`, and
Kani's build invocation receives `--ignore-rust-version`. This bypasses only
Cargo's package metadata check; it does not change the compiler or the bounded
proof semantics. The gate first checks that the installed `cargo-kani` exposes
the flag and reports a typed `blocked` result when an unpatched bundle does not.
Kani 0.67 does not expose Cargo's `--locked` option in its own CLI; the
checked-in `verification/kani/Cargo.lock` remains the dependency lock source,
and the gate intentionally avoids passing an unsupported flag.
