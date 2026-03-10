import std/[json, os, strutils]

const ScriptDir = currentSourcePath().parentDir()
const CompileTimeRoot = ScriptDir.parentDir().parentDir().parentDir().parentDir()

type
  YamlLine = object
    indent: int
    key: string
    value: string
    isListItem: bool
    raw: string

proc loadDotEnv(dir: string): string =
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
  ""

proc vidaRoot*(): string =
  let envRoot = getEnv("VIDA_ROOT")
  if envRoot.len > 0:
    return envRoot
  let binDir = getAppDir()
  let dotEnvRoot = loadDotEnv(binDir)
  if dotEnvRoot.len > 0:
    return dotEnvRoot
  let cwdRoot = loadDotEnv(getCurrentDir())
  if cwdRoot.len > 0:
    return cwdRoot
  CompileTimeRoot

proc vidaWorkspaceDir*(): string =
  vidaRoot() / ".vida"

proc vidaWorkspacePath*(parts: varargs[string]): string =
  result = vidaWorkspaceDir()
  for part in parts:
    result = result / part

proc configPath*(): string =
  vidaRoot() / "vida.config.yaml"

proc configExists*(): bool =
  fileExists(configPath())

proc parseYamlSubset*(content: string): JsonNode

proc loadRawConfig*(): JsonNode =
  let path = configPath()
  if not fileExists(path):
    return newJObject()
  try:
    parseYamlSubset(readFile(path))
  except:
    newJObject()

proc parseYamlLine(line: string): YamlLine =
  result.raw = line
  let stripped = line.strip(trailing = false)
  result.indent = line.len - stripped.len

  var content = stripped
  if content.startsWith("#") or content.len == 0:
    return

  if content.startsWith("- "):
    result.isListItem = true
    content = content[2..^1].strip()

  let colonPos = content.find(':')
  if colonPos >= 0:
    let beforeColon = content[0..<colonPos]
    if ' ' notin beforeColon or beforeColon.startsWith("\""):
      result.key = beforeColon.strip().strip(chars = {'"', '\''})
      if colonPos + 1 < content.len:
        result.value = content[colonPos + 1..^1].strip()
      return

  result.value = content

proc yamlScalarToJson(value: string): JsonNode =
  if value.len == 0:
    return newJNull()
  if value == "[]":
    return newJArray()
  if value == "{}":
    return newJObject()
  if (value.startsWith("\"") and value.endsWith("\"")) or
     (value.startsWith("'") and value.endsWith("'")):
    return newJString(value[1..^2])
  let lower = value.toLowerAscii()
  if lower in ["true", "yes", "on"]:
    return newJBool(true)
  if lower in ["false", "no", "off"]:
    return newJBool(false)
  if lower in ["null", "~", ""]:
    return newJNull()
  try:
    return newJInt(parseInt(value))
  except ValueError:
    discard
  try:
    return newJFloat(parseFloat(value))
  except ValueError:
    discard
  newJString(value)

proc parseYamlSubset*(content: string): JsonNode =
  let lines = content.splitLines()
  var lineInfos: seq[YamlLine] = @[]
  for line in lines:
    let trimmed = line.strip()
    if trimmed.len == 0 or trimmed.startsWith("#"):
      continue
    lineInfos.add(parseYamlLine(line))

  proc parseLevel(start: int, minIndent: int): (JsonNode, int) =
    if start >= lineInfos.len:
      return (newJObject(), lineInfos.len)

    let firstLine = lineInfos[start]
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
            var obj = newJObject()
            obj[line.key] = yamlScalarToJson(line.value)
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
            arr.add(yamlScalarToJson(line.value))
            i += 1
        else:
          i += 1
      return (arr, i)

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
        obj[line.key] = yamlScalarToJson(line.value)
        i += 1
      else:
        if i + 1 < lineInfos.len and lineInfos[i + 1].indent > line.indent:
          let (subNode, nextI) = parseLevel(i + 1, lineInfos[i + 1].indent)
          obj[line.key] = subNode
          i = nextI
        else:
          obj[line.key] = newJNull()
          i += 1
    (obj, i)

  let (root, _) = parseLevel(0, 0)
  root
