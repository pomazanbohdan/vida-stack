#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
WORKFLOW_SCRIPT="$SCRIPT_DIR/beads-workflow.sh"
TODO_TOOL="$SCRIPT_DIR/todo-tool.sh"
LOG_FILE="$ROOT_DIR/.vida/logs/beads-execution.jsonl"

cd "$ROOT_DIR"

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/vida-command-audit.sh report <task_id>
  bash _vida/scripts/vida-command-audit.sh plan <task_id> [--limit N]
  bash _vida/scripts/vida-command-audit.sh repair-next <task_id>

Purpose:
  Keep command-layer audit TODO in sync with actual progress.

Notes:
  - Uses canonical `/vida-*#CLx` protocol units as inventory.
  - Detects completed protocol-unit analyses from beads execution events.
  - `plan` pre-registers TODO blocks for pending protocol units.
  - `repair-next` rewrites CMD block `next_step` links to canonical CMD chain.
EOF
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[vida-command-audit] Missing command: $1" >&2
    exit 1
  fi
}

require_cmd jq
require_cmd awk
require_cmd sed

cmd="${1:-}"
task_id="${2:-}"
[[ -n "$cmd" && -n "$task_id" ]] || {
  usage
  exit 1
}

shift 2 || true

limit="0"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --limit)
      limit="${2:-0}"
      shift 2
      ;;
    *)
      echo "[vida-command-audit] Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

inventory_file="$tmp_dir/inventory.txt"
done_file="$tmp_dir/done.txt"
planned_file="$tmp_dir/planned.txt"
pending_file="$tmp_dir/pending.txt"
unplanned_file="$tmp_dir/unplanned.txt"

list_inventory() {
  local command
  while IFS= read -r command; do
    [[ -n "$command" ]] || continue
    printf '%s#CL1\n' "$command"
    printf '%s#CL2\n' "$command"
    printf '%s#CL3\n' "$command"
    printf '%s#CL4\n' "$command"
    printf '%s#CL5\n' "$command"
  done < <(
    find "$ROOT_DIR/_vida/commands" -maxdepth 1 -type f -name 'vida*.md' \
      | sed -E 's#^.*/(vida[^/]*)\.md$#/\1#' \
      | sort -u
  )
}

extract_units_from_text() {
  sed -E 's~([^/]|^)(/vida[-a-z0-9]*(#CL[1-5])?)~\n\2\n~g' \
    | grep -E '^/vida[-a-z0-9]*(#CL[1-5])?$' \
    | sort -u
}

collect_done() {
  [[ -f "$LOG_FILE" ]] || return 0
  jq -r --arg task_id "$task_id" '
    select((.task_id // "") == $task_id and (.type == "block_end") and ((.result // "") == "done"))
    | [.goal // "", .actions // "", .next_step // "", .evidence_ref // ""]
    | join(" ")
  ' "$LOG_FILE" | extract_units_from_text
}

collect_planned() {
  bash "$TODO_TOOL" ui-json "$task_id" \
    | jq -r '.steps[]? | [.goal // "", .next_step // ""] | join(" ")' \
    | extract_units_from_text
}

next_cmd_block_num_base() {
  bash "$TODO_TOOL" ui-json "$task_id" \
    | jq -r '.steps[]?.block_id // ""' \
    | awk '/^CMD[0-9]+$/ {gsub(/^CMD/, "", $0); if ($0+0>m) m=$0+0} END {print m+0}'
}

list_inventory > "$inventory_file"
collect_done > "$tmp_dir/done_raw.txt" || true
collect_planned > "$tmp_dir/planned_raw.txt" || true

comm -12 <(sort -u "$inventory_file") <(sort -u "$tmp_dir/done_raw.txt") > "$done_file"
comm -12 <(sort -u "$inventory_file") <(sort -u "$tmp_dir/planned_raw.txt") > "$planned_file"

comm -23 "$inventory_file" <(sort -u "$done_file") | sort -u > "$pending_file"
comm -23 "$pending_file" <(sort -u "$planned_file") | sort -u > "$unplanned_file"

case "$cmd" in
  report)
    echo "Inventory: $(wc -l < "$inventory_file" | tr -d ' ')"
    echo "Done:      $(wc -l < "$done_file" | tr -d ' ')"
    echo "Pending:   $(wc -l < "$pending_file" | tr -d ' ')"
    echo "Planned:   $(wc -l < "$planned_file" | tr -d ' ')"
    echo "Unplanned: $(wc -l < "$unplanned_file" | tr -d ' ')"
    echo
    echo "Done protocol units:"
    sed 's/^/- /' "$done_file"
    echo
    echo "Pending protocol units:"
    sed 's/^/- /' "$pending_file"
    ;;
  plan)
    created=0
    mapfile -t pending_units < "$unplanned_file"
    total="${#pending_units[@]}"
    base_num="$(next_cmd_block_num_base)"

    for i in "${!pending_units[@]}"; do
      protocol_unit="${pending_units[$i]}"
      [[ -n "$protocol_unit" ]] || continue
      if [[ "$limit" -gt 0 && "$created" -ge "$limit" ]]; then
        break
      fi
      current_num=$((base_num + created + 1))
      block_id="$(printf 'CMD%02d' "$current_num")"
      goal="Analyze_Protocol_Unit_${protocol_unit}_and_Decide_Keep_Simplify_Merge_Remove"
      if (( i + 1 < total )) && { [[ "$limit" -eq 0 ]] || (( created + 1 < limit )); }; then
        next_num=$((current_num + 1))
        next_hint="$(printf 'CMD%02d' "$next_num")"
      else
        next_hint="-"
      fi
      bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "$block_id" "$goal" "main" "orchestrator" "-" "$next_hint"
      created=$((created + 1))
    done

    echo "[vida-command-audit] planned=$created task=$task_id"
    ;;
  repair-next)
    repaired=0
    bash "$TODO_TOOL" ui-json "$task_id" \
      | jq -r '.steps[]? | select(.block_id|test("^CMD[0-9]+$"))
        | [.block_id, (.goal // ""), (.track_id // "main"), (.owner // "orchestrator"), (.depends_on // "")]
        | @tsv' \
      | sort -t$'\t' -k1,1 \
      > "$tmp_dir/cmd_steps.tsv"

    mapfile -t cmd_lines < "$tmp_dir/cmd_steps.tsv"
    total_cmd="${#cmd_lines[@]}"

    if [[ "$total_cmd" -eq 0 ]]; then
      echo "[vida-command-audit] no CMD blocks found for task=$task_id"
      exit 0
    fi

    for i in "${!cmd_lines[@]}"; do
      IFS=$'\t' read -r block_id goal track_id owner depends_on <<< "${cmd_lines[$i]}"
      [[ -n "$track_id" ]] || track_id="main"
      [[ -n "$owner" ]] || owner="orchestrator"
      if (( i + 1 < total_cmd )); then
        IFS=$'\t' read -r next_id _ <<< "${cmd_lines[$((i + 1))]}"
        next_step="$next_id"
      else
        next_step="-"
      fi
      bash "$WORKFLOW_SCRIPT" block-plan "$task_id" "$block_id" "$goal" "$track_id" "$owner" "$depends_on" "$next_step" >/dev/null
      repaired=$((repaired + 1))
    done

    echo "[vida-command-audit] repaired_next_chain=$repaired task=$task_id"
    ;;
  *)
    usage
    exit 1
    ;;
esac
