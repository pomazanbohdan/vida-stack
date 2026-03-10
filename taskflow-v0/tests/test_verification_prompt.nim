## Tests for gates/verification_prompt module

import std/[json, os, strutils, unittest]
import ../src/gates/verification_prompt
import ../src/gates/worker_packet

suite "verification prompt":
  let root = "/tmp/vida_scripts_nim_verification_prompt"
  discard existsOrCreateDir(root)
  putEnv("VIDA_ROOT", root)

  test "verification prompt passes worker packet gate":
    let outputFile = root / "qwen.output.txt"
    writeFile(outputFile, "Validated candidate synthesis with direct evidence refs.\n")
    let prompt = verificationPromptText(
      "Task: verify routed implementation",
      "implementation",
      "verification",
      newJObject(),
      %*{
        "consensus_mode": "semantic_majority",
        "decision_ready": true,
        "dominant_finding": {
          "cluster_id": "def456",
          "sample": "worker packet gate should block invalid output contracts"
        },
        "success_agent_backends": ["qwen_cli"],
        "open_conflicts": []
      },
      %*[
        {
          "agent_backend": "qwen_cli",
          "status": "success",
          "output_file": outputFile
        }
      ]
    )
    check worker_packet.validatePacketText(prompt) == newSeq[string]()
    check "Primary ensemble summary:" in prompt
    check "Success lane excerpts:" in prompt
    check "qwen_cli: Validated candidate synthesis with direct evidence refs." in prompt
