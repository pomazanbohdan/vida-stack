#!/usr/bin/env python3
"""Unified canonical documentation and inventory toolkit for VIDA."""

from __future__ import annotations

import fnmatch
import json
import os
from collections import Counter
from datetime import datetime
from io import StringIO
from pathlib import Path
from typing import Annotated

import msgspec
import toon_format
import typer
from ruamel.yaml import YAML


app = typer.Typer(no_args_is_help=True, add_completion=False, pretty_exceptions_enable=False)

REPO_ROOT = Path(__file__).resolve().parents[1]
POLICY_PATH = Path(__file__).resolve().with_name("docsys_policy.yaml")
SCHEMA_PATH = Path(__file__).resolve().with_name("docsys_schema.yaml")
PROJECT_PATH = Path(os.environ.get("DOCSYS_PROJECT_CONFIG", Path(__file__).resolve().with_name("docsys_project.yaml")))
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
MARKDOWN_EXTS = {".md", ".MD"}

yaml = YAML()
yaml.default_flow_style = False
yaml.allow_unicode = True
yaml.width = 4096

result_note = ""
current_command = "codex"


class PolicyRule(msgspec.Struct):
    scope: str = "any"
    glob: str = ""
    reason: str = ""


class PolicyConfig(msgspec.Struct):
    schema_version: int = 1
    footer_optional: list[PolicyRule] = []
    changelog_only: list[PolicyRule] = []
    mutation_disabled: list[PolicyRule] = []
    scan_ignored: list[PolicyRule] = []
    profiles: dict[str, list[str]] = {}


class ProfileSettings(msgspec.Struct):
    warnings_fail: bool = False


class SchemaRegistry(msgspec.Struct):
    path: str = "vida/config/codex-registry.current.jsonl"


class SchemaConfig(msgspec.Struct):
    schema_version: int = 1
    statuses: list[str] = []
    owners: list[str] = []
    layers: list[str] = []
    artifact_types: list[str] = []
    profiles: dict[str, ProfileSettings] = {}
    canonical_registry: SchemaRegistry = msgspec.field(default_factory=SchemaRegistry)


class LayerOwnerRule(msgspec.Struct):
    pattern: str
    layer: str
    owner: str


class RootBootstrapConfig(msgspec.Struct):
    artifact_prefix: str = "project/repository"
    artifact_type: str = "bootstrap_doc"


class ProjectConfig(msgspec.Struct):
    schema_version: int = 1
    root_bootstrap: RootBootstrapConfig = msgspec.field(default_factory=RootBootstrapConfig)
    layer_owner_rules: dict[str, list[LayerOwnerRule]] = {}


class FooterMetadata(msgspec.Struct):
    artifact_path: str
    artifact_type: str
    artifact_version: str
    artifact_revision: str
    schema_version: str
    status: str
    source_path: str
    created_at: str
    updated_at: str
    changelog_ref: str


class ChangelogEvent(msgspec.Struct, kw_only=True):
    ts: str
    event: str
    artifact_path: str = ""
    artifact_type: str = ""
    artifact_version: str | int = ""
    artifact_revision: str | int = ""
    source_path: str = ""
    reason: str = ""
    task_id: str = ""
    actor: str = ""
    scope: str = ""
    tags: list[str] = []


def set_command(name: str) -> None:
    global current_command
    current_command = name


def set_result_note(note: str) -> None:
    global result_note
    result_note = note


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


def relative_to_root(path: Path, root: Path = REPO_ROOT) -> str:
    try:
        return path.resolve().relative_to(root).as_posix()
    except ValueError:
        return path.as_posix()


def load_policy() -> PolicyConfig:
    data = yaml.load(POLICY_PATH.read_text(encoding="utf-8")) or {}
    return msgspec.convert(data, type=PolicyConfig)


def load_schema() -> SchemaConfig:
    data = yaml.load(SCHEMA_PATH.read_text(encoding="utf-8")) or {}
    return msgspec.convert(data, type=SchemaConfig)


def load_project() -> ProjectConfig:
    data = yaml.load(PROJECT_PATH.read_text(encoding="utf-8")) or {}
    return msgspec.convert(data, type=ProjectConfig)


def profile_names() -> list[str]:
    return sorted(load_policy().profiles.keys())


def profile_settings(profile_name: str) -> ProfileSettings:
    schema = load_schema()
    return schema.profiles.get(profile_name, ProfileSettings())


def match_policy_rule(markdown_file: Path, rule: PolicyRule) -> bool:
    target = markdown_file.resolve()
    try:
        rel = target.relative_to(REPO_ROOT)
    except ValueError:
        rel = Path(markdown_file.name)
    rel_posix = rel.as_posix()
    if rule.scope == "repo_root":
        return len(rel.parts) == 1 and fnmatch.fnmatch(markdown_file.name, rule.glob)
    if rule.scope == "relative_path":
        return fnmatch.fnmatch(rel_posix, rule.glob)
    if rule.scope == "filename":
        return fnmatch.fnmatch(markdown_file.name, rule.glob)
    return fnmatch.fnmatch(rel_posix, rule.glob) or fnmatch.fnmatch(markdown_file.name, rule.glob)


def policy_allows(markdown_file: Path, policy_key: str) -> bool:
    policy = load_policy()
    return any(match_policy_rule(markdown_file, rule) for rule in getattr(policy, policy_key))


def policy_reason(markdown_file: Path, policy_key: str) -> str:
    policy = load_policy()
    for rule in getattr(policy, policy_key):
        if match_policy_rule(markdown_file, rule):
            return rule.reason
    return ""


def is_footer_optional(path: Path) -> bool:
    return policy_allows(path, "footer_optional")


def is_changelog_only(path: Path) -> bool:
    return policy_allows(path, "changelog_only")


def is_mutation_disabled(path: Path) -> bool:
    return policy_allows(path, "mutation_disabled")


def is_scan_ignored(path: Path) -> bool:
    return policy_allows(path, "scan_ignored")


def roots_for_profile(profile_name: str) -> list[Path]:
    policy = load_policy()
    if profile_name not in policy.profiles:
        raise KeyError(profile_name)
    roots: list[Path] = []
    for rel in policy.profiles[profile_name]:
        roots.append(REPO_ROOT if rel == "." else (REPO_ROOT / rel).resolve())
    return roots


def split_body_and_footer(text: str) -> tuple[str, dict[str, str]]:
    idx = text.rfind(FOOTER_MARKER)
    if idx < 0:
        return text, {}
    body = text[:idx]
    footer_text = text[idx + len(FOOTER_MARKER):].strip()
    loaded = yaml.load(footer_text) or {}
    return body, {str(k): str(v) for k, v in loaded.items()}


def render_footer(footer: dict[str, str]) -> str:
    ordered: dict[str, str] = {}
    for key in REQUIRED_FOOTER_FIELDS:
        if key in footer:
            ordered[key] = footer[key]
    for key, value in footer.items():
        if key not in ordered:
            ordered[key] = value
    buf = StringIO()
    yaml.dump(ordered, buf)
    return "-----\n" + buf.getvalue()


def load_markdown(markdown_file: Path) -> tuple[str, str, dict[str, str]]:
    text = markdown_file.read_text(encoding="utf-8")
    body, footer = split_body_and_footer(text)
    return text, body, footer


def write_markdown(markdown_file: Path, body: str, footer: dict[str, str]) -> None:
    markdown_file.write_text(body.rstrip() + "\n\n" + render_footer(footer), encoding="utf-8")


def normalize_tag_list(raw: str) -> list[str]:
    return [item.strip() for item in raw.split(",") if item.strip()]


def validate_footer(footer: dict[str, str]) -> None:
    missing = [field for field in REQUIRED_FOOTER_FIELDS if not footer.get(field)]
    if missing:
        raise ValueError(",".join(missing))
    msgspec.convert({key: footer[key] for key in REQUIRED_FOOTER_FIELDS}, type=FooterMetadata)


def append_changelog_event(changelog_path: Path, event: dict[str, object]) -> None:
    typed = msgspec.convert(event, type=ChangelogEvent)
    with changelog_path.open("ab") as fh:
        fh.write(msgspec.json.encode(typed))
        fh.write(b"\n")


def read_changelog_rows(changelog_path: Path) -> list[dict[str, object]]:
    if not changelog_path.exists():
        return []
    rows: list[dict[str, object]] = []
    for raw in changelog_path.read_bytes().splitlines():
        if not raw.strip():
            continue
        rows.append(msgspec.to_builtins(msgspec.json.decode(raw, type=ChangelogEvent)))
    return rows


def latest_changelog_row(changelog_path: Path) -> dict[str, object] | None:
    rows = read_changelog_rows(changelog_path)
    if not rows:
        return None
    rows.sort(key=lambda row: parse_ts(row.get("ts", "")))
    return rows[-1]


def changelog_path_for(markdown_file: Path) -> Path:
    _, _, footer = load_markdown(markdown_file)
    changelog_ref = footer.get("changelog_ref", "")
    if not changelog_ref and is_footer_optional(markdown_file):
        changelog_ref = markdown_file.with_suffix("").name + ".changelog.jsonl"
    if not changelog_ref:
        raise ValueError("footer metadata is missing changelog_ref")
    return markdown_file.with_name(changelog_ref)


def bootstrap_optional_event(markdown_file: Path, event_name: str, reason: str, task_id: str, actor: str, scope: str, tags: list[str]) -> dict[str, object]:
    project = load_project()
    return {
        "ts": now_iso(),
        "event": event_name,
        "artifact_path": f"{project.root_bootstrap.artifact_prefix}/{markdown_file.stem.lower()}",
        "artifact_type": project.root_bootstrap.artifact_type,
        "artifact_version": "",
        "artifact_revision": "",
        "source_path": relative_to_root(markdown_file),
        "reason": reason,
        "task_id": task_id,
        "actor": actor,
        "scope": scope,
        "tags": tags,
    }


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
        if line.lower().startswith("purpose:"):
            return line.split(":", 1)[1].strip()
    return ""


def titleize_stem(stem: str) -> str:
    return " ".join(part.capitalize() for part in stem.replace(".", "-").split("-") if part)


def iter_markdown_files(scan_root: Path):
    for path in sorted(scan_root.rglob("*")):
        if path.is_file() and path.suffix.lower() == ".md" and not is_scan_ignored(path):
            yield path


def classify_layer_and_owner(scan_root: Path, rel: Path, artifact_path: str) -> tuple[str, str]:
    project = load_project()
    normalized_artifact = artifact_path.replace("\\", "/")
    rel_posix = relative_to_root((scan_root / rel).resolve())
    for rule in project.layer_owner_rules.get("by_relative_path", []):
        if fnmatch.fnmatch(rel_posix, rule.pattern):
            return rule.layer, rule.owner
    for rule in project.layer_owner_rules.get("by_artifact_prefix", []):
        if normalized_artifact.startswith(rule.pattern):
            return rule.layer, rule.owner
    return "unknown", "unknown"


def build_record(scan_root: Path, file_path: Path) -> dict[str, object]:
    _, body, footer = load_markdown(file_path)
    rel = file_path.relative_to(scan_root)
    description = extract_description(body)
    purpose = extract_purpose(body)
    changelog_name = footer.get("changelog_ref", "")
    changelog_path = file_path.with_name(changelog_name) if changelog_name else None
    artifact_path = footer.get("artifact_path", "")
    artifact_type = footer.get("artifact_type", "")
    status = footer.get("status", "missing")
    layer, owner = classify_layer_and_owner(scan_root, rel, artifact_path)
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
    schema = load_schema()
    state = record["state"]
    if not state["has_footer"]:
        if not is_footer_optional(file_path):
            problems.append("missing_footer")
        return problems
    _, _, footer = load_markdown(file_path)
    for field in REQUIRED_FOOTER_FIELDS:
        if not footer.get(field):
            problems.append(f"missing_footer_field:{field}")
    try:
        validate_footer(footer)
    except Exception:
        problems.append("invalid_footer_schema")
    kind = str(record.get("kind", ""))
    if kind and kind not in schema.artifact_types:
        problems.append(f"unknown_artifact_type:{kind}")
    layer = str(record.get("layer", ""))
    if layer and layer not in schema.layers:
        problems.append(f"unknown_layer:{layer}")
    owner = str(record.get("owner", ""))
    if owner and owner not in schema.owners:
        problems.append(f"unknown_owner:{owner}")
    status = str(state.get("status", ""))
    if status and status not in schema.statuses:
        problems.append(f"unknown_status:{status}")
    if record.get("artifact") and not record.get("kind"):
        problems.append("missing_kind")
    if not state["has_changelog"]:
        problems.append("missing_changelog")
    return problems


def validate_footer_consistency(markdown_file: Path, footer: dict[str, str]) -> list[str]:
    problems: list[str] = []
    if footer.get("source_path") and footer["source_path"] != relative_to_root(markdown_file):
        problems.append("source_path_mismatch")
    return problems


def collect_registry(scan_root: Path) -> list[dict[str, object]]:
    rows = [build_record(scan_root, path) for path in iter_markdown_files(scan_root)]
    rows.sort(key=lambda row: str(row.get("artifact", row["path"])))
    return rows


def registry_rows_for(root: Path | None, profile: str) -> list[dict[str, object]]:
    if profile:
        rows = [build_record(scope_root, path) for scope_root, path in scan_targets(root, profile)]
    else:
        rows = collect_registry(root.resolve())
    rows.sort(key=lambda row: str(row.get("artifact", row["path"])))
    return rows


def summary_payload(rows: list[dict[str, object]], root_label: str) -> dict[str, object]:
    layer_counts = Counter(row.get("layer", "unknown") for row in rows)
    owner_counts = Counter(row.get("owner", "unknown") for row in rows)
    status_counts = Counter(row.get("state", {}).get("status", "missing") for row in rows)
    totals = {
        "root": root_label,
        "files": len(rows),
        "missing_footer": sum(not row.get("state", {}).get("has_footer", False) for row in rows),
        "missing_changelog": sum(not row.get("state", {}).get("has_changelog", False) for row in rows),
        "missing_description": sum(not row.get("state", {}).get("has_description", False) for row in rows),
        "missing_purpose": sum(not row.get("state", {}).get("has_purpose", False) for row in rows),
    }
    return {
        "totals": totals,
        "layers": [{"layer": value, "files": count} for value, count in sorted(layer_counts.items())],
        "owners": [{"owner": value, "files": count} for value, count in sorted(owner_counts.items())],
        "statuses": [{"status": value, "files": count} for value, count in sorted(status_counts.items())],
    }


def materialize_registry_rows(rows: list[dict[str, object]], output_path: Path) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with output_path.open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(row, ensure_ascii=True, separators=(",", ":")) + "\n")


def flatten_for_toon(value: object) -> object:
    if isinstance(value, dict):
        flattened: dict[str, object] = {}
        for key, item in value.items():
            if item in ("", None, [], {}):
                continue
            if isinstance(item, dict):
                for nested_key, nested_value in item.items():
                    if nested_value in ("", None, [], {}):
                        continue
                    flattened[f"{key}.{nested_key}"] = nested_value
                continue
            if isinstance(item, list):
                flattened[key] = ",".join(str(part) for part in item if str(part).strip())
                continue
            flattened[key] = item
        return flattened
    return value


def tabularize_toon_rows(rows: list[dict[str, object]]) -> list[dict[str, object]]:
    flattened_rows = [flatten_for_toon(row) for row in rows]
    keys: list[str] = []
    seen: set[str] = set()
    for row in flattened_rows:
        if not isinstance(row, dict):
            continue
        for key in row.keys():
            if key not in seen:
                seen.add(key)
                keys.append(key)
    normalized: list[dict[str, object]] = []
    for row in flattened_rows:
        if not isinstance(row, dict):
            normalized.append({"value": row})
            continue
        normalized.append({key: row.get(key, "") for key in keys})
    return normalized


def emit_toon_sections(sections: dict[str, object]) -> None:
    rendered: list[str] = []
    for key, value in sections.items():
        if value in ("", None, [], {}):
            continue
        rendered.append(f"{key}:")
        body = toon_format.encode(value).rstrip()
        for line in body.splitlines():
            rendered.append(f"  {line}")
    typer.echo("\n".join(rendered))


def emit_rows(rows: list[dict[str, object]], output_format: str) -> None:
    if output_format == "toon":
        typer.echo(toon_format.encode(tabularize_toon_rows(rows)))
        return
    for row in rows:
        typer.echo(json.dumps(row, ensure_ascii=True, separators=(",", ":")))


def scan_targets(root: Path | None, profile: str) -> list[tuple[Path, Path]]:
    if profile:
        seen: set[Path] = set()
        items: list[tuple[Path, Path]] = []
        for scan_root in roots_for_profile(profile):
            if scan_root.is_file() and scan_root.suffix.lower() == ".md" and not is_scan_ignored(scan_root):
                if scan_root.resolve() not in seen:
                    seen.add(scan_root.resolve())
                    items.append((scan_root.parent, scan_root))
                continue
            if not scan_root.exists():
                continue
            for path in iter_markdown_files(scan_root):
                if path.resolve() not in seen:
                    seen.add(path.resolve())
                    items.append((scan_root, path))
        return items
    if root is None:
        raise typer.BadParameter("either --root or --profile is required")
    scan_root = root.resolve()
    return [(scan_root, path) for path in iter_markdown_files(scan_root)]


def run_quiet_check(scan_root: Path, paths: list[Path]) -> int:
    had_error = False
    for file_path in paths:
        record = build_record(scan_root, file_path)
        problems = validate_record(file_path, record)
        if problems:
            had_error = True
            typer.echo(json.dumps({"path": relative_to_root(file_path), "issues": problems}, ensure_ascii=True, separators=(",", ":")))
    return 1 if had_error else 0


def extract_link_targets(markdown_file: Path, body: str) -> list[dict[str, object]]:
    import re
    link_re = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
    links: list[dict[str, object]] = []
    for target in link_re.findall(body):
        target = target.strip()
        if not target or "://" in target or target.startswith("#"):
            continue
        candidate = (markdown_file.parent / target).resolve() if not target.startswith("/") else Path(target)
        links.append({
            "target": target,
            "resolved": relative_to_root(candidate),
            "exists": candidate.exists(),
        })
    return links


def rewrite_markdown_links(body: str, old_target: str, new_target: str) -> tuple[str, int]:
    import re
    link_re = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
    replacements = 0

    def replace(match: re.Match[str]) -> str:
        nonlocal replacements
        if match.group(1) != old_target:
            return match.group(0)
        replacements += 1
        return match.group(0).replace(f"({old_target})", f"({new_target})")

    return link_re.sub(replace, body), replacements


def resolve_markdown_scope(target: Path) -> list[Path]:
    resolved = target.resolve()
    if resolved.is_file():
        return [resolved]
    return [path for path in iter_markdown_files(resolved)]


def impact_rows_for_reference(value: str) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    for file_path in iter_markdown_files(REPO_ROOT):
        _, body, footer = load_markdown(file_path)
        reasons: list[str] = []
        if value and footer.get("artifact_path", "") == value:
            reasons.append("artifact_path")
        if value and value in body:
            reasons.append("body_reference")
        if value and value in json.dumps(footer):
            reasons.append("footer_reference")
        if reasons:
            rows.append({
                "path": relative_to_root(file_path),
                "artifact": footer.get("artifact_path", ""),
                "reasons": ",".join(sorted(set(reasons))),
            })
    rows.sort(key=lambda row: (row["path"], row["reasons"]))
    return rows


def build_reference_index() -> tuple[dict[str, list[dict[str, object]]], dict[str, list[dict[str, object]]]]:
    artifact_index: dict[str, list[dict[str, object]]] = {}
    path_index: dict[str, list[dict[str, object]]] = {}
    for file_path in iter_markdown_files(REPO_ROOT):
        _, body, footer = load_markdown(file_path)
        rel_path = relative_to_root(file_path)
        artifact = footer.get("artifact_path", "")
        row = {"path": rel_path, "artifact": artifact}
        if artifact:
            artifact_index.setdefault(artifact, []).append({**row, "reasons": "artifact_path,footer_reference"})
        path_index.setdefault(rel_path, []).append({**row, "reasons": "path"})
        for link in extract_link_targets(file_path, body):
            path_index.setdefault(str(link["resolved"]), []).append({**row, "reasons": "markdown_link"})
        for key in ("projection_ref", "contract_ref", "template_ref", "parent_definition_ref", "artifact_path"):
            value = footer.get(key, "")
            if value:
                artifact_index.setdefault(value, []).append({**row, "reasons": f"footer_ref:{key}"})
    return artifact_index, path_index


def infer_reference_value(markdown_file: Path | None, artifact_path: str) -> tuple[str, str]:
    if artifact_path:
        return artifact_path, "artifact"
    if markdown_file is None:
        raise typer.BadParameter("either --file or --artifact is required")
    path = markdown_file.resolve()
    if not path.exists():
        raise typer.BadParameter(f"markdown file not found: {path}")
    _, _, footer = load_markdown(path)
    return footer.get("artifact_path", relative_to_root(path)), "file"


def apply_finalize_updates(footer: dict[str, str], status: str, artifact_version: str, artifact_revision: str, set_values: list[str]) -> tuple[dict[str, str], list[str]]:
    updated = dict(footer)
    applied_updates: list[str] = []
    if status:
        updated["status"] = status
        applied_updates.append(f"status={status}")
    if artifact_version:
        updated["artifact_version"] = artifact_version
        applied_updates.append(f"artifact_version={artifact_version}")
    if artifact_revision:
        updated["artifact_revision"] = artifact_revision
        applied_updates.append(f"artifact_revision={artifact_revision}")
    for item in set_values:
        if "=" not in item:
            raise typer.BadParameter(f"invalid --set pair: {item}")
        key, value = item.split("=", 1)
        updated[key.strip()] = value.strip()
        applied_updates.append(f"{key.strip()}={value.strip()}")
    return updated, applied_updates


def complete_status(code: int) -> None:
    status = "✅ OK" if code == 0 else "❌ ERROR"
    suffix = f" ({result_note})" if result_note else ""
    typer.echo(f"{status}: {current_command}{suffix}", err=True)
    raise typer.Exit(code)


@app.command("scan")
def cmd_scan(
    root: Annotated[Path | None, typer.Option(help="Root directory to scan.")] = None,
    profile: Annotated[str, typer.Option(help="Named scan profile from policy.")] = "",
    missing_only: Annotated[bool, typer.Option(help="Emit only files missing footer metadata.")] = False,
) -> None:
    set_command("scan")
    for scope_root, file_path in scan_targets(root, profile):
        record = build_record(scope_root, file_path)
        if missing_only and record["state"]["has_footer"]:
            continue
        typer.echo(json.dumps(record, ensure_ascii=True, separators=(",", ":")))
    complete_status(0)


@app.command("summary")
def cmd_summary(
    root: Annotated[Path | None, typer.Option(help="Root directory to summarize.")] = None,
    profile: Annotated[str, typer.Option(help="Named scan profile from policy.")] = "",
    output_format: Annotated[str, typer.Option("--format", help="Output format: toon or jsonl.")] = "toon",
) -> None:
    set_command("summary")
    rows = [build_record(scope_root, path) for scope_root, path in scan_targets(root, profile)]
    root_label = profile or str(root.resolve())
    payload = summary_payload(rows, root_label)
    if output_format == "toon":
        emit_toon_sections({
            "context": {"command": "summary", "root": root_label},
            "totals": payload["totals"],
            "layers": payload["layers"],
            "owners": payload["owners"],
            "statuses": payload["statuses"],
        })
    else:
        typer.echo(json.dumps({"summary": "totals", **payload["totals"]}, ensure_ascii=True, separators=(",", ":")))
        for label, key in (("layer", "layer"), ("owner", "owner"), ("status", "status")):
            for row in payload[f"{label}s" if label != "status" else "statuses"]:
                typer.echo(json.dumps({"summary": label, key: row[key], "files": row["files"]}, ensure_ascii=True, separators=(",", ":")))
    complete_status(0)


@app.command("overview")
def cmd_overview(
    profile: Annotated[str, typer.Option(help="Named scan profile from policy.")] = "active-canon",
    show_warnings: Annotated[bool, typer.Option(help="Include doctor warnings in the overview.")] = True,
) -> None:
    set_command("overview")
    rows = [build_record(scope_root, path) for scope_root, path in scan_targets(None, profile)]
    payload = summary_payload(rows, profile)
    issue_rows: list[dict[str, object]] = []
    targets = scan_targets(None, profile)
    for scope_root, file_path in targets:
        record = build_record(scope_root, file_path)
        _, body, footer = load_markdown(file_path)
        problems = validate_record(file_path, record) + validate_footer_consistency(file_path, footer)
        warnings = []
        if is_footer_optional(file_path):
            warnings.append("footer_optional_policy")
        if is_changelog_only(file_path):
            warnings.append("changelog_only_policy")
        if is_mutation_disabled(file_path):
            warnings.append("mutation_disabled_policy")
        if problems:
            issue_rows.append({"severity": "error", "path": relative_to_root(file_path), "issues": ",".join(sorted(set(problems)))})
        elif show_warnings and warnings:
            issue_rows.append({"severity": "warning", "path": relative_to_root(file_path), "issues": ",".join(sorted(set(warnings)))})
    emit_toon_sections({
        "context": {"command": "overview", "profile": profile},
        "totals": payload["totals"],
        "layers": payload["layers"],
        "owners": payload["owners"],
        "statuses": payload["statuses"],
        "issues": issue_rows,
    })
    complete_status(0)


@app.command("registry")
def cmd_registry(
    root: Annotated[Path | None, typer.Option(help="Root directory to scan.")] = None,
    profile: Annotated[str, typer.Option(help="Named scan profile from policy.")] = "",
) -> None:
    set_command("registry")
    rows = registry_rows_for(root, profile)
    for row in rows:
        typer.echo(json.dumps(row, ensure_ascii=True, separators=(",", ":")))
    complete_status(0)


@app.command("registry-write")
def cmd_registry_write(
    root: Annotated[Path | None, typer.Option(help="Root directory to scan.")] = None,
    profile: Annotated[str, typer.Option(help="Named scan profile from policy.")] = "",
    output: Annotated[Path, typer.Option(help="Output JSONL path.")] = REPO_ROOT / "_temp" / "codex-registry.jsonl",
    canonical: Annotated[bool, typer.Option(help="Write to the canonical registry path from schema.")] = False,
) -> None:
    set_command("registry-write")
    rows = registry_rows_for(root, profile)
    final_output = (REPO_ROOT / load_schema().canonical_registry.path).resolve() if canonical else output.resolve()
    materialize_registry_rows(rows, final_output)
    set_result_note(f"wrote registry for {profile or relative_to_root(root.resolve())} to {relative_to_root(final_output)}")
    complete_status(0)


def mutation_event(task_id: str, actor: str, scope: str, tags: str, footer: dict[str, str], event: str, reason: str, path: Path) -> dict[str, object]:
    return {
        "ts": footer.get("updated_at", now_iso()),
        "event": event,
        "artifact_path": footer.get("artifact_path", ""),
        "artifact_type": footer.get("artifact_type", ""),
        "artifact_version": footer.get("artifact_version", ""),
        "artifact_revision": footer.get("artifact_revision", ""),
        "source_path": footer.get("source_path", relative_to_root(path)),
        "reason": reason,
        "task_id": task_id,
        "actor": actor,
        "scope": scope,
        "tags": normalize_tag_list(tags),
    }


@app.command("touch")
def cmd_touch(
    markdown_file: Path,
    change_note: str,
    event: Annotated[str, typer.Option(help="Changelog event name.")] = "artifact_revision_updated",
    task_id: Annotated[str, typer.Option(help="Optional task identifier.")] = "",
    actor: Annotated[str, typer.Option(help="Actor responsible for the change.")] = "manual",
    scope: Annotated[str, typer.Option(help="Optional bounded scope label.")] = "",
    tags: Annotated[str, typer.Option(help="Optional comma-separated tags.")] = "",
) -> None:
    set_command("touch")
    path = markdown_file.resolve()
    if not path.exists():
        raise typer.BadParameter(f"markdown file not found: {path}")
    if is_changelog_only(path):
        append_changelog_event(changelog_path_for(path), bootstrap_optional_event(path, event, change_note, task_id, actor, scope, normalize_tag_list(tags)))
        set_result_note(f"{path.name} changelog updated; {policy_reason(path, 'changelog_only') or 'bootstrap file body left unchanged'}")
        complete_status(0)
    _, body, footer = load_markdown(path)
    if not footer:
        complete_status(2)
    footer["updated_at"] = now_iso()
    write_markdown(path, body, footer)
    append_changelog_event(changelog_path_for(path), mutation_event(task_id, actor, scope, tags, footer, event, change_note, path))
    complete_status(run_quiet_check(path.parent, [path]))


@app.command("finalize-edit")
def cmd_finalize_edit(
    markdown_files: Annotated[list[Path], typer.Argument(help="One or more markdown files to finalize.")] ,
    change_note: str,
    event: Annotated[str, typer.Option(help="Changelog event name.")] = "artifact_revision_updated",
    status: Annotated[str, typer.Option(help="Optional new status value.")] = "",
    artifact_version: Annotated[str, typer.Option(help="Optional new artifact version.")] = "",
    artifact_revision: Annotated[str, typer.Option(help="Optional new artifact revision.")] = "",
    set_values: Annotated[list[str], typer.Option("--set", help="Metadata override in key=value form.")] = [],
    task_id: Annotated[str, typer.Option(help="Optional task identifier.")] = "",
    actor: Annotated[str, typer.Option(help="Actor responsible for the change.")] = "manual",
    scope: Annotated[str, typer.Option(help="Optional bounded scope label.")] = "",
    tags: Annotated[str, typer.Option(help="Optional comma-separated tags.")] = "",
) -> None:
    set_command("finalize-edit")
    resolved_files = [path.resolve() for path in markdown_files]
    if not resolved_files:
        raise typer.BadParameter("at least one markdown file is required")
    changed_paths: list[Path] = []
    changelog_only_count = 0
    skipped_paths: list[str] = []
    for path in resolved_files:
        if not path.exists():
            raise typer.BadParameter(f"markdown file not found: {path}")
        if is_mutation_disabled(path) and not is_changelog_only(path):
            skipped_paths.append(path.name)
            continue
        if is_changelog_only(path):
            append_changelog_event(changelog_path_for(path), bootstrap_optional_event(path, event, change_note, task_id, actor, scope, normalize_tag_list(tags)))
            changelog_only_count += 1
            changed_paths.append(path)
            continue
        _, body, footer = load_markdown(path)
        if not footer:
            complete_status(2)
        footer, applied_updates = apply_finalize_updates(footer, status, artifact_version, artifact_revision, set_values)
        footer["updated_at"] = now_iso()
        write_markdown(path, body, footer)
        event_row = mutation_event(task_id, actor, scope, tags, footer, event, change_note, path)
        if applied_updates:
            event_row["metadata_updates"] = applied_updates
        append_changelog_event(changelog_path_for(path), event_row)
        changed_paths.append(path)
    if skipped_paths:
        set_result_note(f"skipped {len(skipped_paths)} mutation-disabled file(s)")
    if not changed_paths:
        complete_status(0)
    scan_root = REPO_ROOT
    if len(changed_paths) == 1 and changed_paths[0].parent.exists():
        scan_root = changed_paths[0].parent
    note_bits = [f"finalized {len(changed_paths)} file(s)"]
    if changelog_only_count:
        note_bits.append(f"{changelog_only_count} changelog-only")
    if skipped_paths:
        note_bits.append(f"{len(skipped_paths)} skipped")
    set_result_note(", ".join(note_bits))
    complete_status(run_quiet_check(scan_root, changed_paths))


@app.command("init")
def cmd_init(
    markdown_file: Path,
    artifact_path: str,
    artifact_type: str,
    change_note: str,
    title: Annotated[str, typer.Option(help="Optional title line.")] = "",
    purpose: Annotated[str, typer.Option(help="Optional Purpose: line.")] = "",
    artifact_version: Annotated[int, typer.Option(help="Artifact version.")] = 1,
    artifact_revision: Annotated[str, typer.Option(help="Artifact revision.")] = "",
    schema_version: Annotated[int, typer.Option(help="Footer schema version.")] = 1,
    status: Annotated[str, typer.Option(help="Artifact status.")] = "canonical",
    task_id: Annotated[str, typer.Option(help="Optional task identifier.")] = "",
    actor: Annotated[str, typer.Option(help="Actor responsible for the change.")] = "manual",
    scope: Annotated[str, typer.Option(help="Optional bounded scope label.")] = "",
    tags: Annotated[str, typer.Option(help="Optional comma-separated tags.")] = "",
) -> None:
    set_command("init")
    path = markdown_file.resolve()
    if path.exists():
        raise typer.BadParameter(f"markdown file already exists: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    created_at = now_iso()
    footer = {
        "artifact_path": artifact_path,
        "artifact_type": artifact_type,
        "artifact_version": str(artifact_version),
        "artifact_revision": artifact_revision or created_at[:10],
        "schema_version": str(schema_version),
        "status": status,
        "source_path": relative_to_root(path),
        "created_at": created_at,
        "updated_at": created_at,
        "changelog_ref": path.with_suffix("").name + ".changelog.jsonl",
    }
    body = f"# {title or titleize_stem(path.stem)}\n\n" + (f"Purpose: {purpose}\n" if purpose else "Purpose:\n")
    write_markdown(path, body, footer)
    append_changelog_event(changelog_path_for(path), mutation_event(task_id, actor, scope, tags, footer, "artifact_initialized", change_note, path))
    complete_status(run_quiet_check(path.parent, [path]))


@app.command("move")
def cmd_move(markdown_file: Path, destination: Path, change_note: str,
             task_id: Annotated[str, typer.Option(help="Optional task identifier.")] = "",
             actor: Annotated[str, typer.Option(help="Actor responsible for the change.")] = "manual",
             scope: Annotated[str, typer.Option(help="Optional bounded scope label.")] = "",
             tags: Annotated[str, typer.Option(help="Optional comma-separated tags.")] = "") -> None:
    set_command("move")
    src = markdown_file.resolve()
    dst = destination.resolve()
    if not src.exists():
        raise typer.BadParameter(f"markdown file not found: {src}")
    if is_mutation_disabled(src):
        set_result_note(f"{src.name} skipped; {policy_reason(src, 'mutation_disabled') or 'bootstrap file mutation is disabled'}")
        complete_status(0)
    if dst.exists():
        raise typer.BadParameter(f"destination already exists: {dst}")
    _, body, footer = load_markdown(src)
    footer["source_path"] = relative_to_root(dst)
    footer["updated_at"] = now_iso()
    footer["changelog_ref"] = dst.with_suffix("").name + ".changelog.jsonl"
    dst.parent.mkdir(parents=True, exist_ok=True)
    write_markdown(dst, body, footer)
    src_changelog = changelog_path_for(src)
    dst_changelog = changelog_path_for(dst)
    if src_changelog.exists():
        dst_changelog.write_text(src_changelog.read_text(encoding="utf-8"), encoding="utf-8")
        append_changelog_event(dst_changelog, {**mutation_event(task_id, actor, scope, tags, footer, "artifact_moved", change_note, dst), "previous_source_path": relative_to_root(src)})
        src_changelog.unlink()
    src.unlink()
    complete_status(run_quiet_check(dst.parent, [dst]))


@app.command("rename-artifact")
def cmd_rename_artifact(markdown_file: Path, artifact_path: str, change_note: str,
                        artifact_type: Annotated[str, typer.Option(help="Optional new artifact type.")] = "",
                        bump_version: Annotated[bool, typer.Option(help="Increment artifact_version after rename.")] = False,
                        event: Annotated[str, typer.Option(help="Changelog event name.")] = "artifact_path_updated",
                        task_id: Annotated[str, typer.Option(help="Optional task identifier.")] = "",
                        actor: Annotated[str, typer.Option(help="Actor responsible for the change.")] = "manual",
                        scope: Annotated[str, typer.Option(help="Optional bounded scope label.")] = "",
                        tags: Annotated[str, typer.Option(help="Optional comma-separated tags.")] = "") -> None:
    set_command("rename-artifact")
    path = markdown_file.resolve()
    if not path.exists():
        raise typer.BadParameter(f"markdown file not found: {path}")
    if is_mutation_disabled(path):
        set_result_note(f"{path.name} skipped; {policy_reason(path, 'mutation_disabled') or 'bootstrap file mutation is disabled'}")
        complete_status(0)
    _, body, footer = load_markdown(path)
    previous = footer.get("artifact_path", "")
    footer["artifact_path"] = artifact_path
    if artifact_type:
        footer["artifact_type"] = artifact_type
    if bump_version:
        footer["artifact_version"] = str(int(footer.get("artifact_version", "0")) + 1)
    footer["updated_at"] = now_iso()
    write_markdown(path, body, footer)
    row = mutation_event(task_id, actor, scope, tags, footer, event, change_note, path)
    row["previous_artifact_path"] = previous
    append_changelog_event(changelog_path_for(path), row)
    complete_status(run_quiet_check(path.parent, [path]))


@app.command("changelog")
def cmd_changelog(markdown_file: Path,
                  limit: Annotated[int, typer.Option(help="Maximum number of rows to emit.")] = 20,
                  newest_first: Annotated[bool, typer.Option(help="Show newest events first.")] = False,
                  output_format: Annotated[str, typer.Option("--format", help="Output format: toon or jsonl.")] = "toon") -> None:
    set_command("changelog")
    path = markdown_file.resolve()
    if not path.exists():
        raise typer.BadParameter(f"markdown file not found: {path}")
    rows = read_changelog_rows(changelog_path_for(path))
    rows.sort(key=lambda row: parse_ts(row.get("ts", "")))
    if newest_first:
        rows.reverse()
    if limit > 0:
        rows = rows[:limit]
    emit_rows(rows, output_format)
    complete_status(0)


@app.command("changelog-task")
def cmd_changelog_task(root: Annotated[Path | None, typer.Option(help="Root directory to scan.")] = None,
                       profile: Annotated[str, typer.Option(help="Named scan profile from policy.")] = "",
                       task_id: str = "",
                       limit: Annotated[int, typer.Option(help="Maximum number of rows to emit.")] = 0,
                       newest_first: Annotated[bool, typer.Option(help="Show newest events first.")] = False,
                       output_format: Annotated[str, typer.Option("--format", help="Output format: toon or jsonl.")] = "toon") -> None:
    set_command("changelog-task")
    matched: list[dict[str, object]] = []
    for scope_root, markdown_file in scan_targets(root, profile):
        try:
            rows = read_changelog_rows(changelog_path_for(markdown_file))
        except Exception:
            continue
        for row in rows:
            if str(row.get("task_id", "")).strip() == task_id:
                payload = {
                    "path": markdown_file.relative_to(scope_root).as_posix(),
                    "changelog": changelog_path_for(markdown_file).name,
                    "artifact": row.get("artifact_path", ""),
                    "event": row.get("event", ""),
                    "ts": row.get("ts", ""),
                    "task_id": row.get("task_id", ""),
                    "reason": row.get("reason", ""),
                }
                if row.get("actor"):
                    payload["actor"] = row["actor"]
                if row.get("scope"):
                    payload["scope"] = row["scope"]
                if row.get("tags"):
                    payload["tags"] = row["tags"]
                matched.append(payload)
    matched.sort(key=lambda item: parse_ts(item.get("ts", "")))
    if newest_first:
        matched.reverse()
    if limit > 0:
        matched = matched[:limit]
    emit_rows(matched, output_format)
    complete_status(0)


@app.command("task-summary")
def cmd_task_summary(root: Annotated[Path | None, typer.Option(help="Root directory to scan.")] = None,
                     profile: Annotated[str, typer.Option(help="Named scan profile from policy.")] = "",
                     task_id: str = "",
                     output_format: Annotated[str, typer.Option("--format", help="Output format: toon or jsonl.")] = "toon") -> None:
    set_command("task-summary")
    actors: Counter[str] = Counter()
    scopes: Counter[str] = Counter()
    tags: Counter[str] = Counter()
    files: Counter[str] = Counter()
    count = 0
    first_ts = ""
    last_ts = ""
    for scope_root, markdown_file in scan_targets(root, profile):
        try:
            rows = read_changelog_rows(changelog_path_for(markdown_file))
        except Exception:
            continue
        for row in rows:
            if str(row.get("task_id", "")).strip() != task_id:
                continue
            count += 1
            rel = markdown_file.relative_to(scope_root).as_posix()
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
    root_label = profile or str(root.resolve())
    if output_format == "toon":
        emit_toon_sections({
            "context": {"command": "task-summary", "task_id": task_id, "root": root_label},
            "totals": {"events": count, "files": len(files), "first_ts": first_ts, "last_ts": last_ts},
            "files": [{"path": value, "events": event_count} for value, event_count in sorted(files.items())],
            "actors": [{"actor": value, "events": event_count} for value, event_count in sorted(actors.items())],
            "scopes": [{"scope": value, "events": event_count} for value, event_count in sorted(scopes.items())],
            "tags": [{"tag": value, "events": event_count} for value, event_count in sorted(tags.items())],
        })
    else:
        rows = [{"summary": "task", "task_id": task_id, "root": root_label, "events": count, "files": len(files), "first_ts": first_ts, "last_ts": last_ts}]
        for label, counts in (("file", files), ("actor", actors), ("scope", scopes), ("tag", tags)):
            for value, event_count in sorted(counts.items()):
                rows.append({"summary": label, label: value, "events": event_count, "task_id": task_id})
        emit_rows(rows, output_format)
    complete_status(0)


@app.command("deps")
def cmd_deps(markdown_file: Path,
             output_format: Annotated[str, typer.Option("--format", help="Output format: toon or jsonl.")] = "toon") -> None:
    set_command("deps")
    path = markdown_file.resolve()
    _, body, footer = load_markdown(path)
    refs = []
    for key in ("projection_ref", "contract_ref", "template_ref", "parent_definition_ref"):
        if footer.get(key):
            refs.append({"kind": key, "target": footer[key]})
    reverse = []
    for other in iter_markdown_files(REPO_ROOT):
        if other == path:
            continue
        other_text, _, other_footer = load_markdown(other)
        if relative_to_root(path) in other_text or (footer.get("artifact_path") and footer["artifact_path"] in json.dumps(other_footer)):
            reverse.append(relative_to_root(other))
    payload = {"path": relative_to_root(path), "artifact": footer.get("artifact_path", ""), "links": extract_link_targets(path, body), "footer_refs": refs, "referenced_by": [{"path": item} for item in sorted(reverse)]}
    if output_format == "toon":
        emit_toon_sections({
            "context": {"command": "deps", "path": payload["path"], "artifact": payload["artifact"]},
            "links": payload["links"],
            "footer_refs": payload["footer_refs"],
            "referenced_by": payload["referenced_by"],
        })
    else:
        typer.echo(json.dumps(payload, ensure_ascii=True, separators=(",", ":")))
    complete_status(0)


@app.command("deps-map")
def cmd_deps_map(path: Path,
                 output_format: Annotated[str, typer.Option("--format", help="Output format: toon or jsonl.")] = "toon") -> None:
    set_command("deps-map")
    target = path.resolve()
    files = resolve_markdown_scope(target)
    rows: list[dict[str, object]] = []
    for file_path in files:
        _, body, footer = load_markdown(file_path)
        artifact = footer.get("artifact_path", "")
        for link in extract_link_targets(file_path, body):
            rows.append({
                "path": relative_to_root(file_path),
                "artifact": artifact,
                "edge_type": "markdown_link",
                "target": link["target"],
                "resolved": link["resolved"],
                "exists": link["exists"],
            })
        for key in ("projection_ref", "contract_ref", "template_ref", "parent_definition_ref"):
            if footer.get(key):
                rows.append({
                    "path": relative_to_root(file_path),
                    "artifact": artifact,
                    "edge_type": key,
                    "target": footer[key],
                    "resolved": footer[key],
                    "exists": True,
                })
    if output_format == "toon":
        emit_toon_sections({
            "context": {"command": "deps-map", "path": relative_to_root(target)},
            "edges": rows,
        })
    else:
        for row in rows:
            typer.echo(json.dumps(row, ensure_ascii=True, separators=(",", ":")))
    complete_status(0)


@app.command("artifact-impact")
def cmd_artifact_impact(markdown_file: Annotated[Path | None, typer.Option("--file", help="Markdown file whose artifact impact should be traced.")] = None,
                        artifact: Annotated[str, typer.Option(help="Artifact path to trace.")] = "",
                        output_format: Annotated[str, typer.Option("--format", help="Output format: toon or jsonl.")] = "toon") -> None:
    set_command("artifact-impact")
    value, source = infer_reference_value(markdown_file, artifact)
    artifact_index, path_index = build_reference_index()
    rows = artifact_index.get(value, []) + path_index.get(value, [])
    deduped: dict[tuple[str, str], dict[str, object]] = {}
    for row in rows:
        key = (str(row["path"]), str(row["reasons"]))
        deduped[key] = row
    rows = sorted(deduped.values(), key=lambda row: (str(row["path"]), str(row["reasons"])))
    if output_format == "toon":
        emit_toon_sections({
            "context": {"command": "artifact-impact", "source": source, "artifact": value},
            "impacts": rows,
        })
    else:
        for row in rows:
            typer.echo(json.dumps(row, ensure_ascii=True, separators=(",", ":")))
    complete_status(0)


@app.command("task-impact")
def cmd_task_impact(root: Annotated[Path | None, typer.Option(help="Root directory to scan.")] = None,
                    profile: Annotated[str, typer.Option(help="Named scan profile from policy.")] = "",
                    task_id: Annotated[str, typer.Option(help="Task id to trace.")] = "",
                    output_format: Annotated[str, typer.Option("--format", help="Output format: toon or jsonl.")] = "toon") -> None:
    set_command("task-impact")
    touched_artifacts: set[str] = set()
    touched_paths: set[str] = set()
    for scope_root, markdown_file in scan_targets(root, profile):
        try:
            rows = read_changelog_rows(changelog_path_for(markdown_file))
        except Exception:
            continue
        for row in rows:
            if str(row.get("task_id", "")).strip() != task_id:
                continue
            touched_paths.add(relative_to_root(markdown_file))
            if row.get("artifact_path"):
                touched_artifacts.add(str(row["artifact_path"]))
    artifact_index, _ = build_reference_index()
    impact_rows: list[dict[str, object]] = []
    seen: set[tuple[str, str]] = set()
    for artifact_value in sorted(touched_artifacts):
        for row in artifact_index.get(artifact_value, []):
            key = (artifact_value, str(row["path"]))
            if row["path"] in touched_paths or key in seen:
                continue
            seen.add(key)
            impact_rows.append({"source_artifact": artifact_value, **row})
    if output_format == "toon":
        emit_toon_sections({
            "context": {"command": "task-impact", "task_id": task_id, "root": profile or (str(root.resolve()) if root else "")},
            "touched": [{"path": path} for path in sorted(touched_paths)],
            "indirect_impacts": impact_rows,
        })
    else:
        for row in impact_rows:
            typer.echo(json.dumps(row, ensure_ascii=True, separators=(",", ":")))
    complete_status(0)


@app.command("links")
def cmd_links(path: Path,
              output_format: Annotated[str, typer.Option("--format", help="Output format: toon or jsonl.")] = "toon") -> None:
    set_command("links")
    target = path.resolve()
    files = [target] if target.is_file() else list(iter_markdown_files(target))
    rows = []
    for file_path in files:
        if file_path.suffix.lower() != ".md":
            continue
        _, body, footer = load_markdown(file_path)
        for link in extract_link_targets(file_path, body):
            rows.append({
                "path": relative_to_root(file_path),
                "artifact": footer.get("artifact_path", ""),
                "target": link["target"],
                "resolved": link["resolved"],
                "exists": link["exists"],
            })
    if output_format == "toon":
        emit_toon_sections({
            "context": {"command": "links", "path": relative_to_root(target)},
            "links": rows,
        })
    else:
        for row in rows:
            typer.echo(json.dumps(row, ensure_ascii=True, separators=(",", ":")))
    complete_status(0)


@app.command("migrate-links")
def cmd_migrate_links(path: Path, old_target: str, new_target: str, change_note: str,
                      task_id: Annotated[str, typer.Option(help="Optional task identifier.")] = "",
                      actor: Annotated[str, typer.Option(help="Actor responsible for the change.")] = "manual",
                      scope: Annotated[str, typer.Option(help="Optional bounded scope label.")] = "",
                      tags: Annotated[str, typer.Option(help="Optional comma-separated tags.")] = "",
                      dry_run: Annotated[bool, typer.Option(help="Preview replacements without mutating files.")] = False,
                      output_format: Annotated[str, typer.Option("--format", help="Output format for preview/summary: toon or jsonl.")] = "toon") -> None:
    set_command("migrate-links")
    target = path.resolve()
    files = resolve_markdown_scope(target)
    changed: list[Path] = []
    skipped = 0
    preview_rows: list[dict[str, object]] = []
    for file_path in files:
        if file_path.suffix.lower() != ".md":
            continue
        if is_mutation_disabled(file_path):
            skipped += 1
            continue
        _, body, footer = load_markdown(file_path)
        updated_body, replacements = rewrite_markdown_links(body, old_target, new_target)
        if replacements == 0:
            continue
        preview_rows.append({
            "path": relative_to_root(file_path),
            "artifact": footer.get("artifact_path", ""),
            "replacements": replacements,
            "old_target": old_target,
            "new_target": new_target,
        })
        if dry_run:
            continue
        footer["updated_at"] = now_iso()
        write_markdown(file_path, updated_body, footer)
        row = mutation_event(task_id, actor, scope, tags, footer, "links_migrated", change_note, file_path)
        row["old_target"] = old_target
        row["new_target"] = new_target
        row["replacements"] = replacements
        append_changelog_event(changelog_path_for(file_path), row)
        changed.append(file_path)
    if dry_run:
        if output_format == "toon":
            emit_toon_sections({
                "context": {"command": "migrate-links", "mode": "dry-run", "path": relative_to_root(target)},
                "totals": {"files": len(preview_rows), "replacements": sum(int(row["replacements"]) for row in preview_rows), "skipped": skipped},
                "changes": preview_rows,
            })
        else:
            for row in preview_rows:
                typer.echo(json.dumps(row, ensure_ascii=True, separators=(",", ":")))
        set_result_note(f"dry-run matched {len(preview_rows)} file(s), skipped {skipped}")
        complete_status(0)
    if not changed and skipped:
        set_result_note(f"skipped {skipped} bootstrap file(s); no markdown links were mutated")
        complete_status(0)
    if output_format == "toon" and changed:
        emit_toon_sections({
            "context": {"command": "migrate-links", "mode": "apply", "path": relative_to_root(target)},
            "totals": {"files": len(preview_rows), "replacements": sum(int(row["replacements"]) for row in preview_rows), "skipped": skipped},
            "changes": preview_rows,
        })
    elif changed:
        for row in preview_rows:
            typer.echo(json.dumps(row, ensure_ascii=True, separators=(",", ":")))
    set_result_note(f"updated {len(changed)} file(s), replacements={sum(int(row['replacements']) for row in preview_rows)}, skipped={skipped}")
    complete_status(run_quiet_check(target if target.is_dir() else target.parent, changed))


@app.command("check")
def cmd_check(root: Annotated[Path | None, typer.Option(help="Root directory for default scan scope.")] = None,
              profile: Annotated[str, typer.Option(help="Named scan profile from policy.")] = "",
              files: Annotated[list[Path], typer.Argument(help="Optional explicit markdown files to validate.")] = []) -> None:
    set_command("check")
    scoped_paths = [(path.resolve().parent, path.resolve()) for path in files if path.exists()] if files else scan_targets(root, profile)
    had_error = False
    for scope_root, file_path in scoped_paths:
        if file_path.suffix.lower() != ".md":
            continue
        problems = validate_record(file_path, build_record(scope_root, file_path))
        if problems:
            had_error = True
            typer.echo(json.dumps({"path": relative_to_root(file_path), "issues": problems}, ensure_ascii=True, separators=(",", ":")))
    complete_status(1 if had_error else 0)


@app.command("doctor")
def cmd_doctor(root: Annotated[Path | None, typer.Option(help="Root directory to scan.")] = None,
               profile: Annotated[str, typer.Option(help="Named scan profile from policy.")] = "",
               show_warnings: Annotated[bool, typer.Option(help="Also emit policy-warning rows for exception-based files.")] = False,
               output_format: Annotated[str, typer.Option("--format", help="Output format: toon or jsonl for emitted issues.")] = "jsonl",
               fail_on_warnings: Annotated[bool, typer.Option(help="Treat warnings as failures.")] = False) -> None:
    set_command("doctor")
    had_error = False
    warning_count = 0
    error_count = 0
    targets = scan_targets(root, profile)
    artifact_seen: dict[str, str] = {}
    changelog_seen: dict[str, str] = {}
    issue_rows: list[dict[str, object]] = []
    severity_counts: Counter[str] = Counter()
    for scope_root, file_path in targets:
        record = build_record(scope_root, file_path)
        _, body, footer = load_markdown(file_path)
        problems = validate_record(file_path, record) + validate_footer_consistency(file_path, footer)
        artifact = footer.get("artifact_path", "")
        if artifact:
            rel = relative_to_root(file_path)
            owner = artifact_seen.get(artifact)
            if owner and owner != rel:
                problems.append(f"duplicate_artifact:{artifact}")
            else:
                artifact_seen[artifact] = rel
        try:
            changelog_path = changelog_path_for(file_path)
            rel_changelog = relative_to_root(changelog_path)
            owner = changelog_seen.get(rel_changelog)
            if owner and owner != relative_to_root(file_path):
                problems.append(f"duplicate_changelog_ref:{rel_changelog}")
            else:
                changelog_seen[rel_changelog] = relative_to_root(file_path)
            latest = latest_changelog_row(changelog_path)
            if latest and latest.get("source_path") and not is_footer_optional(file_path) and str(latest["source_path"]) != footer.get("source_path", ""):
                problems.append("latest_changelog_source_path_mismatch")
        except Exception:
            pass
        for link in extract_link_targets(file_path, body):
            if not link["exists"]:
                problems.append(f"broken_link:{link['target']}")
        warnings = []
        if is_footer_optional(file_path):
            warnings.append("footer_optional_policy")
        if is_changelog_only(file_path):
            warnings.append("changelog_only_policy")
        if is_mutation_disabled(file_path):
            warnings.append("mutation_disabled_policy")
        if problems:
            had_error = True
            error_count += len(set(problems))
            severity_counts["error"] += 1
            issue_rows.append({"severity": "error", "path": relative_to_root(file_path), "issues": ",".join(sorted(set(problems)))})
        elif show_warnings and warnings:
            warning_count += len(set(warnings))
            severity_counts["warning"] += 1
            issue_rows.append({"severity": "warning", "path": relative_to_root(file_path), "issues": ",".join(sorted(set(warnings)))})
    changelog_paths = set()
    for _, file_path in targets:
        try:
            changelog_paths.add(changelog_path_for(file_path).resolve())
        except Exception:
            pass
    candidate_dirs = {scope_root for scope_root, _ in targets}
    for candidate_dir in candidate_dirs:
        for changelog_file in candidate_dir.rglob("*.changelog.jsonl"):
            if is_scan_ignored(changelog_file):
                continue
            if changelog_file.resolve() not in changelog_paths:
                had_error = True
                error_count += 1
                severity_counts["error"] += 1
                issue_rows.append({"severity": "error", "path": relative_to_root(changelog_file), "issues": "orphan_changelog"})
    if issue_rows:
        if output_format == "toon":
            emit_toon_sections({
                "context": {"command": "doctor", "root": profile or (str(root.resolve()) if root else "")},
                "totals": {"error_rows": severity_counts.get("error", 0), "warning_rows": severity_counts.get("warning", 0)},
                "issues": issue_rows,
            })
        else:
            for row in issue_rows:
                typer.echo(json.dumps(row, ensure_ascii=True, separators=(",", ":")))
    profile_fail = bool(profile and profile_settings(profile).warnings_fail)
    if had_error or (show_warnings and warning_count):
        set_result_note(f"errors={error_count}, warnings={warning_count}")
    complete_status(1 if had_error or ((fail_on_warnings or profile_fail) and warning_count > 0) else 0)


def main() -> int:
    try:
        app(standalone_mode=False)
        return 0
    except typer.Exit as exc:
        return exc.exit_code or 0
    except Exception as exc:
        typer.echo(str(exc), err=True)
        typer.echo(f"❌ ERROR: {current_command}", err=True)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
