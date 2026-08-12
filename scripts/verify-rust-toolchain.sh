#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MINIMUM_VERSION="${VIDA_RUST_MINIMUM_VERSION:-1.97.1}"

fail() {
  printf '[rust-toolchain] ERROR: %s\n' "$*" >&2
  exit 1
}

if [[ -n "${VIDA_RUST_TOOLCHAIN_BIN:-}" ]]; then
  if [[ ! -f "$VIDA_RUST_TOOLCHAIN_BIN" ]]; then
    fail "VIDA_RUST_TOOLCHAIN_BIN does not point to a file: $VIDA_RUST_TOOLCHAIN_BIN"
  fi
  VIDA_REPO_ROOT="$ROOT_DIR" exec "$VIDA_RUST_TOOLCHAIN_BIN" \
    --minimum-version "$MINIMUM_VERSION" \
    --format text \
    --text-style bash \
    "$@"
fi

GO_BIN="${GO:-go}"
if [[ "$GO_BIN" == */* || "$GO_BIN" == *\\* ]]; then
  [[ -f "$GO_BIN" ]] || fail "go is required to run the Rust toolchain verifier"
else
  GO_BIN="$(command -v "$GO_BIN" || true)"
fi
[[ -n "$GO_BIN" ]] || fail "go is required to run the Rust toolchain verifier"

cd "$ROOT_DIR/tools/verify-rust-toolchain"
VIDA_REPO_ROOT="$ROOT_DIR" exec "$GO_BIN" run . \
  --minimum-version "$MINIMUM_VERSION" \
  --format text \
  --text-style bash \
  "$@"
