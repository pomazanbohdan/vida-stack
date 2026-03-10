import std/json
import ../schema
import ./[framework, routing, features, agent_extensions]

proc validateConfig*(config: JsonNode): ValidationResult =
  result.valid = true
  result.errors = @[]
  result.warnings = @[]
  validateFrameworkDomain(config, result)
  if not result.valid and config.kind != JObject:
    return
  validateAgentExtensionsDomain(config, result)
  validateRoutingDomain(config, result)
  validateFeatureDomains(config, result)
