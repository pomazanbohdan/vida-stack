#!/usr/bin/env python3
"""Persistent framework memory ledger for VIDA lessons, corrections, and anomalies."""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
STATE_PATH = ROOT_DIR / ".vida" / "state" / "framework-memory.json"
VALID_KINDS = {"lesson", "correction", "anomaly"}


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def load_state() -> dict[str, Any]:
    if not STATE_PATH.exists():
        return {"entries": [], "summary": {"lesson_count": 0, "correction_count": 0, "anomaly_count": 0}}
    try:
        payload = json.loads(STATE_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {"entries": [], "summary": {"lesson_count": 0, "correction_count": 0, "anomaly_count": 0}}
    if not isinstance(payload, dict):
        return {"entries": [], "summary": {"lesson_count": 0, "correction_count": 0, "anomaly_count": 0}}
    payload.setdefault("entries", [])
    payload.setdefault("summary", {"lesson_count": 0, "correction_count": 0, "anomaly_count": 0})
    return payload


def save_state(payload: dict[str, Any]) -> None:
    STATE_PATH.parent.mkdir(parents=True, exist_ok=True)
    STATE_PATH.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def record_entry(*, kind: str, summary: str, source_task: str = "", details: dict[str, Any] | None = None) -> dict[str, Any]:
    normalized_kind = kind.strip().casefold()
    if normalized_kind not in VALID_KINDS:
        raise ValueError(f"invalid framework memory kind: {kind}")
    payload = load_state()
    entry = {
        "ts": now_utc(),
        "kind": normalized_kind,
        "summary": summary.strip(),
        "source_task": source_task.strip(),
        "details": details or {},
    }
    payload.setdefault("entries", []).append(entry)
    summary_bucket = payload.setdefault("summary", {})
    summary_key = f"{normalized_kind}_count"
    summary_bucket[summary_key] = int(summary_bucket.get(summary_key, 0) or 0) + 1
    save_state(payload)
    return entry


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)

    record = sub.add_parser("record")
    record.add_argument("kind", choices=sorted(VALID_KINDS))
    record.add_argument("--summary", required=True)
    record.add_argument("--source-task", default="")
    record.add_argument("--details-json", default="")

    sub.add_parser("status")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.command == "status":
        print(json.dumps(load_state(), indent=2, sort_keys=True))
        return 0
    if args.command == "record":
        details = json.loads(args.details_json) if args.details_json else {}
        print(
            json.dumps(
                record_entry(
                    kind=args.kind,
                    summary=args.summary,
                    source_task=args.source_task,
                    details=details if isinstance(details, dict) else {},
                ),
                indent=2,
                sort_keys=True,
            )
        )
        return 0
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
