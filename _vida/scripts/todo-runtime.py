#!/usr/bin/env python3
"""Python engine for VIDA TODO runtime tools."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT_DIR = Path(__file__).resolve().parents[2]
LOG_FILE = ROOT_DIR / ".vida" / "logs" / "beads-execution.jsonl"
TODO_INDEX_DIR = ROOT_DIR / ".vida" / "logs" / "todo-index"
TODO_LOG_DIR = ROOT_DIR / ".vida" / "logs"
TODO_SYNC_STATE_DIR = ROOT_DIR / ".vida" / "logs" / "todo-sync-state"
STATEFUL_SEQUENCE_SCRIPT = ROOT_DIR / "_vida" / "scripts" / "stateful-sequence-check.sh"


@dataclass
class Step:
    block_id: str
    task_id: str
    goal: str = ""
    track_id: str = "main"
    owner: str = "orchestrator"
    depends_on: str = ""
    next_step: str = ""
    ts_start: str = ""
    ts_end: str = ""
    result: str = ""
    actions: str = ""
    evidence_ref: str = ""
    merge_ready: str = ""

    def status(self) -> str:
        if self.result == "done":
            return "done"
        if self.result == "failed":
            return "blocked"
        if self.result == "redirected":
            return "superseded"
        if self.result == "partial":
            return "todo"
        if self.ts_start and not self.ts_end:
            return "doing"
        return "todo"

    def to_json(self) -> dict[str, Any]:
        payload = {
            "block_id": self.block_id,
            "task_id": self.task_id,
            "goal": self.goal,
            "track_id": self.track_id or "main",
            "owner": self.owner or "orchestrator",
            "depends_on": self.depends_on,
            "next_step": self.next_step,
            "ts_start": self.ts_start,
            "ts_end": self.ts_end,
            "result": self.result,
            "actions": self.actions,
            "evidence_ref": self.evidence_ref,
            "merge_ready": self.merge_ready,
            "status": self.status(),
        }
        return {k: v for k, v in payload.items() if v != ""}


def ensure_log() -> None:
    if not LOG_FILE.exists():
        raise SystemExit(f"[todo-runtime] Log file not found: {LOG_FILE}")


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    if not path.exists():
        return events
    with path.open() as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            events.append(json.loads(line))
    return events


def task_scope_paths(task_id: str) -> list[str]:
    scope: list[str] = []
    seen: set[str] = set()
    for event in read_jsonl(LOG_FILE):
        if event.get("task_id") != task_id or event.get("type") != "block_end":
            continue
        raw = (event.get("artifacts") or "").strip()
        if not raw:
            continue
        for item in raw.split(","):
            path = item.strip()
            if not path or path in seen:
                continue
            seen.add(path)
            scope.append(path)
    return scope


def path_in_scope(path: str, scope_paths: list[str]) -> bool:
    for scope in scope_paths:
        normalized = scope.rstrip("/")
        if path == normalized or path.startswith(normalized + "/"):
            return True
    return False


def log_signature() -> str:
    if not LOG_FILE.exists():
        return "missing"
    st = LOG_FILE.stat()
    return f"{int(st.st_mtime)}:{st.st_size}"


def compute_steps(task_id: str) -> list[dict[str, Any]]:
    events = read_jsonl(LOG_FILE)
    relevant = [
        e
        for e in events
        if (e.get("task_id") or "") == task_id and e.get("type") in {"block_plan", "block_start", "block_end"}
    ]
    by_block: dict[str, Step] = {}
    for e in relevant:
        block_id = e.get("block_id")
        if not block_id:
            continue
        step = by_block.setdefault(block_id, Step(block_id=block_id, task_id=task_id))
        if e["type"] == "block_plan":
            step.goal = e.get("goal", step.goal)
            step.track_id = e.get("track_id") or step.track_id or "main"
            step.owner = e.get("owner") or step.owner or "orchestrator"
            step.depends_on = e.get("depends_on", step.depends_on)
            if e.get("next_step", "") != "":
                step.next_step = e["next_step"]
        elif e["type"] == "block_start":
            step.goal = e.get("goal", step.goal)
            step.track_id = e.get("track_id") or step.track_id or "main"
            step.owner = e.get("owner") or step.owner or "orchestrator"
            step.depends_on = e.get("depends_on", step.depends_on)
            if e.get("next_step", "") != "":
                step.next_step = e["next_step"]
            step.ts_start = e.get("ts", step.ts_start)
            step.ts_end = ""
            step.result = ""
        elif e["type"] == "block_end":
            step.result = e.get("result", "")
            step.next_step = e.get("next_step", step.next_step)
            step.actions = e.get("actions", "")
            step.evidence_ref = e.get("evidence_ref", "")
            step.merge_ready = e.get("merge_ready", "")
            step.ts_end = e.get("ts_end") or e.get("ts", "")
    return [by_block[key].to_json() for key in sorted(by_block)]


def steps_json(task_id: str) -> list[dict[str, Any]]:
    TODO_INDEX_DIR.mkdir(parents=True, exist_ok=True)
    index_file = TODO_INDEX_DIR / f"{task_id}.json"
    signature = log_signature()

    if index_file.exists():
        try:
            cached = json.loads(index_file.read_text())
            if cached.get("log_signature") == signature:
                return sorted(cached.get("steps", []), key=lambda s: s["block_id"])
        except Exception:
            pass

    steps = compute_steps(task_id)
    payload = {
        "task_id": task_id,
        "updated_at": subprocess.check_output(["date", "-u", "+%Y-%m-%dT%H:%M:%SZ"], text=True).strip(),
        "log_signature": signature,
        "steps": steps,
    }
    index_file.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    return steps


def cmd_ui_json(task_id: str) -> int:
    print(json.dumps({"task_id": task_id, "steps": steps_json(task_id)}, separators=(",", ":")))
    return 0


def cmd_list(task_id: str) -> int:
    for step in steps_json(task_id):
        next_step = step.get("next_step") or "-"
        print(f"{step['block_id']} [{step['status']}] goal={step.get('goal','-')} next={next_step} track={step.get('track_id','main')}")
    return 0


def cmd_current(task_id: str) -> int:
    doing = [s for s in steps_json(task_id) if s["status"] == "doing"]
    if not doing:
        print("none")
        return 0
    for step in doing:
        print(f"{step['block_id']}: {step.get('goal','-')}")
    return 0


def cmd_next(task_id: str) -> int:
    next_values = [s.get("next_step", "") for s in steps_json(task_id) if s.get("next_step")]
    print(next_values[-1] if next_values else "none")
    return 0


def group_ids(steps: list[dict[str, Any]], status: str) -> str:
    return ", ".join([s["block_id"] for s in steps if s["status"] == status])


def cmd_board(task_id: str) -> int:
    steps = steps_json(task_id)
    print(f"TODO:    {group_ids(steps, 'todo')}")
    print(f"DOING:   {group_ids(steps, 'doing')}")
    print(f"DONE:    {group_ids(steps, 'done')}")
    print(f"BLOCKED: {group_ids(steps, 'blocked')}")
    print(f"SUPERSEDED: {group_ids(steps, 'superseded')}")
    return 0


def shorten(text: str, limit: int = 72) -> str:
    return text if len(text) <= limit else text[:limit] + "..."


def compact_line(steps: list[dict[str, Any]], status: str, limit: int) -> str:
    items = [s for s in steps if s["status"] == status]
    if not items:
        return f"{status.upper()}(0): -"
    body = " | ".join(f"{s['block_id']}:{shorten(s.get('goal','-'))}" for s in items[:limit])
    if len(items) > limit:
        body += f" | +{len(items) - limit} more"
    return f"{status.upper()}({len(items)}): {body}"


def cmd_compact(task_id: str, limit: int) -> int:
    steps = steps_json(task_id)
    print(compact_line(steps, "todo", limit))
    print(compact_line(steps, "doing", limit))
    print(compact_line(steps, "done", limit))
    print(compact_line(steps, "blocked", limit))
    print(compact_line(steps, "superseded", limit))
    return 0


def cmd_tracks(task_id: str) -> int:
    steps = steps_json(task_id)
    grouped: dict[str, list[dict[str, Any]]] = {}
    for step in steps:
        grouped.setdefault(step.get("track_id", "main"), []).append(step)
    for track_id in sorted(grouped):
        parts = []
        for step in grouped[track_id]:
            nxt = f"->{step['next_step']}" if step.get("next_step") else ""
            parts.append(f"{step['block_id']}[{step['status']}]"+nxt)
        print(f"TRACK {track_id}: {', '.join(parts)}")
    return 0


def write_sync_json(task_id: str) -> tuple[dict[str, Any], Path, Path]:
    TODO_LOG_DIR.mkdir(parents=True, exist_ok=True)
    TODO_SYNC_STATE_DIR.mkdir(parents=True, exist_ok=True)
    payload = {"task_id": task_id, "steps": steps_json(task_id)}
    json_out = TODO_LOG_DIR / f"todo-sync-{task_id}.json"
    state_out = TODO_SYNC_STATE_DIR / f"{task_id}-last-ui.json"
    json_out.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    return payload, json_out, state_out


def sync_delta(prev_payload: dict[str, Any], payload: dict[str, Any]) -> list[str]:
    prev = {s["block_id"]: s for s in prev_payload.get("steps", [])}
    cur = {s["block_id"]: s for s in payload.get("steps", [])}
    lines: list[str] = []
    for block_id in sorted(set(prev) | set(cur)):
        pv = prev.get(block_id)
        cv = cur.get(block_id)
        if pv is None and cv is not None:
            lines.append(f"- [+] {block_id} — {cv.get('goal','-')} (status={cv.get('status','todo')})")
        elif pv is not None and cv is None:
            lines.append(f"- [-] {block_id} — {pv.get('goal','-')}")
        elif pv and cv and pv.get("status") != cv.get("status"):
            lines.append(
                f"- [~] {block_id} — {cv.get('goal', pv.get('goal','-'))} ({pv.get('status','')} -> {cv.get('status','')})"
            )
    return lines or ["- no status changes"]


def cmd_sync(task_id: str, mode: str, stdout_only: bool, quiet: bool, max_items: int) -> int:
    payload, json_out, state_out = write_sync_json(task_id)
    if mode == "json-only":
        if stdout_only:
            print(json.dumps(payload, separators=(",", ":")))
        elif not quiet:
            print(f"Snapshot JSON: {json_out}")
        return 0

    if mode == "delta":
        prev_payload = {"task_id": "", "steps": []}
        if state_out.exists():
            prev_payload = json.loads(state_out.read_text())
        if not quiet:
            print(f"# TODO Delta Snapshot: {task_id}\n")
            for line in sync_delta(prev_payload, payload):
                print(line)
        state_out.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
        return 0

    if mode == "compact":
        if not quiet:
            print(f"# TODO Compact Snapshot: {task_id}\n")
            steps = payload["steps"]
            print(compact_line(steps, "todo", max_items))
            print(compact_line(steps, "doing", max_items))
            print(compact_line(steps, "done", max_items))
            print(compact_line(steps, "blocked", max_items))
            print(compact_line(steps, "superseded", max_items))
            if not stdout_only:
                print(f"\nSnapshot JSON: {json_out}")
        state_out.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
        return 0

    if not quiet:
        print(f"# TODO Sync Snapshot: {task_id}\n")
        for step in sorted(payload["steps"], key=lambda s: s["block_id"]):
            mark = "x" if step["status"] == "done" else "~" if step["status"] == "superseded" else " "
            print(f"- [{mark}] {step['block_id']} — {step.get('goal','-')} (status={step.get('status','todo')})")
        if not stdout_only:
            print(f"\nSnapshot JSON: {json_out}")
    state_out.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    return 0


def run_checked(cmd: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=ROOT_DIR, text=True, capture_output=True, check=False)


def cmd_validate(task_id: str, strict: bool, quiet: bool, diff_aware: bool, base_ref: str) -> int:
    steps = steps_json(task_id)
    if not steps:
        print(f"[todo-plan-validate] FAIL: no planned steps for task {task_id}", file=sys.stderr)
        return 1

    fail = False
    if STATEFUL_SEQUENCE_SCRIPT.exists() and STATEFUL_SEQUENCE_SCRIPT.is_file():
        proc = run_checked(["bash", str(STATEFUL_SEQUENCE_SCRIPT), "assert", "block-plan", "--quiet"])
        if proc.returncode != 0 and not quiet:
            print("[todo-plan-validate] WARN: stateful operation currently in-flight; plan snapshot may be briefly stale", file=sys.stderr)
            fail = fail or strict

    ids = [s["block_id"] for s in steps]
    dup_ids = len(ids) - len(set(ids))
    missing_goal = sum(1 for s in steps if not s.get("goal", ""))
    bad_next_ref = sum(
        1
        for s in steps
        if s.get("next_step") not in {"", "-", None} and s.get("next_step") not in ids
    )
    bad_dep_ref = sum(1 for s in steps if s.get("depends_on") and s["depends_on"] not in ids)
    step_map = {s["block_id"]: s for s in steps}
    cross_track_next = 0
    for step in steps:
        next_step = step.get("next_step", "")
        if next_step and next_step != "-" and next_step in step_map:
            if step.get("track_id", "main") != step_map[next_step].get("track_id", "main"):
                cross_track_next += 1

    last_block = max(ids) if ids else ""
    warn_missing_next = sum(
        1 for s in steps if s["status"] != "done" and not s.get("next_step", "") and s["block_id"] != last_block
    )

    if not quiet:
        print(f"ℹ️ [todo-plan-validate] task={task_id} steps={len(steps)}")
    if dup_ids:
        print(f"⚠️ [todo-plan-validate] FAIL: duplicate block_id entries: {dup_ids}", file=sys.stderr)
        fail = True
    if missing_goal:
        print(f"⚠️ [todo-plan-validate] FAIL: blocks with empty goal: {missing_goal}", file=sys.stderr)
        fail = True
    if bad_next_ref:
        print(f"⚠️ [todo-plan-validate] WARN: blocks with invalid next_step target: {bad_next_ref}", file=sys.stderr)
        fail = fail or strict
    if bad_dep_ref:
        print(f"⚠️ [todo-plan-validate] FAIL: blocks with invalid depends_on target: {bad_dep_ref}", file=sys.stderr)
        fail = True
    if cross_track_next:
        print(f"⚠️ [todo-plan-validate] WARN: cross-track next_step links: {cross_track_next}", file=sys.stderr)
        fail = fail or strict
    if warn_missing_next:
        print(f"⚠️ [todo-plan-validate] WARN: non-done blocks missing next_step: {warn_missing_next}", file=sys.stderr)
        fail = fail or strict

    if diff_aware:
        changed_files: set[str] = set()
        for cmd in (
            ["git", "diff", "--name-only", f"{base_ref}...HEAD"],
            ["git", "diff", "--name-only", "--cached"],
            ["git", "ls-files", "--others", "--exclude-standard"],
        ):
            proc = run_checked(cmd)
            for line in proc.stdout.splitlines():
                if line.strip():
                    changed_files.add(line.strip())
        scoped_changes = changed_files
        scope_paths = task_scope_paths(task_id)
        if scope_paths:
            scoped_changes = {path for path in changed_files if path_in_scope(path, scope_paths)}
        if changed_files and not quiet:
            print(f"ℹ️ [todo-plan-validate] diff-aware changed_files={len(changed_files)} base={base_ref}")
        all_goals = " || ".join((s.get("goal") or "").lower() for s in steps)
        if scope_paths and changed_files and not quiet:
            print(f"ℹ️ [todo-plan-validate] diff-aware task_scope_paths={len(scope_paths)} scoped_changes={len(scoped_changes)}")
        if any(p == "AGENTS.md" or p.startswith("_vida/") for p in scoped_changes):
            if not any(
                token in all_goals
                for token in (
                    "framework",
                    "protocol",
                    "agent",
                    "boot",
                    "todo",
                    "subagent",
                    "docs",
                    "script",
                    "beads",
                    "log",
                    "verify",
                    "verifier",
                    "runtime",
                )
            ):
                print("[todo-plan-validate] WARN: framework files changed but task plan does not mention framework/protocol/docs scope", file=sys.stderr)
                fail = fail or strict
        if any(p.startswith("docs/") or p.startswith("scripts/") for p in scoped_changes):
            if not any(token in all_goals for token in ("docs", "script", "ops", "runbook", "observability", "audit", "build", "framework")):
                print("[todo-plan-validate] WARN: docs/scripts changed but task plan does not mention docs/script/ops scope", file=sys.stderr)
                fail = fail or strict
        if changed_files and not quiet:
            effective_scope = scoped_changes if scope_paths else changed_files
            print(
                "ℹ️ [todo-plan-validate] diff-aware scope=framework"
                if any(p == "AGENTS.md" or p.startswith("_vida/") for p in effective_scope)
                else "ℹ️ [todo-plan-validate] diff-aware scope=unknown"
            )

    if fail:
        return 1
    if not quiet:
        print("✅ [todo-plan-validate] OK")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="VIDA TODO runtime engine")
    sub = parser.add_subparsers(dest="cmd", required=True)

    for name in ["ui-json", "list", "current", "next", "board", "tracks"]:
        sp = sub.add_parser(name)
        sp.add_argument("task_id")

    compact = sub.add_parser("compact")
    compact.add_argument("task_id")
    compact.add_argument("limit", nargs="?", type=int, default=3)

    sync = sub.add_parser("sync")
    sync.add_argument("task_id")
    sync.add_argument("--stdout-only", action="store_true")
    sync.add_argument("--mode", choices=["full", "json-only", "delta", "compact"], default="full")
    sync.add_argument("--quiet", action="store_true")
    sync.add_argument("--max-items", type=int, default=3)

    validate = sub.add_parser("validate")
    validate.add_argument("task_id")
    validate.add_argument("--strict", action="store_true")
    validate.add_argument("--quiet", action="store_true")
    validate.add_argument("--diff-aware", action="store_true")
    validate.add_argument("--base", default="HEAD")
    return parser


def main(argv: list[str]) -> int:
    ensure_log()
    parser = build_parser()
    args = parser.parse_args(argv[1:])
    if args.cmd == "ui-json":
        return cmd_ui_json(args.task_id)
    if args.cmd == "list":
        return cmd_list(args.task_id)
    if args.cmd == "current":
        return cmd_current(args.task_id)
    if args.cmd == "next":
        return cmd_next(args.task_id)
    if args.cmd == "board":
        return cmd_board(args.task_id)
    if args.cmd == "compact":
        return cmd_compact(args.task_id, args.limit)
    if args.cmd == "tracks":
        return cmd_tracks(args.task_id)
    if args.cmd == "sync":
        return cmd_sync(args.task_id, args.mode, args.stdout_only, args.quiet, args.max_items)
    if args.cmd == "validate":
        return cmd_validate(args.task_id, args.strict, args.quiet, args.diff_aware, args.base)
    parser.error("unknown command")
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
