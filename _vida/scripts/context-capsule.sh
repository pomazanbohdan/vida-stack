#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/beads-runtime.sh"
LOG_SCRIPT="$SCRIPT_DIR/beads-log.sh"
CAPSULE_DIR="$ROOT_DIR/.vida/logs/context-capsules"

cd "$ROOT_DIR"

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/context-capsule.sh write <task_id> <done> <next> [risks] [acceptance_slice] [constraints] [task_role]
  bash _vida/scripts/context-capsule.sh read <task_id>
  bash _vida/scripts/context-capsule.sh hydrate <task_id>

Notes:
  - The script persists compact task+epic context for post-compact recovery.
  - hydrate exits non-zero when mandatory capsule fields are missing.
EOF
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[context-capsule] Missing required command: $1" >&2
    exit 1
  fi
}

require_cmd jq

mkdir -p "$CAPSULE_DIR"

now_utc() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

normalize_optional() {
  local value="${1:-}"
  if [[ -z "$value" || "$value" == "-" ]]; then
    echo ""
  else
    echo "$value"
  fi
}

write_capsule() {
  local task_id="$1"
  local done_text="$2"
  local next_step="$3"
  local risks="$4"
  local acceptance_slice="$5"
  local constraints="$6"
  local task_role="$7"

  local task_json
  task_json="$(beads_br show "$task_id" --json)"
  local task_title task_desc parent_epic
  task_title="$(jq -r 'if type=="array" then .[0].title else .title end // ""' <<<"$task_json")"
  task_desc="$(jq -r 'if type=="array" then .[0].description else .description end // ""' <<<"$task_json")"
  parent_epic="$(jq -r 'if type=="array" then .[0].parent else .parent end // ""' <<<"$task_json")"

  local epic_goal=""
  if [[ -n "$parent_epic" ]]; then
    local epic_json
    epic_json="$(beads_br show "$parent_epic" --json 2>/dev/null || true)"
    if [[ -n "$epic_json" ]]; then
      epic_goal="$(jq -r 'if type=="array" then ((.[0].description // .[0].title) // "") else ((.description // .title) // "") end' <<<"$epic_json")"
    fi
  fi

  if [[ -z "$epic_goal" ]]; then
    epic_goal="$task_title"
  fi

  local capsule_path="$CAPSULE_DIR/${task_id}.json"
  jq -cn \
    --arg ts "$(now_utc)" \
    --arg task_id "$task_id" \
    --arg task_title "$task_title" \
    --arg task_desc "$task_desc" \
    --arg parent_epic "$parent_epic" \
    --arg epic_goal "$epic_goal" \
    --arg done_text "$done_text" \
    --arg next_step "$next_step" \
    --arg risks "$risks" \
    --arg acceptance_slice "$acceptance_slice" \
    --arg constraints "$constraints" \
    --arg task_role "$task_role" \
    '{
      updated_at:$ts,
      trace_id:("ctx-" + ($ts | gsub("[-:TZ]";""))),
      epic_id:$parent_epic,
      epic_goal:$epic_goal,
      task_id:$task_id,
      task_title:$task_title,
      task_role_in_epic:$task_role,
      done:$done_text,
      next:$next_step,
      constraints:(if $constraints=="" then ["follow-L0-invariants","legacy-zero","br-ssot"] else [$constraints] end),
      open_risks:(if $risks=="" then [] else [$risks] end),
      acceptance_slice:$acceptance_slice,
      task_context:$task_desc
    }' > "$capsule_path"

  local meta
  meta="capsule_path=${capsule_path};next=${next_step};slice=${acceptance_slice}"
  bash "$LOG_SCRIPT" op-event "$task_id" "context_capsule_written" "$meta"
}

hydrate_capsule() {
  local task_id="$1"
  local capsule_path="$CAPSULE_DIR/${task_id}.json"
  if [[ ! -f "$capsule_path" ]]; then
    if [[ "${VIDA_CONTEXT_HYDRATE_ALLOW_MISSING:-0}" == "1" ]]; then
      bash "$LOG_SCRIPT" op-event "$task_id" "context_hydration_pending" "reason=missing_capsule"
      echo "[context-capsule] CONTEXT_HYDRATION_PENDING: missing capsule for $task_id" >&2
      return 3
    fi
    bash "$LOG_SCRIPT" op-event "$task_id" "context_hydration_failed" "reason=missing_capsule"
    echo "[context-capsule] BLK_CONTEXT_NOT_HYDRATED: missing capsule for $task_id" >&2
    return 2
  fi

  local missing
  missing="$(jq -r '[
      (if ((.epic_goal // "") == "") then "epic_goal" else empty end),
      (if ((.task_id // "") == "") then "task_id" else empty end),
      (if ((.next // "") == "") then "next" else empty end)
    ] | join(",")' "$capsule_path")"

  if [[ -n "$missing" ]]; then
    bash "$LOG_SCRIPT" op-event "$task_id" "context_hydration_failed" "reason=missing_fields;fields=${missing}"
    echo "[context-capsule] BLK_CONTEXT_NOT_HYDRATED: missing fields (${missing}) for $task_id" >&2
    return 2
  fi

  local meta
  meta="capsule_path=${capsule_path};next=$(jq -r '.next' "$capsule_path")"
  bash "$LOG_SCRIPT" op-event "$task_id" "context_hydrated" "$meta"
  cat "$capsule_path"
}

cmd="${1:-}"
case "$cmd" in
  write)
    task_id="${2:-}"
    done_text="${3:-}"
    next_step="${4:-}"
    risks="$(normalize_optional "${5:--}")"
    acceptance_slice="$(normalize_optional "${6:--}")"
    constraints="$(normalize_optional "${7:--}")"
    task_role="$(normalize_optional "${8:--}")"
    [[ -n "$task_id" && -n "$done_text" && -n "$next_step" ]] || { usage; exit 1; }
    write_capsule "$task_id" "$done_text" "$next_step" "$risks" "$acceptance_slice" "$constraints" "$task_role"
    ;;
  read)
    task_id="${2:-}"
    [[ -n "$task_id" ]] || { usage; exit 1; }
    cat "$CAPSULE_DIR/${task_id}.json"
    ;;
  hydrate)
    task_id="${2:-}"
    [[ -n "$task_id" ]] || { usage; exit 1; }
    hydrate_capsule "$task_id"
    ;;
  *)
    usage
    exit 1
    ;;
esac
