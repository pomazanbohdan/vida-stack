#!/usr/bin/env python3
"""Render a compact task-state snapshot for dev-oriented boot paths."""

from __future__ import annotations

import argparse
import importlib.util
import json
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
VIDA_CONFIG = SCRIPT_DIR / "vida-config.py"
RUN_GRAPH = SCRIPT_DIR / "run-graph.py"
TASK_STATE_RECONCILE = SCRIPT_DIR / "task-state-reconcile.py"
VIDA_LEGACY_BIN = ROOT_DIR / "_vida" / "scripts-nim" / "vida-legacy"
TURSO_PYTHON = str(ROOT_DIR / ".venv" / "bin" / "python3")


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def run_vida_legacy_json(*args: str) -> list[dict[str, Any]]:
    if not VIDA_LEGACY_BIN.exists():
        raise SystemExit(f"[vida-boot-snapshot] vida-legacy binary is missing: {VIDA_LEGACY_BIN}")
    env = os.environ.copy()
    env.setdefault("VIDA_ROOT", str(ROOT_DIR))
    env.setdefault("VIDA_LEGACY_TURSO_PYTHON", TURSO_PYTHON)
    proc = subprocess.run(
        [str(VIDA_LEGACY_BIN), "task", *args, "--json"],
        cwd=ROOT_DIR,
        capture_output=True,
        text=True,
        env=env,
        check=False,
    )
    if proc.returncode != 0:
        detail = (proc.stderr or proc.stdout).strip()
        raise SystemExit(
            f"[vida-boot-snapshot] vida-legacy task call failed: {' '.join(args)}\n{detail}"
        )
    payload = proc.stdout.strip() or "[]"
    try:
        data = json.loads(payload)
    except json.JSONDecodeError as exc:
        raise SystemExit(
            f"[vida-boot-snapshot] Failed to decode JSON from vida-legacy task call: {' '.join(args)}"
        ) from exc
    if not isinstance(data, list):
        raise SystemExit(
            f"[vida-boot-snapshot] Unexpected vida-legacy payload type for: {' '.join(args)}"
        )
    return data


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def framework_self_diagnosis_config() -> dict[str, Any]:
    try:
        vida_config = load_module("vida_boot_snapshot_config", VIDA_CONFIG)
        cfg = vida_config.load_validated_config()
    except Exception:
        return {}
    payload = cfg.get("framework_self_diagnosis")
    return payload if isinstance(payload, dict) else {}


def run_graph_status(task_id: str) -> dict[str, Any]:
    try:
        run_graph = load_module("vida_boot_snapshot_run_graph", RUN_GRAPH)
        payload = run_graph.status_payload(task_id)
    except Exception:
        return {}
    return payload if isinstance(payload, dict) else {}


def reconciliation_status(task_id: str) -> dict[str, Any]:
    try:
        reconcile = load_module("vida_boot_snapshot_task_reconcile", TASK_STATE_RECONCILE)
        payload = reconcile.build_status_payload(task_id)
    except Exception:
        return {}
    return payload if isinstance(payload, dict) else {}


def show_issue(issue_id: str) -> dict[str, Any]:
    rows = run_vida_legacy_json("show", issue_id)
    if not rows:
        raise SystemExit(f"[vida-boot-snapshot] Missing issue in vida-legacy task show: {issue_id}")
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


def issue_priority(issue: dict[str, Any]) -> int:
    try:
        return int(issue.get("priority", 999) or 999)
    except (TypeError, ValueError):
        return 999


def parse_issue_timestamp(value: Any) -> float:
    text = str(value or "").strip()
    if not text:
        return 0.0
    try:
        return datetime.fromisoformat(text.replace("Z", "+00:00")).timestamp()
    except ValueError:
        return 0.0


def issue_sort_key(issue: dict[str, Any]) -> tuple[int, float, float, str]:
    return (
        issue_priority(issue),
        -parse_issue_timestamp(issue.get("updated_at")),
        -parse_issue_timestamp(issue.get("created_at")),
        str(issue.get("id") or ""),
    )


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
    issue_id = str(issue.get("id") or "")
    run_graph = run_graph_status(issue_id) if issue_id else {}
    reconciliation = reconciliation_status(issue_id) if issue_id else {}
    return {
        "id": issue.get("id"),
        "title": clean_text(issue.get("title")),
        "status": issue.get("status"),
        "mode": issue_mode(issue),
        "priority": issue_priority(issue),
        "updated_at": issue.get("updated_at"),
        "run_graph": {
            "present": bool(run_graph.get("present", False)),
            "resume_hint": run_graph.get("resume_hint", {}),
        },
        "reconciliation": {
            "classification": str(reconciliation.get("classification", "")).strip(),
            "allowed_actions": reconciliation.get("allowed_actions", []),
        },
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


def child_entries_for(issue_id: str, all_rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    children = []
    for candidate in all_rows:
        if candidate.get("status") not in {"open", "in_progress"}:
            continue
        matched = False
        for dependency in candidate.get("dependencies") or []:
            if dependency.get("type") != "parent-child":
                continue
            if str(dependency.get("depends_on_id", "")).strip() != issue_id:
                continue
            matched = True
            break
        if not matched:
            continue
        children.append(
            {
                "id": candidate.get("id"),
                "title": candidate.get("title"),
                "status": candidate.get("status"),
                "priority": candidate.get("priority"),
                "updated_at": candidate.get("updated_at"),
                "created_at": candidate.get("created_at"),
            }
        )
    children.sort(key=lambda item: (0 if item.get("status") == "in_progress" else 1, *issue_sort_key(item)))
    return children


def build_snapshot(top_limit: int, ready_limit: int, subtasks_limit: int) -> dict[str, Any]:
    all_rows = run_vida_legacy_json("list", "--all")
    open_rows = [row for row in all_rows if row.get("status") == "open"]
    doing_rows = [row for row in all_rows if row.get("status") == "in_progress"]
    blocked_rows = [row for row in all_rows if row.get("status") == "blocked"]
    ready_rows = run_vida_legacy_json("ready")

    top_open = sorted(top_level(open_rows), key=issue_sort_key)
    top_doing = sorted(top_level(doing_rows), key=issue_sort_key)
    top_blocked = sorted(top_level(blocked_rows), key=issue_sort_key)
    top_ready = unique_by_id(sorted(top_level(ready_rows), key=issue_sort_key))
    top_ready_open = [row for row in top_ready if row.get("status") == "open"]
    top_ready_in_progress = [row for row in top_ready if row.get("status") == "in_progress"]

    in_progress = [
        issue_entry(row, child_entries_for(str(row.get("id")), all_rows), subtasks_limit)
        for row in top_doing[:top_limit]
    ]
    ready_head = [
        issue_entry(row, child_entries_for(str(row.get("id")), all_rows), subtasks_limit)
        for row in top_ready_open[:ready_limit]
    ]
    decision_required = [
        {
            "id": row.get("id"),
            "title": clean_text(row.get("title")),
            "status": row.get("status"),
            "priority": issue_priority(row),
            "updated_at": row.get("updated_at"),
        }
        for row in unique_by_id(top_doing + top_open + top_blocked)
        if issue_mode(row) == "decision_required"
    ]

    framework_diag = framework_self_diagnosis_config()
    active_run_graphs = sum(1 for item in in_progress if item.get("run_graph", {}).get("present"))
    return {
        "generated_at": now_utc(),
        "execution_continue_default": {
            "mode": "route_then_external_analysis",
            "summary": (
                "For write-producing continuation work in hybrid mode, stop after compact task snapshot, "
                "build the route receipt, and obtain the external analysis receipt before writer dispatch."
            ),
            "selection_policy": "prefer explicit priority first, then most recently updated work within the same priority band",
            "compact_assumption": "compact_can_happen_any_time",
            "prepare_execution_command": (
                "python3 _vida/scripts/subagent-dispatch.py prepare-execution <task_id> <writer_task_class> "
                "<prompt_file> <output_dir> [workdir]"
            ),
        },
        "summary": {
            "top_level_in_progress": len(top_doing),
            "top_level_open": len(top_open),
            "top_level_blocked": len(top_blocked),
            "ready_total": len(top_ready),
            "ready_open": len(top_ready_open),
            "ready_in_progress": len(top_ready_in_progress),
            "active_run_graphs": active_run_graphs,
        },
        "framework_self_diagnosis": {
            "enabled": bool(framework_diag.get("enabled", False)),
            "silent_mode": bool(framework_diag.get("silent_mode", False)),
            "auto_capture_bugs": bool(framework_diag.get("auto_capture_bugs", False)),
            "parent_issue": str(framework_diag.get("parent_issue", "")).strip(),
            "defer_fix_until_task_boundary": bool(framework_diag.get("defer_fix_until_task_boundary", False)),
            "session_reflection_required": bool(framework_diag.get("session_reflection_required", False)),
            "platform_direction": str(framework_diag.get("platform_direction", "")).strip(),
            "quality_token_efficiency": str(framework_diag.get("quality_token_efficiency", "")).strip(),
            "session_reflection_criteria": [
                str(item).strip()
                for item in (framework_diag.get("session_reflection_criteria") or [])
                if str(item).strip()
            ],
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
        run_graph = item.get("run_graph") or {}
        resume_hint = run_graph.get("resume_hint") if isinstance(run_graph, dict) else {}
        if run_graph.get("present") and isinstance(resume_hint, dict):
            next_node = str(resume_hint.get("next_node", "")).strip() or "-"
            status = str(resume_hint.get("status", "")).strip() or "-"
            reason = str(resume_hint.get("reason", "")).strip()
            detail = f"  - run_graph: next={next_node} status={status}"
            if reason:
                detail += f" reason={reason}"
            lines.append(detail)
        for child in item.get("subtasks", []):
            lines.append(f"  - [{child['status']}] {child['id']}  {child['title']}")
        hidden = int(item.get("hidden_subtasks") or 0)
        if hidden > 0:
            lines.append(f"  - +{hidden} more")


def render_text(snapshot: dict[str, Any]) -> str:
    summary = snapshot["summary"]
    framework_diag = snapshot.get("framework_self_diagnosis", {})
    lines = [
        "VIDA BOOT SNAPSHOT",
        f"execution_continue_default: {snapshot['execution_continue_default']['summary']}",
        f"task_selection_policy: {snapshot['execution_continue_default']['selection_policy']}",
        f"compact_assumption: {snapshot['execution_continue_default']['compact_assumption']}",
        (
            "summary: "
            f"in_progress={summary['top_level_in_progress']} "
            f"open={summary['top_level_open']} "
            f"blocked={summary['top_level_blocked']} "
            f"ready_total={summary['ready_total']} "
            f"ready_open={summary['ready_open']} "
            f"active_run_graphs={summary['active_run_graphs']}"
        ),
    ]
    if framework_diag.get("enabled"):
        lines.append(
            "framework_self_diagnosis: "
            f"silent_mode={framework_diag.get('silent_mode')} "
            f"auto_capture_bugs={framework_diag.get('auto_capture_bugs')} "
            f"defer_fix_until_task_boundary={framework_diag.get('defer_fix_until_task_boundary')} "
            f"platform_direction={framework_diag.get('platform_direction') or '-'} "
            f"quality_token_efficiency={framework_diag.get('quality_token_efficiency') or '-'}"
        )
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
