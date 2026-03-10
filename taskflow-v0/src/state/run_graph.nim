## VIDA Run Graph — durable execution graph ledger.
##
## Replaces `run-graph.py` (200 lines).
## Tracks execution stages (analysis→writer→coach→verifier→approval→synthesis)
## with typed status transitions and resume hints.

import std/[json, os]
import ../core/[utils, config, toon]

# ─────────────────────────── Constants ───────────────────────────

const DefaultNodes* = @[
  "analysis", "writer", "coach", "problem_party",
  "verifier", "approval", "synthesis"
]

const AllowedStatus* = [
  "pending", "ready", "running", "completed",
  "blocked", "failed", "skipped"
]

const ResumePriority* = DefaultNodes  # same order

# ─────────────────────────── Paths ───────────────────────────

proc runGraphStateDir*(): string =
  let envDir = getEnv("VIDA_RUN_GRAPH_STATE_DIR")
  if envDir.len > 0:
    return envDir
  vidaRoot() / ".vida" / "state" / "run-graphs"

proc graphPath*(taskId: string): string =
  runGraphStateDir() / (taskId & ".json")

# ─────────────────────────── Load / Write ───────────────────────────

proc loadGraph*(taskId: string): JsonNode =
  let path = graphPath(taskId)
  if not fileExists(path):
    return newJObject()
  try:
    let payload = loadJson(path)
    if payload.kind == JObject:
      return payload
    return newJObject()
  except:
    return newJObject()

proc writeGraph*(taskId: string, payload: JsonNode): string =
  let path = graphPath(taskId)
  createDir(path.parentDir())
  saveJson(path, payload)
  return path

# ─────────────────────────── Ensure / Init ───────────────────────────

proc ensureGraph*(taskId, taskClass: string,
                  routeTaskClass: string = ""): JsonNode =
  let existing = loadGraph(taskId)
  let existingNodes = existing{"nodes"}
  var nodes = newJObject()

  # Preserve existing valid nodes
  if not existingNodes.isNil and existingNodes.kind == JObject:
    for name, value in existingNodes:
      if value.kind == JObject:
        nodes[name] = value

  # Ensure all default nodes exist
  for node in DefaultNodes:
    if not nodes.hasKey(node):
      nodes[node] = %*{
        "status": "pending",
        "updated_at": nowUtc(),
        "attempts": 0,
        "meta": {},
      }

  let rtc = if routeTaskClass.len > 0: routeTaskClass
    else: policyValue(existing{"route_task_class"}, "")

  result = %*{
    "task_id": taskId,
    "task_class": taskClass,
    "route_task_class": rtc,
    "updated_at": nowUtc(),
    "nodes": nodes,
  }

# ─────────────────────────── Update Node ───────────────────────────

proc updateNode*(taskId, taskClass, node, status: string,
                 routeTaskClass: string = "",
                 meta: JsonNode = newJObject()): string =
  if node notin DefaultNodes:
    raise newException(ValueError, "invalid_node:" & node)
  if status notin AllowedStatus:
    raise newException(ValueError, "invalid_status:" & status)

  var payload = ensureGraph(taskId, taskClass, routeTaskClass)
  var entry = payload["nodes"][node]
  let previousStatus = policyValue(entry{"status"}, "")
  var attempts = policyInt(entry{"attempts"}, 0)
  if status == "running" and previousStatus != "running":
    attempts += 1
  entry["status"] = %status
  entry["updated_at"] = %nowUtc()
  entry["attempts"] = %attempts
  entry["meta"] = meta
  payload["nodes"][node] = entry
  payload["updated_at"] = %nowUtc()
  return writeGraph(taskId, payload)

# ─────────────────────────── Resume Hint ───────────────────────────

proc resumeHint*(payload: JsonNode): JsonNode =
  let nodes = payload{"nodes"}
  if nodes.isNil or nodes.kind != JObject:
    return %*{"next_node": "", "reason": "missing_nodes"}
  for node in ResumePriority:
    if nodes.hasKey(node):
      let entry = nodes[node]
      let status = policyValue(entry{"status"}, "")
      if status in ["blocked", "failed", "running", "ready"]:
        return %*{
          "next_node": node,
          "status": status,
          "reason": policyValue(entry{"meta"}{"reason"}, ""),
        }
  return %*{"next_node": "", "status": "completed", "reason": "no_resumable_node"}

proc statusPayload*(taskId: string): JsonNode =
  let payload = loadGraph(taskId)
  if payload.len == 0 or not payload.hasKey("nodes"):
    return %*{"task_id": taskId, "present": false}
  result = %*{
    "task_id": taskId,
    "present": true,
    "task_class": payload{"task_class"},
    "route_task_class": payload{"route_task_class"},
    "updated_at": payload{"updated_at"},
    "nodes": payload{"nodes"},
    "resume_hint": resumeHint(payload),
  }

# ─────────────────────────── CLI ───────────────────────────

proc cmdRunGraph*(args: seq[string]): int =
  if args.len < 2:
    echo """Usage:
  taskflow-v0 run-graph init <task_id> <task_class> [route_task_class]
  taskflow-v0 run-graph update <task_id> <task_class> <node> <status> [route_task_class] [meta_json]
  taskflow-v0 run-graph status <task_id>"""
    return 2

  let command = args[0]
  let taskId = args[1]

  case command
  of "init":
    if args.len < 3:
      echo "Usage: taskflow-v0 run-graph init <task_id> <task_class> [route_task_class]"
      return 2
    let taskClass = args[2]
    let rtc = if args.len > 3: args[3] else: ""
    let path = writeGraph(taskId, ensureGraph(taskId, taskClass, rtc))
    echo path
    return 0

  of "update":
    if args.len < 5:
      echo "Usage: taskflow-v0 run-graph update <task_id> <task_class> <node> <status> [route_task_class] [meta_json]"
      return 2
    let taskClass = args[2]
    let node = args[3]
    let status = args[4]
    let rtc = if args.len > 5: args[5] else: ""
    var meta = newJObject()
    if args.len > 6:
      try:
        meta = parseJson(args[6])
      except:
        echo "[run-graph] meta_json must be valid JSON"
        return 2
    try:
      let path = updateNode(taskId, taskClass, node, status, rtc, meta)
      echo path
      return 0
    except ValueError as e:
      echo "[run-graph] " & e.msg
      return 2

  of "status":
    let payload = normalizeJson(statusPayload(taskId))
    if "--json" in args:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0

  else:
    echo "Unknown run-graph subcommand: " & command
    return 2
