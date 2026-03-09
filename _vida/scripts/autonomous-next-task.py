#!/usr/bin/env python3
"""Select the next lawful task for autonomous follow-through when dependency readiness is unreliable."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
VIDA_LEGACY_BIN = ROOT / "_vida" / "scripts-nim" / "vida-legacy"
TURSO_PYTHON = str(ROOT / ".venv" / "bin" / "python3")


def vida_legacy_list() -> list[dict]:
    if not VIDA_LEGACY_BIN.exists():
        raise SystemExit(
            f"vida-legacy binary is missing: {VIDA_LEGACY_BIN}. Build it before autonomous-next-task."
        )
    env = os.environ.copy()
    env.setdefault("VIDA_ROOT", str(ROOT))
    env.setdefault("VIDA_LEGACY_TURSO_PYTHON", TURSO_PYTHON)
    cmd = [str(VIDA_LEGACY_BIN), "task", "list", "--all", "--json"]
    completed = subprocess.run(cmd, cwd=ROOT, env=env, capture_output=True, text=True, check=False)
    if completed.returncode != 0:
        raise SystemExit((completed.stderr or completed.stdout).strip() or "vida-legacy task list failed")
    return json.loads(completed.stdout or "[]")


def todo_steps(task_id: str) -> list[dict]:
    cmd = ["python3", str(ROOT / "_vida" / "scripts" / "todo-runtime.py"), "ui-json", task_id]
    completed = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=False)
    if completed.returncode != 0:
        stderr = (completed.stderr or completed.stdout).strip()
        if stderr:
            print(
                f"[autonomous-next-task] todo ui-json failed for {task_id}: {stderr}",
                file=sys.stderr,
            )
        return []
    try:
        payload = json.loads(completed.stdout or "{}")
    except json.JSONDecodeError:
        print(
            f"[autonomous-next-task] todo ui-json returned invalid JSON for {task_id}",
            file=sys.stderr,
        )
        return []
    return payload.get("steps") or []


def sort_key(issue_id: str) -> tuple:
    parts = []
    for token in issue_id.split("."):
        if token.isdigit():
            parts.append((0, int(token)))
        else:
            parts.append((1, token))
    return tuple(parts)


def matches_scope(issue: dict, prefix: str, labels: list[str]) -> bool:
    issue_id = str(issue.get("id", ""))
    if prefix and not issue_id.startswith(prefix):
        return False
    if labels:
        issue_labels = set(issue.get("labels") or [])
        if not all(label in issue_labels for label in labels):
            return False
    return True


def choose_next(issues: list[dict], prefix: str, labels: list[str], include_epics: bool) -> dict | None:
    scoped = [
        issue
        for issue in issues
        if matches_scope(issue, prefix, labels) and issue.get("status") in {"open", "in_progress"}
    ]
    if not include_epics:
        scoped = [issue for issue in scoped if issue.get("issue_type") != "epic"]

    with_todo = [issue for issue in scoped if todo_steps(str(issue.get("id", "")))]
    candidates = with_todo or scoped

    in_progress = sorted(
        [issue for issue in candidates if issue.get("status") == "in_progress"],
        key=lambda item: sort_key(str(item.get("id", ""))),
    )
    if in_progress:
        return in_progress[0]

    open_items = sorted(
        [issue for issue in candidates if issue.get("status") == "open"],
        key=lambda item: (sort_key(str(item.get("id", ""))), item.get("priority", 9)),
    )
    if open_items:
        return open_items[0]
    return None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--prefix", required=True)
    parser.add_argument("--label", action="append", default=[])
    parser.add_argument("--include-epics", action="store_true")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    selected = choose_next(vida_legacy_list(), args.prefix.strip(), args.label, args.include_epics)
    if args.json:
        print(json.dumps(selected or {}, indent=2))
    elif selected:
        print(f"{selected['id']}\t{selected.get('status','')}\t{selected.get('title','')}")
    return 0 if selected else 1


if __name__ == "__main__":
    raise SystemExit(main())
