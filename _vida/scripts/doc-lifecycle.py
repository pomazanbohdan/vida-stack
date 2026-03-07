#!/usr/bin/env python3
"""Document lifecycle and freshness ledger for VIDA framework documents."""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
STATE_PATH = ROOT_DIR / ".vida" / "state" / "doc-lifecycle.json"
VALID_STATES = {"proposed", "current", "superseded", "deprecated", "stale"}


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def load_state() -> dict[str, Any]:
    if not STATE_PATH.exists():
        return {"entries": {}}
    try:
        payload = json.loads(STATE_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {"entries": {}}
    if not isinstance(payload, dict):
        return {"entries": {}}
    payload.setdefault("entries", {})
    return payload


def save_state(payload: dict[str, Any]) -> None:
    STATE_PATH.parent.mkdir(parents=True, exist_ok=True)
    STATE_PATH.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def record_doc_state(*, path: str, state: str, owner: str, notes: str) -> dict[str, Any]:
    normalized_state = state.strip().casefold()
    if normalized_state not in VALID_STATES:
        raise ValueError(f"invalid document lifecycle state: {state}")
    payload = load_state()
    entry = {
        "path": path.strip(),
        "state": normalized_state,
        "owner": owner.strip(),
        "notes": notes.strip(),
        "last_reviewed_at": now_utc(),
    }
    payload.setdefault("entries", {})[entry["path"]] = entry
    save_state(payload)
    return entry


def validate_doc_state(path: str, *, max_age_days: int = 90) -> dict[str, Any]:
    payload = load_state()
    entry = payload.get("entries", {}).get(path, {})
    if not isinstance(entry, dict) or not entry:
        return {"valid": False, "reason": "missing_document_state", "entry": {}}
    reviewed_at_raw = str(entry.get("last_reviewed_at", "")).strip()
    if not reviewed_at_raw:
        return {"valid": False, "reason": "missing_last_reviewed_at", "entry": entry}
    try:
        reviewed_at = datetime.fromisoformat(reviewed_at_raw.replace("Z", "+00:00"))
    except ValueError:
        return {"valid": False, "reason": "invalid_last_reviewed_at", "entry": entry}
    age_days = (datetime.now(timezone.utc) - reviewed_at).days
    if age_days > max_age_days and entry.get("state") == "current":
        return {"valid": False, "reason": "stale_document", "entry": entry, "age_days": age_days}
    return {"valid": True, "reason": "", "entry": entry, "age_days": age_days}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)

    record = sub.add_parser("record")
    record.add_argument("path")
    record.add_argument("state", choices=sorted(VALID_STATES))
    record.add_argument("--owner", required=True)
    record.add_argument("--notes", required=True)

    validate = sub.add_parser("validate")
    validate.add_argument("path")
    validate.add_argument("--max-age-days", type=int, default=90)

    sub.add_parser("status")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.command == "status":
        print(json.dumps(load_state(), indent=2, sort_keys=True))
        return 0
    if args.command == "record":
        print(
            json.dumps(
                record_doc_state(
                    path=args.path,
                    state=args.state,
                    owner=args.owner,
                    notes=args.notes,
                ),
                indent=2,
                sort_keys=True,
            )
        )
        return 0
    if args.command == "validate":
        result = validate_doc_state(args.path, max_age_days=max(1, args.max_age_days))
        print(json.dumps(result, indent=2, sort_keys=True))
        return 0 if result.get("valid") else 2
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
