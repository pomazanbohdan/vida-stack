#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
LOG_DIR="$ROOT_DIR/.vida/logs"
LOG_FILE="$LOG_DIR/beads-execution.jsonl"
TODO_INDEX_DIR="$LOG_DIR/todo-index"
VIDA_TODO_VERBOSE="${VIDA_TODO_VERBOSE:-0}"

cd "$ROOT_DIR"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[beads-log] Missing required command: $1" >&2
    exit 1
  fi
}

require_cmd jq

mkdir -p "$LOG_DIR"
mkdir -p "$TODO_INDEX_DIR"

now_utc() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

iso_to_epoch_ms() {
  local ts="$1"
  python3 - "$ts" <<'PY'
import datetime
import sys

raw = sys.argv[1]
if not raw:
    print(0)
    raise SystemExit
if raw.endswith("Z"):
    raw = raw[:-1] + "+00:00"
dt = datetime.datetime.fromisoformat(raw)
print(int(dt.timestamp() * 1000))
PY
}

compute_duration_ms() {
  local ts_start="$1"
  local ts_end="$2"
  local s e
  s="$(iso_to_epoch_ms "$ts_start")"
  e="$(iso_to_epoch_ms "$ts_end")"
  if [[ -z "$s" || -z "$e" ]]; then
    echo 0
    return
  fi
  if (( e < s )); then
    echo 0
  else
    echo $((e - s))
  fi
}

trace_id() {
  date -u +"trace-%Y%m%d%H%M%S"
}

log_info() {
  if [[ "$VIDA_TODO_VERBOSE" == "1" ]]; then
    echo "$*" >&2
  fi
}

append_log() {
  local payload="$1"
  printf '%s\n' "$payload" >> "$LOG_FILE"
}

ensure_todo_index_file() {
  local task_id="$1"
  local idx="$TODO_INDEX_DIR/${task_id}.json"
  if [[ ! -f "$idx" ]]; then
    jq -cn --arg task_id "$task_id" --arg updated_at "$(now_utc)" '{task_id:$task_id,updated_at:$updated_at,steps:[]}' > "$idx"
  fi
}

update_todo_index_block_start() {
  local task_id="$1"
  local block_id="$2"
  local goal="$3"
  local track_id="$4"
  local owner="$5"
  local depends_on="$6"
  local next_step="$7"
  local ts_start="$8"
  local idx="$TODO_INDEX_DIR/${task_id}.json"

  ensure_todo_index_file "$task_id"

  if [[ -z "$track_id" ]]; then
    track_id="main"
  fi
  if [[ -z "$owner" ]]; then
    owner="orchestrator"
  fi

  jq \
    --arg task_id "$task_id" \
    --arg block_id "$block_id" \
    --arg goal "$goal" \
    --arg track_id "$track_id" \
    --arg owner "$owner" \
    --arg depends_on "$depends_on" \
    --arg next_step "$next_step" \
    --arg ts_start "$ts_start" \
    --arg updated_at "$(now_utc)" \
    '
    .updated_at = $updated_at
    | .steps = (
        if ([.steps[] | select(.block_id==$block_id)] | length) > 0 then
          [.steps[] | if .block_id==$block_id then
            . + {
              task_id:$task_id,
              block_id:$block_id,
              goal:$goal,
              track_id:$track_id,
              owner:$owner,
              depends_on:$depends_on,
              next_step:(if $next_step != "" then $next_step else (.next_step // "") end),
              ts_start:$ts_start,
              ts_end:"",
              result:"",
              status:(if (.result // "") == "done" then "done"
                      elif (.result // "") == "failed" then "blocked"
                      elif (.result // "") == "redirected" then "superseded"
                      elif (.result // "") == "partial" then "partial"
                      elif ($ts_start != "" and ((.ts_end // "") == "")) then "doing"
                      else (.status // "todo") end)
            }
          else . end]
        else
          .steps + [{
            task_id:$task_id,
            block_id:$block_id,
            goal:$goal,
            track_id:$track_id,
            owner:$owner,
            depends_on:$depends_on,
            next_step:$next_step,
            ts_start:$ts_start,
            status:"doing"
          }]
        end
      )
    | .steps |= sort_by(.block_id)
    ' "$idx" > "$idx.tmp"
  mv "$idx.tmp" "$idx"
}

update_todo_index_block_plan() {
  local task_id="$1"
  local block_id="$2"
  local goal="$3"
  local track_id="$4"
  local owner="$5"
  local depends_on="$6"
  local next_step="$7"
  local idx="$TODO_INDEX_DIR/${task_id}.json"

  ensure_todo_index_file "$task_id"

  if [[ -z "$track_id" ]]; then
    track_id="main"
  fi
  if [[ -z "$owner" ]]; then
    owner="orchestrator"
  fi

  jq \
    --arg task_id "$task_id" \
    --arg block_id "$block_id" \
    --arg goal "$goal" \
    --arg track_id "$track_id" \
    --arg owner "$owner" \
    --arg depends_on "$depends_on" \
    --arg next_step "$next_step" \
    --arg updated_at "$(now_utc)" \
    '
    .updated_at = $updated_at
    | .steps = (
        if ([.steps[] | select(.block_id==$block_id)] | length) > 0 then
          [.steps[] | if .block_id==$block_id then
            . + {
              task_id:$task_id,
              block_id:$block_id,
              goal:$goal,
              track_id:$track_id,
              owner:$owner,
              depends_on:$depends_on,
              next_step:(if $next_step != "" then $next_step else (.next_step // "") end),
              status:(if (.status // "") == "done" then "done"
                      elif (.status // "") == "blocked" then "blocked"
                      elif (.status // "") == "doing" then "doing"
                      else "todo" end)
            }
          else . end]
        else
          .steps + [{
            task_id:$task_id,
            block_id:$block_id,
            goal:$goal,
            track_id:$track_id,
            owner:$owner,
            depends_on:$depends_on,
            next_step:$next_step,
            status:"todo"
          }]
        end
      )
    | .steps |= sort_by(.block_id)
    ' "$idx" > "$idx.tmp"
  mv "$idx.tmp" "$idx"
}

update_todo_index_block_end() {
  local task_id="$1"
  local block_id="$2"
  local result="$3"
  local next_step="$4"
  local actions="$5"
  local evidence_ref="$6"
  local merge_ready="$7"
  local ts_end="$8"
  local idx="$TODO_INDEX_DIR/${task_id}.json"

  ensure_todo_index_file "$task_id"

  jq \
    --arg task_id "$task_id" \
    --arg block_id "$block_id" \
    --arg result "$result" \
    --arg next_step "$next_step" \
    --arg actions "$actions" \
    --arg evidence_ref "$evidence_ref" \
    --arg merge_ready "$merge_ready" \
    --arg ts_end "$ts_end" \
    --arg updated_at "$(now_utc)" \
    '
    .updated_at = $updated_at
    | .steps = (
        if ([.steps[] | select(.block_id==$block_id)] | length) > 0 then
          [.steps[] | if .block_id==$block_id then
            . + {
              task_id:$task_id,
              block_id:$block_id,
              result:$result,
              next_step:$next_step,
              actions:$actions,
              evidence_ref:$evidence_ref,
              merge_ready:$merge_ready,
              ts_end:$ts_end,
              status:(if $result == "done" then "done"
                      elif $result == "failed" then "blocked"
                      elif $result == "redirected" then "superseded"
                      elif $result == "partial" then "partial"
                      elif ((.ts_start // "") != "" and $ts_end == "") then "doing"
                      else "todo" end)
            }
          else . end]
        else
          .steps + [{
            task_id:$task_id,
            block_id:$block_id,
            result:$result,
            next_step:$next_step,
            actions:$actions,
            evidence_ref:$evidence_ref,
            merge_ready:$merge_ready,
            ts_end:$ts_end,
            status:(if $result == "done" then "done"
                    elif $result == "failed" then "blocked"
                    elif $result == "redirected" then "superseded"
                    elif $result == "partial" then "partial"
                    else "todo" end)
          }]
        end
      )
    | .steps |= sort_by(.block_id)
    ' "$idx" > "$idx.tmp"
  mv "$idx.tmp" "$idx"
}

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/beads-log.sh block-plan <task_id> <block_id> <goal> [track_id] [owner] [depends_on] [next_step]
  bash _vida/scripts/beads-log.sh block-start <task_id> <block_id> <goal> [track_id] [owner] [depends_on] [next_step]
  bash _vida/scripts/beads-log.sh block-end <task_id> <block_id> <result> <next_step> <actions> [artifacts] [risks] [assumptions] [evidence_ref] [track_id] [owner] [merge_ready]
  bash _vida/scripts/beads-log.sh pack-start <task_id> <pack_id> <goal> [constraints]
  bash _vida/scripts/beads-log.sh pack-end <task_id> <pack_id> <result> <summary> [next_step]
  bash _vida/scripts/beads-log.sh self-reflection <task_id> <goal> <constraints> <evidence> <decision> <risks> <next_step> [confidence]
  bash _vida/scripts/beads-log.sh compact-pre <task_id> <done> <next> [risk]
  bash _vida/scripts/beads-log.sh compact-post <task_before> <task_after> <drift_detected:yes|no> [recovery_action]
  bash _vida/scripts/beads-log.sh op-event <task_id> <name> [meta]
  bash _vida/scripts/beads-log.sh telemetry-event <task_id> <block_id|-|> <agent_role> <action> <duration_ms> <result> [success]
  bash _vida/scripts/beads-log.sh show [task_id]

Notes:
  - Log file: .vida/logs/beads-execution.jsonl
  - Use '-' for empty optional values.
EOF
}

normalize_optional() {
  local value="$1"
  if [[ "$value" == "-" ]]; then
    echo ""
  else
    echo "$value"
  fi
}

cmd="${1:-}"

case "$cmd" in
  block-plan)
    task_id="${2:-}"
    block_id="${3:-}"
    goal="${4:-}"
    track_id="$(normalize_optional "${5:--}")"
    owner="$(normalize_optional "${6:--}")"
    depends_on="$(normalize_optional "${7:--}")"
    next_step="$(normalize_optional "${8:--}")"
    [[ -n "$task_id" && -n "$block_id" && -n "$goal" ]] || { usage; exit 1; }

    payload="$(jq -cn \
      --arg ts "$(now_utc)" \
      --arg type "block_plan" \
      --arg task_id "$task_id" \
      --arg block_id "$block_id" \
      --arg goal "$goal" \
      --arg track_id "$track_id" \
      --arg owner "$owner" \
      --arg depends_on "$depends_on" \
      --arg next_step "$next_step" \
      '{ts:$ts,type:$type,task_id:$task_id,block_id:$block_id,goal:$goal,track_id:$track_id,owner:$owner,depends_on:$depends_on,next_step:$next_step}')"
    append_log "$payload"
    update_todo_index_block_plan "$task_id" "$block_id" "$goal" "$track_id" "$owner" "$depends_on" "$next_step"
    log_info "[beads-log] block_plan recorded: $task_id/$block_id"
    ;;

  block-start)
    task_id="${2:-}"
    block_id="${3:-}"
    goal="${4:-}"
    track_id="$(normalize_optional "${5:--}")"
    owner="$(normalize_optional "${6:--}")"
    depends_on="$(normalize_optional "${7:--}")"
    next_step="$(normalize_optional "${8:--}")"
    [[ -n "$task_id" && -n "$block_id" && -n "$goal" ]] || { usage; exit 1; }

    payload="$(jq -cn \
      --arg ts "$(now_utc)" \
      --arg type "block_start" \
      --arg task_id "$task_id" \
      --arg block_id "$block_id" \
      --arg goal "$goal" \
      --arg track_id "$track_id" \
      --arg owner "$owner" \
      --arg depends_on "$depends_on" \
      --arg next_step "$next_step" \
      '{ts:$ts,type:$type,task_id:$task_id,block_id:$block_id,goal:$goal,track_id:$track_id,owner:$owner,depends_on:$depends_on,next_step:$next_step}')"
    append_log "$payload"
    update_todo_index_block_start "$task_id" "$block_id" "$goal" "$track_id" "$owner" "$depends_on" "$next_step" "$(now_utc)"
    log_info "[beads-log] block_start recorded: $task_id/$block_id"
    ;;

  pack-start)
    task_id="${2:-}"
    pack_id="${3:-}"
    goal="${4:-}"
    constraints="$(normalize_optional "${5:--}")"
    [[ -n "$task_id" && -n "$pack_id" && -n "$goal" ]] || { usage; exit 1; }

    payload="$(jq -cn \
      --arg ts "$(now_utc)" \
      --arg type "pack_start" \
      --arg task_id "$task_id" \
      --arg pack_id "$pack_id" \
      --arg goal "$goal" \
      --arg constraints "$constraints" \
      '{ts:$ts,type:$type,task_id:$task_id,pack_id:$pack_id,goal:$goal,constraints:$constraints}')"
    append_log "$payload"
    log_info "[beads-log] pack_start recorded: $task_id/$pack_id"
    ;;

  pack-end)
    task_id="${2:-}"
    pack_id="${3:-}"
    result="${4:-}"
    summary="${5:-}"
    next_step="$(normalize_optional "${6:--}")"
    [[ -n "$task_id" && -n "$pack_id" && -n "$result" && -n "$summary" ]] || { usage; exit 1; }

    payload="$(jq -cn \
      --arg ts "$(now_utc)" \
      --arg type "pack_end" \
      --arg task_id "$task_id" \
      --arg pack_id "$pack_id" \
      --arg result "$result" \
      --arg summary "$summary" \
      --arg next_step "$next_step" \
      '{ts:$ts,type:$type,task_id:$task_id,pack_id:$pack_id,result:$result,summary:$summary,next_step:$next_step}')"
    append_log "$payload"
    log_info "[beads-log] pack_end recorded: $task_id/$pack_id"
    ;;

  block-end)
    task_id="${2:-}"
    block_id="${3:-}"
    result="${4:-}"
    next_step="${5:-}"
    actions="${6:-}"
    artifacts="$(normalize_optional "${7:--}")"
    risks="$(normalize_optional "${8:--}")"
    assumptions="$(normalize_optional "${9:--}")"
    evidence_ref="$(normalize_optional "${10:--}")"
    track_id="$(normalize_optional "${11:--}")"
    owner="$(normalize_optional "${12:--}")"
    merge_ready="$(normalize_optional "${13:--}")"
    [[ -n "$task_id" && -n "$block_id" && -n "$result" && -n "$next_step" && -n "$actions" ]] || { usage; exit 1; }

    ts_start="$(jq -r \
      --arg task_id "$task_id" \
      --arg block_id "$block_id" \
      'select(.type=="block_start" and .task_id==$task_id and .block_id==$block_id) | .ts' \
      "$LOG_FILE" 2>/dev/null | tail -n1)"
    if [[ -z "$ts_start" ]]; then
      ts_start="$(now_utc)"
    fi

    ts_end_now="$(now_utc)"
    duration_ms="$(compute_duration_ms "$ts_start" "$ts_end_now")"

    payload="$(jq -cn \
      --arg ts "$(now_utc)" \
      --arg type "block_end" \
      --arg task_id "$task_id" \
      --arg block_id "$block_id" \
      --arg ts_start "$ts_start" \
      --arg ts_end "$ts_end_now" \
      --arg duration_ms "$duration_ms" \
      --arg result "$result" \
      --arg next_step "$next_step" \
      --arg actions "$actions" \
      --arg artifacts "$artifacts" \
      --arg risks "$risks" \
      --arg assumptions "$assumptions" \
      --arg evidence_ref "$evidence_ref" \
      --arg track_id "$track_id" \
      --arg owner "$owner" \
      --arg merge_ready "$merge_ready" \
      '{ts:$ts,type:$type,task_id:$task_id,block_id:$block_id,ts_start:$ts_start,ts_end:$ts_end,duration_ms:($duration_ms|tonumber),result:$result,next_step:$next_step,actions:$actions,artifacts:$artifacts,risks:$risks,assumptions:$assumptions,evidence_ref:$evidence_ref,track_id:$track_id,owner:$owner,merge_ready:$merge_ready}')"
    append_log "$payload"
    update_todo_index_block_end "$task_id" "$block_id" "$result" "$next_step" "$actions" "$evidence_ref" "$merge_ready" "$(now_utc)"
    log_info "[beads-log] block_end recorded: $task_id/$block_id"
    ;;

  self-reflection)
    task_id="${2:-}"
    goal="${3:-}"
    constraints="${4:-}"
    evidence="${5:-}"
    decision="${6:-}"
    risks="${7:-}"
    next_step="${8:-}"
    confidence="${9:-80}"
    [[ -n "$task_id" && -n "$goal" && -n "$constraints" && -n "$evidence" && -n "$decision" && -n "$risks" && -n "$next_step" ]] || { usage; exit 1; }

    payload="$(jq -cn \
      --arg ts "$(now_utc)" \
      --arg type "self_reflection" \
      --arg task_id "$task_id" \
      --arg goal "$goal" \
      --arg constraints "$constraints" \
      --arg evidence "$evidence" \
      --arg decision "$decision" \
      --arg risks "$risks" \
      --arg next_step "$next_step" \
      --arg confidence "$confidence" \
      '{ts:$ts,type:$type,task_id:$task_id,goal:$goal,constraints:$constraints,evidence:$evidence,decision:$decision,risks:$risks,next_step:$next_step,confidence:$confidence}')"
    append_log "$payload"
    log_info "[beads-log] self_reflection recorded: $task_id"
    ;;

  compact-pre)
    task_id="${2:-}"
    done_text="${3:-}"
    next_text="${4:-}"
    risk_text="$(normalize_optional "${5:--}")"
    [[ -n "$task_id" && -n "$done_text" && -n "$next_text" ]] || { usage; exit 1; }

    payload="$(jq -cn \
      --arg ts "$(now_utc)" \
      --arg type "compact_pre" \
      --arg task_id "$task_id" \
      --arg done "$done_text" \
      --arg next "$next_text" \
      --arg risk "$risk_text" \
      '{ts:$ts,type:$type,task_id:$task_id,checkpoint:{done:$done,next:$next,risk:$risk}}')"
    append_log "$payload"
    log_info "[beads-log] compact_pre recorded: $task_id"
    ;;

  compact-post)
    task_before="${2:-}"
    task_after="${3:-}"
    drift_detected="${4:-}"
    recovery_action="$(normalize_optional "${5:--}")"
    [[ -n "$task_before" && -n "$task_after" && -n "$drift_detected" ]] || { usage; exit 1; }

    payload="$(jq -cn \
      --arg ts "$(now_utc)" \
      --arg type "compact_post" \
      --arg task_before "$task_before" \
      --arg task_after "$task_after" \
      --arg drift_detected "$drift_detected" \
      --arg recovery_action "$recovery_action" \
      '{ts:$ts,type:$type,task_before:$task_before,task_after:$task_after,drift_detected:$drift_detected,recovery_action:$recovery_action}')"
    append_log "$payload"
    log_info "[beads-log] compact_post recorded: $task_before -> $task_after"
    ;;

  op-event)
    task_id="${2:-}"
    name="${3:-}"
    meta="$(normalize_optional "${4:--}")"
    [[ -n "$task_id" && -n "$name" ]] || { usage; exit 1; }

    payload="$(jq -cn \
      --arg ts "$(now_utc)" \
      --arg type "op_event" \
      --arg task_id "$task_id" \
      --arg name "$name" \
      --arg meta "$meta" \
      '{ts:$ts,type:$type,task_id:$task_id,name:$name,meta:$meta}')"
    append_log "$payload"
    log_info "[beads-log] op_event recorded: $task_id/$name"
    ;;

  telemetry-event)
    task_id="${2:-}"
    block_id="$(normalize_optional "${3:--}")"
    agent_role="${4:-orchestrator}"
    action="${5:-}"
    duration_ms="${6:-0}"
    result="${7:-unknown}"
    success="${8:-true}"
    [[ -n "$task_id" && -n "$action" ]] || { usage; exit 1; }

    payload="$(jq -cn \
      --arg ts "$(now_utc)" \
      --arg trace_id "$(trace_id)" \
      --arg type "telemetry_event" \
      --arg task_id "$task_id" \
      --arg block_id "$block_id" \
      --arg agent_role "$agent_role" \
      --arg action "$action" \
      --arg duration_ms "$duration_ms" \
      --arg result "$result" \
      --arg success "$success" \
      '{ts:$ts,trace_id:$trace_id,type:$type,task_id:$task_id,block_id:$block_id,agent_role:$agent_role,action:$action,duration_ms:($duration_ms|tonumber),result:$result,success:(if ($success|ascii_downcase)=="true" then true else false end)}')"
    append_log "$payload"
    log_info "[beads-log] telemetry_event recorded: $task_id/$action"
    ;;

  show)
    task_id="${2:-}"
    if [[ -z "$task_id" ]]; then
      tail -n 30 "$LOG_FILE" 2>/dev/null || true
    else
      jq -c --arg task_id "$task_id" 'select((.task_id // "") == $task_id or (.task_before // "") == $task_id or (.task_after // "") == $task_id)' "$LOG_FILE" 2>/dev/null || true
    fi
    ;;

  *)
    usage
    exit 1
    ;;
esac
