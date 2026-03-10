## VIDA Prepare Execution — minimal local artifact bridge for writer readiness.

import std/[json, os, strutils]
import ../core/[utils, config, toon]
import ./route
import ../state/[run_graph, spec_intake, spec_delta, draft_execution_spec, issue_contract, context]
import ../gates/worker_packet

proc writeManifest(path: string, payload: JsonNode): string =
  saveJson(path, payload)
  path

proc prepareExecutionManifestPath*(outputDir: string): string =
  outputDir / "prepare-execution.json"

proc analysisOutputPath*(outputDir: string): string =
  outputDir / "analysis.output.json"

proc projectPreflightDoc(): string =
  let cfg = loadRawConfig()
  let configured = dottedGetStr(cfg, "project_bootstrap.project_operations_doc")
  if configured.len > 0: configured else: route.DefaultProjectPreflightDoc

proc workerMachineReadableContract(): string =
  pretty(route.WorkerMachineReadableTemplate)

proc writerPromptText*(originalPrompt, writerTaskClass: string, issueContractPayload: JsonNode): string =
  let blockingQuestion =
    "What is the minimal implementation and verification change-set that satisfies the normalized issue contract?"
  let normalizedContract =
    if issueContractPayload.isNil or issueContractPayload.kind != JObject: newJObject()
    else: issueContractPayload
  let lines = @[
    "Runtime Role Packet:",
    "- worker_lane_confirmed: true",
    "- worker_role: worker",
    "- orchestrator_entry_fallback: vida/config/instructions/agent-definitions.orchestrator-entry.md",
    "- worker_entry: " & $worker_packet.WorkerEntryDoc,
    "- worker_thinking: " & $worker_packet.WorkerThinkingDoc,
    "- impact_tail_policy: required_for_non_stc",
    "- impact_analysis_scope: bounded_to_assigned_scope",
    "Worker Entry Contract:",
    "- You are a bounded worker, not the orchestrator.",
    "- Follow " & $worker_packet.WorkerEntryDoc & " as the worker-level entry contract.",
    "- Follow " & $worker_packet.WorkerThinkingDoc & " as the worker thinking subset.",
    "- Do not bootstrap repository-wide orchestration policy.",
    "- Stay inside the provided scope and return evidence in the requested format.",
    "- Prefer concrete findings over workflow narration.",
    "Blocking Question: " & blockingQuestion,
    "Task: Implement the requested change for " & writerTaskClass & ".",
    "Mode: WRITE within the approved bounded scope.",
    "Scope: original prompt/spec plus the normalized issue contract below.",
    "Constraints:",
    "- Follow project preflight from " & projectPreflightDoc() & " before analyze/test/build commands.",
    "- Use STC by default; use PR-CoT only if a bounded implementation trade-off appears inside scope.",
    "- Read target files before editing and keep the change-set inside the approved scope.",
    "- Treat `issue_contract.proven_scope` as the allowed behavior surface and `issue_contract.scope_out` as forbidden change area.",
    "- `issue_contract.reported_scope` may be broader than the executable fix; do not silently widen into reported-but-unproven scope.",
    "- Do not widen task ownership or rewrite orchestration decisions.",
    "- Machine-readable summaries must always include impact_analysis; for STC keep it minimal, and for PR-CoT or MAR populate bounded downstream effects.",
    "Verification:",
    "- Run only the bounded verification commands needed to prove the changed scope.",
    "Deliverable:",
    "- Return the machine-readable summary below.",
    "```json",
    workerMachineReadableContract(),
    "```",
    "",
    "Original prompt/spec:",
    "<<<PROMPT",
    originalPrompt.strip(),
    "PROMPT",
    "",
    "Normalized issue contract:",
    "```json",
    pretty(normalizedContract),
    "```",
  ]
  lines.join("\n").strip() & "\n"

proc writeWriterIssueContractPrompt*(outputDir, originalPrompt, writerTaskClass: string, issueContractPayload: JsonNode): string =
  let path = outputDir / "writer.issue-contract.prompt.txt"
  writeFile(path, writerPromptText(originalPrompt, writerTaskClass, issueContractPayload))
  path

proc prepareExecutionContextSources*(taskId, taskClass: string, specIntakePayload, specDeltaPayload, issueContractPayload: JsonNode): JsonNode =
  result = newJArray()
  if specIntakePayload.len > 0:
    result.add(%*{
      "source_class": "local_runtime",
      "path": route.specIntakePath(taskId),
      "freshness": "current",
      "provenance": "spec_intake_artifact",
      "role_scope": "orchestrator",
      "notes": taskClass,
    })
  if specDeltaPayload.len > 0:
    result.add(%*{
      "source_class": "local_runtime",
      "path": route.specDeltaPath(taskId),
      "freshness": "current",
      "provenance": "spec_delta_artifact",
      "role_scope": "orchestrator",
      "notes": taskClass,
    })
  if issueContractPayload.len > 0:
    let wvpStatus = dottedGetStr(issueContractPayload, "wvp_status", "unknown")
    let sourceClass = if wvpStatus == "validated": "web_validated" else: "local_runtime"
    let freshness = if sourceClass == "web_validated": "validated" else: "current"
    result.add(%*{
      "source_class": sourceClass,
      "path": route.issueContractPath(taskId),
      "freshness": freshness,
      "provenance": "issue_contract_artifact",
      "role_scope": "orchestrator",
      "notes": taskClass,
    })

proc buildManifest*(taskId, writerTaskClass, promptFile, outputDir: string,
                    workdir: string = ""): tuple[exitCode: int, payload: JsonNode] =
  let (_, writerRoute) = route.routeSnapshot(writerTaskClass, taskId)
  discard route.writeRouteReceipt(taskId, writerTaskClass, writerRoute)

  let analysisPayload = loadJson(analysisOutputPath(outputDir))
  if analysisPayload.len > 0 and dottedGetStr(analysisPayload, "status") == "done":
    discard run_graph.updateNode(taskId, writerTaskClass, "analysis", "completed", writerTaskClass,
      %*{"reason": "local_analysis_manifest"})
  else:
    discard run_graph.updateNode(taskId, writerTaskClass, "analysis", "failed", writerTaskClass,
      %*{"reason": "missing_or_invalid_analysis_manifest"})
  let draftPath = route.draftExecSpecPath(taskId)
  let specIntakePath = route.specIntakePath(taskId)
  let specDeltaPath = route.specDeltaPath(taskId)
  let issueContractPath = route.issueContractPath(taskId)

  var manifest = %*{
    "task_id": taskId,
    "writer_task_class": writerTaskClass,
    "prompt_file": promptFile,
    "effective_prompt_file": promptFile,
    "output_dir": outputDir,
    "workdir": workdir,
    "status": "analysis_failed",
    "writer_authorized": false,
    "issue_contract_path": issueContractPath,
    "spec_intake_path": specIntakePath,
    "spec_delta_path": specDeltaPath,
    "draft_execution_spec_path": draftPath,
    "analysis_manifest": analysisPayload,
    "prompt_resolution": {"writer_packet_mode": "existing_prompt"},
  }

  let draftPayload = loadJson(draftPath)
  let normalizedDraft =
    if draftPayload.len > 0: draft_execution_spec.normalizePayload(taskId, draftPayload)
    else: newJObject()
  let (draftOk, draftErr) =
    if normalizedDraft.len > 0: draft_execution_spec.validatePayload(normalizedDraft, taskId)
    else: (true, "ok")
  manifest["draft_execution_spec"] = normalizedDraft
  if not draftOk:
    manifest["draft_execution_spec_error"] = %draftErr

  let intakePayload = loadJson(specIntakePath)
  let normalizedIntake =
    if intakePayload.len > 0: spec_intake.normalizePayload(taskId, intakePayload)
    else: newJObject()
  var specIntakeOk = true
  var specIntakeErr = ""
  if normalizedIntake.len > 0:
    let (ok, err) = spec_intake.validatePayload(normalizedIntake, taskId)
    specIntakeOk = ok
    specIntakeErr = err
    if not ok:
      specIntakeErr = "spec_intake_" & err
    else:
      let intakeStatus = dottedGetStr(normalizedIntake, "status")
      case intakeStatus
      of "needs_user_negotiation":
        specIntakeOk = false
        specIntakeErr = "spec_intake_needs_user_negotiation"
      of "needs_spec_delta":
        specIntakeOk = false
        specIntakeErr = "spec_intake_needs_spec_delta"
      of "insufficient_intake":
        specIntakeOk = false
        specIntakeErr = "spec_intake_insufficient_intake"
      else:
        discard
  manifest["spec_intake"] = normalizedIntake
  if specIntakeErr.len > 0:
    manifest["spec_intake_error"] = %specIntakeErr

  var normalizedIssue = newJObject()
  let analysisIssueContract = analysisPayload{"issue_contract"}
  if not analysisIssueContract.isNil and analysisIssueContract.kind == JObject:
    normalizedIssue = issue_contract.normalizePayload(taskId, writerTaskClass, writerRoute, analysisIssueContract)
    saveJson(issueContractPath, normalizedIssue)
  else:
    let existingIssue = loadJson(issueContractPath)
    if existingIssue.len > 0:
      normalizedIssue = existingIssue
  manifest["issue_contract"] = normalizedIssue

  var normalizedDelta = newJObject()
  if normalizedIssue.len > 0:
    normalizedDelta = issue_contract.buildSpecDeltaFromIssueContract(normalizedIssue)
    if normalizedDelta.len > 0:
      saveJson(specDeltaPath, normalizedDelta)
  elif loadJson(specDeltaPath).len > 0:
    normalizedDelta = spec_delta.normalizePayload(taskId, loadJson(specDeltaPath))
  manifest["spec_delta"] = normalizedDelta

  var specDeltaOk = true
  var specDeltaErr = ""
  if normalizedDelta.len > 0:
    let (ok, err) = spec_delta.validatePayload(normalizedDelta, taskId)
    specDeltaOk = ok
    specDeltaErr = if ok: "" else: "spec_delta_" & err
  if dottedGetStr(normalizedIssue, "status") == "spec_delta_required":
    specDeltaOk = false
    specDeltaErr = "spec_delta_needs_scp_reconciliation"
  if specDeltaErr.len > 0:
    manifest["spec_delta_error"] = %specDeltaErr

  let contextSources = prepareExecutionContextSources(taskId, writerTaskClass, normalizedIntake, normalizedDelta, normalizedIssue)
  let contextValidation = context.validateSources(contextSources)
  manifest["context_governance"] = contextValidation
  if dottedGetBool(contextValidation, "valid", false):
    discard context.recordEntry(taskId, "prepare_execution", contextValidation{"sources"}, writerTaskClass)

  var issueOk = false
  var issueErr = "missing_issue_contract"
  if normalizedIssue.len > 0:
    let (ok, err) = issue_contract.validatePayload(normalizedIssue, writerRoute)
    issueOk = ok
    issueErr = err
  elif normalizedDraft.len > 0 and draftOk:
    issueErr = "missing_issue_contract"
  manifest["issue_contract"] = normalizedIssue
  if issueErr.len > 0 and not issueOk:
    manifest["issue_contract_error"] = %issueErr

  if issueOk and specIntakeOk and specDeltaOk and draftOk:
    let originalPrompt = if fileExists(promptFile): readFile(promptFile) else: ""
    if worker_packet.validatePacketText(originalPrompt).len == 0:
      manifest["prompt_resolution"] = %*{
        "writer_packet_mode": "existing_worker_packet",
        "writer_packet_file": promptFile,
      }
    else:
      let renderedPrompt = writeWriterIssueContractPrompt(outputDir, originalPrompt, writerTaskClass, normalizedIssue)
      manifest["effective_prompt_file"] = %renderedPrompt
      manifest["prompt_resolution"] = %*{
        "writer_packet_mode": "issue_contract_rendered",
        "writer_packet_file": renderedPrompt,
      }
      manifest["writer_packet_errors"] = %worker_packet.validatePacketText(readFile(renderedPrompt))
    manifest["writer_authorized"] = %true
    manifest["status"] = %"analysis_ready"
    discard run_graph.updateNode(taskId, writerTaskClass, "writer", "ready", writerTaskClass,
      %*{"reason": "analysis_ready", "manifest_path": prepareExecutionManifestPath(outputDir)})
    discard writeManifest(prepareExecutionManifestPath(outputDir), manifest)
    return (0, manifest)

  manifest["status"] = %"issue_contract_blocked"
  discard run_graph.updateNode(taskId, writerTaskClass, "writer", "blocked", writerTaskClass,
    %*{"reason": policyValue(manifest{"issue_contract_error"}, policyValue(manifest{"spec_delta_error"}, policyValue(manifest{"spec_intake_error"}, "issue_contract_blocked")))})
  discard writeManifest(prepareExecutionManifestPath(outputDir), manifest)
  (2, manifest)

proc cmdPrepareExecution*(args: seq[string]): int =
  if args.len < 4:
    echo "Usage: taskflow-v0 prepare-execution <task_id> <writer_task_class> <prompt_file> <output_dir> [workdir]"
    return 2
  let workdir = if args.len > 4: args[4] else: ""
  let (exitCode, rawPayload) = buildManifest(args[0], args[1], args[2], args[3], workdir)
  let payload = normalizeJson(rawPayload)
  if "--json" in args:
    echo pretty(payload)
  else:
    echo renderToon(payload)
  exitCode
