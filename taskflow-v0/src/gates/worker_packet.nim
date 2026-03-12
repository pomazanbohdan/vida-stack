## VIDA Worker Packet Gate — validate worker packets and machine-readable outputs.
##
## Replaces `worker-packet-gate.py` (234 lines).
## Validates packet text markers, blocking questions, machine-readable
## contract examples, and worker output JSON payloads.

import std/[json, strutils, sets, tables]
import ../core/[toon, utils]

# ─────────────────────────── Constants ───────────────────────────

const WorkerEntryDoc* = "vida/config/instructions/agent-definitions/entry.worker-entry.md"
const WorkerThinkingDoc* = "vida/config/instructions/instruction-contracts/role.worker-thinking.md"

let RequiredPacketMarkers* = {
  "worker_lane_confirmed: true": "missing worker_lane_confirmed marker",
  "worker_role: worker": "missing worker_role marker",
  "worker_entry: " & WorkerEntryDoc: "missing worker_entry marker",
  "worker_thinking: " & WorkerThinkingDoc: "missing worker_thinking marker",
  "impact_tail_policy: required_for_non_stc": "missing impact_tail_policy marker",
  "impact_analysis_scope: bounded_to_assigned_scope": "missing impact_analysis_scope marker",
}.toTable

const PlaceholderPreflight* = "<active project preflight doc from overlay>"
const PlaceholderBlockingQuestion* = "[provide one explicit blocking question for this worker lane]"

const TopLevelOutputKeys* = [
  "status", "question_answered", "answer", "evidence_refs",
  "changed_files", "verification_commands", "verification_results",
  "merge_ready", "blockers", "notes", "recommended_next_action",
  "impact_analysis",
].toHashSet

const ImpactAnalysisKeys* = [
  "affected_scope", "contract_impact", "follow_up_actions", "residual_risks",
].toHashSet

const YesNoValues* = ["yes", "no"].toHashSet
const StatusValues* = ["done", "partial", "blocked"].toHashSet

# ─────────────────────────── Helpers ───────────────────────────

proc isStringList(node: JsonNode): bool =
  if node.isNil or node.kind != JArray: return false
  for item in node:
    if item.kind != JString: return false
  return true

proc machineReadableContractRequired*(text: string): bool =
  "return the machine-readable summary below" in text.toLowerAscii()

proc hasLineStartingWith(text: string, prefix: string, requireContent: bool = false): bool =
  ## Check if any line in text starts with the given prefix.
  ## If requireContent=true, require non-whitespace after the prefix.
  for line in text.splitLines():
    let trimmed = line.strip()
    if trimmed.startsWith(prefix):
      if not requireContent:
        return true
      let rest = trimmed[prefix.len..^1].strip()
      if rest.len > 0:
        return true
  return false

proc extractJsonBlocks(text: string): seq[string] =
  ## Extract JSON from ```json ... ``` fenced code blocks.
  var blocks: seq[string] = @[]
  var inBlock = false
  var current = ""
  for line in text.splitLines():
    if not inBlock:
      if line.strip().startsWith("```json"):
        inBlock = true
        current = ""
    else:
      if line.strip() == "```":
        inBlock = false
        if current.strip().len > 0:
          blocks.add(current.strip())
      else:
        current.add(line & "\n")
  return blocks

proc extractBalancedJsonObjects(text: string): seq[string] =
  ## Extract balanced top-level JSON objects from arbitrary prose.
  var start = -1
  var depth = 0
  var inString = false
  var escape = false
  for i, ch in text:
    if start < 0:
      if ch == '{':
        start = i
        depth = 1
        inString = false
        escape = false
      continue

    if inString:
      if escape:
        escape = false
      elif ch == '\\':
        escape = true
      elif ch == '"':
        inString = false
      continue

    case ch
    of '"':
      inString = true
    of '{':
      inc depth
    of '}':
      dec depth
      if depth == 0:
        let candidate = text[start..i].strip()
        if candidate.len > 0:
          result.add(candidate)
        start = -1
    else:
      discard

proc extractJsonPayload*(text: string): JsonNode =
  ## Extract best-matching JSON from text (fenced code blocks or raw JSON).
  let stripped = text.strip()
  if stripped.len == 0: return nil

  var bestPayload: JsonNode = nil
  var bestScore = -1

  # Try fenced json blocks
  for jsonBlock in extractJsonBlocks(text):
    try:
      let payload = parseJson(jsonBlock)
      if payload.kind == JObject:
        var overlap = 0
        for key in payload.keys:
          if key in TopLevelOutputKeys: overlap += 1
        let score = overlap * 1000 + jsonBlock.len
        if score > bestScore:
          bestPayload = payload
          bestScore = score
    except: discard

  # Try the whole text as JSON
  if stripped.startsWith("{") and stripped.endsWith("}"):
    try:
      let payload = parseJson(stripped)
      if payload.kind == JObject:
        var overlap = 0
        for key in payload.keys:
          if key in TopLevelOutputKeys: overlap += 1
        let score = overlap * 1000 + stripped.len
        if score > bestScore:
          bestPayload = payload
          bestScore = score
    except: discard

  # Try balanced JSON objects embedded inside prose
  for candidate in extractBalancedJsonObjects(text):
    try:
      let payload = parseJson(candidate)
      if payload.kind == JObject:
        var overlap = 0
        for key in payload.keys:
          if key in TopLevelOutputKeys: overlap += 1
        let score = overlap * 1000 + candidate.len
        if score > bestScore:
          bestPayload = payload
          bestScore = score
    except: discard

  return bestPayload

# ─────────────────────────── Validation ───────────────────────────

proc validateOutputPayload*(payload: JsonNode, prefix: string): seq[string] =
  for key in TopLevelOutputKeys:
    if not payload.hasKey(key):
      result.add(prefix & " missing key: " & key)

  if payload.hasKey("answer") and payload["answer"].kind != JString:
    result.add(prefix & " answer must be a string")
  if payload.hasKey("notes") and payload["notes"].kind != JString:
    result.add(prefix & " notes must be a string")
  if payload.hasKey("recommended_next_action") and payload["recommended_next_action"].kind != JString:
    result.add(prefix & " recommended_next_action must be a string")

  for key in ["evidence_refs", "changed_files", "verification_commands",
              "verification_results", "blockers"]:
    if payload.hasKey(key) and not isStringList(payload[key]):
      result.add(prefix & " " & key & " must be a list of strings")

  let impact = payload{"impact_analysis"}
  if not impact.isNil:
    if impact.kind == JObject:
      for key in ImpactAnalysisKeys:
        if not impact.hasKey(key):
          result.add(prefix & " impact_analysis missing key: " & key)
        elif not isStringList(impact[key]):
          result.add(prefix & " impact_analysis " & key & " must be a list of strings")
    else:
      result.add(prefix & " impact_analysis must be an object")

  let status = payload{"status"}
  if not status.isNil and status.kind == JString and status.getStr() notin StatusValues:
    result.add(prefix & " status must be one of [blocked, done, partial]")
  let qa = payload{"question_answered"}
  if not qa.isNil and qa.kind == JString and qa.getStr() notin YesNoValues:
    result.add(prefix & " question_answered must be one of [no, yes]")
  let mr = payload{"merge_ready"}
  if not mr.isNil and mr.kind == JString and mr.getStr() notin YesNoValues:
    result.add(prefix & " merge_ready must be one of [no, yes]")

proc validatePacketText*(text: string): seq[string] =
  for marker, message in RequiredPacketMarkers:
    if marker notin text:
      result.add(message)
  if "Runtime Role Packet:" notin text:
    result.add("missing Runtime Role Packet section")
  if not hasLineStartingWith(text, "Scope:", requireContent = true):
    result.add("missing Scope section")
  if not hasLineStartingWith(text, "Verification:"):
    result.add("missing Verification section")
  if not hasLineStartingWith(text, "Deliverable:"):
    result.add("missing Deliverable section")

  # Check Blocking Question
  var foundBQ = false
  for line in text.splitLines():
    let trimmed = line.strip()
    if trimmed.startsWith("Blocking Question:"):
      foundBQ = true
      let question = trimmed["Blocking Question:".len..^1].strip()
      if question.len == 0 or question == PlaceholderBlockingQuestion:
        result.add("blocking question must be explicit and non-placeholder")
      break
  if not foundBQ:
    result.add("missing Blocking Question section")

  if PlaceholderPreflight in text:
    result.add("project preflight placeholder must be resolved from overlay")

  if machineReadableContractRequired(text):
    let payload = extractJsonPayload(text)
    if payload.isNil:
      result.add("missing machine-readable contract example")
    else:
      result.add(validateOutputPayload(payload, "machine-readable contract"))

proc validateOutputText*(promptText, outputText: string): seq[string] =
  if not machineReadableContractRequired(promptText):
    return @[]
  let payload = extractJsonPayload(outputText)
  if payload.isNil:
    return @["worker output must be valid JSON when the prompt requires a machine-readable summary"]
  return validateOutputPayload(payload, "machine-readable output")

# ─────────────────────────── CLI ───────────────────────────

proc cmdWorkerPacket*(args: seq[string]): int =
  if args.len < 2:
    echo """Usage:
  taskflow-v0 worker check <prompt_file|->
  taskflow-v0 worker check-output <prompt_file|-> <output_file|->"""
    return 1

  case args[0]
  of "check":
    let text = if args[1] == "-": stdin.readAll() else: readFile(args[1])
    let errors = validatePacketText(text)
    let payload = normalizeJson(%*{"status": (if errors.len == 0: "ok" else: "blocked"), "errors": errors})
    if "--json" in args: echo pretty(payload) else: echo renderToon(payload)
    return if errors.len == 0: 0 else: 2

  of "check-output":
    if args.len < 3:
      echo "Usage: taskflow-v0 worker check-output <prompt_file> <output_file>"; return 1
    let promptText = if args[1] == "-": stdin.readAll() else: readFile(args[1])
    let outputText = if args[2] == "-": stdin.readAll() else: readFile(args[2])
    let errors = validateOutputText(promptText, outputText)
    let payload = normalizeJson(%*{"status": (if errors.len == 0: "ok" else: "blocked"), "errors": errors})
    if "--json" in args: echo pretty(payload) else: echo renderToon(payload)
    return if errors.len == 0: 0 else: 2

  else:
    echo "Unknown worker subcommand: " & args[0]; return 1
