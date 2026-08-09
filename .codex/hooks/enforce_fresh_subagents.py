#!/usr/bin/env python3
"""Force every Codex subagent spawn to start from a fresh context."""

from __future__ import annotations

import json
import re
import sys
from typing import Any


def _deny(reason: str) -> dict[str, Any]:
    return {
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }
    }


def _normalized_tool_name(value: Any) -> str:
    if not isinstance(value, str):
        return ""
    return re.sub(r"[^a-z0-9]", "", value.lower())


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

    if not _normalized_tool_name(tool_name).endswith("spawnagent"):
        print("{}")
        return 0

    tool_input = event.get("tool_input")
    if not isinstance(tool_input, dict):
        print(json.dumps(_deny("Malformed spawn-agent event: tool_input must be a JSON object.")))
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
