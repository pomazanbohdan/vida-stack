#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
LOG_FILE="$ROOT_DIR/.vida/logs/beads-execution.jsonl"

cd "$ROOT_DIR"

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/todo-overhead-report.sh <task_id>

Purpose:
  Lightweight self-diagnostic for TODO overhead from beads execution log.

Output:
  - Event counts by type
  - Approximate overhead index (weighted)
  - Optimization hints
EOF
}

task_id="${1:-}"
[[ -n "$task_id" ]] || { usage; exit 1; }

if [[ ! -f "$LOG_FILE" ]]; then
  echo "[todo-overhead-report] log file not found: $LOG_FILE" >&2
  exit 2
fi

events_json="$(jq -sc --arg task_id "$task_id" '[.[] | select((.task_id // "") == $task_id)]' "$LOG_FILE")"
total="$(jq -r 'length' <<<"$events_json")"

if [[ "$total" -eq 0 ]]; then
  echo "Task: $task_id"
  echo "Events: 0"
  echo "No TODO telemetry found."
  exit 0
fi

echo "Task: $task_id"
echo "Total events: $total"
echo
echo "Event counts:"
jq -r '
  group_by(.type)
  | map({type: (.[0].type // "unknown"), n: length})
  | sort_by(.n) | reverse
  | .[]
  | "- " + .type + ": " + (.n|tostring)
' <<<"$events_json"

# Weighted overhead heuristic (execution-observability overhead, not token-accurate)
score="$(jq -r '
  def cnt($t): map(select(.type==$t)) | length;
  (cnt("block_plan") * 1)
  + (cnt("block_start") * 1)
  + (cnt("block_end") * 2)
  + (cnt("self_reflection") * 2)
  + (cnt("pack_start") * 1)
  + (cnt("pack_end") * 1)
  + (cnt("compact_pre") * 1)
  + (cnt("compact_post") * 1)
' <<<"$events_json")"

per_block="$(jq -r '
  def cnt($t): map(select(.type==$t)) | length;
  (cnt("block_end")) as $ended
  | if $ended == 0 then 0 else ((
      (cnt("block_plan") * 1)
      + (cnt("block_start") * 1)
      + (cnt("block_end") * 2)
      + (cnt("self_reflection") * 2)
      + (cnt("pack_start") * 1)
      + (cnt("pack_end") * 1)
    ) / $ended) end
' <<<"$events_json")"

echo
echo "Overhead index (heuristic): $score"
echo "Overhead per completed block: $per_block"
echo
echo "Hints:"
echo "- Prefer rolling window planning (2-3 upcoming blocks)."
echo "- Use block-finish to collapse block-end + reflect + verify into one call."
echo "- Use todo-sync-plan --mode compact|delta instead of full snapshots by default."
echo "- Keep TODO_AUTO_SYNC_LEVEL=lean unless full sync is required."
