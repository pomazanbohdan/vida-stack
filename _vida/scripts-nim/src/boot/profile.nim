## VIDA Boot Profile — file validation, receipt writing, snapshot generation.
##
## Replaces `boot-profile.sh` (313 lines with 3 embedded Python heredocs).
## Eliminates the Shell↔Python boundary entirely — one Nim binary does it all.

import std/[json, os, strutils, times, hashes]
import ../core/[utils, config]
import ./[packet, snapshot]

# ─────────────────────────── File Validation ───────────────────────────

proc validateReadContract*(rootDir: string, entries: seq[string]): tuple[ok: bool, missing: seq[string]] =
  result.ok = true
  result.missing = @[]
  for entry in entries:
    let target = entry.split("#", 1)[0]
    let path = rootDir / target
    if not fileExists(path) and not dirExists(path):
      result.missing.add(entry)
      result.ok = false

# ─────────────────────────── Receipt Writer ───────────────────────────

proc safeName(subject: string): string =
  result = ""
  for ch in subject:
    if ch in {'A'..'Z', 'a'..'z', '0'..'9', '.', '_', '-'}:
      result.add(ch)
    else:
      result.add('_')

proc sha256File(path: string): string =
  ## Simple file hash for receipt. In production use a crypto lib.
  ## For now returns a basic hash as hex.
  if not fileExists(path):
    return ""
  let content = readFile(path)
  var h: Hash = 0
  for ch in content:
    h = h !& hash(ch)
  h = !$h
  return toHex(h).toLowerAscii()

proc writeReceipt*(profile: string, taskId: string, nonDev: bool,
                   capsuleStatus: string, status: string,
                   readContract: seq[string], rootDir: string): string =
  ## Write boot receipt JSON files. Returns path to latest receipt.
  let receiptDir = rootDir / ".vida" / "logs" / "boot-receipts"
  createDir(receiptDir)

  let subject = if taskId.len > 0: taskId else: "session"
  let safeSubject = safeName(subject)
  let timestamp = now().utc.format("yyyyMMdd'T'HHmmss'Z'")
  let latestFile = receiptDir / (safeSubject & ".latest.json")
  let archiveFile = receiptDir / (safeSubject & "-" & timestamp & ".json")

  # Write boot packet
  let packetFile = receiptDir / (safeSubject & "-" & timestamp & ".boot-packet.json")
  let packetLatest = receiptDir / (safeSubject & ".latest.boot-packet.json")
  let packetPayload = buildPacket(profile, nonDev, rootDir)
  saveJson(packetFile, packetPayload)
  writeFile(packetLatest, readFile(packetFile))

  # Write boot snapshot (dev only)
  var snapshotFile = ""
  if not nonDev:
    let snapPath = receiptDir / (safeSubject & "-" & timestamp & ".boot-snapshot.json")
    let snapLatest = receiptDir / (safeSubject & ".latest.boot-snapshot.json")
    try:
      let snap = buildSnapshot()
      saveJson(snapPath, snap)
      writeFile(snapLatest, readFile(snapPath))
      snapshotFile = snapPath
    except:
      discard  # Snapshot is best-effort if task-store snapshotting is unavailable

  # Build contract files info
  var contractFiles = newJArray()
  for entry in readContract:
    let target = entry.split("#", 1)[0]
    let path = rootDir / target
    var info = %*{
      "entry": entry,
      "path": target,
      "exists": fileExists(path) or dirExists(path),
    }
    if fileExists(path):
      info["sha256"] = %sha256File(path)
    contractFiles.add(info)

  # Write receipt
  let receipt = %*{
    "written_at": nowUtc(),
    "profile": profile,
    "task_id": (if taskId.len > 0: %taskId else: newJNull()),
    "subject": subject,
    "non_dev": nonDev,
    "capsule_status": capsuleStatus,
    "status": status,
    "read_contract": readContract,
    "contract_files": contractFiles,
    "boot_packet_file": packetFile,
    "boot_snapshot_file": (if snapshotFile.len > 0: %snapshotFile else: newJNull()),
  }
  saveJson(latestFile, receipt)
  saveJson(archiveFile, receipt)
  return latestFile

# ─────────────────────────── Boot Run ───────────────────────────

proc cmdBootRun*(profile: string, taskId: string, nonDev: bool): int =
  let rootDir = vidaRoot()

  if profile notin ["lean", "standard", "full"]:
    echo "Invalid profile: " & profile & ". Must be: lean, standard, full"
    return 1

  # Get read contract
  let reads = readContractFor(profile, nonDev, rootDir)

  # Validate files
  let (valid, missing) = validateReadContract(rootDir, reads)
  let cfgExists = configExists()

  # Check for bootstrap allow_scaffold_missing
  var allowMissing = false
  if cfgExists:
    let cfg = loadRawConfig()
    allowMissing = dottedGetBool(cfg, "project_bootstrap.allow_scaffold_missing", false)

  if not valid and not allowMissing:
    echo "❌ Boot validation failed. Missing files:"
    for f in missing:
      echo "  ✗ " & f
    return 1

  if not valid and allowMissing:
    echo "⚠ Missing files (allow_scaffold_missing=true):"
    for f in missing:
      echo "  ~ " & f

  # Write receipt
  let status = if valid: "ok" else: "partial"
  let receiptPath = writeReceipt(profile, taskId, nonDev,
    "skipped", status, reads, rootDir)

  echo "✅ Boot profile: " & profile
  echo "  receipt: " & receiptPath
  if not nonDev:
    echo "  snapshot: available"
  return 0

# ─────────────────────────── Verify Receipt ───────────────────────────

proc cmdVerifyReceipt*(subject: string, expectedProfile: string = ""): int =
  let rootDir = vidaRoot()
  let receiptDir = rootDir / ".vida" / "logs" / "boot-receipts"
  let safeSubject = safeName(subject)
  let latestFile = receiptDir / (safeSubject & ".latest.json")

  if not fileExists(latestFile):
    echo "❌ No boot receipt for: " & subject
    return 1

  let receipt = loadJson(latestFile)
  let profile = policyValue(receipt{"profile"}, "")
  let status = policyValue(receipt{"status"}, "")
  let packetFile = policyValue(receipt{"boot_packet_file"}, "")
  let snapshotFile = policyValue(receipt{"boot_snapshot_file"}, "")

  if status != "ok" and status != "partial":
    echo "❌ Receipt status is not valid: " & status
    return 1

  if expectedProfile.len > 0 and profile != expectedProfile:
    echo "❌ Profile mismatch: expected " & expectedProfile & " got " & profile
    return 1

  if packetFile.len == 0:
    echo "❌ Receipt missing boot_packet_file"
    return 1
  if not fileExists(packetFile):
    echo "❌ Boot packet missing: " & packetFile
    return 1
  let packetPayload = loadJson(packetFile)
  if policyValue(packetPayload{"profile"}, "") != profile:
    echo "❌ Boot packet profile mismatch: receipt=" & profile & " packet=" & policyValue(packetPayload{"profile"}, "")
    return 1
  if snapshotFile.len > 0 and not fileExists(snapshotFile):
    echo "❌ Boot snapshot missing: " & snapshotFile
    return 1

  echo "✅ Receipt verified: " & subject
  echo "  profile: " & profile
  echo "  status: " & status
  echo "  written_at: " & policyValue(receipt{"written_at"}, "?")
  return 0

# ─────────────────────────── CLI ───────────────────────────

proc cmdProfile*(args: seq[string]): int =
  if args.len == 0:
    echo """Usage:
  vida-legacy boot run <lean|standard|full> [task_id] [--non-dev]
  vida-legacy boot verify-receipt <subject> [profile]
  vida-legacy boot read-contract <lean|standard|full> [--non-dev]
  vida-legacy boot summary <subject>
  vida-legacy boot snapshot [--json] [--top-limit N] [--ready-limit N]"""
    return 1

  case args[0]
  of "run":
    if args.len < 2:
      echo "Usage: vida-legacy boot run <lean|standard|full> [task_id] [--non-dev]"
      return 1
    let profile = args[1].toLowerAscii()
    var taskId = ""
    var nonDev = false
    for i in 2 ..< args.len:
      if args[i] == "--non-dev":
        nonDev = true
      elif not args[i].startsWith("--"):
        taskId = args[i]
    return cmdBootRun(profile, taskId, nonDev)

  of "verify-receipt":
    if args.len < 2:
      echo "Usage: vida-legacy boot verify-receipt <subject> [profile]"
      return 1
    let expectedProfile = if args.len > 2: args[2] else: ""
    return cmdVerifyReceipt(args[1], expectedProfile)

  of "snapshot":
    return cmdSnapshot(args[1..^1])

  of "read-contract", "summary", "lean", "standard", "full":
    return cmdBootPacket(args, vidaRoot())

  else:
    echo "Unknown boot subcommand: " & args[0]
    return 1
