## VIDA Leases — distributed resource lease management.
##
## Extracted from the legacy agent-system script.
## Provides acquire/renew/release with fencing tokens,
## conflict detection, history tracking, and automatic expiration.

import std/[json, os, strutils, algorithm, sequtils, times, options]
import ../core/[utils, config, toon]

# ─────────────────────────── Paths ───────────────────────────

proc leasePath*(): string = vidaRoot() / ".vida" / "state" / "agent-backend-leases.json"

# ─────────────────────────── Load / Save ───────────────────────────

proc loadLeases(): JsonNode =
  let default = %*{"leases": {}, "next_fencing_token": 1, "history": []}
  let payload = loadJson(leasePath(), default)
  if payload.kind != JObject:
    return default
  if not payload.hasKey("leases"): payload["leases"] = newJObject()
  if not payload.hasKey("next_fencing_token"): payload["next_fencing_token"] = %1
  if not payload.hasKey("history"): payload["history"] = newJArray()
  return payload

proc saveLeases(payload: JsonNode) =
  let path = leasePath()
  createDir(path.parentDir())
  saveJson(path, payload)

# ─────────────────────────── Cleanup ───────────────────────────

proc cleanupLeases(payload: JsonNode, retainHours: int = 24): JsonNode =
  result = payload
  let leases = result["leases"]
  let now = getTime()
  let retainAfter = now - initDuration(hours = max(1, retainHours))
  var pruned = newJObject()
  for key, lease in leases:
    if lease.kind != JObject:
      continue
    # Expire active leases past their time
    let expiresAt = parseUtcTimestamp(policyValue(lease{"expires_at"}, ""))
    if policyValue(lease{"status"}, "active") == "active":
      if expiresAt.isSome and expiresAt.get.toTime < now:
        lease["status"] = %"expired"
        lease["expired_at"] = %nowUtc()
    let status = policyValue(lease{"status"}, "active")
    var keep = true
    if status in ["released", "expired"]:
      let releasedAt = parseUtcTimestamp(policyValue(lease{"released_at"}, ""))
      let marker = if releasedAt.isSome: releasedAt else: expiresAt
      if marker.isSome and marker.get.toTime < retainAfter:
        keep = false
    if keep:
      pruned[key] = lease
  result["leases"] = pruned
  # Trim history to 200
  let history = result{"history"}
  if not history.isNil and history.kind == JArray and history.len > 200:
    var trimmed = newJArray()
    for i in max(0, history.len - 200) ..< history.len:
      trimmed.add(history[i])
    result["history"] = trimmed

proc addHistory(payload: JsonNode, event: JsonNode, limit: int = 50) =
  let history = payload{"history"}
  if history.isNil or history.kind != JArray:
    payload["history"] = newJArray()
  payload["history"].add(event)
  if payload["history"].len > limit:
    var trimmed = newJArray()
    for i in max(0, payload["history"].len - limit) ..< payload["history"].len:
      trimmed.add(payload["history"][i])
    payload["history"] = trimmed

# ─────────────────────────── Acquire ───────────────────────────

proc acquireLease*(resourceType, resourceId, holder: string,
                   ttlSeconds: int = 3600): JsonNode =
  var payload = cleanupLeases(loadLeases())
  let key = resourceType & ":" & resourceId
  let current = payload["leases"]{key}
  let now = getTime()

  if not current.isNil and current.kind == JObject:
    let expiresAt = parseUtcTimestamp(policyValue(current{"expires_at"}, ""))
    let currentStatus = policyValue(current{"status"}, "active")
    let currentHolder = policyValue(current{"holder"}, "")
    if currentStatus == "active" and currentHolder != holder and
       expiresAt.isSome and expiresAt.get.toTime > now:
      # Conflict
      current["conflict_count"] = %(policyInt(current{"conflict_count"}, 0) + 1)
      current["last_conflict_at"] = %nowUtc()
      current["last_conflict_holder"] = %holder
      addHistory(payload, %*{
        "ts": nowUtc(), "resource_type": resourceType,
        "resource_id": resourceId, "holder": holder,
        "active_holder": currentHolder, "event": "lease_conflict",
      })
      saveLeases(payload)
      return %*{
        "status": "blocked", "resource_type": resourceType,
        "resource_id": resourceId, "event": "lease_conflict",
        "lease": current,
      }

  let fencingToken = policyInt(payload{"next_fencing_token"}, 1)
  payload["next_fencing_token"] = %(fencingToken + 1)
  let ttlMinutes = max(1, ttlSeconds div 60)
  let lease = %*{
    "resource_type": resourceType, "resource_id": resourceId,
    "holder": holder, "acquired_at": nowUtc(),
    "expires_at": futureUtcIso(minutes = ttlMinutes),
    "fencing_token": fencingToken, "status": "active",
    "conflict_count": 0,
  }
  payload["leases"][key] = lease
  addHistory(payload, %*{
    "ts": nowUtc(), "resource_type": resourceType,
    "resource_id": resourceId, "holder": holder,
    "event": "lease_acquired", "fencing_token": fencingToken,
  })
  saveLeases(payload)
  return %*{"status": "acquired", "lease": lease}

# ─────────────────────────── Renew ───────────────────────────

proc renewLease*(resourceType, resourceId, holder: string,
                 ttlSeconds: int = 3600): JsonNode =
  var payload = cleanupLeases(loadLeases())
  let key = resourceType & ":" & resourceId
  let current = payload["leases"]{key}
  if current.isNil or current.kind != JObject:
    return %*{"status": "noop", "reason": "missing"}
  if policyValue(current{"holder"}, "") != holder:
    current["conflict_count"] = %(policyInt(current{"conflict_count"}, 0) + 1)
    current["last_conflict_at"] = %nowUtc()
    current["last_conflict_holder"] = %holder
    addHistory(payload, %*{
      "ts": nowUtc(), "resource_type": resourceType,
      "resource_id": resourceId, "holder": holder,
      "active_holder": policyValue(current{"holder"}, ""),
      "event": "renew_conflict",
    }, 200)
    saveLeases(payload)
    return %*{"status": "blocked", "reason": "holder_mismatch", "lease": current}
  if policyValue(current{"status"}, "active") != "active":
    return %*{"status": "noop", "reason": "not_active", "lease": current}
  let ttlMinutes = max(1, ttlSeconds div 60)
  current["renewed_at"] = %nowUtc()
  current["expires_at"] = %futureUtcIso(minutes = ttlMinutes)
  addHistory(payload, %*{
    "ts": nowUtc(), "resource_type": resourceType,
    "resource_id": resourceId, "holder": holder,
    "event": "lease_renewed", "fencing_token": current{"fencing_token"},
  }, 200)
  saveLeases(payload)
  return %*{"status": "renewed", "lease": current}

# ─────────────────────────── Release ───────────────────────────

proc releaseLease*(resourceType, resourceId, holder: string): JsonNode =
  var payload = cleanupLeases(loadLeases())
  let key = resourceType & ":" & resourceId
  let current = payload["leases"]{key}
  if current.isNil or current.kind != JObject:
    return %*{"status": "noop", "reason": "missing"}
  if policyValue(current{"holder"}, "") != holder:
    current["conflict_count"] = %(policyInt(current{"conflict_count"}, 0) + 1)
    current["last_conflict_at"] = %nowUtc()
    current["last_conflict_holder"] = %holder
    addHistory(payload, %*{
      "ts": nowUtc(), "resource_type": resourceType,
      "resource_id": resourceId, "holder": holder,
      "active_holder": policyValue(current{"holder"}, ""),
      "event": "release_conflict",
    }, 200)
    saveLeases(payload)
    return %*{"status": "blocked", "reason": "holder_mismatch", "lease": current}
  current["status"] = %"released"
  current["released_at"] = %nowUtc()
  addHistory(payload, %*{
    "ts": nowUtc(), "resource_type": resourceType,
    "resource_id": resourceId, "holder": holder,
    "event": "lease_released", "fencing_token": current{"fencing_token"},
  }, 200)
  saveLeases(payload)
  return %*{"status": "released", "lease": current}

# ─────────────────────────── Active Leases ───────────────────────────

proc activeLeases*(): JsonNode =
  var payload = cleanupLeases(loadLeases())
  let leases = payload["leases"]
  var rows: seq[JsonNode] = @[]
  var byResourceType = newJObject()

  for key, lease in leases:
    if lease.kind != JObject: continue
    var row = lease.copy()
    row["key"] = %key
    rows.add(row)
    let rt = policyValue(lease{"resource_type"}, "unknown")
    let status = policyValue(lease{"status"}, "active")
    if not byResourceType.hasKey(rt):
      byResourceType[rt] = %*{"active": 0, "released": 0, "expired": 0}
    let current = byResourceType[rt]{status}
    byResourceType[rt][status] = %(if current.isNil: 1 else: current.getInt() + 1)

  saveLeases(payload)
  rows.sort(proc(a, b: JsonNode): int =
    let aActive = policyValue(a{"status"}, "") == "active"
    let bActive = policyValue(b{"status"}, "") == "active"
    if aActive != bActive: return (if aActive: -1 else: 1)
    return cmp(policyValue(a{"key"}, ""), policyValue(b{"key"}, ""))
  )

  let history = payload{"history"}
  var recentHistory: seq[JsonNode] = @[]
  if not history.isNil and history.kind == JArray:
    let start = max(0, history.len - 20)
    for i in start ..< history.len:
      if history[i].kind == JObject:
        recentHistory.add(history[i])

  var conflicts = 0
  for item in recentHistory:
    if policyValue(item{"event"}, "").endsWith("conflict"):
      conflicts += 1

  result = %*{
    "generated_at": nowUtc(),
    "leases": rows,
    "history": recentHistory,
    "summary": {
      "active": rows.filterIt(policyValue(it{"status"}, "") == "active").len,
      "released": rows.filterIt(policyValue(it{"status"}, "") == "released").len,
      "expired": rows.filterIt(policyValue(it{"status"}, "") == "expired").len,
      "recent_conflicts": conflicts,
      "by_resource_type": byResourceType,
    },
  }

# ─────────────────────────── CLI ───────────────────────────

proc cmdLease*(args: seq[string]): int =
  if args.len == 0:
    echo """Usage:
  taskflow-v0 lease acquire <resource_type> <resource_id> <holder> [--ttl-seconds N]
  taskflow-v0 lease renew <resource_type> <resource_id> <holder> [--ttl-seconds N]
  taskflow-v0 lease release <resource_type> <resource_id> <holder>
  taskflow-v0 lease list"""
    return 1

  case args[0]
  of "acquire":
    if args.len < 4:
      echo "Usage: taskflow-v0 lease acquire <resource_type> <resource_id> <holder>"
      return 1
    var ttl = 3600
    if args.len > 4 and args[4] == "--ttl-seconds" and args.len > 5:
      ttl = parseInt(args[5])
    let payload = normalizeJson(acquireLease(args[1], args[2], args[3], max(60, ttl)))
    if "--json" in args: echo pretty(payload) else: echo renderToon(payload)
    return 0

  of "renew":
    if args.len < 4:
      echo "Usage: taskflow-v0 lease renew <resource_type> <resource_id> <holder>"
      return 1
    var ttl = 3600
    if args.len > 4 and args[4] == "--ttl-seconds" and args.len > 5:
      ttl = parseInt(args[5])
    let payload = normalizeJson(renewLease(args[1], args[2], args[3], max(60, ttl)))
    if "--json" in args: echo pretty(payload) else: echo renderToon(payload)
    return 0

  of "release":
    if args.len < 4:
      echo "Usage: taskflow-v0 lease release <resource_type> <resource_id> <holder>"
      return 1
    let payload = normalizeJson(releaseLease(args[1], args[2], args[3]))
    if "--json" in args: echo pretty(payload) else: echo renderToon(payload)
    return 0

  of "list":
    let payload = normalizeJson(activeLeases())
    if "--json" in args: echo pretty(payload) else: echo renderToon(payload)
    return 0

  else:
    echo "Unknown lease subcommand: " & args[0]
    return 1
