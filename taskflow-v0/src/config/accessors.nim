import std/[json, os]
import ../core/utils
import ./loader

proc getLanguagePolicy*(config: JsonNode): JsonNode = dottedGet(config, "language_policy", newJObject())
proc getProtocolActivation*(config: JsonNode): JsonNode = dottedGet(config, "protocol_activation", newJObject())
proc isProtocolActive*(config: JsonNode, protocol: string): bool = dottedGetBool(config, "protocol_activation." & protocol, false)
proc getAgentSystem*(config: JsonNode): JsonNode = dottedGet(config, "agent_system", newJObject())
proc getAgentBackends*(config: JsonNode): JsonNode = dottedGet(config, "agent_system.subagents", newJObject())
proc getScoring*(config: JsonNode): JsonNode = dottedGet(config, "agent_system.scoring", newJObject())
proc getFrameworkSelfDiagnosis*(config: JsonNode): JsonNode = dottedGet(config, "framework_self_diagnosis", newJObject())
proc getProject*(config: JsonNode): JsonNode = dottedGet(config, "project", newJObject())
proc getProjectBootstrap*(config: JsonNode): JsonNode = dottedGet(config, "project_bootstrap", newJObject())
proc isBootstrapEnabled*(config: JsonNode): bool = dottedGetBool(config, "project_bootstrap.enabled", false)
proc getAutonomousExecution*(config: JsonNode): JsonNode = dottedGet(config, "autonomous_execution", newJObject())
proc getPartyChat*(config: JsonNode): JsonNode = dottedGet(config, "party_chat", newJObject())
proc isNextTaskBoundaryAnalysisEnabled*(config: JsonNode): bool = dottedGetBool(config, "autonomous_execution.next_task_boundary_analysis", false)
proc nextTaskBoundaryReport*(config: JsonNode): string = dottedGetStr(config, "autonomous_execution.next_task_boundary_report", "brief_plan")
proc continueAfterReportsEnabled*(config: JsonNode): bool = dottedGetBool(config, "autonomous_execution.continue_after_reports", false)
proc specReadyAutoDevelopmentEnabled*(config: JsonNode): bool = dottedGetBool(config, "autonomous_execution.spec_ready_auto_development", false)
proc validationReportRequiredBeforeImplementation*(config: JsonNode): bool =
  dottedGetBool(config, "autonomous_execution.validation_report_required_before_implementation", false)
proc resumeAfterValidationGateEnabled*(config: JsonNode): bool =
  dottedGetBool(config, "autonomous_execution.resume_after_validation_gate", false)
proc getRouting*(config: JsonNode): JsonNode = dottedGet(config, "agent_system.routing", newJObject())

proc getRoutingProfile*(config: JsonNode, taskClass: string): JsonNode =
  let routing = getRouting(config)
  if routing.kind == JObject and routing.hasKey(taskClass):
    return routing[taskClass]
  if routing.kind == JObject and routing.hasKey("default"):
    return routing["default"]
  newJObject()

proc getRouteAgentBackends*(config: JsonNode, taskClass: string): seq[string] =
  splitCsv(dottedGet(getRoutingProfile(config, taskClass), "subagents"))
proc getRouteModelOverride*(config: JsonNode, taskClass, agentBackendName: string): string =
  dottedGetStr(getRoutingProfile(config, taskClass), "models." & agentBackendName)
proc getRouteProfileOverride*(config: JsonNode, taskClass, agentBackendName: string): string =
  dottedGetStr(getRoutingProfile(config, taskClass), "profiles." & agentBackendName)
proc isExternalFirstRequired*(config: JsonNode, taskClass: string): bool =
  dottedGetBool(getRoutingProfile(config, taskClass), "external_first_required", false)
proc isAnalysisRequired*(config: JsonNode, taskClass: string): bool =
  dottedGetBool(getRoutingProfile(config, taskClass), "analysis_required", false)
proc isCoachRequired*(config: JsonNode, taskClass: string): bool =
  dottedGetBool(getRoutingProfile(config, taskClass), "coach_required", false)
proc isWebSearchRequired*(config: JsonNode, taskClass: string): bool =
  dottedGetBool(getRoutingProfile(config, taskClass), "web_search_required", false)
proc isIndependentVerificationRequired*(config: JsonNode, taskClass: string): bool =
  dottedGetBool(getRoutingProfile(config, taskClass), "independent_verification_required", false)
proc getRouteWriteScope*(config: JsonNode, taskClass: string): string =
  dottedGetStr(getRoutingProfile(config, taskClass), "write_scope", "none")
proc getRouteDispatchRequired*(config: JsonNode, taskClass: string): string =
  dottedGetStr(getRoutingProfile(config, taskClass), "dispatch_required", "external_first_when_eligible")
proc getRouteBudgetUnits*(config: JsonNode, taskClass: string): int =
  dottedGetInt(getRoutingProfile(config, taskClass), "max_budget_units", 4)
proc getRouteFanoutAgentBackends*(config: JsonNode, taskClass: string): seq[string] =
  splitCsv(dottedGet(getRoutingProfile(config, taskClass), "fanout_subagents"))
proc getRouteMaxCliCalls*(config: JsonNode, taskClass: string): int =
  dottedGetInt(getRoutingProfile(config, taskClass), "max_cli_subagent_calls", 5)
proc getRouteMaxRuntimeSeconds*(config: JsonNode, taskClass: string): int =
  dottedGetInt(getRoutingProfile(config, taskClass), "max_total_runtime_seconds", 420)
proc getAgentBackendDispatch*(config: JsonNode, agentBackendName: string): JsonNode =
  dottedGet(config, "agent_system.subagents." & agentBackendName & ".dispatch", newJObject())
proc getDispatchCommand*(config: JsonNode, agentBackendName: string): string =
  dottedGetStr(getAgentBackendDispatch(config, agentBackendName), "command")
proc getDispatchOutputMode*(config: JsonNode, agentBackendName: string): string =
  dottedGetStr(getAgentBackendDispatch(config, agentBackendName), "output_mode", "stdout")
proc getDispatchPromptMode*(config: JsonNode, agentBackendName: string): string =
  dottedGetStr(getAgentBackendDispatch(config, agentBackendName), "prompt_mode", "positional")
proc getDispatchStartupTimeout*(config: JsonNode, agentBackendName: string): int =
  dottedGetInt(getAgentBackendDispatch(config, agentBackendName), "startup_timeout_seconds", 60)
proc isAgentBackendEnabled*(config: JsonNode, agentBackendName: string): bool =
  dottedGetBool(config, "agent_system.subagents." & agentBackendName & ".enabled", false)
proc getAgentBackendWriteScope*(config: JsonNode, agentBackendName: string): string =
  dottedGetStr(config, "agent_system.subagents." & agentBackendName & ".write_scope", "none")
proc getAgentBackendBillingTier*(config: JsonNode, agentBackendName: string): string =
  dottedGetStr(config, "agent_system.subagents." & agentBackendName & ".billing_tier", "free")
proc getAgentBackendCostUnits*(config: JsonNode, agentBackendName: string): int =
  dottedGetInt(config, "agent_system.subagents." & agentBackendName & ".budget_cost_units", 0)
proc getAgentBackendCapabilityBand*(config: JsonNode, agentBackendName: string): seq[string] =
  splitCsv(dottedGet(config, "agent_system.subagents." & agentBackendName & ".capability_band"))
proc getAgentBackendSpecialties*(config: JsonNode, agentBackendName: string): seq[string] =
  splitCsv(dottedGet(config, "agent_system.subagents." & agentBackendName & ".specialties"))
proc getAgentBackendDetectCommand*(config: JsonNode, agentBackendName: string): string =
  dottedGetStr(config, "agent_system.subagents." & agentBackendName & ".detect_command")

proc getAgentBackendBinaryPath*(config: JsonNode, agentBackendName: string): string =
  let raw = dottedGetStr(config, "agent_system.subagents." & agentBackendName & ".binary_path")
  if raw.len == 0:
    return ""
  if raw.isAbsolute:
    return raw
  vidaRoot() / raw

proc getPackRouterKeywords*(config: JsonNode): JsonNode = dottedGet(config, "pack_router_keywords", newJObject())
proc getAgentExtensions*(config: JsonNode): JsonNode = dottedGet(config, "agent_extensions", newJObject())
proc getAgentRoleSelection*(config: JsonNode): JsonNode = dottedGet(config, "agent_extensions.role_selection", newJObject())
proc getAgentRoleSelectionMode*(config: JsonNode): string = dottedGetStr(config, "agent_extensions.role_selection.mode", "fixed")
proc getConversationModes*(config: JsonNode): JsonNode = dottedGet(config, "agent_extensions.role_selection.conversation_modes", newJObject())

proc agentBackendHasWebSearchWiring*(agentBackendCfg: JsonNode): bool =
  if agentBackendCfg.isNil or agentBackendCfg.kind != JObject:
    return false
  let dispatch = dottedGet(agentBackendCfg, "dispatch")
  if dispatch.kind != JObject:
    return false
  dottedGetStr(dispatch, "web_search_mode").len > 0 or dottedGetStr(dispatch, "web_search_flag").len > 0
