## VIDA Subagent Pool — leased subagent pool management.
##
## Replaces `subagent-pool.py` (132 lines).
## Borrow/release subagents with pool-scoped leases,
## candidate selection excluding already-leased subagents.

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
    if policyValue(item{"resource_type"}, "") != "subagent_pool": continue
    let resourceId = policyValue(item{"resource_id"}, "")
    if resourceId.len > 0:
      result[resourceId] = item

# ─────────────────────────── Borrow ───────────────────────────

proc borrowSubagent*(taskClass, holder: string, ttlSeconds: int = 1800): JsonNode =
  let leased = activePoolLeases()
  let cfg = loadRawConfig()
  let subagents = detectSubagents(cfg)
  # Simple candidate selection: available subagents not already leased
  var candidates: seq[JsonNode] = @[]
  for name, payload in subagents:
    if not dottedGetBool(payload, "available", false): continue
    if leased.hasKey(name): continue
    candidates.add(%*{
      "subagent": name,
      "billing_tier": payload{"billing_tier"},
      "speed_tier": payload{"speed_tier"},
      "quality_tier": payload{"quality_tier"},
      "cost_priority": payload{"cost_priority"},
    })

  if candidates.len == 0:
    return %*{
      "status": "blocked",
      "reason": "no_pool_candidate",
      "leased_subagents": sorted(leased.keys.toSeq),
    }

  let selected = candidates[0]
  let subagent = policyValue(selected{"subagent"}, "")
  let lease = acquireLease("subagent_pool", subagent, holder, ttlSeconds)
  return %*{
    "status": policyValue(lease{"status"}, "blocked"),
    "task_class": taskClass,
    "holder": holder,
    "selected_subagent": subagent,
    "candidate": selected,
    "lease": lease{"lease"},
    "leased_subagents": sorted(leased.keys.toSeq),
  }

# ─────────────────────────── Release ───────────────────────────

proc releaseSubagent*(subagent, holder: string): JsonNode =
  let lease = releaseLease("subagent_pool", subagent, holder)
  return %*{
    "status": policyValue(lease{"status"}, "blocked"),
    "subagent": subagent,
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
      if item.kind == JObject and policyValue(item{"resource_type"}, "") == "subagent_pool":
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
  vida-v0 pool release <subagent> <holder>
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
    let payload = normalizeJson(borrowSubagent(args[1], args[2], max(60, ttl)))
    if "--json" in args: echo pretty(payload) else: echo renderToon(payload)
    return 0

  of "release":
    if args.len < 3:
      echo "Usage: vida-v0 pool release <subagent> <holder>"
      return 1
    let payload = normalizeJson(releaseSubagent(args[1], args[2]))
    if "--json" in args: echo pretty(payload) else: echo renderToon(payload)
    return 0

  of "status":
    let payload = normalizeJson(poolStatus())
    if "--json" in args: echo pretty(payload) else: echo renderToon(payload)
    return 0

  else:
    echo "Unknown pool subcommand: " & args[0]
    return 1
