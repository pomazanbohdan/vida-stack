#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$ROOT_DIR"

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/nondev-pack-init.sh <task_id> <pack_id> <goal> [constraints] [--mode autonomous|decision_required] [--profile lean|standard|full] [--no-scaffold]

Behavior:
  - starts the task,
  - sets task execution mode,
  - starts the non-dev pack,
  - optionally scaffolds canonical pack blocks,
  - validates TODO plan,
  - runs non-dev boot profile.
EOF
}

task_id="${1:-}"
pack_id="${2:-}"
goal="${3:-}"
constraints="${4:--}"
shift $(( $# >= 4 ? 4 : $# ))

[[ -n "$task_id" && -n "$pack_id" && -n "$goal" ]] || { usage; exit 1; }

case "$pack_id" in
  research-pack|spec-pack|work-pool-pack|bug-pool-pack|reflection-pack)
    ;;
  *)
    echo "[nondev-pack-init] Unsupported non-dev pack: $pack_id" >&2
    exit 1
    ;;
esac

execution_mode="autonomous"
profile="full"
scaffold="yes"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode)
      execution_mode="${2:-}"
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
    *)
      echo "[nondev-pack-init] Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

case "$execution_mode" in
  autonomous|decision_required) ;;
  *)
    echo "[nondev-pack-init] Invalid mode: $execution_mode" >&2
    exit 1
    ;;
esac

case "$profile" in
  lean|standard|full) ;;
  *)
    echo "[nondev-pack-init] Invalid profile: $profile" >&2
    exit 1
    ;;
esac

bash _vida/scripts/beads-workflow.sh start "$task_id"
bash _vida/scripts/task-execution-mode.sh set "$task_id" "$execution_mode" "Initialized via nondev-pack-init for $pack_id"
bash _vida/scripts/vida-pack-helper.sh start "$task_id" "$pack_id" "$goal" "$constraints"

if [[ "$scaffold" == "yes" ]]; then
  bash _vida/scripts/vida-pack-helper.sh scaffold "$task_id" "$pack_id"
  bash _vida/scripts/todo-plan-validate.sh "$task_id" --strict
fi

bash _vida/scripts/boot-profile.sh run "$profile" "$task_id" --non-dev
echo "[nondev-pack-init] ready task=$task_id pack=$pack_id mode=$execution_mode profile=$profile scaffold=$scaffold"
