#!/usr/bin/env python3
"""Synchronize framework wave task state from a declarative manifest."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
DEFAULT_MANIFEST_PATH = ROOT_DIR / ".vida" / "state" / "framework-wave-task-sync.json"
BR_MUTATION_QUEUE_SCRIPT = SCRIPT_DIR / "br-mutation-queue.py"


def load_json(path: Path, default: Any) -> Any:
    if not path.exists():
        return default
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return default


def run_br_read(*args: str) -> dict[str, Any]:
    for extra_args in (["--json"], ["--json", "--no-db"]):
        completed = subprocess.run(
            ["br", *args, *extra_args],
            cwd=str(ROOT_DIR),
            capture_output=True,
            text=True,
            check=False,
        )
        if completed.returncode != 0:
            continue
        payload = json.loads((completed.stdout or "").strip())
        if isinstance(payload, list):
            if not payload or not isinstance(payload[0], dict):
                raise RuntimeError("br read returned empty/non-object list payload")
            return payload[0]
        if not isinstance(payload, dict):
            raise RuntimeError("br read returned non-object payload")
        return payload
    raise RuntimeError((completed.stderr or completed.stdout or "br read failed").strip())


def run_br_mutation(args: list[str]) -> dict[str, Any]:
    completed = subprocess.run(
        [sys.executable, str(BR_MUTATION_QUEUE_SCRIPT), "br", "--", *args, "--json", "--no-db"],
        cwd=str(ROOT_DIR),
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError((completed.stderr or completed.stdout or "br mutation failed").strip())
    payload = json.loads((completed.stdout or "").strip())
    if isinstance(payload, list):
        return payload[0] if payload else {}
    return payload if isinstance(payload, dict) else {}


def sync_task(entry: dict[str, Any]) -> dict[str, Any]:
    task_id = str(entry.get("task_id", "")).strip()
    target_status = str(entry.get("status", "")).strip()
    reason = str(entry.get("reason", "")).strip()
    if not task_id or not target_status:
        return {"status": "skipped", "reason": "invalid_manifest_entry", "entry": entry}
    current = run_br_read("show", task_id)
    current_status = str(current.get("status", "")).strip()
    if current_status == target_status:
        return {"status": "noop", "task_id": task_id, "current_status": current_status}
    if target_status == "closed":
        result = run_br_mutation(["close", task_id, "--reason", reason or "framework task synchronized from manifest"])
        return {"status": "closed", "task_id": task_id, "result": result}
    result = run_br_mutation(["update", task_id, "--status", target_status])
    return {"status": "updated", "task_id": task_id, "result": result}


def sync_manifest(path: Path) -> dict[str, Any]:
    payload = load_json(path, {})
    tasks = payload.get("tasks", []) if isinstance(payload, dict) else []
    if not isinstance(tasks, list):
        raise RuntimeError("manifest.tasks must be a list")
    results = [sync_task(item) for item in tasks if isinstance(item, dict)]
    return {
        "manifest_path": str(path),
        "wave_id": payload.get("wave_id", "") if isinstance(payload, dict) else "",
        "results": results,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices={"apply"})
    parser.add_argument("--manifest", default=str(DEFAULT_MANIFEST_PATH))
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.command == "apply":
        result = sync_manifest(Path(args.manifest).expanduser())
        print(json.dumps(result, indent=2, sort_keys=True))
        return 0
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
