#!/usr/bin/env python3
"""Spec intake normalization helper for VIDA."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
LOG_DIR = ROOT_DIR / ".vida" / "logs" / "spec-intake"

ALLOWED_CLASSES = {"research", "issue", "release_signal", "user_negotiation", "mixed"}
ALLOWED_STATUS = {
    "ready_for_scp",
    "ready_for_issue_contract",
    "needs_user_negotiation",
    "needs_spec_delta",
    "insufficient_intake",
}
ALLOWED_PATHS = {"scp", "issue_contract", "spec_delta", "user_negotiation", "gather_evidence"}


def normalized_string(value: Any) -> str:
    if value is None:
        return ""
    return str(value).strip()


def normalized_string_list(value: Any) -> list[str]:
    if value is None:
        return []
    if isinstance(value, list):
        return [str(item).strip() for item in value if str(item).strip()]
    text = str(value).strip()
    return [text] if text else []


def intake_path(task_id: str) -> Path:
    return LOG_DIR / f"{task_id}.json"


def issue_like(path: str) -> bool:
    return path == "issue_contract"


def normalize_payload(task_id: str, payload: dict[str, Any]) -> dict[str, Any]:
    intake_class = normalized_string(payload.get("intake_class")).lower() or "mixed"
    if intake_class not in ALLOWED_CLASSES:
        intake_class = "mixed"
    status = normalized_string(payload.get("status")).lower() or "insufficient_intake"
    if status not in ALLOWED_STATUS:
        status = "insufficient_intake"
    recommended_contract_path = normalized_string(payload.get("recommended_contract_path")).lower() or "gather_evidence"
    if recommended_contract_path not in ALLOWED_PATHS:
        recommended_contract_path = "gather_evidence"
    normalized = {
        "task_id": normalized_string(payload.get("task_id")) or task_id,
        "intake_class": intake_class,
        "source_inputs": normalized_string_list(payload.get("source_inputs")),
        "problem_statement": normalized_string(payload.get("problem_statement")),
        "requested_outcome": normalized_string(payload.get("requested_outcome")),
        "research_findings": normalized_string_list(payload.get("research_findings")),
        "issue_signals": normalized_string_list(payload.get("issue_signals")),
        "release_signals": normalized_string_list(payload.get("release_signals")),
        "assumptions": normalized_string_list(payload.get("assumptions")),
        "proposed_scope_in": normalized_string_list(payload.get("proposed_scope_in")),
        "proposed_scope_out": normalized_string_list(payload.get("proposed_scope_out")),
        "open_decisions": normalized_string_list(payload.get("open_decisions")),
        "acceptance_checks": normalized_string_list(payload.get("acceptance_checks")),
        "recommended_contract_path": recommended_contract_path,
        "status": status,
    }
    return normalized


def validate_payload(payload: dict[str, Any], task_id: str) -> tuple[bool, str]:
    if payload.get("task_id") != task_id:
        return False, "task_id_mismatch"
    if payload.get("intake_class") not in ALLOWED_CLASSES:
        return False, "invalid_intake_class"
    if payload.get("status") not in ALLOWED_STATUS:
        return False, "invalid_status"
    if payload.get("recommended_contract_path") not in ALLOWED_PATHS:
        return False, "invalid_recommended_contract_path"
    if not payload.get("problem_statement"):
        return False, "missing_problem_statement"
    if not payload.get("requested_outcome"):
        return False, "missing_requested_outcome"
    if payload.get("status") in {"ready_for_scp", "ready_for_issue_contract", "needs_spec_delta"} and not payload.get("proposed_scope_in"):
        return False, "missing_proposed_scope_in"
    if payload.get("status") == "needs_user_negotiation" and not payload.get("open_decisions"):
        return False, "missing_open_decisions"
    if payload.get("status") == "ready_for_issue_contract" and not issue_like(payload.get("recommended_contract_path", "")):
        return False, "issue_contract_path_required"
    if payload.get("status") == "needs_spec_delta" and payload.get("recommended_contract_path") != "spec_delta":
        return False, "spec_delta_path_required"
    if payload.get("status") == "insufficient_intake" and payload.get("recommended_contract_path") != "gather_evidence":
        return False, "gather_evidence_path_required"
    return True, "ok"


def write_payload(task_id: str, input_path: Path, output_path: Path | None) -> int:
    raw = json.loads(input_path.read_text(encoding="utf-8"))
    if not isinstance(raw, dict):
        print("[spec-intake] input must be a JSON object", file=sys.stderr)
        return 2
    payload = normalize_payload(task_id, raw)
    ok, reason = validate_payload(payload, task_id)
    if not ok:
        print(f"[spec-intake] {reason}", file=sys.stderr)
        return 2
    path = output_path or intake_path(task_id)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(str(path))
    return 0


def load_payload(task_id: str, path: Path | None) -> tuple[dict[str, Any], Path]:
    selected = path or intake_path(task_id)
    if not selected.exists():
        raise FileNotFoundError(selected)
    payload = json.loads(selected.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError("payload must be an object")
    return payload, selected


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print(
            "Usage: python3 _vida/scripts/spec-intake.py write <task_id> <input.json> [--output PATH] | "
            "validate <task_id> [--path PATH] | status <task_id> [--path PATH]",
            file=sys.stderr,
        )
        return 2
    command = argv[1]
    task_id = argv[2]
    if command == "write":
        if len(argv) < 4:
            print("[spec-intake] missing input path", file=sys.stderr)
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
            print(f"[spec-intake] missing file: {exc}", file=sys.stderr)
            return 1
        except ValueError as exc:
            print(f"[spec-intake] {exc}", file=sys.stderr)
            return 2
        payload = normalize_payload(task_id, payload)
        ok, reason = validate_payload(payload, task_id)
        if command == "validate":
            if not ok:
                print(f"[spec-intake] {reason}", file=sys.stderr)
                return 2
            print(f"OK {selected}")
            return 0
        status_payload = {
            "path": str(selected),
            "valid": ok,
            "reason": reason,
            "status": payload.get("status", ""),
            "recommended_contract_path": payload.get("recommended_contract_path", ""),
            "open_decisions": payload.get("open_decisions", []),
        }
        print(json.dumps(status_payload, indent=2, sort_keys=True))
        return 0 if ok else 2
    print(f"[spec-intake] unknown command: {command}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
