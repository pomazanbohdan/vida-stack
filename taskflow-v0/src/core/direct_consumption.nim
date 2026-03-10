## VIDA Direct Runtime Consumption — final taskflow -> codex consumption loop.

import std/[json, os, osproc, strutils]
import ./[config, role_selection, runtime_bundle, toon, utils]

const CodexScriptPath = currentSourcePath().parentDir().parentDir().parentDir().parentDir() / "codex-v0" / "codex.py"

proc runtimeConsumptionDir*(): string =
  vidaWorkspacePath("state", "runtime-consumption")

proc runtimeConsumptionPath*(requestText: string): string =
  runtimeConsumptionDir() / (safeName(requestText, "request") & ".json")

proc codexPython*(): string =
  let envPython = getEnv("VIDA_CODEX_PYTHON")
  if envPython.len > 0:
    return envPython
  let python = findExe("python3")
  if python.len > 0:
    return python
  "python3"

proc runCodexCheck(command: seq[string]): JsonNode =
  putEnv("VIDA_ROOT", vidaRoot())
  var fullCommand = quoteShell(codexPython()) & " " & quoteShell(CodexScriptPath)
  for arg in command:
    fullCommand &= " " & quoteShell(arg)
  let output = execCmdEx(fullCommand, options = {poUsePath, poStdErrToStdOut})
  let normalizedOutput = output.output.strip()
  let outputSignalsError = "ERROR:" in normalizedOutput or "\nERROR:" in normalizedOutput
  %*{
    "command": command.join(" "),
    "exit_code": output.exitCode,
    "ok": output.exitCode == 0 and not outputSignalsError,
    "output": normalizedOutput,
  }

proc finalConsumptionEvidence*(): JsonNode =
  %*{
    "overview": runCodexCheck(@["overview", "--profile", "active-canon"]),
    "readiness": runCodexCheck(@["readiness-check", "--profile", "active-canon"]),
    "proof": runCodexCheck(@["proofcheck", "--profile", "active-canon"]),
  }

proc finalConsumptionPayload*(requestText: string, cfg: JsonNode = loadRawConfig()): JsonNode =
  let bundle = buildRuntimeKernelBundle(cfg)
  let bundleCheck = runtimeKernelBundleReady(bundle)
  let roleSelection = selectAgentRoleForRequest(requestText, cfg)
  let bundleOk = dottedGetBool(bundleCheck, "ok", false)
  let codexEvidence =
    if bundleOk:
      finalConsumptionEvidence()
    else:
      %*{
        "overview": {"ok": false, "exit_code": 2, "output": "skipped_due_to_bundle_failure"},
        "readiness": {"ok": false, "exit_code": 2, "output": "skipped_due_to_bundle_failure"},
        "proof": {"ok": false, "exit_code": 2, "output": "skipped_due_to_bundle_failure"},
      }
  let readinessOk = dottedGetBool(codexEvidence, "readiness.ok", false)
  let proofOk = dottedGetBool(codexEvidence, "proof.ok", false)
  let overviewOk = dottedGetBool(codexEvidence, "overview.ok", false)

  result = %*{
    "artifact_name": "taskflow_direct_runtime_consumption",
    "artifact_type": "runtime_consumption",
    "generated_at": nowUtc(),
    "closure_authority": "taskflow",
    "request_text": requestText,
    "runtime_bundle": bundle,
    "bundle_check": bundleCheck,
    "role_selection": roleSelection,
    "codex_activation": {
      "activated": true,
      "runtime_family": "codex",
      "owner_runtime": "taskflow",
      "evidence": codexEvidence,
    },
    "direct_consumption_ready": bundleOk and readinessOk and proofOk and overviewOk,
  }
  let snapshotPath = runtimeConsumptionPath(requestText)
  result["snapshot_path"] = %snapshotPath
  saveJson(snapshotPath, normalizeJson(result))

proc cmdDirectConsumption*(args: seq[string]): int =
  if args.len < 1:
    echo """Usage:
  taskflow-v0 consume bundle [--json]
  taskflow-v0 consume final <request_text> [--json]"""
    return 1

  let asJson = "--json" in args
  case args[0]
  of "bundle":
    let payload = normalizeJson(buildRuntimeKernelBundle())
    if asJson: echo pretty(payload) else: echo renderToon(payload)
    return 0
  of "final":
    if args.len < 2:
      echo "Usage: taskflow-v0 consume final <request_text> [--json]"
      return 1
    var requestParts: seq[string] = @[]
    for i in 1 ..< args.len:
      if args[i] == "--json":
        continue
      requestParts.add(args[i])
    let requestText = requestParts.join(" ").strip()
    if requestText.len == 0:
      echo "Usage: taskflow-v0 consume final <request_text> [--json]"
      return 1
    let payload = normalizeJson(finalConsumptionPayload(requestText))
    if asJson: echo pretty(payload) else: echo renderToon(payload)
    return (if dottedGetBool(payload, "direct_consumption_ready", false): 0 else: 2)
  else:
    echo "Unknown consume subcommand: " & args[0]
    return 1
