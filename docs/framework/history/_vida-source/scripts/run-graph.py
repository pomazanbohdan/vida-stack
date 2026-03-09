#!/usr/bin/env python3
"""Durable run-graph ledger helper for VIDA."""

from __future__ import annotations

import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
STATE_DIR = ROOT_DIR / ".vida" / "state" / "run-graphs"
STATE_DIR_ENV = "VIDA_RUN_GRAPH_STATE_DIR"
DEFAULT_NODES = ("analysis", "writer", "coach", "problem_party", "verifier", "approval", "synthesis")
ALLOWED_STATUS = {
    "pending",
    "ready",
    "running",
    "completed",
    "blocked",
    "failed",
    "skipped",
}
RESUME_PRIORITY = ("analysis", "writer", "coach", "problem_party", "verifier", "approval", "synthesis")


def now_utc() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def resolve_state_dir(state_dir: Path | None = None) -> Path:
    if state_dir is not None:
        return Path(state_dir)
    env_value = os.environ.get(STATE_DIR_ENV, "").strip()
    if env_value:
        return Path(env_value)
    return STATE_DIR


def graph_path(task_id: str, *, state_dir: Path | None = None) -> Path:
    return resolve_state_dir(state_dir) / f"{task_id}.json"


def load_graph(task_id: str, *, state_dir: Path | None = None) -> dict[str, Any]:
    path = graph_path(task_id, state_dir=state_dir)
    if not path.exists():
        return {}
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return payload if isinstance(payload, dict) else {}


def write_graph(task_id: str, payload: dict[str, Any], *, state_dir: Path | None = None) -> Path:
    path = graph_path(task_id, state_dir=state_dir)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def ensure_graph(task_id: str, task_class: str, route_task_class: str = "", *, state_dir: Path | None = None) -> dict[str, Any]:
    existing = load_graph(task_id, state_dir=state_dir)
    nodes = existing.get("nodes") if isinstance(existing.get("nodes"), dict) else {}
    normalized_nodes = {
        node: value
        for node, value in nodes.items()
        if isinstance(value, dict)
    }
    for node in DEFAULT_NODES:
        normalized_nodes.setdefault(node, {"status": "pending", "updated_at": now_utc(), "attempts": 0, "meta": {}})
    payload = {
        "task_id": task_id,
        "task_class": task_class,
        "route_task_class": route_task_class or existing.get("route_task_class", ""),
        "updated_at": now_utc(),
        "nodes": normalized_nodes,
    }
    return payload


def update_node(
    task_id: str,
    task_class: str,
    node: str,
    status: str,
    *,
    route_task_class: str = "",
    meta: dict[str, Any] | None = None,
    state_dir: Path | None = None,
) -> Path:
    if node not in DEFAULT_NODES:
        raise ValueError(f"invalid_node:{node}")
    if status not in ALLOWED_STATUS:
        raise ValueError(f"invalid_status:{status}")
    payload = ensure_graph(task_id, task_class, route_task_class=route_task_class, state_dir=state_dir)
    entry = payload["nodes"][node]
    previous_status = entry.get("status")
    attempts = int(entry.get("attempts", 0) or 0)
    if status == "running" and previous_status != "running":
        attempts += 1
    entry.update(
        {
            "status": status,
            "updated_at": now_utc(),
            "attempts": attempts,
            "meta": meta or {},
        }
    )
    payload["updated_at"] = now_utc()
    return write_graph(task_id, payload, state_dir=state_dir)


def status_payload(task_id: str, *, state_dir: Path | None = None) -> dict[str, Any]:
    payload = load_graph(task_id, state_dir=state_dir)
    if not payload:
        return {"task_id": task_id, "present": False}
    return {
        "task_id": task_id,
        "present": True,
        **payload,
        "resume_hint": resume_hint(payload),
    }


def resume_hint(payload: dict[str, Any]) -> dict[str, Any]:
    nodes = payload.get("nodes")
    if not isinstance(nodes, dict):
        return {"next_node": "", "reason": "missing_nodes"}
    for node in RESUME_PRIORITY:
        entry = nodes.get(node)
        if not isinstance(entry, dict):
            continue
        status = str(entry.get("status", "")).strip()
        if status in {"blocked", "failed", "running", "ready"}:
            return {
                "next_node": node,
                "status": status,
                "reason": str((entry.get("meta") or {}).get("reason", "")).strip(),
            }
    return {"next_node": "", "status": "completed", "reason": "no_resumable_node"}


def usage() -> int:
    print(
        "Usage:\n"
        "  python3 _vida/scripts/run-graph.py init <task_id> <task_class> [route_task_class]\n"
        "  python3 _vida/scripts/run-graph.py update <task_id> <task_class> <node> <status> [route_task_class] [meta_json]\n"
        "  python3 _vida/scripts/run-graph.py status <task_id>",
        file=sys.stderr,
    )
    return 2


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        return usage()
    command = argv[1]
    task_id = argv[2]
    if command == "init":
        if len(argv) < 4:
            return usage()
        task_class = argv[3]
        route_task_class = argv[4] if len(argv) > 4 else ""
        path = write_graph(
            task_id,
            ensure_graph(task_id, task_class, route_task_class=route_task_class),
        )
        print(str(path))
        return 0
    if command == "update":
        if len(argv) < 6:
            return usage()
        task_class = argv[3]
        node = argv[4]
        status = argv[5]
        route_task_class = argv[6] if len(argv) > 6 else ""
        meta: dict[str, Any] = {}
        if len(argv) > 7:
            raw_meta = json.loads(argv[7])
            if not isinstance(raw_meta, dict):
                print("[run-graph] meta_json must be an object", file=sys.stderr)
                return 2
            meta = raw_meta
        path = update_node(task_id, task_class, node, status, route_task_class=route_task_class, meta=meta)
        print(str(path))
        return 0
    if command == "status":
        print(json.dumps(status_payload(task_id), indent=2, sort_keys=True))
        return 0
    return usage()


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
