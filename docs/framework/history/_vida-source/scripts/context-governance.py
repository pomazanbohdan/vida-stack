#!/usr/bin/env python3
"""Context governance ledger for VIDA source classes, freshness, and provenance."""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
STATE_PATH = ROOT_DIR / ".vida" / "state" / "context-governance.json"
VALID_SOURCE_CLASSES = {
    "local_repo",
    "local_runtime",
    "overlay_declared",
    "web_validated",
    "external_connector",
}


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def load_state() -> dict[str, Any]:
    default = {"entries": [], "summary": {"by_source_class": {}, "task_count": 0, "web_validated_count": 0}}
    if not STATE_PATH.exists():
        return default
    try:
        payload = json.loads(STATE_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return default
    if not isinstance(payload, dict):
        return default
    payload.setdefault("entries", [])
    payload.setdefault("summary", {"by_source_class": {}, "task_count": 0, "web_validated_count": 0})
    return payload


def save_state(payload: dict[str, Any]) -> None:
    STATE_PATH.parent.mkdir(parents=True, exist_ok=True)
    STATE_PATH.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def valid_source_class(value: str) -> str:
    normalized = value.strip().casefold()
    if normalized not in VALID_SOURCE_CLASSES:
        raise ValueError(f"invalid source class: {value}")
    return normalized


def normalize_sources(items: list[dict[str, Any]]) -> list[dict[str, Any]]:
    normalized: list[dict[str, Any]] = []
    seen: set[tuple[str, str]] = set()
    for item in items:
        if not isinstance(item, dict):
            continue
        source_class = valid_source_class(str(item.get("source_class", "")))
        path = str(item.get("path", "")).strip()
        if not path:
            continue
        key = (source_class, path)
        if key in seen:
            continue
        seen.add(key)
        normalized.append(
            {
                "source_class": source_class,
                "path": path,
                "freshness": str(item.get("freshness", "")).strip() or ("validated" if source_class == "web_validated" else "current"),
                "provenance": str(item.get("provenance", "")).strip() or source_class,
                "role_scope": str(item.get("role_scope", "")).strip() or "orchestrator",
                "notes": str(item.get("notes", "")).strip(),
            }
        )
    return normalized


def summarize_sources(items: list[dict[str, Any]]) -> dict[str, Any]:
    by_source_class: dict[str, int] = {}
    role_scopes: dict[str, int] = {}
    freshness: dict[str, int] = {}
    for item in items:
        source_class = str(item.get("source_class", "")).strip()
        role_scope = str(item.get("role_scope", "")).strip()
        freshness_value = str(item.get("freshness", "")).strip()
        if source_class:
            by_source_class[source_class] = int(by_source_class.get(source_class, 0) or 0) + 1
        if role_scope:
            role_scopes[role_scope] = int(role_scopes.get(role_scope, 0) or 0) + 1
        if freshness_value:
            freshness[freshness_value] = int(freshness.get(freshness_value, 0) or 0) + 1
    return {
        "by_source_class": by_source_class,
        "role_scopes": role_scopes,
        "freshness": freshness,
        "web_validated_count": int(by_source_class.get("web_validated", 0) or 0),
        "source_count": len(items),
    }


def record_entry(*, task_id: str, phase: str, sources: list[dict[str, Any]], notes: str = "") -> dict[str, Any]:
    payload = load_state()
    normalized_sources = normalize_sources(sources)
    entry = {
        "ts": now_utc(),
        "task_id": task_id.strip(),
        "phase": phase.strip(),
        "sources": normalized_sources,
        "summary": summarize_sources(normalized_sources),
        "notes": notes.strip(),
    }
    entries = payload.setdefault("entries", [])
    entries.append(entry)
    aggregate_counts: dict[str, int] = {}
    web_validated_count = 0
    task_ids: set[str] = set()
    for row in entries:
        if not isinstance(row, dict):
            continue
        task_id_value = str(row.get("task_id", "")).strip()
        if task_id_value:
            task_ids.add(task_id_value)
        for source in row.get("sources", []):
            if not isinstance(source, dict):
                continue
            source_class = str(source.get("source_class", "")).strip()
            if source_class:
                aggregate_counts[source_class] = int(aggregate_counts.get(source_class, 0) or 0) + 1
                if source_class == "web_validated":
                    web_validated_count += 1
    payload["summary"] = {
        "by_source_class": aggregate_counts,
        "task_count": len(task_ids),
        "web_validated_count": web_validated_count,
        "last_recorded_at": entry["ts"],
    }
    save_state(payload)
    return entry


def validate_sources(sources: list[dict[str, Any]]) -> dict[str, Any]:
    try:
        normalized = normalize_sources(sources)
    except ValueError as exc:
        return {"valid": False, "reason": str(exc), "sources": []}
    if not normalized:
        return {"valid": False, "reason": "missing_context_sources", "sources": []}
    for item in normalized:
        if item["source_class"] == "web_validated" and item["freshness"] not in {"validated", "current"}:
            return {"valid": False, "reason": "invalid_web_validated_freshness", "sources": normalized}
    return {"valid": True, "reason": "", "sources": normalized, "summary": summarize_sources(normalized)}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)

    record = sub.add_parser("record")
    record.add_argument("--task-id", required=True)
    record.add_argument("--phase", required=True)
    record.add_argument("--sources-json", required=True)
    record.add_argument("--notes", default="")

    validate = sub.add_parser("validate")
    validate.add_argument("--sources-json", required=True)

    sub.add_parser("status")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.command == "status":
        print(json.dumps(load_state(), indent=2, sort_keys=True))
        return 0
    if args.command == "validate":
        payload = json.loads(args.sources_json)
        result = validate_sources(payload if isinstance(payload, list) else [])
        print(json.dumps(result, indent=2, sort_keys=True))
        return 0 if result.get("valid") else 2
    if args.command == "record":
        payload = json.loads(args.sources_json)
        result = record_entry(
            task_id=args.task_id,
            phase=args.phase,
            sources=payload if isinstance(payload, list) else [],
            notes=args.notes,
        )
        print(json.dumps(result, indent=2, sort_keys=True))
        return 0
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
