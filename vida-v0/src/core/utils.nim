## VIDA Core Utilities — single source for all shared helpers.
##
## Replaces the duplicated `now_utc()`, `load_json()`, `save_json()`,
## `policy_value()`, `policy_int()`, `split_csv()` found across 14+ Python files.

import std/[json, os, strutils, times, options]

# ─────────────────────────── Time ───────────────────────────

proc nowUtc*(): string =
  ## ISO 8601 timestamp in UTC with 'Z' suffix.
  ## Replaces the `now_utc()` function duplicated across every Python script.
  now().utc().format("yyyy-MM-dd'T'HH:mm:ss'Z'")

proc nowUtcDt*(): DateTime =
  now().utc()

proc parseUtcTimestamp*(value: string): Option[DateTime] =
  ## Parse ISO 8601 timestamp, returning none on failure.
  let trimmed = value.strip()
  if trimmed.len == 0:
    return none(DateTime)
  try:
    let normalized = trimmed.replace("Z", "+00:00")
    return some(parse(normalized, "yyyy-MM-dd'T'HH:mm:sszzz", utc()))
  except:
    return none(DateTime)

proc futureUtcIso*(minutes: int = 0, hours: int = 0): string =
  let future = nowUtcDt() + initDuration(hours = hours, minutes = minutes)
  future.format("yyyy-MM-dd'T'HH:mm:ss'Z'")

proc nextUtcDayIso*(): string =
  let tomorrow = nowUtcDt() + initDuration(days = 1)
  let dayStart = dateTime(tomorrow.year, tomorrow.month, tomorrow.monthday, 0, 0, 0, zone = utc())
  dayStart.format("yyyy-MM-dd'T'HH:mm:ss'Z'")

proc parseIssueTimestamp*(value: string): float =
  ## Parse timestamp to epoch float, returning 0.0 on failure.
  let dt = parseUtcTimestamp(value)
  if dt.isSome:
    return dt.get().toTime().toUnixFloat()
  return 0.0

# ─────────────────────────── Policy Helpers ───────────────────────────

proc policyValue*(value: JsonNode, default: string): string =
  ## Extract string value with fallback — replaces Python's `policy_value()`.
  if value.isNil or value.kind == JNull:
    return default
  if value.kind == JString:
    let trimmed = value.getStr().strip()
    if trimmed.len == 0: default else: trimmed
  else:
    $value

proc policyValue*(value: string, default: string): string =
  let trimmed = value.strip()
  if trimmed.len == 0: default else: trimmed

proc policyInt*(value: JsonNode, default: int): int =
  ## Extract int value with fallback — replaces Python's `policy_int()`.
  if value.isNil or value.kind == JNull:
    return default
  if value.kind == JInt:
    return value.getInt()
  if value.kind == JString:
    try:
      return parseInt(value.getStr())
    except ValueError:
      return default
  return default

proc policyBool*(value: JsonNode, default: bool): bool =
  if value.isNil or value.kind == JNull:
    return default
  if value.kind == JBool:
    return value.getBool()
  return default

# ─────────────────────────── String Helpers ───────────────────────────

proc splitCsv*(value: JsonNode): seq[string] =
  ## Split comma-separated string or JSON array into seq[string].
  ## Replaces Python's `split_csv()`.
  result = @[]
  if value.isNil or value.kind == JNull:
    return
  if value.kind == JString:
    for item in value.getStr().split(','):
      let trimmed = item.strip()
      if trimmed.len > 0:
        result.add(trimmed)
  elif value.kind == JArray:
    for item in value:
      let text = (if item.kind == JString: item.getStr() else: $item).strip()
      if text.len > 0:
        result.add(text)

proc splitCsv*(value: string): seq[string] =
  result = @[]
  for item in value.split(','):
    let trimmed = item.strip()
    if trimmed.len > 0:
      result.add(trimmed)

proc safeName*(value: string, fallback: string = ""): string =
  ## Normalize a string into a filesystem-safe name.
  var normalized = ""
  for c in value.strip():
    if c in {'A'..'Z', 'a'..'z', '0'..'9', '.', '_', '-'}:
      normalized.add(c)
    else:
      normalized.add('-')
  if normalized.len == 0: fallback else: normalized

proc cleanText*(value: string): string =
  ## Clean text for display — collapse whitespace, replace control chars.
  if value.len == 0:
    return "-"
  var text = value.replace("\n", " ").replace("\r", " ").replace("\t", " ")
  # Collapse multiple spaces
  var prev = false
  var cleaned = ""
  for c in text:
    if c == ' ':
      if not prev:
        cleaned.add(c)
      prev = true
    else:
      cleaned.add(c)
      prev = false
  if cleaned.len == 0: "-" else: cleaned.strip()

# ─────────────────────────── JSON File I/O ───────────────────────────

proc loadJson*(path: string, default: JsonNode = newJObject()): JsonNode =
  ## Load JSON file, returning default on any error.
  ## Replaces Python's `load_json()` pattern.
  if not fileExists(path):
    return default
  try:
    return parseJson(readFile(path))
  except:
    return default

proc saveJson*(path: string, payload: JsonNode) =
  ## Write JSON to file, creating parent directories.
  ## Replaces Python's `save_json()` pattern.
  let dir = parentDir(path)
  if not dirExists(dir):
    createDir(dir)
  writeFile(path, pretty(payload) & "\n")

proc appendJsonl*(path: string, payload: JsonNode) =
  ## Append a JSON line to a JSONL file, creating parent directories.
  let dir = parentDir(path)
  if not dirExists(dir):
    createDir(dir)
  let f = open(path, fmAppend)
  defer: f.close()
  f.writeLine($payload)

proc writeJson*(path: string, payload: JsonNode): string =
  ## Write JSON and return the path.
  saveJson(path, payload)
  return path

proc normalizeJson*(node: JsonNode): JsonNode =
  ## Replace nil JsonNode references with explicit nulls recursively.
  if node.isNil:
    return newJNull()
  case node.kind
  of JObject:
    result = newJObject()
    for key, value in node:
      result[key] = normalizeJson(value)
  of JArray:
    result = newJArray()
    for item in node:
      result.add(normalizeJson(item))
  else:
    return node

# ─────────────────────────── Dotted Get ───────────────────────────

proc dottedGet*(node: JsonNode, path: string, default: JsonNode = newJNull()): JsonNode =
  ## Navigate nested JSON using dot-separated path.
  ## Replaces Python's `vida_config.dotted_get()`.
  if node.isNil or node.kind != JObject:
    return default
  var current = node
  for key in path.split('.'):
    if current.kind != JObject or not current.hasKey(key):
      return default
    current = current[key]
  return current

proc dottedGetStr*(node: JsonNode, path: string, default: string = ""): string =
  policyValue(dottedGet(node, path), default)

proc dottedGetBool*(node: JsonNode, path: string, default: bool = false): bool =
  policyBool(dottedGet(node, path), default)

proc dottedGetInt*(node: JsonNode, path: string, default: int = 0): int =
  policyInt(dottedGet(node, path), default)

# ─────────────────────────── Domain Normalization ───────────────────────────

import std/tables

const DomainTagAliases = {
  "odoo_api": "api_contract",
  "flutter_ui": "frontend_ui",
  "riverpod_state": "state_management",
}.toTable

proc normalizeDomainTag*(tag: string): string =
  let text = tag.strip().toLowerAscii()
  if text in DomainTagAliases:
    DomainTagAliases[text]
  else:
    text

proc normalizeDomainTags*(tags: seq[string]): seq[string] =
  result = @[]
  for tag in tags:
    let normalized = normalizeDomainTag(tag)
    if normalized.len > 0 and normalized notin result:
      result.add(normalized)
