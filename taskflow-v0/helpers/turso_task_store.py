#!/usr/bin/env python3
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
import re
import time
import uuid
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
        cur.execute(
            """
            CREATE TABLE IF NOT EXISTS protocol_binding_state (
                scenario TEXT NOT NULL,
                protocol_id TEXT NOT NULL,
                source_path TEXT NOT NULL,
                activation_class TEXT NOT NULL,
                runtime_owner TEXT NOT NULL,
                enforcement_type TEXT NOT NULL,
                proof_surface TEXT NOT NULL,
                primary_state_authority TEXT NOT NULL,
                binding_status TEXT NOT NULL,
                active INTEGER NOT NULL,
                blockers TEXT NOT NULL,
                synced_at TEXT NOT NULL,
                PRIMARY KEY (scenario, protocol_id)
            )
            """
        )
        cur.execute(
            """
            CREATE TABLE IF NOT EXISTS protocol_binding_receipt (
                receipt_id TEXT PRIMARY KEY,
                scenario TEXT NOT NULL,
                total_bindings INTEGER NOT NULL,
                active_bindings INTEGER NOT NULL,
                script_bound_count INTEGER NOT NULL,
                rust_bound_count INTEGER NOT NULL,
                fully_runtime_bound_count INTEGER NOT NULL,
                unbound_count INTEGER NOT NULL,
                blocking_issue_count INTEGER NOT NULL,
                primary_state_authority TEXT NOT NULL,
                recorded_at TEXT NOT NULL
            )
            """
        )
        con.commit()
    return con


def now_utc() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


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


def normalize_protocol_binding_row(row: dict, *, scenario: str, primary_state_authority: str, synced_at: str) -> dict:
    protocol_id = str(row.get("protocol_id", "")).strip()
    if not protocol_id:
        raise SystemExit("invalid protocol binding row: missing protocol_id")
    blockers = row.get("blockers", []) or []
    if not isinstance(blockers, list):
        blockers = [str(blockers)]
    normalized_blockers = [str(item).strip() for item in blockers if str(item).strip()]
    binding_status = str(row.get("binding_status", "")).strip() or "unbound"
    if normalized_blockers:
        binding_status = "unbound"
    return {
        "protocol_id": protocol_id,
        "source_path": str(row.get("source_path", "")).strip(),
        "activation_class": str(row.get("activation_class", "")).strip(),
        "runtime_owner": str(row.get("runtime_owner", "")).strip(),
        "enforcement_type": str(row.get("enforcement_type", "")).strip(),
        "proof_surface": str(row.get("proof_surface", "")).strip(),
        "primary_state_authority": str(row.get("primary_state_authority", "")).strip() or primary_state_authority,
        "binding_status": binding_status,
        "active": bool(row.get("active", True)),
        "blockers": normalized_blockers,
        "scenario": str(row.get("scenario", "")).strip() or scenario,
        "synced_at": str(row.get("synced_at", "")).strip() or synced_at,
    }


def protocol_binding_sync(db_path: str, source_path: str) -> dict:
    payload = json.loads(Path(source_path).read_text(encoding="utf-8"))
    scenario = str(payload.get("scenario", "")).strip()
    primary_state_authority = str(payload.get("primary_state_authority", "")).strip()
    rows = payload.get("bindings", []) or []
    if not scenario:
        raise SystemExit("invalid protocol binding payload: missing scenario")
    if not primary_state_authority:
        raise SystemExit("invalid protocol binding payload: missing primary_state_authority")
    if not isinstance(rows, list) or not rows:
        raise SystemExit("invalid protocol binding payload: missing bindings")

    con = connect_db(db_path, init_schema=True)
    cur = con.cursor()
    cur.execute("DELETE FROM protocol_binding_state WHERE scenario = ?", [scenario])

    synced_at = now_utc()
    active_bindings = 0
    script_bound_count = 0
    rust_bound_count = 0
    fully_runtime_bound_count = 0
    unbound_count = 0
    blocking_issue_count = 0
    normalized_rows: list[dict] = []

    for row in rows:
        normalized = normalize_protocol_binding_row(
            row,
            scenario=scenario,
            primary_state_authority=primary_state_authority,
            synced_at=synced_at,
        )
        normalized_rows.append(normalized)
        if normalized["active"]:
            active_bindings += 1
        if normalized["binding_status"] == "script-bound":
            script_bound_count += 1
        elif normalized["binding_status"] == "rust-bound":
            rust_bound_count += 1
        elif normalized["binding_status"] == "fully-runtime-bound":
            fully_runtime_bound_count += 1
        else:
            unbound_count += 1
        blocking_issue_count += len(normalized["blockers"])
        cur.execute(
            """
            INSERT INTO protocol_binding_state(
                scenario, protocol_id, source_path, activation_class, runtime_owner,
                enforcement_type, proof_surface, primary_state_authority,
                binding_status, active, blockers, synced_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(scenario, protocol_id) DO UPDATE SET
              source_path=excluded.source_path,
              activation_class=excluded.activation_class,
              runtime_owner=excluded.runtime_owner,
              enforcement_type=excluded.enforcement_type,
              proof_surface=excluded.proof_surface,
              primary_state_authority=excluded.primary_state_authority,
              binding_status=excluded.binding_status,
              active=excluded.active,
              blockers=excluded.blockers,
              synced_at=excluded.synced_at
            """,
            [
                normalized["scenario"],
                normalized["protocol_id"],
                normalized["source_path"],
                normalized["activation_class"],
                normalized["runtime_owner"],
                normalized["enforcement_type"],
                normalized["proof_surface"],
                normalized["primary_state_authority"],
                normalized["binding_status"],
                1 if normalized["active"] else 0,
                json.dumps(normalized["blockers"], ensure_ascii=True, separators=(",", ":")),
                normalized["synced_at"],
            ],
        )

    receipt = {
        "receipt_id": f"protocol-binding-{uuid.uuid4().hex}",
        "scenario": scenario,
        "total_bindings": len(normalized_rows),
        "active_bindings": active_bindings,
        "script_bound_count": script_bound_count,
        "rust_bound_count": rust_bound_count,
        "fully_runtime_bound_count": fully_runtime_bound_count,
        "unbound_count": unbound_count,
        "blocking_issue_count": blocking_issue_count,
        "primary_state_authority": primary_state_authority,
        "recorded_at": synced_at,
    }
    cur.execute(
        """
        INSERT INTO protocol_binding_receipt(
            receipt_id, scenario, total_bindings, active_bindings, script_bound_count,
            rust_bound_count, fully_runtime_bound_count, unbound_count,
            blocking_issue_count, primary_state_authority, recorded_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """,
        [
            receipt["receipt_id"],
            receipt["scenario"],
            receipt["total_bindings"],
            receipt["active_bindings"],
            receipt["script_bound_count"],
            receipt["rust_bound_count"],
            receipt["fully_runtime_bound_count"],
            receipt["unbound_count"],
            receipt["blocking_issue_count"],
            receipt["primary_state_authority"],
            receipt["recorded_at"],
        ],
    )
    con.commit()
    con.close()
    return {
        "status": "ok",
        "ok": unbound_count == 0 and blocking_issue_count == 0,
        "scenario": scenario,
        "primary_state_authority": primary_state_authority,
        "receipt": receipt,
        "rows": normalized_rows,
        "source_path": str(source_path),
        "db_path": db_path,
    }


def load_protocol_binding_rows(cur, scenario: str) -> list[dict]:
    rows = []
    for row in cur.execute(
        """
        SELECT protocol_id, source_path, activation_class, runtime_owner, enforcement_type,
               proof_surface, primary_state_authority, binding_status, active,
               blockers, scenario, synced_at
        FROM protocol_binding_state
        WHERE scenario = ?
        ORDER BY protocol_id ASC
        """,
        [scenario],
    ).fetchall():
        blockers = []
        if row[9]:
            try:
                blockers = json.loads(row[9])
            except json.JSONDecodeError:
                blockers = [str(row[9])]
        rows.append(
            {
                "protocol_id": row[0],
                "source_path": row[1],
                "activation_class": row[2],
                "runtime_owner": row[3],
                "enforcement_type": row[4],
                "proof_surface": row[5],
                "primary_state_authority": row[6],
                "binding_status": row[7],
                "active": bool(row[8]),
                "blockers": blockers,
                "scenario": row[10],
                "synced_at": row[11],
            }
        )
    return rows


def latest_protocol_binding_receipt(cur, *, scenario: str | None = None) -> dict | None:
    sql = """
        SELECT receipt_id, scenario, total_bindings, active_bindings, script_bound_count,
               rust_bound_count, fully_runtime_bound_count, unbound_count,
               blocking_issue_count, primary_state_authority, recorded_at
        FROM protocol_binding_receipt
    """
    params: list[object] = []
    if scenario:
        sql += " WHERE scenario = ?"
        params.append(scenario)
    sql += " ORDER BY recorded_at DESC LIMIT 1"
    row = cur.execute(sql, params).fetchone()
    if row is None:
        return None
    return {
        "receipt_id": row[0],
        "scenario": row[1],
        "total_bindings": int(row[2]),
        "active_bindings": int(row[3]),
        "script_bound_count": int(row[4]),
        "rust_bound_count": int(row[5]),
        "fully_runtime_bound_count": int(row[6]),
        "unbound_count": int(row[7]),
        "blocking_issue_count": int(row[8]),
        "primary_state_authority": row[9],
        "recorded_at": row[10],
    }


def protocol_binding_status(db_path: str, *, scenario: str | None, include_rows: bool) -> dict:
    con = connect_db(db_path, init_schema=True)
    cur = con.cursor()
    receipt = latest_protocol_binding_receipt(cur, scenario=scenario)
    rows: list[dict] = []
    if receipt is not None and include_rows:
        rows = load_protocol_binding_rows(cur, receipt["scenario"])
    con.close()
    ok = bool(receipt) and int(receipt["unbound_count"]) == 0 and int(receipt["blocking_issue_count"]) == 0
    return {
        "status": "ok",
        "ok": ok,
        "has_receipt": receipt is not None,
        "receipt": receipt,
        "rows": rows,
        "db_path": db_path,
    }


def protocol_binding_check(
    db_path: str,
    *,
    scenario: str | None,
    expected_count: int,
    required_authority: str | None,
) -> dict:
    payload = protocol_binding_status(db_path, scenario=scenario, include_rows=True)
    blocking_reasons: list[str] = []
    receipt = payload.get("receipt")
    if receipt is None:
        blocking_reasons.append("missing_protocol_binding_receipt")
    else:
        if expected_count > 0 and int(receipt["total_bindings"]) != expected_count:
            blocking_reasons.append(
                f"unexpected_binding_count:{receipt['total_bindings']}!=expected:{expected_count}"
            )
        if int(receipt["unbound_count"]) > 0:
            blocking_reasons.append(f"unbound_bindings:{receipt['unbound_count']}")
        if int(receipt["blocking_issue_count"]) > 0:
            blocking_reasons.append(f"blocking_issues:{receipt['blocking_issue_count']}")
        if required_authority and str(receipt["primary_state_authority"]).strip() != required_authority:
            blocking_reasons.append(
                f"unexpected_primary_state_authority:{receipt['primary_state_authority']}"
            )
    for row in payload.get("rows", []):
        for blocker in row.get("blockers", []):
            blocking_reasons.append(f"{row['protocol_id']}:{blocker}")
    return {
        "status": "ok",
        "ok": len(blocking_reasons) == 0,
        "receipt": receipt,
        "rows": payload.get("rows", []),
        "blocking_reasons": blocking_reasons,
        "remediation_commands": [],
        "db_path": db_path,
    }


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

    p_protocol_sync = sub.add_parser("protocol-binding-sync")
    p_protocol_sync.add_argument("source")

    p_protocol_status = sub.add_parser("protocol-binding-status")
    p_protocol_status.add_argument("--scenario", default="")
    p_protocol_status.add_argument("--rows", action="store_true")

    p_protocol_check = sub.add_parser("protocol-binding-check")
    p_protocol_check.add_argument("--scenario", default="")
    p_protocol_check.add_argument("--expected-count", type=int, default=0)
    p_protocol_check.add_argument("--required-authority", default="")

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
    elif args.command == "protocol-binding-sync":
        payload = protocol_binding_sync(args.db, args.source)
    elif args.command == "protocol-binding-status":
        payload = protocol_binding_status(
            args.db,
            scenario=args.scenario or None,
            include_rows=args.rows,
        )
    elif args.command == "protocol-binding-check":
        payload = protocol_binding_check(
            args.db,
            scenario=args.scenario or None,
            expected_count=args.expected_count,
            required_authority=args.required_authority or None,
        )
    else:
        payload = ready_tasks(args.db)

    print(json.dumps(payload, ensure_ascii=True))
    if args.command == "protocol-binding-check":
        return 0 if payload.get("ok") else 1
    if args.command == "protocol-binding-sync":
        return 0 if payload.get("ok") else 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
