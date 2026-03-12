## VIDA Verification Prompt helpers — render read-only verification prompts.

import std/[json, os, strutils]
import ../core/config
import ../core/utils
import ./worker_packet
import ../agents/route

proc projectPreflightDoc(): string =
  let cfg = loadRawConfig()
  let configured = dottedGetStr(cfg, "project_bootstrap.project_operations_doc")
  if configured.len > 0: configured else: route.DefaultProjectPreflightDoc

proc workerMachineReadableContract(): string =
  pretty(route.WorkerMachineReadableTemplate)

proc previewText(text: string, limit: int = 320): string =
  let trimmed = text.strip()
  if trimmed.len <= limit:
    return trimmed
  trimmed[0..<limit].strip() & "..."

proc jsonStringList(node: JsonNode): seq[string] =
  result = @[]
  if node.isNil or node.kind != JArray:
    return
  for item in node:
    let text = policyValue(item, "")
    if text.len > 0:
      result.add(text)

proc verificationPromptText*(originalPrompt, taskClass, verificationTaskClass: string,
                             mergeSummary, postArbitrationMergeSummary: JsonNode,
                             results: JsonNode): string =
  let effectiveSummary =
    if not postArbitrationMergeSummary.isNil and postArbitrationMergeSummary.kind == JObject and postArbitrationMergeSummary.len > 0:
      postArbitrationMergeSummary
    else:
      mergeSummary
  let dominant = effectiveSummary{"dominant_finding"}
  let dominantCluster = dottedGetStr(dominant, "cluster_id")
  let dominantSample = dottedGetStr(dominant, "sample")
  let successAgentBackends =
    block:
      let current = jsonStringList(effectiveSummary{"success_agent_backends"})
      if current.len > 0: current else: jsonStringList(effectiveSummary{"success_subagents"})
  let conflictClusters =
    if effectiveSummary{"open_conflicts"}.kind == JArray:
      effectiveSummary{"open_conflicts"}
    else:
      newJArray()
  let blockingQuestion =
    "Is orchestrator synthesis justified from the current primary ensemble result, and if not, what blocker prevents it?"

  var lines = @[
    "Runtime Role Packet:",
    "- worker_lane_confirmed: true",
    "- worker_role: worker",
    "- orchestrator_entry_fallback: vida/config/instructions/agent-definitions/entry.orchestrator-entry.md",
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
    "Task: Verify orchestrator synthesis readiness for " & taskClass & ".",
    "Mode: READ-ONLY independent verification.",
    "Scope: original prompt, primary ensemble summary, success-lane excerpts, and listed conflicts only.",
    "Constraints:",
    "- Follow project preflight from " & projectPreflightDoc() & " before analysis/test/build commands.",
    "- Use STC by default; use PR-CoT only if a bounded verification trade-off appears inside scope.",
    "- Do not re-solve the task from scratch.",
    "- Do not create or update files under `AGENTS.md`, `docs/framework/history/_vida-source/*`, `docs/*`, `scripts/*`, project root config files, or application source trees; return the JSON summary in stdout only.",
    "- Validate whether the candidate conclusion is sufficiently supported for orchestrator synthesis.",
    "- Highlight contract risks, residual blockers, and whether synthesis is justified.",
    "- Primary task class: " & taskClass,
    "- Verification task class: " & verificationTaskClass,
    "Verification:",
    "- Review the original prompt, the primary ensemble summary, the success-lane excerpts, and any open conflicts.",
    "Deliverable:",
    "- Return the machine-readable summary below.",
    "```json",
    workerMachineReadableContract(),
    "```",
    "",
    "Original prompt:",
    "<<<PROMPT",
    originalPrompt.strip(),
    "PROMPT",
    "",
    "Primary ensemble summary:",
    "- consensus_mode: " & dottedGetStr(effectiveSummary, "consensus_mode", "none"),
    "- decision_ready: " & $dottedGetBool(effectiveSummary, "decision_ready", false),
    "- dominant_cluster_id: " & (if dominantCluster.len > 0: dominantCluster else: "(none)"),
    "- dominant_sample: " & (if dominantSample.len > 0: dominantSample else: "(none)"),
    "- success_agent_backends: " & (if successAgentBackends.len > 0: successAgentBackends.join(", ") else: "(none)"),
    "- open_conflicts: " & $conflictClusters.len,
    "",
    "Success lane excerpts:",
  ]

  if results.kind == JArray:
    for item in results:
      if dottedGetStr(item, "status") != "success":
        continue
      let outputFile = dottedGetStr(item, "output_file")
      if outputFile.len == 0 or not fileExists(outputFile):
        continue
      let excerpt = previewText(readFile(outputFile), 320)
      if excerpt.len == 0:
        continue
      lines.add("- " & dottedGetStr(item, "agent_backend", "(unknown)") & ": " & excerpt)

  if conflictClusters.len > 0:
    lines.add("")
    lines.add("Open conflicts:")
    for cluster in conflictClusters:
      let clusterId = dottedGetStr(cluster, "cluster_id")
      let sample = dottedGetStr(cluster, "sample")
      let agentBackendList =
        block:
          let current = jsonStringList(cluster{"agent_backends"})
          if current.len > 0: current else: jsonStringList(cluster{"subagents"})
      let agentBackends = if agentBackendList.len > 0: agentBackendList.join(", ") else: "(none)"
      lines.add("- " & (if clusterId.len > 0: clusterId else: "(none)") &
        " | agent_backends=" & agentBackends &
        " | sample=" & (if sample.len > 0: sample else: "(empty)"))

  lines.join("\n").strip() & "\n"

proc cmdVerificationPrompt*(args: seq[string]): int =
  if args.len < 5:
    echo "Usage: taskflow-v0 verification-prompt <original_prompt_file> <task_class> <verification_task_class> <merge_summary_json> <post_arbitration_summary_json> [results_json]"
    return 1
  let originalPrompt = readFile(args[0])
  let mergeSummary = loadJson(args[3], newJObject())
  let postSummary = loadJson(args[4], newJObject())
  let results = if args.len > 5: loadJson(args[5], newJArray()) else: newJArray()
  echo verificationPromptText(originalPrompt, args[1], args[2], mergeSummary, postSummary, results)
  0
