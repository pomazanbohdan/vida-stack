## VIDA Capability Registry — typed capability checks and task-class compatibility.
##
## Replaces `capability-registry.py` (138 lines).
## Provides task-class → subagent compatibility validation
## based on write_scope, capability_band, and forbidden capabilities.

import std/[json, os, strutils, tables, sets, algorithm, sequtils]
import ../core/[utils, config, toon]

# ─────────────────────────── Task Class Requirements ───────────────────────────

type TaskClassRequirement* = object
  allowedWriteScopes*: HashSet[string]
  requiredCapabilityAny*: HashSet[string]
  requiredArtifacts*: seq[string]
  forbiddenCapabilities*: HashSet[string]

proc initReq(scopes, caps: openArray[string],
             artifacts: openArray[string],
             forbidden: openArray[string] = []): TaskClassRequirement =
  result.allowedWriteScopes = toHashSet(scopes)
  result.requiredCapabilityAny = toHashSet(caps)
  result.requiredArtifacts = @artifacts
  result.forbiddenCapabilities = toHashSet(forbidden)

let TaskClassRequirements* = {
  "analysis": initReq(["none"], ["read_only", "review_safe"], ["analysis_receipt"]),
  "coach": initReq(["none"], ["review_safe"], ["coach_review"]),
  "verification": initReq(["none"], ["review_safe"], ["verification_manifest"]),
  "verification_ensemble": initReq(["none"], ["review_safe"], ["verification_manifest"]),
  "review_ensemble": initReq(["none"], ["review_safe"], ["verification_manifest"]),
  "problem_party": initReq(["none"], ["read_only", "review_safe"],
                           ["problem_party_receipt"], ["bounded_write_safe"]),
  "read_only_prep": initReq(["none"], ["read_only"], ["prep_manifest"]),
  "implementation": initReq(["scoped_only", "orchestrator_native"],
                            ["bounded_write_safe"], ["writer_output"]),
}.toTable

# ─────────────────────────── Capability Entry ───────────────────────────

proc capabilityEntry(name: string, payload: JsonNode): JsonNode =
  %*{
    "subagent": name,
    "backend_class": dottedGetStr(payload, "subagent_backend_class"),
    "role": dottedGetStr(payload, "role"),
    "write_scope": dottedGetStr(payload, "write_scope", "none"),
    "capability_band": splitCsv(payload{"capability_band"}),
    "specialties": splitCsv(payload{"specialties"}),
    "billing_tier": dottedGetStr(payload, "billing_tier"),
    "speed_tier": dottedGetStr(payload, "speed_tier"),
    "quality_tier": dottedGetStr(payload, "quality_tier"),
    "web_search_wired": subagentHasWebSearchWiring(payload),
  }

# ─────────────────────────── Registry Builder ───────────────────────────

proc buildRegistry*(cfg: JsonNode): JsonNode =
  let subagentsCfg = getSubagents(cfg)
  var subagents = newJObject()
  if subagentsCfg.kind == JObject:
    for name, payload in subagentsCfg:
      if payload.kind == JObject:
        subagents[name] = capabilityEntry(name, payload)

  var reqs = newJObject()
  for name, req in TaskClassRequirements:
    reqs[name] = %*{
      "allowed_write_scopes": sorted(req.allowedWriteScopes.toSeq),
      "required_capability_any": sorted(req.requiredCapabilityAny.toSeq),
      "required_artifacts": req.requiredArtifacts,
      "forbidden_capabilities": sorted(req.forbiddenCapabilities.toSeq),
    }

  result = %*{
    "generated_at": "runtime",
    "subagents": subagents,
    "task_class_requirements": reqs,
  }

proc requirementFor*(taskClass: string): TaskClassRequirement =
  if TaskClassRequirements.hasKey(taskClass):
    return TaskClassRequirements[taskClass]
  return initReq(["none"], ["read_only"], [])

# ─────────────────────────── Compatibility Check ───────────────────────────

type CompatibilityResult* = object
  compatible*: bool
  reason*: string
  taskClass*: string
  subagent*: string
  requiredArtifacts*: seq[string]

proc compatibilityFor*(taskClass, subagentName: string,
                       registry: JsonNode = nil): CompatibilityResult =
  let reg = if registry.isNil: buildRegistry(loadRawConfig()) else: registry
  let subagent = reg{"subagents"}{subagentName}
  result.taskClass = taskClass
  result.subagent = subagentName

  if subagent.isNil or subagent.kind != JObject:
    result.compatible = false
    result.reason = "unknown_subagent"
    return

  let req = requirementFor(taskClass)
  let capBand = splitCsv(subagent{"capability_band"}).mapIt(it.toLowerAscii).toHashSet
  let writeScope = dottedGetStr(subagent, "write_scope", "none")
  var reasons: seq[string] = @[]

  if writeScope notin req.allowedWriteScopes:
    reasons.add("write_scope_mismatch")

  if req.requiredCapabilityAny.len > 0:
    let lowReq = req.requiredCapabilityAny.toSeq.mapIt(it.toLowerAscii).toHashSet
    if (capBand * lowReq).len == 0:
      reasons.add("missing_required_capability_band")

  if req.forbiddenCapabilities.len > 0:
    let lowForbidden = req.forbiddenCapabilities.toSeq.mapIt(it.toLowerAscii).toHashSet
    if (capBand * lowForbidden).len > 0:
      reasons.add("forbidden_capability_present")

  result.compatible = reasons.len == 0
  result.reason = if reasons.len == 0: "ok" else: reasons.join(",")
  result.requiredArtifacts = req.requiredArtifacts

# ─────────────────────────── CLI ───────────────────────────

proc cmdRegistry*(args: seq[string]): int =
  if args.len == 0:
    echo "Usage: vida-v0 registry <build|check <task_class> <subagent>>"
    return 2

  case args[0]
  of "build":
    let cfg = loadRawConfig()
    let reg = buildRegistry(cfg)
    let path = vidaRoot() / ".vida" / "state" / "capability-registry.json"
    createDir(path.parentDir())
    saveJson(path, reg)
    echo path
    return 0

  of "check":
    if args.len < 3:
      echo "Usage: vida-v0 registry check <task_class> <subagent>"
      return 2
    let cr = compatibilityFor(args[1], args[2])
    let payload = normalizeJson(%*{
      "compatible": cr.compatible,
      "reason": cr.reason,
      "task_class": cr.taskClass,
      "subagent": cr.subagent,
      "required_artifacts": cr.requiredArtifacts,
    })
    if "--json" in args: echo pretty(payload) else: echo renderToon(payload)
    return 0

  else:
    echo "Unknown registry subcommand: " & args[0]
    return 2
