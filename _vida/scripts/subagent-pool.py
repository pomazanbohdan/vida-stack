#!/usr/bin/env python3
"""Reusable leased subagent pool helper built on top of existing lease primitives."""

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


subagent_system = load_module("vida_subagent_system_for_pool", SCRIPT_DIR / "subagent-system.py")


def active_pool_leases() -> dict[str, dict[str, Any]]:
    payload = subagent_system.active_leases()
    rows = payload.get("active", [])
    if not isinstance(rows, list):
        return {}
    out: dict[str, dict[str, Any]] = {}
    for item in rows:
        if not isinstance(item, dict):
            continue
        if str(item.get("resource_type", "")).strip() != "subagent_pool":
            continue
        resource_id = str(item.get("resource_id", "")).strip()
        if resource_id:
            out[resource_id] = item
    return out


def borrow_subagent(task_class: str, holder: str, ttl_seconds: int) -> dict[str, Any]:
    snapshot = subagent_system.runtime_snapshot()
    config = subagent_system.vida_config.load_validated_config()
    strategy = subagent_system.load_strategy_memory()
    leased = active_pool_leases()
    context = subagent_system.route_candidate_context(
        task_class,
        snapshot,
        config,
        strategy,
        excluded_subagents=set(leased.keys()),
    )
    candidates = context.get("candidates", [])
    if not candidates:
        return {
            "status": "blocked",
            "reason": "no_pool_candidate",
            "leased_subagents": sorted(leased.keys()),
        }
    selected = candidates[0]
    subagent = str(selected.get("subagent", "")).strip()
    lease = subagent_system.acquire_lease("subagent_pool", subagent, holder, ttl_seconds)
    return {
        "status": lease.get("status", "blocked"),
        "task_class": task_class,
        "holder": holder,
        "selected_subagent": subagent,
        "candidate": selected,
        "lease": lease.get("lease", lease),
        "leased_subagents": sorted(leased.keys()),
    }


def release_subagent(subagent: str, holder: str) -> dict[str, Any]:
    lease = subagent_system.release_lease("subagent_pool", subagent, holder)
    return {
        "status": lease.get("status", "blocked"),
        "subagent": subagent,
        "holder": holder,
        "lease": lease.get("lease", lease),
    }


def pool_status() -> dict[str, Any]:
    payload = subagent_system.active_leases()
    return {
        "generated_at": subagent_system.now_utc(),
        "active_pool_leases": [
            item
            for item in payload.get("active", [])
            if isinstance(item, dict) and str(item.get("resource_type", "")).strip() == "subagent_pool"
        ],
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)

    borrow = sub.add_parser("borrow")
    borrow.add_argument("task_class")
    borrow.add_argument("holder")
    borrow.add_argument("--ttl-seconds", type=int, default=1800)

    release = sub.add_parser("release")
    release.add_argument("subagent")
    release.add_argument("holder")

    sub.add_parser("status")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.command == "borrow":
        print(json.dumps(borrow_subagent(args.task_class, args.holder, max(60, args.ttl_seconds)), indent=2, sort_keys=True))
        return 0
    if args.command == "release":
        print(json.dumps(release_subagent(args.subagent, args.holder), indent=2, sort_keys=True))
        return 0
    if args.command == "status":
        print(json.dumps(pool_status(), indent=2, sort_keys=True))
        return 0
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
