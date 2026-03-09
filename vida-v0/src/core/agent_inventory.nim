## VIDA Runtime Agent Inventory — derive concrete installed agents from overlay/runtime.
##
## Product law stays in `vida/config/agents/*`.
## Concrete executable inventory is built from `vida.config.yaml` plus runtime detection.

import std/[json, os, strutils]
import ./[config, kernel_config, utils]

proc runtimeAgentType(subagentCfg: JsonNode): string =
  let backend = dottedGetStr(subagentCfg, "subagent_backend_class", "external_cli")
  case backend
  of "internal", "system":
    "system_agent"
  of "human", "human_agent":
    "human_agent"
  of "service", "service_agent":
    "service_agent"
  else:
    "llm_agent"

proc detectOverlaySubagents(cfg: JsonNode): JsonNode =
  let subagents = getSubagents(cfg)
  result = newJObject()
  if subagents.kind != JObject:
    return

  for name, subCfg in subagents:
    if subCfg.kind != JObject:
      continue
    let enabled = dottedGetBool(subCfg, "enabled", false)
    var detectCmd = dottedGetStr(subCfg, "detect_command")
    var available = false
    if name == "internal_subagents":
      available = enabled
    else:
      if detectCmd.len == 0:
        detectCmd = name.replace("_cli", "")
      available = enabled and findExe(detectCmd).len > 0
    result[name] = %*{
      "enabled": enabled,
      "available": available,
      "detect_command": detectCmd,
    }

proc hasAny(values, required: seq[string]): bool =
  if required.len == 0:
    return true
  for value in values:
    if value in required:
      return true
  return false

proc hasAll(values, required: seq[string]): bool =
  if required.len == 0:
    return true
  for item in required:
    if item notin values:
      return false
  return true

proc matchesSelector(selector, traits: JsonNode): bool =
  if selector.kind != JObject:
    return false

  let requireAvailable = dottedGetBool(selector, "require_available", true)
  if requireAvailable and not dottedGetBool(traits, "available", false):
    return false

  let capabilityBand = splitCsv(traits{"capability_band"})
  let writeScopes = @[policyValue(traits{"write_scope"}, "none")]
  let agentTypes = @[policyValue(traits{"agent_type"}, "llm_agent")]
  let tiers = @[policyValue(traits{"orchestration_tier"}, "standard")]
  let specialties = splitCsv(traits{"specialties"})
  let billingTiers = @[policyValue(traits{"billing_tier"}, "unknown")]
  let qualityTiers = @[policyValue(traits{"quality_tier"}, "unknown")]

  if not hasAny(capabilityBand, splitCsv(selector{"capability_band_any"})):
    return false
  if not hasAll(capabilityBand, splitCsv(selector{"capability_band_all"})):
    return false
  if not hasAny(writeScopes, splitCsv(selector{"write_scopes_any"})):
    return false
  if not hasAny(agentTypes, splitCsv(selector{"agent_types_any"})):
    return false
  if not hasAny(tiers, splitCsv(selector{"orchestration_tiers_any"})):
    return false
  if not hasAny(specialties, splitCsv(selector{"specialties_any"})):
    return false
  if not hasAny(billingTiers, splitCsv(selector{"billing_tiers_any"})):
    return false
  if not hasAny(qualityTiers, splitCsv(selector{"quality_tiers_any"})):
    return false

  true

proc resolveWorkflowRoles(traits, groupsDoc: JsonNode): seq[string] =
  let groups = dottedGet(groupsDoc, "groups")
  if groups.kind != JObject:
    return @[]

  for _, payload in groups:
    if payload.kind != JObject:
      continue
    if dottedGetBool(payload, "manual_only", false):
      continue
    let workflowRole = dottedGetStr(payload, "workflow_role")
    if workflowRole.len == 0:
      continue
    if matchesSelector(dottedGet(payload, "selector"), traits) and workflowRole notin result:
      result.add(workflowRole)

proc deriveCapabilities(workflowRoles: seq[string], subCfg: JsonNode): seq[string] =
  for capability in splitCsv(subCfg{"specialties"}):
    if capability notin result:
      result.add(capability)
  for capability in splitCsv(subCfg{"capability_band"}):
    if capability notin result:
      result.add(capability)

  for role in workflowRoles:
    let roleCapability =
      case role
      of "analyst": "analysis"
      of "writer": "implementation"
      of "coach": "review"
      of "verifier": "verification"
      of "approver": "approval"
      of "synthesizer": "synthesis"
      of "orchestrator": "escalation"
      else: ""
    if roleCapability.len > 0 and roleCapability notin result:
      result.add(roleCapability)

proc overlayTraits(name: string, subCfg, detected: JsonNode): JsonNode =
  let enabled = dottedGetBool(subCfg, "enabled", false)
  let available = dottedGetBool(detected, "available", false)
  %*{
    "id": name,
    "enabled": enabled,
    "available": available,
    "agent_type": runtimeAgentType(subCfg),
    "overlay_role": dottedGetStr(subCfg, "role", "secondary"),
    "orchestration_tier": dottedGetStr(subCfg, "orchestration_tier", "standard"),
    "cost_priority": dottedGetStr(subCfg, "cost_priority", "normal"),
    "budget_cost_units": policyInt(subCfg{"budget_cost_units"}, 0),
    "capability_band": splitCsv(subCfg{"capability_band"}),
    "write_scope": dottedGetStr(subCfg, "write_scope", "none"),
    "billing_tier": dottedGetStr(subCfg, "billing_tier", "unknown"),
    "speed_tier": dottedGetStr(subCfg, "speed_tier", "unknown"),
    "quality_tier": dottedGetStr(subCfg, "quality_tier", "unknown"),
    "specialties": splitCsv(subCfg{"specialties"}),
    "max_concurrency": policyInt(subCfg{"max_concurrency"}, 1),
    "priority": 0,
    "max_runtime_seconds": policyInt(subCfg{"max_runtime_seconds"}, 0),
    "min_output_bytes": policyInt(subCfg{"min_output_bytes"}, 0),
  }

proc computePriority(subCfg, assignmentPolicy: JsonNode): int =
  let costPriority = dottedGetStr(subCfg, "cost_priority", "normal")
  let billingTier = dottedGetStr(subCfg, "billing_tier", "unknown")
  let qualityTier = dottedGetStr(subCfg, "quality_tier", "unknown")
  let speedTier = dottedGetStr(subCfg, "speed_tier", "unknown")
  let enabled = dottedGetBool(subCfg, "enabled", false)
  if not enabled:
    return 0

  result = 50
  case costPriority
  of "premium": result += 35
  of "fallback": result += 10
  of "highest": result += 20
  else: discard

  case billingTier
  of "internal": result += 20
  of "low": result += 10
  of "free": result += 5
  else: discard

  case qualityTier
  of "high": result += 15
  of "medium": result += 5
  else: discard

  case speedTier
  of "fast": result += 10
  of "medium": result += 5
  else: discard

  let maxParallel = policyInt(dottedGet(assignmentPolicy, "runtime.max_parallel_agents"), 1)
  if maxParallel > 1:
    result += min(maxParallel, 5)

proc buildAgentEntry(name: string, subCfg, detected, groupsDoc, assignmentPolicy: JsonNode): JsonNode =
  let traits = overlayTraits(name, subCfg, detected)
  let workflowRoles = resolveWorkflowRoles(traits, groupsDoc)
  let capabilities = deriveCapabilities(workflowRoles, subCfg)
  result = normalizeJson(traits)
  result["status"] =
    (if not dottedGetBool(subCfg, "enabled", false): %"disabled"
     elif dottedGetBool(detected, "available", false): %"active"
     else: %"unavailable")
  result["workflow_roles"] = %workflowRoles
  result["capabilities"] = %capabilities
  result["priority"] = %computePriority(subCfg, assignmentPolicy)

proc buildRuntimeAgentInventory*(cfg: JsonNode = loadRawConfig()): JsonNode =
  let groupsDoc = loadKernelArtifact("agents", "agent_groups")
  let assignmentPolicy = loadPolicySpec("assignment_policy")
  let detected = detectOverlaySubagents(cfg)
  let subagents = getSubagents(cfg)
  var agents: seq[JsonNode] = @[]
  if subagents.kind == JObject:
    for name, subCfg in subagents:
      if subCfg.kind != JObject:
        continue
      agents.add(buildAgentEntry(name, subCfg, dottedGet(detected, name, newJObject()), groupsDoc, assignmentPolicy))

  result = %*{
    "artifact_name": "runtime_agent_inventory",
    "artifact_type": "runtime_agent_inventory",
    "source": "vida.config.yaml",
    "generated_at": nowUtc(),
    "agents": agents,
  }
