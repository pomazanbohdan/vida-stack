#!/usr/bin/env python3
"""Spec delta normalization helper for VIDA."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
LOG_DIR = ROOT_DIR / ".vida" / "logs" / "spec-deltas"

ALLOWED_SOURCES = {"issue_contract", "spec_intake", "release_signal", "research_findings", "coach_reopen"}
ALLOWED_STATUS = {
    "delta_ready",
    "needs_user_confirmation",
    "needs_scp_reconciliation",
    "not_required",
    "insufficient_delta",
}


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


def delta_path(task_id: str) -> Path:
    return LOG_DIR / f"{task_id}.json"


def normalize_payload(task_id: str, payload: dict[str, Any]) -> dict[str, Any]:
    delta_source = text(payload.get("delta_source")).lower() or "issue_contract"
    if delta_source not in ALLOWED_SOURCES:
        delta_source = "issue_contract"
    status = text(payload.get("status")).lower() or "insufficient_delta"
    if status not in ALLOWED_STATUS:
        status = "insufficient_delta"
    return {
        "task_id": text(payload.get("task_id")) or task_id,
        "delta_source": delta_source,
        "trigger_status": text(payload.get("trigger_status")),
        "current_contract": text(payload.get("current_contract")),
        "proposed_contract": text(payload.get("proposed_contract")),
        "delta_summary": text(payload.get("delta_summary")),
        "behavior_change": text(payload.get("behavior_change")),
        "scope_impact": text_list(payload.get("scope_impact")),
        "user_confirmation_required": text(payload.get("user_confirmation_required")).lower() or "no",
        "reconciliation_targets": text_list(payload.get("reconciliation_targets")),
        "status": status,
    }


def validate_payload(payload: dict[str, Any], task_id: str) -> tuple[bool, str]:
    if payload.get("task_id") != task_id:
        return False, "task_id_mismatch"
    if payload.get("delta_source") not in ALLOWED_SOURCES:
        return False, "invalid_delta_source"
    if payload.get("status") not in ALLOWED_STATUS:
        return False, "invalid_status"
    if payload.get("status") != "not_required":
        required = ("current_contract", "proposed_contract", "delta_summary", "behavior_change")
        for field in required:
            if not payload.get(field):
                return False, f"missing_{field}"
    if payload.get("status") in {"delta_ready", "needs_scp_reconciliation"} and not payload.get("reconciliation_targets"):
        return False, "missing_reconciliation_targets"
    if payload.get("status") == "needs_user_confirmation" and payload.get("user_confirmation_required") != "yes":
        return False, "user_confirmation_required_yes_expected"
    if payload.get("status") == "not_required" and payload.get("delta_summary"):
        return False, "not_required_should_not_describe_delta"
    return True, "ok"


def write_payload(task_id: str, input_path: Path, output_path: Path | None) -> int:
    raw = json.loads(input_path.read_text(encoding="utf-8"))
    if not isinstance(raw, dict):
        print("[spec-delta] input must be a JSON object", file=sys.stderr)
        return 2
    payload = normalize_payload(task_id, raw)
    ok, reason = validate_payload(payload, task_id)
    if not ok:
        print(f"[spec-delta] {reason}", file=sys.stderr)
        return 2
    path = output_path or delta_path(task_id)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(str(path))
    return 0


def load_payload(task_id: str, path: Path | None) -> tuple[dict[str, Any], Path]:
    selected = path or delta_path(task_id)
    if not selected.exists():
        raise FileNotFoundError(selected)
    payload = json.loads(selected.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError("payload must be an object")
    return payload, selected


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print(
            "Usage: python3 _vida/scripts/spec-delta.py write <task_id> <input.json> [--output PATH] | "
            "validate <task_id> [--path PATH] | status <task_id> [--path PATH]",
            file=sys.stderr,
        )
        return 2
    command = argv[1]
    task_id = argv[2]
    if command == "write":
        if len(argv) < 4:
            print("[spec-delta] missing input path", file=sys.stderr)
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
            print(f"[spec-delta] missing file: {exc}", file=sys.stderr)
            return 1
        except ValueError as exc:
            print(f"[spec-delta] {exc}", file=sys.stderr)
            return 2
        payload = normalize_payload(task_id, payload)
        ok, reason = validate_payload(payload, task_id)
        if command == "validate":
            if not ok:
                print(f"[spec-delta] {reason}", file=sys.stderr)
                return 2
            print(f"OK {selected}")
            return 0
        status_payload = {
            "path": str(selected),
            "valid": ok,
            "reason": reason,
            "status": payload.get("status", ""),
            "user_confirmation_required": payload.get("user_confirmation_required", "no"),
        }
        print(json.dumps(status_payload, indent=2, sort_keys=True))
        return 0 if ok else 2
    print(f"[spec-delta] unknown command: {command}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
