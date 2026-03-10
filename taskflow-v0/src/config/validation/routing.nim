import std/json
import ../../core/utils
import ../[accessors, schema]

proc validateRoutingDomain*(config: JsonNode, result: var ValidationResult) =
  let agentExtensions = getAgentExtensions(config)
  let roleSelection = dottedGet(agentExtensions, "role_selection", newJObject())

  if roleSelection.kind != JObject and roleSelection.kind != JNull:
    result.errors.add("agent_extensions.role_selection must be a mapping")
    result.valid = false
    return

  if roleSelection.kind != JObject:
    return

  let selectionMode = dottedGetStr(roleSelection, "mode", "fixed")
  if selectionMode notin ["fixed", "auto"]:
    result.errors.add("agent_extensions.role_selection.mode must be fixed or auto; got: " & selectionMode)
    result.valid = false

  let fallbackRole = dottedGetStr(roleSelection, "fallback_role", "orchestrator")
  let enabled = dottedGetBool(agentExtensions, "enabled", false)
  if enabled:
    if fallbackRole notin KnownFrameworkRoles:
      let projectRoles = splitCsv(dottedGet(agentExtensions, "enabled_project_roles"))
      if fallbackRole notin projectRoles:
        result.errors.add("agent_extensions.role_selection.fallback_role contains unknown role: " & fallbackRole)
        result.valid = false
  elif fallbackRole notin KnownFrameworkRoles:
    result.errors.add("agent_extensions.role_selection.fallback_role contains unknown framework role: " & fallbackRole)
    result.valid = false

  let conversationModes = dottedGet(roleSelection, "conversation_modes", newJObject())
  if conversationModes.kind != JObject:
    result.errors.add("agent_extensions.role_selection.conversation_modes must be a mapping")
    result.valid = false
    return

  let enabledProjectRoles = splitCsv(dottedGet(agentExtensions, "enabled_project_roles"))
  for modeId, row in conversationModes:
    if row.kind != JObject:
      result.errors.add("agent_extensions.role_selection.conversation_modes." & modeId & " must be a mapping")
      result.valid = false
      continue
    let roleId = dottedGetStr(row, "role")
    if roleId.len == 0:
      result.errors.add("agent_extensions.role_selection.conversation_modes." & modeId & " missing role")
      result.valid = false
    elif roleId notin KnownFrameworkRoles and roleId notin enabledProjectRoles:
      let suffix = if dottedGetBool(agentExtensions, "enabled", false): "" else: " framework"
      result.errors.add("agent_extensions.role_selection.conversation_modes." & modeId & " contains unknown" & suffix & " role: " & roleId)
      result.valid = false
    let trackedFlow = dottedGetStr(row, "tracked_flow_entry")
    if trackedFlow.len > 0 and trackedFlow notin KnownTrackedFlowEntries:
      result.errors.add("agent_extensions.role_selection.conversation_modes." & modeId & " contains unknown tracked_flow_entry: " & trackedFlow)
      result.valid = false
