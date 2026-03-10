## VIDA Route Resolution — route snapshot, receipt writing, mutation tracking.
##
## Extracted from the legacy dispatch script into a focused route core.
## Builds route snapshots, writes receipts, tracks framework/project mutations,
## and provides route receipt hashing.

import std/[json, os, strutils, sequtils, hashes, times]
import ../core/[assignment_engine, projection_engine, utils, config, toon]
import ./[system, registry]

# ─────────────────────────── Paths ───────────────────────────

proc routeReceiptDir*(): string = vidaRoot() / ".vida" / "logs" / "route-receipts"
proc issueContractDir*(): string = vidaRoot() / ".vida" / "logs" / "issue-contracts"
proc specIntakeDir*(): string = vidaRoot() / ".vida" / "logs" / "spec-intake"
proc specDeltaDir*(): string = vidaRoot() / ".vida" / "logs" / "spec-deltas"
proc draftExecSpecDir*(): string = vidaRoot() / ".vida" / "logs" / "draft-execution-specs"
proc runLogPath*(): string = vidaRoot() / ".vida" / "logs" / "agent-backend-runs.jsonl"

proc routeReceiptPath*(taskId, taskClass: string): string =
  routeReceiptDir() / (safeName(taskId, "task") & "." & safeName(taskClass, "tc") & ".route.json")

proc analysisReceiptPath*(taskId, taskClass: string): string =
  routeReceiptDir() / (safeName(taskId, "task") & "." & safeName(taskClass, "tc") & ".analysis.json")

proc analysisBlockerPath*(taskId, taskClass: string): string =
  routeReceiptDir() / (safeName(taskId, "task") & "." & safeName(taskClass, "tc") & ".analysis-blocker.json")

proc issueContractPath*(taskId: string): string =
  issueContractDir() / (safeName(taskId, "task") & ".json")

proc specIntakePath*(taskId: string): string =
  specIntakeDir() / (safeName(taskId, "task") & ".json")

proc specDeltaPath*(taskId: string): string =
  specDeltaDir() / (safeName(taskId, "task") & ".json")

proc draftExecSpecPath*(taskId: string): string =
  draftExecSpecDir() / (safeName(taskId, "task") & ".json")

proc coachReceiptPath*(taskId, taskClass: string): string =
  routeReceiptDir() / (safeName(taskId, "task") & "." & safeName(taskClass, "tc") & ".coach.json")

proc coachBlockerPath*(taskId, taskClass: string): string =
  routeReceiptDir() / (safeName(taskId, "task") & "." & safeName(taskClass, "tc") & ".coach-blocker.json")

proc internalEscalationReceiptPath*(taskId, taskClass: string): string =
  routeReceiptDir() / (safeName(taskId, "task") & "." & safeName(taskClass, "tc") & ".internal-escalation.json")

proc reworkHandoffPath*(taskId, taskClass: string): string =
  routeReceiptDir() / (safeName(taskId, "task") & "." & safeName(taskClass, "tc") & ".rework-handoff.json")

# ─────────────────────────── Templates ───────────────────────────

const DefaultProjectPreflightDoc* = "docs/process/project-operations.md"

let WorkerMachineReadableTemplate* = %*{
  "status": "done", "question_answered": "yes",
  "answer": "direct bounded answer",
  "evidence_refs": ["path/to/file:12", "command -> key line"],
  "changed_files": ["path/a", "path/b"],
  "verification_commands": ["exact command"],
  "verification_results": ["command -> pass|fail"],
  "merge_ready": "yes", "blockers": [],
  "notes": "short note",
  "recommended_next_action": "concise next step",
  "impact_analysis": {
    "affected_scope": ["bounded files/modules"],
    "contract_impact": ["impact or none"],
    "follow_up_actions": ["follow-up or none"],
    "residual_risks": ["risk or none"],
  },
}

let IssueContractTemplate* = %*{
  "classification": "defect_equivalent|defect_needs_contract_update|feature_delta|as_designed|not_a_bug|insufficient_evidence",
  "equivalence_assessment": "equivalent_fix|spec_delta_required|as_designed|not_a_bug|insufficient_evidence",
  "reported_behavior": "what is happening now",
  "expected_behavior": "what should happen",
  "reported_scope": ["reported symptoms/behavioral surface before proof narrowing"],
  "proven_scope": ["bounded proven behavioral scope the writer may change"],
  "scope_in": ["bounded behavioral scope that may change"],
  "scope_out": ["related areas that must not change"],
  "acceptance_checks": ["direct acceptance checks for the writer"],
  "spec_sync_targets": ["spec/docs targets to update if this path proceeds"],
  "wvp_required": "yes|no",
  "wvp_status": "validated|not_required|conflicting|unknown",
}

# ─────────────────────────── Kernel Assignment Bridge ───────────────────────────

proc verificationTaskClassFor(taskClass, writeScope: string): string =
  let normalizedTask = policyValue(%taskClass, "default")
  let normalizedScope = policyValue(%writeScope, "none")
  if normalizedTask in ["review", "review_ensemble", "verification", "verification_ensemble"]:
    return ""
  if normalizedScope != "none":
    return "review_ensemble"
  case normalizedTask
  of "research", "research_fast", "ui_research", "research_deep", "analysis", "meta_analysis":
    return "verification_ensemble"
  of "architecture", "small_patch", "small_patch_write", "ui_patch", "implementation":
    return "review_ensemble"
  else:
    return ""

proc assignmentContext(snapshot: JsonNode, taskClass: string, externalFirstRequired: bool,
    excluded: seq[string] = @[], maxBudgetUnits: int = -1): JsonNode =
  result = %*{
    "task_class": taskClass,
    "effective_mode": dottedGet(snapshot, "agent_system.effective_mode", %"hybrid"),
    "external_first_required": externalFirstRequired,
    "exclude_agents": excluded,
  }
  if maxBudgetUnits >= 0:
    result["max_budget_units"] = %maxBudgetUnits

proc rootCandidates(payload: JsonNode): seq[JsonNode] =
  let items = payload{"candidates"}
  let selected = policyValue(payload{"selected_agent_backend"}, "")
  var preferred: seq[JsonNode] = @[]
  var remaining: seq[JsonNode] = @[]
  if items.kind == JArray:
    for item in items:
      if item.kind == JObject:
        if selected.len > 0 and policyValue(item{"agent_backend"}, "") == selected:
          preferred.add(item)
        else:
          remaining.add(item)
  result = preferred & remaining

proc routeCandidatesFromOverlay(taskClass: string, cfg: JsonNode): seq[JsonNode] =
  let subagents = getAgentBackends(cfg)
  let reg = buildRegistry(cfg)
  if not subagents.isNil and subagents.kind == JObject:
    for name, subCfg in subagents:
      if subCfg.kind != JObject:
        continue
      let compat = compatibilityFor(taskClass, name, reg)
      if compat.compatible:
        result.add(%*{
          "agent_backend": name,
          "compatible": true,
          "billing_tier": dottedGetStr(subCfg, "billing_tier", "unknown"),
          "speed_tier": dottedGetStr(subCfg, "speed_tier", "unknown"),
          "quality_tier": dottedGetStr(subCfg, "quality_tier", "unknown"),
          "write_scope": dottedGetStr(subCfg, "write_scope", "none"),
        })

proc addUniqueAgent(target: var seq[string], agentId: string) =
  let trimmed = agentId.strip()
  if trimmed.len > 0 and trimmed notin target:
    target.add(trimmed)

# ─────────────────────────── Route Receipt Hashing ───────────────────────────

proc routeReceiptPayload*(route: JsonNode): JsonNode =
  %*{
    "task_class": route{"task_class"},
    "selected_agent_backend": route{"selected_agent_backend"},
    "risk_class": route{"risk_class"},
    "coach_required": route{"coach_required"},
    "verification_required": route{"verification_required"},
    "analysis_required": route{"analysis_required"},
    "dispatch_policy": route{"dispatch_policy"},
  }

proc routeReceiptHash*(route: JsonNode): string =
  let payload = routeReceiptPayload(route)
  var h: Hash = 0
  for ch in $payload: h = h !& hash(ch)
  result = toHex(!$h).toLowerAscii()

# ─────────────────────────── Route Snapshot ───────────────────────────

proc routeSnapshot*(taskClass: string, taskId: string = ""): tuple[snapshot: JsonNode, route: JsonNode] =
  ## Build runtime snapshot and resolve route for a task class.
  let snapshot = runtimeSnapshot(taskId)
  let cfg = loadRawConfig()
  let preferredAgentBackends = getRouteAgentBackends(cfg, taskClass)
  let configuredWriteScope = getRouteWriteScope(cfg, taskClass)
  let configuredExternalFirst = isExternalFirstRequired(cfg, taskClass) or
    taskClassBindingBool(taskClass, "external_first_required", false)
  
  var candidates: seq[JsonNode] = @[]
  let selectionCtx = assignmentContext(snapshot, taskClass, configuredExternalFirst)
  let rootSelection =
    if kernelAssignmentReady():
      resolveAssignmentForTaskClass(taskClass, selectionCtx)
    else:
      newJObject()
  let usedRootSelection = dottedGetBool(rootSelection, "ok", false)
  if usedRootSelection:
    candidates = rootCandidates(rootSelection)
  else:
    candidates = routeCandidatesFromOverlay(taskClass, cfg)

  var selectedAgentBackend =
    if usedRootSelection:
      policyValue(rootSelection{"selected_agent_backend"}, "")
    elif candidates.len > 0:
      policyValue(candidates[0]{"agent_backend"}, "")
    else:
      ""
  if not usedRootSelection and preferredAgentBackends.len > 0:
    for preferred in preferredAgentBackends:
      for candidate in candidates:
        if policyValue(candidate{"agent_backend"}, "") == preferred:
          selectedAgentBackend = preferred
          break
      if selectedAgentBackend == preferred:
        break
  var candidateWriteScope = "none"
  for candidate in candidates:
    if policyValue(candidate{"agent_backend"}, "") == selectedAgentBackend:
      candidateWriteScope = policyValue(candidate{"write_scope"}, "none")
      break
  let effectiveWriteScope = if configuredWriteScope.len > 0 and configuredWriteScope != "none": configuredWriteScope else: candidateWriteScope
  let riskClass = inferredRiskClass(taskClass, effectiveWriteScope, "")
  let analysisReq = analysisRequiredFor(taskClass, effectiveWriteScope)
  let localExecutionAllowed = if effectiveWriteScope == "none" and riskClass == "R0": "yes" else: "no"
  let externalFirstRequired = if configuredExternalFirst: "yes" else: (if riskClass in ["R2", "R3"]: "yes" else: "no")
  let analysisTaskClass = analysisRouteTaskClassFor(taskClass, effectiveWriteScope)
  let analysisSelection =
    if kernelAssignmentReady() and analysisTaskClass.len > 0:
      let maxBudget = if analysisReq and dottedGet(snapshot, "agent_system.effective_mode", %"").kind == JString and
          policyValue(dottedGet(snapshot, "agent_system.effective_mode"), "") == "hybrid" and analysisTaskClass.len > 0: 0 else: -1
      resolveAssignmentForTaskClass(
        analysisTaskClass,
        assignmentContext(snapshot, analysisTaskClass, configuredExternalFirst, @[], maxBudget),
      )
    else:
      newJObject()
  let coachSelection =
    if kernelAssignmentReady() and riskClass in ["R2", "R3"]:
      resolveAssignmentForTaskClass(
        "coach",
        assignmentContext(snapshot, "coach", false, (if selectedAgentBackend.len > 0: @[selectedAgentBackend] else: @[])),
      )
    else:
      newJObject()
  let verificationTaskClass = verificationTaskClassFor(taskClass, effectiveWriteScope)
  let verificationLane =
    if verificationTaskClass.len > 0: taskClassLane(verificationTaskClass) else: ""
  let verificationIndependence =
    if verificationLane.len > 0: laneIndependenceClass(verificationLane) else: "none"
  var verificationExcluded: seq[string] = @[]
  addUniqueAgent(verificationExcluded, selectedAgentBackend)
  if verificationIndependence == "not_same_route_chain":
    addUniqueAgent(verificationExcluded, policyValue(analysisSelection{"selected_agent_backend"}, ""))
    addUniqueAgent(verificationExcluded, policyValue(coachSelection{"selected_agent_backend"}, ""))
  let verificationSelection =
    if kernelAssignmentReady() and verificationTaskClass.len > 0 and riskClass != "R0":
      resolveAssignmentForTaskClass(
        verificationTaskClass,
        assignmentContext(snapshot, verificationTaskClass, false, verificationExcluded),
      )
    else:
      newJObject()

  let route = %*{
    "task_class": taskClass,
    "task_id": taskId,
    "selected_agent_backend": selectedAgentBackend,
    "candidates": candidates,
    "risk_class": riskClass,
    "write_scope": effectiveWriteScope,
    "route_lane": taskClassLane(taskClass),
    "assignment_source": (if usedRootSelection: "root_config" else: "legacy_overlay"),
    "inventory_source": (if usedRootSelection: policyValue(rootSelection{"inventory_source"}, "overlay_runtime") else: "legacy_overlay"),
    "review_state": targetReviewState(riskClass),
    "analysis_required": (if analysisReq: "yes" else: "no"),
    "analysis_route_task_class": analysisTaskClass,
    "coach_required": (if riskClass in ["R2", "R3"]: "yes" else: "no"),
    "verification_required": (if riskClass != "R0": "yes" else: "no"),
    "analysis_plan": {
      "required": (if analysisReq: "yes" else: "no"),
      "receipt_required": (if analysisReq: "yes" else: "no"),
      "selected_agent_backend": analysisSelection{"selected_agent_backend"},
      "reason": policyValue(analysisSelection{"reason"}, (if analysisReq: "analysis_phase_required" else: "analysis_not_required")),
    },
    "coach_plan": {
      "required": (if riskClass in ["R2", "R3"]: "yes" else: "no"),
      "route_task_class": "coach",
      "selected_agent_backend": coachSelection{"selected_agent_backend"},
      "selected_agent_backends":
        (if policyValue(coachSelection{"selected_agent_backend"}, "").len > 0:
          %(@[policyValue(coachSelection{"selected_agent_backend"}, "")])
        else:
          %(@[])),
      "reason": policyValue(coachSelection{"reason"}, (if riskClass in ["R2", "R3"]: "no_eligible_coach" else: "coach_not_required")),
    },
    "verification_plan": {
      "required": (if riskClass != "R0": "yes" else: "no"),
      "route_task_class": verificationTaskClass,
      "selected_agent_backend": verificationSelection{"selected_agent_backend"},
      "reason":
        (if riskClass != "R0":
          policyValue(verificationSelection{"reason"}, "no_eligible_verifier")
        else:
          ""),
    },
    "dispatch_policy": {
      "local_execution_allowed": localExecutionAllowed,
      "external_first_required": externalFirstRequired,
      "internal_escalation_allowed": "no",
      "required_dispatch_path": routeRequiredDispatchPath(
        (if analysisReq: "yes" else: "no"),
        localExecutionAllowed,
        externalFirstRequired,
        "", "no"
      ),
    },
    "generated_at": nowUtc(),
  }
  return (snapshot, route)

proc projectRouteSnapshot*(taskClass: string, taskId: string = ""): JsonNode =
  let (_, route) = routeSnapshot(taskClass, taskId)
  if route.kind != JObject or route.len == 0:
    return %*{
      "ok": false,
      "reason": "route_snapshot_unavailable",
      "task_class": taskClass,
      "task_id": taskId,
    }
  projectRoutePayload(route)

# ─────────────────────────── Write Route Receipt ───────────────────────────

proc writeRouteReceipt*(taskId, taskClass: string, route: JsonNode): string =
  let payload = %*{
    "ts": nowUtc(),
    "task_id": taskId,
    "task_class": taskClass,
    "route_receipt": routeReceiptPayload(route),
    "route_receipt_hash": routeReceiptHash(route),
  }
  let path = routeReceiptPath(taskId, taskClass)
  createDir(path.parentDir())
  saveJson(path, payload)
  return path

# ─────────────────────────── Mutation Tracking ───────────────────────────

const FrameworkMutationRoots* = ["AGENTS.md", "_vida"]
const FrameworkMutationIgnoredSegments* = ["__pycache__"]
const ProjectMutationExcludedRoots* = [
  "AGENTS.md", "_vida", ".vida", ".beads", "_temp", ".git",
]
const ProjectMutationIgnoredSegments* = [
  "__pycache__", ".dart_tool", "build", "node_modules",
  ".gradle", ".idea", ".vscode", "Pods", "DerivedData",
  "target", "dist", "coverage", ".pytest_cache", ".mypy_cache",
]

proc mutationSnapshot*(roots: seq[string], excludeSegments: seq[string] = @[]): JsonNode =
  result = newJObject()
  let rootDir = vidaRoot()
  for relRoot in roots:
    let target = rootDir / relRoot
    if fileExists(target):
      let info = getFileInfo(target)
      result[relRoot] = %*{"size": info.size, "mtime": $info.lastWriteTime}
    elif dirExists(target):
      for entry in walkDirRec(target):
        let rel = relativePath(entry, rootDir)
        var skip = false
        for seg in excludeSegments:
          if seg in rel: skip = true; break
        if skip: continue
        if rel.endsWith(".pyc") or rel.endsWith(".log"): continue
        try:
          let info = getFileInfo(entry)
          result[rel] = %*{"size": info.size, "mtime": $info.lastWriteTime}
        except: discard

# ─────────────────────────── Ensemble ───────────────────────────

proc ensembleAgentBackends*(route: JsonNode): seq[string] =
  var seen: seq[string] = @[]
  let fanout = route{"fanout_subagents"}
  if not fanout.isNil and fanout.kind == JArray:
    for item in fanout:
      let name = item.getStr()
      if name.len > 0 and name notin seen:
        seen.add(name)
  let primary = policyValue(route{"selected_agent_backend"}, "")
  if primary.len > 0 and primary notin seen:
    seen.insert(primary, 0)
  return seen

# ─────────────────────────── CLI ───────────────────────────

proc cmdRoute*(args: seq[string]): int =
  if args.len == 0:
    echo """Usage:
  vida-v0 route snapshot <task_class> [task_id]
  vida-v0 route receipt <task_id> <task_class>
  vida-v0 route hash <task_id> <task_class>
  vida-v0 route framework-snapshot
  vida-v0 route project-snapshot"""
    return 1

  case args[0]
  of "snapshot":
    if args.len < 2: echo "Usage: vida-v0 route snapshot <task_class> [task_id]"; return 1
    let taskId = if args.len > 2: args[2] else: ""
    let (snapshot, route) = routeSnapshot(args[1], taskId)
    let payload = normalizeJson(%*{"snapshot": snapshot, "route": route})
    if "--json" in args:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0

  of "receipt":
    if args.len < 3: echo "Usage: vida-v0 route receipt <task_id> <task_class>"; return 1
    let (_, route) = routeSnapshot(args[2], args[1])
    let path = writeRouteReceipt(args[1], args[2], route)
    echo path; return 0

  of "hash":
    if args.len < 3: echo "Usage: vida-v0 route hash <task_id> <task_class>"; return 1
    let (_, route) = routeSnapshot(args[2], args[1])
    echo routeReceiptHash(route); return 0

  of "framework-snapshot":
    let payload = normalizeJson(mutationSnapshot(FrameworkMutationRoots.toSeq, FrameworkMutationIgnoredSegments.toSeq))
    if "--json" in args:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0

  of "project-snapshot":
    # Walk all roots except excluded ones
    var roots: seq[string] = @[]
    for entry in walkDir(vidaRoot()):
      let name = extractFilename(entry.path)
      if name notin ProjectMutationExcludedRoots:
        roots.add(name)
    let payload = normalizeJson(mutationSnapshot(roots, ProjectMutationIgnoredSegments.toSeq))
    if "--json" in args:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0

  else:
    echo "Unknown route subcommand: " & args[0]; return 1
