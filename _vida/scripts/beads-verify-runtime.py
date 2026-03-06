#!/usr/bin/env python3
"""Python engine for VIDA beads log verification."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


ROOT_DIR = Path(__file__).resolve().parents[2]
LOG_FILE = ROOT_DIR / ".vida" / "logs" / "beads-execution.jsonl"
BR_SAFE_SCRIPT = ROOT_DIR / "_vida" / "scripts" / "br-safe.sh"


def icon(level: str) -> str:
    return {
        "ok": "✅",
        "warn": "⚠️",
        "fail": "❌",
        "blocked": "⛔",
        "info": "ℹ️",
        "sparkle": "✨",
        "progress": "🔄",
    }.get(level, "•")


def status_line(level: str, message: str) -> None:
    print(f"{icon(level)} {message}")


def parse_iso_utc(raw: str) -> datetime | None:
    if not raw:
        return None
    if raw.endswith("Z"):
        raw = raw[:-1] + "+00:00"
    try:
        return datetime.fromisoformat(raw).astimezone(UTC)
    except ValueError:
        return None


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    events: list[dict[str, Any]] = []
    with path.open() as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            events.append(json.loads(line))
    return events


def task_logs(task_id: str) -> list[dict[str, Any]]:
    return [
        event
        for event in read_jsonl(LOG_FILE)
        if event.get("task_id") == task_id
        or event.get("task_before") == task_id
        or event.get("task_after") == task_id
    ]


def show_task_status(task_id: str) -> str:
    if not BR_SAFE_SCRIPT.exists():
        return ""
    proc = subprocess.run(
        ["bash", str(BR_SAFE_SCRIPT), "show", task_id, "--json"],
        cwd=ROOT_DIR,
        text=True,
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        return ""
    try:
        payload = json.loads(proc.stdout)
    except json.JSONDecodeError:
        return ""
    if isinstance(payload, list) and payload:
        return str(payload[0].get("status") or "")
    if isinstance(payload, dict):
        return str(payload.get("status") or "")
    return ""


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="beads-verify-log.sh",
        description="Verify VIDA beads execution logs for a task.",
    )
    parser.add_argument("--task", required=True)
    parser.add_argument("--strict", action="store_true")
    parser.add_argument("--assumption-hours", type=int, default=8)
    return parser


def main(argv: list[str]) -> int:
    args = build_parser().parse_args(argv[1:])

    if not LOG_FILE.exists():
        if args.strict:
            print(f"[beads-verify-log] CRITICAL: log file not found: {LOG_FILE}")
            return 2
        print("[beads-verify-log] WARN: no log file yet")
        return 0

    logs = task_logs(args.task)
    entry_count = len(logs)
    if entry_count == 0:
        if args.strict:
            print(f"[beads-verify-log] CRITICAL: no log entries for {args.task}")
            return 2
        print(f"[beads-verify-log] WARN: no log entries for {args.task}")
        return 0

    critical_count = 0
    warn_count = 0

    missing_next_done = sum(
        1
        for event in logs
        if event.get("type") == "block_end"
        and event.get("result") == "done"
        and not (event.get("next_step") or "")
    )
    if missing_next_done:
        status_line("fail", f"[beads-verify-log] CRITICAL: block_end(done) missing next_step: {missing_next_done}")
        critical_count += missing_next_done

    task_status = show_task_status(args.task)
    if task_status == "closed":
        done_count = sum(
            1 for event in logs if event.get("type") == "block_end" and event.get("result") == "done"
        )
        if done_count == 0:
            status_line("fail", "[beads-verify-log] CRITICAL: task is closed but no block_end(done) entries")
            critical_count += 1

    now = datetime.now(UTC)
    threshold_seconds = args.assumption_hours * 3600
    stale_assumptions = 0
    for event in logs:
        if event.get("type") != "block_end" or not (event.get("assumptions") or ""):
            continue
        stamp = parse_iso_utc(str(event.get("ts_end") or event.get("ts") or ""))
        if stamp is None:
            continue
        if int((now - stamp).total_seconds()) > threshold_seconds:
            stale_assumptions += 1
    if stale_assumptions:
        status_line("warn", f"[beads-verify-log] WARN: stale unresolved assumptions: {stale_assumptions}")
        warn_count += stale_assumptions

    missing_evidence = sum(
        1
        for event in logs
        if event.get("type") == "block_end"
        and (event.get("actions") or "")
        and not (event.get("artifacts") or "")
        and not (event.get("evidence_ref") or "")
    )
    if missing_evidence:
        status_line(
            "warn",
            f"[beads-verify-log] WARN: block_end actions without artifacts/evidence_ref: {missing_evidence}",
        )
        warn_count += missing_evidence

    compact_switch_no_recovery = sum(
        1
        for event in logs
        if event.get("type") == "compact_post"
        and (event.get("task_before") or "") != (event.get("task_after") or "")
        and not (event.get("recovery_action") or "")
    )
    if compact_switch_no_recovery:
        status_line(
            "fail",
            f"[beads-verify-log] CRITICAL: compact_post task switch without recovery_action: {compact_switch_no_recovery}",
        )
        critical_count += compact_switch_no_recovery

    if critical_count > 0:
        status_line(
            "fail",
            f"[beads-verify-log] task={args.task} entries={entry_count} critical={critical_count} warnings={warn_count}",
        )
    elif warn_count > 0:
        status_line(
            "warn",
            f"[beads-verify-log] task={args.task} entries={entry_count} critical={critical_count} warnings={warn_count}",
        )
    else:
        status_line(
            "ok",
            f"[beads-verify-log] task={args.task} entries={entry_count} critical={critical_count} warnings={warn_count}",
        )

    if critical_count > 0:
        return 2

    if args.strict:
        block_end_entries = sum(1 for event in logs if event.get("type") == "block_end")
        if block_end_entries == 0:
            print("[beads-verify-log] CRITICAL: strict mode requires at least one block_end entry")
            return 2

        reflection_entries = sum(1 for event in logs if event.get("type") == "self_reflection")
        if reflection_entries == 0:
            print("[beads-verify-log] CRITICAL: strict mode requires at least one self_reflection entry")
            return 2

    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
