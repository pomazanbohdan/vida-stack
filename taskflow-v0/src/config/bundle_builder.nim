import std/[json, os]
import ./loader

proc resolveConfigRelativePath*(path: string): string =
  if path.len == 0:
    return ""
  if path.isAbsolute:
    return path
  vidaRoot() / path

proc loadYamlRegistry*(path: string): JsonNode =
  if path.len == 0:
    return newJObject()
  let resolved = resolveConfigRelativePath(path)
  if not fileExists(resolved):
    return newJObject()
  try:
    parseYamlSubset(readFile(resolved))
  except:
    newJObject()

proc loadValidatedConfig*(): JsonNode =
  if not configExists():
    return newJObject()
  loadRawConfig()
