#!/usr/bin/env python3
"""Regression tests for the fresh-subagent PreToolUse hook."""

from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path
from typing import Any


HOOK = Path(__file__).with_name("enforce_fresh_subagents.py")


def run_hook(event: dict[str, Any]) -> dict[str, Any]:
    result = subprocess.run(
        [sys.executable, str(HOOK)],
        input=json.dumps(event),
        capture_output=True,
        check=True,
        text=True,
    )
    return json.loads(result.stdout)


class EnforceFreshSubagentsTests(unittest.TestCase):
    def test_spoofed_spawn_agent_names_receive_no_permission_decision(self) -> None:
        for tool_name in ("evil_spawn_agent", "despawn_agent", "x.spawn-agent"):
            with self.subTest(tool_name=tool_name):
                self.assertEqual(
                    run_hook({"tool_name": tool_name, "tool_input": {"cmd": "id"}}),
                    {},
                )

    def test_trusted_spawn_agent_rejects_unknown_input_fields(self) -> None:
        output = run_hook(
            {
                "tool_name": "spawn_agent",
                "tool_input": {
                    "task_name": "bounded_task",
                    "message": "Do bounded work.",
                    "cmd": "id",
                },
            }
        )

        hook_output = output["hookSpecificOutput"]
        self.assertEqual(hook_output["permissionDecision"], "deny")
        self.assertIn("unsupported tool_input fields: cmd", hook_output["permissionDecisionReason"])
        self.assertNotIn("updatedInput", hook_output)

    def test_trusted_spawn_agent_forces_fresh_context(self) -> None:
        output = run_hook(
            {
                "tool_name": "spawn_agent",
                "tool_input": {
                    "task_name": "bounded_task",
                    "message": "Do bounded work.",
                    "fork_turns": "all",
                },
            }
        )

        hook_output = output["hookSpecificOutput"]
        self.assertEqual(hook_output["permissionDecision"], "allow")
        self.assertEqual(
            hook_output["updatedInput"],
            {
                "task_name": "bounded_task",
                "message": "Do bounded work.",
                "fork_turns": "none",
            },
        )


if __name__ == "__main__":
    unittest.main()
