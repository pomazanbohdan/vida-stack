#!/usr/bin/env python3
"""Aggregated framework operator status surface for approvals and memory."""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
FRAMEWORK_MEMORY_PATH = ROOT_DIR / ".vida" / "state" / "framework-memory.json"
CONTEXT_GOVERNANCE_PATH = ROOT_DIR / ".vida" / "state" / "context-governance.json"
SILENT_DIAGNOSIS_PATH = ROOT_DIR / ".vida" / "state" / "silent-framework-diagnosis.json"
ISSUES_JSONL_PATH = ROOT_DIR / ".beads" / "issues.jsonl"
ROUTE_RECEIPT_DIR = ROOT_DIR / ".vida" / "logs" / "route-receipts"
RUN_GRAPH_DIR = ROOT_DIR / ".vida" / "state" / "run-graphs"
TASK_STATE_RECONCILE = SCRIPT_DIR / "task-state-reconcile.py"


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def load_json(path: Path, default: Any) -> Any:
    if not path.exists():
        return default
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return default


def load_module(name: str, path: Path):
    import importlib.util

    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def issue_status_map() -> dict[str, str]:
    if not ISSUES_JSONL_PATH.exists():
        return {}
    statuses: dict[str, str] = {}
    try:
        for raw_line in ISSUES_JSONL_PATH.read_text(encoding="utf-8").splitlines():
            line = raw_line.strip()
            if not line:
                continue
            payload = json.loads(line)
            if not isinstance(payload, dict):
                continue
            issue_id = str(payload.get("id", "")).strip()
            status = str(payload.get("status", "")).strip()
            if issue_id:
                statuses[issue_id] = status
    except (OSError, json.JSONDecodeError):
        return {}
    return statuses


def framework_issue_rows() -> list[dict[str, Any]]:
    if not ISSUES_JSONL_PATH.exists():
        return []
    rows: list[dict[str, Any]] = []
    try:
        for raw_line in ISSUES_JSONL_PATH.read_text(encoding="utf-8").splitlines():
            line = raw_line.strip()
            if not line:
                continue
            payload = json.loads(line)
            if not isinstance(payload, dict):
                continue
            status = str(payload.get("status", "")).strip()
            if status not in {"open", "in_progress"}:
                continue
            issue_id = str(payload.get("id", "")).strip()
            labels = payload.get("labels", []) or []
            if "framework" in labels or issue_id.startswith("mobile-1j1") or issue_id.startswith("mobile-2bi") or issue_id.startswith("mobile-3rf"):
                rows.append(payload)
    except (OSError, json.JSONDecodeError):
        return []
    return rows


def reconcile_pending_framework_bugs(payload: Any) -> list[dict[str, Any]]:
    pending = payload.get("pending_framework_bugs", []) if isinstance(payload, dict) else []
    if not isinstance(pending, list):
        return []
    statuses = issue_status_map()
    filtered: list[dict[str, Any]] = []
    for item in pending:
        if not isinstance(item, dict):
            continue
        bug_id = str(item.get("bug_id", "")).strip()
        if bug_id and statuses.get(bug_id) == "closed":
            continue
        filtered.append(item)
    return filtered


def task_reconciliation_summary() -> dict[str, Any]:
    try:
        reconcile = load_module("framework_operator_task_reconcile", TASK_STATE_RECONCILE)
    except Exception:
        return {"counts": {}, "sample_ids": {}}
    counts: dict[str, int] = {}
    sample_ids: dict[str, list[str]] = {}
    for item in framework_issue_rows():
        task_id = str(item.get("id", "")).strip()
        if not task_id:
            continue
        try:
            payload = reconcile.build_status_payload(task_id)
        except Exception:
            continue
        classification = str(payload.get("classification", "")).strip()
        if not classification:
            continue
        counts[classification] = int(counts.get(classification, 0) or 0) + 1
        bucket = sample_ids.setdefault(classification, [])
        if len(bucket) < 5:
            bucket.append(task_id)
    return {"counts": counts, "sample_ids": sample_ids}


def approval_summary() -> dict[str, Any]:
    approved = 0
    rejected = 0
    review_states: dict[str, int] = {}
    for path in sorted(ROUTE_RECEIPT_DIR.glob("*.approval.json")):
        payload = load_json(path, {})
        if not isinstance(payload, dict):
            continue
        decision = str(payload.get("decision", "")).strip().casefold()
        review_state = str(payload.get("review_state", "")).strip()
        if decision == "approved":
            approved += 1
        elif decision == "rejected":
            rejected += 1
        if review_state:
            review_states[review_state] = int(review_states.get(review_state, 0) or 0) + 1
    return {
        "approved_count": approved,
        "rejected_count": rejected,
        "review_states": review_states,
    }


def framework_memory_summary() -> dict[str, Any]:
    payload = load_json(FRAMEWORK_MEMORY_PATH, {})
    summary = payload.get("summary", {}) if isinstance(payload, dict) else {}
    if not isinstance(summary, dict):
        summary = {}
    return {
        "lesson_count": int(summary.get("lesson_count", 0) or 0),
        "correction_count": int(summary.get("correction_count", 0) or 0),
        "anomaly_count": int(summary.get("anomaly_count", 0) or 0),
    }


def context_governance_summary() -> dict[str, Any]:
    payload = load_json(CONTEXT_GOVERNANCE_PATH, {})
    summary = payload.get("summary", {}) if isinstance(payload, dict) else {}
    if not isinstance(summary, dict):
        summary = {}
    return {
        "by_source_class": dict(summary.get("by_source_class", {}) or {}),
        "task_count": int(summary.get("task_count", 0) or 0),
        "web_validated_count": int(summary.get("web_validated_count", 0) or 0),
        "last_recorded_at": str(summary.get("last_recorded_at", "")).strip(),
    }


def silent_diagnosis_summary() -> dict[str, Any]:
    payload = load_json(SILENT_DIAGNOSIS_PATH, {})
    pending = reconcile_pending_framework_bugs(payload)
    reflections = payload.get("session_reflections", []) if isinstance(payload, dict) else []
    pending_items = [item for item in pending if isinstance(item, dict)]
    summary_by_task: dict[str, int] = {}
    for item in pending_items:
        task_id = str(item.get("current_task", "")).strip()
        if task_id:
            summary_by_task[task_id] = int(summary_by_task.get(task_id, 0) or 0) + 1
    return {
        "pending_bug_count": len(pending_items),
        "pending_bug_ids": [str(item.get("bug_id", "")).strip() for item in pending_items if str(item.get("bug_id", "")).strip()],
        "pending_by_task": summary_by_task,
        "latest_pending_summary": str(pending_items[-1].get("summary", "")).strip() if pending_items else "",
        "recent_pending": [
            {
                "bug_id": str(item.get("bug_id", "")).strip(),
                "summary": str(item.get("summary", "")).strip(),
                "current_task": str(item.get("current_task", "")).strip(),
                "created_at": str(item.get("created_at", "")).strip(),
                "workaround": str(item.get("workaround", "")).strip(),
            }
            for item in pending_items[-5:]
        ],
        "session_reflection_count": len([item for item in reflections if isinstance(item, dict)]),
        "recent_reflections": [
            {
                "current_task": str(item.get("current_task", "")).strip(),
                "criteria": [str(value).strip() for value in (item.get("criteria") or []) if str(value).strip()],
                "gaps": [str(value).strip() for value in (item.get("gaps") or []) if str(value).strip()],
                "ts": str(item.get("ts", "")).strip(),
            }
            for item in reflections[-3:]
            if isinstance(item, dict)
        ],
    }


def route_rationale_summary() -> dict[str, Any]:
    route_files = sorted(ROUTE_RECEIPT_DIR.glob("*.route.json"))
    if not route_files:
        return {
            "route_receipt_count": 0,
            "dispatch_required": {},
            "writer_selected": {},
            "analysis_selected": {},
            "verification_selected": {},
            "coach_selected": {},
            "estimated_route_cost_class": {},
            "budget_over_cap_count": 0,
            "internal_writer_count": 0,
            "web_search_required_count": 0,
        }

    def bump(bucket: dict[str, int], key: str) -> None:
        if key:
            bucket[key] = int(bucket.get(key, 0) or 0) + 1

    dispatch_required: dict[str, int] = {}
    writer_selected: dict[str, int] = {}
    analysis_selected: dict[str, int] = {}
    verification_selected: dict[str, int] = {}
    coach_selected: dict[str, int] = {}
    estimated_route_cost_class: dict[str, int] = {}
    budget_over_cap_count = 0
    internal_writer_count = 0
    web_search_required_count = 0

    for path in route_files:
        payload = load_json(path, {})
        route = payload.get("route_receipt", {}) if isinstance(payload, dict) else {}
        if not isinstance(route, dict):
            continue
        route_graph = route.get("route_graph", {})
        nodes = route_graph.get("nodes", []) if isinstance(route_graph, dict) else []
        writer_name = ""
        if isinstance(nodes, list):
            for node in nodes:
                if not isinstance(node, dict):
                    continue
                if str(node.get("id", "")).strip() == "writer":
                    writer_name = str(node.get("selected_subagent", "")).strip()
                    break
        analysis_plan = route.get("analysis_plan", {})
        verification_plan = route.get("verification_plan", {})
        coach_plan = route.get("coach_plan", {})
        route_budget = route.get("route_budget", {})

        bump(dispatch_required, str(route.get("dispatch_required", "")).strip())
        bump(writer_selected, writer_name)
        bump(analysis_selected, str(analysis_plan.get("selected_subagent", "")).strip() if isinstance(analysis_plan, dict) else "")
        bump(verification_selected, str(verification_plan.get("selected_subagent", "")).strip() if isinstance(verification_plan, dict) else "")
        if isinstance(coach_plan, dict):
            coach_names = coach_plan.get("selected_subagents", [])
            if isinstance(coach_names, list) and coach_names:
                for name in coach_names:
                    bump(coach_selected, str(name).strip())
            else:
                bump(coach_selected, str(coach_plan.get("selected_subagent", "")).strip())
        if isinstance(route_budget, dict):
            bump(estimated_route_cost_class, str(route_budget.get("estimated_route_cost_class", "")).strip())
            max_units = int(route_budget.get("max_budget_units", 0) or 0)
            est_units = int(route_budget.get("estimated_route_cost_units", 0) or 0)
            if max_units > 0 and est_units > max_units:
                budget_over_cap_count += 1
        if writer_name == "internal_subagents":
            internal_writer_count += 1
        if str(route.get("web_search_required", "")).strip() == "yes":
            web_search_required_count += 1

    return {
        "route_receipt_count": len(route_files),
        "dispatch_required": dispatch_required,
        "writer_selected": writer_selected,
        "analysis_selected": analysis_selected,
        "verification_selected": verification_selected,
        "coach_selected": coach_selected,
        "estimated_route_cost_class": estimated_route_cost_class,
        "budget_over_cap_count": budget_over_cap_count,
        "internal_writer_count": internal_writer_count,
        "web_search_required_count": web_search_required_count,
    }


def anomaly_cluster_summary() -> dict[str, Any]:
    payload = load_json(FRAMEWORK_MEMORY_PATH, {})
    entries = payload.get("entries", []) if isinstance(payload, dict) else []
    by_summary: dict[str, int] = {}
    by_source: dict[str, int] = {}
    for item in entries:
        if not isinstance(item, dict):
            continue
        if str(item.get("kind", "")).strip() != "anomaly":
            continue
        summary = str(item.get("summary", "")).strip()
        source_task = str(item.get("source_task", "")).strip()
        if summary:
            by_summary[summary] = int(by_summary.get(summary, 0) or 0) + 1
        if source_task:
            by_source[source_task] = int(by_source.get(source_task, 0) or 0) + 1
    top_summaries = [
        {"summary": key, "count": count}
        for key, count in sorted(by_summary.items(), key=lambda item: (-item[1], item[0]))[:5]
    ]
    return {
        "top_summaries": top_summaries,
        "anomaly_tasks": by_source,
        "unique_anomaly_count": len(by_summary),
    }


def run_graph_summary() -> dict[str, Any]:
    active = 0
    real_active = 0
    blocked = 0
    next_nodes: dict[str, int] = {}
    suspicious_artifacts: list[str] = []
    for path in sorted(RUN_GRAPH_DIR.glob("*.json")):
        payload = load_json(path, {})
        if not isinstance(payload, dict):
            continue
        task_id = str(payload.get("task_id", path.stem)).strip()
        if task_id and not task_id.startswith("mobile-"):
            suspicious_artifacts.append(path.name)
        resume_hint = payload.get("resume_hint")
        if not isinstance(resume_hint, dict):
            from_run_graph = load_json(path, {})
            resume_hint = {}
            nodes = from_run_graph.get("nodes") if isinstance(from_run_graph, dict) else {}
            if isinstance(nodes, dict):
                for node in ("analysis", "writer", "coach", "problem_party", "verifier", "approval", "synthesis"):
                    entry = nodes.get(node)
                    if not isinstance(entry, dict):
                        continue
                    status = str(entry.get("status", "")).strip()
                    if status in {"blocked", "failed", "running", "ready"}:
                        resume_hint = {"next_node": node, "status": status}
                        break
        next_node = str((resume_hint or {}).get("next_node", "")).strip()
        status = str((resume_hint or {}).get("status", "")).strip()
        if next_node:
            active += 1
            if path.name not in suspicious_artifacts:
                real_active += 1
            next_nodes[next_node] = int(next_nodes.get(next_node, 0) or 0) + 1
        if status in {"blocked", "failed"}:
            blocked += 1
    return {
        "active_run_graphs": active,
        "real_active_run_graphs": real_active,
        "blocked_or_failed": blocked,
        "next_nodes": next_nodes,
        "suspicious_artifacts": suspicious_artifacts,
    }


def build_status_payload() -> dict[str, Any]:
    return {
        "generated_at": now_utc(),
        "framework_memory": framework_memory_summary(),
        "anomaly_clusters": anomaly_cluster_summary(),
        "context_governance": context_governance_summary(),
        "silent_diagnosis": silent_diagnosis_summary(),
        "task_reconciliation": task_reconciliation_summary(),
        "approval_summary": approval_summary(),
        "route_rationale": route_rationale_summary(),
        "run_graphs": run_graph_summary(),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices={"status"})
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.command == "status":
        print(json.dumps(build_status_payload(), indent=2, sort_keys=True))
        return 0
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
