#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
VIDA_LEGACY_BIN="$ROOT_DIR/_vida/scripts-nim/vida-legacy"
TURSO_PYTHON="$ROOT_DIR/.venv/bin/python3"

cd "$ROOT_DIR"

vida_icon() {
  case "${1:-info}" in
    ok) printf '✅' ;;
    warn) printf '⚠️' ;;
    fail) printf '❌' ;;
    blocked) printf '⛔' ;;
    info) printf 'ℹ️' ;;
    sparkle) printf '✨' ;;
    progress) printf '🔄' ;;
    *) printf '•' ;;
  esac
}

vida_status_line() {
  local level="${1:-info}"
  shift || true
  printf '%s %s\n' "$(vida_icon "$level")" "$*"
}

jsonl_stats() {
  local issues_jsonl="$ROOT_DIR/.beads/issues.jsonl"
  if [[ ! -f "$issues_jsonl" ]]; then
    echo '{"path":"","total":0,"unique":0,"duplicates":0}'
    return 0
  fi
  jq -sc --arg path "$issues_jsonl" '
    def ids: map(.id // "");
    {
      path:$path,
      total:length,
      unique:(ids | unique | length),
      duplicates:(length - (ids | unique | length))
    }
  ' "$issues_jsonl"
}

validate_required_skills() {
  local -a required_skills=("plan" "skill-creator" "skill-installer")
  local -a search_roots=(
    "$ROOT_DIR/.agents/skills"
    "$HOME/.agents/skills"
    "$HOME/.code/skills"
  )
  local missing=0
  local skill root found_path
  declare -A discovered_skills=()

  for root in "${search_roots[@]}"; do
    [[ -d "$root" ]] || continue
    while IFS= read -r candidate; do
      skill_name="$(basename "$(dirname "$candidate")")"
      [[ -n "$skill_name" ]] || continue
      if [[ -z "${discovered_skills[$skill_name]:-}" ]]; then
        discovered_skills["$skill_name"]="$candidate"
      fi
    done < <(find "$root" -type f -name SKILL.md 2>/dev/null)
  done

  for skill in "${required_skills[@]}"; do
    found_path="${discovered_skills[$skill]:-}"
    if [[ -z "$found_path" ]]; then
      echo "[MISSING] $skill" >&2
      missing=$((missing + 1))
    fi
  done
  return "$missing"
}

TASK_ID=""
MODE="full"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode)
      MODE="${2:-full}"
      shift 2
      ;;
    -h|--help)
      echo "Usage: bash _vida/scripts/quality-health-check.sh [--mode quick|full|strict-dev] [task_id]" >&2
      exit 0
      ;;
    *)
      if [[ -z "$TASK_ID" ]]; then
        TASK_ID="$1"
        shift
      else
        vida_status_line fail "[health] Unexpected argument: $1" >&2
        exit 1
      fi
      ;;
  esac
done

if [[ "$MODE" != "quick" && "$MODE" != "full" && "$MODE" != "strict-dev" ]]; then
  vida_status_line fail "[health] Invalid mode: $MODE (expected quick|full|strict-dev)" >&2
  exit 1
fi

vida_status_line sparkle "[health] repo: $ROOT_DIR"

required_files=(
  "AGENTS.md"
  "_vida/docs/pipelines.md"
  "_vida/docs/protocol-index.md"
  "_vida/docs/todo-protocol.md"
  "_vida/docs/use-case-packs.md"
  "_vida/docs/bug-fix-protocol.md"
  "_vida/docs/web-validation-protocol.md"
  "_vida/docs/spec-contract-protocol.md"
  "_vida/docs/spec-contract-artifacts.md"
  "_vida/docs/form-task-protocol.md"
  "_vida/docs/implement-execution-protocol.md"
  "_vida/docs/log-policy.md"
  "_vida/docs/self-reflection-protocol.md"
  "_vida/docs/beads-protocol.md"
  "_vida/docs/project-overlay-protocol.md"
  "_vida/docs/project-bootstrap-protocol.md"
  "_vida/docs/human-approval-protocol.md"
  "_vida/docs/framework-memory-protocol.md"
  "_vida/docs/document-lifecycle-protocol.md"
  "_vida/docs/context-governance-protocol.md"
  "_vida/docs/run-graph-protocol.md"
  "_vida/docs/trace-eval-protocol.md"
  "_vida/docs/capability-registry-protocol.md"
  "_vida/docs/subagent-system-protocol.md"
  "_vida/scripts/beads-workflow.sh"
  "_vida/scripts/boot-profile.sh"
  "_vida/scripts/eval-pack.py"
  "_vida/scripts/beads-log.sh"
  "_vida/scripts-nim/src/state/beads.nim"
  "_vida/scripts/todo-runtime.py"
  "_vida/scripts/framework-boundary-check.sh"
  "_vida/scripts/project-bootstrap.py"
  "_vida/scripts/skill-discovery.py"
  "_vida/scripts/scp-confidence.py"
  "_vida/scripts/vida-config.py"
  "_vida/templates/vida.config.yaml.template"
  "_vida/scripts/subagent-system.py"
  "_vida/scripts/subagent-dispatch.py"
  "_vida/scripts/subagent-eval-pack.py"
  "_vida/scripts/human-approval-gate.py"
  "_vida/scripts/framework-memory.py"
  "_vida/scripts/doc-lifecycle.py"
  "_vida/scripts/context-governance.py"
  "_vida/scripts/framework-operator-status.py"
  "_vida/scripts/run-graph.py"
  "_vida/scripts/trace-eval.py"
  "_vida/scripts/capability-registry.py"
  "_vida/scripts/task-state-reconcile.py"
)

vida_status_line info "[health] checking required files"
for f in "${required_files[@]}"; do
  if [[ ! -f "$f" ]]; then
    echo "[health] MISSING: $f" >&2
    exit 2
  fi
done

vida_status_line info "[health] skills"
if ! validate_required_skills; then
  vida_status_line fail "[health] FAIL: required skills missing" >&2
  exit 2
fi
bash _vida/scripts/framework-boundary-check.sh --strict >/dev/null

jsonl_stats="$(jsonl_stats)"
jsonl_duplicates="$(jq -r '.duplicates' <<<"$jsonl_stats")"

if [[ "$jsonl_duplicates" != "0" ]]; then
  vida_status_line fail "[health] FAIL: duplicate task ids detected in legacy JSONL snapshot ($jsonl_duplicates)" >&2
  exit 5
fi

runtime_scripts=(
  "_vida/scripts/beads-workflow.sh"
  "_vida/scripts/quality-health-check.sh"
  "_vida/scripts-nim/src/vida"
  "_vida/scripts-nim/src/state/context_capsule.nim"
  "_vida/scripts/eval-pack.py"
)

raw_br_hits="$(rg -n --pcre2 '^(?!\\s*#).*?\\bbr (?=(ready|show|update|close|sync|list|create|dep|init)\\b)' "${runtime_scripts[@]}" 2>/dev/null || true)"
if [[ -n "$raw_br_hits" ]]; then
  vida_status_line fail "[health] FAIL: raw br runtime calls remain" >&2
  echo "$raw_br_hits" >&2
  exit 6
fi

vida_status_line info "[health] beads status"
if ! bash _vida/scripts/beads-workflow.sh status >/dev/null; then
  vida_status_line fail "[health] FAIL: beads-workflow status failed" >&2
  exit 7
fi

if command -v gh >/dev/null 2>&1; then
  vida_status_line ok "[health] gh CLI available"
else
  vida_status_line warn "[health] WARN: gh CLI not found in PATH"
fi

if [[ -f "vida.config.yaml" ]]; then
  vida_status_line info "[health] overlay schema validation"
  if ! python3 _vida/scripts/vida-config.py validate >/dev/null; then
    vida_status_line fail "[health] FAIL: vida.config.yaml schema validation failed" >&2
    exit 2
  fi

  if python3 _vida/scripts/subagent-system.py diagnose >/tmp/vida-subagent-health.json 2>/dev/null; then
    degraded_count="$(jq -r '.summary.degraded // 0' /tmp/vida-subagent-health.json 2>/dev/null)"
    cooldown_count="$(jq -r '.summary.cooldown // 0' /tmp/vida-subagent-health.json 2>/dev/null)"
    probe_count="$(jq -r '.summary.probe_required // 0' /tmp/vida-subagent-health.json 2>/dev/null)"
    degraded_names="$(jq -r '[.alerts[] | select(.kind == "availability_or_lifecycle" and .lifecycle_stage == "degraded") | .subagent] | unique | join(", ")' /tmp/vida-subagent-health.json 2>/dev/null)"
    cooldown_names="$(jq -r '[.alerts[] | select(.kind == "availability_or_lifecycle" and .lifecycle_stage == "cooldown") | .subagent] | unique | join(", ")' /tmp/vida-subagent-health.json 2>/dev/null)"
    probe_names="$(jq -r '[.alerts[] | select(.recommended_action == "run_probe" or .recommended_action == "repair_auth_then_probe" or .recommended_action == "fix_headless_profile_then_probe") | .subagent] | unique | join(", ")' /tmp/vida-subagent-health.json 2>/dev/null)"
    lease_conflicts="$(jq -r '.leases.recent_conflicts // 0' /tmp/vida-subagent-health.json 2>/dev/null)"
    unstable_names="$(jq -r '[.unstable_by_timeout_class[] | .subagent] | unique | join(", ")' /tmp/vida-subagent-health.json 2>/dev/null)"
    if [[ "$degraded_count" -gt 0 || "$cooldown_count" -gt 0 || "$probe_count" -gt 0 ]]; then
      vida_status_line warn "[health] cli subagent status: degraded=$degraded_count [${degraded_names:-none}] cooldown=$cooldown_count [${cooldown_names:-none}] probe_required=$probe_count [${probe_names:-none}]"
    else
      vida_status_line ok "[health] cli subagent status healthy"
    fi
    if [[ "${lease_conflicts:-0}" -gt 0 || -n "${unstable_names:-}" ]]; then
      vida_status_line info "[health] cli subagent diagnosis: recent_lease_conflicts=${lease_conflicts:-0} unstable_timeout_classes=[${unstable_names:-none}]"
    fi
  fi
fi

resolve_task_labels() {
  local issue_id="$1"
  local labels
  labels="$(
    VIDA_ROOT="$ROOT_DIR" \
    VIDA_LEGACY_TURSO_PYTHON="${VIDA_LEGACY_TURSO_PYTHON:-$TURSO_PYTHON}" \
      "$VIDA_LEGACY_BIN" task show "$issue_id" --json 2>/dev/null \
      | jq -r '.labels // [] | join(" ")' 2>/dev/null || true
  )"
  if [[ -n "${labels// }" ]]; then
    printf '%s\n' "$labels"
    return 0
  fi
  return 0
}

task_has_open_pack() {
  local issue_id="$1"
  local pack_id="$2"
  [[ -f ".vida/logs/beads-execution.jsonl" ]] || return 1

  local balance
  balance="$(
    jq -sr --arg task "$issue_id" --arg pack "$pack_id" '
      reduce .[] as $event (0;
        if (($event.task_id // "") == $task and ($event.pack_id // "") == $pack) then
          if ($event.type // "") == "pack_start" then . + 1
          elif ($event.type // "") == "pack_end" then . - 1
          else .
          end
        else .
        end
      )
    ' .vida/logs/beads-execution.jsonl 2>/dev/null || echo 0
  )"
  [[ "${balance:-0}" =~ ^-?[0-9]+$ ]] || balance=0
  (( balance > 0 ))
}

if [[ -n "$TASK_ID" ]]; then
  vida_status_line info "[health] boot receipt verification for task: $TASK_ID"
  if ! bash _vida/scripts/boot-profile.sh verify-receipt "$TASK_ID" >/dev/null; then
    vida_status_line fail "[health] FAIL: missing or invalid boot receipt/packet for task $TASK_ID" >&2
    exit 4
  fi

  if [[ "$MODE" == "full" || "$MODE" == "strict-dev" ]]; then
    vida_status_line info "[health] strict log verify for task: $TASK_ID"
    VIDA_ROOT="${VIDA_ROOT:-$ROOT_DIR}" VIDA_LEGACY_TURSO_PYTHON="${VIDA_LEGACY_TURSO_PYTHON:-$ROOT_DIR/.venv/bin/python3}" _vida/scripts-nim/src/vida beads verify --task "$TASK_ID" --strict

    vida_status_line info "[health] todo sync snapshot for task: $TASK_ID"
    python3 _vida/scripts/todo-runtime.py sync "$TASK_ID" --mode json-only --quiet >/dev/null

    vida_status_line info "[health] todo board render for task: $TASK_ID"
    python3 _vida/scripts/todo-runtime.py board "$TASK_ID" >/dev/null

    vida_status_line info "[health] todo plan integrity for task: $TASK_ID"
    if ! python3 _vida/scripts/todo-runtime.py validate "$TASK_ID" --quiet >/dev/null 2>&1; then
      vida_status_line fail "[health] FAIL: TODO plan integrity check failed for task $TASK_ID" >&2
      exit 4
    fi

    vida_status_line info "[health] pack coverage for task: $TASK_ID"
    pack_start_count="$(jq -r --arg task "$TASK_ID" 'select((.task_id // "") == $task and .type == "pack_start") | 1' .vida/logs/beads-execution.jsonl 2>/dev/null | wc -l | awk '{print $1}')"
    pack_end_count="$(jq -r --arg task "$TASK_ID" 'select((.task_id // "") == $task and .type == "pack_end") | 1' .vida/logs/beads-execution.jsonl 2>/dev/null | wc -l | awk '{print $1}')"
    block_count="$(jq -r --arg task "$TASK_ID" 'select((.task_id // "") == $task and .type == "block_end") | 1' .vida/logs/beads-execution.jsonl 2>/dev/null | wc -l | awk '{print $1}')"

    if [[ "$block_count" -gt 0 && "$pack_start_count" -eq 0 ]]; then
      vida_status_line fail "[health] FAIL: block execution exists but pack_start is missing for task $TASK_ID" >&2
      exit 3
    fi

    if [[ "$pack_start_count" -ne "$pack_end_count" ]]; then
      open_packs="$(jq -sr --arg task "$TASK_ID" '
        [ .[]
          | select((.task_id // "") == $task and ((.type // "") == "pack_start" or (.type // "") == "pack_end")) ] as $events
        | reduce $events[] as $event ({};
            if $event.type == "pack_start" then
              .[$event.pack_id] = ((.[$event.pack_id] // 0) + 1)
            else
              .[$event.pack_id] = ((.[$event.pack_id] // 0) - 1)
            end)
        | to_entries
        | map(select(.value > 0) | .key)
        | join(", ")
      ' .vida/logs/beads-execution.jsonl 2>/dev/null)"
      vida_status_line blocked "[health] BLOCKED: open pack(s) ${open_packs:-unknown} for task $TASK_ID" >&2
      vida_status_line info "[health] hint: finish with pack-end before full/strict-dev health-check, or use --mode quick for in-flight checkpoints" >&2
      exit 3
    fi
  else
    vida_status_line info "[health] quick mode: non-strict task checks for $TASK_ID"
    VIDA_ROOT="${VIDA_ROOT:-$ROOT_DIR}" VIDA_LEGACY_TURSO_PYTHON="${VIDA_LEGACY_TURSO_PYTHON:-$ROOT_DIR/.venv/bin/python3}" _vida/scripts-nim/src/vida beads verify --task "$TASK_ID" >/dev/null
    if ! python3 _vida/scripts/todo-runtime.py validate "$TASK_ID" --quiet >/dev/null 2>&1; then
      vida_status_line fail "[health] FAIL: TODO plan integrity check failed for task $TASK_ID" >&2
      exit 4
    fi
  fi

  if reconcile_json="$(python3 _vida/scripts/task-state-reconcile.py status "$TASK_ID" 2>/dev/null)"; then
    reconcile_class="$(jq -r '.classification // ""' <<<"$reconcile_json" 2>/dev/null)"
    if [[ -n "$reconcile_class" ]]; then
      vida_status_line info "[health] task-state reconciliation: $reconcile_class"
      if [[ "$MODE" == "strict-dev" && ( "$reconcile_class" == "drift_detected" || "$reconcile_class" == "invalid_state" ) ]]; then
        vida_status_line fail "[health] FAIL: task-state reconciliation reported $reconcile_class for $TASK_ID" >&2
        exit 10
      fi
    fi
  fi

  if [[ -f ".vida/logs/beads-execution.jsonl" ]]; then
    implementation_context_present="false"
    if task_has_open_pack "$TASK_ID" "dev-pack"; then
      implementation_context_present="true"
    fi
    if [[ "$implementation_context_present" == "true" ]]; then
      vida_status_line info "[health] execution authorization gate verification for task: $TASK_ID"
      if ! python3 _vida/scripts/execution-auth-gate.py check "$TASK_ID" implementation --local-write >/dev/null; then
        vida_status_line fail "[health] FAIL: execution authorization gate is blocked for task $TASK_ID" >&2
        exit 4
      fi
    fi

    vida_status_line info "[health] coach review gate check for task: $TASK_ID"
    if ! python3 _vida/scripts/coach-review-gate.py check "$TASK_ID" >/dev/null; then
      vida_status_line fail "[health] FAIL: coach review gate is blocked for task $TASK_ID" >&2
      exit 4
    fi

    task_labels="$(resolve_task_labels "$TASK_ID")"
    task_scope_is_framework="no"
    if grep -Eq '(^| )(scope:framework|domain:framework|framework|fsap|agent-system|vida-stack)( |$)' <<<"$task_labels"; then
      task_scope_is_framework="yes"
    fi

    subagent_run_count="0"
    if [[ -f ".vida/logs/subagent-runs.jsonl" ]]; then
      subagent_run_count="$({
        jq -r --arg task "$TASK_ID" '
          select((.task_id // "") == $task and (.type // "") == "subagent_run")
          | 1
        ' .vida/logs/subagent-runs.jsonl 2>/dev/null || true
      } | wc -l | awk "{print \$1}")"
    fi

    if [[ "$subagent_run_count" -gt 0 && ! -f ".vida/logs/subagent-review-${TASK_ID}.json" ]]; then
      vida_status_line fail "[health] FAIL: subagent runs detected for $TASK_ID but subagent review file is missing" >&2
      exit 4
    fi

    vida_status_line info "[health] tracked FSAP verification gate check for task: $TASK_ID"
    if ! python3 _vida/scripts/fsap-verification-gate.py check "$TASK_ID" >/dev/null; then
      vida_status_line fail "[health] FAIL: tracked FSAP verification gate is blocked for task $TASK_ID" >&2
      exit 4
    fi

    latest_task_receipt_epoch=""
    if [[ "$MODE" == "quick" && -d ".vida/logs/route-receipts" ]]; then
      latest_task_receipt_epoch="$(
        find .vida/logs/route-receipts -maxdepth 1 -type f -name "${TASK_ID}.*.json" -printf '%T@\n' 2>/dev/null \
          | sort -nr \
          | head -1
      )"
    fi

    if [[ "$subagent_run_count" -gt 0 && -f ".vida/logs/subagent-runs.jsonl" ]]; then
      budget_policy_summary="$(jq -sr --arg task "$TASK_ID" --arg since_epoch "$latest_task_receipt_epoch" '
        reduce .[] as $item (
          {
            bypass: 0,
            budget_violation: 0,
            internal_escalation: 0,
            internal_missing_receipt: 0
          };
          if (
            ($item.task_id // "") == $task
            and ($item.type // "") == "subagent_run"
            and (
              ($since_epoch | length) == 0
              or (((($item.ts_end // $item.ts // "") | fromdateiso8601?) // 0) >= (($since_epoch | tonumber?) // 0))
            )
          ) then
            .bypass += (if ($item.policy_bypass // false) then 1 else 0 end)
            | .budget_violation += (if ($item.budget_violation // false) then 1 else 0 end)
            | .internal_escalation += (if ($item.internal_escalation_used // false) then 1 else 0 end)
            | .internal_missing_receipt += (
                if ($item.internal_escalation_used // false) and (((($item.internal_escalation_receipt // {}) | type) != "object") or (($item.internal_escalation_receipt // {}) == {}))
                then 1 else 0 end
              )
          else . end
        )
      ' .vida/logs/subagent-runs.jsonl 2>/dev/null)"
      bypass_count="$(jq -r '.bypass // 0' <<<"$budget_policy_summary" 2>/dev/null)"
      budget_violation_count="$(jq -r '.budget_violation // 0' <<<"$budget_policy_summary" 2>/dev/null)"
      internal_escalation_count="$(jq -r '.internal_escalation // 0' <<<"$budget_policy_summary" 2>/dev/null)"
      internal_missing_receipt_count="$(jq -r '.internal_missing_receipt // 0' <<<"$budget_policy_summary" 2>/dev/null)"
      if [[ "${bypass_count:-0}" -gt 0 ]]; then
        vida_status_line warn "[health] WARN: subagent routing bypass detected for $TASK_ID (policy_bypass=${bypass_count})"
      fi
      if [[ "${budget_violation_count:-0}" -gt 0 ]]; then
        vida_status_line fail "[health] FAIL: budget policy violations detected for $TASK_ID (${budget_violation_count})" >&2
        exit 4
      fi
      if [[ "${internal_missing_receipt_count:-0}" -gt 0 ]]; then
        vida_status_line warn "[health] WARN: internal escalations without receipt detected for $TASK_ID (${internal_missing_receipt_count}/${internal_escalation_count})"
      fi
    fi

    wvp_event_text="$(jq -r --arg task "$TASK_ID" '
      select((.task_id // "") == $task and ((.type // "") == "block_end" or (.type // "") == "self_reflection" or (.type // "") == "pack_end"))
      | [(.goal // ""), (.summary // ""), (.actions // ""), (.constraints // ""), (.decision // ""), (.risks // ""), (.evidence // ""), (.evidence_ref // ""), (.assumptions // "")]
      | join(" ")
    ' .vida/logs/beads-execution.jsonl 2>/dev/null)"

    wvp_strong_trigger_count="$(printf '%s\n' "$wvp_event_text" | grep -Eic '(^|[^[:alnum:]_])(api|auth|security|crypto|package|migration|platform|ios|android)([^[:alnum:]_]|$)|dependency[_ -]*(version|upgrade|pin|compat|compatibility)|external[_ -]*(api|service|docs|contract|source|best[_ -]*practice)' || true)"
    wvp_soft_trigger_count="$(printf '%s\n' "$wvp_event_text" | grep -Eic '(^|[^[:alnum:]_])(dependency|integration|external)([^[:alnum:]_]|$)' || true)"
    wvp_trigger_count="$wvp_strong_trigger_count"
    if [[ "$wvp_trigger_count" -eq 0 && "$task_scope_is_framework" != "yes" && "$wvp_soft_trigger_count" -gt 0 ]]; then
      wvp_trigger_count="$wvp_soft_trigger_count"
    fi

    wvp_structured_count="$(jq -r --arg task "$TASK_ID" '
      select((.task_id // "") == $task and (.type // "") == "op_event" and (((.name // "") == "wvp_evidence") or ((.name // "") == "wvp_not_required")))
      | 1
    ' .vida/logs/beads-execution.jsonl 2>/dev/null | wc -l | awk "{print \$1}")"

    wvp_evidence_count="$(jq -r --arg task "$TASK_ID" '
      select((.task_id // "") == $task and ((.type // "") == "block_end" or (.type // "") == "self_reflection" or (.type // "") == "pack_end"))
      | [(.evidence // ""), (.evidence_ref // ""), (.actions // ""), (.summary // "")]
      | join(" ")
    ' .vida/logs/beads-execution.jsonl 2>/dev/null | grep -Eic '(https?://|\bcurl\b|\bWVP:\b|\bLIVE:\b|live_check=|sources=|agreement=)' || true)"

    if [[ "$wvp_strong_trigger_count" -eq 0 && "$task_scope_is_framework" == "yes" && "$wvp_soft_trigger_count" -gt 0 ]]; then
      vida_status_line info "[health] framework-scope task: soft WVP keywords ignored for $TASK_ID"
    elif [[ "$wvp_trigger_count" -gt 0 && "$wvp_structured_count" -eq 0 && "$wvp_evidence_count" -eq 0 ]]; then
      if [[ "$MODE" == "strict-dev" ]]; then
        vida_status_line info "[health] strict-dev: WVP-like triggers detected for $TASK_ID with no markers; non-blocking in dev cycle"
      elif [[ "$task_scope_is_framework" == "yes" ]]; then
        vida_status_line info "[health] framework task: WVP-like triggers are informational for $TASK_ID"
      else
        vida_status_line warn "[health] WARN: WVP-like triggers detected for $TASK_ID but no web-validation evidence markers found (see _vida/docs/web-validation-protocol.md)"
      fi
    fi
  fi
else
  vida_status_line info "[health] no task id provided, skip strict task log verification"
fi

vida_status_line ok "[health] OK legacy_jsonl_duplicates=${jsonl_duplicates}"
