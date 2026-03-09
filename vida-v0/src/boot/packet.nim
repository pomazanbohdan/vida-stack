## VIDA Boot Packet — read contract + boot packet generation.
##
## Replaces `boot-packet.py` (168 lines).
## Generates the machine-readable boot packet that defines
## which files must be read and which invariants apply.

import std/[json, os, strutils]
import ../core/[utils, config, toon]

# ─────────────────────────── Read Contracts ───────────────────────────

const CommonReads* = @[
  "AGENTS.md",
  "vida/config/instructions/instruction-contracts.thinking-protocol.md#section-algorithm-selector",
  "vida/config/instructions/instruction-contracts.thinking-protocol.md#section-stc",
  "vida/config/instructions/instruction-contracts.thinking-protocol.md#section-pr-cot",
  "vida/config/instructions/instruction-contracts.thinking-protocol.md#section-mar",
  "vida/config/instructions/instruction-contracts.thinking-protocol.md#section-5-solutions",
  "vida/config/instructions/instruction-contracts.thinking-protocol.md#section-meta-analysis",
  "vida/config/instructions/instruction-contracts.thinking-protocol.md#section-bug-reasoning",
  "vida/config/instructions/instruction-contracts.thinking-protocol.md#section-web-search",
  "vida/config/instructions/instruction-contracts.thinking-protocol.md#section-reasoning-modules",
  "vida/config/instructions/runtime-instructions.web-validation-protocol.md",
  "vida/config/instructions/runtime-instructions.beads-protocol.md",
  "vida/config/instructions/runtime-instructions.project-overlay-protocol.md",
]

const StandardReads* = @[
  "vida/config/instructions/runtime-instructions.todo-protocol.md",
  "vida/config/instructions/command-instructions.implement-execution-protocol.md",
  "vida/config/instructions/command-instructions.use-case-packs.md",
]

const FullReads* = @[
  "vida/config/instructions/instruction-contracts.orchestration-protocol.md",
  "vida/config/instructions/command-instructions.pipelines.md",
]

const Invariants* = @[
  "read AGENTS.md first after compression",
  "apply thinking-protocol algorithms",
  "task state lives in vida-v0 task store",
  "execute only through TODO blocks",
  "external-first fanout for eligible read-only work",
]

# ─────────────────────────── Read Contract Builder ───────────────────────────

proc profileReads*(profile: string, nonDev: bool, agentSystemActive: bool,
                   rootDir: string): seq[string] =
  result = CommonReads
  if profile in ["standard", "full"]:
    result.add(StandardReads)
  if profile == "full":
    result.add(FullReads)
  if nonDev:
    result.add("vida/config/instructions/runtime-instructions.spec-contract-protocol.md")
  if fileExists(rootDir / "vida.config.yaml"):
    result.add("vida.config.yaml")
  if agentSystemActive:
    result.add("vida/config/instructions/instruction-contracts.agent-system-protocol.md")

# ─────────────────────────── Packet Builder ───────────────────────────

proc buildPacket*(profile: string, nonDev: bool, rootDir: string): JsonNode =
  let cfg = loadRawConfig()
  let langPolicy = getLanguagePolicy(cfg)
  let protocolAct = getProtocolActivation(cfg)
  let agentSystemActive = dottedGetBool(protocolAct, "agent_system", false)
  let reads = profileReads(profile, nonDev, agentSystemActive, rootDir)

  let snapshotCmd = if nonDev: ""
    else: "VIDA_ROOT=$PWD vida-v0/vida-v0 boot snapshot --json"
  let snapshotScope = if nonDev: "disabled for non-dev boot"
    else: "top-level in-progress tasks, ready head, and open subtask tree"

  result = %*{
    "generated_at": nowUtc(),
    "profile": profile,
    "non_dev": nonDev,
    "language_policy": langPolicy,
    "protocol_activation": protocolAct,
    "read_contract": reads,
    "invariants": Invariants,
    "runtime_hints": {
      "compact_boot_snapshot_command": snapshotCmd,
      "compact_boot_snapshot_scope": snapshotScope,
    },
  }

proc readContractFor*(profile: string, nonDev: bool, rootDir: string): seq[string] =
  let cfg = loadRawConfig()
  let protocolAct = getProtocolActivation(cfg)
  let agentSystemActive = dottedGetBool(protocolAct, "agent_system", false)
  profileReads(profile, nonDev, agentSystemActive, rootDir)

# ─────────────────────────── Packet Summary ───────────────────────────

proc packetSummary*(subject: string, rootDir: string): JsonNode =
  let latest = rootDir / ".vida" / "logs" / "boot-receipts" / (subject & ".latest.boot-packet.json")
  if not fileExists(latest):
    return %*{"error": "Missing packet: " & latest}
  let payload = loadJson(latest)
  let readContract = payload{"read_contract"}
  let invariants = payload{"invariants"}
  result = %*{
    "subject": subject,
    "profile": payload{"profile"},
    "non_dev": payload{"non_dev"},
    "read_contract_count": (if readContract.isNil: 0 else: readContract.len),
    "invariants_count": (if invariants.isNil: 0 else: invariants.len),
    "protocol_activation": payload{"protocol_activation"},
    "runtime_hints": payload{"runtime_hints"},
  }

# ─────────────────────────── CLI Commands ───────────────────────────

proc cmdBootPacket*(args: seq[string], rootDir: string): int =
  if args.len == 0:
    echo "Usage: vida-v0 boot <lean|standard|full|read-contract|summary> [args]"
    return 1

  let command = args[0].toLowerAscii().strip()

  case command
  of "read-contract":
    if args.len < 2:
      echo "Usage: vida-v0 boot read-contract <lean|standard|full> [--non-dev]"
      return 1
    let profile = args[1].toLowerAscii().strip()
    if profile notin ["lean", "standard", "full"]:
      echo "Invalid profile: " & profile & ". Must be: lean, standard, full"
      return 1
    let nonDev = "--non-dev" in args[2..^1]
    for entry in readContractFor(profile, nonDev, rootDir):
      echo entry
    return 0

  of "summary":
    if args.len < 2:
      echo "Usage: vida-v0 boot summary <subject>"
      return 1
    let subject = args[1].strip()
    let payload = normalizeJson(packetSummary(subject, rootDir))
    if "--json" in args:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0

  of "lean", "standard", "full":
    let profile = command
    let nonDev = "--non-dev" in args[1..^1]
    let payload = normalizeJson(buildPacket(profile, nonDev, rootDir))
    if "--json" in args:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0

  else:
    echo "Unknown boot subcommand: " & command
    return 1
