#!/usr/bin/env python3
"""Aggregated framework operator status surface for approvals and memory."""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
FRAMEWORK_MEMORY_PATH = ROOT_DIR / ".vida" / "state" / "framework-memory.json"
ROUTE_RECEIPT_DIR = ROOT_DIR / ".vida" / "logs" / "route-receipts"


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def load_json(path: Path, default: Any) -> Any:
    if not path.exists():
        return default
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return default


def approval_summary() -> dict[str, Any]:
    approved = 0
    rejected = 0
    review_states: dict[str, int] = {}
    for path in sorted(ROUTE_RECEIPT_DIR.glob("*.approval.json")):
        payload = load_json(path, {})
        if not isinstance(payload, dict):
            continue
        decision = str(payload.get("decision", "")).strip().casefold()
        review_state = str(payload.get("review_state", "")).strip()
        if decision == "approved":
            approved += 1
        elif decision == "rejected":
            rejected += 1
        if review_state:
            review_states[review_state] = int(review_states.get(review_state, 0) or 0) + 1
    return {
        "approved_count": approved,
        "rejected_count": rejected,
        "review_states": review_states,
    }


def framework_memory_summary() -> dict[str, Any]:
    payload = load_json(FRAMEWORK_MEMORY_PATH, {})
    summary = payload.get("summary", {}) if isinstance(payload, dict) else {}
    if not isinstance(summary, dict):
        summary = {}
    return {
        "lesson_count": int(summary.get("lesson_count", 0) or 0),
        "correction_count": int(summary.get("correction_count", 0) or 0),
        "anomaly_count": int(summary.get("anomaly_count", 0) or 0),
    }


def build_status_payload() -> dict[str, Any]:
    return {
        "generated_at": now_utc(),
        "framework_memory": framework_memory_summary(),
        "approval_summary": approval_summary(),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices={"status"})
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.command == "status":
        print(json.dumps(build_status_payload(), indent=2, sort_keys=True))
        return 0
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
