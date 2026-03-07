#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/status-ui.sh"

STRICT=0

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/framework-boundary-check.sh [--strict]

Checks framework/project boundary violations in framework-owned space:
  - legacy project-specific command references from _vida/*
  - hardcoded project planning paths from _vida/scripts/*
  - L0 orchestrator identity leakage outside AGENTS.md
  - orchestration-only section duplication outside orchestration-protocol.md
  - task-local runtime artifacts leaked into _vida/docs
  - framework runtime-law leakage into project-owned docs/*
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --strict)
      STRICT=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage
      exit 1
      ;;
  esac
done

cd "$ROOT_DIR"

declare -a findings=()

legacy_cmd_hits="$(rg -n --pcre2 '(?<!_vida/)scripts/(br-safe|br-db-quarantine)\.sh' _vida/docs _vida/scripts 2>/dev/null || true)"
if [[ -n "$legacy_cmd_hits" ]]; then
  findings+=("legacy_project_command_refs")
  echo "$legacy_cmd_hits" >&2
fi

hardcoded_plan_hits="$(rg -n --glob '!_vida/docs/vida-artifacts/**' --glob '!_vida/scripts/framework-boundary-check.sh' 'doc/planning/' _vida/scripts _vida/docs 2>/dev/null || true)"
if [[ -n "$hardcoded_plan_hits" ]]; then
  findings+=("hardcoded_project_planning_paths")
  echo "$hardcoded_plan_hits" >&2
fi

identity_leak_hits="$(rg -n --glob '!AGENTS.md' --glob '!_vida/scripts/framework-boundary-check.sh' 'Agent G|Agentic Product Engineer|L0 ORCHESTRATOR CONTRACT' _vida/docs _vida/scripts _vida/commands docs/process 2>/dev/null || true)"
if [[ -n "$identity_leak_hits" ]]; then
  findings+=("l0_identity_leak")
  echo "$identity_leak_hits" >&2
fi

orchestration_section_hits="$(rg -n --glob '!_vida/docs/orchestration-protocol.md' --glob '!_vida/scripts/framework-boundary-check.sh' '^## (Orchestration Lenses|Problem Framing Contract|Dynamic Expert Injection)$' _vida/docs _vida/commands 2>/dev/null || true)"
if [[ -n "$orchestration_section_hits" ]]; then
  findings+=("orchestration_section_duplication")
  echo "$orchestration_section_hits" >&2
fi

task_local_artifact_hits="$(rg -n --files _vida/docs 2>/dev/null | rg '/[A-Za-z0-9._-]*[a-z]+-[0-9][A-Za-z0-9._-]*\\.(json|txt|ya?ml)$' || true)"
if [[ -n "$task_local_artifact_hits" ]]; then
  findings+=("task_local_framework_artifacts")
  echo "$task_local_artifact_hits" >&2
fi

project_runtime_rule_hits="$(rg -n 'read-only lanes must not mutate framework files, project docs/scripts/config roots, or product source trees' docs 2>/dev/null || true)"
if [[ -n "$project_runtime_rule_hits" ]]; then
  findings+=("framework_runtime_rule_leak_to_project_docs")
  echo "$project_runtime_rule_hits" >&2
fi

if [[ "${#findings[@]}" -eq 0 ]]; then
  vida_status_line ok "[boundary-check] ✨ No open issues"
  exit 0
fi

vida_status_line warn "[boundary-check] findings=${findings[*]}"
if [[ "$STRICT" == "1" ]]; then
  exit 3
fi
exit 0
