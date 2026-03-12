import std/json
import ../../config/loader
import ../../state/protocol_binding
import ../../core/utils

proc cmdStatus*(args: seq[string]): int =
  let asJson = "--json" in args
  let protocolPayload = protocolBindingStatusPayload(includeRows = asJson)
  let payload = %*{
    "runtime": "taskflow-v0",
    "version": "0.2.2",
    "vida_root": vidaRoot(),
    "vida_workspace": vidaWorkspaceDir(),
    "config_path": configPath(),
    "protocol_binding": protocolPayload,
  }
  if asJson:
    echo pretty(normalizeJson(payload))
  else:
    echo "VIDA v0 Runtime v0.2.2"
    echo "VIDA_ROOT: " & vidaRoot()
    echo "VIDA_WORKSPACE: " & vidaWorkspaceDir()
    echo "Config: " & configPath()
    echo "Protocol binding ok: " & $dottedGetBool(protocolPayload, "ok", false)
    echo "Protocol binding count: " & $dottedGetInt(protocolPayload, "build.binding_count", 0)
    echo "Protocol binding authority: " & dottedGetStr(protocolPayload, "build.primary_state_authority")
  return 0
