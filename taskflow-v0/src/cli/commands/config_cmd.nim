import std/json
import ../../config/[loader, bundle_builder, validation/aggregate, accessors]

proc cmdConfigValidate*(): int =
  if not configExists():
    echo "vida.config.yaml not found at: " & configPath()
    return 1
  let config = loadRawConfig()
  let vr = validateConfig(config)
  if vr.valid:
    echo "✅ vida.config.yaml is valid"
    for w in vr.warnings:
      echo "  ⚠ " & w
    return 0
  echo "❌ vida.config.yaml validation failed:"
  for e in vr.errors:
    echo "  ✗ " & e
  for w in vr.warnings:
    echo "  ⚠ " & w
  1

proc cmdConfigDump*(): int =
  if not configExists():
    echo "{}"
    return 0
  echo pretty(loadRawConfig())
  0

proc cmdConfigProtocolActive*(protocol: string): int =
  let config = loadValidatedConfig()
  if isProtocolActive(config, protocol):
    echo "true"
    return 0
  echo "false"
  1

proc cmdConfig*(args: seq[string]): int =
  if args.len == 0:
    echo "Usage: taskflow-v0 config <validate|dump|protocol-active>"
    return 1
  case args[0]
  of "validate":
    cmdConfigValidate()
  of "dump":
    cmdConfigDump()
  of "protocol-active":
    if args.len < 2:
      echo "Usage: taskflow-v0 config protocol-active <protocol>"
      return 1
    cmdConfigProtocolActive(args[1])
  else:
    echo "Unknown config subcommand: " & args[0]
    1
