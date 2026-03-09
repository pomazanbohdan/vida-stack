#!/usr/bin/env python3
"""Gate tracked FSAP tasks on delegated verification or explicit override."""

from __future__ import annotations

import json
import sys
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


ROOT_DIR = Path(__file__).resolve().parents[2]
LOG_FILE = ROOT_DIR / ".vida" / "logs" / "beads-execution.jsonl"
REVIEW_DIR = ROOT_DIR / ".vida" / "logs"
RECEIPT_DIR = ROOT_DIR / ".vida" / "logs" / "fsap-verification"
ALLOWED_OVERRIDE_REASONS = {
    "no_available_verifier",
    "runtime_blocker",
}


def now_utc() -> str:
    return datetime.now(UTC).strftime("%Y-%m-%dT%H:%M:%SZ")


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    events: list[dict[str, Any]] = []
    with path.open() as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            try:
                payload = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(payload, dict):
                events.append(payload)
    return events


def load_json(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {}
    try:
        payload = json.loads(path.read_text())
    except json.JSONDecodeError:
        return {}
    return payload if isinstance(payload, dict) else {}


def write_json(path: Path, payload: dict[str, Any]) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    return path


def review_payload_path(task_id: str) -> Path:
    return REVIEW_DIR / f"subagent-review-{task_id}.json"


def override_receipt_path(task_id: str) -> Path:
    return RECEIPT_DIR / f"{task_id}.json"


def is_tracked_fsap_task(task_id: str) -> bool:
    if not LOG_FILE.exists():
        return False
    saw_reflection_pack = False
    saw_fsap_plan = False
    with LOG_FILE.open() as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            if not isinstance(event, dict) or event.get("task_id") != task_id:
                continue
            if event.get("type") == "pack_start" and event.get("pack_id") == "reflection-pack":
                saw_reflection_pack = True
            if event.get("type") == "block_plan" and str(event.get("block_id", "")).startswith("FSAP"):
                saw_fsap_plan = True
            if saw_reflection_pack and saw_fsap_plan:
                return True
    return False


def write_override_receipt(
    task_id: str,
    reason: str,
    notes: str,
    *,
    evidence: str = "",
    actor: str = "",
) -> Path:
    normalized_reason = reason.strip()
    if normalized_reason not in ALLOWED_OVERRIDE_REASONS:
        raise ValueError(f"unsupported override reason: {normalized_reason}")
    payload = {
        "task_id": task_id,
        "reason": normalized_reason,
        "notes": notes.strip(),
        "evidence": evidence.strip(),
        "actor": actor.strip(),
        "ts": now_utc(),
    }
    return write_json(override_receipt_path(task_id), payload)


def check_gate(task_id: str) -> tuple[int, dict[str, Any]]:
    tracked_fsap = is_tracked_fsap_task(task_id)
    review_path = review_payload_path(task_id)
    review_payload = load_json(review_path)
    review_runs_seen = int(review_payload.get("subagent_runs_seen", 0) or 0)
    review_runs_processed = int(review_payload.get("subagent_runs_processed", 0) or 0)
    override_path = override_receipt_path(task_id)
    override_payload = load_json(override_path)
    override_reason = str(override_payload.get("reason", "") or "")
    override_valid = bool(override_payload) and override_reason in ALLOWED_OVERRIDE_REASONS

    payload = {
        "task_id": task_id,
        "tracked_fsap": tracked_fsap,
        "status": "ok",
        "authorized_via": "",
        "blockers": [],
        "review_path": str(review_path),
        "review_present": bool(review_payload),
        "review_status": str(review_payload.get("status", "") or ""),
        "review_evidence_present": bool(review_payload.get("review_evidence_present", False)),
        "review_subagent_runs_seen": review_runs_seen,
        "review_subagent_runs_processed": review_runs_processed,
        "override_receipt_path": str(override_path),
        "override_receipt_present": bool(override_payload),
        "override_receipt": override_payload,
    }

    if not tracked_fsap:
        payload["status"] = "not_required"
        payload["authorized_via"] = "not_tracked_fsap"
        return 0, payload
    if review_runs_processed > 0:
        payload["authorized_via"] = "delegated_review"
        return 0, payload
    if override_valid:
        payload["authorized_via"] = "structured_override"
        return 0, payload

    payload["status"] = "blocked"
    payload["authorized_via"] = "missing"
    payload["blockers"] = ["missing_delegated_fsap_verification"]
    if override_payload and not override_valid:
        payload["blockers"].append("invalid_structured_override")
    return 2, payload


def usage() -> str:
    return (
        "Usage:\n"
        "  python3 _vida/scripts/fsap-verification-gate.py check <task_id>\n"
        "  python3 _vida/scripts/fsap-verification-gate.py authorize-skip <task_id> <reason> <notes> [evidence] [actor]\n"
    )


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print(usage(), file=sys.stderr)
        return 1
    cmd = argv[1]
    if cmd == "check":
        task_id = argv[2]
        exit_code, payload = check_gate(task_id)
        print(json.dumps(payload, indent=2, sort_keys=True))
        return exit_code
    if cmd == "authorize-skip":
        if len(argv) < 5:
            print(usage(), file=sys.stderr)
            return 1
        task_id = argv[2]
        reason = argv[3]
        notes = argv[4]
        evidence = argv[5] if len(argv) > 5 else ""
        actor = argv[6] if len(argv) > 6 else ""
        try:
            path = write_override_receipt(task_id, reason, notes, evidence=evidence, actor=actor)
        except ValueError as exc:
            print(str(exc), file=sys.stderr)
            return 1
        print(path)
        return 0
    print(usage(), file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
