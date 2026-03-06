#!/usr/bin/env bash
set -euo pipefail

# Canonical runtime wrapper around `br`.
#
# Why: VIDA workflow scripts still mutate issue state through the JSONL mutator
# (`beads_mutate`). Until the runtime is migrated to a single direct-`br`
# mutation path, `_vida/*` wrappers must stay JSONL-first to avoid mixed
# JSONL/SQLite state and duplicate-import failures.

if ! command -v br >/dev/null 2>&1; then
  echo "br command not found" >&2
  exit 127
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MODE_FILE="$ROOT_DIR/.beads/runtime-mode.json"

args=("$@")

for arg in "${args[@]}"; do
  if [[ "$arg" == "--no-db" ]]; then
    exec br "${args[@]}"
  fi
done

mode="jsonl_safe"
if [[ -f "$MODE_FILE" ]] && command -v jq >/dev/null 2>&1; then
  mode="$(jq -r '.mode // "jsonl_safe"' "$MODE_FILE" 2>/dev/null || echo "jsonl_safe")"
fi

case "$mode" in
  jsonl_safe|direct|"")
    if [[ "$mode" == "direct" ]]; then
      echo "[br-safe] direct wrapper mode is disabled while beads_mutate owns writes; falling back to --no-db" >&2
    fi
    exec br --no-db "${args[@]}"
    ;;
  *)
    echo "[br-safe] Unknown runtime mode '$mode'; falling back to --no-db" >&2
    exec br --no-db "${args[@]}"
    ;;
esac
