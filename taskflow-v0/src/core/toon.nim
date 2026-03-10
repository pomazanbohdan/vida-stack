import std/[json, os, osproc, strutils, times]
import ./utils

const DefaultToonPython = currentSourcePath().parentDir().parentDir().parentDir().parentDir().parentDir() / ".venv" / "bin" / "python3"
const ToonHelperPath = currentSourcePath().parentDir().parentDir().parentDir() / "helpers" / "toon_render.py"

proc toonPython*(): string =
  let overridePath = getEnv("VIDA_V0_TOON_PYTHON")
  if overridePath.len > 0:
    return overridePath
  if fileExists(DefaultToonPython):
    return DefaultToonPython
  return "python3"

proc renderToon*(payload: JsonNode): string =
  let uniqueStamp = safeName($epochTime(), "tmp").replace(".", "-")
  let tempPath = getTempDir() / ("taskflow-v0-toon-" & safeName(nowUtc(), "tmp") & "-" & uniqueStamp & ".json")
  writeFile(tempPath, $normalizeJson(payload) & "\n")
  try:
    execProcess(
      toonPython(),
      args = @[ToonHelperPath, tempPath],
      options = {poUsePath, poStdErrToStdOut}
    ).strip()
  finally:
    if fileExists(tempPath):
      removeFile(tempPath)
