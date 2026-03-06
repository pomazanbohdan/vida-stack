#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
LOG_SCRIPT="$SCRIPT_DIR/beads-log.sh"

cd "$ROOT_DIR"

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/wvp-evidence.sh record <task_id> <trigger> <agreement> <live_check> <decision_impact> [sources_csv]
  bash _vida/scripts/wvp-evidence.sh not-required <task_id> <reason>

Examples:
  bash _vida/scripts/wvp-evidence.sh record mobile-100 api agreed "curl ok" "validated auth contract" "https://docs.example/api,https://spec.example/auth"
  bash _vida/scripts/wvp-evidence.sh not-required mobile-100 "reflection task; no external facts changed"
EOF
}

cmd="${1:-}"
case "$cmd" in
  record)
    task_id="${2:-}"
    trigger="${3:-}"
    agreement="${4:-}"
    live_check="${5:-}"
    decision_impact="${6:-}"
    sources_csv="${7:-}"
    [[ -n "$task_id" && -n "$trigger" && -n "$agreement" && -n "$live_check" && -n "$decision_impact" ]] || { usage; exit 1; }

    meta="$(jq -cn \
      --arg trigger "$trigger" \
      --arg agreement "$agreement" \
      --arg live_check "$live_check" \
      --arg decision_impact "$decision_impact" \
      --arg sources_csv "$sources_csv" \
      '{
        trigger:$trigger,
        agreement:$agreement,
        live_check:$live_check,
        decision_impact:$decision_impact,
        sources:(if ($sources_csv | length) == 0 then [] else ($sources_csv | split(",") | map(gsub("^ +| +$"; "")) | map(select(length > 0))) end)
      }')"
    bash "$LOG_SCRIPT" op-event "$task_id" "wvp_evidence" "$meta"
    echo "WVP recorded for $task_id"
    ;;
  not-required)
    task_id="${2:-}"
    reason="${3:-}"
    [[ -n "$task_id" && -n "$reason" ]] || { usage; exit 1; }
    meta="$(jq -cn --arg reason "$reason" '{reason:$reason}')"
    bash "$LOG_SCRIPT" op-event "$task_id" "wvp_not_required" "$meta"
    echo "WVP not-required recorded for $task_id"
    ;;
  *)
    usage
    exit 1
    ;;
esac
