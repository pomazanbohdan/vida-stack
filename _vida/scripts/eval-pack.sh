#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/beads-runtime.sh"
LOG_FILE="$ROOT_DIR/.vida/logs/beads-execution.jsonl"
OUT_DIR="$ROOT_DIR/.vida/logs"

cd "$ROOT_DIR"

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/eval-pack.sh run <task_id>

Output:
  .vida/logs/eval-pack-<task_id>.json

Metrics:
  - task_completion
  - block_success_rate
  - avg_block_duration_ms
  - human_intervention_rate_proxy
  - drift_alert_count
  - compact_recovery_scenario
EOF
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[eval-pack] Missing command: $1" >&2
    exit 1
  fi
}

require_cmd jq

cmd="${1:-}"
if [[ "$cmd" != "run" ]]; then
  usage
  exit 1
fi

task_id="${2:-}"
[[ -n "$task_id" ]] || { usage; exit 1; }

mkdir -p "$OUT_DIR"
out="$OUT_DIR/eval-pack-${task_id}.json"

task_status="$(beads_br show "$task_id" --json | jq -r 'if type=="array" then .[0].status else .status end')"

block_total="$(jq -r --arg t "$task_id" 'select(.type=="block_end" and .task_id==$t) | 1' "$LOG_FILE" 2>/dev/null | wc -l | awk '{print $1}')"
block_done="$(jq -r --arg t "$task_id" 'select(.type=="block_end" and .task_id==$t and .result=="done") | 1' "$LOG_FILE" 2>/dev/null | wc -l | awk '{print $1}')"

if [[ "$block_total" -eq 0 ]]; then
  success_rate="0"
  avg_duration="0"
else
  success_rate="$(awk -v d="$block_done" -v t="$block_total" 'BEGIN { printf "%.2f", (d*100.0)/t }')"
  avg_duration="$(jq -r --arg t "$task_id" 'select(.type=="block_end" and .task_id==$t) | (.duration_ms // 0)' "$LOG_FILE" 2>/dev/null | awk '{sum+=$1; n+=1} END { if (n==0) print 0; else printf "%.0f", sum/n }')"
fi

human_intervention_rate_proxy="$(jq -r -s --arg t "$task_id" '
  ([.[] | select((.task_id // "") == $t and .type=="self_reflection")] | length) as $refl
  | ([.[] | select((.task_id // "") == $t and .type=="block_end")] | length) as $blocks
  | if $blocks==0 then 0 else (($refl * 100.0) / $blocks) end
' "$LOG_FILE" 2>/dev/null)"

drift_alert_count="$(jq -r --arg t "$task_id" 'select((.task_id // "") == $t and .type=="op_event" and .name=="context_drift_detected") | 1' "$LOG_FILE" 2>/dev/null | wc -l | awk '{print $1}')"

compact_pre_count="$(jq -r --arg t "$task_id" 'select((.task_id // "") == $t and .type=="compact_pre") | 1' "$LOG_FILE" 2>/dev/null | wc -l | awk '{print $1}')"
compact_post_count="$(jq -r --arg t "$task_id" 'select((.task_before // "") == $t or (.task_after // "") == $t) | select(.type=="compact_post") | 1' "$LOG_FILE" 2>/dev/null | wc -l | awk '{print $1}')"
hydrated_count="$(jq -r --arg t "$task_id" 'select((.task_id // "") == $t and .type=="op_event" and .name=="context_hydrated") | 1' "$LOG_FILE" 2>/dev/null | wc -l | awk '{print $1}')"

compact_recovery="not_applicable"
if [[ "$compact_pre_count" -gt 0 || "$compact_post_count" -gt 0 ]]; then
  if [[ "$compact_pre_count" -gt 0 && "$compact_post_count" -gt 0 && "$hydrated_count" -gt 0 ]]; then
    compact_recovery="pass"
  else
    compact_recovery="partial"
  fi
fi

jq -cn \
  --arg task_id "$task_id" \
  --arg task_status "$task_status" \
  --argjson block_total "$block_total" \
  --argjson block_done "$block_done" \
  --argjson block_success_rate "$success_rate" \
  --argjson avg_block_duration_ms "$avg_duration" \
  --argjson human_intervention_rate_proxy "$human_intervention_rate_proxy" \
  --argjson drift_alert_count "$drift_alert_count" \
  --arg compact_recovery_scenario "$compact_recovery" \
  --arg generated_at "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
  '{
    generated_at:$generated_at,
    task_id:$task_id,
    task_completion:(if $task_status=="closed" then "closed" else "open_or_in_progress" end),
    task_status:$task_status,
    block_total:$block_total,
    block_done:$block_done,
    block_success_rate:$block_success_rate,
    avg_block_duration_ms:$avg_block_duration_ms,
    human_intervention_rate_proxy:$human_intervention_rate_proxy,
    drift_alert_count:$drift_alert_count,
    compact_recovery_scenario:$compact_recovery_scenario
  }' > "$out"

echo "$out"
