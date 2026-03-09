## VIDA Dispatch — CLI command assembly and subagent availability.
##
## Closes GAPs D & E from config parameter analysis:
## - GAP D: builds the full CLI command from SubagentDispatchConfig fields
## - GAP E: checks subagent availability via detect_command → findExe
##
## Replaces subagent-dispatch.py lines 429-510 (build_dispatch_command_line,
## build_dispatch_env, check_subagent_available)

import std/[json, os, osproc, strutils, tables]
import ./[utils, config]

# ─────────────────────────── Subagent Availability (GAP E) ───────────────────────────

proc isSubagentAvailable*(cfg: JsonNode, subagentName: string): bool =
  ## Check if a subagent's CLI command is available.
  ## Replaces subagent-system.py's shutil.which(detect_command) pattern.
  if not isSubagentEnabled(cfg, subagentName):
    return false
  let backendClass = dottedGetStr(cfg,
    "agent_system.subagents." & subagentName & ".subagent_backend_class")
  if backendClass == "internal":
    return true  # internal subagents are always available
  let detectCmd = getSubagentDetectCommand(cfg, subagentName)
  if detectCmd.len == 0:
    return false
  return findExe(detectCmd).len > 0

proc getAvailableSubagents*(cfg: JsonNode): seq[string] =
  ## Get all available subagent names.
  let subagents = getSubagents(cfg)
  if subagents.kind != JObject:
    return @[]
  for name, _ in subagents:
    if isSubagentAvailable(cfg, name):
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
    subagentName: string,
    prompt: string,
    model: string = "",
    workdir: string = "",
    writeMode: bool = false,
    webSearch: bool = false,
    outputFile: string = ""
  ): DispatchCommand =
  ## Build the full CLI command line from dispatch config.
  ## Replaces subagent-dispatch.py build_dispatch_command_line (lines 429-480).
  let dispatch = getSubagentDispatch(cfg, subagentName)

  # 1. Command
  let command = dottedGetStr(dispatch, "command")
  if command.len == 0:
    raise newException(ValueError, "No dispatch.command for subagent: " & subagentName)
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
  let actualWorkdir = if workdir.len > 0: workdir else: getCurrentDir()
  if workdirFlag.len > 0:
    result.args.add(workdirFlag)
    result.args.add(actualWorkdir)

  # 7. Model flag
  let modelFlag = dottedGetStr(dispatch, "model_flag")
  let actualModel = if model.len > 0: model
    else: dottedGetStr(cfg, "agent_system.subagents." & subagentName & ".default_model")
  if modelFlag.len > 0 and actualModel.len > 0:
    result.args.add(modelFlag)
    result.args.add(actualModel)

  # 8. Output mode
  result.outputMode = dottedGetStr(dispatch, "output_mode", "stdout")
  if result.outputMode == "file":
    let outFlag = dottedGetStr(dispatch, "output_flag")
    result.outputFile = if outputFile.len > 0: outputFile
      else: getTempDir() / "vida-dispatch-" & subagentName & ".out"
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

proc getTimeout*(cfg: JsonNode, subagentName: string,
                 key: string, default: int): int =
  ## Get a timeout value from dispatch config with a floor of 5 seconds.
  ## Replaces subagent-dispatch.py's _dispatch_timeout helper.
  let dispatch = getSubagentDispatch(cfg, subagentName)
  max(5, dottedGetInt(dispatch, key, default))

proc startupTimeout*(cfg: JsonNode, subagentName: string): int =
  getTimeout(cfg, subagentName, "startup_timeout_seconds", 60)

proc noOutputTimeout*(cfg: JsonNode, subagentName: string): int =
  getTimeout(cfg, subagentName, "no_output_timeout_seconds", 180)

proc progressIdleTimeout*(cfg: JsonNode, subagentName: string): int =
  getTimeout(cfg, subagentName, "progress_idle_timeout_seconds", 120)

proc maxRuntimeExtension*(cfg: JsonNode, subagentName: string): int =
  getTimeout(cfg, subagentName, "max_runtime_extension_seconds", 90)

# ─────────────────────────── Effective Config (Merged) ───────────────────────────

proc effectiveRuntimeSeconds*(cfg: JsonNode, taskClass, subagentName: string): int =
  ## Get the effective max runtime — max of route-level and subagent-level.
  ## Replaces subagent-dispatch.py effective_runtime_limit (line 557-563).
  let routeLimit = getRouteMaxRuntimeSeconds(cfg, taskClass)
  let subagentLimit = dottedGetInt(cfg,
    "agent_system.subagents." & subagentName & ".max_runtime_seconds", 0)
  if routeLimit > 0 and subagentLimit > 0:
    return max(routeLimit, subagentLimit)
  elif routeLimit > 0:
    return routeLimit
  elif subagentLimit > 0:
    return subagentLimit
  else:
    return 420  # default

proc effectiveMinOutputBytes*(cfg: JsonNode, taskClass, subagentName: string): int =
  ## Get effective min_output_bytes — max of route and subagent levels.
  let profile = getRoutingProfile(cfg, taskClass)
  let routeMin = dottedGetInt(profile, "min_output_bytes", 0)
  let subagentMin = dottedGetInt(cfg,
    "agent_system.subagents." & subagentName & ".min_output_bytes", 0)
  max(routeMin, subagentMin)

proc effectiveOrchestrationTier*(cfg: JsonNode, taskClass, subagentName: string): string =
  ## Get the effective orchestration tier — route-level override > subagent-level.
  ## Closes GAP B: merged config overlays.
  let profile = getRoutingProfile(cfg, taskClass)
  let routeOverride = dottedGetStr(profile, "orchestration_tier")
  if routeOverride.len > 0:
    return routeOverride
  return dottedGetStr(cfg,
    "agent_system.subagents." & subagentName & ".orchestration_tier", "standard")
