#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/beads-runtime.sh"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[task-execution-mode] Missing command: $1" >&2
    exit 1
  fi
}

require_cmd jq

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/task-execution-mode.sh get <task_id>
  bash _vida/scripts/task-execution-mode.sh recommend <task_id>
  bash _vida/scripts/task-execution-mode.sh set <task_id> <decision_required|autonomous> [reason]

Modes:
  decision_required  - assistant analyzes, user confirms key decisions before implementation.
  autonomous         - assistant executes implementation end-to-end with checkpoints.
EOF
}

task_json() {
  local task_id="$1"
  beads_br show "$task_id" --json | jq 'if type=="array" then .[0] else . end'
}

mode_from_labels() {
  local json="$1"
  jq -r '
    (.labels // []) as $labels
    | if ($labels | index("mode:autonomous")) != null then "autonomous"
      elif ($labels | index("mode:decision_required")) != null then "decision_required"
      else ""
      end
  ' <<<"$json"
}

recommend_mode() {
  local json="$1"
  jq -r '
    .issue_type as $t
    | (.labels // []) as $labels
    | if (($labels | index("docs")) != null) or (($labels | index("research")) != null) or $t=="docs" then
        "decision_required"
      else
        "autonomous"
      end
  ' <<<"$json"
}

cmd="${1:-}"
case "$cmd" in
  -h|--help|help)
    usage
    exit 0
    ;;
  get)
    task_id="${2:-}"
    [[ -n "$task_id" ]] || { usage; exit 1; }
    j="$(task_json "$task_id")"
    explicit="$(mode_from_labels "$j")"
    if [[ -n "$explicit" ]]; then
      echo "$explicit"
    else
      recommend_mode "$j"
    fi
    ;;
  recommend)
    task_id="${2:-}"
    [[ -n "$task_id" ]] || { usage; exit 1; }
    j="$(task_json "$task_id")"
    recommend_mode "$j"
    ;;
  set)
    task_id="${2:-}"
    mode="${3:-}"
    reason="${4:-}"
    [[ -n "$task_id" && -n "$mode" ]] || { usage; exit 1; }
    if [[ "$mode" != "decision_required" && "$mode" != "autonomous" ]]; then
      echo "[task-execution-mode] Invalid mode: $mode" >&2
      exit 1
    fi
    beads_mutate update "$task_id" --remove-label mode:autonomous --remove-label mode:decision_required >/dev/null
    beads_mutate update "$task_id" --add-label "mode:${mode}" >/dev/null
    if [[ -n "$reason" ]]; then
      beads_mutate update "$task_id" --notes "mode=${mode}; reason=${reason}" >/dev/null
    fi
    echo "mode=${mode} task=${task_id}"
    ;;
  *)
    usage
    exit 1
    ;;
esac
