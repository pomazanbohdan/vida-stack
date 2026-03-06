#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKFLOW_SCRIPT="$SCRIPT_DIR/beads-workflow.sh"
LOG_SCRIPT="$SCRIPT_DIR/beads-log.sh"
TODO_SYNC_SCRIPT="$SCRIPT_DIR/todo-sync-plan.sh"
CONTEXT_CAPSULE_SCRIPT="$SCRIPT_DIR/context-capsule.sh"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/beads-runtime.sh"
STATE_DIR="$ROOT_DIR/.vida/logs"
STATE_FILE="$STATE_DIR/beads-compact-state.json"

cd "$ROOT_DIR"

mkdir -p "$STATE_DIR"

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/beads-compact.sh pre <id> <done> <next> [risk]
  bash _vida/scripts/beads-compact.sh post [task_after]

Examples:
  bash _vida/scripts/beads-compact.sh pre bd-18gm "adapter fixed" "run tests" "auth edge case"
  bash _vida/scripts/beads-compact.sh post bd-18gm
EOF
}

mode="${1:-}"

case "$mode" in
  pre)
    issue_id="${2:-}"
    done_text="${3:-}"
    next_text="${4:-}"
    risk_text="${5:-none}"
    [[ -n "$issue_id" && -n "$done_text" && -n "$next_text" ]] || { usage; exit 1; }

    bash "$WORKFLOW_SCRIPT" show "$issue_id"
    bash "$CONTEXT_CAPSULE_SCRIPT" write "$issue_id" "$done_text" "$next_text" "$risk_text" "compact-checkpoint" "compact-recovery-required" "preserve-epic-goal"
    bash "$LOG_SCRIPT" compact-pre "$issue_id" "$done_text" "$next_text" "$risk_text"
    printf '{"task_before":"%s"}\n' "$issue_id" > "$STATE_FILE"
    bash "$WORKFLOW_SCRIPT" checkpoint "$issue_id" "$done_text" "$next_text" "$risk_text"
    bash "$WORKFLOW_SCRIPT" status
    bash "$WORKFLOW_SCRIPT" sync
    ;;
  post)
    issue_id="${2:-}"

    task_before="unknown"
    if [[ -f "$STATE_FILE" ]]; then
      task_before="$(jq -r '.task_before // "unknown"' "$STATE_FILE" 2>/dev/null || echo "unknown")"
    fi

    task_after="${issue_id:-unknown}"
    if [[ -z "$issue_id" && "$task_before" != "unknown" ]]; then
      task_after="$task_before"
    fi

    drift_detected="no"
    if [[ "$task_before" != "unknown" && "$task_after" != "unknown" && "$task_before" != "$task_after" ]]; then
      drift_detected="yes"
    fi

    recovery_action="-"
    bash "$WORKFLOW_SCRIPT" ready
    if [[ -n "$issue_id" ]]; then
      if ! bash "$CONTEXT_CAPSULE_SCRIPT" hydrate "$issue_id" >/dev/null; then
        echo "[beads-compact] BLK_CONTEXT_NOT_HYDRATED for $issue_id" >&2
        exit 2
      fi
      bash "$WORKFLOW_SCRIPT" show "$issue_id"
      current_status="$(beads_br show "$issue_id" --json | jq -r 'if type=="array" then .[0].status else .status end')"
      if [[ "$current_status" != "in_progress" ]]; then
        beads_mutate update "$issue_id" --status in_progress
        echo "[beads-compact] restored $issue_id -> in_progress"
        recovery_action="restored-in-progress"
      fi
    fi
    if [[ "$drift_detected" == "yes" && "$recovery_action" == "-" ]]; then
      recovery_action="task-switch-detected-manual-review"
    fi

    bash "$LOG_SCRIPT" compact-post "$task_before" "$task_after" "$drift_detected" "$recovery_action"
    if [[ "$task_after" != "unknown" ]]; then
      bash "$TODO_SYNC_SCRIPT" "$task_after" --mode json-only --quiet >/dev/null 2>&1 || true
    fi
    rm -f "$STATE_FILE"
    bash "$WORKFLOW_SCRIPT" status
    ;;
  *)
    usage
    exit 1
    ;;
esac
