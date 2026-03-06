#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PROJECT_PREFLIGHT_DOC="docs/process/project-operations.md"

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/render-subagent-prompt.sh <audit|implementation|decision|patch> \
    --task "..." \
    [--protocol-unit "/vida-command#CLx"] \
    --scope "..." \
    --verification "..." \
    [--extra "..."] \
    [--repo-root PATH] \
    [--odoo-json]

Examples:
  bash _vida/scripts/render-subagent-prompt.sh audit \
    --task "Audit auth retry flow" \
    --scope "src/lib/core/api" \
    --verification "rg -n \"retry\" src/lib/core/api"

  bash _vida/scripts/render-subagent-prompt.sh implementation \
    --task "Implement menu cache invalidation" \
    --scope "src/lib/shared/providers,src/lib/core/cache" \
    --verification "cd src && flutter analyze --no-pub" \
    --extra "Keep changes scoped to menu refresh flow."
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

template="${1:-}"
shift || true

task=""
protocol_unit=""
scope=""
verification=""
extra=""
repo_root="$ROOT_DIR"
odoo_json="no"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --task)
      task="${2:-}"
      shift 2
      ;;
    --scope)
      scope="${2:-}"
      shift 2
      ;;
    --protocol-unit)
      protocol_unit="${2:-}"
      shift 2
      ;;
    --verification)
      verification="${2:-}"
      shift 2
      ;;
    --extra)
      extra="${2:-}"
      shift 2
      ;;
    --repo-root)
      repo_root="${2:-}"
      shift 2
      ;;
    --odoo-json)
      odoo_json="yes"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[render-subagent-prompt] Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ -z "$template" || -z "$task" || -z "$scope" || -z "$verification" ]]; then
  usage >&2
  exit 1
fi

protocol_unit_line=""
if [[ -n "$protocol_unit" ]]; then
  protocol_unit_line="Protocol Unit: $protocol_unit"
fi

odoo_line=""
if [[ "$odoo_json" == "yes" ]]; then
  odoo_line=$'- Odoo JSON note: Odoo returns false instead of null for empty fields.'
fi

json_contract=$(cat <<'EOF'
{
  "status": "done|partial|blocked",
  "changed_files": ["path/a", "path/b"],
  "verification_commands": ["exact command"],
  "verification_results": ["command -> pass|fail"],
  "merge_ready": "yes|no",
  "blockers": [],
  "notes": "short note"
}
EOF
)

extra_line=""
if [[ -n "$extra" ]]; then
  extra_line="- Additional constraint: $extra"
fi

optional_lines() {
  local first="$1"
  local second="$2"
  if [[ -n "$first" ]]; then
    printf '%s\n' "$first"
  fi
  if [[ -n "$second" ]]; then
    printf '%s\n' "$second"
  fi
}

case "$template" in
  audit|read-only-audit)
    cat <<EOF
Task: $task in $repo_root.
Mode: READ-ONLY (do not modify files).
$protocol_unit_line
Scope: $scope
Must do:
- Follow project preflight from $PROJECT_PREFLIGHT_DOC before analysis/test/build commands.
- Report concrete findings with file paths and severity.
- Distinguish confirmed facts from assumptions.
$extra_line
Verification:
- $verification
Deliverable:
- Bullet list: findings, risks, recommended fixes.
EOF
    ;;
  implementation)
    cat <<EOF
Task: $task in $repo_root.
$protocol_unit_line
Scope: $scope
Constraints:
- Follow project preflight from $PROJECT_PREFLIGHT_DOC before analyze/test/build.
- Read target files before editing.
- Do not add packages absent in pubspec.yaml.
$(optional_lines "$odoo_line" "$extra_line")
Verification:
- $verification
Deliverable:
- Return the machine-readable summary below.
\`\`\`json
$json_contract
\`\`\`
EOF
    ;;
  decision|complex-decision)
    cat <<EOF
Task: Produce architecture decision for $task in $repo_root.
Mode: analysis-first, then minimal implementation plan.
$protocol_unit_line
Scope: $scope
Must do:
- Follow project preflight from $PROJECT_PREFLIGHT_DOC before analysis/test/build commands.
- Compare at least 2 alternatives.
- Provide pros/cons, risk, migration impact, and rollback strategy.
$extra_line
Verification:
- $verification
Deliverable:
- Decision memo plus concrete next implementation steps.
EOF
    ;;
  patch|small-patch)
    cat <<EOF
Task: Apply a small isolated patch for $task in $repo_root.
$protocol_unit_line
Scope: $scope
Must do:
- Follow project preflight from $PROJECT_PREFLIGHT_DOC before analyze/test/build.
- Keep diff minimal.
- Read target files before editing.
- Do not refactor unrelated code.
$(optional_lines "$odoo_line" "$extra_line")
Verification:
- $verification
Deliverable:
- Return the machine-readable summary below.
\`\`\`json
$json_contract
\`\`\`
EOF
    ;;
  *)
    echo "[render-subagent-prompt] Unknown template: $template" >&2
    usage >&2
    exit 1
    ;;
esac
