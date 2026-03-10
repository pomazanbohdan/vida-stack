import std/[json, os, unittest]
import ../src/state/[recovery, run_graph]

suite "recovery runtime":
  let root = "/tmp/vida_taskflow_recovery_test"
  if dirExists(root):
    removeDir(root)
  discard existsOrCreateDir(root)
  putEnv("VIDA_ROOT", root)

  test "recovery status and resume use checkpoint plus run graph":
    discard writeGraph("task-1", ensureGraph("task-1", "implementation", "implementation"))
    discard updateNode("task-1", "implementation", "writer", "blocked", "implementation", %*{"reason": "awaiting_fix"})
    discard writeCheckpointCommit("task-1", "route_cursor", "writer:blocked", "dispatch.writer")

    let status = recoveryStatus("task-1")
    check status["recovery_ready"].getBool() == true
    check status["resume_hint"]["next_node"].getStr() == "writer"

    let resume = resumePayload("task-1")
    check resume["ok"].getBool() == true
    check resume["resume_node"].getStr() == "writer"

  test "gateway trigger resolves and gates resume":
    discard openGatewayHandle("task-2", "approval_wait", "approve:123", "approval", "approval")
    let resolved = resolveGatewayTrigger("approve:123")
    check resolved["match_count"].getInt() == 1

    discard writeGraph("task-2", ensureGraph("task-2", "implementation", "implementation"))
    discard updateNode("task-2", "implementation", "approval", "blocked", "implementation", %*{"reason": "awaiting_approval"})
    discard writeCheckpointCommit("task-2", "manual_gateway", "approval:blocked", "resume.approval")
    let resume = resumePayload("task-2", "approve:123")
    check resume["ok"].getBool() == true
    check resume["gateway_resolution"]["match_count"].getInt() == 1
