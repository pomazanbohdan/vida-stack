#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$ROOT_DIR"

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/framework-wave-start.sh <task_id> <pack_id> <goal> [constraints] [--mode autonomous|decision_required] [--profile lean|standard|full] [--no-scaffold] [--variant <name>]

Purpose:
  Lean starter for framework-owned VIDA work. It keeps SSOT/TODO invariants,
  but compresses the common start path into one command:
  - task start
  - execution mode
  - pack start
  - optional scaffold
  - TODO plan validation
  - boot profile
  - compact TODO snapshot
EOF
}

task_id="${1:-}"
pack_id="${2:-}"
goal="${3:-}"
constraints="${4:--}"
shift $(( $# >= 4 ? 4 : $# ))

[[ -n "$task_id" && -n "$pack_id" && -n "$goal" ]] || { usage; exit 1; }

mode="autonomous"
profile="lean"
scaffold="yes"
variant=""

has_existing_plan() {
  local issue_id="$1"
  local count
  count="$(
    python3 _vida/scripts/todo-runtime.py ui-json "$issue_id" 2>/dev/null \
      | jq -r '.steps | length' 2>/dev/null || echo 0
  )"
  [[ "${count:-0}" =~ ^[0-9]+$ ]] || count=0
  (( count > 0 ))
}

resolve_task_labels() {
  local issue_id="$1"
  local labels
  labels="$(
    br show "$issue_id" --json 2>/dev/null \
      | jq -r '.[0].labels // [] | join(" ")' 2>/dev/null || true
  )"
  if [[ -n "${labels// }" ]]; then
    printf '%s\n' "$labels"
    return 0
  fi
  python3 - "$issue_id" <<'PY'
import json
import sys
from pathlib import Path

issue_id = sys.argv[1]
issues_path = Path(".beads/issues.jsonl")
if not issues_path.exists():
    raise SystemExit(0)
for line in issues_path.read_text(encoding="utf-8").splitlines():
    line = line.strip()
    if not line:
        continue
    try:
        item = json.loads(line)
    except json.JSONDecodeError:
        continue
    if str(item.get("id", "")).strip() != issue_id:
        continue
    labels = item.get("labels", [])
    if isinstance(labels, list):
        print(" ".join(str(label).strip() for label in labels if str(label).strip()))
    break
PY
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode)
      mode="${2:-}"
      shift 2
      ;;
    --profile)
      profile="${2:-}"
      shift 2
      ;;
    --no-scaffold)
      scaffold="no"
      shift
      ;;
    --variant)
      variant="${2:-}"
      shift 2
      ;;
    *)
      echo "[framework-wave-start] Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

case "$mode" in
  autonomous|decision_required) ;;
  *)
    echo "[framework-wave-start] Invalid mode: $mode" >&2
    exit 1
    ;;
esac

case "$profile" in
  lean|standard|full) ;;
  *)
    echo "[framework-wave-start] Invalid profile: $profile" >&2
    exit 1
    ;;
esac

case "$pack_id" in
  research-pack|spec-pack|work-pool-pack|dev-pack|bug-pool-pack|reflection-pack) ;;
  *)
    echo "[framework-wave-start] Unsupported pack: $pack_id" >&2
    exit 1
    ;;
esac

labels="$(resolve_task_labels "$task_id")"

if ! grep -Eq '(^| )(framework|agent-system|fsap|vida-stack|local-platform-alignment|registry|evals|context|operator-surface|durability)( |$)' <<<"$labels"; then
  echo "[framework-wave-start] Refusing non-framework task: $task_id" >&2
  echo "[framework-wave-start] Expected labels to include framework|agent-system|fsap|vida-stack|local-platform-alignment|registry|evals|context|operator-surface|durability" >&2
  exit 1
fi

bash _vida/scripts/beads-workflow.sh start "$task_id"
bash _vida/scripts/task-execution-mode.sh set "$task_id" "$mode" "Initialized via framework-wave-start for $pack_id"
bash _vida/scripts/vida-pack-helper.sh start "$task_id" "$pack_id" "$goal" "$constraints"

if [[ "$scaffold" == "yes" ]]; then
  if [[ -n "$variant" ]]; then
    bash _vida/scripts/vida-pack-helper.sh scaffold "$task_id" "$pack_id" "$variant"
  else
    bash _vida/scripts/vida-pack-helper.sh scaffold "$task_id" "$pack_id"
  fi
  bash _vida/scripts/todo-plan-validate.sh "$task_id" --strict
elif ! has_existing_plan "$task_id"; then
  if [[ -n "$variant" ]]; then
    bash _vida/scripts/vida-pack-helper.sh scaffold "$task_id" "$pack_id" "$variant"
  else
    bash _vida/scripts/vida-pack-helper.sh scaffold "$task_id" "$pack_id"
  fi
  bash _vida/scripts/todo-plan-validate.sh "$task_id" --strict
  echo "[framework-wave-start] no-scaffold requested but no TODO plan existed; minimal scaffold created for $task_id" >&2
fi

if [[ "$pack_id" == "dev-pack" ]]; then
  bash _vida/scripts/boot-profile.sh run "$profile" "$task_id"
else
  bash _vida/scripts/boot-profile.sh run "$profile" "$task_id" --non-dev
fi

bash _vida/scripts/todo-tool.sh compact "$task_id" 4
echo "[framework-wave-start] ready task=$task_id pack=$pack_id mode=$mode profile=$profile scaffold=$scaffold"
