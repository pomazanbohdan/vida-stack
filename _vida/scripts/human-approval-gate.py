#!/usr/bin/env python3
"""Human approval receipt helper for VIDA review-state gates."""

from __future__ import annotations

import argparse
import importlib.util
import json
from datetime import datetime, timezone
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


dispatch = load_module("vida_subagent_dispatch_for_human_gate", SCRIPT_DIR / "subagent-dispatch.py")


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def write_receipt(
    *,
    task_id: str,
    task_class: str,
    review_state: str,
    decision: str,
    approver_id: str,
    notes: str,
) -> Path:
    _, route = dispatch.route_snapshot(task_class, task_id)
    payload = {
        "ts": now_utc(),
        "task_id": task_id,
        "task_class": task_class,
        "review_state": review_state,
        "decision": decision,
        "approver_id": approver_id,
        "notes": notes,
        "route_receipt_hash": dispatch.route_receipt_hash(route),
        "route_receipt": dispatch.route_receipt_payload(route),
    }
    path = dispatch.approval_receipt_path(task_id, task_class)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)

    record = sub.add_parser("record")
    record.add_argument("task_id")
    record.add_argument("task_class")
    record.add_argument("review_state")
    record.add_argument("decision", choices={"approved", "rejected"})
    record.add_argument("--approver-id", required=True)
    record.add_argument("--notes", required=True)

    status = sub.add_parser("status")
    status.add_argument("task_id")
    status.add_argument("task_class")

    validate = sub.add_parser("validate")
    validate.add_argument("task_id")
    validate.add_argument("task_class")
    validate.add_argument("review_state")

    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.command == "record":
        path = write_receipt(
            task_id=args.task_id,
            task_class=args.task_class,
            review_state=args.review_state,
            decision=args.decision,
            approver_id=args.approver_id,
            notes=args.notes,
        )
        print(str(path))
        return 0
    if args.command == "status":
        print(json.dumps(dispatch.load_approval_receipt(args.task_id, args.task_class), indent=2, sort_keys=True))
        return 0
    if args.command == "validate":
        _, route = dispatch.route_snapshot(args.task_class, args.task_id)
        ok, payload, reason = dispatch.validate_approval_receipt(
            args.task_id,
            args.task_class,
            route,
            args.review_state,
        )
        print(json.dumps({"valid": ok, "reason": reason, "receipt": payload}, indent=2, sort_keys=True))
        return 0 if ok else 2
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
