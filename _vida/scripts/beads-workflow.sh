#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/beads-runtime.sh"
source "$SCRIPT_DIR/status-ui.sh"
LOG_SCRIPT="$SCRIPT_DIR/beads-log.sh"
VERIFY_SCRIPT="$SCRIPT_DIR/beads-verify-log.sh"
TODO_SYNC_SCRIPT="$SCRIPT_DIR/todo-sync-plan.sh"
CONTEXT_CAPSULE_SCRIPT="$SCRIPT_DIR/context-capsule.sh"
CONTEXT_DRIFT_SCRIPT="$SCRIPT_DIR/context-drift-sentinel.sh"
BG_SYNC_SCRIPT="$SCRIPT_DIR/beads-bg-sync.sh"
STATEFUL_SEQUENCE_SCRIPT="$SCRIPT_DIR/stateful-sequence-check.sh"
EVAL_PACK_SCRIPT="$SCRIPT_DIR/eval-pack.sh"
SUBAGENT_EVAL_SCRIPT="$SCRIPT_DIR/subagent-eval-pack.py"
TODO_SYNC_STATE_DIR="$ROOT_DIR/.vida/logs/todo-sync-state"
TODO_SYNC_DEBOUNCE_SEC="${TODO_SYNC_DEBOUNCE_SEC:-2}"
TODO_AUTO_SYNC_LEVEL="${TODO_AUTO_SYNC_LEVEL:-lean}"
VIDA_TODO_VERBOSE="${VIDA_TODO_VERBOSE:-0}"
VIDA_BG_SYNC_AUTOSTART="${VIDA_BG_SYNC_AUTOSTART:-1}"
VIDA_BG_SYNC_AUTOSTART_INTERVAL="${VIDA_BG_SYNC_AUTOSTART_INTERVAL:-600}"
VIDA_STATEFUL_LOCK_TIMEOUT_SEC="${VIDA_STATEFUL_LOCK_TIMEOUT_SEC:-45}"
STATEFUL_LOCK_FILE="$ROOT_DIR/.vida/locks/stateful-workflow.lock"
STATEFUL_LOCK_HELD=0
AUTO_STARTED_BLOCK=""
AUTO_STARTED_GOAL=""
AUTO_STARTED_STATUS="none"

# Suppress noisy internal br info logs unless explicitly overridden.
export RUST_LOG="${RUST_LOG:-error}"

cd "$ROOT_DIR"

if [[ ! -d ".beads" ]]; then
  echo "[beads-workflow] .beads directory not found in repo root: $ROOT_DIR" >&2
  exit 1
fi

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[beads-workflow] Missing required command: $1" >&2
    exit 1
  fi
}

require_cmd jq

is_stateful_cmd() {
  local cmd="${1:-}"
  case "$cmd" in
    start|checkpoint|redirect|block-plan|block-start|block-end|block-finish|pack-start|pack-end|reflect|finish|sync)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

release_stateful_lock() {
  if [[ "$STATEFUL_LOCK_HELD" == "1" ]]; then
    flock -u 9 || true
    STATEFUL_LOCK_HELD=0
  fi
}

acquire_stateful_lock() {
  local cmd="${1:-}"
  local timeout="$VIDA_STATEFUL_LOCK_TIMEOUT_SEC"

  if ! command -v flock >/dev/null 2>&1; then
    vida_status_line fail "[beads-workflow] missing required command: flock (needed for stateful lock)"
    exit 1
  fi

  mkdir -p "$(dirname "$STATEFUL_LOCK_FILE")"
  exec 9>"$STATEFUL_LOCK_FILE"
  if ! flock -w "$timeout" 9; then
    vida_status_line blocked "[beads-workflow] stateful_conflict: command='$cmd' is blocked by another active stateful operation"
    vida_status_line info "[beads-workflow] wait for the in-flight stateful command to finish and retry"
    exit 42
  fi
  STATEFUL_LOCK_HELD=1
}

log_info() {
  if [[ "$VIDA_TODO_VERBOSE" == "1" ]]; then
    echo "$*" >&2
  fi
}

run_quiet_or_verbose() {
  if [[ "$VIDA_TODO_VERBOSE" == "1" ]]; then
    "$@"
  else
    "$@" >/dev/null 2>&1
  fi
}

normalize_optional() {
  local value="${1:-}"
  if [[ "$value" == "-" ]]; then
    echo ""
  else
    echo "$value"
  fi
}

todo_block_json() {
  local issue_id="$1"
  local block_id="$2"
  bash "$SCRIPT_DIR/todo-tool.sh" ui-json "$issue_id" \
    | jq -c --arg bid "$block_id" '.steps[]? | select(.block_id==$bid)' \
    | head -n1
}

ensure_context_capsule_bootstrap() {
  local issue_id="$1"
  local next_step="${2:-planning}"
  local acceptance_slice="${3:-runtime-bootstrap}"

  if bash "$CONTEXT_CAPSULE_SCRIPT" hydrate "$issue_id" >/dev/null 2>&1; then
    return 0
  fi

  bash "$CONTEXT_CAPSULE_SCRIPT" write \
    "$issue_id" \
    "bootstrap" \
    "${next_step:-planning}" \
    "-" \
    "$acceptance_slice" \
    "legacy-zero,br-ssot" \
    "runtime-bootstrap" >/dev/null 2>&1 || true
}

ensure_bg_sync_autostart() {
  [[ "$VIDA_BG_SYNC_AUTOSTART" == "1" ]] || return 0
  [[ -f "$BG_SYNC_SCRIPT" ]] || return 0
  command -v tmux >/dev/null 2>&1 || return 0

  local status_line running interval
  status_line="$(bash "$BG_SYNC_SCRIPT" status 2>/dev/null || true)"
  running="$(echo "$status_line" | sed -n 's/.*running=\([a-z]*\).*/\1/p' | head -n1)"
  if [[ "$running" == "yes" ]]; then
    return 0
  fi

  interval="$VIDA_BG_SYNC_AUTOSTART_INTERVAL"
  if ! [[ "$interval" =~ ^[0-9]+$ ]] || (( interval < 120 )); then
    interval=600
  fi

  bash "$BG_SYNC_SCRIPT" start --interval "$interval" >/dev/null 2>&1 || true
}

auto_start_next_block() {
  local issue_id="$1"
  local source_block_id="$2"
  local next_step="$3"
  local result="$4"

  AUTO_STARTED_BLOCK=""
  AUTO_STARTED_GOAL=""
  AUTO_STARTED_STATUS="none"

  [[ "$result" == "done" ]] || return 0
  [[ -n "$next_step" && "$next_step" != "-" ]] || return 0

  # Only auto-start when next_step points to an existing block_id for this task.
  local target
  target="$(bash "$SCRIPT_DIR/todo-tool.sh" ui-json "$issue_id" \
    | jq -r --arg ns "$next_step" '.steps[]? | select(.block_id==$ns) | .block_id' \
    | head -n 1)"

  [[ -n "$target" ]] || return 0

  local source_track target_goal target_status target_track
  source_track="$(bash "$SCRIPT_DIR/todo-tool.sh" ui-json "$issue_id" \
    | jq -r --arg bid "$source_block_id" '.steps[]? | select(.block_id==$bid) | .track_id // "main"' \
    | head -n 1)"
  target_goal="$(bash "$SCRIPT_DIR/todo-tool.sh" ui-json "$issue_id" \
    | jq -r --arg bid "$target" '.steps[]? | select(.block_id==$bid) | .goal // ""' \
    | head -n 1)"
  target_status="$(bash "$SCRIPT_DIR/todo-tool.sh" ui-json "$issue_id" \
    | jq -r --arg bid "$target" '.steps[]? | select(.block_id==$bid) | .status // ""' \
    | head -n 1)"
  target_track="$(bash "$SCRIPT_DIR/todo-tool.sh" ui-json "$issue_id" \
    | jq -r --arg bid "$target" '.steps[]? | select(.block_id==$bid) | .track_id // "main"' \
    | head -n 1)"

  if [[ "$target_status" == "todo" && "$source_track" == "$target_track" ]]; then
    bash "$LOG_SCRIPT" block-start "$issue_id" "$target" "${target_goal:--}" "$target_track" "orchestrator" "-" "-"
    AUTO_STARTED_BLOCK="$target"
    AUTO_STARTED_GOAL="${target_goal:--}"
    AUTO_STARTED_STATUS="started"
    log_info "[beads-workflow] auto-started next block: $issue_id/$target"
  elif [[ -n "$target" ]]; then
    AUTO_STARTED_BLOCK="$target"
    AUTO_STARTED_GOAL="${target_goal:--}"
    AUTO_STARTED_STATUS="planned"
  fi

  return 0
}

auto_sync_todo() {
  local issue_id="$1"
  local force_sync="${2:-no}"
  if [[ -z "$issue_id" || ! -f "$TODO_SYNC_SCRIPT" ]]; then
    return
  fi

  mkdir -p "$TODO_SYNC_STATE_DIR"
  local stamp_file="$TODO_SYNC_STATE_DIR/${issue_id}.stamp"
  local now_ts
  now_ts="$(date +%s)"

  if [[ "$force_sync" != "force" && -f "$stamp_file" ]]; then
    local last_ts
    last_ts="$(cat "$stamp_file" 2>/dev/null || echo 0)"
    if [[ "$last_ts" =~ ^[0-9]+$ ]]; then
      if (( now_ts - last_ts < TODO_SYNC_DEBOUNCE_SEC )); then
        return
      fi
    fi
  fi

  bash "$TODO_SYNC_SCRIPT" "$issue_id" --mode json-only --quiet >/dev/null 2>&1 || true
  printf '%s\n' "$now_ts" > "$stamp_file"
}

should_auto_sync_event() {
  local event="${1:-}"
  case "$TODO_AUTO_SYNC_LEVEL" in
    off)
      return 1
      ;;
    full)
      return 0
      ;;
    lean)
      case "$event" in
        start|block-start|block-end|finish|sync)
          return 0
          ;;
        *)
          return 1
          ;;
      esac
      ;;
    *)
      echo "[beads-workflow] Unknown TODO_AUTO_SYNC_LEVEL=$TODO_AUTO_SYNC_LEVEL (expected off|lean|full)" >&2
      return 1
      ;;
  esac
}

maybe_auto_sync_todo() {
  local issue_id="$1"
  local event="$2"
  local mode="${3:-normal}"

  if should_auto_sync_event "$event"; then
    if [[ "$mode" == "force" ]]; then
      auto_sync_todo "$issue_id" force
    else
      auto_sync_todo "$issue_id"
    fi
  fi
}

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/beads-workflow.sh ready
  bash _vida/scripts/beads-workflow.sh show <id>
  bash _vida/scripts/beads-workflow.sh start <id>
  bash _vida/scripts/beads-workflow.sh checkpoint <id> <done> <next> [risk]
  bash _vida/scripts/beads-workflow.sh redirect <id> <from_block_id> <to_block_id> <reason>
  bash _vida/scripts/beads-workflow.sh block-plan <id> <block_id> <goal> [track_id] [owner] [depends_on] [next_step]
  bash _vida/scripts/beads-workflow.sh block-start <id> <block_id> <goal> [track_id] [owner] [depends_on] [next_step]
  bash _vida/scripts/beads-workflow.sh block-end <id> <block_id> <done|partial|failed> <next_step> <actions> [artifacts] [risks] [assumptions] [evidence_ref] [track_id] [owner] [merge_ready]
  bash _vida/scripts/beads-workflow.sh block-finish <id> <block_id> <done|partial|failed> <next_step> <actions> [artifacts] [risks] [assumptions] [evidence_ref] [confidence] [track_id] [owner] [merge_ready]
  bash _vida/scripts/beads-workflow.sh pack-start <id> <pack_id> <goal> [constraints]
  bash _vida/scripts/beads-workflow.sh pack-end <id> <pack_id> <done|partial|failed> <summary> [next_step]
  bash _vida/scripts/beads-workflow.sh reflect <id> <goal> <constraints> <evidence> <decision> <risks> <next_step> [confidence]
  bash _vida/scripts/beads-workflow.sh verify <id>
  bash _vida/scripts/beads-workflow.sh finish <id> <reason>
  bash _vida/scripts/beads-workflow.sh sync
  bash _vida/scripts/beads-workflow.sh status

Examples:
  bash _vida/scripts/beads-workflow.sh start bd-18gm
  bash _vida/scripts/beads-workflow.sh checkpoint bd-18gm "api fixed" "write tests" "token edge case"
  bash _vida/scripts/beads-workflow.sh redirect bd-18gm B02 B03 "user changed focus to protocol split"
  bash _vida/scripts/beads-workflow.sh block-plan bd-18gm B01 "Audit AGENTS.md" - - - B02
  bash _vida/scripts/beads-workflow.sh block-start bd-18gm B01 "Audit AGENTS.md" - - - B02
  bash _vida/scripts/beads-workflow.sh block-end bd-18gm B01 done "Start B02" "Updated files" "AGENTS.md,_vida/docs/pipelines.md" - - "git diff"
  bash _vida/scripts/beads-workflow.sh block-finish bd-18gm B01 done B02 "Updated files" "AGENTS.md" "low" - "git diff" 90
  bash _vida/scripts/beads-workflow.sh pack-start bd-18gm research-pack "Audit current docs"
  bash _vida/scripts/beads-workflow.sh pack-end bd-18gm research-pack done "Research complete" "Start spec pack"
  bash _vida/scripts/beads-workflow.sh reflect bd-18gm "Goal" "Constraints" "Evidence" "Decision" "Risks" "Next step" "85"
  bash _vida/scripts/beads-workflow.sh verify bd-18gm
  bash _vida/scripts/beads-workflow.sh finish bd-18gm "All ACs met"
EOF
}

cmd="${1:-}"

if [[ "$cmd" != "" && "$cmd" != "-h" && "$cmd" != "--help" && "$cmd" != "help" ]]; then
  ensure_bg_sync_autostart
fi

if is_stateful_cmd "$cmd"; then
  if [[ -x "$STATEFUL_SEQUENCE_SCRIPT" ]]; then
    bash "$STATEFUL_SEQUENCE_SCRIPT" assert "$cmd" --quiet || exit 42
  fi
  acquire_stateful_lock "$cmd"
  trap release_stateful_lock EXIT
fi

case "$cmd" in
  -h|--help|help)
    usage
    exit 0
    ;;
  ready)
    beads_br ready
    ;;
  show)
    issue_id="${2:-}"
    [[ -n "$issue_id" ]] || { usage; exit 1; }
    beads_br show "$issue_id"
    ;;
  start)
    issue_id="${2:-}"
    [[ -n "$issue_id" ]] || { usage; exit 1; }
    run_quiet_or_verbose beads_mutate update "$issue_id" --status in_progress
    ensure_context_capsule_bootstrap "$issue_id" "planning" "task-start"
    log_info "[beads-workflow] issue $issue_id -> in_progress"
    if [[ "$VIDA_TODO_VERBOSE" == "1" ]]; then
      beads_br show "$issue_id"
    fi
    bash "$LOG_SCRIPT" telemetry-event "$issue_id" - orchestrator task_start 0 started true
    maybe_auto_sync_todo "$issue_id" start force
    ;;
  checkpoint)
    issue_id="${2:-}"
    done_text="${3:-}"
    next_text="${4:-}"
    risk_text="${5:-none}"
    [[ -n "$issue_id" && -n "$done_text" && -n "$next_text" ]] || { usage; exit 1; }
    note="compact-checkpoint: done=${done_text}; next=${next_text}; risk=${risk_text}"
    run_quiet_or_verbose beads_mutate update "$issue_id" --notes "$note"
    log_info "[beads-workflow] checkpoint saved for $issue_id"
    maybe_auto_sync_todo "$issue_id" checkpoint force
    ;;
  redirect)
    issue_id="${2:-}"
    from_block_id="${3:-}"
    to_block_id="${4:-}"
    reason="${5:-}"
    [[ -n "$issue_id" && -n "$from_block_id" && -n "$to_block_id" && -n "$reason" ]] || { usage; exit 1; }

    source_block_json="$(todo_block_json "$issue_id" "$from_block_id")"
    target_block_json="$(todo_block_json "$issue_id" "$to_block_id")"

    [[ -n "$source_block_json" ]] || { vida_status_line fail "[beads-workflow] redirect source block not found: $issue_id/$from_block_id" >&2; exit 1; }
    [[ -n "$target_block_json" ]] || { vida_status_line fail "[beads-workflow] redirect target block not found: $issue_id/$to_block_id" >&2; exit 1; }

    source_status="$(jq -r '.status // ""' <<<"$source_block_json")"
    if [[ "$source_status" != "doing" ]]; then
      vida_status_line fail "[beads-workflow] redirect requires active source block: $issue_id/$from_block_id status=$source_status" >&2
      exit 1
    fi

    source_goal="$(jq -r '.goal // ""' <<<"$source_block_json")"
    source_track="$(jq -r '.track_id // "main"' <<<"$source_block_json")"
    source_owner="$(jq -r '.owner // "orchestrator"' <<<"$source_block_json")"
    target_goal="$(jq -r '.goal // ""' <<<"$target_block_json")"
    target_track="$(jq -r '.track_id // "main"' <<<"$target_block_json")"
    target_owner="$(jq -r '.owner // "orchestrator"' <<<"$target_block_json")"
    target_depends_on="$(jq -r '.depends_on // "-"' <<<"$target_block_json")"
    target_next_step="$(jq -r '.next_step // "-"' <<<"$target_block_json")"

    bash "$LOG_SCRIPT" block-end \
      "$issue_id" \
      "$from_block_id" \
      "redirected" \
      "$to_block_id" \
      "redirected: $reason" \
      "-" \
      "redirect from ${from_block_id} to ${to_block_id}" \
      "-" \
      "source_goal=${source_goal}" \
      "$source_track" \
      "$source_owner" \
      "-"
    bash "$LOG_SCRIPT" telemetry-event "$issue_id" "$from_block_id" "$source_owner" block_redirect 0 partial true

    redirect_meta="$(jq -cn \
      --arg from "$from_block_id" \
      --arg to "$to_block_id" \
      --arg reason "$reason" \
      '{from:$from,to:$to,reason:$reason}')"
    bash "$LOG_SCRIPT" op-event \
      "$issue_id" \
      "block_redirect" \
      "$redirect_meta"

    bash "$LOG_SCRIPT" block-start \
      "$issue_id" \
      "$to_block_id" \
      "${target_goal:--}" \
      "$target_track" \
      "$target_owner" \
      "$target_depends_on" \
      "$target_next_step"
    bash "$LOG_SCRIPT" telemetry-event "$issue_id" "$to_block_id" "$target_owner" block_start 0 in_progress true

    maybe_auto_sync_todo "$issue_id" block-end force
    maybe_auto_sync_todo "$issue_id" block-start force

    vida_status_line progress "[beads-workflow] redirect task=$issue_id from=$from_block_id to=$to_block_id"
    vida_status_line info "[beads-workflow] reason=$reason"
    ;;
  block-plan)
    issue_id="${2:-}"
    block_id="${3:-}"
    goal="${4:-}"
    track_id="${5:-main}"
    owner="${6:-orchestrator}"
    depends_on="${7:--}"
    next_step="${8:--}"
    [[ -n "$issue_id" && -n "$block_id" && -n "$goal" ]] || { usage; exit 1; }
    ensure_context_capsule_bootstrap "$issue_id" "${next_step:--}" "block-plan:${block_id}"
    bash "$LOG_SCRIPT" block-plan "$issue_id" "$block_id" "$goal" "$track_id" "$owner" "$depends_on" "$next_step"
    maybe_auto_sync_todo "$issue_id" block-plan force
    ;;
  block-start)
    issue_id="${2:-}"
    block_id="${3:-}"
    goal="${4:-}"
    track_id="${5:-main}"
    owner="${6:-orchestrator}"
    depends_on="${7:--}"
    next_step="${8:--}"
    [[ -n "$issue_id" && -n "$block_id" && -n "$goal" ]] || { usage; exit 1; }
    bash "$LOG_SCRIPT" block-start "$issue_id" "$block_id" "$goal" "$track_id" "$owner" "$depends_on" "$next_step"
    bash "$LOG_SCRIPT" telemetry-event "$issue_id" "$block_id" "${owner:-orchestrator}" block_start 0 in_progress true
    maybe_auto_sync_todo "$issue_id" block-start force
    ;;
  block-end)
    issue_id="${2:-}"
    block_id="${3:-}"
    result="${4:-}"
    next_step="${5:-}"
    actions="${6:-}"
    artifacts="${7:--}"
    risks="${8:--}"
    assumptions="${9:--}"
    evidence_ref="${10:--}"
    track_id="${11:--}"
    owner="${12:--}"
    merge_ready="${13:--}"
    [[ -n "$issue_id" && -n "$block_id" && -n "$result" && -n "$next_step" && -n "$actions" ]] || { usage; exit 1; }
    bash "$LOG_SCRIPT" block-end "$issue_id" "$block_id" "$result" "$next_step" "$actions" "$artifacts" "$risks" "$assumptions" "$evidence_ref" "$track_id" "$owner" "$merge_ready"
    duration_ms="$(jq -r --arg t "$issue_id" --arg b "$block_id" 'select(.type=="block_end" and .task_id==$t and .block_id==$b) | .duration_ms // 0' .vida/logs/beads-execution.jsonl 2>/dev/null | tail -n1)"
    [[ -n "$duration_ms" ]] || duration_ms=0
    bash "$LOG_SCRIPT" telemetry-event "$issue_id" "$block_id" "${owner:-orchestrator}" block_end "$duration_ms" "$result" true
    auto_start_next_block "$issue_id" "$block_id" "$next_step" "$result"
    maybe_auto_sync_todo "$issue_id" block-end
    ;;
  block-finish)
    issue_id="${2:-}"
    block_id="${3:-}"
    result="${4:-}"
    next_step="${5:-}"
    actions="${6:-}"
    artifacts="${7:--}"
    risks="${8:--}"
    assumptions="${9:--}"
    evidence_ref="${10:--}"
    confidence="${11:-85}"
    track_id="${12:--}"
    owner="${13:--}"
    merge_ready="${14:--}"
    [[ -n "$issue_id" && -n "$block_id" && -n "$result" && -n "$next_step" && -n "$actions" ]] || { usage; exit 1; }

    bash "$LOG_SCRIPT" block-end "$issue_id" "$block_id" "$result" "$next_step" "$actions" "$artifacts" "$risks" "$assumptions" "$evidence_ref" "$track_id" "$owner" "$merge_ready"
    auto_start_next_block "$issue_id" "$block_id" "$next_step" "$result"

    reflect_goal="Block_${block_id}_completion"
    reflect_constraints="$(normalize_optional "${risks}")"
    if [[ -z "$reflect_constraints" ]]; then
      reflect_constraints="none"
    fi
    reflect_evidence="$(normalize_optional "${evidence_ref}")"
    if [[ -z "$reflect_evidence" ]]; then
      reflect_evidence="$actions"
    fi
    reflect_decision="result=${result}; actions=${actions}"
    reflect_risks="$(normalize_optional "${risks}")"
    if [[ -z "$reflect_risks" ]]; then
      reflect_risks="none"
    fi

    bash "$LOG_SCRIPT" self-reflection "$issue_id" "$reflect_goal" "$reflect_constraints" "$reflect_evidence" "$reflect_decision" "$reflect_risks" "$next_step" "$confidence"
    bash "$CONTEXT_CAPSULE_SCRIPT" write "$issue_id" "$actions" "$next_step" "$risks" "block:${block_id}" "track:${track_id:-main}" "execution-step"
    bash "$CONTEXT_DRIFT_SCRIPT" check "$issue_id" >/dev/null || true
    bash "$VERIFY_SCRIPT" --task "$issue_id"

    maybe_auto_sync_todo "$issue_id" block-end force

    if command -v bash >/dev/null 2>&1; then
      bash "$SCRIPT_DIR/todo-sync-plan.sh" "$issue_id" --mode compact --max-items 2 --quiet >/dev/null 2>&1 || true
    fi

    if [[ "$result" == "done" ]]; then
      vida_status_line ok "[beads-workflow] block-finish task=$issue_id block=$block_id next=$next_step"
      if [[ "$AUTO_STARTED_STATUS" == "started" ]]; then
        vida_status_line progress "[beads-workflow] active next block=$AUTO_STARTED_BLOCK goal=$AUTO_STARTED_GOAL"
      elif [[ "$AUTO_STARTED_STATUS" == "planned" ]]; then
        vida_status_line info "[beads-workflow] planned next block=$AUTO_STARTED_BLOCK goal=$AUTO_STARTED_GOAL"
      fi
    elif [[ "$result" == "partial" ]]; then
      vida_status_line warn "[beads-workflow] block-finish partial task=$issue_id block=$block_id next=$next_step"
    else
      vida_status_line fail "[beads-workflow] block-finish failed task=$issue_id block=$block_id next=$next_step"
    fi

    log_info "[beads-workflow] block-finish completed: $issue_id/$block_id"
    ;;
  pack-start)
    issue_id="${2:-}"
    pack_id="${3:-}"
    goal="${4:-}"
    constraints="${5:--}"
    [[ -n "$issue_id" && -n "$pack_id" && -n "$goal" ]] || { usage; exit 1; }
    ensure_context_capsule_bootstrap "$issue_id" "pack:${pack_id}" "pack-start:${pack_id}"
    bash "$LOG_SCRIPT" pack-start "$issue_id" "$pack_id" "$goal" "$constraints"
    maybe_auto_sync_todo "$issue_id" pack-start force
    ;;
  pack-end)
    issue_id="${2:-}"
    pack_id="${3:-}"
    result="${4:-}"
    summary="${5:-}"
    next_step="${6:--}"
    [[ -n "$issue_id" && -n "$pack_id" && -n "$result" && -n "$summary" ]] || { usage; exit 1; }
    bash "$LOG_SCRIPT" pack-end "$issue_id" "$pack_id" "$result" "$summary" "$next_step"
    maybe_auto_sync_todo "$issue_id" pack-end
    ;;
  verify)
    issue_id="${2:-}"
    [[ -n "$issue_id" ]] || { usage; exit 1; }
    bash "$VERIFY_SCRIPT" --task "$issue_id"
    ;;
  reflect)
    issue_id="${2:-}"
    goal="${3:-}"
    constraints="${4:-}"
    evidence="${5:-}"
    decision="${6:-}"
    risks="${7:-}"
    next_step="${8:-}"
    confidence="${9:-80}"
    [[ -n "$issue_id" && -n "$goal" && -n "$constraints" && -n "$evidence" && -n "$decision" && -n "$risks" && -n "$next_step" ]] || { usage; exit 1; }
    bash "$LOG_SCRIPT" self-reflection "$issue_id" "$goal" "$constraints" "$evidence" "$decision" "$risks" "$next_step" "$confidence"
    maybe_auto_sync_todo "$issue_id" reflect
    ;;
  finish)
    issue_id="${2:-}"
    reason="${3:-}"
    [[ -n "$issue_id" && -n "$reason" ]] || { usage; exit 1; }

    if ! bash "$VERIFY_SCRIPT" --task "$issue_id" --strict; then
      echo "[beads-workflow] Refusing to close $issue_id: log verification failed" >&2
      exit 2
    fi

    run_quiet_or_verbose beads_mutate close "$issue_id" --reason "$reason"
    run_quiet_or_verbose beads_br sync --flush-only
    run_quiet_or_verbose beads_br sync --status
    if [[ -f "$EVAL_PACK_SCRIPT" ]]; then
      run_quiet_or_verbose bash "$EVAL_PACK_SCRIPT" run "$issue_id"
    fi
    if [[ -f "$SUBAGENT_EVAL_SCRIPT" ]]; then
      run_quiet_or_verbose python3 "$SUBAGENT_EVAL_SCRIPT" run "$issue_id"
      bash "$LOG_SCRIPT" op-event "$issue_id" subagent_review_completed ".vida/logs/subagent-review-$issue_id.json"
    fi
    bash "$LOG_SCRIPT" telemetry-event "$issue_id" - orchestrator task_finish 0 closed true
    maybe_auto_sync_todo "$issue_id" finish
    log_info "[beads-workflow] issue $issue_id closed and synced"
    ;;
  sync)
    beads_br sync --flush-only
    beads_br sync --status
    ;;
  status)
    beads_br sync --status
    ;;
  *)
    usage
    exit 1
    ;;
esac
