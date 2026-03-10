import std/json
import ../../core/utils
import ../[accessors, schema]

proc validateFeatureDomains*(config: JsonNode, result: var ValidationResult) =
  let partyChat = getPartyChat(config)
  if partyChat.kind != JObject or partyChat.len == 0:
    return

  let executionMode = dottedGetStr(partyChat, "execution_mode", "multi_agent")
  if executionMode notin ["single_agent", "multi_agent"]:
    result.errors.add("party_chat.execution_mode must be single_agent or multi_agent; got: " & executionMode)
    result.valid = false

  let routingStrategy = dottedGetStr(partyChat, "model_routing_strategy", "by_role")
  if routingStrategy notin ["uniform", "by_role"]:
    result.errors.add("party_chat.model_routing_strategy must be uniform or by_role; got: " & routingStrategy)
    result.valid = false

  let defaultBoard = dottedGetStr(partyChat, "default_board_size", "small")
  if defaultBoard notin ["small", "large"]:
    result.errors.add("party_chat.default_board_size must be small or large; got: " & defaultBoard)
    result.valid = false

  let minExperts = max(1, dottedGetInt(partyChat, "min_experts", 1))
  let maxExperts = max(1, dottedGetInt(partyChat, "max_experts", 1))
  let hardCapAgents = max(1, dottedGetInt(partyChat, "hard_cap_agents", 1))
  if minExperts > maxExperts:
    result.errors.add("party_chat.min_experts must be <= party_chat.max_experts")
    result.valid = false
  if hardCapAgents < 2:
    result.errors.add("party_chat.hard_cap_agents must be >= 2")
    result.valid = false
  elif hardCapAgents - 1 < minExperts:
    result.errors.add("party_chat.hard_cap_agents cannot satisfy party_chat.min_experts with a facilitator slot reserved")
    result.valid = false

  let singleAgent = dottedGet(partyChat, "single_agent", newJObject())
  if executionMode == "single_agent":
    if dottedGetStr(singleAgent, "backend").len == 0:
      result.errors.add("party_chat.single_agent.backend is required when execution_mode=single_agent")
      result.valid = false
    if dottedGetStr(singleAgent, "model").len == 0:
      result.errors.add("party_chat.single_agent.model is required when execution_mode=single_agent")
      result.valid = false

  let roleBindings = dottedGet(partyChat, "role_model_bindings", newJObject())
  if roleBindings.kind != JObject:
    result.errors.add("party_chat.role_model_bindings must be a mapping")
    result.valid = false
    return

  for roleId, binding in roleBindings:
    if roleId notin KnownPartyChatRoles:
      result.errors.add("party_chat.role_model_bindings contains unknown role: " & roleId)
      result.valid = false
      continue
    if binding.kind != JObject:
      result.errors.add("party_chat.role_model_bindings." & roleId & " must be a mapping")
      result.valid = false
      continue
    if dottedGetStr(binding, "backend").len == 0:
      result.errors.add("party_chat.role_model_bindings." & roleId & ".backend is required")
      result.valid = false
    if dottedGetStr(binding, "model").len == 0:
      result.errors.add("party_chat.role_model_bindings." & roleId & ".model is required")
      result.valid = false
