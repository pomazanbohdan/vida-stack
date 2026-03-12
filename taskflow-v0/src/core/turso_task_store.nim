import std/[json, os, osproc, strutils]
import ./config

const CompileTimeHelperPath = currentSourcePath().parentDir().parentDir().parentDir() / "helpers" / "turso_task_store.py"

proc v0TaskDbPath*(): string =
  vidaRoot() / ".vida" / "state" / "taskflow-state.db"

proc tursoPython*(): string =
  let overridePath = getEnv("VIDA_V0_TURSO_PYTHON")
  if overridePath.len > 0:
    return overridePath
  let runtimeVenvPython = vidaRoot() / ".venv" / "bin" / "python3"
  if fileExists(runtimeVenvPython):
    return runtimeVenvPython
  return "python3"

proc helperPath*(): string =
  let runtimeHelperPath = vidaRoot() / "taskflow-v0" / "helpers" / "turso_task_store.py"
  if fileExists(runtimeHelperPath):
    return runtimeHelperPath
  CompileTimeHelperPath

proc runTaskStore*(args: seq[string]): JsonNode =
  let output = execProcess(
    tursoPython(),
    args = @[helperPath(), "--db", v0TaskDbPath()] & args,
    workingDir = vidaRoot(),
    options = {poUsePath, poStdErrToStdOut}
  ).strip()
  if output.len == 0:
    return newJNull()
  try:
    return parseJson(output)
  except JsonParsingError:
    return %*{"status": "error", "reason": "invalid_helper_output", "output": output}
