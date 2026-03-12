#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NIMCACHE_DIR="${NIMCACHE_DIR:-$ROOT_DIR/.vida/scratchpad/nimcache/precommit}"
BIN_PATH="${BIN_PATH:-$ROOT_DIR/.vida/scratchpad/bin/taskflow-v0-precommit}"

fail() {
  printf '[precommit-build-json] ERROR: %s\n' "$*" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "Missing required command: $1"
}

require_cmd nim
mkdir -p "$(dirname "$BIN_PATH")" "$NIMCACHE_DIR"

printf '[precommit-build-json] Compiling taskflow-v0\n'
nim c -d:release --nimcache:"$NIMCACHE_DIR" -o:"$BIN_PATH" "$ROOT_DIR/taskflow-v0/src/vida.nim" >/dev/null

printf '[precommit-build-json] Building protocol-binding JSON artifacts\n'
VIDA_ROOT="$ROOT_DIR" "$BIN_PATH" protocol-binding build --json >/dev/null

printf '[precommit-build-json] OK\n'
