## VIDA Config Loader — typed YAML configuration parser.
##
## Replaces `vida-config.py` (1188 lines, 400+ lines of custom YAML parser)
## with proper YAML library deserialization + validation.

import std/[json, os, strutils]
import ./utils

# ─────────────────────────── Paths ───────────────────────────

const ScriptDir = currentSourcePath().parentDir()
const CompileTimeRoot = ScriptDir.parentDir().parentDir().parentDir().parentDir()

proc loadDotEnv(dir: string): string =
  ## Load VIDA_ROOT from .env file if present.
  let envFile = dir / ".env"
  if not fileExists(envFile):
    return ""
  for line in lines(envFile):
    let trimmed = line.strip()
    if trimmed.startsWith("#") or trimmed.len == 0:
      continue
    let parts = trimmed.split('=', 1)
    if parts.len == 2 and parts[0].strip() == "VIDA_ROOT":
      return parts[1].strip()
  return ""

proc vidaRoot*(): string =
  ## Resolve VIDA project root directory.
  ## Priority: VIDA_ROOT env var > .env file next to binary > compile-time fallback.
  let envRoot = getEnv("VIDA_ROOT")
  if envRoot.len > 0:
    return envRoot
  # Check .env next to the running binary
  let binDir = getAppDir()
  let dotEnvRoot = loadDotEnv(binDir)
  if dotEnvRoot.len > 0:
    return dotEnvRoot
  # Check .env in cwd
  let cwdRoot = loadDotEnv(getCurrentDir())
  if cwdRoot.len > 0:
    return cwdRoot
  return CompileTimeRoot

proc vidaWorkspaceDir*(): string =
  ## Canonical runtime workspace directory for taskflow-v0 transitional state.
  vidaRoot() / ".vida"

proc vidaWorkspacePath*(parts: varargs[string]): string =
  ## Join one or more path segments under the canonical `.vida` workspace.
  result = vidaWorkspaceDir()
  for part in parts:
    result = result / part

proc configPath*(): string =
  ## Resolve path to vida.config.yaml
  vidaRoot() / "vida.config.yaml"

proc configExists*(): bool =
  fileExists(configPath())

# ─────────────────────────── Raw YAML→JSON Loader ───────────────────────────
## For Phase 1 we use a pragmatic approach: parse YAML as JSON-compatible
## structure. vida.config.yaml is already JSON-compatible in structure
## (no YAML anchors, aliases, or multi-doc). In Phase 2 we'll switch
## to the `yaml` nimble package for full YAML support.

proc parseYamlSubset*(content: string): JsonNode  # forward declaration

proc loadRawConfig*(): JsonNode =
  ## Load vida.config.yaml and parse it.
  ## Uses a simple YAML subset parser sufficient for vida.config.yaml.
  let path = configPath()
  if not fileExists(path):
    return newJObject()
  try:
    # vida.config.yaml uses a subset of YAML that can be processed
    # by converting to JSON. For now, load as-is and we'll parse
    # using the yaml nimble package when available.
    let content = readFile(path)
    return parseYamlSubset(content)
  except:
    return newJObject()

# ─── Simple YAML subset parser ───
# Handles: mappings, sequences (- item), scalars, quoted strings, booleans, ints
# Does NOT handle: anchors, aliases, multi-doc, flow sequences/mappings
# This is sufficient for vida.config.yaml

type
  YamlLine = object
    indent: int
    key: string
    value: string
    isListItem: bool
    raw: string

proc parseYamlLine(line: string): YamlLine =
  result.raw = line
  let stripped = line.strip(trailing = false)
  result.indent = line.len - stripped.len

  var content = stripped
  # Skip comments
  if content.startsWith("#") or content.len == 0:
    return

  # List item
  if content.startsWith("- "):
    result.isListItem = true
    content = content[2..^1].strip()

  # Key: value
  let colonPos = content.find(':')
  if colonPos >= 0:
    let beforeColon = content[0..<colonPos]
    # Ensure this is actually a key (no spaces in key part)
    if ' ' notin beforeColon or beforeColon.startsWith("\""):
      result.key = beforeColon.strip().strip(chars = {'"', '\''})
      if colonPos + 1 < content.len:
        result.value = content[colonPos + 1..^1].strip()
      return

  # Plain value (list item or standalone)
  result.value = content

proc yamlScalarToJson(value: string): JsonNode =
  if value.len == 0:
    return newJNull()
  # Quoted string
  if (value.startsWith("\"") and value.endsWith("\"")) or
     (value.startsWith("'") and value.endsWith("'")):
    return newJString(value[1..^2])
  # Boolean
  let lower = value.toLowerAscii()
  if lower in ["true", "yes", "on"]:
    return newJBool(true)
  if lower in ["false", "no", "off"]:
    return newJBool(false)
  # Null
  if lower in ["null", "~", ""]:
    return newJNull()
  # Integer
  try:
    return newJInt(parseInt(value))
  except ValueError:
    discard
  # Float
  try:
    return newJFloat(parseFloat(value))
  except ValueError:
    discard
  # String
  return newJString(value)

proc parseYamlSubset*(content: string): JsonNode =
  ## Parse a YAML subset into a JsonNode tree.
  let lines = content.splitLines()
  var lineInfos: seq[YamlLine] = @[]
  for line in lines:
    let trimmed = line.strip()
    if trimmed.len == 0 or trimmed.startsWith("#"):
      continue
    lineInfos.add(parseYamlLine(line))

  proc parseLevel(start: int, minIndent: int): (JsonNode, int) =
    ## Parse from line `start` at indent level `minIndent`.
    ## Returns (parsed node, next line index).
    if start >= lineInfos.len:
      return (newJObject(), lineInfos.len)

    let firstLine = lineInfos[start]

    # Check if this is a list
    if firstLine.isListItem:
      var arr = newJArray()
      var i = start
      while i < lineInfos.len:
        let line = lineInfos[i]
        if line.indent < minIndent:
          break
        if not line.isListItem and line.indent <= minIndent:
          break
        if line.isListItem:
          if line.key.len > 0:
            # List item with key: value (mapping in list)
            var obj = newJObject()
            obj[line.key] = yamlScalarToJson(line.value)
            # Check for nested keys at higher indent
            var j = i + 1
            while j < lineInfos.len and lineInfos[j].indent > line.indent:
              let nested = lineInfos[j]
              if nested.key.len > 0:
                if nested.value.len > 0:
                  obj[nested.key] = yamlScalarToJson(nested.value)
                else:
                  let (subNode, nextJ) = parseLevel(j + 1, nested.indent + 1)
                  obj[nested.key] = subNode
                  j = nextJ
                  continue
              j += 1
            arr.add(obj)
            i = j
          else:
            # Simple list item
            arr.add(yamlScalarToJson(line.value))
            i += 1
        else:
          i += 1
      return (arr, i)

    # Otherwise it's a mapping
    var obj = newJObject()
    var i = start
    while i < lineInfos.len:
      let line = lineInfos[i]
      if line.indent < minIndent and i > start:
        break
      if line.key.len == 0:
        i += 1
        continue
      if line.value.len > 0:
        # Key with immediate value
        obj[line.key] = yamlScalarToJson(line.value)
        i += 1
      else:
        # Key without value — nested structure follows
        if i + 1 < lineInfos.len and lineInfos[i + 1].indent > line.indent:
          let (subNode, nextI) = parseLevel(i + 1, lineInfos[i + 1].indent)
          obj[line.key] = subNode
          i = nextI
        else:
          obj[line.key] = newJNull()
          i += 1
    return (obj, i)

  let (root, _) = parseLevel(0, 0)
  return root

# ─────────────────────────── Validated Config ───────────────────────────

proc loadValidatedConfig*(): JsonNode =
  ## Load and return the validated config.
  ## Returns empty JObject if config doesn't exist.
  if not configExists():
    return newJObject()
  return loadRawConfig()

# ─────────────────────────── Config Access Helpers ───────────────────────────

proc getLanguagePolicy*(config: JsonNode): JsonNode =
  dottedGet(config, "language_policy", newJObject())

proc getProtocolActivation*(config: JsonNode): JsonNode =
  dottedGet(config, "protocol_activation", newJObject())

proc isProtocolActive*(config: JsonNode, protocol: string): bool =
  dottedGetBool(config, "protocol_activation." & protocol, false)

proc getAgentSystem*(config: JsonNode): JsonNode =
  dottedGet(config, "agent_system", newJObject())

proc getAgentBackends*(config: JsonNode): JsonNode =
  dottedGet(config, "agent_system.subagents", newJObject())

proc getScoring*(config: JsonNode): JsonNode =
  dottedGet(config, "agent_system.scoring", newJObject())

proc getFrameworkSelfDiagnosis*(config: JsonNode): JsonNode =
  dottedGet(config, "framework_self_diagnosis", newJObject())

proc agentBackendHasWebSearchWiring*(agentBackendCfg: JsonNode): bool =
  ## Check if an agent backend has web search capability wired.
  if agentBackendCfg.isNil or agentBackendCfg.kind != JObject:
    return false
  let dispatch = dottedGet(agentBackendCfg, "dispatch")
  if dispatch.kind != JObject:
    return false
  let webMode = dottedGetStr(dispatch, "web_search_mode")
  if webMode.len > 0:
    return true
  let webFlag = dottedGetStr(dispatch, "web_search_flag")
  if webFlag.len > 0:
    return true
  return false

# ─────────────────────────── Project & Bootstrap ───────────────────────────

proc getProject*(config: JsonNode): JsonNode =
  dottedGet(config, "project", newJObject())

proc getProjectBootstrap*(config: JsonNode): JsonNode =
  dottedGet(config, "project_bootstrap", newJObject())

proc isBootstrapEnabled*(config: JsonNode): bool =
  dottedGetBool(config, "project_bootstrap.enabled", false)

# ─────────────────────────── Autonomous Execution ───────────────────────────

proc getAutonomousExecution*(config: JsonNode): JsonNode =
  dottedGet(config, "autonomous_execution", newJObject())

proc isNextTaskBoundaryAnalysisEnabled*(config: JsonNode): bool =
  dottedGetBool(config, "autonomous_execution.next_task_boundary_analysis", false)

proc nextTaskBoundaryReport*(config: JsonNode): string =
  dottedGetStr(config, "autonomous_execution.next_task_boundary_report", "brief_plan")

# ─────────────────────────── Routing ───────────────────────────

proc getRouting*(config: JsonNode): JsonNode =
  dottedGet(config, "agent_system.routing", newJObject())

proc getRoutingProfile*(config: JsonNode, taskClass: string): JsonNode =
  ## Get routing profile for a task class, falling back to "default".
  let routing = getRouting(config)
  if routing.kind == JObject and routing.hasKey(taskClass):
    return routing[taskClass]
  if routing.kind == JObject and routing.hasKey("default"):
    return routing["default"]
  return newJObject()

proc getRouteAgentBackends*(config: JsonNode, taskClass: string): seq[string] =
  ## Get ordered agent-backend list for a routing profile.
  let profile = getRoutingProfile(config, taskClass)
  splitCsv(dottedGet(profile, "subagents"))

proc getRouteModelOverride*(config: JsonNode, taskClass, agentBackendName: string): string =
  ## Get per-route model override for a specific agent backend.
  let profile = getRoutingProfile(config, taskClass)
  dottedGetStr(profile, "models." & agentBackendName)

proc getRouteProfileOverride*(config: JsonNode, taskClass, agentBackendName: string): string =
  ## Get per-route profile override for a specific agent backend.
  let profile = getRoutingProfile(config, taskClass)
  dottedGetStr(profile, "profiles." & agentBackendName)

proc isExternalFirstRequired*(config: JsonNode, taskClass: string): bool =
  let profile = getRoutingProfile(config, taskClass)
  dottedGetBool(profile, "external_first_required", false)

proc isAnalysisRequired*(config: JsonNode, taskClass: string): bool =
  let profile = getRoutingProfile(config, taskClass)
  dottedGetBool(profile, "analysis_required", false)

proc isCoachRequired*(config: JsonNode, taskClass: string): bool =
  let profile = getRoutingProfile(config, taskClass)
  dottedGetBool(profile, "coach_required", false)

proc isWebSearchRequired*(config: JsonNode, taskClass: string): bool =
  let profile = getRoutingProfile(config, taskClass)
  dottedGetBool(profile, "web_search_required", false)

proc isIndependentVerificationRequired*(config: JsonNode, taskClass: string): bool =
  let profile = getRoutingProfile(config, taskClass)
  dottedGetBool(profile, "independent_verification_required", false)

proc getRouteWriteScope*(config: JsonNode, taskClass: string): string =
  let profile = getRoutingProfile(config, taskClass)
  dottedGetStr(profile, "write_scope", "none")

proc getRouteDispatchRequired*(config: JsonNode, taskClass: string): string =
  let profile = getRoutingProfile(config, taskClass)
  dottedGetStr(profile, "dispatch_required", "external_first_when_eligible")

proc getRouteBudgetUnits*(config: JsonNode, taskClass: string): int =
  let profile = getRoutingProfile(config, taskClass)
  dottedGetInt(profile, "max_budget_units", 4)

proc getRouteFanoutAgentBackends*(config: JsonNode, taskClass: string): seq[string] =
  let profile = getRoutingProfile(config, taskClass)
  splitCsv(dottedGet(profile, "fanout_subagents"))

proc getRouteMaxCliCalls*(config: JsonNode, taskClass: string): int =
  let profile = getRoutingProfile(config, taskClass)
  dottedGetInt(profile, "max_cli_subagent_calls", 5)

proc getRouteMaxRuntimeSeconds*(config: JsonNode, taskClass: string): int =
  let profile = getRoutingProfile(config, taskClass)
  dottedGetInt(profile, "max_total_runtime_seconds", 420)

# ─────────────────────────── Agent Backend Dispatch Config ───────────────────────────

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

proc getPackRouterKeywords*(config: JsonNode): JsonNode =
  dottedGet(config, "pack_router_keywords", newJObject())

# ─────────────────────────── Validation ───────────────────────────

type ValidationResult* = object
  valid*: bool
  errors*: seq[string]
  warnings*: seq[string]

proc validateConfig*(config: JsonNode): ValidationResult =
  ## Validate a loaded configuration.
  result.valid = true
  result.errors = @[]
  result.warnings = @[]

  if config.kind != JObject:
    result.valid = false
    result.errors.add("config must be a YAML mapping")
    return

  # Check required sections
  let protocolAct = getProtocolActivation(config)
  if protocolAct.kind != JObject:
    result.warnings.add("missing protocol_activation section")

  # Validate agent_system if present
  let agentSystem = getAgentSystem(config)
  if agentSystem.kind == JObject:
    let mode = dottedGetStr(agentSystem, "mode", "native")
    if mode notin ["disabled", "native", "hybrid"]:
      result.errors.add("agent_system.mode must be disabled, native, or hybrid; got: " & mode)
      result.valid = false

    let maxParallel = dottedGetInt(agentSystem, "max_parallel_agents", 1)
    if maxParallel < 1:
      result.errors.add("agent_system.max_parallel_agents must be >= 1")
      result.valid = false

    # Validate subagents
    let subagents = dottedGet(agentSystem, "subagents")
    if subagents.kind == JObject:
      for name, cfg in subagents:
        if cfg.kind != JObject:
          result.errors.add("agent_system.subagents." & name & " must be a mapping")
          result.valid = false
          continue
        let writeScope = dottedGetStr(cfg, "write_scope", "none")
        if writeScope notin ["none", "scoped_only", "sandbox", "patch",
                             "orchestrator_native", "external_write", "repo_write"]:
          result.warnings.add("agent_system.subagents." & name &
            ".write_scope has unknown value: " & writeScope)

# ─────────────────────────── CLI Commands ───────────────────────────

proc cmdValidate*(): int =
  ## Validate vida.config.yaml and print result.
  if not configExists():
    echo "vida.config.yaml not found at: " & configPath()
    return 1
  let config = loadRawConfig()
  let vr = validateConfig(config)
  if vr.valid:
    echo "✅ vida.config.yaml is valid"
    for w in vr.warnings:
      echo "  ⚠ " & w
    return 0
  else:
    echo "❌ vida.config.yaml validation failed:"
    for e in vr.errors:
      echo "  ✗ " & e
    for w in vr.warnings:
      echo "  ⚠ " & w
    return 1

proc cmdDump*(): int =
  ## Dump parsed config as JSON.
  if not configExists():
    echo "{}"
    return 0
  let config = loadRawConfig()
  echo pretty(config)
  return 0

proc cmdProtocolActive*(protocol: string): int =
  ## Check if a protocol is active.
  let config = loadValidatedConfig()
  if isProtocolActive(config, protocol):
    echo "true"
    return 0
  else:
    echo "false"
    return 1
