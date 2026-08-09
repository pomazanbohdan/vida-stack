#!/usr/bin/env python3
"""Force every Codex subagent spawn to start from a fresh context."""

from __future__ import annotations

import json
import sys
from typing import Any


TRUSTED_SPAWN_AGENT_TOOL = "spawn_agent"
ALLOWED_SPAWN_AGENT_INPUTS = {"task_name", "message", "fork_turns"}


def _deny(reason: str) -> dict[str, Any]:
    return {
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }
    }


def main() -> int:
    raw = sys.stdin.read()
    try:
        event = json.loads(raw)
    except (json.JSONDecodeError, TypeError):
        print(json.dumps(_deny("Malformed hook event: expected valid JSON object.")))
        return 0

    if not isinstance(event, dict):
        print(json.dumps(_deny("Malformed hook event: expected a JSON object.")))
        return 0

    if "tool_name" not in event:
        print(json.dumps(_deny("Malformed hook event: missing tool_name.")))
        return 0

    tool_name = event.get("tool_name")
    if not isinstance(tool_name, str):
        print(json.dumps(_deny("Malformed hook event: tool_name must be a string.")))
        return 0

    if tool_name != TRUSTED_SPAWN_AGENT_TOOL:
        print("{}")
        return 0

    tool_input = event.get("tool_input")
    if not isinstance(tool_input, dict):
        print(json.dumps(_deny("Malformed spawn-agent event: tool_input must be a JSON object.")))
        return 0

    unknown_fields = sorted(set(tool_input) - ALLOWED_SPAWN_AGENT_INPUTS)
    if unknown_fields:
        print(
            json.dumps(
                _deny(
                    "Malformed spawn-agent event: unsupported tool_input fields: "
                    + ", ".join(unknown_fields)
                    + "."
                )
            )
        )
        return 0

    if not isinstance(tool_input.get("task_name"), str) or not tool_input["task_name"]:
        print(json.dumps(_deny("Malformed spawn-agent event: task_name must be a non-empty string.")))
        return 0

    if not isinstance(tool_input.get("message"), str) or not tool_input["message"]:
        print(json.dumps(_deny("Malformed spawn-agent event: message must be a non-empty string.")))
        return 0

    fork_turns = tool_input.get("fork_turns")
    if fork_turns is not None and not isinstance(fork_turns, str):
        print(json.dumps(_deny("Malformed spawn-agent event: fork_turns must be a string.")))
        return 0

    updated_input = dict(tool_input)
    updated_input["fork_turns"] = "none"
    print(
        json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "allow",
                    "updatedInput": updated_input,
                }
            }
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
