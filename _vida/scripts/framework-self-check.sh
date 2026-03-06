#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/status-ui.sh"

TASK_ID="${1:-}"
if [[ -z "$TASK_ID" ]]; then
  echo "Usage: bash _vida/scripts/framework-self-check.sh <task_id>" >&2
  exit 1
fi

cd "$ROOT_DIR"

echo "=== FSAP Snapshot: $TASK_ID ==="
bash _vida/scripts/beads-workflow.sh show "$TASK_ID"
echo
bash _vida/scripts/todo-tool.sh compact "$TASK_ID"
echo
bash _vida/scripts/quality-health-check.sh --mode quick "$TASK_ID"
echo
bash _vida/scripts/framework-boundary-check.sh --strict
echo
vida_status_line ok "[framework-self-check] ✨ No open issues"
