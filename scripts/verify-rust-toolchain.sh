#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MINIMUM_VERSION="${VIDA_RUST_MINIMUM_VERSION:-1.97.1}"
RUSTC_BIN="${RUSTC:-${HOME:-}/.cargo/bin/rustc}"
RUSTUP_BIN="${RUSTUP:-${HOME:-}/.cargo/bin/rustup}"
CARGO_BIN="${CARGO:-${HOME:-}/.cargo/bin/cargo}"

fail() {
  printf '[rust-toolchain] ERROR: %s\n' "$*" >&2
  exit 1
}

version_number() {
  local version="$1"
  local major minor patch
  IFS=. read -r major minor patch <<< "$version"
  printf '%d%03d%03d\n' "$major" "$minor" "$patch"
}

if [[ ! -x "$RUSTC_BIN" ]]; then
  RUSTC_BIN="$(command -v rustc || true)"
fi
if [[ -z "$RUSTC_BIN" ]]; then
  fail "Unable to resolve rustc from $HOME/.cargo/bin or PATH"
fi
rustc_output="$("$RUSTC_BIN" --version)" || fail "rustc --version failed"
actual_version="$(printf '%s\n' "$rustc_output" | sed -E 's/^rustc ([0-9]+\.[0-9]+\.[0-9]+).*/\1/')"
[[ "$actual_version" != "$rustc_output" ]] || fail "Unable to parse rustc version: $rustc_output"

if (( $(version_number "$actual_version") < $(version_number "$MINIMUM_VERSION") )); then
  fail "Rust $actual_version is below required minimum $MINIMUM_VERSION"
fi

if [[ ! -x "$RUSTUP_BIN" ]]; then
  RUSTUP_BIN="$(command -v rustup || true)"
fi
[[ -n "$RUSTUP_BIN" ]] || fail "rustup is required for the pinned rust-toolchain.toml"
"$RUSTUP_BIN" show active-toolchain >/dev/null || fail "Unable to resolve the active rustup toolchain"

if [[ ! -x "$CARGO_BIN" ]]; then
  CARGO_BIN="$(command -v cargo || true)"
fi
[[ -n "$CARGO_BIN" ]] || fail "Unable to resolve cargo from $HOME/.cargo/bin or PATH"

cd "$ROOT_DIR"
"$CARGO_BIN" metadata --manifest-path "$ROOT_DIR/Cargo.toml" --no-deps --format-version 1 >/dev/null || fail "cargo metadata failed for Cargo.toml"
if [[ -f "$ROOT_DIR/tests/model/Cargo.toml" ]]; then
  "$CARGO_BIN" metadata --manifest-path "$ROOT_DIR/tests/model/Cargo.toml" --no-deps --format-version 1 >/dev/null || fail "cargo metadata failed for tests/model/Cargo.toml"
fi

printf '[rust-toolchain] pass: %s; minimum=%s\n' "$rustc_output" "$MINIMUM_VERSION"
