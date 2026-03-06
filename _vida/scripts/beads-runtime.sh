#!/usr/bin/env bash
# shellcheck shell=bash

BEADS_RUNTIME_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BEADS_RUNTIME_ROOT="$(cd "$BEADS_RUNTIME_DIR/../.." && pwd)"
BEADS_RUNTIME_BR_SAFE_SCRIPT="$BEADS_RUNTIME_ROOT/_vida/scripts/br-safe.sh"
BEADS_RUNTIME_BEADS_DIR="$BEADS_RUNTIME_ROOT/.beads"
BEADS_RUNTIME_ISSUES_JSONL="$BEADS_RUNTIME_BEADS_DIR/issues.jsonl"
BEADS_RUNTIME_BACKUP_DIR="$BEADS_RUNTIME_BEADS_DIR/backups"
BEADS_RUNTIME_MODE_FILE="$BEADS_RUNTIME_BEADS_DIR/runtime-mode.json"
BEADS_RUNTIME_DEFAULT_MODE="jsonl_safe"
BEADS_RUNTIME_MUTATOR="$BEADS_RUNTIME_DIR/br-jsonl-mutate.py"

beads_require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[beads-runtime] Missing command: $1" >&2
    return 1
  fi
}

beads_now_utc() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

beads_ensure_mode_file() {
  mkdir -p "$BEADS_RUNTIME_BEADS_DIR"
  if [[ ! -f "$BEADS_RUNTIME_MODE_FILE" ]]; then
    beads_require_cmd jq >/dev/null
    jq -cn \
      --arg mode "$BEADS_RUNTIME_DEFAULT_MODE" \
      --arg updated_at "$(beads_now_utc)" \
      --arg reason "auto-init" \
      '{mode:$mode,updated_at:$updated_at,reason:$reason}' > "$BEADS_RUNTIME_MODE_FILE"
  fi
}

beads_mode() {
  beads_require_cmd jq >/dev/null
  beads_ensure_mode_file
  jq -r '.mode // "jsonl_safe"' "$BEADS_RUNTIME_MODE_FILE"
}

beads_set_mode() {
  local mode="$1"
  local reason="${2:-manual}"
  beads_require_cmd jq >/dev/null
  beads_ensure_mode_file
  case "$mode" in
    jsonl_safe|"")
      mode="jsonl_safe"
      ;;
    direct)
      mode="jsonl_safe"
      reason="direct-request-denied: ${reason}"
      ;;
    *)
      mode="jsonl_safe"
      reason="unknown-mode-${mode}: ${reason}"
      ;;
  esac
  mkdir -p "$(dirname "$BEADS_RUNTIME_MODE_FILE")"
  jq -cn \
    --arg mode "$mode" \
    --arg updated_at "$(beads_now_utc)" \
    --arg reason "$reason" \
    '{mode:$mode,updated_at:$updated_at,reason:$reason}' > "${BEADS_RUNTIME_MODE_FILE}.tmp"
  mv "${BEADS_RUNTIME_MODE_FILE}.tmp" "$BEADS_RUNTIME_MODE_FILE"
}

beads_br() {
  if [[ ! -x "$BEADS_RUNTIME_BR_SAFE_SCRIPT" ]]; then
    echo "[beads-runtime] Missing executable router: $BEADS_RUNTIME_BR_SAFE_SCRIPT" >&2
    return 127
  fi
  bash "$BEADS_RUNTIME_BR_SAFE_SCRIPT" "$@"
}

beads_mutate() {
  if [[ ! -f "$BEADS_RUNTIME_MUTATOR" ]]; then
    echo "[beads-runtime] Missing mutator: $BEADS_RUNTIME_MUTATOR" >&2
    return 127
  fi
  python3 "$BEADS_RUNTIME_MUTATOR" "$@"
}

beads_snapshot_jsonl() {
  local reason="${1:-manual}"
  if [[ ! -f "$BEADS_RUNTIME_ISSUES_JSONL" ]]; then
    echo "[beads-runtime] Missing JSONL source: $BEADS_RUNTIME_ISSUES_JSONL" >&2
    return 2
  fi

  mkdir -p "$BEADS_RUNTIME_BACKUP_DIR"

  local stamp dest tmp latest_tmp latest_dest
  stamp="$(date -u +"%Y%m%d-%H%M%S")"
  dest="$BEADS_RUNTIME_BACKUP_DIR/issues-${stamp}.jsonl"
  tmp="${dest}.tmp"
  latest_dest="$BEADS_RUNTIME_BACKUP_DIR/latest.jsonl"
  latest_tmp="${latest_dest}.tmp"

  cat "$BEADS_RUNTIME_ISSUES_JSONL" > "$tmp"
  mv "$tmp" "$dest"

  cat "$dest" > "$latest_tmp"
  mv "$latest_tmp" "$latest_dest"

  beads_require_cmd jq >/dev/null
  jq -cn \
    --arg mode "$(beads_mode)" \
    --arg reason "$reason" \
    --arg snapshot "$dest" \
    --arg updated_at "$(beads_now_utc)" \
    '{mode:$mode,reason:$reason,snapshot:$snapshot,updated_at:$updated_at}' \
    > "$BEADS_RUNTIME_BACKUP_DIR/latest.meta.json"

  printf '%s\n' "$dest"
}

beads_latest_snapshot_path() {
  printf '%s\n' "$BEADS_RUNTIME_BACKUP_DIR/latest.jsonl"
}

beads_snapshot_age_seconds() {
  local latest
  latest="$(beads_latest_snapshot_path)"
  if [[ ! -f "$latest" ]]; then
    echo -1
    return 0
  fi

  local now_ts file_ts
  now_ts="$(date +%s)"
  file_ts="$(stat -c %Y "$latest" 2>/dev/null || echo 0)"
  if ! [[ "$file_ts" =~ ^[0-9]+$ ]]; then
    echo -1
    return 0
  fi

  echo $((now_ts - file_ts))
}

beads_jsonl_stats() {
  if [[ ! -f "$BEADS_RUNTIME_ISSUES_JSONL" ]]; then
    echo '{"path":"","total":0,"unique":0,"duplicates":0}'
    return 0
  fi

  jq -sc --arg path "$BEADS_RUNTIME_ISSUES_JSONL" '
    def ids: map(.id // "");
    {
      path:$path,
      total:length,
      unique:(ids | unique | length),
      duplicates:(length - (ids | unique | length))
    }
  ' "$BEADS_RUNTIME_ISSUES_JSONL"
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  set -euo pipefail
  cmd="${1:-}"
  case "$cmd" in
    mode)
      beads_mode
      ;;
    set-mode)
      mode="${2:-}"
      reason="${3:-manual}"
      [[ -n "$mode" ]] || exit 1
      beads_set_mode "$mode" "$reason"
      ;;
    snapshot)
      beads_snapshot_jsonl "${2:-manual}"
      ;;
    snapshot-age)
      beads_snapshot_age_seconds
      ;;
    jsonl-stats)
      beads_jsonl_stats
      ;;
    *)
      echo "Usage: bash _vida/scripts/beads-runtime.sh <mode|set-mode|snapshot|snapshot-age|jsonl-stats>" >&2
      exit 1
      ;;
  esac
fi
