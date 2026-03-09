## VIDA Assignment Engine — root config-driven lane/agent resolution.
##
## Reads product-law artifacts from `vida/config/{routes,agents,policies}`
## and resolves eligible agents for a given lane or task class.
## Concrete installed agent inventory is derived from `vida.config.yaml`.

import std/[algorithm, json, sequtils]
import ./[agent_inventory, kernel_config, utils]

proc kernelAssignmentReady*(): bool =
  let routeCatalog = loadRouteCatalog()
  let agentRegistry = buildRuntimeAgentInventory()
  routeCatalog.kind == JObject and routeCatalog.len > 0 and
    agentRegistry.kind == JObject and agentRegistry{"agents"}.kind == JArray

proc laneSpec(routeCatalog: JsonNode, lane: string): JsonNode =
  dottedGet(routeCatalog, "routes." & lane, newJObject())

proc taskClassBinding(routeCatalog: JsonNode, taskClass: string): JsonNode =
  dottedGet(routeCatalog, "task_class_bindings." & taskClass, newJObject())

proc taskClassLane*(taskClass: string, laneKind: string = "primary_lane"): string =
  let routeCatalog = loadRouteCatalog()
  let binding = taskClassBinding(routeCatalog, taskClass)
  if binding.kind != JObject or binding.len == 0:
    return ""
  let fallback = policyValue(dottedGet(routeCatalog, "defaults." & laneKind), "")
  policyValue(binding{laneKind}, fallback)

proc taskClassBindingBool*(taskClass, key: string, default: bool = false): bool =
  let routeCatalog = loadRouteCatalog()
  let binding = taskClassBinding(routeCatalog, taskClass)
  if binding.kind != JObject or binding.len == 0:
    return default
  policyBool(binding{key}, default)

proc laneIndependenceClass*(lane: string): string =
  let routeCatalog = loadRouteCatalog()
  policyValue(laneSpec(routeCatalog, lane){"independence_class"}, "none")

proc workflowRoles(agent: JsonNode): seq[string] =
  let roles = splitCsv(agent{"workflow_roles"})
  if roles.len > 0:
    return roles
  return splitCsv(agent{"roles"})

proc capabilities(agent: JsonNode): seq[string] =
  splitCsv(agent{"capabilities"})

proc specialties(agent: JsonNode): seq[string] =
  splitCsv(agent{"specialties"})

proc candidateWriteScope(agent: JsonNode): string =
  policyValue(agent{"write_scope"}, "none")

proc candidateLoad(agentId: string, ctx: JsonNode): int =
  dottedGetInt(ctx, "current_load." & agentId, 0)

proc isSystemAgent(agent: JsonNode): bool =
  policyValue(agent{"agent_type"}, "") == "system_agent"

proc isHumanAgent(agent: JsonNode): bool =
  policyValue(agent{"agent_type"}, "") == "human_agent"

proc isActiveAgent(agent: JsonNode): bool =
  dottedGetBool(agent, "enabled", true) and
    policyValue(agent{"status"}, "active") == "active"

proc effectiveMode(ctx, assignmentPolicy: JsonNode): string =
  let ctxMode = policyValue(ctx{"effective_mode"}, "")
  if ctxMode.len > 0:
    return ctxMode
  return policyValue(dottedGet(assignmentPolicy, "runtime.mode"), "hybrid")

proc excludes(ctx: JsonNode): seq[string] =
  splitCsv(ctx{"exclude_agents"})

proc capabilityBonus(agent: JsonNode, taskClass, requiredCapability: string): int =
  var bonus = 0
  let caps = capabilities(agent)
  let specs = specialties(agent)
  if requiredCapability.len > 0 and requiredCapability in caps:
    bonus += 40
  if taskClass.len > 0 and taskClass in specs:
    bonus += 25
  if taskClass.len > 0 and taskClass in caps:
    bonus += 20
  return bonus

proc preferredTraitBoost(agent, routeSpec: JsonNode): int =
  var bonus = 0
  let preferredAgentTypes = splitCsv(routeSpec{"preferred_agent_types"})
  if preferredAgentTypes.len > 0 and policyValue(agent{"agent_type"}, "") in preferredAgentTypes:
    bonus += 120

  let preferredWriteScopes = splitCsv(routeSpec{"preferred_write_scopes"})
  if preferredWriteScopes.len > 0 and candidateWriteScope(agent) in preferredWriteScopes:
    bonus += 90

  let preferredCapabilities = splitCsv(routeSpec{"preferred_capabilities"})
  if preferredCapabilities.len > 0:
    for capability in capabilities(agent):
      if capability in preferredCapabilities:
        bonus += 80
        break

  let preferredBillingTiers = splitCsv(routeSpec{"preferred_billing_tiers"})
  if preferredBillingTiers.len > 0 and policyValue(agent{"billing_tier"}, "") in preferredBillingTiers:
    bonus += 40

  let preferredCostPriorities = splitCsv(routeSpec{"preferred_cost_priorities"})
  if preferredCostPriorities.len > 0 and policyValue(agent{"cost_priority"}, "") in preferredCostPriorities:
    bonus += 30

  return bonus

proc candidatePayload(
  agent: JsonNode,
  lane, taskClass: string,
  score, load: int,
  selectionStrategy: string,
): JsonNode =
  %*{
    "agent_backend": policyValue(agent{"id"}, ""),
    "compatible": true,
    "lane": lane,
    "task_class": taskClass,
    "agent_type": policyValue(agent{"agent_type"}, ""),
    "workflow_roles": workflowRoles(agent),
    "capabilities": capabilities(agent),
    "write_scope": candidateWriteScope(agent),
    "billing_tier": policyValue(agent{"billing_tier"}, "unknown"),
    "speed_tier": policyValue(agent{"speed_tier"}, "unknown"),
    "quality_tier": policyValue(agent{"quality_tier"}, "unknown"),
    "priority": policyInt(agent{"priority"}, 0),
    "max_concurrency": policyInt(agent{"max_concurrency"}, 1),
    "budget_cost_units": policyInt(agent{"budget_cost_units"}, 0),
    "selection_strategy": selectionStrategy,
    "load": load,
    "effective_score": score,
  }

proc compareCandidates(a, b: JsonNode): int =
  let scoreA = policyInt(a{"effective_score"}, 0)
  let scoreB = policyInt(b{"effective_score"}, 0)
  if scoreA != scoreB:
    return cmp(scoreB, scoreA)
  let priorityA = policyInt(a{"priority"}, 0)
  let priorityB = policyInt(b{"priority"}, 0)
  if priorityA != priorityB:
    return cmp(priorityB, priorityA)
  let loadA = policyInt(a{"load"}, 0)
  let loadB = policyInt(b{"load"}, 0)
  if loadA != loadB:
    return cmp(loadA, loadB)
  return cmp(policyValue(a{"agent_backend"}, ""), policyValue(b{"agent_backend"}, ""))

proc selectCandidate(candidates: seq[JsonNode], selectionStrategy: string, ctx: JsonNode): string =
  if candidates.len == 0:
    return ""
  case selectionStrategy
  of "manual_assignment":
    return ""
  of "pinned_agent":
    let pinned = policyValue(ctx{"pinned_agent"}, "")
    if pinned.len > 0:
      for candidate in candidates:
        if policyValue(candidate{"agent_backend"}, "") == pinned:
          return pinned
    for candidate in candidates:
      if policyValue(candidate{"agent_type"}, "") == "system_agent":
        return policyValue(candidate{"agent_backend"}, "")
    return policyValue(candidates[0]{"agent_backend"}, "")
  else:
    return policyValue(candidates[0]{"agent_backend"}, "")

proc resolveAssignmentForLane*(lane: string, ctx: JsonNode = newJObject()): JsonNode =
  let routeCatalog = loadRouteCatalog()
  let assignmentPolicy = loadPolicySpec("assignment_policy")
  let registry = buildRuntimeAgentInventory()
  let spec = laneSpec(routeCatalog, lane)
  if spec.kind != JObject or spec.len == 0:
    return %*{"ok": false, "lane": lane, "reason": "unknown_lane", "candidates": []}
  let agents = registry{"agents"}
  if agents.kind != JArray:
    return %*{"ok": false, "lane": lane, "reason": "missing_agent_registry", "candidates": []}

  let requiredRoles = splitCsv(spec{"required_roles"})
  let requiredCapability = policyValue(spec{"capability"}, "")
  let assignmentMode = policyValue(spec{"assignment_mode"}, "single")
  let independenceClass = policyValue(spec{"independence_class"}, "none")
  let selectionStrategy =
    policyValue(
      spec{"selection_strategy"},
      policyValue(dottedGet(assignmentPolicy, "defaults." & lane), "priority"),
    )
  let mode = effectiveMode(ctx, assignmentPolicy)
  let excludedAgents = excludes(ctx)
  let taskClass = policyValue(ctx{"task_class"}, "")
  let externalFirstRequired = policyBool(ctx{"external_first_required"}, false)
  let routeExternalFirst = dottedGetBool(spec, "external_first_required", false)
  let requireExternalFirst = externalFirstRequired or routeExternalFirst
  let maxBudgetUnits = dottedGetInt(ctx, "max_budget_units", -1)

  if mode == "disabled":
    return %*{
      "ok": false,
      "lane": lane,
      "reason": "assignment_runtime_disabled",
      "selection_strategy": selectionStrategy,
      "candidates": [],
    }

  var candidates: seq[JsonNode] = @[]
  var externalCandidateExists = false
  for agent in agents:
    if agent.kind != JObject:
      continue
    let agentId = policyValue(agent{"id"}, "")
    if agentId.len == 0 or not isActiveAgent(agent):
      continue
    if mode == "native" and not isSystemAgent(agent):
      continue
    if selectionStrategy == "human_required" and not isHumanAgent(agent):
      continue
    if independenceClass in ["human_independent_required", "service_independent_required"] and not isHumanAgent(agent):
      continue
    if independenceClass != "none" and agentId in excludedAgents:
      continue
    let roles = workflowRoles(agent)
    if requiredRoles.len > 0 and (roles.filterIt(it in requiredRoles)).len == 0:
      continue
    let caps = capabilities(agent)
    if requiredCapability.len > 0 and requiredCapability notin caps:
      continue
    if maxBudgetUnits >= 0 and policyInt(agent{"budget_cost_units"}, 0) > maxBudgetUnits:
      continue

    let load = candidateLoad(agentId, ctx)
    var score = policyInt(agent{"priority"}, 0)
    let fit = capabilityBonus(agent, taskClass, requiredCapability)
    score += preferredTraitBoost(agent, spec)
    case selectionStrategy
    of "least_loaded":
      score = score + fit - (load * 100)
    of "capability_score":
      score = score + (fit * 4) - (load * 10)
    of "strict_independent":
      score = score + (fit * 2) - (load * 20)
    of "first_eligible":
      score = score + fit
    of "human_required":
      score = score + fit
    of "pinned_agent":
      if agentId == policyValue(ctx{"pinned_agent"}, ""):
        score += 1000
    else:
      score = score + fit - (load * 10)

    let candidate = candidatePayload(agent, lane, taskClass, score, load, selectionStrategy)
    if not isSystemAgent(agent):
      externalCandidateExists = true
    candidates.add(candidate)

  if requireExternalFirst and externalCandidateExists:
    for candidate in mitems(candidates):
      if policyValue(candidate{"agent_type"}, "") == "system_agent":
        candidate["effective_score"] = %(policyInt(candidate{"effective_score"}, 0) - 1000)

  candidates.sort(compareCandidates)
  let selected = selectCandidate(candidates, selectionStrategy, ctx)
  let ok = selected.len > 0 and assignmentMode != "manual"
  let reason =
    if assignmentMode == "manual":
      "manual_assignment_required"
    elif selected.len > 0:
      "selected"
    else:
      "no_eligible_agent"

  result = %*{
    "ok": ok,
    "source": "root_config",
    "inventory_source": "overlay_runtime",
    "lane": lane,
    "route_stage": policyValue(spec{"route_stage"}, ""),
    "required_roles": requiredRoles,
    "capability": requiredCapability,
    "assignment_mode": assignmentMode,
    "selection_strategy": selectionStrategy,
    "independence_class": independenceClass,
    "selected_agent_backend": (if selected.len > 0: %selected else: newJNull()),
    "reason": reason,
    "candidates": candidates,
  }

proc resolveAssignmentForTaskClass*(taskClass: string, ctx: JsonNode = newJObject()): JsonNode =
  let lane = taskClassLane(taskClass)
  if lane.len == 0:
    return %*{
      "ok": false,
      "task_class": taskClass,
      "reason": "unbound_task_class",
      "candidates": [],
    }
  var laneCtx = normalizeJson(ctx)
  if laneCtx.kind != JObject:
    laneCtx = newJObject()
  laneCtx["task_class"] = %taskClass
  let payload = resolveAssignmentForLane(lane, laneCtx)
  result = normalizeJson(payload)
  result["task_class"] = %taskClass
