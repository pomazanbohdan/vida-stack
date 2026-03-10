#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
import time
from pathlib import Path

import turso

CONNECT_RETRIES = 20
CONNECT_BACKOFF_SEC = 0.05


def is_lock_error(exc: Exception) -> bool:
    return "lock" in str(exc).casefold()


def normalize_task_payload(payload: dict) -> dict:
    normalized = dict(payload)
    task_id = str(normalized.get("id", "")).strip()
    display_id = str(normalized.get("display_id", "")).strip()
    if not display_id:
        legacy_match = re.match(r"^vida-stack-([^.]+(?:\.[^.]+)*)$", task_id)
        if legacy_match:
            display_id = f"vida-{legacy_match.group(1)}"
        else:
            display_id = task_id
    normalized["display_id"] = display_id
    return normalized


def display_sort_key(value: str) -> tuple:
    text = (value or "").strip()
    if not text:
        return ("", ())
    prefix, dot, suffix = text.partition(".")
    prefix_match = re.match(r"^(.*?)-([^.]+)$", prefix)
    if prefix_match:
        family = prefix_match.group(1)
        root = prefix_match.group(2)
    else:
        family = prefix
        root = ""
    seq_parts: list[tuple[int, object]] = []
    if suffix:
        for part in suffix.split("."):
            if part.isdigit():
                seq_parts.append((0, int(part)))
            else:
                seq_parts.append((1, part))
    return (family, root, tuple(seq_parts), text)


def connect_db(path: str, *, init_schema: bool = False):
    db_path = Path(path)
    db_path.parent.mkdir(parents=True, exist_ok=True)
    last_exc: Exception | None = None
    con = None
    for attempt in range(CONNECT_RETRIES):
        try:
            con = turso.connect(str(db_path))
            break
        except Exception as exc:  # pragma: no cover - depends on runtime contention
            last_exc = exc
            if not is_lock_error(exc) or attempt + 1 >= CONNECT_RETRIES:
                raise
            time.sleep(CONNECT_BACKOFF_SEC * (attempt + 1))
    if con is None:
        assert last_exc is not None
        raise last_exc
    if init_schema:
        cur = con.cursor()
        cur.execute(
            """
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                payload TEXT NOT NULL,
                status TEXT NOT NULL,
                priority INTEGER NOT NULL,
                issue_type TEXT NOT NULL
            )
            """
        )
        cur.execute(
            """
            CREATE TABLE IF NOT EXISTS dependencies (
                issue_id TEXT NOT NULL,
                depends_on_id TEXT NOT NULL,
                dep_type TEXT NOT NULL,
                payload TEXT NOT NULL
            )
            """
        )
        con.commit()
    return con


def import_jsonl(db_path: str, source_path: str) -> dict:
    con = connect_db(db_path, init_schema=True)
    cur = con.cursor()
    imported = 0
    unchanged = 0
    updated = 0
    for idx, raw in enumerate(Path(source_path).read_text(encoding="utf-8").splitlines(), start=1):
        line = raw.strip()
        if not line:
            continue
        payload = normalize_task_payload(json.loads(line))
        task_id = str(payload.get("id", "")).strip()
        if not task_id:
            raise SystemExit(f"invalid task record at line {idx}: missing id")
        serialized = json.dumps(payload, ensure_ascii=True, separators=(",", ":"))
        existing = cur.execute("SELECT payload FROM tasks WHERE id = ?", [task_id]).fetchone()
        if existing is None:
            imported += 1
        elif existing[0] == serialized:
            unchanged += 1
        else:
            updated += 1
        cur.execute(
            """
            INSERT INTO tasks(id, payload, status, priority, issue_type)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
              payload=excluded.payload,
              status=excluded.status,
              priority=excluded.priority,
              issue_type=excluded.issue_type
            """,
            [
                task_id,
                serialized,
                str(payload.get("status", "")),
                int(payload.get("priority", 999) or 999),
                str(payload.get("issue_type", "")),
            ],
        )
        cur.execute("DELETE FROM dependencies WHERE issue_id = ?", [task_id])
        for dep in payload.get("dependencies", []) or []:
            cur.execute(
                """
                INSERT INTO dependencies(issue_id, depends_on_id, dep_type, payload)
                VALUES (?, ?, ?, ?)
                """,
                [
                    task_id,
                    str(dep.get("depends_on_id", "")),
                    str(dep.get("type", "")),
                    json.dumps(dep, ensure_ascii=True, separators=(",", ":")),
                ],
            )
    con.commit()
    con.close()
    return {
        "status": "ok",
        "imported_count": imported,
        "unchanged_count": unchanged,
        "updated_count": updated,
        "source_path": str(source_path),
        "db_path": db_path,
    }


def export_jsonl(db_path: str, target_path: str) -> dict:
    rows = list_tasks(db_path, status=None, include_all=True)
    target = Path(target_path)
    target.parent.mkdir(parents=True, exist_ok=True)
    tmp_path = target.with_suffix(target.suffix + ".tmp")
    with tmp_path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(normalize_task_payload(row), ensure_ascii=True, separators=(",", ":")))
            handle.write("\n")
    tmp_path.replace(target)
    return {
        "status": "ok",
        "exported_count": len(rows),
        "target_path": str(target),
        "db_path": db_path,
    }


def list_tasks(db_path: str, status: str | None, include_all: bool) -> list[dict]:
    con = connect_db(db_path)
    cur = con.cursor()
    sql = "SELECT payload FROM tasks"
    params: list[object] = []
    where = []
    if status:
        where.append("status = ?")
        params.append(status)
    if not include_all:
        where.append("status != 'closed'")
    if where:
        sql += " WHERE " + " AND ".join(where)
    sql += " ORDER BY priority ASC, id ASC"
    rows = [normalize_task_payload(json.loads(row[0])) for row in cur.execute(sql, params).fetchall()]
    rows.sort(key=lambda item: (
        int(item.get("priority", 999) or 999),
        display_sort_key(str(item.get("display_id", item.get("id", "")))),
        str(item.get("id", "")),
    ))
    con.close()
    return rows


def show_task(db_path: str, task_id: str) -> dict:
    con = connect_db(db_path)
    cur = con.cursor()
    row = cur.execute("SELECT payload FROM tasks WHERE id = ?", [task_id]).fetchone()
    con.close()
    if row is None:
        return {"status": "missing", "task_id": task_id}
    return normalize_task_payload(json.loads(row[0]))


def ready_tasks(db_path: str) -> list[dict]:
    con = connect_db(db_path)
    cur = con.cursor()
    tasks = {
        row[0]: normalize_task_payload(json.loads(row[1]))
        for row in cur.execute(
            """
            SELECT id, payload FROM tasks
            WHERE status IN ('open', 'in_progress') AND issue_type != 'epic'
            """
        ).fetchall()
    }
    status_by_id = {row[0]: row[1] for row in cur.execute("SELECT id, status FROM tasks").fetchall()}
    ready: list[dict] = []
    for task_id, payload in tasks.items():
        ok = True
        deps = cur.execute(
            "SELECT depends_on_id, dep_type FROM dependencies WHERE issue_id = ?",
            [task_id],
        ).fetchall()
        for depends_on_id, dep_type in deps:
            if dep_type == "parent-child":
                continue
            if status_by_id.get(depends_on_id) != "closed":
                ok = False
                break
        if ok:
            ready.append(payload)
    ready.sort(
        key=lambda item: (
            0 if item.get("status") == "in_progress" else 1,
            int(item.get("priority", 999) or 999),
            display_sort_key(str(item.get("display_id", item.get("id", "")))),
            str(item.get("id", "")),
        )
    )
    con.close()
    return ready


def create_task(
    db_path: str,
    task_id: str,
    title: str,
    *,
    issue_type: str,
    status: str,
    priority: int,
    display_id: str,
    parent_id: str,
    description: str,
    labels: list[str] | None,
) -> dict:
    normalized = normalize_task_payload(
        {
            "id": task_id,
            "display_id": display_id or task_id,
            "title": title,
            "status": status,
            "priority": priority,
            "issue_type": issue_type,
        }
    )
    if description:
        normalized["description"] = description
    if labels:
        normalized["labels"] = list(dict.fromkeys(label for label in labels if label))
    dependencies: list[dict] = []
    if parent_id:
        dependencies.append(
            {
                "issue_id": task_id,
                "depends_on_id": parent_id,
                "type": "parent-child",
            }
        )
    if dependencies:
        normalized["dependencies"] = dependencies

    con = connect_db(db_path, init_schema=True)
    cur = con.cursor()
    existing = cur.execute("SELECT payload FROM tasks WHERE id = ?", [task_id]).fetchone()
    if existing is not None:
        con.close()
        return {
            "status": "error",
            "reason": "task_already_exists",
            "task_id": task_id,
        }
    serialized = json.dumps(normalized, ensure_ascii=True, separators=(",", ":"))
    cur.execute(
        """
        INSERT INTO tasks(id, payload, status, priority, issue_type)
        VALUES (?, ?, ?, ?, ?)
        """,
        [task_id, serialized, status, priority, issue_type],
    )
    for dep in dependencies:
        cur.execute(
            """
            INSERT INTO dependencies(issue_id, depends_on_id, dep_type, payload)
            VALUES (?, ?, ?, ?)
            """,
            [
                task_id,
                str(dep.get("depends_on_id", "")),
                str(dep.get("type", "")),
                json.dumps(dep, ensure_ascii=True, separators=(",", ":")),
            ],
        )
    con.commit()
    con.close()
    return {
        "status": "ok",
        "created": True,
        "task": normalized,
    }


def set_labels(payload: dict, set_labels: list[str] | None, add_labels: list[str] | None, remove_labels: list[str] | None) -> None:
    labels = list(payload.get("labels", []) or [])
    if set_labels is not None:
        labels = []
        for label in set_labels:
            if label not in labels:
                labels.append(label)
    for label in add_labels or []:
        if label not in labels:
            labels.append(label)
    for label in remove_labels or []:
        labels = [existing for existing in labels if existing != label]
    if labels:
        payload["labels"] = labels
    else:
        payload.pop("labels", None)


def update_task(
    db_path: str,
    task_id: str,
    *,
    status: str | None,
    notes: str | None,
    description: str | None,
    set_labels_arg: list[str] | None,
    add_labels_arg: list[str] | None,
    remove_labels_arg: list[str] | None,
) -> dict:
    con = connect_db(db_path, init_schema=True)
    cur = con.cursor()
    row = cur.execute("SELECT payload FROM tasks WHERE id = ?", [task_id]).fetchone()
    if row is None:
        con.close()
        return {"status": "missing", "task_id": task_id}
    payload = normalize_task_payload(json.loads(row[0]))
    changed = False
    if status is not None:
        payload["status"] = status
        if status != "closed":
            payload.pop("closed_at", None)
            payload.pop("close_reason", None)
        changed = True
    if notes is not None:
        payload["notes"] = notes
        changed = True
    if description is not None:
        payload["description"] = description
        changed = True
    if set_labels_arg is not None or add_labels_arg or remove_labels_arg:
        set_labels(payload, set_labels_arg, add_labels_arg, remove_labels_arg)
        changed = True
    if changed:
        serialized = json.dumps(payload, ensure_ascii=True, separators=(",", ":"))
        cur.execute(
            """
            UPDATE tasks
            SET payload = ?, status = ?, priority = ?, issue_type = ?
            WHERE id = ?
            """,
            [
                serialized,
                str(payload.get("status", "")),
                int(payload.get("priority", 999) or 999),
                str(payload.get("issue_type", "")),
                task_id,
            ],
        )
        con.commit()
    con.close()
    return {"status": "ok", "updated": changed, "task": payload}


def close_task(db_path: str, task_id: str, *, reason: str) -> dict:
    con = connect_db(db_path, init_schema=True)
    cur = con.cursor()
    row = cur.execute("SELECT payload FROM tasks WHERE id = ?", [task_id]).fetchone()
    if row is None:
        con.close()
        return {"status": "missing", "task_id": task_id}
    payload = normalize_task_payload(json.loads(row[0]))
    payload["status"] = "closed"
    payload["close_reason"] = reason
    serialized = json.dumps(payload, ensure_ascii=True, separators=(",", ":"))
    cur.execute(
        """
        UPDATE tasks
        SET payload = ?, status = ?, priority = ?, issue_type = ?
        WHERE id = ?
        """,
        [
            serialized,
            "closed",
            int(payload.get("priority", 999) or 999),
            str(payload.get("issue_type", "")),
            task_id,
        ],
    )
    con.commit()
    con.close()
    return {"status": "ok", "closed": True, "task": payload}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--db", required=True)
    sub = parser.add_subparsers(dest="command", required=True)

    p_import = sub.add_parser("import-jsonl")
    p_import.add_argument("source")

    p_export = sub.add_parser("export-jsonl")
    p_export.add_argument("target")

    p_list = sub.add_parser("list")
    p_list.add_argument("--status", default="")
    p_list.add_argument("--all", action="store_true")

    p_show = sub.add_parser("show")
    p_show.add_argument("task_id")

    sub.add_parser("ready")

    p_create = sub.add_parser("create")
    p_create.add_argument("task_id")
    p_create.add_argument("title")
    p_create.add_argument("--type", default="task")
    p_create.add_argument("--status", default="open")
    p_create.add_argument("--priority", type=int, default=2)
    p_create.add_argument("--display-id", default="")
    p_create.add_argument("--parent-id", default="")
    p_create.add_argument("--description", default="")
    p_create.add_argument("--labels", action="append")

    p_update = sub.add_parser("update")
    p_update.add_argument("task_id")
    p_update.add_argument("--status")
    p_update.add_argument("--notes")
    p_update.add_argument("--description")
    p_update.add_argument("--add-label", action="append")
    p_update.add_argument("--remove-label", action="append")
    p_update.add_argument("--set-labels", action="append")

    p_close = sub.add_parser("close")
    p_close.add_argument("task_id")
    p_close.add_argument("--reason", required=True)

    args = parser.parse_args()
    if args.command == "import-jsonl":
        payload = import_jsonl(args.db, args.source)
    elif args.command == "export-jsonl":
        payload = export_jsonl(args.db, args.target)
    elif args.command == "list":
        payload = list_tasks(args.db, args.status or None, args.all)
    elif args.command == "show":
        payload = show_task(args.db, args.task_id)
    elif args.command == "create":
        payload = create_task(
            args.db,
            args.task_id,
            args.title,
            issue_type=args.type,
            status=args.status,
            priority=args.priority,
            display_id=args.display_id,
            parent_id=args.parent_id,
            description=args.description,
            labels=args.labels,
        )
    elif args.command == "update":
        payload = update_task(
            args.db,
            args.task_id,
            status=args.status,
            notes=args.notes,
            description=args.description,
            set_labels_arg=args.set_labels,
            add_labels_arg=args.add_label,
            remove_labels_arg=args.remove_label,
        )
    elif args.command == "close":
        payload = close_task(args.db, args.task_id, reason=args.reason)
    else:
        payload = ready_tasks(args.db)

    print(json.dumps(payload, ensure_ascii=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
