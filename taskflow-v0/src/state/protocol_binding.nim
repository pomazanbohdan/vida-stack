## VIDA Protocol Binding Runtime Surface — script-era DB-backed protocol-binding bridge.

import std/[json, os, strutils]
import ../core/[config, turso_task_store, utils]

const
  InvalidSeedReason = "invalid_seed_payload"

proc protocolBindingSeedPath*(): string =
  vidaRoot() / "taskflow-v0" / "config" / "protocol_binding.seed.json"

proc protocolBindingSnapshotPath*(): string =
  vidaWorkspacePath("state", "protocol-binding", "latest.json")

proc protocolBindingCompiledPath*(): string =
  vidaRoot() / "taskflow-v0" / "generated" / "protocol_binding.compiled.json"

proc protocolBindingInstructionLines*(payload: JsonNode): seq[string] =
  if payload.kind == JObject and payload.hasKey("remediation_commands") and payload["remediation_commands"].kind == JArray:
    for item in payload["remediation_commands"]:
      let text = policyValue(item, "")
      if text.len > 0:
        result.add(text)

proc collectRowBlockers(row: JsonNode): seq[string] =
  if row.kind != JObject or not row.hasKey("blockers") or row["blockers"].kind != JArray:
    return @[]
  for blocker in row["blockers"]:
    let text = policyValue(blocker, "")
    if text.len > 0:
      result.add(text)

proc buildProtocolBindingPayload*(): JsonNode =
  let seedPath = protocolBindingSeedPath()
  if not fileExists(seedPath):
    return %*{
      "status": "error",
      "ok": false,
      "reason": "missing_seed_payload",
      "seed_path": seedPath,
      "remediation_commands": [],
      "bindings": [],
    }

  let seedPayload = loadJson(seedPath, newJObject())
  let scenario = policyValue(seedPayload{"scenario"}, "")
  let primaryStateAuthority = policyValue(seedPayload{"primary_state_authority"}, "")
  let protocolIndexPath = vidaRoot() / "vida" / "config" / "instructions" / "system-maps" / "protocol.index.md"
  let protocolIndexExists = fileExists(protocolIndexPath)
  let protocolIndex = if protocolIndexExists: readFile(protocolIndexPath) else: ""
  var bindings = newJArray()
  var ok = true

  if seedPayload.kind != JObject or not seedPayload.hasKey("bindings") or seedPayload["bindings"].kind != JArray:
    return %*{
      "status": "error",
      "ok": false,
      "reason": InvalidSeedReason,
      "seed_path": seedPath,
      "remediation_commands": protocolBindingInstructionLines(seedPayload),
      "bindings": [],
    }

  if scenario.len == 0 or primaryStateAuthority.len == 0:
    return %*{
      "status": "error",
      "ok": false,
      "reason": InvalidSeedReason,
      "seed_path": seedPath,
      "remediation_commands": protocolBindingInstructionLines(seedPayload),
      "bindings": [],
    }

  for seed in seedPayload["bindings"]:
    var blockers: seq[string] = @[]
    let protocolId = policyValue(seed{"protocol_id"}, "")
    let sourcePath = policyValue(seed{"source_path"}, "")
    if protocolId.len == 0:
      blockers.add("missing_protocol_id")
    if sourcePath.len == 0:
      blockers.add("missing_source_path_field")
    if sourcePath.len > 0 and not fileExists(vidaRoot() / sourcePath):
      blockers.add("missing_source_path:" & sourcePath)
    if not protocolIndexExists:
      blockers.add("missing_protocol_index:" & protocolIndexPath)
    elif sourcePath.len > 0 and not protocolIndex.contains(sourcePath):
      blockers.add("missing_protocol_index_binding:" & protocolId)

    let bindingStatus =
      if blockers.len == 0: policyValue(seed{"binding_status"}, "script-bound")
      else: "unbound"
    if blockers.len > 0:
      ok = false

    bindings.add(%*{
      "protocol_id": protocolId,
      "source_path": sourcePath,
      "activation_class": policyValue(seed{"activation_class"}, ""),
      "runtime_owner": policyValue(seed{"runtime_owner"}, ""),
      "enforcement_type": policyValue(seed{"enforcement_type"}, ""),
      "proof_surface": policyValue(seed{"proof_surface"}, ""),
      "primary_state_authority": primaryStateAuthority,
      "binding_status": bindingStatus,
      "active": true,
      "blockers": blockers,
      "scenario": scenario,
      "synced_at": "",
    })

  %*{
    "status": "ok",
    "ok": ok,
    "scenario": scenario,
    "primary_state_authority": primaryStateAuthority,
    "seed_path": seedPath,
    "protocol_index_path": protocolIndexPath,
    "materialization_owner": "taskflow-v0/src/state/protocol_binding.nim",
    "remediation_commands": protocolBindingInstructionLines(seedPayload),
    "bindings": bindings,
    "binding_count": bindings.len,
  }

proc protocolBindingExpectedCount*(): int =
  let payload = buildProtocolBindingPayload()
  if payload.kind != JObject or not payload.hasKey("bindings") or payload["bindings"].kind != JArray:
    return 0
  payload["bindings"].len

proc writeProtocolBindingPayload*(path: string): JsonNode =
  let payload = normalizeJson(buildProtocolBindingPayload())
  saveJson(path, payload)
  %*{
    "status": "ok",
    "ok": dottedGetBool(payload, "ok", false),
    "output_path": path,
    "build": payload,
  }

proc protocolBindingSyncPayload*(sourcePath = ""): JsonNode =
  let effectiveSource =
    if sourcePath.len > 0: sourcePath
    else: protocolBindingCompiledPath()
  var buildPayload = newJNull()
  if sourcePath.len == 0 or not fileExists(effectiveSource):
    let buildResult = writeProtocolBindingPayload(effectiveSource)
    buildPayload = buildResult{"build"}
  else:
    buildPayload = loadJson(effectiveSource, newJObject())
    if buildPayload.kind == JObject and not buildPayload.hasKey("bindings") and buildPayload.hasKey("build"):
      buildPayload = buildPayload{"build"}

  let dbPayload = runTaskStore(@["protocol-binding-sync", effectiveSource])
  let resultPayload = %*{
    "status": "ok",
    "ok": dottedGetBool(dbPayload, "ok", false) and dottedGetBool(buildPayload, "ok", false),
    "build": buildPayload,
    "db_sync": dbPayload,
    "compiled_path": effectiveSource,
    "snapshot_path": protocolBindingSnapshotPath(),
  }
  saveJson(protocolBindingSnapshotPath(), normalizeJson(resultPayload))
  resultPayload

proc protocolBindingStatusPayload*(includeRows = false): JsonNode =
  let buildPayload = buildProtocolBindingPayload()
  let dbPayload = runTaskStore(@["protocol-binding-status"] & (if includeRows: @["--rows"] else: @[]))
  %*{
    "status": "ok",
    "ok": dottedGetBool(dbPayload, "ok", false) and dottedGetBool(buildPayload, "ok", false),
    "build": {
      "ok": dottedGetBool(buildPayload, "ok", false),
      "seed_path": dottedGetStr(buildPayload, "seed_path"),
      "binding_count": dottedGetInt(buildPayload, "binding_count"),
      "primary_state_authority": dottedGetStr(buildPayload, "primary_state_authority"),
    },
    "db": dbPayload,
    "snapshot_path": protocolBindingSnapshotPath(),
  }

proc protocolBindingCheckPayload*(): JsonNode =
  let buildPayload = buildProtocolBindingPayload()
  let expectedCount = protocolBindingExpectedCount()
  let requiredAuthority = policyValue(buildPayload{"primary_state_authority"}, "")
  var dbPayload = runTaskStore(@[
    "protocol-binding-check",
    "--expected-count", $expectedCount,
    "--required-authority", requiredAuthority,
  ])
  var blockingReasons: seq[string] = @[]
  if dbPayload.kind == JObject and dbPayload.hasKey("blocking_reasons") and dbPayload["blocking_reasons"].kind == JArray:
    for reason in dbPayload["blocking_reasons"]:
      let text = policyValue(reason, "")
      if text.len > 0:
        blockingReasons.add(text)
  if not dottedGetBool(buildPayload, "ok", false):
    blockingReasons.add("invalid_or_missing_protocol_binding_seed")
    if buildPayload.kind == JObject and buildPayload.hasKey("bindings") and buildPayload["bindings"].kind == JArray:
      for row in buildPayload["bindings"]:
        for blocker in collectRowBlockers(row):
          blockingReasons.add(policyValue(row{"protocol_id"}, "unknown") & ":" & blocker)

  %*{
    "status": "ok",
    "ok": dottedGetBool(dbPayload, "ok", false) and dottedGetBool(buildPayload, "ok", false),
    "expected_count": expectedCount,
    "required_primary_state_authority": requiredAuthority,
    "build": buildPayload,
    "db": dbPayload,
    "blocking_reasons": blockingReasons,
    "remediation_commands": protocolBindingInstructionLines(buildPayload),
    "snapshot_path": protocolBindingSnapshotPath(),
  }

proc protocolBindingReady*(): bool =
  dottedGetBool(protocolBindingCheckPayload(), "ok", false)

proc protocolBindingRemediationMessage*(payload: JsonNode): string =
  var lines = @[
    "Protocol-binding runtime state is missing or invalid.",
  ]
  let dbPath = dottedGetStr(payload, "db.db_path")
  if dbPath.len > 0:
    lines.add("DB path: " & dbPath)
  let seedPath = dottedGetStr(payload, "build.seed_path")
  if seedPath.len > 0:
    lines.add("Seed payload: " & seedPath)
  let compiledPath = dottedGetStr(payload, "compiled_path")
  if compiledPath.len > 0:
    lines.add("Compiled payload: " & compiledPath)
  if payload.kind == JObject and payload.hasKey("blocking_reasons") and payload["blocking_reasons"].kind == JArray:
    for reason in payload["blocking_reasons"]:
      let text = policyValue(reason, "")
      if text.len > 0:
        lines.add("Blocker: " & text)
  for command in protocolBindingInstructionLines(payload{"build"}):
    lines.add("Run: " & command)
  lines.join("\n")

proc enforceProtocolBinding*(): int =
  let payload = protocolBindingCheckPayload()
  if dottedGetBool(payload, "ok", false):
    return 0
  echo protocolBindingRemediationMessage(payload)
  1

proc printProtocolBindingSummary(payload: JsonNode) =
  echo "Protocol binding:"
  echo "  seed_ok: " & $dottedGetBool(payload, "build.ok", false)
  echo "  binding_count: " & $dottedGetInt(payload, "build.binding_count", 0)
  echo "  authority: " & dottedGetStr(payload, "build.primary_state_authority")
  echo "  db_ok: " & $dottedGetBool(payload, "db.ok", false)
  echo "  snapshot: " & dottedGetStr(payload, "snapshot_path", protocolBindingSnapshotPath())
  let receiptId = dottedGetStr(payload, "db.receipt.receipt_id")
  if receiptId.len > 0:
    echo "  receipt: " & receiptId
    echo "  total_bindings: " & $dottedGetInt(payload, "db.receipt.total_bindings", 0)
    echo "  unbound_count: " & $dottedGetInt(payload, "db.receipt.unbound_count", 0)
    echo "  blocking_issue_count: " & $dottedGetInt(payload, "db.receipt.blocking_issue_count", 0)

proc cmdProtocolBinding*(args: seq[string]): int =
  if args.len == 0:
    echo """Usage:
  taskflow-v0 protocol-binding build [--json]
  taskflow-v0 protocol-binding sync [--json]
  taskflow-v0 protocol-binding status [--json]
  taskflow-v0 protocol-binding check [--json]"""
    return 1

  let asJson = "--json" in args

  case args[0]
  of "build":
    let payload = normalizeJson(writeProtocolBindingPayload(protocolBindingCompiledPath()))
    if asJson:
      echo pretty(payload)
    else:
      printProtocolBindingSummary(payload{"build"})
      echo "Compiled payload: " & dottedGetStr(payload, "output_path", protocolBindingCompiledPath())
    return (if dottedGetBool(payload, "ok", false): 0 else: 1)
  of "sync":
    let payload = normalizeJson(protocolBindingSyncPayload())
    if asJson:
      echo pretty(payload)
    else:
      printProtocolBindingSummary(payload)
    return (if dottedGetBool(payload, "ok", false): 0 else: 1)
  of "status":
    let payload = normalizeJson(protocolBindingStatusPayload(includeRows = asJson))
    if asJson:
      echo pretty(payload)
    else:
      printProtocolBindingSummary(payload)
    return 0
  of "check":
    let payload = normalizeJson(protocolBindingCheckPayload())
    if asJson:
      echo pretty(payload)
    else:
      if dottedGetBool(payload, "ok", false):
        printProtocolBindingSummary(payload)
      else:
        echo protocolBindingRemediationMessage(payload)
    return (if dottedGetBool(payload, "ok", false): 0 else: 1)
  else:
    echo "Unknown protocol-binding subcommand: " & args[0]
    return 1
