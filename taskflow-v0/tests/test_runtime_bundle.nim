import std/[json, os, unittest]
import ../src/core/[runtime_bundle]

suite "runtime bundle":
  let root = getCurrentDir()
  putEnv("VIDA_ROOT", root)

  test "builds compiled runtime bundle":
    let payload = buildRuntimeKernelBundle()
    check payload["artifact_type"].getStr() == "runtime_bundle"
    check payload["instruction_bundle"]["artifact_name"].getStr().len > 0
    check payload["runtime_agent_inventory"]["agents"].kind == JArray
    check payload["compiled_agent_extensions"].kind == JObject

  test "bundle readiness check passes":
    let payload = buildRuntimeKernelBundle()
    let ready = runtimeKernelBundleReady(payload)
    check ready["ok"].getBool() == true
    check ready["bundle_order"].kind == JArray
