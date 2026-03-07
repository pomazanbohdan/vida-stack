#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PROJECT_PREFLIGHT_DOC="<active project preflight doc from overlay>"
SUBAGENT_ENTRY_DOC="_vida/docs/SUBAGENT-ENTRY.MD"
SUBAGENT_THINKING_DOC="_vida/docs/SUBAGENT-THINKING.MD"

usage() {
  cat <<'EOF'
Usage:
  bash _vida/scripts/render-subagent-prompt.sh <audit|implementation|decision|patch> \
    --task "..." \
    [--protocol-unit "/vida-command#CLx"] \
    --scope "..." \
    --verification "..." \
    [--question "..."] \
    [--extra "..."] \
    [--repo-root PATH]

Examples:
  bash _vida/scripts/render-subagent-prompt.sh audit \
    --task "Audit retry flow" \
    --scope "app/core,app/api" \
    --question "What exact runtime packet markers are missing?" \
    --verification "rg -n \"retry\" app/core app/api"

  bash _vida/scripts/render-subagent-prompt.sh implementation \
    --task "Implement cache invalidation" \
    --scope "app/shared,app/cache" \
    --question "What is the minimal isolated change that fixes the requested cache flow?" \
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
question=""
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
    --question)
      question="${2:-}"
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
  "question_answered": "yes|no",
  "answer": "direct bounded answer",
  "evidence_refs": ["path/to/file:12", "command -> key line"],
  "changed_files": ["path/a", "path/b"],
  "verification_commands": ["exact command"],
  "verification_results": ["command -> pass|fail"],
  "merge_ready": "yes|no",
  "blockers": [],
  "notes": "short note",
  "recommended_next_action": "concise next step",
  "impact_analysis": {
    "affected_scope": ["bounded files/modules"],
    "contract_impact": ["impact or none"],
    "follow_up_actions": ["follow-up or none"],
    "residual_risks": ["risk or none"]
  }
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
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: subagent
- orchestrator_entry_fallback: _vida/docs/ORCHESTRATOR-ENTRY.MD
- worker_entry: $SUBAGENT_ENTRY_DOC
- worker_thinking: $SUBAGENT_THINKING_DOC
- impact_tail_policy: required_for_non_stc
- impact_analysis_scope: bounded_to_assigned_scope
Worker Entry Contract:
- You are a bounded worker, not the orchestrator.
- Follow $SUBAGENT_ENTRY_DOC as the worker-level entry contract.
- Follow $SUBAGENT_THINKING_DOC as the worker thinking subset.
- Do not bootstrap repository-wide orchestration policy.
- Stay inside the provided scope and return evidence in the requested format.
- Prefer concrete findings over workflow narration.
EOF
}

thinking_hint() {
  case "$template" in
    audit|read-only-audit)
      printf '%s\n' "Use STC by default for this scoped audit."
      ;;
    implementation|patch|small-patch)
      printf '%s\n' "Use STC by default; use PR-CoT only if a bounded implementation trade-off appears inside scope."
      ;;
    decision|complex-decision)
      printf '%s\n' "Use PR-CoT for bounded alternatives; use MAR only if this turns into a root-cause investigation."
      ;;
  esac
}

blocking_question_line() {
  if [[ -n "$question" ]]; then
    printf '%s\n' "Blocking Question: $question"
  else
    printf '%s\n' "Blocking Question: [provide one explicit blocking question for this worker lane]"
  fi
}

case "$template" in
  audit|read-only-audit)
    cat <<EOF
$(entry_contract)
Task: $task in $repo_root.
Mode: READ-ONLY (do not modify files).
$protocol_unit_line
Scope: $scope
$(blocking_question_line)
Must do:
- Follow project preflight from $PROJECT_PREFLIGHT_DOC before analysis/test/build commands.
- $(thinking_hint)
- Use host-project quirks only when they are explicitly provided by the task packet.
- Answer the blocking question directly before optional context.
- Report concrete findings with file paths and severity.
- Distinguish confirmed facts from assumptions.
- Return findings directly; do not restate framework orchestration policy.
- Do not perform broad .vida/logs, .vida/state, or .beads sweeps unless the task packet explicitly escalates to them.
- If you use PR-CoT or MAR, end with a bounded impact analysis tail for the assigned scope.
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
$(blocking_question_line)
Constraints:
- Follow project preflight from $PROJECT_PREFLIGHT_DOC before analyze/test/build.
- $(thinking_hint)
- Read target files before editing.
- Do not add dependencies absent from the host project's canonical manifest.
- Do not widen task ownership or rewrite orchestration decisions.
- Answer the blocking question directly before optional context.
- Do not perform broad .vida/logs, .vida/state, or .beads sweeps unless the task packet explicitly escalates to them.
- If you use PR-CoT or MAR, include `impact_analysis` in the machine-readable summary.
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
$(blocking_question_line)
Must do:
- Follow project preflight from $PROJECT_PREFLIGHT_DOC before analysis/test/build commands.
- $(thinking_hint)
- Compare at least 2 alternatives.
- Provide pros/cons, risk, migration impact, and rollback strategy.
- Keep the decision scoped to the requested slice; do not assume orchestrator ownership.
- Answer the blocking question directly before optional context.
- Do not perform broad .vida/logs, .vida/state, or .beads sweeps unless the task packet explicitly escalates to them.
- Include bounded impact analysis for the requested scope before finishing.
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
$(blocking_question_line)
Must do:
- Follow project preflight from $PROJECT_PREFLIGHT_DOC before analyze/test/build.
- $(thinking_hint)
- Keep diff minimal.
- Read target files before editing.
- Do not refactor unrelated code.
- Do not widen scope beyond the isolated patch.
- Answer the blocking question directly before optional context.
- Do not perform broad .vida/logs, .vida/state, or .beads sweeps unless the task packet explicitly escalates to them.
- If you use PR-CoT or MAR, include `impact_analysis` in the machine-readable summary.
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
