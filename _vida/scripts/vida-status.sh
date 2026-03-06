#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/beads-runtime.sh"

# Keep status output readable without upstream rust tracing noise.
export RUST_LOG="${RUST_LOG:-error}"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[vida-status-todo] Missing command: $1" >&2
    exit 1
  fi
}

require_cmd jq

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

open_json="$tmp_dir/open.json"
doing_json="$tmp_dir/doing.json"
blocked_json="$tmp_dir/blocked.json"
ready_json="$tmp_dir/ready.json"

beads_br list --status open --json > "$open_json"
beads_br list --status in_progress --json > "$doing_json"
beads_br list --status blocked --json > "$blocked_json"
beads_br ready --json > "$ready_json"

all_json="$tmp_dir/all.json"
jq -s '.[0] + .[1] + .[2]' "$open_json" "$doing_json" "$blocked_json" > "$all_json"

mode_of_issue() {
  local issue_id="$1"
  local labels
  labels="$(jq -r --arg id "$issue_id" '.[] | select(.id==$id) | (.labels // []) | @json' "$all_json" | head -n 1)"
  if [[ "$labels" == *"mode:autonomous"* ]]; then
    echo "autonomous"
  elif [[ "$labels" == *"mode:decision_required"* ]]; then
    echo "decision_required"
  else
    echo "auto"
  fi
}

ready_ids="$tmp_dir/ready_ids.txt"
jq -r '.[].id // empty' "$ready_json" | sort -u > "$ready_ids"

timestamp="$(date -u +"%Y-%m-%d %H:%M:%S UTC")"

echo "VIDA STATUS TODO (read-only)"
echo "Generated: $timestamp"
echo

open_total="$(jq '[.[] | select(.parent == null)] | length' "$open_json")"
doing_total="$(jq '[.[] | select(.parent == null)] | length' "$doing_json")"
sub_open_total="$(jq '[.[] | select(.parent != null)] | length' "$open_json")"
sub_doing_total="$(jq '[.[] | select(.parent != null)] | length' "$doing_json")"
blocked_total="$(jq '[.[] | select(.parent == null)] | length' "$blocked_json")"
sub_blocked_total="$(jq '[.[] | select(.parent != null)] | length' "$blocked_json")"

echo "Summary:"
echo "- Top-level TODO (open): $open_total"
echo "- Top-level IN PROGRESS: $doing_total"
echo "- Top-level BLOCKED: $blocked_total"
echo "- Subtasks TODO (open): $sub_open_total"
echo "- Subtasks IN PROGRESS: $sub_doing_total"
echo "- Subtasks BLOCKED: $sub_blocked_total"
echo

echo "Top-level TODO (open):"
jq -r '
  [.[] | select(.parent == null and .status == "open")]
  | sort_by(.id)
  | .[]
  | "\(.id) [open] \(.title // "-") | " + ((.description // "-") | gsub("[\n\r\t]"; " ") | .[0:120])
' "$all_json" | while IFS= read -r line; do
  [[ -n "$line" ]] || continue
  issue_id="$(awk '{print $1}' <<<"$line")"
  if grep -qx "$issue_id" "$ready_ids"; then
    echo "- $line | ready=yes | mode=$(mode_of_issue "$issue_id")"
  else
    echo "- $line | ready=no | mode=$(mode_of_issue "$issue_id")"
  fi
  echo
done

echo "Top-level IN PROGRESS:"
jq -r '
  [.[] | select(.parent == null and .status == "in_progress")]
  | sort_by(.id)
  | .[]
  | "\(.id) [in_progress] \(.title // "-") | " + ((.description // "-") | gsub("[\n\r\t]"; " ") | .[0:120])
' "$all_json" | while IFS= read -r line; do
  [[ -n "$line" ]] || continue
  issue_id="$(awk '{print $1}' <<<"$line")"
  if grep -qx "$issue_id" "$ready_ids"; then
    echo "- $line | ready=yes | mode=$(mode_of_issue "$issue_id")"
  else
    echo "- $line | ready=no | mode=$(mode_of_issue "$issue_id")"
  fi
  echo
done
echo

echo "Top-level BLOCKED:"
jq -r '
  [.[] | select(.parent == null and .status == "blocked")]
  | sort_by(.id)
  | .[]
  | "\(.id) [blocked] \(.title // "-") | " + ((.description // "-") | gsub("[\n\r\t]"; " ") | .[0:120])
' "$all_json" | while IFS= read -r line; do
  [[ -n "$line" ]] || continue
  issue_id="$(awk '{print $1}' <<<"$line")"
  if grep -qx "$issue_id" "$ready_ids"; then
    echo "- $line | ready=yes | mode=$(mode_of_issue "$issue_id")"
  else
    echo "- $line | ready=no | mode=$(mode_of_issue "$issue_id")"
  fi
  echo
done
echo

echo "Subtasks TODO (open):"
jq -r '
  [.[] | select(.parent != null)]
  | sort_by(.parent, .id)
  | .[]
  | "\(.id) parent=\(.parent) [todo] \(.title // "-") | " + ((.description // "-") | gsub("[\n\r\t]"; " ") | .[0:120])
' "$open_json" | while IFS= read -r line; do
  [[ -n "$line" ]] || continue
  echo "- $line"
  echo
done
echo

echo "Subtasks IN PROGRESS:"
jq -r '
  [.[] | select(.parent != null)]
  | sort_by(.parent, .id)
  | .[]
  | "\(.id) parent=\(.parent) [in_progress] \(.title // "-") | " + ((.description // "-") | gsub("[\n\r\t]"; " ") | .[0:120])
' "$doing_json" | while IFS= read -r line; do
  [[ -n "$line" ]] || continue
  echo "- $line"
  echo
done

echo
echo "Subtasks BLOCKED:"
jq -r '
  [.[] | select(.parent != null)]
  | sort_by(.parent, .id)
  | .[]
  | "\(.id) parent=\(.parent) [blocked] \(.title // "-") | " + ((.description // "-") | gsub("[\n\r\t]"; " ") | .[0:120])
' "$blocked_json" | while IFS= read -r line; do
  [[ -n "$line" ]] || continue
  echo "- $line"
  echo
done
