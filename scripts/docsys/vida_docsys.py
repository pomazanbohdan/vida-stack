#!/usr/bin/env python3
"""Unified markdown document system toolkit for VIDA."""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from datetime import datetime
from pathlib import Path


FOOTER_MARKER = "\n-----\n"
REQUIRED_FOOTER_FIELDS = (
    "artifact_path",
    "artifact_type",
    "artifact_version",
    "artifact_revision",
    "schema_version",
    "status",
    "source_path",
    "created_at",
    "updated_at",
    "changelog_ref",
)
ORDERED_FOOTER_FIELDS = REQUIRED_FOOTER_FIELDS
FOOTER_OPTIONAL_FILES = {"AGENTS.md"}
MARKDOWN_LINK_RE = re.compile(r"\[[^\]]+\]\(([^)]+)\)")


def now_iso() -> str:
    return datetime.now().astimezone().replace(microsecond=0).isoformat()


def parse_ts(value: object) -> tuple[int, str]:
    text = str(value or "").strip()
    if not text:
        return (0, "")
    normalized = text.replace("Z", "+00:00")
    try:
        return (1, datetime.fromisoformat(normalized).isoformat())
    except ValueError:
        return (0, text)


def split_body_and_footer(text: str) -> tuple[str, dict[str, str]]:
    idx = text.rfind(FOOTER_MARKER)
    if idx < 0:
        return text, {}
    body = text[:idx]
    footer_text = text[idx + len(FOOTER_MARKER):].strip()
    footer: dict[str, str] = {}
    for line in footer_text.splitlines():
        raw = line.strip()
        if not raw or ":" not in raw:
            continue
        key, value = raw.split(":", 1)
        footer[key.strip()] = value.strip()
    return body, footer


def render_footer(footer: dict[str, str]) -> str:
    lines = ["-----"]
    seen = set()
    for key in ORDERED_FOOTER_FIELDS:
        if key in footer:
            lines.append(f"{key}: {footer[key]}")
            seen.add(key)
    for key, value in footer.items():
        if key not in seen:
            lines.append(f"{key}: {value}")
    return "\n".join(lines) + "\n"


def normalize_tag_list(raw: str) -> list[str]:
    return [tag for tag in (item.strip() for item in raw.split(",")) if tag]


def load_markdown(markdown_file: Path) -> tuple[str, str, dict[str, str]]:
    text = markdown_file.read_text(encoding="utf-8")
    body, footer = split_body_and_footer(text)
    return text, body, footer


def write_markdown(markdown_file: Path, body: str, footer: dict[str, str]) -> None:
    markdown_file.write_text(body.rstrip() + "\n\n" + render_footer(footer), encoding="utf-8")


def relative_to_root(path: Path, root: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return path.as_posix()


def append_changelog_event(changelog_path: Path, event: dict[str, object]) -> None:
    with changelog_path.open("a", encoding="utf-8") as fh:
        fh.write(json.dumps(event, ensure_ascii=True) + "\n")


def is_footer_optional(markdown_file: Path) -> bool:
    repo_root = Path(__file__).resolve().parents[2]
    try:
        rel = markdown_file.resolve().relative_to(repo_root)
    except ValueError:
        return markdown_file.name in FOOTER_OPTIONAL_FILES
    return len(rel.parts) == 1 and markdown_file.suffix.lower() == ".md"


def bootstrap_optional_event(markdown_file: Path, args: argparse.Namespace, event_name: str, reason: str) -> dict[str, object]:
    return {
        "ts": now_iso(),
        "event": event_name,
        "artifact_path": f"project/repository/{markdown_file.stem.lower()}",
        "artifact_type": "bootstrap_router",
        "artifact_version": "",
        "artifact_revision": "",
        "source_path": markdown_file.relative_to(Path(__file__).resolve().parents[2]).as_posix(),
        "reason": reason,
        "task_id": getattr(args, "task_id", ""),
        "actor": getattr(args, "actor", "manual"),
        "scope": getattr(args, "scope", ""),
        "tags": normalize_tag_list(getattr(args, "tags", "")),
    }


def set_result_note(args: argparse.Namespace, note: str) -> None:
    setattr(args, "_result_note", note)


def extract_description(body: str) -> str:
    for raw in body.splitlines():
        line = raw.strip()
        if not line:
            continue
        if line == "-----":
            break
        if line.startswith("#"):
            line = line.lstrip("#").strip()
            if line:
                return line
            continue
        if line.startswith(">"):
            line = line.lstrip(">").strip()
        if line.startswith(("- ", "* ", "1. ")):
            line = line[2:].strip() if not line.startswith("1. ") else line[3:].strip()
        if line.startswith("[!") or line.startswith("```"):
            continue
        return line
    return ""


def extract_purpose(body: str) -> str:
    for raw in body.splitlines():
        line = raw.strip()
        if not line:
            continue
        if line.lower().startswith("purpose:"):
            return line.split(":", 1)[1].strip()
    return ""


def titleize_stem(stem: str) -> str:
    return " ".join(part.capitalize() for part in stem.replace(".", "-").split("-") if part)


def latest_changelog_row(changelog_path: Path) -> dict[str, object] | None:
    rows = read_changelog_rows(changelog_path)
    if not rows:
        return None
    rows.sort(key=lambda row: parse_ts(row.get("ts", "")))
    return rows[-1]


def iter_markdown_files(scan_root: Path):
    for path in sorted(scan_root.rglob("*")):
        if path.is_file() and path.suffix.lower() == ".md":
            yield path


def classify_layer_and_owner(repo_root: Path, scan_root: Path, rel: Path, artifact_path: str) -> tuple[str, str]:
    normalized_artifact = artifact_path.replace("\\", "/")
    rel_posix = rel.as_posix()
    try:
        root_name = scan_root.relative_to(repo_root).parts[0]
    except Exception:
        root_name = scan_root.name
    if root_name == "docs":
        if rel_posix.startswith("framework/plans/"):
            return "framework_plan", "framework"
        if rel_posix.startswith("framework/research/"):
            return "framework_research", "framework"
        if rel_posix.startswith("framework/history/"):
            return "framework_history", "framework"
        if rel_posix.startswith("product/spec/"):
            return "product_spec", "product"
        if rel_posix.startswith("product/research/"):
            return "product_research", "product"
        if rel_posix == "product/index.md":
            return "product_index", "product"
        if rel_posix.startswith("process/"):
            return "project_process", "project"
        if rel_posix.startswith("project-memory/"):
            return "project_memory", "project"
    if root_name == "vida":
        if rel_posix.startswith("config/instructions/"):
            return "instruction_canon", "product"
        if rel_posix.startswith("config/"):
            return "executable_law", "product"
    if rel_posix in {"README.md", "CONTRIBUTING.md", "VERSION-PLAN.md"}:
        return "repository_doc", "project"
    if normalized_artifact.startswith("framework/plans/"):
        return "framework_plan", "framework"
    if normalized_artifact.startswith("framework/research/"):
        return "framework_research", "framework"
    if normalized_artifact.startswith("framework/history/"):
        return "framework_history", "framework"
    if normalized_artifact.startswith("framework/"):
        return "framework_doc", "framework"
    if normalized_artifact.startswith("product/spec/"):
        return "product_spec", "product"
    if normalized_artifact.startswith("product/research/"):
        return "product_research", "product"
    if normalized_artifact == "product/index":
        return "product_index", "product"
    if normalized_artifact.startswith("process/"):
        return "project_process", "project"
    if normalized_artifact.startswith("project-memory/"):
        return "project_memory", "project"
    if normalized_artifact.startswith("project/repository/"):
        return "repository_doc", "project"
    if normalized_artifact.startswith("config/instructions/"):
        return "instruction_canon", "product"
    if normalized_artifact.startswith("config/"):
        return "executable_law", "product"
    return "unknown", "unknown"


def build_record(repo_root: Path, scan_root: Path, file_path: Path) -> dict[str, object]:
    _, body, footer = load_markdown(file_path)
    rel = file_path.relative_to(scan_root)
    description = extract_description(body)
    purpose = extract_purpose(body)
    changelog_name = footer.get("changelog_ref", "")
    changelog_path = file_path.with_name(changelog_name) if changelog_name else None
    artifact_path = footer.get("artifact_path", "")
    artifact_type = footer.get("artifact_type", "")
    status = footer.get("status", "missing")
    layer, owner = classify_layer_and_owner(repo_root, scan_root, rel, artifact_path)
    record: dict[str, object] = {
        "path": rel.as_posix(),
        "description": description,
        "purpose": purpose,
        "layer": layer,
        "owner": owner,
        "state": {
            "status": status,
            "has_footer": bool(footer),
            "has_changelog": bool(changelog_path and changelog_path.exists()),
            "has_description": bool(description),
            "has_purpose": bool(purpose),
        },
    }
    if artifact_path:
        record["artifact"] = artifact_path
    if artifact_type:
        record["kind"] = artifact_type
    if changelog_name:
        record["changelog"] = changelog_name
        if changelog_path and changelog_path.exists():
            latest = latest_changelog_row(changelog_path)
            if latest:
                record["latest_change"] = {
                    "ts": latest.get("ts", ""),
                    "event": latest.get("event", ""),
                    "task_id": latest.get("task_id", ""),
                    "actor": latest.get("actor", ""),
                }
    return record


def validate_record(file_path: Path, record: dict[str, object]) -> list[str]:
    problems: list[str] = []
    state = record["state"]
    if not state["has_footer"]:
        if not is_footer_optional(file_path):
            problems.append("missing_footer")
        return problems
    text = file_path.read_text(encoding="utf-8")
    _, footer = split_body_and_footer(text)
    for field in REQUIRED_FOOTER_FIELDS:
        if not footer.get(field):
            problems.append(f"missing_footer_field:{field}")
    if record.get("artifact") and not record.get("kind"):
        problems.append("missing_kind")
    if not state["has_changelog"]:
        problems.append("missing_changelog")
    return problems


def validate_footer_consistency(markdown_file: Path, footer: dict[str, str]) -> list[str]:
    problems: list[str] = []
    repo_root = Path(__file__).resolve().parents[2]
    expected_source = relative_to_root(markdown_file, repo_root)
    if footer.get("source_path") and footer["source_path"] != expected_source:
        problems.append("source_path_mismatch")
    return problems


def changelog_path_for(markdown_file: Path) -> Path:
    text = markdown_file.read_text(encoding="utf-8")
    _, footer = split_body_and_footer(text)
    changelog_ref = footer.get("changelog_ref", "")
    if not changelog_ref and is_footer_optional(markdown_file):
        changelog_ref = markdown_file.with_suffix("").name + ".changelog.jsonl"
    if not changelog_ref:
        raise ValueError("footer metadata is missing changelog_ref")
    return markdown_file.with_name(changelog_ref)


def read_changelog_rows(changelog_path: Path) -> list[dict[str, object]]:
    if not changelog_path.exists():
        return []
    rows: list[dict[str, object]] = []
    for line in changelog_path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        rows.append(json.loads(line))
    return rows


def collect_records(repo_root: Path, scan_root: Path) -> list[tuple[Path, dict[str, object]]]:
    return [(path, build_record(repo_root, scan_root, path)) for path in iter_markdown_files(scan_root)]


def collect_registry(repo_root: Path, scan_root: Path) -> list[dict[str, object]]:
    registry: list[dict[str, object]] = []
    for file_path, record in collect_records(repo_root, scan_root):
        row: dict[str, object] = dict(record)
        row["abs_path"] = str(file_path)
        registry.append(row)
    registry.sort(key=lambda row: str(row.get("artifact", row["path"])))
    return registry


def run_quiet_check(scan_root: Path, paths: list[Path]) -> int:
    check_args = argparse.Namespace(root=scan_root, files=paths, quiet=True)
    return cmd_check(check_args)


def extract_link_targets(markdown_file: Path, body: str, repo_root: Path) -> list[dict[str, object]]:
    links: list[dict[str, object]] = []
    for target in MARKDOWN_LINK_RE.findall(body):
        target = target.strip()
        if not target or "://" in target or target.startswith("#"):
            continue
        candidate = (markdown_file.parent / target).resolve() if not target.startswith("/") else Path(target)
        payload: dict[str, object] = {"target": target}
        if candidate.exists():
            payload["resolved"] = relative_to_root(candidate, repo_root)
            payload["exists"] = True
        else:
            payload["resolved"] = relative_to_root(candidate, repo_root)
            payload["exists"] = False
        links.append(payload)
    return links


def rewrite_markdown_links(body: str, old_target: str, new_target: str) -> tuple[str, int]:
    replacements = 0

    def replace(match: re.Match[str]) -> str:
        nonlocal replacements
        target = match.group(1)
        if target != old_target:
            return match.group(0)
        replacements += 1
        return match.group(0).replace(f"({old_target})", f"({new_target})")

    updated = MARKDOWN_LINK_RE.sub(replace, body)
    return updated, replacements


def cmd_scan(args: argparse.Namespace) -> int:
    repo_root = Path(__file__).resolve().parents[2]
    scan_root = args.root.resolve()
    try:
        for file_path in iter_markdown_files(scan_root):
            record = build_record(repo_root, scan_root, file_path)
            if args.missing_only and record["state"]["has_footer"]:
                continue
            print(json.dumps(record, ensure_ascii=True, separators=(",", ":")))
    except BrokenPipeError:
        return 0
    return 0


def cmd_summary(args: argparse.Namespace) -> int:
    repo_root = Path(__file__).resolve().parents[2]
    scan_root = args.root.resolve()
    rows = [build_record(repo_root, scan_root, path) for path in iter_markdown_files(scan_root)]
    layer_counts = Counter(row.get("layer", "unknown") for row in rows)
    owner_counts = Counter(row.get("owner", "unknown") for row in rows)
    status_counts = Counter(row.get("state", {}).get("status", "missing") for row in rows)
    print(json.dumps({
        "summary": "totals",
        "root": str(scan_root),
        "files": len(rows),
        "missing_footer": sum(not row.get("state", {}).get("has_footer", False) for row in rows),
        "missing_changelog": sum(not row.get("state", {}).get("has_changelog", False) for row in rows),
        "missing_description": sum(not row.get("state", {}).get("has_description", False) for row in rows),
        "missing_purpose": sum(not row.get("state", {}).get("has_purpose", False) for row in rows),
    }, ensure_ascii=True, separators=(",", ":")))
    for label, counts, key in (
        ("layer", layer_counts, "layer"),
        ("owner", owner_counts, "owner"),
        ("status", status_counts, "status"),
    ):
        for value, count in sorted(counts.items()):
            print(json.dumps({"summary": label, key: value, "files": count}, ensure_ascii=True, separators=(",", ":")))
    return 0


def cmd_touch(args: argparse.Namespace) -> int:
    path = args.markdown_file.resolve()
    if not path.exists() or not path.is_file():
        print(f"markdown file not found: {path}", file=sys.stderr)
        return 2
    if is_footer_optional(path):
        append_changelog_event(
            changelog_path_for(path),
            bootstrap_optional_event(path, args, args.event, args.change_note),
        )
        set_result_note(args, f"{path.name} changelog updated; bootstrap file body left unchanged")
        return 0
    _, body, footer = load_markdown(path)
    if not footer:
        print(f"{path}: missing_footer", file=sys.stderr)
        return 2
    footer["updated_at"] = now_iso()
    write_markdown(path, body, footer)
    changelog_ref = footer.get("changelog_ref")
    if not changelog_ref:
        print(f"{path}: missing_footer_field:changelog_ref", file=sys.stderr)
        return 2
    changelog_path = path.with_name(changelog_ref)
    event = {
        "ts": footer["updated_at"],
        "event": args.event,
        "artifact_path": footer.get("artifact_path", ""),
        "artifact_type": footer.get("artifact_type", ""),
        "artifact_version": footer.get("artifact_version", ""),
        "artifact_revision": footer.get("artifact_revision", ""),
        "source_path": footer.get("source_path", path.as_posix()),
        "reason": args.change_note,
        "task_id": args.task_id,
        "actor": args.actor,
        "scope": args.scope,
        "tags": normalize_tag_list(args.tags),
    }
    append_changelog_event(changelog_path, event)
    return run_quiet_check(path.parent, [path])


def cmd_finalize_edit(args: argparse.Namespace) -> int:
    path = args.markdown_file.resolve()
    if not path.exists() or not path.is_file():
        print(f"markdown file not found: {path}", file=sys.stderr)
        return 2
    if is_footer_optional(path):
        append_changelog_event(
            changelog_path_for(path),
            bootstrap_optional_event(path, args, args.event, args.change_note),
        )
        set_result_note(args, f"{path.name} finalized via changelog only; bootstrap file body left unchanged")
        return 0
    _, body, footer = load_markdown(path)
    if not footer:
        print(f"{path}: missing_footer", file=sys.stderr)
        return 2
    if args.status:
        footer["status"] = args.status
    if args.artifact_version:
        footer["artifact_version"] = args.artifact_version
    if args.artifact_revision:
        footer["artifact_revision"] = args.artifact_revision
    for item in args.set:
        if "=" not in item:
            print(f"invalid --set pair: {item}", file=sys.stderr)
            return 2
        key, value = item.split("=", 1)
        footer[key.strip()] = value.strip()
    footer["updated_at"] = now_iso()
    write_markdown(path, body, footer)
    event = {
        "ts": footer["updated_at"],
        "event": args.event,
        "artifact_path": footer.get("artifact_path", ""),
        "artifact_type": footer.get("artifact_type", ""),
        "artifact_version": footer.get("artifact_version", ""),
        "artifact_revision": footer.get("artifact_revision", ""),
        "source_path": footer.get("source_path", path.as_posix()),
        "reason": args.change_note,
        "task_id": args.task_id,
        "actor": args.actor,
        "scope": args.scope,
        "tags": normalize_tag_list(args.tags),
    }
    if args.status:
        event["status"] = args.status
    if args.set:
        event["metadata_updates"] = args.set
    append_changelog_event(changelog_path_for(path), event)
    return run_quiet_check(path.parent, [path])


def cmd_init(args: argparse.Namespace) -> int:
    path = args.markdown_file.resolve()
    if path.exists():
        print(f"markdown file already exists: {path}", file=sys.stderr)
        return 2
    path.parent.mkdir(parents=True, exist_ok=True)
    created_at = now_iso()
    footer = {
        "artifact_path": args.artifact_path,
        "artifact_type": args.artifact_type,
        "artifact_version": str(args.artifact_version),
        "artifact_revision": args.artifact_revision or created_at[:10],
        "schema_version": str(args.schema_version),
        "status": args.status,
        "source_path": path.as_posix(),
        "created_at": created_at,
        "updated_at": created_at,
        "changelog_ref": path.with_suffix("").name + ".changelog.jsonl",
    }
    description = args.title or titleize_stem(path.stem)
    purpose_line = f"Purpose: {args.purpose}" if args.purpose else "Purpose:"
    body = f"# {description}\n\n{purpose_line}\n"
    write_markdown(path, body, footer)
    append_changelog_event(
        path.with_name(footer["changelog_ref"]),
        {
            "ts": created_at,
            "event": "artifact_initialized",
            "artifact_path": footer["artifact_path"],
            "artifact_type": footer["artifact_type"],
            "artifact_version": footer["artifact_version"],
            "artifact_revision": footer["artifact_revision"],
            "source_path": footer["source_path"],
            "reason": args.change_note,
            "task_id": args.task_id,
            "actor": args.actor,
            "scope": args.scope,
            "tags": normalize_tag_list(args.tags),
        },
    )
    return run_quiet_check(path.parent, [path])


def cmd_move(args: argparse.Namespace) -> int:
    src = args.markdown_file.resolve()
    dst = args.destination.resolve()
    if not src.exists() or not src.is_file():
        print(f"markdown file not found: {src}", file=sys.stderr)
        return 2
    if is_footer_optional(src):
        set_result_note(args, f"{src.name} skipped; bootstrap file mutation is disabled")
        return 0
    if dst.exists():
        print(f"destination already exists: {dst}", file=sys.stderr)
        return 2
    _, body, footer = load_markdown(src)
    changelog_src = changelog_path_for(src)
    changelog_dst_name = dst.with_suffix("").name + ".changelog.jsonl"
    dst.parent.mkdir(parents=True, exist_ok=True)
    footer["source_path"] = dst.as_posix()
    footer["updated_at"] = now_iso()
    footer["changelog_ref"] = changelog_dst_name
    write_markdown(dst, body, footer)
    if changelog_src.exists():
        changelog_dst = dst.with_name(changelog_dst_name)
        changelog_dst.write_text(changelog_src.read_text(encoding="utf-8"), encoding="utf-8")
        append_changelog_event(
            changelog_dst,
            {
                "ts": footer["updated_at"],
                "event": "artifact_moved",
                "artifact_path": footer.get("artifact_path", ""),
                "artifact_type": footer.get("artifact_type", ""),
                "artifact_version": footer.get("artifact_version", ""),
                "artifact_revision": footer.get("artifact_revision", ""),
                "source_path": footer["source_path"],
                "reason": args.change_note,
                "task_id": args.task_id,
                "actor": args.actor,
                "scope": args.scope,
                "tags": normalize_tag_list(args.tags),
                "previous_source_path": src.as_posix(),
            },
        )
        changelog_src.unlink()
    src.unlink()
    return run_quiet_check(dst.parent, [dst])


def cmd_rename_artifact(args: argparse.Namespace) -> int:
    path = args.markdown_file.resolve()
    if not path.exists() or not path.is_file():
        print(f"markdown file not found: {path}", file=sys.stderr)
        return 2
    if is_footer_optional(path):
        set_result_note(args, f"{path.name} skipped; bootstrap file mutation is disabled")
        return 0
    _, body, footer = load_markdown(path)
    if not footer:
        print(f"{path}: missing_footer", file=sys.stderr)
        return 2
    previous = footer.get("artifact_path", "")
    footer["artifact_path"] = args.artifact_path
    if args.artifact_type:
        footer["artifact_type"] = args.artifact_type
    if args.bump_version:
        footer["artifact_version"] = str(int(footer.get("artifact_version", "0")) + 1)
    footer["updated_at"] = now_iso()
    write_markdown(path, body, footer)
    append_changelog_event(
        changelog_path_for(path),
        {
            "ts": footer["updated_at"],
            "event": args.event,
            "artifact_path": footer.get("artifact_path", ""),
            "artifact_type": footer.get("artifact_type", ""),
            "artifact_version": footer.get("artifact_version", ""),
            "artifact_revision": footer.get("artifact_revision", ""),
            "source_path": footer.get("source_path", path.as_posix()),
            "reason": args.change_note,
            "task_id": args.task_id,
            "actor": args.actor,
            "scope": args.scope,
            "tags": normalize_tag_list(args.tags),
            "previous_artifact_path": previous,
        },
    )
    return run_quiet_check(path.parent, [path])


def cmd_changelog(args: argparse.Namespace) -> int:
    path = args.markdown_file.resolve()
    if not path.exists() or not path.is_file():
        print(f"markdown file not found: {path}", file=sys.stderr)
        return 2
    try:
        changelog_path = changelog_path_for(path)
    except ValueError as exc:
        print(f"{path}: {exc}", file=sys.stderr)
        return 2
    if not changelog_path.exists():
        print(f"{path}: missing_changelog", file=sys.stderr)
        return 2
    rows = read_changelog_rows(changelog_path)
    rows.sort(key=lambda row: parse_ts(row.get("ts", "")))
    if args.newest_first:
        rows = rows[::-1]
    if args.limit > 0:
        rows = rows[: args.limit]
    try:
        for row in rows:
            print(json.dumps(row, ensure_ascii=True, separators=(",", ":")))
    except BrokenPipeError:
        return 0
    return 0


def cmd_changelog_task(args: argparse.Namespace) -> int:
    repo_root = Path(__file__).resolve().parents[2]
    scan_root = args.root.resolve()
    matched: list[dict[str, object]] = []
    for markdown_file in iter_markdown_files(scan_root):
        try:
            changelog_path = changelog_path_for(markdown_file)
        except ValueError:
            continue
        for row in read_changelog_rows(changelog_path):
            if str(row.get("task_id", "")).strip() != args.task_id:
                continue
            rel = markdown_file.relative_to(scan_root)
            payload = {
                "path": rel.as_posix(),
                "changelog": changelog_path.name,
                "artifact": row.get("artifact_path", ""),
                "event": row.get("event", ""),
                "ts": row.get("ts", ""),
                "task_id": row.get("task_id", ""),
                "reason": row.get("reason", ""),
            }
            if row.get("actor", ""):
                payload["actor"] = row["actor"]
            if row.get("scope", ""):
                payload["scope"] = row["scope"]
            if row.get("tags", []):
                payload["tags"] = row["tags"]
            matched.append(payload)
    matched.sort(key=lambda item: parse_ts(item.get("ts", "")))
    if args.newest_first:
        matched = matched[::-1]
    if args.limit > 0:
        matched = matched[: args.limit]
    try:
        for row in matched:
            print(json.dumps(row, ensure_ascii=True, separators=(",", ":")))
    except BrokenPipeError:
        return 0
    return 0


def cmd_task_summary(args: argparse.Namespace) -> int:
    scan_root = args.root.resolve()
    matched: list[dict[str, object]] = []
    actors: Counter[str] = Counter()
    scopes: Counter[str] = Counter()
    tags: Counter[str] = Counter()
    files: Counter[str] = Counter()
    first_ts = ""
    last_ts = ""
    for markdown_file in iter_markdown_files(scan_root):
        try:
            changelog_path = changelog_path_for(markdown_file)
        except ValueError:
            continue
        for row in read_changelog_rows(changelog_path):
            if str(row.get("task_id", "")).strip() != args.task_id:
                continue
            rel = markdown_file.relative_to(scan_root).as_posix()
            matched.append(row)
            files[rel] += 1
            if row.get("actor"):
                actors[str(row["actor"])] += 1
            if row.get("scope"):
                scopes[str(row["scope"])] += 1
            for tag in row.get("tags", []) or []:
                tags[str(tag)] += 1
            ts = str(row.get("ts", ""))
            if ts and (not first_ts or parse_ts(ts) < parse_ts(first_ts)):
                first_ts = ts
            if ts and (not last_ts or parse_ts(ts) > parse_ts(last_ts)):
                last_ts = ts
    print(json.dumps({
        "summary": "task",
        "task_id": args.task_id,
        "root": str(scan_root),
        "events": len(matched),
        "files": len(files),
        "first_ts": first_ts,
        "last_ts": last_ts,
    }, ensure_ascii=True, separators=(",", ":")))
    for label, counts in (("file", files), ("actor", actors), ("scope", scopes), ("tag", tags)):
        for value, count in sorted(counts.items()):
            print(json.dumps({"summary": label, label: value, "events": count, "task_id": args.task_id}, ensure_ascii=True, separators=(",", ":")))
    return 0


def cmd_deps(args: argparse.Namespace) -> int:
    repo_root = Path(__file__).resolve().parents[2]
    path = args.markdown_file.resolve()
    if not path.exists() or not path.is_file():
        print(f"markdown file not found: {path}", file=sys.stderr)
        return 2
    _, body, footer = load_markdown(path)
    links = extract_link_targets(path, body, repo_root)
    deps: list[dict[str, object]] = []
    refs = ("projection_ref", "contract_ref", "template_ref", "parent_definition_ref")
    for key in refs:
        if footer.get(key):
            deps.append({"kind": key, "target": footer[key]})
    reverse: list[str] = []
    root = repo_root
    for other in iter_markdown_files(root):
        if other == path:
            continue
        other_text, other_body, other_footer = load_markdown(other)
        if relative_to_root(path, root) in other_text or footer.get("artifact_path", "") and footer["artifact_path"] in json.dumps(other_footer):
            reverse.append(relative_to_root(other, root))
    payload = {
        "path": relative_to_root(path, repo_root),
        "artifact": footer.get("artifact_path", ""),
        "links": links,
        "footer_refs": deps,
        "referenced_by": sorted(reverse),
    }
    print(json.dumps(payload, ensure_ascii=True, separators=(",", ":")))
    return 0


def cmd_registry(args: argparse.Namespace) -> int:
    repo_root = Path(__file__).resolve().parents[2]
    scan_root = args.root.resolve()
    try:
        for row in collect_registry(repo_root, scan_root):
            row.pop("abs_path", None)
            print(json.dumps(row, ensure_ascii=True, separators=(",", ":")))
    except BrokenPipeError:
        return 0
    return 0


def cmd_links(args: argparse.Namespace) -> int:
    repo_root = Path(__file__).resolve().parents[2]
    target = args.path.resolve()
    if not target.exists():
        print(f"path not found: {target}", file=sys.stderr)
        return 2
    files = [target] if target.is_file() else list(iter_markdown_files(target))
    try:
        for file_path in files:
            if file_path.suffix.lower() != ".md":
                continue
            _, body, footer = load_markdown(file_path)
            payload = {
                "path": relative_to_root(file_path, repo_root),
                "artifact": footer.get("artifact_path", ""),
                "links": extract_link_targets(file_path, body, repo_root),
            }
            print(json.dumps(payload, ensure_ascii=True, separators=(",", ":")))
    except BrokenPipeError:
        return 0
    return 0


def cmd_migrate_links(args: argparse.Namespace) -> int:
    target = args.path.resolve()
    if not target.exists():
        print(f"path not found: {target}", file=sys.stderr)
        return 2
    files = [target] if target.is_file() else list(iter_markdown_files(target))
    changed: list[Path] = []
    skipped_optional = 0
    for file_path in files:
        if file_path.suffix.lower() != ".md":
            continue
        if is_footer_optional(file_path):
            skipped_optional += 1
            continue
        _, body, footer = load_markdown(file_path)
        if not footer:
            print(f"{file_path}: missing_footer", file=sys.stderr)
            return 2
        updated_body, replacements = rewrite_markdown_links(body, args.old_target, args.new_target)
        if replacements == 0:
            continue
        footer["updated_at"] = now_iso()
        write_markdown(file_path, updated_body, footer)
        append_changelog_event(
            changelog_path_for(file_path),
            {
                "ts": footer["updated_at"],
                "event": "links_migrated",
                "artifact_path": footer.get("artifact_path", ""),
                "artifact_type": footer.get("artifact_type", ""),
                "artifact_version": footer.get("artifact_version", ""),
                "artifact_revision": footer.get("artifact_revision", ""),
                "source_path": footer.get("source_path", file_path.as_posix()),
                "reason": args.change_note,
                "task_id": args.task_id,
                "actor": args.actor,
                "scope": args.scope,
                "tags": normalize_tag_list(args.tags),
                "old_target": args.old_target,
                "new_target": args.new_target,
                "replacements": replacements,
            },
        )
        changed.append(file_path)
    if not changed:
        if skipped_optional:
            set_result_note(args, f"skipped {skipped_optional} bootstrap file(s); no markdown links were mutated")
        return 0
    return run_quiet_check(target if target.is_dir() else target.parent, changed)


def cmd_doctor(args: argparse.Namespace) -> int:
    repo_root = Path(__file__).resolve().parents[2]
    scan_root = args.root.resolve()
    had_error = False
    artifact_seen: dict[str, str] = {}
    changelog_seen: dict[str, str] = {}
    markdowns = list(iter_markdown_files(scan_root))
    markdown_set = {path.resolve() for path in markdowns}
    for file_path in markdowns:
        record = build_record(repo_root, scan_root, file_path)
        problems = validate_record(file_path, record)
        _, body, footer = load_markdown(file_path)
        problems.extend(validate_footer_consistency(file_path, footer))
        artifact = footer.get("artifact_path", "")
        if artifact:
            owner = artifact_seen.get(artifact)
            rel = file_path.relative_to(scan_root).as_posix()
            if owner and owner != rel:
                problems.append(f"duplicate_artifact:{artifact}")
            else:
                artifact_seen[artifact] = rel
        changelog_ref = footer.get("changelog_ref", "")
        if changelog_ref:
            rel = file_path.relative_to(scan_root).as_posix()
            owner = changelog_seen.get(changelog_ref)
            if owner and owner != rel:
                problems.append(f"duplicate_changelog_ref:{changelog_ref}")
            else:
                changelog_seen[changelog_ref] = rel
            changelog_path = file_path.with_name(changelog_ref)
            latest_row = latest_changelog_row(changelog_path)
            if latest_row and latest_row.get("source_path") and str(latest_row["source_path"]) != footer.get("source_path", ""):
                problems.append("latest_changelog_source_path_mismatch")
        for link in extract_link_targets(file_path, body, repo_root):
            if not link["exists"] and not str(link["target"]).startswith(("http://", "https://")):
                problems.append(f"broken_link:{link['target']}")
        if problems:
            had_error = True
            print(json.dumps({"path": file_path.relative_to(scan_root).as_posix(), "issues": sorted(set(problems))}, ensure_ascii=True, separators=(",", ":")))
    changelog_paths = {path.with_name(split_body_and_footer(path.read_text(encoding='utf-8'))[1].get("changelog_ref", "")) for path in markdowns if split_body_and_footer(path.read_text(encoding='utf-8'))[1].get("changelog_ref", "")}
    for changelog_file in scan_root.rglob("*.changelog.jsonl"):
        if changelog_file.resolve() not in {item.resolve() for item in changelog_paths}:
            had_error = True
            print(json.dumps({"path": changelog_file.relative_to(scan_root).as_posix(), "issues": ["orphan_changelog"]}, ensure_ascii=True, separators=(",", ":")))
    return 1 if had_error else 0


def cmd_check(args: argparse.Namespace) -> int:
    repo_root = Path(__file__).resolve().parents[2]
    scan_root = args.root.resolve()
    paths = [Path(p).resolve() for p in args.files] if args.files else list(iter_markdown_files(scan_root))
    had_error = False
    for file_path in paths:
        if not file_path.exists() or file_path.suffix.lower() != ".md":
            continue
        rel_root = file_path.parent if args.files else scan_root
        rel = file_path.relative_to(rel_root) if file_path.is_relative_to(rel_root) else file_path.relative_to(repo_root)
        record = build_record(repo_root, rel_root if args.files else scan_root, file_path)
        problems = validate_record(file_path, record)
        if problems:
            had_error = True
            print(json.dumps({"path": rel.as_posix(), "issues": problems}, ensure_ascii=True, separators=(",", ":")))
    return 1 if had_error else 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Unified VIDA markdown document toolkit.",
        formatter_class=argparse.RawTextHelpFormatter,
    )
    sub = parser.add_subparsers(dest="command", required=True)

    scan = sub.add_parser("scan", help="Emit one compact JSONL row per markdown file.")
    scan.add_argument("--root", type=Path, required=True, help="Root directory to scan.")
    scan.add_argument("--missing-only", action="store_true", help="Emit only files missing footer metadata.")
    scan.set_defaults(func=cmd_scan)

    summary = sub.add_parser("summary", help="Emit compact JSONL summary rows.")
    summary.add_argument("--root", type=Path, required=True, help="Root directory to summarize.")
    summary.set_defaults(func=cmd_summary)

    registry = sub.add_parser("registry", help="Emit one registry JSONL row per markdown file.")
    registry.add_argument("--root", type=Path, required=True, help="Root directory to scan.")
    registry.set_defaults(func=cmd_registry)

    touch = sub.add_parser("touch", help="Update updated_at and append a changelog event.")
    touch.add_argument("markdown_file", type=Path, help="Path to markdown file.")
    touch.add_argument("change_note", help="Short changelog note.")
    touch.add_argument("--event", default="artifact_revision_updated", help="Changelog event name.")
    touch.add_argument("--task-id", default="", help="Optional task identifier for the change.")
    touch.add_argument("--actor", default="manual", help="Actor responsible for the change. Default: manual")
    touch.add_argument("--scope", default="", help="Optional bounded scope label for the change.")
    touch.add_argument("--tags", default="", help="Optional comma-separated tags for the change event.")
    touch.set_defaults(func=cmd_touch)

    finalize_edit = sub.add_parser(
        "finalize-edit",
        help="Finalize one or more manual diff edits with a single metadata/changelog update.",
    )
    finalize_edit.add_argument("markdown_file", type=Path, help="Path to markdown file.")
    finalize_edit.add_argument("change_note", help="Short changelog note for the finalized edit batch.")
    finalize_edit.add_argument("--event", default="artifact_revision_updated", help="Changelog event name.")
    finalize_edit.add_argument("--status", default="", help="Optional new status value.")
    finalize_edit.add_argument("--artifact-version", default="", help="Optional new artifact version.")
    finalize_edit.add_argument("--artifact-revision", default="", help="Optional new artifact revision.")
    finalize_edit.add_argument("--set", action="append", default=[], help="Optional metadata override in key=value form. Repeatable.")
    finalize_edit.add_argument("--task-id", default="", help="Optional task identifier for the change.")
    finalize_edit.add_argument("--actor", default="manual", help="Actor responsible for the change. Default: manual")
    finalize_edit.add_argument("--scope", default="", help="Optional bounded scope label for the change.")
    finalize_edit.add_argument("--tags", default="", help="Optional comma-separated tags for the change event.")
    finalize_edit.set_defaults(func=cmd_finalize_edit)

    init = sub.add_parser("init", help="Create a new canonical markdown artifact with footer and changelog.")
    init.add_argument("markdown_file", type=Path, help="Path to markdown file to create.")
    init.add_argument("artifact_path", help="Canonical artifact path for the new document.")
    init.add_argument("artifact_type", help="Artifact type for the new document.")
    init.add_argument("change_note", help="Initialization changelog note.")
    init.add_argument("--title", default="", help="Optional title line. Defaults to a titleized file stem.")
    init.add_argument("--purpose", default="", help="Optional Purpose: line.")
    init.add_argument("--artifact-version", type=int, default=1, help="Artifact version. Default: 1")
    init.add_argument("--artifact-revision", default="", help="Artifact revision. Default: current date")
    init.add_argument("--schema-version", type=int, default=1, help="Footer schema version. Default: 1")
    init.add_argument("--status", default="canonical", help="Artifact status. Default: canonical")
    init.add_argument("--task-id", default="", help="Optional task identifier for the change.")
    init.add_argument("--actor", default="manual", help="Actor responsible for the change. Default: manual")
    init.add_argument("--scope", default="", help="Optional bounded scope label for the change.")
    init.add_argument("--tags", default="", help="Optional comma-separated tags for the change event.")
    init.set_defaults(func=cmd_init)

    move = sub.add_parser("move", help="Move a markdown artifact and its changelog, then validate quietly.")
    move.add_argument("markdown_file", type=Path, help="Existing markdown file to move.")
    move.add_argument("destination", type=Path, help="Destination markdown path.")
    move.add_argument("change_note", help="Short changelog note.")
    move.add_argument("--task-id", default="", help="Optional task identifier for the change.")
    move.add_argument("--actor", default="manual", help="Actor responsible for the change. Default: manual")
    move.add_argument("--scope", default="", help="Optional bounded scope label for the change.")
    move.add_argument("--tags", default="", help="Optional comma-separated tags for the change event.")
    move.set_defaults(func=cmd_move)

    rename_artifact = sub.add_parser("rename-artifact", help="Update artifact_path and optional artifact_type for one markdown file.")
    rename_artifact.add_argument("markdown_file", type=Path, help="Markdown file to update.")
    rename_artifact.add_argument("artifact_path", help="New canonical artifact path.")
    rename_artifact.add_argument("change_note", help="Short changelog note.")
    rename_artifact.add_argument("--artifact-type", default="", help="Optional new artifact type.")
    rename_artifact.add_argument("--bump-version", action="store_true", help="Increment artifact_version after rename.")
    rename_artifact.add_argument("--event", default="artifact_path_updated", help="Changelog event name.")
    rename_artifact.add_argument("--task-id", default="", help="Optional task identifier for the change.")
    rename_artifact.add_argument("--actor", default="manual", help="Actor responsible for the change. Default: manual")
    rename_artifact.add_argument("--scope", default="", help="Optional bounded scope label for the change.")
    rename_artifact.add_argument("--tags", default="", help="Optional comma-separated tags for the change event.")
    rename_artifact.set_defaults(func=cmd_rename_artifact)

    changelog = sub.add_parser("changelog", help="Print changelog events for a markdown file.")
    changelog.add_argument("markdown_file", type=Path, help="Path to markdown file.")
    changelog.add_argument("--limit", type=int, default=20, help="Maximum number of changelog rows to emit. Default: 20")
    changelog.add_argument(
        "--newest-first",
        action="store_true",
        help="Show newest events first. By default, output is oldest to newest by timestamp.",
    )
    changelog.set_defaults(func=cmd_changelog)

    changelog_task = sub.add_parser("changelog-task", help="Print changelog events for one task_id across a scope.")
    changelog_task.add_argument("--root", type=Path, required=True, help="Root directory to scan for markdown files.")
    changelog_task.add_argument("task_id", help="Task identifier to match inside changelog rows.")
    changelog_task.add_argument("--limit", type=int, default=0, help="Maximum number of matched rows to emit. Default: 0 (no limit)")
    changelog_task.add_argument(
        "--newest-first",
        action="store_true",
        help="Show newest matching rows first. By default, output is oldest to newest by timestamp.",
    )
    changelog_task.set_defaults(func=cmd_changelog_task)

    task_summary = sub.add_parser("task-summary", help="Emit aggregate JSONL summary rows for one task_id across a scope.")
    task_summary.add_argument("--root", type=Path, required=True, help="Root directory to scan for markdown files.")
    task_summary.add_argument("task_id", help="Task identifier to match inside changelog rows.")
    task_summary.set_defaults(func=cmd_task_summary)

    deps = sub.add_parser("deps", help="Emit compact dependency information for one markdown file.")
    deps.add_argument("markdown_file", type=Path, help="Markdown file to inspect.")
    deps.set_defaults(func=cmd_deps)

    links = sub.add_parser("links", help="Emit compact markdown-link rows for one file or a whole directory.")
    links.add_argument("path", type=Path, help="Markdown file or directory to inspect.")
    links.set_defaults(func=cmd_links)

    migrate_links = sub.add_parser("migrate-links", help="Rewrite markdown link targets in one file or across a directory.")
    migrate_links.add_argument("path", type=Path, help="Markdown file or directory to update.")
    migrate_links.add_argument("old_target", help="Exact markdown link target to replace.")
    migrate_links.add_argument("new_target", help="New markdown link target.")
    migrate_links.add_argument("change_note", help="Short changelog note.")
    migrate_links.add_argument("--task-id", default="", help="Optional task identifier for the change.")
    migrate_links.add_argument("--actor", default="manual", help="Actor responsible for the change. Default: manual")
    migrate_links.add_argument("--scope", default="", help="Optional bounded scope label for the change.")
    migrate_links.add_argument("--tags", default="", help="Optional comma-separated tags for the change event.")
    migrate_links.set_defaults(func=cmd_migrate_links)

    check = sub.add_parser("check", help="Validate markdown footer/changelog health.")
    check.add_argument("--root", type=Path, required=True, help="Root directory for default scan scope.")
    check.add_argument("files", nargs="*", help="Optional explicit markdown files to validate.")
    check.add_argument("--quiet", action="store_true", help="Reserved for wrapper compatibility.")
    check.set_defaults(func=cmd_check)

    doctor = sub.add_parser("doctor", help="Run stronger consistency checks for markdown artifacts and changelogs.")
    doctor.add_argument("--root", type=Path, required=True, help="Root directory to scan.")
    doctor.set_defaults(func=cmd_doctor)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    code = args.func(args)
    status = "✅ OK" if code == 0 else "❌ ERROR"
    suffix = ""
    if hasattr(args, "_result_note") and getattr(args, "_result_note"):
        suffix = f" ({getattr(args, '_result_note')})"
    print(f"{status}: {args.command}{suffix}", file=sys.stderr)
    return code


if __name__ == "__main__":
    raise SystemExit(main())
