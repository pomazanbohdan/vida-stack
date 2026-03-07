#!/usr/bin/env python3
import argparse
import datetime as dt
import fcntl
import json
import os
import subprocess
import sys
import uuid
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
BEADS_DIR = ROOT / ".beads"
QUEUE_LOG = BEADS_DIR / "mutation-queue.jsonl"
QUEUE_LOCK = BEADS_DIR / "mutation-queue.lock"
BR_SAFE = ROOT / "_vida" / "scripts" / "br-safe.sh"
JSONL_MUTATOR = ROOT / "_vida" / "scripts" / "br-jsonl-mutate.py"


def now_utc() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")


def append_jsonl(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(payload, ensure_ascii=True, separators=(",", ":")) + "\n")


def backend_command(backend: str, backend_args: list[str]) -> list[str]:
    if backend == "mutator":
        return [sys.executable, str(JSONL_MUTATOR), *backend_args]
    if backend == "br":
        return ["bash", str(BR_SAFE), *backend_args]
    raise ValueError(f"unsupported backend: {backend}")


def run_request(backend: str, backend_args: list[str]) -> int:
    BEADS_DIR.mkdir(parents=True, exist_ok=True)
    QUEUE_LOCK.touch(exist_ok=True)
    request_id = f"mq-{uuid.uuid4().hex[:12]}"
    queued_at = now_utc()
    command = backend_command(backend, backend_args)

    with QUEUE_LOCK.open("r+", encoding="utf-8") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        append_jsonl(
            QUEUE_LOG,
            {
                "type": "mutation_queue",
                "event": "queued",
                "request_id": request_id,
                "backend": backend,
                "args": backend_args,
                "queued_at": queued_at,
                "pid": os.getpid(),
            },
        )
        completed = subprocess.run(
            command,
            cwd=str(ROOT),
            capture_output=True,
            text=True,
            check=False,
        )
        append_jsonl(
            QUEUE_LOG,
            {
                "type": "mutation_queue",
                "event": "completed",
                "request_id": request_id,
                "backend": backend,
                "args": backend_args,
                "queued_at": queued_at,
                "completed_at": now_utc(),
                "return_code": completed.returncode,
            },
        )

    if completed.stdout:
        sys.stdout.write(completed.stdout)
    if completed.stderr:
        sys.stderr.write(completed.stderr)
    return int(completed.returncode)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("backend", choices=["mutator", "br"])
    parser.add_argument("backend_args", nargs=argparse.REMAINDER)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    backend_args = list(args.backend_args)
    if backend_args and backend_args[0] == "--":
        backend_args = backend_args[1:]
    if not backend_args:
        print("missing backend arguments", file=sys.stderr)
        return 2
    return run_request(args.backend, backend_args)


if __name__ == "__main__":
    raise SystemExit(main())
