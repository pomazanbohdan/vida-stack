#!/usr/bin/env python3
"""Execution authorization gate for local writer execution."""

from __future__ import annotations

import hashlib
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


dispatch_runtime = load_module("vida_subagent_dispatch_runtime", SCRIPT_DIR / "subagent-dispatch.py")


def usage() -> int:
    print(
        "Usage:\n"
        "  python3 _vida/scripts/execution-auth-gate.py check <task_id> <task_class> [--local-write] [--block-id <id>]\n"
        "  python3 _vida/scripts/execution-auth-gate.py authorize-local <task_id> <task_class> <reason> <scope> <notes> [evidence] [actor]\n"
        "  python3 _vida/scripts/execution-auth-gate.py authorize-internal <task_id> <task_class> <reason> <scope> <notes> [evidence] [actor]",
        file=sys.stderr,
    )
    return 1


def now_utc() -> str:
    return dispatch_runtime.now_utc()


def json_hash(payload: dict[str, Any]) -> str:
    encoded = json.dumps(payload, sort_keys=True).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def safe_name(value: str, fallback: str) -> str:
    normalized = re.sub(r"[^A-Za-z0-9._-]+", "-", value.strip() or fallback)
    return normalized if normalized else fallback


def local_execution_receipt_path(task_id: str, task_class: str) -> Path:
    safe_task_id = safe_name(task_id, "task")
    safe_task_class = safe_name(task_class, "task_class")
    return ROUTE_RECEIPT_DIR / f"{safe_task_id}.{safe_task_class}.local-exec.json"


def execution_auth_receipt_path(task_id: str, task_class: str) -> Path:
    safe_task_id = safe_name(task_id, "task")
    safe_task_class = safe_name(task_class, "task_class")
    return ROUTE_RECEIPT_DIR / f"{safe_task_id}.{safe_task_class}.execution-auth.json"


def load_json(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {}
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return payload if isinstance(payload, dict) else {}


def write_json(path: Path, payload: dict[str, Any]) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def route_context(task_id: str, task_class: str) -> tuple[dict[str, Any], Path]:
    _, route = dispatch_runtime.route_snapshot(task_class, task_id)
    route_receipt_path = dispatch_runtime.write_route_receipt(task_id, task_class, route)
    return route, route_receipt_path


def validate_analysis_blocker(
    task_id: str,
    task_class: str,
    route: dict[str, Any],
) -> tuple[bool, dict[str, Any], str]:
    receipt = dispatch_runtime.load_analysis_blocker(task_id, task_class)
    if not receipt:
        return False, {}, "missing_analysis_receipt"

    if not str(receipt.get("reason", "")).strip():
        return False, receipt, "missing_analysis_blocker_reason"

    if str(receipt.get("status", "")).strip() not in {"analysis_failed", "blocked_missing_analysis_route"}:
        return False, receipt, "invalid_analysis_blocker_status"

    route_hash = dispatch_runtime.route_receipt_hash(route)
    if receipt.get("route_receipt_hash") != route_hash:
        return False, receipt, "stale_analysis_blocker"

    return True, receipt, ""


def validate_local_execution_receipt(
    task_id: str,
    task_class: str,
    route: dict[str, Any],
) -> tuple[bool, dict[str, Any], str]:
    receipt_path = local_execution_receipt_path(task_id, task_class)
    receipt = load_json(receipt_path)
    if not receipt:
        return False, {}, "missing_local_execution_receipt"

    if receipt.get("reason") != "emergency_override":
        return False, receipt, "invalid_local_execution_reason"
    if not str(receipt.get("scope", "")).strip():
        return False, receipt, "missing_local_execution_scope"
    if not str(receipt.get("notes", "")).strip():
        return False, receipt, "missing_local_execution_notes"

    route_hash = json_hash(dispatch_runtime.route_receipt_payload(route))
    if receipt.get("route_receipt_hash") != route_hash:
        return False, receipt, "stale_local_execution_receipt"

    return True, receipt, ""


def check_gate(
    task_id: str,
    task_class: str,
    *,
    local_write: bool,
    block_id: str,
) -> tuple[int, dict[str, Any]]:
    route, route_receipt = route_context(task_id, task_class)
    route_payload = dispatch_runtime.route_receipt_payload(route)
    analysis_plan = route.get("analysis_plan") or {}
    verification_plan = route.get("verification_plan") or {}
    dispatch_policy = route.get("dispatch_policy") or {}

    analysis_receipt = dispatch_runtime.load_analysis_receipt(task_id, task_class)
    analysis_receipt_path = dispatch_runtime.analysis_receipt_path(task_id, task_class)
    analysis_blocker_ok = False
    analysis_blocker, analysis_blocker_error = {}, ""
    issue_contract_ok, issue_contract, issue_contract_error = dispatch_runtime.validate_issue_contract(task_id, task_class, route)
    local_receipt_ok = False
    local_receipt, local_receipt_error = {}, ""
    local_allowed_by_route = dispatch_policy.get("local_execution_allowed") == "yes"
    analysis_prereq_via = "not_required"

    blockers: list[str] = []
    if analysis_plan.get("required") == "yes" and analysis_plan.get("receipt_required") == "yes":
        if analysis_receipt:
            analysis_prereq_via = "analysis_receipt"
        else:
            analysis_blocker_ok, analysis_blocker, analysis_blocker_error = validate_analysis_blocker(task_id, task_class, route)
            if analysis_blocker_ok:
                analysis_prereq_via = "analysis_blocker"
            else:
                blockers.append(analysis_blocker_error or "missing_analysis_receipt")

    if verification_plan.get("required") == "yes" and not verification_plan.get("selected_subagent"):
        blockers.append("missing_verifier_plan")

    if not issue_contract_ok:
        blockers.append(issue_contract_error or "missing_issue_contract")

    if local_write and not local_allowed_by_route:
        local_receipt_ok, local_receipt, local_receipt_error = validate_local_execution_receipt(task_id, task_class, route)
        if not local_receipt_ok:
            blockers.append(local_receipt_error)

    authorized_via = "route_local_execution" if local_allowed_by_route else ("local_emergency_override" if local_receipt_ok else "")
    payload = {
        "ts": now_utc(),
        "task_id": task_id,
        "task_class": task_class,
        "block_id": block_id or None,
        "status": "ok" if not blockers else "blocked",
        "local_write": local_write,
        "route_receipt_path": str(route_receipt),
        "analysis_receipt_path": str(analysis_receipt_path),
        "analysis_receipt_present": bool(analysis_receipt),
        "analysis_blocker_path": str(dispatch_runtime.analysis_blocker_path(task_id, task_class)),
        "analysis_blocker_present": bool(analysis_blocker),
        "analysis_prereq_via": analysis_prereq_via,
        "issue_contract_path": str(dispatch_runtime.issue_contract_path(task_id)),
        "issue_contract_present": bool(issue_contract),
        "issue_contract": issue_contract if issue_contract_ok else issue_contract,
        "local_execution_receipt_path": str(local_execution_receipt_path(task_id, task_class)),
        "local_execution_allowed": local_allowed_by_route,
        "local_execution_authorized": local_allowed_by_route or local_receipt_ok,
        "authorized_via": authorized_via,
        "required_dispatch_path": dispatch_policy.get("required_dispatch_path", []),
        "route_receipt": route_payload,
        "analysis_blocker": analysis_blocker if analysis_blocker_ok else {},
        "local_execution_receipt": local_receipt if local_receipt_ok else {},
        "blockers": blockers,
    }
    write_json(execution_auth_receipt_path(task_id, task_class), payload)
    return (0 if not blockers else 2), payload


def authorize_local(argv: list[str]) -> int:
    if len(argv) < 7:
        return usage()
    task_id, task_class, reason, scope, notes = argv[2:7]
    evidence = argv[7] if len(argv) > 7 else ""
    actor = argv[8] if len(argv) > 8 else "orchestrator"

    if reason != "emergency_override":
        print("[execution-auth-gate] only the explicit `emergency_override` reason is allowed", file=sys.stderr)
        return 1

    route, route_receipt = route_context(task_id, task_class)
    receipt_payload = {
        "ts": now_utc(),
        "task_id": task_id,
        "task_class": task_class,
        "reason": reason,
        "scope": scope,
        "notes": notes,
        "evidence": evidence,
        "actor": actor,
        "route_receipt_path": str(route_receipt),
        "route_receipt_hash": json_hash(dispatch_runtime.route_receipt_payload(route)),
        "analysis_receipt_present": bool(dispatch_runtime.load_analysis_receipt(task_id, task_class)),
    }
    path = write_json(local_execution_receipt_path(task_id, task_class), receipt_payload)
    print(str(path))
    return 0


def authorize_internal(argv: list[str]) -> int:
    if len(argv) < 7:
        return usage()
    task_id, task_class, reason, scope, notes = argv[2:7]
    evidence = argv[7] if len(argv) > 7 else ""
    actor = argv[8] if len(argv) > 8 else "orchestrator"

    route, route_receipt = route_context(task_id, task_class)
    dispatch_policy = route.get("dispatch_policy") or {}
    allowed_reasons = [str(item).strip() for item in dispatch_policy.get("allowed_internal_reasons", []) if str(item).strip()]
    if dispatch_policy.get("internal_escalation_allowed") != "yes":
        print("[execution-auth-gate] internal escalation is not allowed for this route", file=sys.stderr)
        return 1
    if allowed_reasons and reason not in allowed_reasons:
        print(
            f"[execution-auth-gate] invalid internal escalation reason: {reason} (allowed: {', '.join(allowed_reasons)})",
            file=sys.stderr,
        )
        return 1

    receipt_payload = {
        "ts": now_utc(),
        "task_id": task_id,
        "task_class": task_class,
        "reason": reason,
        "scope": scope,
        "notes": notes,
        "evidence": evidence,
        "actor": actor,
        "route_receipt_path": str(route_receipt),
        "route_receipt_hash": dispatch_runtime.route_receipt_hash(route),
        "allowed_reasons": allowed_reasons,
        "required_dispatch_path": dispatch_policy.get("required_dispatch_path", []),
        "analysis_receipt_present": bool(dispatch_runtime.load_analysis_receipt(task_id, task_class)),
    }
    path = write_json(dispatch_runtime.internal_escalation_receipt_path(task_id, task_class), receipt_payload)
    print(str(path))
    return 0


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        return usage()

    cmd = argv[1]
    if cmd == "authorize-local":
        return authorize_local(argv)
    if cmd == "authorize-internal":
        return authorize_internal(argv)

    if cmd != "check" or len(argv) < 4:
        return usage()

    task_id, task_class = argv[2:4]
    local_write = False
    block_id = ""
    idx = 4
    while idx < len(argv):
        arg = argv[idx]
        if arg == "--local-write":
            local_write = True
            idx += 1
            continue
        if arg == "--block-id" and idx + 1 < len(argv):
            block_id = argv[idx + 1]
            idx += 2
            continue
        return usage()

    exit_code, payload = check_gate(task_id, task_class, local_write=local_write, block_id=block_id)
    print(json.dumps(payload, indent=2, sort_keys=True))
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
