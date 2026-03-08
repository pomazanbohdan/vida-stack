#!/usr/bin/env python3
"""Silent VIDA framework self-diagnosis state and bug-capture helper."""

from __future__ import annotations

import argparse
import importlib.util
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
STATE_PATH = ROOT_DIR / ".vida" / "state" / "silent-framework-diagnosis.json"
ISSUES_JSONL_PATH = ROOT_DIR / ".beads" / "issues.jsonl"
QUEUE_RUNNER = SCRIPT_DIR / "br-mutation-queue.py"
FRAMEWORK_MEMORY_MODULE = None


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def load_config() -> dict[str, Any]:
    vida_config = load_module("vida_config_for_silent_diag", SCRIPT_DIR / "vida-config.py")
    try:
        return vida_config.load_validated_config()
    except Exception:
        return {}


def framework_memory_module():
    global FRAMEWORK_MEMORY_MODULE
    if FRAMEWORK_MEMORY_MODULE is None:
        FRAMEWORK_MEMORY_MODULE = load_module("vida_framework_memory", SCRIPT_DIR / "framework-memory.py")
    return FRAMEWORK_MEMORY_MODULE


def diagnosis_config(cfg: dict[str, Any] | None = None) -> dict[str, Any]:
    cfg = cfg or load_config()
    payload = cfg.get("framework_self_diagnosis")
    if not isinstance(payload, dict):
        return {}
    return payload


def load_state() -> dict[str, Any]:
    if not STATE_PATH.exists():
        return {"pending_framework_bugs": [], "session_reflections": []}
    try:
        payload = json.loads(STATE_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {"pending_framework_bugs": [], "session_reflections": []}
    if not isinstance(payload, dict):
        return {"pending_framework_bugs": [], "session_reflections": []}
    payload.setdefault("pending_framework_bugs", [])
    payload.setdefault("session_reflections", [])
    return payload


def issue_status_map() -> dict[str, str]:
    if not ISSUES_JSONL_PATH.exists():
        return {}
    statuses: dict[str, str] = {}
    try:
        lines = ISSUES_JSONL_PATH.read_text(encoding="utf-8").splitlines()
    except OSError:
        return {}
    for line in lines:
        line = line.strip()
        if not line:
            continue
        try:
            item = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(item, dict):
            continue
        issue_id = str(item.get("id", "")).strip()
        status = str(item.get("status", "")).strip()
        if issue_id:
            statuses[issue_id] = status
    return statuses


def reconcile_pending_framework_bugs(state: dict[str, Any]) -> dict[str, Any]:
    pending = state.get("pending_framework_bugs", [])
    if not isinstance(pending, list):
        state["pending_framework_bugs"] = []
        return state
    statuses = issue_status_map()
    if not statuses:
        return state
    filtered: list[dict[str, Any]] = []
    changed = False
    for item in pending:
        if not isinstance(item, dict):
            changed = True
            continue
        bug_id = str(item.get("bug_id", "")).strip()
        if bug_id and statuses.get(bug_id) == "closed":
            changed = True
            continue
        filtered.append(item)
    if changed:
        state["pending_framework_bugs"] = filtered
    return state


def save_state(payload: dict[str, Any]) -> None:
    payload = reconcile_pending_framework_bugs(payload)
    STATE_PATH.parent.mkdir(parents=True, exist_ok=True)
    STATE_PATH.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def capture_bug(
    *,
    summary: str,
    details: str,
    current_task: str,
    workaround: str,
    parent_issue: str,
) -> dict[str, Any]:
    state = load_state()
    fingerprint = summary.strip().casefold()
    for item in state.get("pending_framework_bugs", []):
        if not isinstance(item, dict):
            continue
        if str(item.get("fingerprint", "")).strip() == fingerprint:
            return item

    title = f"VIDA silent diagnosis: {summary.strip()}"
    description_lines = [
        "Captured by silent VIDA framework diagnosis.",
        "",
        f"Summary: {summary.strip()}",
    ]
    if current_task:
        description_lines.append(f"Current task at detection: {current_task}")
    if workaround:
        description_lines.append(f"Current-task workaround: {workaround}")
    if details:
        description_lines.extend(["", "Details:", details.strip()])
    description_lines.extend(
        [
            "",
            "Expected follow-up:",
            "1. finish the current task without silently hard-coding the framework workaround into VIDA;",
            "2. return to this framework bug after the task boundary;",
            "3. run web-backed research for the best applicable fix pattern;",
            "4. implement and verify the systemic VIDA fix;",
            "5. resume product work after framework closure.",
        ]
    )
    command = [
        sys.executable,
        str(QUEUE_RUNNER),
        "br",
        "--",
        "create",
        title,
        "--type",
        "bug",
        "--priority",
        "1",
        "--labels",
        "framework,vida-silent-diagnosis,mode:autonomous",
        "--description",
        "\n".join(description_lines),
        "--parent",
        parent_issue,
        "--no-db",
        "--json",
    ]
    completed = subprocess.run(
        command,
        cwd=str(ROOT_DIR),
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        raise SystemExit((completed.stderr or completed.stdout).strip() or "failed to create framework bug")
    payload = json.loads((completed.stdout or "{}").strip() or "{}")
    entry = {
        "bug_id": str(payload.get("id", "")).strip(),
        "title": title,
        "summary": summary.strip(),
        "fingerprint": fingerprint,
        "current_task": current_task.strip(),
        "workaround": workaround.strip(),
        "created_at": now_utc(),
        "status": "pending_follow_up",
    }
    state.setdefault("pending_framework_bugs", []).append(entry)
    save_state(state)
    try:
        framework_memory_module().record_entry(
            kind="anomaly",
            summary=summary.strip(),
            source_task=current_task.strip(),
            details={"source": "silent_framework_diagnosis", "bug_id": entry["bug_id"]},
        )
    except Exception:
        pass
    return entry


def record_session_reflection(current_task: str, criteria: list[str], gaps: list[str]) -> dict[str, Any]:
    state = load_state()
    entry = {
        "ts": now_utc(),
        "current_task": current_task.strip(),
        "criteria": [item.strip() for item in criteria if item.strip()],
        "gaps": [item.strip() for item in gaps if item.strip()],
    }
    state.setdefault("session_reflections", []).append(entry)
    save_state(state)
    for gap in entry["gaps"]:
        try:
            framework_memory_module().record_entry(
                kind="anomaly",
                summary=gap,
                source_task=current_task.strip(),
                details={"source": "session_reflection", "criteria": entry["criteria"]},
            )
        except Exception:
            continue
    return entry


def build_status_payload() -> dict[str, Any]:
    cfg = diagnosis_config()
    state = reconcile_pending_framework_bugs(load_state())
    return {
        "generated_at": now_utc(),
        "enabled": bool(cfg.get("enabled", False)),
        "silent_mode": bool(cfg.get("silent_mode", False)),
        "auto_capture_bugs": bool(cfg.get("auto_capture_bugs", False)),
        "parent_issue": str(cfg.get("parent_issue", "")).strip(),
        "defer_fix_until_task_boundary": bool(cfg.get("defer_fix_until_task_boundary", False)),
        "session_reflection_required": bool(cfg.get("session_reflection_required", False)),
        "platform_direction": str(cfg.get("platform_direction", "")).strip(),
        "quality_token_efficiency": str(cfg.get("quality_token_efficiency", "")).strip(),
        "session_reflection_criteria": [
            str(item).strip() for item in (cfg.get("session_reflection_criteria") or []) if str(item).strip()
        ],
        "pending_framework_bugs": state.get("pending_framework_bugs", []),
        "session_reflections": state.get("session_reflections", []),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("status")

    capture = sub.add_parser("capture")
    capture.add_argument("--summary", required=True)
    capture.add_argument("--details", default="")
    capture.add_argument("--current-task", default="")
    capture.add_argument("--workaround", default="")
    capture.add_argument("--parent-issue", default="")

    reflect = sub.add_parser("session-reflect")
    reflect.add_argument("--current-task", default="")
    reflect.add_argument("--criteria", default="")
    reflect.add_argument("--gap", action="append", default=[])

    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.command == "status":
        print(json.dumps(build_status_payload(), indent=2, sort_keys=True))
        return 0
    if args.command == "capture":
        cfg = diagnosis_config()
        parent_issue = args.parent_issue or str(cfg.get("parent_issue", "")).strip() or "mobile-1hv"
        entry = capture_bug(
            summary=args.summary,
            details=args.details,
            current_task=args.current_task,
            workaround=args.workaround,
            parent_issue=parent_issue,
        )
        print(json.dumps(entry, indent=2, sort_keys=True))
        return 0
    if args.command == "session-reflect":
        cfg = diagnosis_config()
        criteria = [item.strip() for item in (args.criteria.split(",") if args.criteria else cfg.get("session_reflection_criteria", [])) if item.strip()]
        payload = record_session_reflection(args.current_task, criteria, args.gap)
        print(json.dumps(payload, indent=2, sort_keys=True))
        return 0
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
