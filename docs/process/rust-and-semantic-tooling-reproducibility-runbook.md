# Rust And Semantic Tooling Reproducibility Runbook

Status: active project process doc

Purpose: record the exact Rust/tooling deployment used by vida-stack, the
reproducible setup path for semantic testing, and the bootstrap route an agent
must use before touching Rust proofs or local gates.

## Scope And Boundary

This runbook covers:

1. the repository Rust pin and host/WSL toolchain split;
2. Windows/MSVC, Linux/WSL, fuzzing, Loom, Kani, and Miri tools;
3. semantic gate commands, target-dir/artifact policy, and typed blocked states;
4. Kani 0.67 compatibility installation and its checked-in patch;
5. bootstrap routing for an agent working without VIDA runtime activation.

It does not activate VIDA runtime, dispatch TaskFlow commands, create
authoritative receipts/effects, or replace the semantic-testing protocol.
Production runtime behavior remains a separate proof surface.

## Bootstrap Read Order

When an agent starts a Rust, TaskFlow-test, semantic-gate, or verification task:

1. read AGENTS.md;
2. read AGENTS.sidecar.md;
3. read docs/project-root-map.md;
4. read this runbook;
5. read docs/process/semantic-testing-and-local-gates-protocol.md;
6. inventory scripts and inspect the gate help surface:

~~~powershell
rg --files scripts | Sort-Object
pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Help
~~~

For this semantic slice, the sequential-versus-parallel posture is:

| Surface | Posture |
| --- | --- |
| read-only inventory, version checks, docs discovery | parallel-safe |
| Cargo commands sharing .vida/cargo-target | serialized |
| fuzz/Loom/Kani/Miri manual profiles | one profile at a time per target/toolchain |
| writes, staging, commit, release/install, runtime activation | serialized |

If VIDA runtime is unavailable or intentionally excluded, use the direct
repository scripts and static documentation checks. Do not fabricate TaskFlow
steps or receipts.

## Canonical Pins

### Repository Rust

rust-toolchain.toml is authoritative:

~~~toml
[toolchain]
channel = "1.97.1"
profile = "minimal"
components = ["clippy", "rustfmt"]
~~~

Cargo.toml repeats workspace.package.rust-version = "1.97.1".
The same exact stable release is required on Windows and WSL/Linux:

~~~text
rustc 1.97.1 (8bab26f4f 2026-07-14)
cargo 1.97.1 (c980f4866 2026-06-30)
~~~

Verify the active override from the repository root:

~~~powershell
rustup show active-toolchain
rustc -Vv
cargo -V
rustup component list --installed
~~~

~~~bash
rustup show active-toolchain
rustc -Vv
cargo -V
rustup component list --installed
~~~

rustup update 1.97.1 is an idempotent refresh, not a request to move the
project to an unpinned stable channel. Never edit the project pin to satisfy a
tool whose own compatibility bundle is older.

### Host Matrix

| Host | Role | Required tools/components |
| --- | --- | --- |
| Windows PowerShell | normal Cargo/MSVC development and P0/P1 pre-push | rustup, Rust 1.97.1 MSVC, clippy, rustfmt, PowerShell Core, pre-commit |
| WSL AlmaLinux-10 | Linux-only semantic profiles | Rust 1.97.1 GNU, cargo-fuzz, cargo-kani, cargo-miri, Git, nightly-2026-08-11 components |
| Kani bundle | bounded proof compiler/tooling | Kani 0.67.0 bundle, bundled rustc 1.93.0-nightly, patched driver |
| nightly-2026-08-11 | Miri and cargo-fuzz | cargo-miri, miri, rust-src |
| nightly-2025-11-21 | Kani source rebuild | rustc-dev, rust-src, llvm-tools, rustfmt |

Observed WSL tool paths:

~~~text
/home/unnamed/.cargo/bin/rustup
/home/unnamed/.cargo/bin/rustc
/home/unnamed/.cargo/bin/cargo
/home/unnamed/.cargo/bin/cargo-fuzz
/home/unnamed/.cargo/bin/cargo-kani
/home/unnamed/.cargo/bin/cargo-miri
~~~

Observed WSL versions:

~~~text
cargo-fuzz 0.13.2
cargo-kani 0.67.0
miri 0.1.0 (12c36e2539 2026-08-10)
~~~

## Initial Installation

### Windows/MSVC

Install rustup with the normal approved Windows distribution, then install the
repository pin and components:

~~~powershell
rustup toolchain install 1.97.1 --profile minimal
rustup component add clippy rustfmt --toolchain 1.97.1-x86_64-pc-windows-msvc
rustup show active-toolchain
~~~

Use scripts/vida-windows-env.ps1 and scripts/vida-cargo-msvc.ps1 for MSVC
proof. Do not hand-roll VsDevCmd.bat, vcvars64.bat, or a second Cargo target
directory. Do not change the global `rustup default`; repository
`rust-toolchain.toml` selects the pinned stable toolchain for this checkout.

### WSL/Linux semantic tools

Run from WSL with the repository root mounted at /mnt/c/project/vida-stack:

~~~bash
cd /mnt/c/project/vida-stack
rustup toolchain install 1.97.1 --profile minimal
rustup component add clippy rustfmt --toolchain 1.97.1-x86_64-unknown-linux-gnu
rustup toolchain install nightly-2026-08-11 --profile minimal
rustup component add miri rust-src --toolchain nightly-2026-08-11-x86_64-unknown-linux-gnu
rustup toolchain install nightly-2025-11-21
rustup component add rustc-dev rust-src llvm-tools-preview rustfmt \
  --toolchain nightly-2025-11-21-x86_64-unknown-linux-gnu
cargo install --locked cargo-fuzz
cargo install --locked kani-verifier --version 0.67.0
~~~

Run cargo kani setup after installing kani-verifier; it installs the official
Kani 0.67 bundle under $HOME/.kani/kani-0.67.0.

## Kani Compatibility Deployment

Kani 0.67.0 currently bundles rustc 1.93.0-nightly, while this repository
declares Rust 1.97.1. The repository therefore keeps the stable Rust pin and
uses an explicit compatibility path:

1. the gate checks that cargo-kani --help exposes --ignore-rust-version;
2. the gate scopes CARGO_UNSTABLE_IGNORE_RUST_VERSION=1 to the Kani run;
3. the patched Kani driver forwards --ignore-rust-version to Cargo build/rustc;
4. unsupported/unpatched Kani is typed blocked, never green.

The exact source patch is
verification/kani/kani-0.67.0-cargo-compat.patch. Rebuild it from the Kani
tag when the installed bundle must be repaired:

~~~bash
set -euo pipefail
WORK_DIR="$(mktemp -d)"
git clone --depth 1 --branch kani-0.67.0 \
  https://github.com/model-checking/kani.git "$WORK_DIR/kani"
cd "$WORK_DIR/kani"
git submodule update --init --recursive
git apply /mnt/c/project/vida-stack/verification/kani/kani-0.67.0-cargo-compat.patch
rustup component add rustc-dev rust-src llvm-tools-preview rustfmt \
  --toolchain nightly-2025-11-21-x86_64-unknown-linux-gnu
cargo build-dev

BUNDLE="$HOME/.kani/kani-0.67.0"
test -x "$BUNDLE/bin/kani-driver"
if [[ ! -e "$BUNDLE/bin/kani-driver.release-0.67.0.orig" ]]; then
  cp "$BUNDLE/bin/kani-driver" "$BUNDLE/bin/kani-driver.release-0.67.0.orig"
fi
install -m 0755 target/kani/bin/kani-driver "$BUNDLE/bin/kani-driver"
sha256sum target/kani/bin/kani-driver "$BUNDLE/bin/kani-driver"
~~~

The installed deployment verified for this repository is:

~~~text
cargo-kani 0.67.0
bundle rustc 1.93.0-nightly (53732d5e0 2025-11-20)
patched kani-driver sha256:
8b3d4eb59713facd1dbd6e3b94984f5b9c41bb582a13fbc7f1d6f2a9ab99ce3c
~~~

Run the bounded proof:

~~~bash
cd /mnt/c/project/vida-stack
CARGO_UNSTABLE_IGNORE_RUST_VERSION=1 cargo kani --ignore-rust-version --manifest-path verification/kani/Cargo.toml
~~~

Expected result: VERIFICATION:- SUCCESSFUL,
Complete - 1 successfully verified harnesses, 0 failures.
The latest verified WSL run took 53.26 seconds; timing is hardware/cache
dependent. This is a bounded proof, not a full runtime or concurrency
guarantee.

## Semantic Profile Commands

The profile owner is scripts/vida-dev-gate.ps1. Every run writes JSON summary,
timings, evidence refs, and command logs under
.vida/tmp/semantic-testing/<run-id>/.

~~~powershell
# P0/P1, automatic pre-push profile
pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode semantic-focused -Json

# P2-P4, manual profiles
pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode semantic-fuzz -Json
pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode semantic-loom -Json
pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode semantic-kani -Json
pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode semantic-miri -Json
~~~

Linux-only profiles must be run from WSL/Linux. Windows returns not_applicable
for Kani, fuzz, and Loom according to project policy. Missing tools or
incompatible versions return blocked with a reason.

### Fuzz

The fuzz workspace contains five pure parser/renderer targets:

~~~text
config_json
jsonl_decoder
cli_parser
workflow_payload
toon_render
~~~

The semantic-fuzz profile uses pinned nightly-2026-08-11, checked-in seeds,
-runs=64, and run-local corpus/artifact directories. It writes only those
run-local test artifacts; it does not write production/runtime state or invoke
VIDA.
The pinned `cargo fuzz check` completed successfully on the current WSL host.

### Loom

The targeted proof is the reservation claim integration test:

~~~bash
cd /mnt/c/project/vida-stack
RUSTFLAGS="--cfg loom" cargo test --manifest-path crates/taskflow-authority/Cargo.toml --test loom_claim_reservation --release --locked
~~~

It is a targeted interleaving proof for the adapter boundary, not a guarantee
for the entire runtime.

### Miri

Use the pinned nightly Miri toolchain and the pure path/effects test only:

~~~bash
cd /mnt/c/project/vida-stack
RUSTUP_TOOLCHAIN=nightly-2026-08-11 cargo miri test -p taskflow-core --lib path_policy
~~~

Observed result on the pinned WSL toolchain: 7 path-policy tests passed, 0
failed. The Miri profile writes only its target cache and test artifacts; it
does not make production/runtime state authoritative.

Miri is not a replacement for Loom and is not expanded to unsafe/FFI or
concurrency claims in this slice.

## Pre-push And Pre-commit

pre-commit remains fast hygiene/script-check only. The filename-independent
vida-semantic-prepush hook runs P0/P1 serially using the existing target-dir
policy. The checked-in pre-commit entry is a Windows `cmd.exe` shim because the
active developer host is Windows; from WSL/Linux, reproduce the same P0/P1
proof directly:

~~~powershell
pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode semantic-focused -Json
~~~

P2-P4 remain manual Linux profiles.

Verify hook/config setup:

~~~powershell
pre-commit validate-config
git diff --check
~~~

Mutation testing remains a separate adequacy gate (quality-cycle and
quality-pack); it is not replaced by semantic profiles.

## Reproduction Checklist

1. Confirm rust-toolchain.toml and Cargo.toml still pin 1.97.1.
2. Confirm active rustup toolchain and required components.
3. Confirm WSL-only tool versions and Kani driver hash.
4. Run scripts/vida-dev-gate.ps1 -Mode script-check -Json.
5. Run scripts/vida-dev-gate.ps1 -Mode semantic-focused -Json.
6. Run Linux manual profiles only when their tools are installed.
7. Inspect the profile summary.json; blocked and not_applicable are explicit
   outcomes, never implicit passes.
8. Run git diff --check and keep Cargo target operations serialized.

## Evidence And Maintenance

Version/tool changes require updating this runbook, its changelog, and the
semantic-testing protocol in the same documentation slice. Do not record
credentials, raw payloads, or machine-local temporary source paths as required
inputs. Machine-local paths above are installation observations; commands use
$HOME, repository-relative paths, and explicit WSL mounts for reproduction.

-----
artifact_path: process/rust-and-semantic-tooling-reproducibility-runbook
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-08-12'
schema_version: '1'
status: canonical
source_path: docs/process/rust-and-semantic-tooling-reproducibility-runbook.md
created_at: '2026-08-12T00:00:00+03:00'
updated_at: '2026-08-12T00:00:00+03:00'
changelog_ref: rust-and-semantic-tooling-reproducibility-runbook.changelog.jsonl
