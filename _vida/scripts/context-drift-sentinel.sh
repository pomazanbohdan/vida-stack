#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
CAPSULE_DIR="$ROOT_DIR/.vida/logs/context-capsules"
LOG_SCRIPT="$SCRIPT_DIR/beads-log.sh"

cd "$ROOT_DIR"

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/context-drift-sentinel.sh check <task_id> [--strict]

Checks:
  - capsule exists
  - capsule.next references known block_id or '-'
  - task in capsule equals requested task

Exit codes:
  0 no drift
  2 drift detected (only in --strict mode; otherwise still returns 0)
EOF
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[context-drift] Missing command: $1" >&2
    exit 1
  fi
}

require_cmd jq

cmd="${1:-}"
if [[ "$cmd" != "check" ]]; then
  usage
  exit 1
fi

task_id="${2:-}"
strict="${3:-}"
[[ -n "$task_id" ]] || { usage; exit 1; }

capsule="$CAPSULE_DIR/${task_id}.json"
if [[ ! -f "$capsule" ]]; then
  bash "$LOG_SCRIPT" op-event "$task_id" "context_drift_detected" "reason=missing_capsule"
  echo "drift=yes reason=missing_capsule task=$task_id"
  [[ "$strict" == "--strict" ]] && exit 2 || exit 0
fi

cap_task="$(jq -r '.task_id // ""' "$capsule")"
cap_next="$(jq -r '.next // ""' "$capsule")"

reasons=()
if [[ "$cap_task" != "$task_id" ]]; then
  reasons+=("task_mismatch")
fi

if [[ -n "$cap_next" && "$cap_next" != "-" ]]; then
  known_next="$(bash _vida/scripts/todo-tool.sh ui-json "$task_id" | jq -r --arg n "$cap_next" 'if ([.steps[]? | select(.block_id==$n)] | length) > 0 then "yes" else "no" end')"
  if [[ "$known_next" != "yes" ]]; then
    reasons+=("next_step_unknown")
  fi
fi

if [[ ${#reasons[@]} -gt 0 ]]; then
  reason_joined="$(IFS=','; echo "${reasons[*]}")"
  bash "$LOG_SCRIPT" op-event "$task_id" "context_drift_detected" "reason=${reason_joined};next=${cap_next}"
  echo "drift=yes reason=${reason_joined} task=$task_id"
  [[ "$strict" == "--strict" ]] && exit 2 || exit 0
fi

bash "$LOG_SCRIPT" op-event "$task_id" "context_drift_checked" "status=ok;next=${cap_next}"
echo "drift=no task=$task_id next=${cap_next}"

