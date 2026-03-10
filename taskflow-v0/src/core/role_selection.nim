## VIDA Role Selection Runtime — compiled agent-extension bundle and auto-role selection.
##
## Consumes project agent-extension config and provides executable role/mode selection
## for bounded conversational stages before tracked pack/taskflow handoff.

import std/[algorithm, json, os, strutils]
import ./[config, toon, utils]

const
  ScopeDiscussionMode* = "scope_discussion"
  PbiDiscussionMode* = "pbi_discussion"

  ScopeDiscussionKeywords = [
    "scope", "scoping", "requirement", "requirements", "acceptance",
    "constraint", "constraints", "clarify", "clarification", "discover",
    "discovery", "spec", "specification", "user story", "ac"
  ]

  PbiDiscussionKeywords = [
    "pbi", "backlog", "priority", "prioritize", "prioritization", "task",
    "ticket", "delivery cut", "estimate", "estimation", "roadmap",
    "decompose", "decomposition", "work pool"
  ]

proc loadRegistry(path: string): JsonNode =
  if path.len == 0:
    return newJObject()
  let resolved = resolveConfigRelativePath(path)
  if not fileExists(resolved):
    return newJObject()
  try:
    return parseYamlSubset(readFile(resolved))
  except:
    return newJObject()

proc normalizeRequestText(request: string): string =
  request.toLowerAscii().replace('\n', ' ').replace('\t', ' ')

proc splitKeywordString(node: JsonNode): seq[string] =
  for raw in splitCsv(node):
    let trimmed = raw.strip().toLowerAscii()
    if trimmed.len > 0:
      result.add(trimmed)

proc requestContainsAny(request: string, keywords: seq[string]): seq[string] =
  for keyword in keywords:
    let normalized = keyword.strip().toLowerAscii()
    if normalized.len > 0 and request.contains(normalized):
      result.add(normalized)

proc appendUnique(target: var seq[string], values: seq[string]) =
  for value in values:
    if value.len > 0 and value notin target:
      target.add(value)

proc packKeywordTerms(cfg: JsonNode, keys: openArray[string]): seq[string] =
  let keywordMap = getPackRouterKeywords(cfg)
  for key in keys:
    appendUnique(result, splitKeywordString(dottedGet(keywordMap, key)))

proc standardModeKeywords(modeId: string, cfg: JsonNode): seq[string] =
  case modeId
  of ScopeDiscussionMode:
    appendUnique(result, @ScopeDiscussionKeywords)
    appendUnique(result, packKeywordTerms(cfg, ["spec"]))
  of PbiDiscussionMode:
    appendUnique(result, @PbiDiscussionKeywords)
    appendUnique(result, packKeywordTerms(cfg, ["pool", "pool_strong", "pool_dependency"]))
  else:
    discard

proc enabledStringList(node: JsonNode): seq[string] =
  for item in splitCsv(node):
    let trimmed = item.strip()
    if trimmed.len > 0:
      result.add(trimmed)

proc registryRows(registry: JsonNode, key: string): seq[JsonNode] =
  let rows = dottedGet(registry, key, newJArray())
  if rows.kind != JArray:
    return @[]
  for row in rows:
    if row.kind == JObject:
      result.add(row)

proc isEnabledId(id: string, enabledIds: seq[string]): bool =
  enabledIds.len == 0 or id in enabledIds

proc buildCompiledAgentExtensionBundle*(cfg: JsonNode = loadRawConfig()): JsonNode =
  let agentExtensions = getAgentExtensions(cfg)
  let enabledProjectRoleIds = enabledStringList(dottedGet(agentExtensions, "enabled_project_roles"))
  let enabledProjectSkillIds = enabledStringList(dottedGet(agentExtensions, "enabled_project_skills"))
  let enabledProjectProfileIds = enabledStringList(dottedGet(agentExtensions, "enabled_project_profiles"))
  let enabledProjectFlowIds = enabledStringList(dottedGet(agentExtensions, "enabled_project_flows"))
  let registries = dottedGet(agentExtensions, "registries", newJObject())

  let rolesRegistry = loadRegistry(dottedGetStr(registries, "roles"))
  let skillsRegistry = loadRegistry(dottedGetStr(registries, "skills"))
  let profilesRegistry = loadRegistry(dottedGetStr(registries, "profiles"))
  let flowsRegistry = loadRegistry(dottedGetStr(registries, "flows"))

  var roleRows: seq[JsonNode] = @[]
  for row in registryRows(rolesRegistry, "roles"):
    let roleId = dottedGetStr(row, "role_id")
    if roleId.len > 0 and isEnabledId(roleId, enabledProjectRoleIds):
      roleRows.add(row)

  var skillRows: seq[JsonNode] = @[]
  for row in registryRows(skillsRegistry, "skills"):
    let skillId = dottedGetStr(row, "skill_id")
    if skillId.len > 0 and isEnabledId(skillId, enabledProjectSkillIds):
      skillRows.add(row)

  var profileRows: seq[JsonNode] = @[]
  for row in registryRows(profilesRegistry, "profiles"):
    let profileId = dottedGetStr(row, "profile_id")
    if profileId.len > 0 and isEnabledId(profileId, enabledProjectProfileIds):
      profileRows.add(row)

  var flowRows: seq[JsonNode] = @[]
  for row in registryRows(flowsRegistry, "flow_sets"):
    let flowId = dottedGetStr(row, "flow_id")
    if flowId.len > 0 and isEnabledId(flowId, enabledProjectFlowIds):
      flowRows.add(row)

  result = %*{
    "ok": true,
    "enabled": dottedGetBool(agentExtensions, "enabled", false),
    "map_doc": dottedGetStr(agentExtensions, "map_doc"),
    "enabled_framework_roles": enabledStringList(dottedGet(agentExtensions, "enabled_framework_roles")),
    "enabled_standard_flow_sets": enabledStringList(dottedGet(agentExtensions, "enabled_standard_flow_sets")),
    "enabled_shared_skills": enabledStringList(dottedGet(agentExtensions, "enabled_shared_skills")),
    "default_flow_set": dottedGetStr(agentExtensions, "default_flow_set"),
    "project_roles": roleRows,
    "project_skills": skillRows,
    "project_profiles": profileRows,
    "project_flows": flowRows,
    "role_selection": getAgentRoleSelection(cfg),
  }

proc roleExistsInBundle(bundle: JsonNode, roleId: string): bool =
  if roleId.len == 0:
    return false
  if roleId in enabledStringList(bundle{"enabled_framework_roles"}):
    return true
  let projectRoles = bundle{"project_roles"}
  if projectRoles.kind == JArray:
    for row in projectRoles:
      if dottedGetStr(row, "role_id") == roleId:
        return true
  return false

proc modeSnapshot(modeId: string, modeCfg: JsonNode, requestText: string, cfg: JsonNode): JsonNode =
  var matchedTerms: seq[string] = @[]
  appendUnique(matchedTerms, requestContainsAny(requestText, standardModeKeywords(modeId, cfg)))
  result = %*{
    "mode_id": modeId,
    "enabled": dottedGetBool(modeCfg, "enabled", true),
    "role": dottedGetStr(modeCfg, "role"),
    "single_task_only": dottedGetBool(modeCfg, "single_task_only", false),
    "tracked_flow_entry": dottedGetStr(modeCfg, "tracked_flow_entry"),
    "allow_freeform_chat": dottedGetBool(modeCfg, "allow_freeform_chat", false),
    "matched_terms": matchedTerms,
    "score": matchedTerms.len,
  }

proc selectAgentRoleForRequest*(request: string, cfg: JsonNode = loadRawConfig()): JsonNode =
  let bundle = buildCompiledAgentExtensionBundle(cfg)
  let roleSelection = bundle{"role_selection"}
  let selectionMode = policyValue(roleSelection{"mode"}, "fixed")
  let configuredFallbackRole = policyValue(roleSelection{"fallback_role"}, "orchestrator")
  let fallbackRole =
    if roleExistsInBundle(bundle, configuredFallbackRole): configuredFallbackRole
    else: "orchestrator"

  let normalizedRequest = normalizeRequestText(request)
  result = %*{
    "ok": true,
    "selection_mode": selectionMode,
    "fallback_role": fallbackRole,
    "request": request,
    "selected_role": fallbackRole,
    "conversational_mode": newJNull(),
    "single_task_only": false,
    "tracked_flow_entry": newJNull(),
    "allow_freeform_chat": false,
    "confidence": "fallback",
    "matched_terms": [],
    "compiled_bundle": bundle,
  }

  if selectionMode != "auto":
    result["reason"] = %"fixed_mode"
    return

  let conversationModes = dottedGet(roleSelection, "conversation_modes", newJObject())
  if conversationModes.kind != JObject or normalizedRequest.len == 0:
    result["reason"] = %"auto_no_modes_or_empty_request"
    return

  var candidates: seq[JsonNode] = @[]
  for modeId, modeCfg in conversationModes:
    if modeCfg.kind != JObject:
      continue
    let snapshot = modeSnapshot(modeId, modeCfg, normalizedRequest, cfg)
    if dottedGetBool(snapshot, "enabled", true):
      candidates.add(snapshot)

  if candidates.len == 0:
    result["reason"] = %"auto_no_enabled_modes"
    return

  candidates.sort(proc(a, b: JsonNode): int =
    let scoreCmp = cmp(policyInt(b{"score"}, 0), policyInt(a{"score"}, 0))
    if scoreCmp != 0: return scoreCmp
    cmp(policyValue(a{"mode_id"}, ""), policyValue(b{"mode_id"}, ""))
  )

  let selected = candidates[0]
  if policyInt(selected{"score"}, 0) <= 0:
    result["reason"] = %"auto_no_keyword_match"
    return

  let selectedRole = policyValue(selected{"role"}, fallbackRole)
  if not roleExistsInBundle(bundle, selectedRole):
    result["reason"] = %"auto_selected_unknown_role"
    return

  result["selected_role"] = %selectedRole
  result["conversational_mode"] = selected{"mode_id"}
  result["single_task_only"] = selected{"single_task_only"}
  result["tracked_flow_entry"] = selected{"tracked_flow_entry"}
  result["allow_freeform_chat"] = selected{"allow_freeform_chat"}
  result["matched_terms"] = selected{"matched_terms"}
  let score = policyInt(selected{"score"}, 0)
  result["confidence"] =
    (if score >= 3: %"high"
     elif score >= 2: %"medium"
     else: %"low")
  result["reason"] = %"auto_keyword_match"

proc cmdRoleSelection*(args: seq[string]): int =
  if args.len == 0:
    echo """Usage:
  taskflow-v0 role-select bundle [--json]
  taskflow-v0 role-select request <text> [--json]"""
    return 1

  let asJson = "--json" in args
  case args[0]
  of "bundle":
    let payload = normalizeJson(buildCompiledAgentExtensionBundle())
    if asJson: echo pretty(payload) else: echo renderToon(payload)
    return 0
  of "request":
    if args.len < 2:
      echo "Usage: taskflow-v0 role-select request <text> [--json]"
      return 1
    let payload = normalizeJson(selectAgentRoleForRequest(args[1]))
    if asJson: echo pretty(payload) else: echo renderToon(payload)
    return 0
  else:
    echo "Unknown role-select subcommand: " & args[0]
    return 1
