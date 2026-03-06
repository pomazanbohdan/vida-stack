#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/status-ui.sh"

LOCK_FILE="$ROOT_DIR/.vida/locks/stateful-workflow.lock"
QUIET=0

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/stateful-sequence-check.sh classify <command>
  bash _vida/scripts/stateful-sequence-check.sh assert <command> [--quiet]

Stateful commands:
  start checkpoint redirect block-plan block-start block-end block-finish
  pack-start pack-end reflect finish sync
EOF
}

is_stateful() {
  case "${1:-}" in
    start|checkpoint|redirect|block-plan|block-start|block-end|block-finish|pack-start|pack-end|reflect|finish|sync)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

cmd="${1:-}"
target="${2:-}"

if [[ "$cmd" == "assert" ]]; then
  shift 2 || true
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --quiet)
        QUIET=1
        shift
        ;;
      *)
        usage
        exit 1
        ;;
    esac
  done
fi

case "$cmd" in
  classify)
    [[ -n "$target" ]] || { usage; exit 1; }
    if is_stateful "$target"; then
      echo "stateful"
    else
      echo "readonly"
    fi
    ;;
  assert)
    [[ -n "$target" ]] || { usage; exit 1; }
    if ! is_stateful "$target"; then
      [[ "$QUIET" == "1" ]] || vida_status_line ok "[stateful-seq] readonly command='$target' ✨ No open issues"
      exit 0
    fi

    if ! command -v flock >/dev/null 2>&1; then
      vida_status_line fail "[stateful-seq] missing required command: flock"
      exit 1
    fi

    mkdir -p "$(dirname "$LOCK_FILE")"
    exec 8>"$LOCK_FILE"
    if flock -n 8; then
      [[ "$QUIET" == "1" ]] || vida_status_line ok "[stateful-seq] stateful command='$target' allowed (no active lock) ✨ No open issues"
      flock -u 8 || true
      exit 0
    fi

    vida_status_line blocked "[stateful-seq] stateful_conflict for '$target' (another stateful command is active)"
    exit 42
    ;;
  -h|--help|help)
    usage
    ;;
  *)
    usage
    exit 1
    ;;
esac
