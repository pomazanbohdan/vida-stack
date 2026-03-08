#!/usr/bin/env python3
"""Typed capability registry and route compatibility checks for VIDA."""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
STATE_DIR = ROOT_DIR / ".vida" / "state"
REGISTRY_PATH = STATE_DIR / "capability-registry.json"


def load_module(name: str, path: Path) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to load module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


vida_config = load_module("vida_capability_registry_config", SCRIPT_DIR / "vida-config.py")


TASK_CLASS_REQUIREMENTS: dict[str, dict[str, Any]] = {
    "analysis": {"allowed_write_scopes": {"none"}, "required_capability_any": {"read_only", "review_safe"}, "required_artifacts": ["analysis_receipt"], "forbidden_capabilities": set()},
    "coach": {"allowed_write_scopes": {"none"}, "required_capability_any": {"review_safe"}, "required_artifacts": ["coach_review"], "forbidden_capabilities": set()},
    "verification": {"allowed_write_scopes": {"none"}, "required_capability_any": {"review_safe"}, "required_artifacts": ["verification_manifest"], "forbidden_capabilities": set()},
    "verification_ensemble": {"allowed_write_scopes": {"none"}, "required_capability_any": {"review_safe"}, "required_artifacts": ["verification_manifest"], "forbidden_capabilities": set()},
    "review_ensemble": {"allowed_write_scopes": {"none"}, "required_capability_any": {"review_safe"}, "required_artifacts": ["verification_manifest"], "forbidden_capabilities": set()},
    "problem_party": {"allowed_write_scopes": {"none"}, "required_capability_any": {"read_only", "review_safe"}, "required_artifacts": ["problem_party_receipt"], "forbidden_capabilities": {"bounded_write_safe"}},
    "read_only_prep": {"allowed_write_scopes": {"none"}, "required_capability_any": {"read_only"}, "required_artifacts": ["prep_manifest"], "forbidden_capabilities": set()},
    "implementation": {"allowed_write_scopes": {"scoped_only", "orchestrator_native"}, "required_capability_any": {"bounded_write_safe"}, "required_artifacts": ["writer_output"], "forbidden_capabilities": set()},
}


def serialize_requirement(requirement: dict[str, Any]) -> dict[str, Any]:
    return {
        "allowed_write_scopes": sorted(requirement.get("allowed_write_scopes", set())),
        "required_capability_any": sorted(requirement.get("required_capability_any", set())),
        "required_artifacts": list(requirement.get("required_artifacts", [])),
        "forbidden_capabilities": sorted(requirement.get("forbidden_capabilities", set())),
    }


def save_json(path: Path, payload: Any) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def split_csv(value: Any) -> list[str]:
    return vida_config._coerce_repeated_strings(value)  # type: ignore[attr-defined]


def capability_entry(name: str, payload: dict[str, Any]) -> dict[str, Any]:
    return {
        "subagent": name,
        "backend_class": str(payload.get("subagent_backend_class", "")).strip(),
        "role": str(payload.get("role", "")).strip(),
        "write_scope": str(payload.get("write_scope", "none")).strip(),
        "capability_band": split_csv(payload.get("capability_band")),
        "specialties": split_csv(payload.get("specialties")),
        "billing_tier": str(payload.get("billing_tier", "")).strip(),
        "speed_tier": str(payload.get("speed_tier", "")).strip(),
        "quality_tier": str(payload.get("quality_tier", "")).strip(),
        "web_search_wired": bool(vida_config.subagent_has_web_search_wiring(payload)),
    }


def build_registry() -> dict[str, Any]:
    config = vida_config.load_validated_config()
    subagents_cfg = vida_config.dotted_get(config, "agent_system.subagents", {}) or {}
    return {
        "generated_at": "runtime",
        "subagents": {name: capability_entry(name, payload) for name, payload in subagents_cfg.items() if isinstance(payload, dict)},
        "task_class_requirements": {name: serialize_requirement(payload) for name, payload in TASK_CLASS_REQUIREMENTS.items()},
    }


def requirement_for(task_class: str) -> dict[str, Any]:
    return TASK_CLASS_REQUIREMENTS.get(task_class, {"allowed_write_scopes": {"none"}, "required_capability_any": {"read_only"}, "required_artifacts": [], "forbidden_capabilities": set()})


def compatibility_for(task_class: str, subagent_name: str, registry: dict[str, Any] | None = None) -> dict[str, Any]:
    registry = registry or build_registry()
    subagent = (registry.get("subagents") or {}).get(subagent_name)
    if not isinstance(subagent, dict):
        return {"compatible": False, "reason": "unknown_subagent", "task_class": task_class, "subagent": subagent_name}
    req = requirement_for(task_class)
    capability_band = {str(item).casefold() for item in subagent.get("capability_band", [])}
    write_scope = str(subagent.get("write_scope", "none")).strip()
    reasons: list[str] = []
    if write_scope not in set(req["allowed_write_scopes"]):
        reasons.append("write_scope_mismatch")
    required_any = {str(item).casefold() for item in req["required_capability_any"]}
    if required_any and not (capability_band & required_any):
        reasons.append("missing_required_capability_band")
    forbidden = {str(item).casefold() for item in req["forbidden_capabilities"]}
    if capability_band & forbidden:
        reasons.append("forbidden_capability_present")
    return {
        "compatible": not reasons,
        "reason": "ok" if not reasons else ",".join(reasons),
        "task_class": task_class,
        "subagent": subagent_name,
        "required_artifacts": list(req["required_artifacts"]),
        "allowed_write_scopes": sorted(req["allowed_write_scopes"]),
        "required_capability_any": sorted(required_any),
    }


def usage() -> int:
    print("Usage:\n  python3 _vida/scripts/capability-registry.py build\n  python3 _vida/scripts/capability-registry.py check <task_class> <subagent>", file=sys.stderr)
    return 2


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        return usage()
    if argv[1] == "build":
        print(str(save_json(REGISTRY_PATH, build_registry())))
        return 0
    if argv[1] == "check" and len(argv) == 4:
        print(json.dumps(compatibility_for(argv[2], argv[3]), indent=2, sort_keys=True))
        return 0
    return usage()


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
