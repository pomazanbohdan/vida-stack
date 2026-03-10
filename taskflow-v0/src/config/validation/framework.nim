import std/[json, os]
import ../../core/utils
import ../[accessors, schema]

proc validateFrameworkDomain*(config: JsonNode, result: var ValidationResult) =
  if config.kind != JObject:
    result.valid = false
    result.errors.add("config must be a YAML mapping")
    return

  let protocolAct = getProtocolActivation(config)
  if protocolAct.kind != JObject:
    result.warnings.add("missing protocol_activation section")

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
          result.warnings.add("agent_system.subagents." & name & ".write_scope has unknown value: " & writeScope)
        let binaryPath = getAgentBackendBinaryPath(config, name)
        if binaryPath.len > 0 and not fileExists(binaryPath):
          result.errors.add("agent_system.subagents." & name & ".binary_path does not exist: " & binaryPath)
          result.valid = false
