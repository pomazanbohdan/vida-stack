#!/usr/bin/env python3
"""Draft execution-spec helper for VIDA."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
LOG_DIR = ROOT_DIR / ".vida" / "logs" / "draft-execution-specs"
ALLOWED_PATHS = {"/vida-form-task", "/vida-bug-fix"}


def text(value: Any) -> str:
    if value is None:
        return ""
    return str(value).strip()


def text_list(value: Any) -> list[str]:
    if value is None:
        return []
    if isinstance(value, list):
        return [str(item).strip() for item in value if str(item).strip()]
    value = str(value).strip()
    return [value] if value else []


def artifact_path(task_id: str) -> Path:
    return LOG_DIR / f"{task_id}.json"


def normalize_payload(task_id: str, payload: dict[str, Any]) -> dict[str, Any]:
    recommended_next_path = text(payload.get("recommended_next_path")) or "/vida-form-task"
    return {
        "task_id": text(payload.get("task_id")) or task_id,
        "scope_in": text_list(payload.get("scope_in")),
        "scope_out": text_list(payload.get("scope_out")),
        "acceptance_checks": text_list(payload.get("acceptance_checks")),
        "assumptions": text_list(payload.get("assumptions")),
        "open_decisions": text_list(payload.get("open_decisions")),
        "recommended_next_path": recommended_next_path,
    }


def validate_payload(payload: dict[str, Any], task_id: str) -> tuple[bool, str]:
    if payload.get("task_id") != task_id:
        return False, "task_id_mismatch"
    if not payload.get("scope_in"):
        return False, "missing_scope_in"
    if not payload.get("acceptance_checks"):
        return False, "missing_acceptance_checks"
    if payload.get("recommended_next_path") not in ALLOWED_PATHS:
        return False, "invalid_recommended_next_path"
    return True, "ok"


def write_payload(task_id: str, input_path: Path, output_path: Path | None) -> int:
    raw = json.loads(input_path.read_text(encoding="utf-8"))
    if not isinstance(raw, dict):
        print("[draft-execution-spec] input must be a JSON object", file=sys.stderr)
        return 2
    payload = normalize_payload(task_id, raw)
    ok, reason = validate_payload(payload, task_id)
    if not ok:
        print(f"[draft-execution-spec] {reason}", file=sys.stderr)
        return 2
    path = output_path or artifact_path(task_id)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(str(path))
    return 0


def load_payload(task_id: str, path: Path | None) -> tuple[dict[str, Any], Path]:
    selected = path or artifact_path(task_id)
    if not selected.exists():
        raise FileNotFoundError(selected)
    payload = json.loads(selected.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError("payload must be an object")
    return payload, selected


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print(
            "Usage: python3 _vida/scripts/draft-execution-spec.py write <task_id> <input.json> [--output PATH] | "
            "validate <task_id> [--path PATH] | status <task_id> [--path PATH]",
            file=sys.stderr,
        )
        return 2
    command = argv[1]
    task_id = argv[2]
    if command == "write":
        if len(argv) < 4:
            print("[draft-execution-spec] missing input path", file=sys.stderr)
            return 2
        input_path = Path(argv[3])
        output_path = None
        if len(argv) > 5 and argv[4] == "--output":
            output_path = Path(argv[5])
        return write_payload(task_id, input_path, output_path)
    if command in {"validate", "status"}:
        path = None
        if len(argv) > 4 and argv[3] == "--path":
            path = Path(argv[4])
        try:
            payload, selected = load_payload(task_id, path)
        except FileNotFoundError as exc:
            print(f"[draft-execution-spec] missing file: {exc}", file=sys.stderr)
            return 1
        except ValueError as exc:
            print(f"[draft-execution-spec] {exc}", file=sys.stderr)
            return 2
        payload = normalize_payload(task_id, payload)
        ok, reason = validate_payload(payload, task_id)
        if command == "validate":
            if not ok:
                print(f"[draft-execution-spec] {reason}", file=sys.stderr)
                return 2
            print(f"OK {selected}")
            return 0
        print(
            json.dumps(
                {
                    "path": str(selected),
                    "valid": ok,
                    "reason": reason,
                    "recommended_next_path": payload.get("recommended_next_path", ""),
                    "open_decisions": payload.get("open_decisions", []),
                },
                indent=2,
                sort_keys=True,
            )
        )
        return 0 if ok else 2
    print(f"[draft-execution-spec] unknown command: {command}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
