#!/usr/bin/env python3
"""Reconcile br/TODO/runtime evidence into one task-state classification."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
ISSUES_JSONL = ROOT_DIR / ".beads" / "issues.jsonl"
TODO_TOOL = SCRIPT_DIR / "todo-tool.sh"
BOOT_PROFILE = SCRIPT_DIR / "boot-profile.sh"
VERIFY_LOG = SCRIPT_DIR / "beads-verify-log.sh"
RUN_GRAPH = SCRIPT_DIR / "run-graph.py"


def load_module(name: str, path: Path):
    import importlib.util

    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def load_issue(task_id: str) -> dict[str, Any]:
    if not ISSUES_JSONL.exists():
        return {}
    for raw_line in ISSUES_JSONL.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line:
            continue
        try:
            payload = json.loads(line)
        except json.JSONDecodeError:
            continue
        if str(payload.get("id", "")).strip() == task_id:
            return payload if isinstance(payload, dict) else {}
    return {}


def run_json_command(command: list[str]) -> dict[str, Any]:
    completed = subprocess.run(
        command,
        cwd=ROOT_DIR,
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        return {}
    stdout = completed.stdout.strip()
    if not stdout:
        return {}
    try:
        payload = json.loads(stdout)
    except json.JSONDecodeError:
        return {}
    return payload if isinstance(payload, dict) else {}


def todo_payload(task_id: str) -> dict[str, Any]:
    return run_json_command(["bash", str(TODO_TOOL), "ui-json", task_id])


def verify_boot_receipt(task_id: str) -> bool:
    completed = subprocess.run(
        ["bash", str(BOOT_PROFILE), "verify-receipt", task_id],
        cwd=ROOT_DIR,
        capture_output=True,
        text=True,
        check=False,
    )
    return completed.returncode == 0


def verify_log_ok(task_id: str) -> bool:
    completed = subprocess.run(
        ["bash", str(VERIFY_LOG), "--task", task_id],
        cwd=ROOT_DIR,
        capture_output=True,
        text=True,
        check=False,
    )
    return completed.returncode == 0


def run_graph_state(task_id: str) -> dict[str, Any]:
    try:
        module = load_module("task_state_reconcile_run_graph", RUN_GRAPH)
        payload = module.status_payload(task_id)
    except Exception:
        return {}
    return payload if isinstance(payload, dict) else {}


def step_counts(steps: list[dict[str, Any]]) -> dict[str, int]:
    counts = {
        "todo": 0,
        "doing": 0,
        "done": 0,
        "blocked": 0,
        "superseded": 0,
        "partial": 0,
    }
    for item in steps:
        if not isinstance(item, dict):
            continue
        status = str(item.get("status", "")).strip()
        if status in counts:
            counts[status] += 1
    return counts


def classify_state(
    *,
    issue_status: str,
    steps: list[dict[str, Any]],
    boot_receipt_ok: bool,
    verify_ok: bool,
    run_graph: dict[str, Any],
) -> tuple[str, list[str], list[str]]:
    counts = step_counts(steps)
    has_steps = bool(steps)
    active_run_graph = bool(run_graph.get("present")) and bool((run_graph.get("resume_hint") or {}).get("status") in {"running", "ready", "blocked"})
    terminal_done_exists = any(
        isinstance(item, dict)
        and str(item.get("status", "")).strip() == "done"
        and str(item.get("next_step", "")).strip() == "-"
        for item in steps
    )

    if issue_status == "closed":
        if counts["doing"] or counts["todo"] or counts["blocked"]:
            return (
                "invalid_state",
                ["closed task still has active TODO backlog"],
                ["manual_review"],
            )
        return ("closed", [], ["none"])

    if counts["blocked"]:
        return ("blocked", ["TODO block is blocked"], ["unblock_or_escalate"])

    if counts["doing"] or active_run_graph:
        return ("active", [], ["continue_current_block"])

    if issue_status == "in_progress" and terminal_done_exists and counts["todo"] > 0:
        return (
            "drift_detected",
            ["terminal done block exists but TODO backlog still remains"],
            ["reconcile_todo_then_close_or_manual_review"],
        )

    if issue_status == "in_progress" and counts["todo"] > 0:
        return (
            "stale_in_progress",
            ["task is in_progress but no active block is running"],
            ["resume_next_block", "or_reconcile_br"],
        )

    if has_steps and counts["todo"] == 0 and counts["doing"] == 0 and counts["blocked"] == 0:
        if issue_status == "in_progress":
            if verify_ok:
                return (
                    "done_ready_to_close",
                    [] if boot_receipt_ok else ["boot receipt missing"],
                    ["close_now"],
                )
            return (
                "drift_detected",
                ["all TODO blocks ended but verify evidence is missing"],
                ["verify_then_close_or_manual_review"],
            )
        if issue_status == "open":
            return (
                "open_but_satisfied",
                [] if verify_ok else ["verify evidence missing"],
                ["close_now_if_scope_satisfied", "or_mark_in_progress_before_resume"],
            )

    if issue_status == "in_progress" and not has_steps:
        return (
            "stale_in_progress",
            ["in_progress task has no TODO execution trace"],
            ["start_or_reconcile"],
        )

    if issue_status == "open" and terminal_done_exists and counts["todo"] > 0:
        return (
            "drift_detected",
            ["terminal done block exists but open backlog still remains"],
            ["reconcile_todo_or_scope"],
        )

    return ("active" if issue_status == "in_progress" else "open", [], ["continue"])


def build_status_payload(task_id: str) -> dict[str, Any]:
    issue = load_issue(task_id)
    issue_status = str(issue.get("status", "")).strip()
    title = str(issue.get("title", "")).strip()
    todo = todo_payload(task_id)
    steps = todo.get("steps", []) if isinstance(todo, dict) else []
    if not isinstance(steps, list):
        steps = []
    boot_ok = verify_boot_receipt(task_id)
    verify_ok = verify_log_ok(task_id)
    graph = run_graph_state(task_id)
    classification, reasons, allowed_actions = classify_state(
        issue_status=issue_status,
        steps=steps,
        boot_receipt_ok=boot_ok,
        verify_ok=verify_ok,
        run_graph=graph,
    )
    counts = step_counts(steps)
    current = next(
        (
            {
                "block_id": str(item.get("block_id", "")).strip(),
                "goal": str(item.get("goal", "")).strip(),
            }
            for item in steps
            if isinstance(item, dict) and str(item.get("status", "")).strip() == "doing"
        ),
        None,
    )
    return {
        "task_id": task_id,
        "title": title,
        "issue_status": issue_status,
        "classification": classification,
        "reasons": reasons,
        "allowed_actions": allowed_actions,
        "boot_receipt_ok": boot_ok,
        "verify_ok": verify_ok,
        "todo_counts": counts,
        "current_block": current,
        "run_graph": {
            "present": bool(graph.get("present")),
            "resume_hint": graph.get("resume_hint", {}),
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)

    status_parser = sub.add_parser("status")
    status_parser.add_argument("task_id")

    args = parser.parse_args()
    if args.command == "status":
        payload = build_status_payload(args.task_id)
        print(json.dumps(payload, indent=2, sort_keys=True))
        return 0
    return 1


if __name__ == "__main__":
    sys.exit(main())
