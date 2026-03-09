#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

import subprocess


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
VIDA_LEGACY_BIN = ROOT_DIR / "_vida" / "scripts-nim" / "vida"
VIDA_LEGACY_BIN_FALLBACK = ROOT_DIR / "_vida" / "scripts-nim" / "vida-legacy"
LOG_FILE = ROOT_DIR / ".vida" / "logs" / "beads-execution.jsonl"
OUT_DIR = ROOT_DIR / ".vida" / "logs"
TURSO_PYTHON = ROOT_DIR / ".venv" / "bin" / "python3"


def usage() -> int:
    print("Usage: python3 _vida/scripts/eval-pack.py run <task_id>", file=sys.stderr)
    return 1


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def legacy_bin() -> Path:
    if VIDA_LEGACY_BIN.exists():
        return VIDA_LEGACY_BIN
    return VIDA_LEGACY_BIN_FALLBACK


def task_status_json(task_id: str) -> dict:
    bin_path = legacy_bin()
    if not bin_path.exists():
        raise RuntimeError(f"Missing vida binary: {bin_path}")
    env = dict(os.environ)
    env.setdefault("VIDA_ROOT", str(ROOT_DIR))
    env.setdefault("VIDA_LEGACY_TURSO_PYTHON", str(TURSO_PYTHON))
    completed = subprocess.run(
        [str(bin_path), "task", "show", task_id, "--json"],
        cwd=str(ROOT_DIR),
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError((completed.stderr or completed.stdout or "vida task show failed").strip())
    payload = json.loads(completed.stdout)
    if not isinstance(payload, dict):
        raise RuntimeError("vida task show returned non-object payload")
    return payload


def iter_log_events(task_id: str) -> list[dict]:
    if not LOG_FILE.exists():
        return []
    events: list[dict] = []
    for raw in LOG_FILE.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line:
            continue
        try:
            payload = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(payload, dict):
            continue
        if payload.get("task_id") == task_id or payload.get("task_before") == task_id or payload.get("task_after") == task_id:
            events.append(payload)
    return events


def count_events(events: list[dict], predicate) -> int:
    return sum(1 for event in events if predicate(event))


def build_eval_pack(task_id: str) -> dict:
    task_payload = task_status_json(task_id)
    task_status = str(task_payload.get("status", "")).strip()
    events = iter_log_events(task_id)

    block_total = count_events(events, lambda e: e.get("type") == "block_end" and e.get("task_id") == task_id)
    block_done = count_events(
        events,
        lambda e: e.get("type") == "block_end" and e.get("task_id") == task_id and e.get("result") == "done",
    )

    if block_total == 0:
        block_success_rate = 0
        avg_block_duration_ms = 0
    else:
        block_success_rate = round((block_done * 100.0) / block_total, 2)
        durations = [int(event.get("duration_ms", 0) or 0) for event in events if event.get("type") == "block_end" and event.get("task_id") == task_id]
        avg_block_duration_ms = round(sum(durations) / len(durations)) if durations else 0

    reflections = count_events(events, lambda e: e.get("type") == "self_reflection" and e.get("task_id") == task_id)
    human_intervention_rate_proxy = 0 if block_total == 0 else round((reflections * 100.0) / block_total, 2)
    drift_alert_count = count_events(
        events,
        lambda e: e.get("task_id") == task_id and e.get("type") == "op_event" and e.get("name") == "context_drift_detected",
    )
    compact_pre_count = count_events(events, lambda e: e.get("task_id") == task_id and e.get("type") == "compact_pre")
    compact_post_count = count_events(
        events,
        lambda e: (e.get("task_before") == task_id or e.get("task_after") == task_id) and e.get("type") == "compact_post",
    )
    hydrated_count = count_events(
        events,
        lambda e: e.get("task_id") == task_id and e.get("type") == "op_event" and e.get("name") == "context_hydrated",
    )

    compact_recovery = "not_applicable"
    if compact_pre_count > 0 or compact_post_count > 0:
        compact_recovery = "pass" if compact_pre_count > 0 and compact_post_count > 0 and hydrated_count > 0 else "partial"

    return {
        "generated_at": now_utc(),
        "task_id": task_id,
        "task_completion": "closed" if task_status == "closed" else "open_or_in_progress",
        "task_status": task_status,
        "block_total": block_total,
        "block_done": block_done,
        "block_success_rate": block_success_rate,
        "avg_block_duration_ms": avg_block_duration_ms,
        "human_intervention_rate_proxy": human_intervention_rate_proxy,
        "drift_alert_count": drift_alert_count,
        "compact_recovery_scenario": compact_recovery,
    }


def run(task_id: str) -> Path:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    out_path = OUT_DIR / f"eval-pack-{task_id}.json"
    out_path.write_text(json.dumps(build_eval_pack(task_id), indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return out_path


def main(argv: list[str]) -> int:
    if len(argv) != 3 or argv[1] != "run":
        return usage()
    out_path = run(argv[2])
    print(out_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
