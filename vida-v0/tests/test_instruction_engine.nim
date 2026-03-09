import std/[json, os, unittest]
import ../src/core/instruction_engine

suite "instruction engine":
  let root = getCurrentDir()
  putEnv("VIDA_ROOT", root)

  test "composes writer instruction surface":
    let payload = composeInstructionSurface("writer", "task_lifecycle", "writer_lane")
    check payload["ok"].getBool() == true
    check payload["role"].getStr() == "writer"
    check payload["bundle"].getStr() == "default_runtime_bundle"
    check payload["role_binding"]["output_contract"].getStr() == "execution_event"
    check payload["route_overlay"]["route"].getStr() == "writer_lane"
    check payload["machine_binding"]["machine"].getStr() == "task_lifecycle"

  test "fails for unknown role":
    let payload = composeInstructionSurface("unknown_role")
    check payload["ok"].getBool() == false
    check payload["reason"].getStr() == "unknown_role"
