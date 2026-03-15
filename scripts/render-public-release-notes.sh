#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  cat <<'EOF'
Render the public GitHub release body from a canonical release-note artifact.

Usage:
  scripts/render-public-release-notes.sh <vX.Y.Z|path-to-release-note.md>
EOF
}

fail() {
  printf '[render-public-release-notes] ERROR: %s\n' "$*" >&2
  exit 1
}

INPUT="${1:-}"
[[ -n "$INPUT" ]] || { usage >&2; exit 1; }

if [[ -f "$INPUT" ]]; then
  SOURCE_PATH="$INPUT"
else
  SOURCE_PATH="$ROOT_DIR/install/release-notes-${INPUT}.md"
fi

[[ -f "$SOURCE_PATH" ]] || fail "Release-note source not found: $SOURCE_PATH"

awk '
  BEGIN {
    dropped_title = 0
    dropped_blank_after_title = 0
  }
  /^-----$/ { exit }
  {
    if (dropped_title == 0 && $0 ~ /^# /) {
      dropped_title = 1
      next
    }
    if (dropped_title == 1 && dropped_blank_after_title == 0 && $0 ~ /^[[:space:]]*$/) {
      dropped_blank_after_title = 1
      next
    }
    print
  }
' "$SOURCE_PATH"
