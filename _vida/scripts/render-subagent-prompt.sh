#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PROJECT_PREFLIGHT_DOC="<active project preflight doc from overlay>"
SUBAGENT_ENTRY_DOC="_vida/docs/SUBAGENT-ENTRY.MD"

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/render-subagent-prompt.sh <audit|implementation|decision|patch> \
    --task "..." \
    [--protocol-unit "/vida-command#CLx"] \
    --scope "..." \
    --verification "..." \
    [--extra "..."] \
    [--repo-root PATH]

Examples:
  bash _vida/scripts/render-subagent-prompt.sh audit \
    --task "Audit retry flow" \
    --scope "app/core,app/api" \
    --verification "rg -n \"retry\" app/core app/api"

  bash _vida/scripts/render-subagent-prompt.sh implementation \
    --task "Implement cache invalidation" \
    --scope "app/shared,app/cache" \
    --verification "<project analyze command>" \
    --extra "Keep changes scoped to the requested cache flow."
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

entry_contract() {
  cat <<EOF
Worker Entry Contract:
- You are a bounded worker, not the orchestrator.
- Follow $SUBAGENT_ENTRY_DOC as the worker-level entry contract.
- Do not bootstrap repository-wide orchestration policy.
- Stay inside the provided scope and return evidence in the requested format.
- Prefer concrete findings over workflow narration.
EOF
}

case "$template" in
  audit|read-only-audit)
    cat <<EOF
$(entry_contract)
Task: $task in $repo_root.
Mode: READ-ONLY (do not modify files).
$protocol_unit_line
Scope: $scope
Must do:
- Follow project preflight from $PROJECT_PREFLIGHT_DOC before analysis/test/build commands.
- Use host-project quirks only when they are explicitly provided by the task packet.
- Report concrete findings with file paths and severity.
- Distinguish confirmed facts from assumptions.
- Return findings directly; do not restate framework orchestration policy.
$extra_line
Verification:
- $verification
Deliverable:
- Bullet list: findings, risks, recommended fixes.
EOF
    ;;
  implementation)
    cat <<EOF
$(entry_contract)
Task: $task in $repo_root.
$protocol_unit_line
Scope: $scope
Constraints:
- Follow project preflight from $PROJECT_PREFLIGHT_DOC before analyze/test/build.
- Read target files before editing.
- Do not add dependencies absent from the host project's canonical manifest.
- Do not widen task ownership or rewrite orchestration decisions.
$(optional_lines "" "$extra_line")
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
$(entry_contract)
Task: Produce architecture decision for $task in $repo_root.
Mode: analysis-first, then minimal implementation plan.
$protocol_unit_line
Scope: $scope
Must do:
- Follow project preflight from $PROJECT_PREFLIGHT_DOC before analysis/test/build commands.
- Compare at least 2 alternatives.
- Provide pros/cons, risk, migration impact, and rollback strategy.
- Keep the decision scoped to the requested slice; do not assume orchestrator ownership.
$extra_line
Verification:
- $verification
Deliverable:
- Decision memo plus concrete next implementation steps.
EOF
    ;;
  patch|small-patch)
    cat <<EOF
$(entry_contract)
Task: Apply a small isolated patch for $task in $repo_root.
$protocol_unit_line
Scope: $scope
Must do:
- Follow project preflight from $PROJECT_PREFLIGHT_DOC before analyze/test/build.
- Keep diff minimal.
- Read target files before editing.
- Do not refactor unrelated code.
- Do not widen scope beyond the isolated patch.
$(optional_lines "" "$extra_line")
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
