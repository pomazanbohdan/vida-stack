## Tests for draft execution spec / spec intake / spec delta helpers

import std/[json, os, unittest]
import ../src/state/draft_execution_spec
import ../src/state/spec_intake
import ../src/state/spec_delta
import ../src/core/utils

suite "spec artifacts":
  let root = "/tmp/vida_scripts_nim_spec_artifacts"
  discard existsOrCreateDir(root)
  putEnv("VIDA_ROOT", root)

  test "draft execution spec validates normalized payload":
    let path = draft_execution_spec.artifactPath("vida-spec-1")
    saveJson(path, %*{
      "task_id": "vida-spec-1",
      "scope_in": ["src/a"],
      "acceptance_checks": ["cargo test"],
      "recommended_next_path": "/vida-form-task",
    })
    check cmdDraftExecutionSpec(@["validate", "vida-spec-1"]) == 0

  test "draft execution spec rejects non canonical next path":
    let path = draft_execution_spec.artifactPath("vida-spec-invalid-next")
    saveJson(path, %*{
      "task_id": "vida-spec-invalid-next",
      "scope_in": ["src/a"],
      "acceptance_checks": ["cargo test"],
      "recommended_next_path": "user_negotiation",
    })
    check cmdDraftExecutionSpec(@["validate", "vida-spec-invalid-next"]) == 2

  test "spec intake validates ready_for_issue_contract":
    let path = spec_intake.artifactPath("vida-spec-2")
    saveJson(path, %*{
      "task_id": "vida-spec-2",
      "intake_class": "issue",
      "problem_statement": "broken flow",
      "requested_outcome": "fix it",
      "proposed_scope_in": ["src/b"],
      "recommended_contract_path": "issue_contract",
      "status": "ready_for_issue_contract",
    })
    check cmdSpecIntake(@["validate", "vida-spec-2"]) == 0

  test "spec intake requires open decisions for negotiation":
    let path = spec_intake.artifactPath("vida-spec-open-decisions")
    saveJson(path, %*{
      "task_id": "vida-spec-open-decisions",
      "intake_class": "user_negotiation",
      "problem_statement": "scope is unclear",
      "requested_outcome": "negotiate accepted scope",
      "recommended_contract_path": "user_negotiation",
      "status": "needs_user_negotiation",
    })
    check cmdSpecIntake(@["validate", "vida-spec-open-decisions"]) == 2

  test "spec intake requires spec delta path for delta status":
    let path = spec_intake.artifactPath("vida-spec-needs-delta")
    saveJson(path, %*{
      "task_id": "vida-spec-needs-delta",
      "intake_class": "mixed",
      "problem_statement": "behavior changed",
      "requested_outcome": "route delta reconciliation",
      "proposed_scope_in": ["settings"],
      "recommended_contract_path": "scp",
      "status": "needs_spec_delta",
    })
    check cmdSpecIntake(@["validate", "vida-spec-needs-delta"]) == 2

  test "spec intake insufficient intake requires gather evidence path":
    let path = spec_intake.artifactPath("vida-spec-insufficient")
    saveJson(path, %*{
      "task_id": "vida-spec-insufficient",
      "intake_class": "mixed",
      "problem_statement": "not enough evidence yet",
      "requested_outcome": "gather more input",
      "recommended_contract_path": "scp",
      "status": "insufficient_intake",
    })
    check cmdSpecIntake(@["validate", "vida-spec-insufficient"]) == 2

  test "spec delta validates delta_ready payload":
    let path = spec_delta.artifactPath("vida-spec-3")
    saveJson(path, %*{
      "task_id": "vida-spec-3",
      "delta_source": "issue_contract",
      "current_contract": "old",
      "proposed_contract": "new",
      "delta_summary": "changed behavior",
      "behavior_change": "yes",
      "reconciliation_targets": ["docs/spec.md"],
      "status": "delta_ready",
    })
    check cmdSpecDelta(@["validate", "vida-spec-3"]) == 0

  test "spec delta requires user confirmation flag":
    let path = spec_delta.artifactPath("vida-spec-confirm")
    saveJson(path, %*{
      "task_id": "vida-spec-confirm",
      "delta_source": "release_signal",
      "current_contract": "old",
      "proposed_contract": "new",
      "delta_summary": "release changes behavior",
      "behavior_change": "user_visible",
      "status": "needs_user_confirmation",
    })
    check cmdSpecDelta(@["validate", "vida-spec-confirm"]) == 2

  test "spec delta not required must not describe delta":
    let path = spec_delta.artifactPath("vida-spec-not-required")
    saveJson(path, %*{
      "task_id": "vida-spec-not-required",
      "delta_source": "research_findings",
      "delta_summary": "should not exist",
      "status": "not_required",
    })
    check cmdSpecDelta(@["validate", "vida-spec-not-required"]) == 2
