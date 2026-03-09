## VIDA Subagent System — runtime snapshot, detection, scoring, mode computation.
##
## Decomposition of `subagent-system.py` (3567 lines → ~400 lines core).
## Handles subagent detection, scorecard management, lifecycle stages,
## effective mode computation, and runtime snapshots.

import std/[json, os, strutils, tables, times, options]
import ../core/[utils, config, toon]

# ─────────────────────────── Constants ───────────────────────────

const WriteProducingTaskClasses* = [
  "small_patch", "small_patch_write", "ui_patch", "implementation",
]

const DomainTagAliases* = {
  "odoo_api": "api_contract",
  "flutter_ui": "frontend_ui",
  "riverpod_state": "state_management",
}.toTable

# ─────────────────────────── Domain Tag Normalization ───────────────────────────

proc normalizeDomainTag*(tag: string): string =
  let low = tag.strip().toLowerAscii()
  if DomainTagAliases.hasKey(low): DomainTagAliases[low]
  else: low

proc normalizeDomainTags*(tags: seq[string]): seq[string] =
  for tag in tags:
    let norm = normalizeDomainTag(tag)
    if norm.len > 0 and norm notin result:
      result.add(norm)

# ─────────────────────────── Scoring Thresholds ───────────────────────────

proc thresholds*(cfg: JsonNode): JsonNode =
  let scoring = dottedGet(getAgentSystem(cfg), "scoring")
  %*{
    "consecutive_failure_limit": policyInt(scoring{"consecutive_failure_limit"}, 5),
    "promotion_score": policyInt(scoring{"promotion_score"}, 80),
    "demotion_score": policyInt(scoring{"demotion_score"}, 35),
    "probation_success_runs": policyInt(scoring{"probation_success_runs"}, 3),
    "probation_task_runs": policyInt(scoring{"probation_task_runs"}, 1),
    "retirement_failure_limit": policyInt(scoring{"retirement_failure_limit"}, 12),
  }

# ─────────────────────────── Score Defaults ───────────────────────────

proc scoreDefaults*(): JsonNode =
  %*{
    "global": {
      "score": 50, "success_count": 0, "failure_count": 0,
      "consecutive_failures": 0, "state": "normal",
      "useful_progress_count": 0, "chatter_only_count": 0,
      "preamble_only_output_count": 0, "missing_machine_readable_payload_count": 0,
      "low_signal_output_count": 0, "timeout_after_progress_count": 0,
      "startup_timeout_count": 0, "no_output_timeout_count": 0,
      "stalled_after_progress_count": 0, "time_to_first_useful_output_samples": 0,
      "avg_time_to_first_useful_output_ms": 0, "useful_progress_rate": 0,
      "subagent_state": "active", "failure_reason": "",
      "cooldown_until": "", "probe_required": false,
      "last_quota_exhausted_at": "", "recovery_attempt_count": 0,
      "recovery_success_count": 0, "last_recovery_at": "",
      "last_recovery_status": "", "authored_runs_count": 0,
      "authored_verified_pass_count": 0, "authored_verified_fail_count": 0,
      "verifier_runs_count": 0, "verifier_success_count": 0,
      "verifier_catch_count": 0, "lifecycle_stage": "declared",
      "retirement_reason": "",
    },
    "by_task_class": {},
    "by_domain": {},
  }

# ─────────────────────────── Lifecycle Stage ───────────────────────────

proc lifecycleStageFor*(subagentCfg, globalCard, scoringCfg: JsonNode): string =
  let enabled = dottedGetBool(subagentCfg, "enabled", false)
  let available = dottedGetBool(subagentCfg, "available", false)
  let subagentState = dottedGetStr(globalCard, "subagent_state", "active")
  let scoreState = dottedGetStr(globalCard, "state", "normal")
  let successCount = policyInt(globalCard{"success_count"}, 0)
  let failureCount = policyInt(globalCard{"failure_count"}, 0)
  let cooldownUntil = parseUtcTimestamp(dottedGetStr(globalCard, "cooldown_until"))
  let probationSuccessRuns = max(1, policyInt(scoringCfg{"probation_success_runs"}, 3))
  let retirementFailureLimit = max(1, policyInt(scoringCfg{"retirement_failure_limit"}, 12))

  if not enabled or subagentState == "disabled_manual":
    return "retired"
  if failureCount >= retirementFailureLimit and successCount <= 0:
    return "retired"
  if cooldownUntil.isSome and cooldownUntil.get.toTime > getTime():
    return "cooldown"
  if subagentState in ["degraded", "quota_exhausted"] or scoreState == "demoted":
    return "degraded"
  let lastRecoveryStatus = dottedGetStr(globalCard, "last_recovery_status")
  let lastRecoveryAt = dottedGetStr(globalCard, "last_recovery_at")
  if lastRecoveryStatus == "success" and lastRecoveryAt.len > 0:
    return "recovered"
  if scoreState == "preferred" or successCount >= probationSuccessRuns:
    return "promoted"
  let lastProbeAt = dottedGetStr(globalCard, "last_probe_at")
  if lastProbeAt.len > 0:
    return if successCount > 0: "probation" else: "probed"
  if available:
    return "detected"
  return "declared"

# ─────────────────────────── Risk / Review ───────────────────────────

proc inferredRiskClass*(taskClass, writeScope, verificationGate: string): string =
  let scope = policyValue(%writeScope, "none")
  let gate = policyValue(%verificationGate, "subagent_return_contract")
  let task = policyValue(%taskClass, "default")
  if scope in ["orchestrator_native", "external_write", "repo_write"]: return "R3"
  if scope in ["scoped_only", "sandbox", "patch"]: return "R2"
  if gate in ["architectural_review", "targeted_verification"]: return "R1"
  if task == "architecture": return "R1"
  return "R0"

proc targetReviewState*(riskClass: string): string =
  case riskClass.toUpperAscii()
  of "R0": "review_passed"
  of "R1": "policy_gate_required"
  of "R2": "senior_review_required"
  else: "human_gate_required"

proc analysisRequiredFor*(taskClass, writeScope: string): bool =
  taskClass in WriteProducingTaskClasses or writeScope != "none"

proc analysisRouteTaskClassFor*(taskClass, writeScope: string): string =
  if not analysisRequiredFor(taskClass, writeScope): return ""
  if taskClass == "architecture": return "meta_analysis"
  return "analysis"

# ─────────────────────────── Subagent Detection ───────────────────────────

proc detectSubagents*(cfg: JsonNode): JsonNode =
  let subagents = getSubagents(cfg)
  result = newJObject()
  if subagents.isNil or subagents.kind != JObject: return

  for name, subCfg in subagents:
    if subCfg.kind != JObject: continue
    let enabled = dottedGetBool(subCfg, "enabled", false)
    var detectCmd = dottedGetStr(subCfg, "detect_command")
    var available: bool
    var reason: string
    if name == "internal_subagents":
      available = enabled
      reason = "runtime-managed"
    else:
      if detectCmd.len == 0:
        detectCmd = name.replace("_cli", "")
      available = enabled and findExe(detectCmd).len > 0
      reason = "command:" & detectCmd

    result[name] = %*{
      "enabled": enabled, "available": available,
      "subagent_backend_class": dottedGetStr(subCfg, "subagent_backend_class", "external_cli"),
      "role": dottedGetStr(subCfg, "role", "secondary"),
      "orchestration_tier": dottedGetStr(subCfg, "orchestration_tier", "standard"),
      "cost_priority": dottedGetStr(subCfg, "cost_priority", "normal"),
      "detect_command": detectCmd,
      "default_model": subCfg{"default_model"},
      "profiles": %(splitCsv(subCfg{"profiles"})),
      "default_profile": subCfg{"default_profile"},
      "capability_band": %(splitCsv(subCfg{"capability_band"})),
      "write_scope": dottedGetStr(subCfg, "write_scope", "none"),
      "billing_tier": dottedGetStr(subCfg, "billing_tier", "unknown"),
      "budget_cost_units": policyInt(subCfg{"budget_cost_units"}, 0),
      "speed_tier": dottedGetStr(subCfg, "speed_tier", "unknown"),
      "quality_tier": dottedGetStr(subCfg, "quality_tier", "unknown"),
      "specialties": %(splitCsv(subCfg{"specialties"})),
      "reason": reason,
    }

# ─────────────────────────── Effective Mode ───────────────────────────

proc effectiveMode*(cfg: JsonNode, subagents: JsonNode): tuple[mode: string, reasons: seq[string]] =
  let protocolActive = dottedGetBool(cfg, "protocol_activation.agent_system", false)
  if not protocolActive:
    return ("disabled", @["protocol_activation.agent_system=false"])

  let requested = dottedGetStr(cfg, "agent_system.mode", "native")
  let hasInternal = subagents.hasKey("internal_subagents") and
    dottedGetBool(subagents{"internal_subagents"}, "available", false)
  var hasExternal = false
  for name, payload in subagents:
    if name != "internal_subagents" and dottedGetBool(payload, "available", false):
      hasExternal = true; break

  case requested
  of "disabled": return ("disabled", @["requested_mode=disabled"])
  of "native":
    if hasInternal: return ("native", @["requested_mode=native"])
    return ("disabled", @["requested_mode=native", "internal_subagents unavailable"])
  of "hybrid":
    if hasInternal and hasExternal: return ("hybrid", @["requested_mode=hybrid"])
    if hasInternal: return ("native", @["requested_mode=hybrid", "external subagents unavailable -> degrade_to=native"])
    if hasExternal: return ("disabled", @["requested_mode=hybrid", "internal subagents unavailable -> degrade_to=disabled"])
    return ("disabled", @["requested_mode=hybrid", "no subagents available"])
  else: return ("disabled", @["unsupported requested_mode=" & requested])

# ─────────────────────────── Runtime Snapshot ───────────────────────────

proc runtimeSnapshot*(taskId: string = ""): JsonNode =
  let cfg = loadRawConfig()
  let currentSubagents = detectSubagents(cfg)
  let scoringCfg = thresholds(cfg)
  let (mode, reasons) = effectiveMode(cfg, currentSubagents)

  result = %*{
    "generated_at": nowUtc(),
    "config_path": configPath(),
    "protocol_activation": {
      "agent_system": dottedGetBool(cfg, "protocol_activation.agent_system", false),
    },
    "agent_system": {
      "init_on_boot": dottedGetBool(cfg, "agent_system.init_on_boot", false),
      "requested_mode": dottedGetStr(cfg, "agent_system.mode", "native"),
      "effective_mode": mode,
      "state_owner": dottedGetStr(cfg, "agent_system.state_owner", "orchestrator_only"),
      "max_parallel_agents": policyInt(dottedGet(getAgentSystem(cfg), "max_parallel_agents"), 1),
      "scoring": scoringCfg,
      "reasons": reasons,
    },
    "subagents": currentSubagents,
    "task_id": (if taskId.len > 0: %taskId else: newJNull()),
  }

# ─────────────────────────── Budget Policy Summary ───────────────────────────

proc budgetPolicySummary*(taskClass: string = ""): JsonNode =
  let runLogPath = vidaRoot() / ".vida" / "logs" / "subagent-runs.jsonl"
  result = %*{
    "run_count": 0, "cheap_lane_attempted": 0,
    "bridge_fallback_used": 0, "authorized_internal_escalations": 0,
    "internal_escalations_without_receipt": 0,
    "policy_bypass_count": 0, "budget_violation_count": 0,
  }
  if not fileExists(runLogPath): return
  for line in lines(runLogPath):
    if line.strip().len == 0: continue
    try:
      let payload = parseJson(line)
      if payload.kind != JObject or policyValue(payload{"type"}, "") != "subagent_run": continue
      if taskClass.len > 0 and policyValue(payload{"task_class"}, "") != taskClass: continue
      result["run_count"] = %(result["run_count"].getInt() + 1)
      if dottedGetBool(payload, "cheap_lane_attempted", false):
        result["cheap_lane_attempted"] = %(result["cheap_lane_attempted"].getInt() + 1)
      if dottedGetBool(payload, "bridge_fallback_used", false):
        result["bridge_fallback_used"] = %(result["bridge_fallback_used"].getInt() + 1)
      if dottedGetBool(payload, "internal_escalation_used", false):
        result["authorized_internal_escalations"] = %(result["authorized_internal_escalations"].getInt() + 1)
      if dottedGetBool(payload, "policy_bypass", false):
        result["policy_bypass_count"] = %(result["policy_bypass_count"].getInt() + 1)
      if dottedGetBool(payload, "budget_violation", false):
        result["budget_violation_count"] = %(result["budget_violation_count"].getInt() + 1)
    except: discard

# ─────────────────────────── Route Dispatch Path ───────────────────────────

proc routeRequiredDispatchPath*(analysisRequired, localExecutionAllowed,
    externalFirstRequired, bridgeFallback, internalEscalationAllowed: string): seq[string] =
  if analysisRequired == "yes":
    result.add("analysis_external_zero_budget")
    result.add("analysis_receipt")
  elif localExecutionAllowed == "yes":
    result.add("local_or_external_free")
  elif externalFirstRequired == "yes":
    result.add("external_free")
  else:
    result.add("route_selected")
  if bridgeFallback.len > 0:
    result.add("bridge_fallback")
  if internalEscalationAllowed == "yes":
    result.add("internal_escalation")

# ─────────────────────────── Cost Class ───────────────────────────

proc costClassForUnits*(units: int): string =
  if units <= 0: "free"
  elif units <= 2: "cheap"
  elif units <= 6: "paid"
  else: "expensive"

# ─────────────────────────── CLI ───────────────────────────

proc cmdSystem*(args: seq[string]): int =
  if args.len == 0:
    echo """Usage:
  vida-v0 system snapshot [task_id]
  vida-v0 system detect
  vida-v0 system mode
  vida-v0 system budget-summary [task_class]"""
    return 1

  case args[0]
  of "snapshot":
    let taskId = if args.len > 1: args[1] else: ""
    let payload = normalizeJson(runtimeSnapshot(taskId))
    if "--json" in args:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0
  of "detect":
    let cfg = loadRawConfig()
    let payload = normalizeJson(detectSubagents(cfg))
    if "--json" in args:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0
  of "mode":
    let cfg = loadRawConfig()
    let subs = detectSubagents(cfg)
    let (mode, reasons) = effectiveMode(cfg, subs)
    let payload = normalizeJson(%*{"effective_mode": mode, "reasons": reasons})
    if "--json" in args:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0
  of "budget-summary":
    let tc = if args.len > 1: args[1] else: ""
    let payload = normalizeJson(budgetPolicySummary(tc))
    if "--json" in args:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0
  else:
    echo "Unknown system subcommand: " & args[0]
    return 1
