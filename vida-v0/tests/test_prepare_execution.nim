## Tests for agents/prepare_execution module

import std/[json, os, strutils, unittest]
import ../src/agents/prepare_execution
import ../src/agents/route
import ../src/state/run_graph
import ../src/gates/worker_packet
import ../src/core/utils

suite "prepare execution":
  let root = "/tmp/vida_scripts_nim_prepare_execution"
  discard existsOrCreateDir(root)
  putEnv("VIDA_ROOT", root)

  test "blocks when spec intake requires negotiation":
    let taskId = "unit-task-negotiation"
    let outputDir = root / "prepare-negotiation"
    createDir(outputDir)
    writeFile(outputDir / "analysis.output.json", $(%*{
      "status": "done",
      "question_answered": "yes",
      "recommended_next_action": "proceed_to_writer",
      "issue_contract": {
        "classification": "defect_equivalent",
        "equivalence_assessment": "equivalent_fix",
        "reported_scope": ["settings flow"],
        "proven_scope": ["settings flow"],
        "acceptance_checks": ["check"]
      }
    }))
    saveJson(route.specIntakePath(taskId), %*{
      "task_id": taskId,
      "intake_class": "mixed",
      "problem_statement": "mixed request",
      "requested_outcome": "clarify scope",
      "proposed_scope_in": ["settings flow"],
      "open_decisions": ["confirm desired settings behavior"],
      "recommended_contract_path": "user_negotiation",
      "status": "needs_user_negotiation",
    })
    let promptFile = outputDir / "implementation.prompt.txt"
    writeFile(promptFile, "Implement the reported bugfix scope.\n")
    let (exitCode, payload) = buildManifest(taskId, "implementation", promptFile, outputDir, root)
    check exitCode == 2
    check payload["status"].getStr() == "issue_contract_blocked"
    check payload["spec_intake_error"].getStr() == "spec_intake_missing_open_decisions" or payload["spec_intake_error"].getStr() == "spec_intake_needs_user_negotiation"
    check payload["writer_authorized"].getBool() == false
    let graph = run_graph.statusPayload(taskId)
    check graph["resume_hint"]["next_node"].getStr() == "writer"
    check graph["resume_hint"]["status"].getStr() == "blocked"

  test "blocks and writes spec delta when issue contract requires spec delta":
    let taskId = "unit-task-spec-delta"
    let outputDir = root / "prepare-spec-delta"
    createDir(outputDir)
    writeFile(outputDir / "analysis.output.json", $(%*{
      "status": "done",
      "question_answered": "yes",
      "recommended_next_action": "route_to_spec_delta",
      "issue_contract": {
        "classification": "feature_delta",
        "equivalence_assessment": "spec_delta_required",
        "reported_behavior": "new behavior",
        "expected_behavior": "old behavior",
        "reported_scope": ["drawer first-level module behavior"],
        "proven_scope": ["drawer first-level module behavior"],
        "acceptance_checks": ["spec updated before implementation"],
        "spec_sync_targets": ["docs/specs/ui.md"]
      }
    }))
    let promptFile = outputDir / "implementation.prompt.txt"
    writeFile(promptFile, "Implement the reported bugfix scope.\n")
    let (exitCode, payload) = buildManifest(taskId, "implementation", promptFile, outputDir, root)
    check exitCode == 2
    check payload["status"].getStr() == "issue_contract_blocked"
    check payload["issue_contract"]["status"].getStr() == "spec_delta_required"
    check payload["spec_delta_error"].getStr() == "spec_delta_needs_scp_reconciliation"
    check fileExists(route.specDeltaPath(taskId))

  test "blocks when draft execution spec exists but issue contract is missing":
    let taskId = "unit-task-missing-issue-contract"
    let outputDir = root / "prepare-missing-issue-contract"
    createDir(outputDir)
    writeFile(outputDir / "analysis.output.json", $(%*{
      "status": "done",
      "question_answered": "yes",
      "recommended_next_action": "proceed_to_writer"
    }))
    saveJson(route.draftExecSpecPath(taskId), %*{
      "task_id": taskId,
      "scope_in": ["settings flow"],
      "scope_out": ["drawer behavior"],
      "acceptance_checks": ["settings render correctly"],
      "recommended_next_path": "/vida-form-task",
    })
    let promptFile = outputDir / "implementation.prompt.txt"
    writeFile(promptFile, "Implement the approved execution spec.\n")
    let (exitCode, payload) = buildManifest(taskId, "implementation", promptFile, outputDir, root)
    check exitCode == 2
    check payload["status"].getStr() == "issue_contract_blocked"
    check payload["issue_contract_error"].getStr() == "missing_issue_contract"

  test "authorizes writer when issue contract is writer ready":
    let taskId = "unit-task-ready"
    let outputDir = root / "prepare-ready"
    createDir(outputDir)
    writeFile(outputDir / "analysis.output.json", $(%*{
      "status": "done",
      "question_answered": "yes",
      "recommended_next_action": "proceed_to_writer",
      "issue_contract": {
        "classification": "defect_equivalent",
        "equivalence_assessment": "equivalent_fix",
        "reported_scope": ["api error handling stack"],
        "proven_scope": ["error interceptor stack"],
        "acceptance_checks": ["cargo test"]
      }
    }))
    let promptFile = outputDir / "implementation.prompt.txt"
    writeFile(promptFile, "Implement the reported bugfix scope.\n")
    let (exitCode, payload) = buildManifest(taskId, "implementation", promptFile, outputDir, root)
    check exitCode == 0
    check payload["status"].getStr() == "analysis_ready"
    check payload["writer_authorized"].getBool() == true
    check payload["prompt_resolution"]["writer_packet_mode"].getStr() == "issue_contract_rendered"
    check fileExists(payload["effective_prompt_file"].getStr())
    check worker_packet.validatePacketText(readFile(payload["effective_prompt_file"].getStr())) == newSeq[string]()
    check readFile(payload["effective_prompt_file"].getStr()).contains("Normalized issue contract:")
    check payload["context_governance"]["valid"].getBool() == true
    check payload["context_governance"]["summary"]["by_source_class"]["local_runtime"].getInt() >= 1
    let graph = run_graph.statusPayload(taskId)
    check graph["resume_hint"]["next_node"].getStr() == "writer"
    check graph["resume_hint"]["status"].getStr() == "ready"

  test "keeps existing worker packet when prompt already validates":
    let taskId = "unit-task-existing-packet"
    let outputDir = root / "prepare-existing-packet"
    createDir(outputDir)
    writeFile(outputDir / "analysis.output.json", $(%*{
      "status": "done",
      "question_answered": "yes",
      "recommended_next_action": "proceed_to_writer",
      "issue_contract": {
        "classification": "defect_equivalent",
        "equivalence_assessment": "equivalent_fix",
        "reported_scope": ["api error handling stack"],
        "proven_scope": ["error interceptor stack"],
        "acceptance_checks": ["cargo test"],
        "wvp_status": "validated"
      }
    }))
    let promptFile = outputDir / "implementation.prompt.txt"
    writeFile(promptFile, """
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: subagent
- worker_entry: docs/framework/SUBAGENT-ENTRY.MD
- worker_thinking: docs/framework/SUBAGENT-THINKING.MD
- impact_tail_policy: required_for_non_stc
- impact_analysis_scope: bounded_to_assigned_scope
Task: implement
Scope: docs/framework/history/_vida-source/scripts
Blocking Question: what changed?
Verification:
- python3 -m unittest
Deliverable:
- findings
""")
    let (exitCode, payload) = buildManifest(taskId, "implementation", promptFile, outputDir, root)
    check exitCode == 0
    check payload["prompt_resolution"]["writer_packet_mode"].getStr() == "existing_worker_packet"
    check payload["context_governance"]["summary"]["by_source_class"]["web_validated"].getInt() == 1

  test "blocks writer ready issue contract with unproven in scope symptoms":
    let taskId = "unit-task-unproven-symptoms"
    let outputDir = root / "prepare-unproven-symptoms"
    createDir(outputDir)
    writeFile(outputDir / "analysis.output.json", $(%*{
      "status": "done",
      "question_answered": "yes",
      "recommended_next_action": "proceed_to_writer",
      "issue_contract": {
        "classification": "defect_equivalent",
        "equivalence_assessment": "equivalent_fix",
        "reported_scope": ["drawer behavior", "navigation ownership"],
        "proven_scope": ["drawer behavior"],
        "acceptance_checks": ["cargo test"],
        "symptoms": [
          {
            "id": "SYM-1",
            "summary": "drawer first-level modules expand instead of navigate",
            "evidence_status": "reproduced",
            "disposition": "in_scope"
          },
          {
            "id": "SYM-2",
            "summary": "navigation ownership drift",
            "evidence_status": "unproven",
            "disposition": "in_scope"
          }
        ]
      }
    }))
    let promptFile = outputDir / "implementation.prompt.txt"
    writeFile(promptFile, "Implement the reported bugfix scope.\n")
    let (exitCode, payload) = buildManifest(taskId, "implementation", promptFile, outputDir, root)
    check exitCode == 2
    check payload["status"].getStr() == "issue_contract_blocked"
    check payload["issue_contract_error"].getStr() == "unproven_symptoms:SYM-2"
    check payload["writer_authorized"].getBool() == false

  test "blocks writer ready issue contract with missing proven scope":
    let taskId = "unit-task-missing-proven-scope"
    let outputDir = root / "prepare-missing-proven-scope"
    createDir(outputDir)
    writeFile(outputDir / "analysis.output.json", $(%*{
      "status": "done",
      "question_answered": "yes",
      "recommended_next_action": "gather more evidence",
      "issue_contract": {
        "classification": "defect_equivalent",
        "equivalence_assessment": "equivalent_fix",
        "reported_scope": ["router redirect", "locale remount"],
        "proven_scope": [],
        "acceptance_checks": ["writer must stay inside proven scope"],
        "symptoms": [
          {
            "id": "SYM-1",
            "summary": "router redirect",
            "evidence_status": "reproduced",
            "disposition": "in_scope"
          },
          {
            "id": "SYM-2",
            "summary": "locale remount",
            "evidence_status": "unproven",
            "disposition": "in_scope"
          }
        ]
      }
    }))
    let promptFile = outputDir / "implementation.prompt.txt"
    writeFile(promptFile, "Implement the reported bugfix scope.\n")
    let (exitCode, payload) = buildManifest(taskId, "implementation", promptFile, outputDir, root)
    check exitCode == 2
    check payload["status"].getStr() == "issue_contract_blocked"
    check payload["issue_contract_error"].getStr() == "missing_proven_scope"
    check payload["writer_authorized"].getBool() == false
