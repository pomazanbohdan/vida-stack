import std/[json, os, osproc, strutils]
import ./config

const DefaultTursoPython = currentSourcePath().parentDir().parentDir().parentDir().parentDir().parentDir() / ".venv" / "bin" / "python3"
const HelperPath = currentSourcePath().parentDir().parentDir().parentDir() / "helpers" / "turso_task_store.py"

proc legacyTaskDbPath*(): string =
  vidaRoot() / ".vida" / "state" / "vida-legacy.db"

proc tursoPython*(): string =
  let overridePath = getEnv("VIDA_LEGACY_TURSO_PYTHON")
  if overridePath.len > 0:
    return overridePath
  if fileExists(DefaultTursoPython):
    return DefaultTursoPython
  return "python3"

proc helperPath*(): string =
  HelperPath

proc runTaskStore*(args: seq[string]): JsonNode =
  let output = execProcess(
    tursoPython(),
    args = @[helperPath(), "--db", legacyTaskDbPath()] & args,
    workingDir = vidaRoot(),
    options = {poUsePath, poStdErrToStdOut}
  ).strip()
  if output.len == 0:
    return newJNull()
  try:
    return parseJson(output)
  except JsonParsingError:
    return %*{"status": "error", "reason": "invalid_helper_output", "output": output}
