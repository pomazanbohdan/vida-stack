#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
LOG_SCRIPT="$SCRIPT_DIR/beads-log.sh"

cd "$ROOT_DIR"

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/tool-capability.sh matrix
  bash _vida/scripts/tool-capability.sh resolve <required_tool>
  bash _vida/scripts/tool-capability.sh evidence <task_id> <required_tool> [impact]
  bash _vida/scripts/tool-capability.sh use <task_id> <required_tool> [impact]

Examples:
  bash _vida/scripts/tool-capability.sh resolve skill_use
  bash _vida/scripts/tool-capability.sh evidence bd-261a.3 skill_use "minimal-token fallback"
  bash _vida/scripts/tool-capability.sh use bd-261a.3 skill_use "minimal-token fallback"
EOF
}

resolve_fallback() {
  local required="$1"
  case "$required" in
    skill_use)
      echo "read SKILL.md directly"
      ;;
    Task)
      echo "use agent tool with isolated scope"
      ;;
    web.run)
      echo "use browser fetch/open flow"
      ;;
    br)
      echo "stop execution (no SSOT fallback)"
      ;;
    *)
      echo "manual fallback with explicit report line"
      ;;
  esac
}

cmd="${1:-}"
case "$cmd" in
  matrix)
    cat <<'EOF'
required_tool | fallback | impact_policy
skill_use | read SKILL.md directly | low
Task | use agent tool with isolated scope | medium
web.run | use browser fetch/open flow | low
br | stop execution (no SSOT fallback) | critical
EOF
    ;;
  resolve)
    required="${2:-}"
    [[ -n "$required" ]] || { usage; exit 1; }
    resolve_fallback "$required"
    ;;
  evidence)
    task_id="${2:-}"
    required="${3:-}"
    impact="${4:-operational}"
    [[ -n "$task_id" && -n "$required" ]] || { usage; exit 1; }
    fallback="$(resolve_fallback "$required")"
    meta="required=${required};fallback=${fallback};impact=${impact}"
    bash "$LOG_SCRIPT" op-event "$task_id" "tool_capability_fallback" "$meta"
    echo "${required} -> ${fallback} -> ${impact}"
    ;;
  use)
    task_id="${2:-}"
    required="${3:-}"
    impact="${4:-operational}"
    [[ -n "$task_id" && -n "$required" ]] || { usage; exit 1; }
    fallback="$(resolve_fallback "$required")"
    meta="required=${required};fallback=${fallback};impact=${impact}"
    bash "$LOG_SCRIPT" op-event "$task_id" "tool_capability_fallback" "$meta"
    echo "$fallback"
    ;;
  *)
    usage
    exit 1
    ;;
esac
