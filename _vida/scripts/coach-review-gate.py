#!/usr/bin/env python3
"""Coach review gate for post-write VIDA execution."""

from __future__ import annotations

import importlib.util
import json
import re
import sys
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
ROUTE_RECEIPT_DIR = ROOT_DIR / ".vida" / "logs" / "route-receipts"


def load_module(name: str, path: Path) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to load module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


dispatch_runtime = load_module("vida_subagent_dispatch_runtime_coach_gate", SCRIPT_DIR / "subagent-dispatch.py")


def usage() -> int:
    print(
        "Usage:\n"
        "  python3 _vida/scripts/coach-review-gate.py check <task_id>",
        file=sys.stderr,
    )
    return 1


def safe_name(value: str, fallback: str) -> str:
    normalized = re.sub(r"[^A-Za-z0-9._-]+", "-", value.strip() or fallback)
    return normalized if normalized else fallback


def load_json(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {}
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return payload if isinstance(payload, dict) else {}


def route_receipt_files(task_id: str) -> list[Path]:
    task_prefix = safe_name(task_id, "task")
    return sorted(ROUTE_RECEIPT_DIR.glob(f"{task_prefix}.*.route.json"))


def validate_coach_artifact(task_id: str, task_class: str, route_receipt: dict[str, Any]) -> dict[str, Any]:
    route_hash = dispatch_runtime.route_receipt_hash(route_receipt)
    coach_receipt = dispatch_runtime.load_coach_receipt(task_id, task_class)
    coach_blocker = dispatch_runtime.load_coach_blocker(task_id, task_class)
    if coach_receipt:
        if coach_receipt.get("route_receipt_hash") == route_hash and coach_receipt.get("status") == "coach_approved":
            return {
                "task_class": task_class,
                "status": "ok",
                "artifact": "coach_receipt",
                "path": str(dispatch_runtime.coach_receipt_path(task_id, task_class)),
            }
        return {
            "task_class": task_class,
            "status": "blocked",
            "artifact": "coach_receipt",
            "path": str(dispatch_runtime.coach_receipt_path(task_id, task_class)),
            "reason": "stale_or_invalid_coach_receipt",
        }
    if coach_blocker:
        blocker_reason = str(coach_blocker.get("reason", "")).strip()
        rework_handoff_path = str(coach_blocker.get("rework_handoff_path", "")).strip()
        rework_handoff_status = str(coach_blocker.get("rework_handoff_status", "")).strip()
        if str(coach_blocker.get("status", "")).strip() == "return_for_rework":
            handoff_ok, _, handoff_error = dispatch_runtime.validate_rework_handoff(task_id, task_class, route_receipt)
            if not handoff_ok:
                blocker_reason = handoff_error or "missing_structured_rework_handoff"
            elif not rework_handoff_path:
                rework_handoff_path = str(dispatch_runtime.rework_handoff_path(task_id, task_class))
                rework_handoff_status = "writer_rework_ready"
        return {
            "task_class": task_class,
            "status": "blocked",
            "artifact": "coach_blocker",
            "path": str(dispatch_runtime.coach_blocker_path(task_id, task_class)),
            "reason": blocker_reason or str(coach_blocker.get("status", "coach_blocked")),
            "rework_handoff_path": rework_handoff_path,
            "rework_handoff_status": rework_handoff_status,
        }
    return {
        "task_class": task_class,
        "status": "blocked",
        "artifact": "missing",
        "path": "",
        "reason": "missing_coach_review_artifact",
    }


def check_gate(task_id: str) -> tuple[int, dict[str, Any]]:
    evaluations: list[dict[str, Any]] = []
    for path in route_receipt_files(task_id):
        payload = load_json(path)
        route_receipt = payload.get("route_receipt")
        if not isinstance(route_receipt, dict):
            continue
        if str(route_receipt.get("coach_required", "no")).strip().casefold() != "yes":
            continue
        task_class = str(route_receipt.get("task_class", "")).strip()
        if not task_class:
            continue
        evaluation = validate_coach_artifact(task_id, task_class, route_receipt)
        evaluation["route_receipt_path"] = str(path)
        evaluations.append(evaluation)

    blockers = [item for item in evaluations if item.get("status") != "ok"]
    payload = {
        "task_id": task_id,
        "status": "ok" if not blockers else "blocked",
        "required_routes": evaluations,
        "blockers": blockers,
    }
    return (0 if not blockers else 2), payload


def main(argv: list[str]) -> int:
    if len(argv) < 3 or argv[1] != "check":
        return usage()
    exit_code, payload = check_gate(argv[2])
    print(json.dumps(payload, indent=2, sort_keys=True))
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
