## VIDA Agent Backend Pool — leased agent backend pool management.
##
## Replaces the legacy pool helper script.
## Borrow/release agent backends with pool-scoped leases,
## candidate selection excluding already-leased backends.

import std/[json, strutils, algorithm, sequtils, tables]
import ../core/[utils, config, toon]
import ./[leases, system]

# ─────────────────────────── Active Pool Leases ───────────────────────────

proc activePoolLeases*(): Table[string, JsonNode] =
  result = initTable[string, JsonNode]()
  let payload = activeLeases()
  let rows = payload{"leases"}
  if rows.isNil or rows.kind != JArray: return
  for item in rows:
    if item.kind != JObject: continue
    if policyValue(item{"resource_type"}, "") != "agent_backend_pool": continue
    let resourceId = policyValue(item{"resource_id"}, "")
    if resourceId.len > 0:
      result[resourceId] = item

# ─────────────────────────── Borrow ───────────────────────────

proc borrowAgentBackend*(taskClass, holder: string, ttlSeconds: int = 1800): JsonNode =
  let leased = activePoolLeases()
  let cfg = loadRawConfig()
  let subagents = detectAgentBackends(cfg)
  # Simple candidate selection: available agent backends not already leased
  var candidates: seq[JsonNode] = @[]
  for name, payload in subagents:
    if not dottedGetBool(payload, "available", false): continue
    if leased.hasKey(name): continue
    candidates.add(%*{
      "agent_backend": name,
      "billing_tier": payload{"billing_tier"},
      "speed_tier": payload{"speed_tier"},
      "quality_tier": payload{"quality_tier"},
      "cost_priority": payload{"cost_priority"},
    })

  if candidates.len == 0:
    return %*{
      "status": "blocked",
      "reason": "no_pool_candidate",
      "leased_agent_backends": sorted(leased.keys.toSeq),
    }

  let selected = candidates[0]
  let agentBackend = policyValue(selected{"agent_backend"}, "")
  let lease = acquireLease("agent_backend_pool", agentBackend, holder, ttlSeconds)
  return %*{
    "status": policyValue(lease{"status"}, "blocked"),
    "task_class": taskClass,
    "holder": holder,
    "selected_agent_backend": agentBackend,
    "candidate": selected,
    "lease": lease{"lease"},
    "leased_agent_backends": sorted(leased.keys.toSeq),
  }

# ─────────────────────────── Release ───────────────────────────

proc releaseAgentBackend*(agentBackend, holder: string): JsonNode =
  let lease = releaseLease("agent_backend_pool", agentBackend, holder)
  return %*{
    "status": policyValue(lease{"status"}, "blocked"),
    "agent_backend": agentBackend,
    "holder": holder,
    "lease": lease{"lease"},
  }

# ─────────────────────────── Pool Status ───────────────────────────

proc poolStatus*(): JsonNode =
  let payload = activeLeases()
  var poolLeases = newJArray()
  let rows = payload{"leases"}
  if not rows.isNil and rows.kind == JArray:
    for item in rows:
      if item.kind == JObject and policyValue(item{"resource_type"}, "") == "agent_backend_pool":
        poolLeases.add(item)
  return %*{
    "generated_at": nowUtc(),
    "active_pool_leases": poolLeases,
  }

# ─────────────────────────── CLI ───────────────────────────

proc cmdPool*(args: seq[string]): int =
  if args.len == 0:
    echo """Usage:
  vida-v0 pool borrow <task_class> <holder> [--ttl-seconds N]
  vida-v0 pool release <agent_backend> <holder>
  vida-v0 pool status"""
    return 1

  case args[0]
  of "borrow":
    if args.len < 3:
      echo "Usage: vida-v0 pool borrow <task_class> <holder> [--ttl-seconds N]"
      return 1
    var ttl = 1800
    if args.len > 3 and args[3] == "--ttl-seconds" and args.len > 4:
      ttl = parseInt(args[4])
    let payload = normalizeJson(borrowAgentBackend(args[1], args[2], max(60, ttl)))
    if "--json" in args: echo pretty(payload) else: echo renderToon(payload)
    return 0

  of "release":
    if args.len < 3:
      echo "Usage: vida-v0 pool release <agent_backend> <holder>"
      return 1
    let payload = normalizeJson(releaseAgentBackend(args[1], args[2]))
    if "--json" in args: echo pretty(payload) else: echo renderToon(payload)
    return 0

  of "status":
    let payload = normalizeJson(poolStatus())
    if "--json" in args: echo pretty(payload) else: echo renderToon(payload)
    return 0

  else:
    echo "Unknown pool subcommand: " & args[0]
    return 1
