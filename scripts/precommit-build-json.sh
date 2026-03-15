#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_PATH="${BIN_PATH:-$ROOT_DIR/target/debug/vida}"

fail() {
  printf '[precommit-build-json] ERROR: %s\n' "$*" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "Missing required command: $1"
}

require_cmd cargo
mkdir -p "$(dirname "$BIN_PATH")"

printf '[precommit-build-json] Building vida\n'
cargo build -q -p vida

printf '[precommit-build-json] Building protocol-binding JSON artifacts\n'
VIDA_ROOT="$ROOT_DIR" "$BIN_PATH" taskflow protocol-binding sync --json >/dev/null

printf '[precommit-build-json] OK\n'
