#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

declare -A DISCOVERED_SKILLS=()

declare -a REQUIRED_SKILLS=(
  "vida-code-quality"
  "vida-git-workflow"
  "vida-security-owasp"
  "vida-delegation"
  "ui-ux-pro-max"
  "systematic-debugging"
)

declare -a SEARCH_ROOTS=(
  "$ROOT_DIR/.agents/skills"
  "$HOME/.agents/skills"
  "$HOME/.code/skills"
)

missing=0
echo "[validate-skills] Checking required skills..."

for root in "${SEARCH_ROOTS[@]}"; do
  [[ -d "$root" ]] || continue
  while IFS= read -r candidate; do
    skill_name="$(basename "$(dirname "$candidate")")"
    [[ -n "$skill_name" ]] || continue
    if [[ -z "${DISCOVERED_SKILLS[$skill_name]:-}" ]]; then
      DISCOVERED_SKILLS["$skill_name"]="$candidate"
    fi
  done < <(find "$root" -type f -name SKILL.md 2>/dev/null)
done

for skill in "${REQUIRED_SKILLS[@]}"; do
  found_path="${DISCOVERED_SKILLS[$skill]:-}"
  if [[ -n "$found_path" ]]; then
    echo "[OK] $skill -> $found_path"
  else
    echo "[MISSING] $skill"
    missing=$((missing + 1))
  fi
done

if [[ "$missing" -gt 0 ]]; then
  echo "[validate-skills] Missing skills: $missing"
  exit 2
fi

echo "[validate-skills] All required skills found"
