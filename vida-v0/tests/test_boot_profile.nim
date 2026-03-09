## Tests for boot/profile module

import std/[json, os, unittest]
import ../src/boot/profile
import ../src/core/utils

suite "boot profile":
  let root = "/tmp/vida_scripts_nim_boot"
  discard existsOrCreateDir(root)
  putEnv("VIDA_ROOT", root)

  test "write receipt records contract files and verify-receipt succeeds":
    createDir(root / "_vida" / "docs")
    createDir(root / ".vida" / "logs" / "boot-receipts")
    writeFile(root / "AGENTS.md", "agents")
    writeFile(root / "_vida" / "docs" / "thinking-protocol.md", "thinking")
    let receiptPath = writeReceipt("lean", "vida-boot-1", true, "present", "ok",
      @["AGENTS.md", "docs/framework/thinking-protocol.md"], root)
    let receipt = loadJson(receiptPath)
    check receipt["contract_files"].len == 2
    check receipt["contract_files"][0]["exists"].getBool() == true
    check cmdVerifyReceipt("vida-boot-1", "lean") == 0

  test "verify-receipt fails when packet file is missing":
    let receiptDir = root / ".vida" / "logs" / "boot-receipts"
    createDir(receiptDir)
    let latestFile = receiptDir / "vida-bad.latest.json"
    saveJson(latestFile, %*{
      "profile": "lean",
      "status": "ok",
      "written_at": "2026-03-08T21:00:00Z",
      "boot_packet_file": root / "missing.boot-packet.json",
      "boot_snapshot_file": newJNull(),
    })
    check cmdVerifyReceipt("vida-bad", "lean") == 1

  test "verify-receipt fails on profile mismatch":
    let receiptDir = root / ".vida" / "logs" / "boot-receipts"
    createDir(receiptDir)
    let packetFile = receiptDir / "vida-profile-mismatch.boot-packet.json"
    saveJson(packetFile, %*{"profile": "lean"})
    let latestFile = receiptDir / "vida-profile-mismatch.latest.json"
    saveJson(latestFile, %*{
      "profile": "lean",
      "status": "ok",
      "written_at": "2026-03-08T21:00:00Z",
      "boot_packet_file": packetFile,
      "boot_snapshot_file": newJNull(),
    })
    check cmdVerifyReceipt("vida-profile-mismatch", "full") == 1

  test "verify-receipt fails when recorded snapshot file is missing":
    let receiptDir = root / ".vida" / "logs" / "boot-receipts"
    createDir(receiptDir)
    let packetFile = receiptDir / "vida-snapshot-missing.boot-packet.json"
    saveJson(packetFile, %*{"profile": "lean"})
    let latestFile = receiptDir / "vida-snapshot-missing.latest.json"
    saveJson(latestFile, %*{
      "profile": "lean",
      "status": "ok",
      "written_at": "2026-03-08T21:00:00Z",
      "boot_packet_file": packetFile,
      "boot_snapshot_file": receiptDir / "vida-snapshot-missing.boot-snapshot.json",
    })
    check cmdVerifyReceipt("vida-snapshot-missing", "lean") == 1
