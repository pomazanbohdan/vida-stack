#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/beads-runtime.sh"
source "$SCRIPT_DIR/status-ui.sh"

cd "$ROOT_DIR"

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
  "_vida/docs/subagent-system-protocol.md"
  "_vida/scripts/validate-skills.sh"
  "_vida/scripts/beads-workflow.sh"
  "_vida/scripts/boot-profile.sh"
  "_vida/scripts/beads-bg-sync.sh"
  "_vida/scripts/tool-capability.sh"
  "_vida/scripts/context-drift-sentinel.sh"
  "_vida/scripts/eval-pack.sh"
  "_vida/scripts/beads-compact.sh"
  "_vida/scripts/beads-log.sh"
  "_vida/scripts/beads-verify-log.sh"
  "_vida/scripts/todo-tool.sh"
  "_vida/scripts/todo-sync-plan.sh"
  "_vida/scripts/todo-plan-validate.sh"
  "_vida/scripts/stateful-sequence-check.sh"
  "_vida/scripts/framework-boundary-check.sh"
  "_vida/scripts/framework-self-check.sh"
  "_vida/scripts/vida-pack-router.sh"
  "_vida/scripts/vida-pack-helper.sh"
  "_vida/scripts/nondev-pack-init.sh"
  "_vida/scripts/wvp-evidence.sh"
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
  "_vida/scripts/framework-operator-status.py"
)

vida_status_line info "[health] checking required files"
for f in "${required_files[@]}"; do
  if [[ ! -f "$f" ]]; then
    echo "[health] MISSING: $f" >&2
    exit 2
  fi
done

vida_status_line info "[health] skills"
bash _vida/scripts/validate-skills.sh
bash _vida/scripts/framework-boundary-check.sh --strict >/dev/null

mode="$(beads_mode)"
jsonl_stats="$(beads_jsonl_stats)"
jsonl_duplicates="$(jq -r '.duplicates' <<<"$jsonl_stats")"
snapshot_age_sec="$(beads_snapshot_age_seconds)"

if [[ "$jsonl_duplicates" != "0" ]]; then
  vida_status_line fail "[health] FAIL: duplicate issue ids detected in issues.jsonl ($jsonl_duplicates)" >&2
  exit 5
fi

if [[ "$snapshot_age_sec" -lt 0 ]]; then
  vida_status_line fail "[health] FAIL: latest JSONL snapshot is missing" >&2
  exit 5
fi

if (( snapshot_age_sec > 1800 )); then
  vida_status_line fail "[health] FAIL: latest JSONL snapshot too old (${snapshot_age_sec}s)" >&2
  exit 5
fi

runtime_scripts=(
  "_vida/scripts/beads-workflow.sh"
  "_vida/scripts/beads-bg-sync.sh"
  "_vida/scripts/quality-health-check.sh"
  "_vida/scripts/beads-verify-log.sh"
  "_vida/scripts/br-db-quarantine.sh"
  "_vida/scripts/vida-status.sh"
  "_vida/scripts/context-capsule.sh"
  "_vida/scripts/beads-compact.sh"
  "_vida/scripts/eval-pack.sh"
  "_vida/scripts/task-execution-mode.sh"
)

raw_br_hits="$(rg -n --pcre2 '^(?!\\s*#).*?\\bbr (?=(ready|show|update|close|sync|list|create|dep|init)\\b)' "${runtime_scripts[@]}" 2>/dev/null || true)"
if [[ -n "$raw_br_hits" ]]; then
  vida_status_line fail "[health] FAIL: raw br runtime calls remain" >&2
  echo "$raw_br_hits" >&2
  exit 6
fi

vida_status_line info "[health] beads status"
if ! bash _vida/scripts/beads-workflow.sh status >/dev/null; then
  vida_status_line fail "[health] FAIL: beads-workflow status failed in mode=$mode" >&2
  exit 7
fi

bg_status="$(bash _vida/scripts/beads-bg-sync.sh status)"
bg_running="$(sed -n 's/.*running=\([a-z]*\).*/\1/p' <<<"$bg_status" | head -n1)"
bg_role="$(sed -n 's/.*role=\([^ ]*\).*/\1/p' <<<"$bg_status" | head -n1)"
if [[ "$bg_running" == "yes" && "$bg_role" != "backup_only" ]]; then
  vida_status_line fail "[health] FAIL: background worker is not in backup_only mode" >&2
  exit 8
fi

if ! rg -q 'beads-bg-sync\.sh.*stop|beads-bg-sync\.sh" stop|BG_SYNC.*stop' _vida/scripts/br-db-quarantine.sh; then
  vida_status_line fail "[health] FAIL: quarantine script does not stop background worker" >&2
  exit 9
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

if [[ -n "$TASK_ID" ]]; then
  vida_status_line info "[health] boot receipt verification for task: $TASK_ID"
  if ! bash _vida/scripts/boot-profile.sh verify-receipt "$TASK_ID" >/dev/null; then
    vida_status_line fail "[health] FAIL: missing or invalid boot receipt/packet for task $TASK_ID" >&2
    exit 4
  fi

  if [[ "$MODE" == "full" || "$MODE" == "strict-dev" ]]; then
    vida_status_line info "[health] strict log verify for task: $TASK_ID"
    bash _vida/scripts/beads-verify-log.sh --task "$TASK_ID" --strict

    vida_status_line info "[health] todo sync snapshot for task: $TASK_ID"
    bash _vida/scripts/todo-sync-plan.sh "$TASK_ID" --mode json-only --quiet >/dev/null

    vida_status_line info "[health] todo board render for task: $TASK_ID"
    bash _vida/scripts/todo-tool.sh board "$TASK_ID" >/dev/null

    vida_status_line info "[health] todo plan integrity for task: $TASK_ID"
    if ! bash _vida/scripts/todo-plan-validate.sh "$TASK_ID" --quiet >/dev/null 2>&1; then
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
    bash _vida/scripts/beads-verify-log.sh --task "$TASK_ID" >/dev/null
    if ! bash _vida/scripts/todo-plan-validate.sh "$TASK_ID" --quiet >/dev/null 2>&1; then
      vida_status_line fail "[health] FAIL: TODO plan integrity check failed for task $TASK_ID" >&2
      exit 4
    fi
  fi

  if [[ -f ".vida/logs/beads-execution.jsonl" ]]; then
    writer_block_started="$(
      jq -sr --arg task "$TASK_ID" '
        any(
          .[];
          (.task_id // "") == $task
          and (.type // "") == "block_start"
          and ((.block_id // "") == "P02" or (.block_id // "") == "IEP04" or (.block_id // "") == "CL4")
        )
      ' .vida/logs/beads-execution.jsonl 2>/dev/null || echo false
    )"
    if [[ "$writer_block_started" == "true" ]]; then
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

    task_labels="$(br show "$TASK_ID" --json 2>/dev/null | jq -r '.[0].labels // [] | join(" ")' 2>/dev/null || true)"
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

vida_status_line ok "[health] OK mode=$mode backup_age=${snapshot_age_sec}s bg=$bg_role"
