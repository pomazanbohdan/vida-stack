## VIDA Dispatch — CLI command assembly and agent-backend availability.
##
## Closes GAPs D & E from config parameter analysis:
## - GAP D: builds the full CLI command from dispatch config fields
## - GAP E: checks agent-backend availability via detect_command → findExe
##
## Replaces the legacy dispatch helper flow for command assembly,
## environment projection, and availability checks.

import std/[json, os, osproc, strutils, tables]
import ./[utils, config]

# ─────────────────────────── Agent Backend Availability (GAP E) ───────────────────────────

proc isAgentBackendAvailable*(cfg: JsonNode, agentBackendName: string): bool =
  ## Check if an agent backend's CLI command is available.
  ## Replaces the legacy detect-command availability check pattern.
  if not isAgentBackendEnabled(cfg, agentBackendName):
    return false
  let backendClass = dottedGetStr(cfg,
    "agent_system.subagents." & agentBackendName & ".subagent_backend_class")
  if backendClass == "internal":
    return true
  let binaryPath = getAgentBackendBinaryPath(cfg, agentBackendName)
  if binaryPath.len > 0:
    return fileExists(binaryPath)
  let detectCmd = getAgentBackendDetectCommand(cfg, agentBackendName)
  if detectCmd.len == 0:
    return false
  return findExe(detectCmd).len > 0

proc getAvailableAgentBackends*(cfg: JsonNode): seq[string] =
  ## Get all available agent-backend names.
  let subagents = getAgentBackends(cfg)
  if subagents.kind != JObject:
    return @[]
  for name, _ in subagents:
    if isAgentBackendAvailable(cfg, name):
      result.add(name)

# ─────────────────────────── Dispatch Command Builder (GAP D) ───────────────────────────

type DispatchCommand* = object
  args*: seq[string]
  env*: OrderedTable[string, string]
  outputMode*: string   # "stdout" or "file"
  outputFile*: string   # if outputMode == "file", the temp file path
  promptMode*: string   # "positional" or "flag"

proc buildDispatchCommand*(
    cfg: JsonNode,
    agentBackendName: string,
    prompt: string,
    model: string = "",
    workdir: string = "",
    writeMode: bool = false,
    webSearch: bool = false,
    outputFile: string = ""
  ): DispatchCommand =
  ## Build the full CLI command line from dispatch config.
  ## Replaces the legacy dispatch command assembly flow.
  let dispatch = getAgentBackendDispatch(cfg, agentBackendName)

  # 1. Command
  let binaryPath = getAgentBackendBinaryPath(cfg, agentBackendName)
  let command = if binaryPath.len > 0: binaryPath else: dottedGetStr(dispatch, "command")
  if command.len == 0:
    raise newException(ValueError, "No dispatch.command for agent backend: " & agentBackendName)
  result.args.add(command)

  # 2. Pre-static args (e.g. [-c, model_reasoning_effort="high", -a, never])
  let preArgs = dispatch{"pre_static_args"}
  if not preArgs.isNil and preArgs.kind == JArray:
    for arg in preArgs:
      let s = arg.getStr().strip()
      if s.len > 0:
        result.args.add(s)

  # 3. Subcommand (e.g. "exec")
  let subcommand = dottedGetStr(dispatch, "subcommand")
  if subcommand.len > 0:
    result.args.add(subcommand)

  # 4. Static args (read-only or write mode)
  if writeMode:
    let writeArgs = dispatch{"write_static_args"}
    if not writeArgs.isNil and writeArgs.kind == JArray:
      for arg in writeArgs:
        let s = arg.getStr().strip()
        if s.len > 0:
          result.args.add(s)
    else:
      # Fallback to regular static_args
      let staticArgs = dispatch{"static_args"}
      if not staticArgs.isNil and staticArgs.kind == JArray:
        for arg in staticArgs:
          let s = arg.getStr().strip()
          if s.len > 0:
            result.args.add(s)
  else:
    let staticArgs = dispatch{"static_args"}
    if not staticArgs.isNil and staticArgs.kind == JArray:
      for arg in staticArgs:
        let s = arg.getStr().strip()
        if s.len > 0:
          result.args.add(s)

  # 5. Web search flag
  if webSearch:
    let webMode = dottedGetStr(dispatch, "web_search_mode")
    if webMode == "flag":
      let webFlag = dottedGetStr(dispatch, "web_search_flag")
      if webFlag.len > 0:
        result.args.add(webFlag)

  # 6. Workdir flag
  let workdirFlag = dottedGetStr(dispatch, "workdir_flag")
  let actualWorkdir = if workdir.len > 0: workdir else: vidaWorkspaceDir()
  if workdirFlag.len > 0:
    result.args.add(workdirFlag)
    result.args.add(actualWorkdir)

  # 7. Model flag
  let modelFlag = dottedGetStr(dispatch, "model_flag")
  let actualModel = if model.len > 0: model
    else: dottedGetStr(cfg, "agent_system.subagents." & agentBackendName & ".default_model")
  if modelFlag.len > 0 and actualModel.len > 0:
    result.args.add(modelFlag)
    result.args.add(actualModel)

  # 8. Output mode
  result.outputMode = dottedGetStr(dispatch, "output_mode", "stdout")
  if result.outputMode == "file":
    let outFlag = dottedGetStr(dispatch, "output_flag")
    result.outputFile = if outputFile.len > 0: outputFile
      else: getTempDir() / "vida-dispatch-" & agentBackendName & ".out"
    if outFlag.len > 0:
      result.args.add(outFlag)
      result.args.add(result.outputFile)

  # 9. Prompt (positional or flag)
  result.promptMode = dottedGetStr(dispatch, "prompt_mode", "positional")
  if result.promptMode == "flag":
    let promptFlag = dottedGetStr(dispatch, "prompt_flag")
    if promptFlag.len > 0:
      result.args.add(promptFlag)
      result.args.add(prompt)
  else:
    # Positional — prompt goes last
    result.args.add(prompt)

  # 10. Environment variables
  result.env = initOrderedTable[string, string]()
  let rawEnv = dispatch{"env"}
  if not rawEnv.isNil and rawEnv.kind == JObject:
    for key, val in rawEnv:
      result.env[key] = val.getStr()

# ─────────────────────────── Timeout Helpers ───────────────────────────

proc getTimeout*(cfg: JsonNode, agentBackendName: string,
                 key: string, default: int): int =
  ## Get a timeout value from dispatch config with a floor of 5 seconds.
  ## Replaces the legacy dispatch timeout helper.
  let dispatch = getAgentBackendDispatch(cfg, agentBackendName)
  max(5, dottedGetInt(dispatch, key, default))

proc startupTimeout*(cfg: JsonNode, agentBackendName: string): int =
  getTimeout(cfg, agentBackendName, "startup_timeout_seconds", 60)

proc noOutputTimeout*(cfg: JsonNode, agentBackendName: string): int =
  getTimeout(cfg, agentBackendName, "no_output_timeout_seconds", 180)

proc progressIdleTimeout*(cfg: JsonNode, agentBackendName: string): int =
  getTimeout(cfg, agentBackendName, "progress_idle_timeout_seconds", 120)

proc maxRuntimeExtension*(cfg: JsonNode, agentBackendName: string): int =
  getTimeout(cfg, agentBackendName, "max_runtime_extension_seconds", 90)

# ─────────────────────────── Effective Config (Merged) ───────────────────────────

proc effectiveRuntimeSeconds*(cfg: JsonNode, taskClass, agentBackendName: string): int =
  ## Get the effective max runtime — max of route-level and agent-backend level.
  ## Replaces the legacy effective runtime limit helper.
  let routeLimit = getRouteMaxRuntimeSeconds(cfg, taskClass)
  let subagentLimit = dottedGetInt(cfg,
    "agent_system.subagents." & agentBackendName & ".max_runtime_seconds", 0)
  if routeLimit > 0 and subagentLimit > 0:
    return max(routeLimit, subagentLimit)
  elif routeLimit > 0:
    return routeLimit
  elif subagentLimit > 0:
    return subagentLimit
  else:
    return 420  # default

proc effectiveMinOutputBytes*(cfg: JsonNode, taskClass, agentBackendName: string): int =
  ## Get effective min_output_bytes — max of route and agent-backend levels.
  let profile = getRoutingProfile(cfg, taskClass)
  let routeMin = dottedGetInt(profile, "min_output_bytes", 0)
  let subagentMin = dottedGetInt(cfg,
    "agent_system.subagents." & agentBackendName & ".min_output_bytes", 0)
  max(routeMin, subagentMin)

proc effectiveOrchestrationTier*(cfg: JsonNode, taskClass, agentBackendName: string): string =
  ## Get the effective orchestration tier — route-level override > agent-backend level.
  ## Closes GAP B: merged config overlays.
  let profile = getRoutingProfile(cfg, taskClass)
  let routeOverride = dottedGetStr(profile, "orchestration_tier")
  if routeOverride.len > 0:
    return routeOverride
  return dottedGetStr(cfg,
    "agent_system.subagents." & agentBackendName & ".orchestration_tier", "standard")
