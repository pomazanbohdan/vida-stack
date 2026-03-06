#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
WORKFLOW_SCRIPT="$SCRIPT_DIR/beads-workflow.sh"
ROUTER_SCRIPT="$SCRIPT_DIR/vida-pack-router.sh"

cd "$ROOT_DIR"

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/vida-pack-helper.sh detect "<request>"
  bash _vida/scripts/vida-pack-helper.sh start <task_id> <pack_id> <goal> [constraints]
  bash _vida/scripts/vida-pack-helper.sh end <task_id> <pack_id> <done|partial|failed> <summary> [next_step]
  bash _vida/scripts/vida-pack-helper.sh scaffold <task_id> <pack_id> [variant]

Pack IDs:
  research-pack | spec-pack | work-pool-pack | dev-pack | bug-pool-pack | reflection-pack
EOF
}

validate_pack() {
  case "$1" in
    research-pack|spec-pack|work-pool-pack|dev-pack|bug-pool-pack|reflection-pack)
      ;;
    *)
      echo "[vida-pack-helper] Unknown pack: $1" >&2
      exit 1
      ;;
  esac
}

matches_any() {
  local haystack="$1"
  shift
  local token
  for token in "$@"; do
    if [[ "$haystack" == *"$token"* ]]; then
      return 0
    fi
  done
  return 1
}

reflection_variant_for() {
  local task_id="$1"
  local explicit_variant="${2:-}"
  local task_blob=""

  if [[ "$explicit_variant" == "fsap" ]]; then
    echo "fsap"
    return 0
  fi

  task_blob="$(
    br show "$task_id" --json 2>/dev/null \
      | jq -r '.[0] | [(.title // ""), (.description // ""), (.notes // ""), ((.labels // []) | join(" "))] | join(" ")' 2>/dev/null \
      | tr '[:upper:]' '[:lower:]'
  )"

  if matches_any "$task_blob" \
    "framework self-analysis" \
    "framework diagnosis" \
    "framework self-diagnosis" \
    "self-analysis" \
    "self-diagnosis" \
    "instruction conflict" \
    "protocol friction" \
    "token overhead" \
    "fsap"; then
    echo "fsap"
  else
    echo "standard"
  fi
}

scaffold_pack_blocks() {
  local task_id="$1"
  local pack_id="$2"
  local variant="${3:-}"
  case "$pack_id" in
    research-pack)
      bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "P01" "Clarify_research_questions_and_acceptance_criteria" "main" "orchestrator" "-" "P02"
      bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "P02" "Collect_sources_and_evidence" "main" "orchestrator" "P01" "P03"
      bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "P03" "Synthesize_findings_and_risks" "main" "orchestrator" "P02" "-"
      ;;
    spec-pack)
      bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "SCP01" "SCP-0_1_Intake_and_Interactive_Discovery" "main" "orchestrator" "-" "SCP02"
      bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "SCP02" "SCP-2_Conflict_Check_and_SCP-3_API_Reality_Validation" "main" "orchestrator" "SCP01" "SCP03"
      bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "SCP03" "SCP-4_Design_Contract_and_SCP-5_Technical_Contract" "main" "orchestrator" "SCP02" "-"
      ;;
    work-pool-pack)
      bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "FT01" "FTP-0_1_Intake_and_Preflight_Checks" "main" "orchestrator" "-" "FT02"
      bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "FT02" "FTP-2_3_Task_Scope_Options_and_User_Decision_Cards" "main" "orchestrator" "FT01" "FT03"
      bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "FT03" "FTP-4_Build_or_Update_Task_Pool_in_br" "main" "orchestrator" "FT02" "-"
      ;;
    dev-pack)
      bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "P01" "Prepare_Context_and_Change_Impact" "main" "orchestrator" "-" "P02"
      bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "P02" "Implement_Changes_with_Tests" "main" "orchestrator" "P01" "P03"
      bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "P03" "Verify_Quality_and_Regressions" "main" "orchestrator" "P02" "-"
      ;;
    bug-pool-pack)
      bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "BFP01" "BFP-0_1_Normalize_Issue_Set_and_Prioritize_Impact" "main" "orchestrator" "-" "BFP02"
      bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "BFP02" "BFP-2_3_Reproduce_and_Root_Cause_with_Evidence" "main" "orchestrator" "BFP01" "BFP03"
      bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "BFP03" "BFP-4_5_Fix_Plan_and_Implementation" "main" "orchestrator" "BFP02" "-"
      ;;
    reflection-pack)
      if [[ "$(reflection_variant_for "$task_id" "$variant")" == "fsap" ]]; then
        bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "FSAP01" "FSAP-0_2_Trigger_Runtime_Snapshot_and_Evidence_Scope" "main" "orchestrator" "-" "FSAP02"
        bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "FSAP02" "FSAP-3_5_Friction_Classification_Ownership_Split_and_Improvement_Decision" "main" "orchestrator" "FSAP01" "FSAP03"
        bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "FSAP03" "FSAP-6_8_Canonical_Update_Verification_and_Report" "main" "orchestrator" "FSAP02" "-"
      else
        bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "P01" "Reconcile_Decisions_with_Canonical_Docs" "main" "orchestrator" "-" "P02"
        bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "P02" "Update_SSoT_and_Protocol_Index" "main" "orchestrator" "P01" "P03"
        bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "P03" "Capture_Changes_and_Verify_Conflicts" "main" "orchestrator" "P02" "-"
      fi
      ;;
  esac
}

cmd="${1:-}"
case "$cmd" in
  detect)
    shift || true
    [[ $# -gt 0 ]] || { usage; exit 1; }
    bash "$ROUTER_SCRIPT" "$*"
    ;;
  start)
    task_id="${2:-}"
    pack_id="${3:-}"
    goal="${4:-}"
    constraints="${5:--}"
    [[ -n "$task_id" && -n "$pack_id" && -n "$goal" ]] || { usage; exit 1; }
    validate_pack "$pack_id"
    bash "$WORKFLOW_SCRIPT" pack-start "$task_id" "$pack_id" "$goal" "$constraints"
    ;;
  end)
    task_id="${2:-}"
    pack_id="${3:-}"
    result="${4:-}"
    summary="${5:-}"
    next_step="${6:--}"
    [[ -n "$task_id" && -n "$pack_id" && -n "$result" && -n "$summary" ]] || { usage; exit 1; }
    validate_pack "$pack_id"
    bash "$WORKFLOW_SCRIPT" pack-end "$task_id" "$pack_id" "$result" "$summary" "$next_step"
    ;;
  scaffold)
    task_id="${2:-}"
    pack_id="${3:-}"
    variant="${4:-}"
    [[ -n "$task_id" && -n "$pack_id" ]] || { usage; exit 1; }
    validate_pack "$pack_id"
    scaffold_pack_blocks "$task_id" "$pack_id" "$variant"
    ;;
  *)
    usage
    exit 1
    ;;
esac
