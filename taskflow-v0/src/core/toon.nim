import std/[json, os, osproc, strutils, times]
import ./[config, utils]

const CompileTimeDefaultToonPython = currentSourcePath().parentDir().parentDir().parentDir().parentDir().parentDir() / ".venv" / "bin" / "python3"
const CompileTimeToonHelperPath = currentSourcePath().parentDir().parentDir().parentDir() / "helpers" / "toon_render.py"

proc toonPython*(): string =
  let overridePath = getEnv("VIDA_V0_TOON_PYTHON")
  if overridePath.len > 0:
    return overridePath
  let runtimePython = vidaRoot() / ".venv" / "bin" / "python3"
  if fileExists(runtimePython):
    return runtimePython
  if fileExists(CompileTimeDefaultToonPython):
    return CompileTimeDefaultToonPython
  return "python3"

proc toonHelperPath*(): string =
  let runtimeHelperPath = vidaRoot() / "taskflow-v0" / "helpers" / "toon_render.py"
  if fileExists(runtimeHelperPath):
    return runtimeHelperPath
  CompileTimeToonHelperPath

proc renderToon*(payload: JsonNode): string =
  let uniqueStamp = safeName($epochTime(), "tmp").replace(".", "-")
  let tempPath = getTempDir() / ("taskflow-v0-toon-" & safeName(nowUtc(), "tmp") & "-" & uniqueStamp & ".json")
  writeFile(tempPath, $normalizeJson(payload) & "\n")
  try:
    execProcess(
      toonPython(),
      args = @[toonHelperPath(), tempPath],
      options = {poUsePath, poStdErrToStdOut}
    ).strip()
  finally:
    if fileExists(tempPath):
      removeFile(tempPath)
