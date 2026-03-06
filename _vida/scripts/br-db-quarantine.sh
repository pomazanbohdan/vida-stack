#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT_DIR/_vida/scripts/beads-runtime.sh"
BEADS_DIR="$ROOT_DIR/.beads"
STAMP="$(date +%Y%m%d-%H%M%S)"
OUT_DIR="$ROOT_DIR/.vida/scratchpad/br-db-quarantine-$STAMP"

mkdir -p "$OUT_DIR"
bash "$ROOT_DIR/_vida/scripts/beads-bg-sync.sh" stop >/dev/null 2>&1 || true
beads_set_mode "jsonl_safe" "db-quarantine"
beads_snapshot_jsonl "pre-quarantine" >/dev/null 2>&1 || true

if [[ -d "$BEADS_DIR" ]]; then
  shopt -s nullglob
  files=(
    "$BEADS_DIR"/*.db
    "$BEADS_DIR"/*.db-wal
    "$BEADS_DIR"/*.db-shm
  )
  shopt -u nullglob

  if (( ${#files[@]} > 0 )); then
    mv "${files[@]}" "$OUT_DIR"/
  fi
fi

echo "Quarantined DB artifacts to: $OUT_DIR"
echo "Mode forced to jsonl_safe; use bash _vida/scripts/br-safe.sh <command>"
