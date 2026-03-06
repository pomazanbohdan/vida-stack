#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/beads-runtime.sh"
STATE_DIR="$ROOT_DIR/.vida/logs"
STATE_FILE="$STATE_DIR/bg-sync-state.json"
SESSION_NAME="${VIDA_BG_SYNC_SESSION:-vida-sync}"
INTERVAL_SEC_DEFAULT="${VIDA_BG_SYNC_INTERVAL_SEC:-600}"

cd "$ROOT_DIR"
mkdir -p "$STATE_DIR"

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/beads-bg-sync.sh start [--interval <sec>] [--session <name>]
  bash _vida/scripts/beads-bg-sync.sh stop [--session <name>]
  bash _vida/scripts/beads-bg-sync.sh status [--session <name>]
  bash _vida/scripts/beads-bg-sync.sh once

Policy:
  - Default interval: 600 sec (10 min).
  - Do not use aggressive intervals (<120 sec) in normal workflow.
EOF
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[beads-bg-sync] Missing required command: $1" >&2
    exit 1
  fi
}

require_cmd jq
require_cmd tmux

parse_args() {
  local session="$SESSION_NAME"
  local interval="$INTERVAL_SEC_DEFAULT"

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --interval)
        interval="${2:-}"
        shift 2
        ;;
      --session)
        session="${2:-}"
        shift 2
        ;;
      *)
        echo "[beads-bg-sync] Unknown argument: $1" >&2
        usage
        exit 1
        ;;
    esac
  done

  if ! [[ "$interval" =~ ^[0-9]+$ ]]; then
    echo "[beads-bg-sync] interval must be integer seconds" >&2
    exit 1
  fi

  if (( interval < 120 )); then
    echo "[beads-bg-sync] interval=$interval too aggressive for normal workflow; use >=120 sec" >&2
    exit 1
  fi

  echo "$session|$interval"
}

write_state() {
  local session="$1"
  local interval="$2"
  jq -cn \
    --arg session "$session" \
    --argjson interval_sec "$interval" \
    --arg role "backup_only" \
    --arg mode "$(beads_mode)" \
    --arg updated_at "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
    '{session:$session,interval_sec:$interval_sec,role:$role,mode:$mode,updated_at:$updated_at}' > "$STATE_FILE"
}

start_sync() {
  local session="$1"
  local interval="$2"

  if tmux has-session -t "$session" 2>/dev/null; then
    tmux kill-session -t "$session"
  fi

  local loop_script="$STATE_DIR/bg-sync-loop.sh"
  beads_snapshot_jsonl "bg-sync-start" >/dev/null 2>>"$STATE_DIR/bg-sync.err" || true
  cat > "$loop_script" <<EOF
#!/usr/bin/env bash
set -euo pipefail
cd "$ROOT_DIR"
source "$SCRIPT_DIR/beads-runtime.sh"
while true; do
  beads_snapshot_jsonl "bg-sync" >/dev/null 2>>"$STATE_DIR/bg-sync.err" || true
  sleep $interval
done
EOF
  chmod +x "$loop_script"

  tmux new-session -d -s "$session" "$loop_script"
  write_state "$session" "$interval"
  echo "started session=$session interval_sec=$interval"
}

stop_sync() {
  local session="$1"
  if tmux has-session -t "$session" 2>/dev/null; then
    tmux kill-session -t "$session"
    echo "stopped session=$session"
  else
    echo "not_running session=$session"
  fi
}

status_sync() {
  local session="$1"
  local running="no"
  if tmux has-session -t "$session" 2>/dev/null; then
    running="yes"
  fi

  local interval="unknown"
  local role="unknown"
  if [[ -f "$STATE_FILE" ]]; then
    interval="$(jq -r '.interval_sec // "unknown"' "$STATE_FILE" 2>/dev/null || echo "unknown")"
    role="$(jq -r '.role // "unknown"' "$STATE_FILE" 2>/dev/null || echo "unknown")"
  fi

  echo "session=$session running=$running interval_sec=$interval role=$role mode=$(beads_mode) snapshot_age_sec=$(beads_snapshot_age_seconds)"
}

cmd="${1:-}"
case "$cmd" in
  start)
    shift
    parsed="$(parse_args "$@")"
    session="${parsed%%|*}"
    interval="${parsed##*|}"
    start_sync "$session" "$interval"
    ;;
  stop)
    shift
    parsed="$(parse_args "$@")"
    session="${parsed%%|*}"
    stop_sync "$session"
    ;;
  status)
    shift
    parsed="$(parse_args "$@")"
    session="${parsed%%|*}"
    status_sync "$session"
    ;;
  once)
    beads_snapshot_jsonl "bg-sync-once" >/dev/null
    echo "snapshot_once"
    ;;
  *)
    usage
    exit 1
    ;;
esac
