#!/usr/bin/env python3
"""Render a compact task-state snapshot for dev-oriented boot paths."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
BR_SAFE = SCRIPT_DIR / "br-safe.sh"


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def run_br_json(*args: str) -> list[dict[str, Any]]:
    env = os.environ.copy()
    env.setdefault("RUST_LOG", "error")
    proc = subprocess.run(
        ["bash", str(BR_SAFE), *args],
        cwd=ROOT_DIR,
        capture_output=True,
        text=True,
        env=env,
        check=False,
    )
    if proc.returncode != 0:
        detail = (proc.stderr or proc.stdout).strip()
        raise SystemExit(f"[vida-boot-snapshot] br call failed: {' '.join(args)}\n{detail}")
    payload = proc.stdout.strip() or "[]"
    try:
        data = json.loads(payload)
    except json.JSONDecodeError as exc:
        raise SystemExit(
            f"[vida-boot-snapshot] Failed to decode JSON from br call: {' '.join(args)}"
        ) from exc
    if not isinstance(data, list):
        raise SystemExit(f"[vida-boot-snapshot] Unexpected br payload type for: {' '.join(args)}")
    return data


def show_issue(issue_id: str) -> dict[str, Any]:
    rows = run_br_json("show", issue_id, "--json")
    if not rows:
        raise SystemExit(f"[vida-boot-snapshot] Missing issue in br show: {issue_id}")
    return rows[0]


def clean_text(value: Any) -> str:
    if value is None:
        return "-"
    text = str(value).replace("\n", " ").replace("\r", " ").replace("\t", " ")
    return " ".join(text.split()) or "-"


def issue_mode(issue: dict[str, Any]) -> str:
    labels = issue.get("labels") or []
    if "mode:decision_required" in labels:
        return "decision_required"
    if "mode:autonomous" in labels:
        return "autonomous"
    return "auto"


def top_level(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return [row for row in rows if not row.get("parent")]


def unique_by_id(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    seen: set[str] = set()
    out: list[dict[str, Any]] = []
    for row in rows:
        issue_id = str(row.get("id") or "")
        if not issue_id or issue_id in seen:
            continue
        seen.add(issue_id)
        out.append(row)
    return out


def issue_entry(issue: dict[str, Any], subtasks: list[dict[str, Any]], subtasks_limit: int) -> dict[str, Any]:
    shown_subtasks = subtasks[:subtasks_limit]
    return {
        "id": issue.get("id"),
        "title": clean_text(issue.get("title")),
        "status": issue.get("status"),
        "mode": issue_mode(issue),
        "subtasks": [
            {
                "id": child.get("id"),
                "title": clean_text(child.get("title")),
                "status": child.get("status"),
            }
            for child in shown_subtasks
        ],
        "hidden_subtasks": max(0, len(subtasks) - len(shown_subtasks)),
    }


def child_entries_for(issue_id: str) -> list[dict[str, Any]]:
    issue = show_issue(issue_id)
    children = []
    for dependent in issue.get("dependents") or []:
        if dependent.get("dependency_type") != "parent-child":
            continue
        if dependent.get("status") not in {"open", "in_progress"}:
            continue
        children.append(
            {
                "id": dependent.get("id"),
                "title": dependent.get("title"),
                "status": dependent.get("status"),
            }
        )
    children.sort(key=lambda item: (0 if item.get("status") == "in_progress" else 1, item.get("id") or ""))
    return children


def build_snapshot(top_limit: int, ready_limit: int, subtasks_limit: int) -> dict[str, Any]:
    open_rows = run_br_json("list", "--status", "open", "--json")
    doing_rows = run_br_json("list", "--status", "in_progress", "--json")
    blocked_rows = run_br_json("list", "--status", "blocked", "--json")
    ready_rows = run_br_json("ready", "--json")

    top_open = sorted(top_level(open_rows), key=lambda item: item.get("id") or "")
    top_doing = sorted(top_level(doing_rows), key=lambda item: item.get("id") or "")
    top_blocked = sorted(top_level(blocked_rows), key=lambda item: item.get("id") or "")
    top_ready = unique_by_id(sorted(top_level(ready_rows), key=lambda item: item.get("id") or ""))
    top_ready_open = [row for row in top_ready if row.get("status") == "open"]
    top_ready_in_progress = [row for row in top_ready if row.get("status") == "in_progress"]

    in_progress = [
        issue_entry(row, child_entries_for(str(row.get("id"))), subtasks_limit)
        for row in top_doing[:top_limit]
    ]
    ready_head = [
        issue_entry(row, child_entries_for(str(row.get("id"))), subtasks_limit)
        for row in top_ready_open[:ready_limit]
    ]
    decision_required = [
        {
            "id": row.get("id"),
            "title": clean_text(row.get("title")),
            "status": row.get("status"),
        }
        for row in unique_by_id(top_doing + top_open + top_blocked)
        if issue_mode(row) == "decision_required"
    ]

    return {
        "generated_at": now_utc(),
        "summary": {
            "top_level_in_progress": len(top_doing),
            "top_level_open": len(top_open),
            "top_level_blocked": len(top_blocked),
            "ready_total": len(top_ready),
            "ready_open": len(top_ready_open),
            "ready_in_progress": len(top_ready_in_progress),
        },
        "in_progress": in_progress,
        "ready_head": ready_head,
        "decision_required": decision_required,
        "limits": {
            "top_level": top_limit,
            "ready_head": ready_limit,
            "subtasks": subtasks_limit,
        },
    }


def render_section(lines: list[str], name: str, items: list[dict[str, Any]]) -> None:
    if not items:
        return
    lines.append("")
    lines.append(f"{name}:")
    for item in items:
        suffix = ""
        if item.get("mode") == "decision_required":
            suffix = "  [decision_required]"
        lines.append(f"- {item['id']}  {item['title']}{suffix}")
        for child in item.get("subtasks", []):
            lines.append(f"  - [{child['status']}] {child['id']}  {child['title']}")
        hidden = int(item.get("hidden_subtasks") or 0)
        if hidden > 0:
            lines.append(f"  - +{hidden} more")


def render_text(snapshot: dict[str, Any]) -> str:
    summary = snapshot["summary"]
    lines = [
        "VIDA BOOT SNAPSHOT",
        (
            "summary: "
            f"in_progress={summary['top_level_in_progress']} "
            f"open={summary['top_level_open']} "
            f"blocked={summary['top_level_blocked']} "
            f"ready_total={summary['ready_total']} "
            f"ready_open={summary['ready_open']}"
        ),
    ]
    render_section(lines, "in_progress", snapshot["in_progress"])
    render_section(lines, "ready_head", snapshot["ready_head"])
    if snapshot["decision_required"]:
        lines.append("")
        lines.append("decision_required:")
        for item in snapshot["decision_required"]:
            lines.append(f"- [{item['status']}] {item['id']}  {item['title']}")
    return "\n".join(lines)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON")
    parser.add_argument("--top-limit", type=int, default=5, help="Max top-level in-progress tasks")
    parser.add_argument("--ready-limit", type=int, default=3, help="Max top-level ready tasks")
    parser.add_argument("--subtasks-limit", type=int, default=5, help="Max open/in-progress subtasks per parent")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    if args.top_limit < 1 or args.ready_limit < 0 or args.subtasks_limit < 0:
        print("[vida-boot-snapshot] Limits must be non-negative and top-limit >= 1", file=sys.stderr)
        return 1
    snapshot = build_snapshot(args.top_limit, args.ready_limit, args.subtasks_limit)
    if args.json:
        print(json.dumps(snapshot, indent=2, sort_keys=True))
    else:
        print(render_text(snapshot))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
