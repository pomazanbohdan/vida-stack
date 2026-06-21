#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GIT_BIN="${GIT:-git}"

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

resolve_current_tag() {
  local input="$1"
  local base
  if [[ -f "$input" ]]; then
    base="$(basename "$input")"
    case "$base" in
      release-notes-v*.md)
        printf '%s\n' "${base#release-notes-}" | sed 's/\.md$//'
        return 0
        ;;
      *)
        fail "Cannot infer release tag from note path: $input"
        ;;
    esac
  fi
  printf '%s\n' "$input"
}

resolve_previous_tag() {
  local current_tag="$1"
  "$GIT_BIN" tag --list 'v*' --sort=-v:refname | awk -v current="$current_tag" '
    $0 == current { seen = 1; next }
    seen == 1 { print; exit }
  '
}

INPUT="${1:-}"
[[ -n "$INPUT" ]] || { usage >&2; exit 1; }

if [[ -f "$INPUT" ]]; then
  SOURCE_PATH="$INPUT"
else
  SOURCE_PATH="$ROOT_DIR/install/release-notes-${INPUT}.md"
fi

[[ -f "$SOURCE_PATH" ]] || fail "Release-note source not found: $SOURCE_PATH"
command -v "$GIT_BIN" >/dev/null 2>&1 || fail "git is required to render the public commit ledger"

CURRENT_TAG="$(resolve_current_tag "$INPUT")"
"$GIT_BIN" rev-parse --verify "${CURRENT_TAG}^{commit}" >/dev/null 2>&1 || fail "Release tag not found: $CURRENT_TAG"
PREVIOUS_TAG="$(resolve_previous_tag "$CURRENT_TAG")"
[[ -n "$PREVIOUS_TAG" ]] || fail "Previous release tag not found before $CURRENT_TAG"
"$GIT_BIN" rev-parse --verify "${PREVIOUS_TAG}^{commit}" >/dev/null 2>&1 || fail "Previous release tag not found: $PREVIOUS_TAG"

awk '
  BEGIN {
    dropped_title = 0
    dropped_blank_after_title = 0
  }
  /^-----$/ { exit }
  /^## Commit Ledger[[:space:]]*$/ { exit }
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

printf '\n## Commit Ledger\n\n'
printf 'Commits since `%s`:\n\n' "$PREVIOUS_TAG"
"$GIT_BIN" log --no-merges --format='- `%h` %s' "${PREVIOUS_TAG}..${CURRENT_TAG}"
