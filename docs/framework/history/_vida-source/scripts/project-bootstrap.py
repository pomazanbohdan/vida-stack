#!/usr/bin/env python3
"""Project bootstrap contract and scaffold helper for VIDA."""

from __future__ import annotations

import argparse
import importlib.util
import json
import sys
from pathlib import Path
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
TEMPLATE_DIR = ROOT_DIR / "_vida" / "templates"
OVERLAY_TEMPLATE_PATH = TEMPLATE_DIR / "vida.config.yaml.template"
VIDA_CONFIG_PATH = SCRIPT_DIR / "vida-config.py"
VIDA_CONFIG_SPEC = importlib.util.spec_from_file_location("vida_config_bootstrap", VIDA_CONFIG_PATH)
if VIDA_CONFIG_SPEC is None or VIDA_CONFIG_SPEC.loader is None:
    raise RuntimeError(f"Unable to load VIDA config helper: {VIDA_CONFIG_PATH}")
vida_config = importlib.util.module_from_spec(VIDA_CONFIG_SPEC)
VIDA_CONFIG_SPEC.loader.exec_module(vida_config)


def bool_value(value: Any, default: bool) -> bool:
    if value is None:
        return default
    if isinstance(value, bool):
        return value
    return str(value).strip().lower() in {"1", "true", "yes", "on"}


def build_contract() -> dict[str, Any]:
    cfg = vida_config.load_validated_config()
    bootstrap_cfg = vida_config.dotted_get(cfg, "project_bootstrap", {}) or {}
    project_id = str(vida_config.dotted_get(cfg, "project.id", "unnamed-project"))
    language_policy = {
        "user_communication": str(vida_config.dotted_get(cfg, "language_policy.user_communication", "en")),
        "reasoning": str(vida_config.dotted_get(cfg, "language_policy.reasoning", "en")),
        "documentation": str(vida_config.dotted_get(cfg, "language_policy.documentation", "en")),
        "todo_protocol": str(vida_config.dotted_get(cfg, "language_policy.todo_protocol", "en")),
    }

    return {
        "enabled": bool_value(bootstrap_cfg.get("enabled"), True),
        "project_id": project_id,
        "documentation_language": language_policy["documentation"],
        "language_policy": language_policy,
        "allow_scaffold_missing": bool_value(bootstrap_cfg.get("allow_scaffold_missing"), True),
        "require_launch_confirmation": bool_value(bootstrap_cfg.get("require_launch_confirmation"), True),
        "paths": {
            "overlay_manifest": "vida.config.yaml",
            "docs_root": str(bootstrap_cfg.get("docs_root", "docs")),
            "process_root": str(bootstrap_cfg.get("process_root", "docs/process")),
            "research_root": str(bootstrap_cfg.get("research_root", "docs/research")),
            "readme_doc": str(bootstrap_cfg.get("readme_doc", "docs/README.md")),
            "architecture_doc": str(bootstrap_cfg.get("architecture_doc", "docs/architecture.md")),
            "decisions_doc": str(bootstrap_cfg.get("decisions_doc", "docs/decisions.md")),
            "environments_doc": str(bootstrap_cfg.get("environments_doc", "docs/environments.md")),
            "project_operations_doc": str(bootstrap_cfg.get("project_operations_doc", "docs/process/operations.md")),
            "agent_system_doc": str(bootstrap_cfg.get("agent_system_doc", "docs/process/agent-system.md")),
        },
        "framework_templates": {
            "overlay_manifest": str(OVERLAY_TEMPLATE_PATH.relative_to(ROOT_DIR)),
        },
        "protocol_activation": vida_config.dotted_get(cfg, "protocol_activation", {}) or {},
    }


def render_template(path: Path, substitutions: dict[str, str]) -> str:
    rendered = path.read_text(encoding="utf-8")
    for key, value in substitutions.items():
        rendered = rendered.replace(key, value)
    return rendered


def template_for(kind: str, contract: dict[str, Any]) -> str:
    project_id = contract["project_id"]
    docs_language = contract["documentation_language"]
    if kind == "overlay_manifest":
        return render_template(
            OVERLAY_TEMPLATE_PATH,
            {
                "__PROJECT_ID__": project_id,
                "__DOCS_ROOT__": contract["paths"]["docs_root"],
                "__PROCESS_ROOT__": contract["paths"]["process_root"],
                "__RESEARCH_ROOT__": contract["paths"]["research_root"],
                "__README_DOC__": contract["paths"]["readme_doc"],
                "__ARCHITECTURE_DOC__": contract["paths"]["architecture_doc"],
                "__DECISIONS_DOC__": contract["paths"]["decisions_doc"],
                "__ENVIRONMENTS_DOC__": contract["paths"]["environments_doc"],
                "__PROJECT_OPERATIONS_DOC__": contract["paths"]["project_operations_doc"],
                "__AGENT_SYSTEM_DOC__": contract["paths"]["agent_system_doc"],
                "__USER_COMMUNICATION__": contract["language_policy"]["user_communication"],
                "__REASONING_LANGUAGE__": contract["language_policy"]["reasoning"],
                "__DOCUMENTATION_LANGUAGE__": contract["language_policy"]["documentation"],
                "__TODO_PROTOCOL_LANGUAGE__": contract["language_policy"]["todo_protocol"],
            },
        )
    if kind == "readme_doc":
        return (
            f"# {project_id} Project Docs\n\n"
            "This directory contains current project-level documentation only.\n\n"
            "## Rules\n\n"
            "1. Keep docs aligned with running code.\n"
            "2. Keep framework/runtime policy in `_vida/*`.\n"
            f"3. Project documentation language: `{docs_language}`.\n"
        )
    if kind == "architecture_doc":
        return (
            "# Architecture\n\n"
            f"Current architecture snapshot for `{project_id}`.\n\n"
            "## Modules\n\n"
            "1. Document the active application structure.\n"
            "2. Keep only current canonical flows.\n"
        )
    if kind == "decisions_doc":
        return (
            "# Decisions\n\n"
            "Track only active project decisions here.\n\n"
            "## Current Decisions\n\n"
            "1. Add new decisions with rationale and impact.\n"
        )
    if kind == "environments_doc":
        return (
            "# Environments\n\n"
            "Document project environments, access notes, and live validation prerequisites here.\n"
        )
    if kind == "project_operations_doc":
        return (
            "# Project Operations Runbook\n\n"
            f"Canonical project command map for `{project_id}`.\n\n"
            "1. Put app-specific executable flows in `scripts/`.\n"
            "2. Keep framework protocol commands in `_vida/scripts/`.\n"
        )
    if kind == "agent_system_doc":
        return (
            "# Agent System\n\n"
            f"Project subagent/runtime runbook for `{project_id}`.\n\n"
            "1. Concrete cli subagent and model choices live here.\n"
            "2. Framework routing logic stays in `_vida/*`.\n"
        )
    raise KeyError(f"Unknown bootstrap template kind: {kind}")


def file_report(path_str: str) -> dict[str, Any]:
    path = ROOT_DIR / path_str
    return {
        "path": path_str,
        "exists": path.exists(),
        "is_dir": path.is_dir(),
    }


def audit(contract: dict[str, Any]) -> dict[str, Any]:
    results = {
        "enabled": contract["enabled"],
        "project_id": contract["project_id"],
        "documentation_language": contract["documentation_language"],
        "required_files": {},
    }
    missing = 0
    for key, path_str in contract["paths"].items():
        if key.endswith("_root"):
            continue
        report = file_report(path_str)
        results["required_files"][key] = report
        if not report["exists"]:
            missing += 1
    results["missing_count"] = missing
    results["status"] = "ready" if missing == 0 else "missing_artifacts"
    return results


def scaffold(contract: dict[str, Any], force: bool) -> dict[str, Any]:
    created: list[str] = []
    skipped: list[str] = []
    for key, path_str in contract["paths"].items():
        path = ROOT_DIR / path_str
        if key.endswith("_root"):
            path.mkdir(parents=True, exist_ok=True)
            continue
        path.parent.mkdir(parents=True, exist_ok=True)
        if path.exists() and not force:
            skipped.append(path_str)
            continue
        path.write_text(template_for(key, contract), encoding="utf-8")
        created.append(path_str)
    return {
        "created": created,
        "skipped": skipped,
        "status": "ok",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="VIDA project bootstrap contract/audit/scaffold")
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_audit = sub.add_parser("audit", help="Audit required project bootstrap artifacts")
    p_audit.add_argument("--json", action="store_true")

    p_emit = sub.add_parser("emit-contract", help="Emit the resolved bootstrap contract")
    p_emit.add_argument("--json", action="store_true")

    p_scaffold = sub.add_parser("scaffold", help="Create missing bootstrap artifacts")
    p_scaffold.add_argument("--force", action="store_true")
    p_scaffold.add_argument("--json", action="store_true")

    args = parser.parse_args()
    try:
        contract = build_contract()
    except (ValueError, vida_config.OverlayValidationError) as exc:
        print(f"[project-bootstrap] {exc}", file=sys.stderr)
        return 2

    if args.cmd == "emit-contract":
      payload = contract
    elif args.cmd == "audit":
      payload = audit(contract)
    else:
      payload = scaffold(contract, args.force)

    if getattr(args, "json", False):
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    else:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
