import std/[json, tables, os, strutils]
import ../../core/utils
import ../[accessors, bundle_builder, schema]

proc validateAgentExtensionsDomain*(config: JsonNode, result: var ValidationResult) =
  let agentExtensions = getAgentExtensions(config)
  if agentExtensions.kind != JObject:
    return

  let enabled = dottedGetBool(agentExtensions, "enabled", false)
  let mapDoc = dottedGetStr(agentExtensions, "map_doc")
  if mapDoc.len > 0 and not fileExists(resolveConfigRelativePath(mapDoc)):
    result.errors.add("agent_extensions.map_doc does not exist: " & mapDoc)
    result.valid = false

  if not enabled:
    for roleId in splitCsv(dottedGet(agentExtensions, "enabled_framework_roles")):
      if roleId notin KnownFrameworkRoles:
        result.errors.add("agent_extensions.enabled_framework_roles contains unknown role: " & roleId)
        result.valid = false
    for flowId in splitCsv(dottedGet(agentExtensions, "enabled_standard_flow_sets")):
      if flowId notin KnownStandardFlowSets:
        result.errors.add("agent_extensions.enabled_standard_flow_sets contains unknown flow set: " & flowId)
        result.valid = false
    let defaultFlow = dottedGetStr(agentExtensions, "default_flow_set")
    if defaultFlow.len > 0 and defaultFlow notin splitCsv(dottedGet(agentExtensions, "enabled_standard_flow_sets")):
      result.errors.add("agent_extensions.default_flow_set must resolve to one enabled standard or project flow set; got: " & defaultFlow)
      result.valid = false
    return

  let registries = dottedGet(agentExtensions, "registries", newJObject())
  if registries.kind != JObject:
    result.errors.add("agent_extensions.registries must be a mapping when agent_extensions.enabled=true")
    result.valid = false
    return

  let requireRegistryFiles = dottedGetBool(agentExtensions, "validation.require_registry_files", true)
  let requireUniqueIds = dottedGetBool(agentExtensions, "validation.require_unique_ids", true)
  let requireFrameworkRoleCompat = dottedGetBool(agentExtensions, "validation.require_framework_role_compatibility", true)
  let requireSkillRoleCompat = dottedGetBool(agentExtensions, "validation.require_skill_role_compatibility", true)
  let requireProfileResolution = dottedGetBool(agentExtensions, "validation.require_profile_resolution", true)
  let requireFlowResolution = dottedGetBool(agentExtensions, "validation.require_flow_resolution", true)

  proc validateRegistryPath(key: string, validation: var ValidationResult): string =
    let raw = dottedGetStr(registries, key)
    if requireRegistryFiles and raw.len == 0:
      validation.errors.add("agent_extensions.registries." & key & " is required")
      validation.valid = false
    elif requireRegistryFiles and not fileExists(resolveConfigRelativePath(raw)):
      validation.errors.add("agent_extensions.registries." & key & " does not exist: " & raw)
      validation.valid = false
    raw

  let rolesRegistry = loadYamlRegistry(validateRegistryPath("roles", result))
  let skillsRegistry = loadYamlRegistry(validateRegistryPath("skills", result))
  let profilesRegistry = loadYamlRegistry(validateRegistryPath("profiles", result))
  let flowsRegistry = loadYamlRegistry(validateRegistryPath("flows", result))

  var projectRoleIds: seq[string] = @[]
  var projectSkillIds: seq[string] = @[]
  var projectProfileIds: seq[string] = @[]
  var projectFlowIds: seq[string] = @[]
  var projectRoleBase = initTable[string, string]()
  var skillCompatibility = initTable[string, seq[string]]()

  let roleRows = dottedGet(rolesRegistry, "roles", newJArray())
  if roleRows.kind != JArray and dottedGetStr(registries, "roles").len > 0:
    result.errors.add("project roles registry must expose `roles` as a list")
    result.valid = false
  elif roleRows.kind == JArray:
    for row in roleRows:
      if row.kind != JObject:
        result.errors.add("each project role row must be a mapping")
        result.valid = false
        continue
      let roleId = dottedGetStr(row, "role_id")
      let baseRole = dottedGetStr(row, "base_role")
      if roleId.len == 0:
        result.errors.add("project role row missing role_id")
        result.valid = false
      elif requireUniqueIds and roleId in projectRoleIds:
        result.errors.add("duplicate project role id: " & roleId)
        result.valid = false
      else:
        projectRoleIds.add(roleId)
        projectRoleBase[roleId] = baseRole
      if baseRole.len == 0:
        result.errors.add("project role " & roleId & " missing base_role")
        result.valid = false
      elif requireFrameworkRoleCompat and baseRole notin KnownFrameworkRoles:
        result.errors.add("project role " & roleId & " has unknown base_role: " & baseRole)
        result.valid = false

  let skillRows = dottedGet(skillsRegistry, "skills", newJArray())
  if skillRows.kind != JArray and dottedGetStr(registries, "skills").len > 0:
    result.errors.add("project skills registry must expose `skills` as a list")
    result.valid = false
  elif skillRows.kind == JArray:
    for row in skillRows:
      if row.kind != JObject:
        result.errors.add("each project skill row must be a mapping")
        result.valid = false
        continue
      let skillId = dottedGetStr(row, "skill_id")
      if skillId.len == 0:
        result.errors.add("project skill row missing skill_id")
        result.valid = false
      elif requireUniqueIds and skillId in projectSkillIds:
        result.errors.add("duplicate project skill id: " & skillId)
        result.valid = false
      else:
        projectSkillIds.add(skillId)
        skillCompatibility[skillId] = splitCsv(dottedGet(row, "compatible_base_roles"))

  let profileRows = dottedGet(profilesRegistry, "profiles", newJArray())
  if profileRows.kind != JArray and dottedGetStr(registries, "profiles").len > 0:
    result.errors.add("project profiles registry must expose `profiles` as a list")
    result.valid = false
  elif profileRows.kind == JArray:
    for row in profileRows:
      if row.kind != JObject:
        result.errors.add("each project profile row must be a mapping")
        result.valid = false
        continue
      let profileId = dottedGetStr(row, "profile_id")
      let roleRef = dottedGetStr(row, "role_ref")
      if profileId.len == 0:
        result.errors.add("project profile row missing profile_id")
        result.valid = false
      elif requireUniqueIds and profileId in projectProfileIds:
        result.errors.add("duplicate project profile id: " & profileId)
        result.valid = false
      else:
        projectProfileIds.add(profileId)
      if requireProfileResolution:
        if roleRef.len == 0:
          result.errors.add("project profile " & profileId & " missing role_ref")
          result.valid = false
        elif roleRef notin KnownFrameworkRoles and roleRef notin projectRoleIds:
          result.errors.add("project profile " & profileId & " has unknown role_ref: " & roleRef)
          result.valid = false
      if requireSkillRoleCompat:
        let resolvedBaseRole = if roleRef in projectRoleBase: projectRoleBase[roleRef] else: roleRef
        for skillRef in splitCsv(dottedGet(row, "skill_refs")):
          if skillRef.startsWith("shared:"):
            discard
          elif skillRef notin projectSkillIds:
            result.errors.add("project profile " & profileId & " references unknown skill: " & skillRef)
            result.valid = false
          elif skillRef in skillCompatibility and skillCompatibility[skillRef].len > 0 and
              resolvedBaseRole notin skillCompatibility[skillRef]:
            result.errors.add("project profile " & profileId & " attaches skill " & skillRef &
              " incompatible with base role " & resolvedBaseRole)
            result.valid = false

  let flowRows = dottedGet(flowsRegistry, "flow_sets", newJArray())
  if flowRows.kind != JArray and dottedGetStr(registries, "flows").len > 0:
    result.errors.add("project flows registry must expose `flow_sets` as a list")
    result.valid = false
  elif flowRows.kind == JArray:
    for row in flowRows:
      if row.kind != JObject:
        result.errors.add("each project flow row must be a mapping")
        result.valid = false
        continue
      let flowId = dottedGetStr(row, "flow_id")
      if flowId.len == 0:
        result.errors.add("project flow row missing flow_id")
        result.valid = false
      elif requireUniqueIds and flowId in projectFlowIds:
        result.errors.add("duplicate project flow id: " & flowId)
        result.valid = false
      else:
        projectFlowIds.add(flowId)
      if requireFlowResolution:
        for roleRef in splitCsv(dottedGet(row, "role_chain")):
          if roleRef notin KnownFrameworkRoles and roleRef notin projectRoleIds:
            result.errors.add("project flow " & flowId & " references unknown role: " & roleRef)
            result.valid = false

  for roleId in splitCsv(dottedGet(agentExtensions, "enabled_framework_roles")):
    if roleId notin KnownFrameworkRoles:
      result.errors.add("agent_extensions.enabled_framework_roles contains unknown role: " & roleId)
      result.valid = false
  for flowId in splitCsv(dottedGet(agentExtensions, "enabled_standard_flow_sets")):
    if flowId notin KnownStandardFlowSets:
      result.errors.add("agent_extensions.enabled_standard_flow_sets contains unknown flow set: " & flowId)
      result.valid = false
  for roleId in splitCsv(dottedGet(agentExtensions, "enabled_project_roles")):
    if roleId notin projectRoleIds:
      result.errors.add("agent_extensions.enabled_project_roles contains unknown role: " & roleId)
      result.valid = false
  for skillId in splitCsv(dottedGet(agentExtensions, "enabled_project_skills")):
    if skillId notin projectSkillIds:
      result.errors.add("agent_extensions.enabled_project_skills contains unknown skill: " & skillId)
      result.valid = false
  for skillId in splitCsv(dottedGet(agentExtensions, "enabled_shared_skills")):
    if skillId.len == 0:
      result.errors.add("agent_extensions.enabled_shared_skills contains an empty skill ref")
      result.valid = false
  for profileId in splitCsv(dottedGet(agentExtensions, "enabled_project_profiles")):
    if profileId notin projectProfileIds:
      result.errors.add("agent_extensions.enabled_project_profiles contains unknown profile: " & profileId)
      result.valid = false
  for flowId in splitCsv(dottedGet(agentExtensions, "enabled_project_flows")):
    if flowId notin projectFlowIds:
      result.errors.add("agent_extensions.enabled_project_flows contains unknown flow: " & flowId)
      result.valid = false

  let defaultFlow = dottedGetStr(agentExtensions, "default_flow_set")
  let enabledStandardFlows = splitCsv(dottedGet(agentExtensions, "enabled_standard_flow_sets"))
  let enabledProjectFlows = splitCsv(dottedGet(agentExtensions, "enabled_project_flows"))
  if defaultFlow.len > 0 and defaultFlow notin enabledStandardFlows and defaultFlow notin enabledProjectFlows:
    result.errors.add("agent_extensions.default_flow_set must resolve to one enabled standard or project flow set; got: " & defaultFlow)
    result.valid = false
