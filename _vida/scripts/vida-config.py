#!/usr/bin/env python3
"""Portable project overlay reader for VIDA."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any


ROOT_DIR = Path(__file__).resolve().parents[2]
CONFIG_PATH = ROOT_DIR / "vida.config.yaml"
TOP_LEVEL_REQUIRED = {"project", "protocol_activation"}
TOP_LEVEL_OPTIONAL = {"language_policy", "pack_router_keywords", "project_bootstrap", "agent_system", "framework_self_diagnosis"}
PROJECT_KEYS = {"id", "overlay_version"}
PROTOCOL_ACTIVATION_KEYS = {"agent_system"}
LANGUAGE_POLICY_KEYS = {"user_communication", "reasoning", "documentation", "todo_protocol"}
PACK_ROUTER_KEYS = {"research", "spec", "pool", "pool_strong", "pool_dependency", "dev", "bug", "reflect", "reflect_strong"}
PROJECT_BOOTSTRAP_KEYS = {
    "enabled",
    "docs_root",
    "process_root",
    "research_root",
    "readme_doc",
    "architecture_doc",
    "decisions_doc",
    "environments_doc",
    "project_operations_doc",
    "agent_system_doc",
    "allow_scaffold_missing",
    "require_launch_confirmation",
}
FRAMEWORK_SELF_DIAGNOSIS_KEYS = {
    "enabled",
    "silent_mode",
    "auto_capture_bugs",
    "parent_issue",
    "defer_fix_until_task_boundary",
    "session_reflection_required",
    "platform_direction",
    "quality_token_efficiency",
    "session_reflection_criteria",
}
AGENT_SYSTEM_KEYS = {"init_on_boot", "mode", "state_owner", "max_parallel_agents", "subagents", "routing", "scoring"}
AGENT_SYSTEM_MODES = {"native", "hybrid", "disabled"}
SUBAGENT_KEYS = {
    "enabled",
    "subagent_backend_class",
    "detect_command",
    "role",
    "orchestration_tier",
    "cost_priority",
    "max_runtime_seconds",
    "min_output_bytes",
    "models_hint",
    "default_model",
    "profiles",
    "default_profile",
    "capability_band",
    "write_scope",
    "billing_tier",
    "budget_cost_units",
    "speed_tier",
    "quality_tier",
    "specialties",
    "dispatch",
}
SUBAGENT_CLASSES = {"internal", "external_cli", "external_review"}
DISPATCH_KEYS = {
    "command",
    "pre_static_args",
    "subcommand",
    "static_args",
    "write_static_args",
    "workdir_flag",
    "model_flag",
    "output_mode",
    "output_flag",
    "prompt_mode",
    "prompt_flag",
    "web_search_mode",
    "web_search_flag",
    "env",
    "probe_static_args",
    "probe_prompt",
    "probe_expect_substring",
    "probe_timeout_seconds",
    "startup_timeout_seconds",
    "no_output_timeout_seconds",
    "progress_idle_timeout_seconds",
    "max_runtime_extension_seconds",
}
DISPATCH_OUTPUT_MODES = {"stdout", "file"}
DISPATCH_PROMPT_MODES = {"positional", "flag"}
DISPATCH_WEB_SEARCH_MODES = {"none", "flag", "provider_configured"}
YES_NO_VALUES = {"yes", "no"}
ROUTING_DISPATCH_REQUIRED_VALUES = {
    "bridge_or_critical_review",
    "bridge_write_then_internal_if_expands",
    "external_first_review",
    "external_first_then_senior_arbitration",
    "external_first_when_eligible",
    "external_readonly_then_senior_writer",
    "fanout_then_synthesize",
    "local_or_external_first",
}
ROUTING_KEYS = {
    "subagents",
    "models",
    "profiles",
    "analysis_required",
    "analysis_route_task_class",
    "analysis_fanout_subagents",
    "analysis_fanout_min_results",
    "analysis_merge_policy",
    "analysis_external_first_required",
    "analysis_receipt_required",
    "analysis_zero_budget_required",
    "analysis_default_in_boot",
    "coach_required",
    "coach_route_task_class",
    "write_scope",
    "verification_gate",
    "max_runtime_seconds",
    "min_output_bytes",
    "fanout_subagents",
    "fanout_min_results",
    "merge_policy",
    "dispatch_required",
    "external_first_required",
    "web_search_required",
    "bridge_fallback_subagent",
    "internal_escalation_trigger",
    "verification_route_task_class",
    "independent_verification_required",
    "graph_strategy",
    "deterministic_first",
    "budget_policy",
    "max_budget_units",
    "max_cli_subagent_calls",
    "max_coach_passes",
    "max_verification_passes",
    "max_fallback_hops",
    "max_total_runtime_seconds",
    "local_execution_allowed",
    "local_execution_preferred",
    "cli_dispatch_required_if_delegating",
    "direct_internal_bypass_forbidden",
    "allowed_internal_reasons",
}
SCORING_KEYS = {
    "consecutive_failure_limit",
    "promotion_score",
    "demotion_score",
    "probation_success_runs",
    "probation_task_runs",
    "retirement_failure_limit",
}


class OverlayValidationError(ValueError):
    def __init__(self, errors: list[str]) -> None:
        self.errors = errors
        super().__init__(format_validation_errors(errors))


def _strip_comment(line: str) -> str:
    in_single = False
    in_double = False
    out: list[str] = []
    for ch in line:
        if ch == "'" and not in_double:
            in_single = not in_single
        elif ch == '"' and not in_single:
            in_double = not in_double
        if ch == "#" and not in_single and not in_double:
            break
        out.append(ch)
    return "".join(out).rstrip()


def _parse_scalar(raw: str) -> Any:
    raw = raw.strip()
    if raw == "":
        return ""
    if (raw.startswith('"') and raw.endswith('"')) or (raw.startswith("'") and raw.endswith("'")):
        return raw[1:-1]
    lowered = raw.lower()
    if lowered == "true":
        return True
    if lowered == "false":
        return False
    if lowered == "null":
        return None
    try:
        if "." in raw:
            return float(raw)
        return int(raw)
    except ValueError:
        return raw


def _tokenize_yaml_subset(text: str) -> list[tuple[int, int, str]]:
    tokens: list[tuple[int, int, str]] = []
    for lineno, raw_line in enumerate(text.splitlines(), start=1):
        line = _strip_comment(raw_line)
        if not line.strip():
            continue
        indent = len(line) - len(line.lstrip(" "))
        if indent % 2 != 0:
            raise ValueError(f"Indentation must use multiples of 2 spaces (line {lineno})")
        tokens.append((lineno, indent, line.strip()))
    return tokens


def _parse_block(tokens: list[tuple[int, int, str]], index: int, indent: int) -> tuple[Any, int]:
    if index >= len(tokens):
        return {}, index

    _, current_indent, content = tokens[index]
    if current_indent != indent:
        raise ValueError(f"Invalid indentation structure (line {tokens[index][0]})")

    if content.startswith("- "):
        items: list[Any] = []
        while index < len(tokens):
            lineno, current_indent, content = tokens[index]
            if current_indent < indent:
                break
            if current_indent != indent:
                raise ValueError(f"Invalid list indentation structure (line {lineno})")
            if not content.startswith("- "):
                raise ValueError(f"Mixed list/dict block is not supported (line {lineno})")

            item_text = content[2:].strip()
            index += 1
            if item_text == "":
                if index >= len(tokens) or tokens[index][1] <= indent:
                    raise ValueError(f"List item requires nested value (line {lineno})")
                child, index = _parse_block(tokens, index, indent + 2)
                items.append(child)
                continue

            if ":" in item_text:
                key, value = item_text.split(":", 1)
                key = key.strip()
                value = value.strip()
                if value == "":
                    if index < len(tokens) and tokens[index][1] > indent:
                        child, index = _parse_block(tokens, index, indent + 2)
                        items.append({key: child})
                    else:
                        items.append({key: {}})
                else:
                    items.append({key: _parse_scalar(value)})
                continue

            items.append(_parse_scalar(item_text))
        return items, index

    mapping: dict[str, Any] = {}
    while index < len(tokens):
        lineno, current_indent, content = tokens[index]
        if current_indent < indent:
            break
        if current_indent != indent:
            raise ValueError(f"Invalid indentation structure (line {lineno})")
        if content.startswith("- "):
            raise ValueError(f"Unexpected list item outside list block (line {lineno})")
        if ":" not in content:
            raise ValueError(f"Expected key:value pair (line {lineno})")
        key, value = content.split(":", 1)
        key = key.strip()
        value = value.strip()
        index += 1
        if value == "":
            if index < len(tokens) and tokens[index][1] > indent:
                child, index = _parse_block(tokens, index, indent + 2)
                mapping[key] = child
            else:
                mapping[key] = {}
        else:
            mapping[key] = _parse_scalar(value)
    return mapping, index


def parse_yaml_subset(text: str) -> dict[str, Any]:
    tokens = _tokenize_yaml_subset(text)
    if not tokens:
        return {}
    payload, index = _parse_block(tokens, 0, 0)
    if index != len(tokens):
        raise ValueError(f"Unexpected trailing content (line {tokens[index][0]})")
    if not isinstance(payload, dict):
        raise ValueError("vida.config.yaml root must be a mapping")
    return payload


def _is_mapping(value: Any) -> bool:
    return isinstance(value, dict)


def _is_non_bool_int(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _validate_allowed_keys(payload: dict[str, Any], allowed: set[str], path: str, errors: list[str]) -> None:
    for key in sorted(payload.keys()):
        if key not in allowed:
            errors.append(f"{path}.{key}: unsupported key")


def _require_mapping(payload: dict[str, Any], key: str, path: str, errors: list[str], required: bool = False) -> dict[str, Any] | None:
    if key not in payload:
        if required:
            errors.append(f"{path}.{key}: missing required mapping")
        return None
    value = payload.get(key)
    if not _is_mapping(value):
        errors.append(f"{path}.{key}: expected mapping")
        return None
    return value


def _validate_string_field(
    payload: dict[str, Any],
    key: str,
    path: str,
    errors: list[str],
    *,
    required: bool = False,
    allow_empty: bool = False,
) -> str | None:
    if key not in payload:
        if required:
            errors.append(f"{path}.{key}: missing required string")
        return None
    value = payload.get(key)
    if not isinstance(value, str):
        errors.append(f"{path}.{key}: expected string")
        return None
    if not allow_empty and not value.strip():
        errors.append(f"{path}.{key}: must not be empty")
        return None
    return value


def _validate_bool_field(payload: dict[str, Any], key: str, path: str, errors: list[str], *, required: bool = False) -> bool | None:
    if key not in payload:
        if required:
            errors.append(f"{path}.{key}: missing required boolean")
        return None
    value = payload.get(key)
    if not isinstance(value, bool):
        errors.append(f"{path}.{key}: expected boolean")
        return None
    return value


def _validate_int_field(
    payload: dict[str, Any],
    key: str,
    path: str,
    errors: list[str],
    *,
    required: bool = False,
    min_value: int | None = None,
) -> int | None:
    if key not in payload:
        if required:
            errors.append(f"{path}.{key}: missing required integer")
        return None
    value = payload.get(key)
    if not _is_non_bool_int(value):
        errors.append(f"{path}.{key}: expected integer")
        return None
    if min_value is not None and value < min_value:
        errors.append(f"{path}.{key}: must be >= {min_value}")
        return None
    return value


def _validate_enum_field(
    payload: dict[str, Any],
    key: str,
    path: str,
    errors: list[str],
    *,
    allowed: set[str],
    required: bool = False,
) -> str | None:
    value = _validate_string_field(payload, key, path, errors, required=required)
    if value is None:
        return None
    if value not in allowed:
        errors.append(f"{path}.{key}: expected one of {sorted(allowed)}")
        return None
    return value


def _validate_repeated_string_field(payload: dict[str, Any], key: str, path: str, errors: list[str]) -> list[str] | None:
    if key not in payload:
        return None
    value = payload.get(key)
    if isinstance(value, str):
        if not value.strip():
            errors.append(f"{path}.{key}: CSV string must not be empty")
            return None
        return [item.strip() for item in value.split(",") if item.strip()]
    if isinstance(value, list):
        out: list[str] = []
        for index, item in enumerate(value):
            if not isinstance(item, str):
                errors.append(f"{path}.{key}[{index}]: expected string")
                continue
            if not item.strip():
                errors.append(f"{path}.{key}[{index}]: must not be empty")
                continue
            out.append(item.strip())
        return out
    errors.append(f"{path}.{key}: expected CSV string or YAML list of strings")
    return None


def _validate_string_map_field(payload: dict[str, Any], key: str, path: str, errors: list[str]) -> dict[str, str] | None:
    if key not in payload:
        return None
    value = payload.get(key)
    if not _is_mapping(value):
        errors.append(f"{path}.{key}: expected mapping")
        return None
    out: dict[str, str] = {}
    for child_key, child_value in value.items():
        if not isinstance(child_key, str) or not child_key.strip():
            errors.append(f"{path}.{key}: keys must be non-empty strings")
            continue
        if not isinstance(child_value, str) or not child_value.strip():
            errors.append(f"{path}.{key}.{child_key}: expected non-empty string")
            continue
        out[child_key] = child_value.strip()
    return out


def _normalized_string(value: Any) -> str | None:
    if not isinstance(value, str):
        return None
    stripped = value.strip()
    return stripped or None


def _coerce_repeated_strings(value: Any) -> list[str]:
    if isinstance(value, str):
        return [item.strip() for item in value.split(",") if item.strip()]
    if isinstance(value, list):
        out: list[str] = []
        for item in value:
            if isinstance(item, str) and item.strip():
                out.append(item.strip())
        return out
    return []


def subagent_declares_web_search_capability(subagent_cfg: dict[str, Any]) -> bool:
    capability_band = {entry.casefold() for entry in _coerce_repeated_strings(subagent_cfg.get("capability_band"))}
    return "web_search" in capability_band


def subagent_dispatch_web_search_mode(subagent_cfg: dict[str, Any]) -> str:
    dispatch_cfg = subagent_cfg.get("dispatch", {})
    if not isinstance(dispatch_cfg, dict):
        return "none"
    return (_normalized_string(dispatch_cfg.get("web_search_mode")) or "none").casefold()


def subagent_has_web_search_wiring(subagent_cfg: dict[str, Any]) -> bool:
    if not subagent_declares_web_search_capability(subagent_cfg):
        return False
    dispatch_cfg = subagent_cfg.get("dispatch", {})
    if not isinstance(dispatch_cfg, dict):
        return False
    mode = subagent_dispatch_web_search_mode(subagent_cfg)
    if mode == "provider_configured":
        return True
    if mode == "flag":
        return bool(_normalized_string(dispatch_cfg.get("web_search_flag")))
    return False


def _validate_project(project_cfg: dict[str, Any], errors: list[str]) -> None:
    path = "project"
    _validate_allowed_keys(project_cfg, PROJECT_KEYS, path, errors)
    _validate_string_field(project_cfg, "id", path, errors, required=True)
    if "overlay_version" in project_cfg:
        _validate_int_field(project_cfg, "overlay_version", path, errors, min_value=1)


def _validate_protocol_activation(protocol_cfg: dict[str, Any], errors: list[str]) -> bool:
    path = "protocol_activation"
    _validate_allowed_keys(protocol_cfg, PROTOCOL_ACTIVATION_KEYS, path, errors)
    active = _validate_bool_field(protocol_cfg, "agent_system", path, errors, required=True)
    return bool(active)


def _validate_flat_string_section(payload: dict[str, Any], allowed: set[str], path: str, errors: list[str]) -> None:
    _validate_allowed_keys(payload, allowed, path, errors)
    for key in sorted(payload.keys()):
        _validate_string_field(payload, key, path, errors, required=True)


def _validate_project_bootstrap(payload: dict[str, Any], errors: list[str]) -> None:
    path = "project_bootstrap"
    _validate_allowed_keys(payload, PROJECT_BOOTSTRAP_KEYS, path, errors)
    for key in {"enabled", "allow_scaffold_missing", "require_launch_confirmation"}:
        if key in payload:
            _validate_bool_field(payload, key, path, errors)
    for key in PROJECT_BOOTSTRAP_KEYS - {"enabled", "allow_scaffold_missing", "require_launch_confirmation"}:
        if key in payload:
            _validate_string_field(payload, key, path, errors)


def _validate_framework_self_diagnosis(payload: dict[str, Any], errors: list[str]) -> None:
    path = "framework_self_diagnosis"
    _validate_allowed_keys(payload, FRAMEWORK_SELF_DIAGNOSIS_KEYS, path, errors)
    enabled = None
    silent_mode = None
    for key in {"enabled", "silent_mode", "auto_capture_bugs", "defer_fix_until_task_boundary", "session_reflection_required"}:
        if key in payload:
            value = _validate_bool_field(payload, key, path, errors)
            if key == "enabled":
                enabled = value
            elif key == "silent_mode":
                silent_mode = value
    for key in {"parent_issue", "platform_direction", "quality_token_efficiency"}:
        if key in payload:
            _validate_string_field(payload, key, path, errors)
    if "quality_token_efficiency" in payload:
        allowed = {"equal_weight", "quality_first", "tokens_first"}
        value = str(payload.get("quality_token_efficiency", "")).strip()
        if value not in allowed:
            errors.append(f"{path}.quality_token_efficiency: expected one of {sorted(allowed)}")
    if "session_reflection_criteria" in payload:
        _validate_repeated_string_field(payload, "session_reflection_criteria", path, errors)
    if enabled is True and silent_mode is False:
        errors.append(f"{path}.silent_mode: must be true when enabled=true")


def _validate_dispatch(subagent_name: str, dispatch_cfg: dict[str, Any], errors: list[str]) -> None:
    path = f"agent_system.subagents.{subagent_name}.dispatch"
    _validate_allowed_keys(dispatch_cfg, DISPATCH_KEYS, path, errors)
    _validate_string_field(dispatch_cfg, "command", path, errors, required=True)
    if "pre_static_args" in dispatch_cfg:
        _validate_repeated_string_field(dispatch_cfg, "pre_static_args", path, errors)
    if "subcommand" in dispatch_cfg:
        _validate_string_field(dispatch_cfg, "subcommand", path, errors)
    if "static_args" in dispatch_cfg:
        _validate_repeated_string_field(dispatch_cfg, "static_args", path, errors)
    output_mode = None
    if "output_mode" in dispatch_cfg:
        output_mode = _validate_enum_field(dispatch_cfg, "output_mode", path, errors, allowed=DISPATCH_OUTPUT_MODES, required=True)
    if output_mode == "file":
        _validate_string_field(dispatch_cfg, "output_flag", path, errors, required=True)
    elif "output_flag" in dispatch_cfg:
        _validate_string_field(dispatch_cfg, "output_flag", path, errors)
    prompt_mode = None
    if "prompt_mode" in dispatch_cfg:
        prompt_mode = _validate_enum_field(dispatch_cfg, "prompt_mode", path, errors, allowed=DISPATCH_PROMPT_MODES, required=True)
    if prompt_mode == "flag":
        _validate_string_field(dispatch_cfg, "prompt_flag", path, errors, required=True)
    elif "prompt_flag" in dispatch_cfg:
        _validate_string_field(dispatch_cfg, "prompt_flag", path, errors)
    if "env" in dispatch_cfg:
        _validate_string_map_field(dispatch_cfg, "env", path, errors)
    for key in {"workdir_flag", "model_flag"}:
        if key in dispatch_cfg:
            _validate_string_field(dispatch_cfg, key, path, errors)
    web_search_mode = None
    if "web_search_mode" in dispatch_cfg:
        web_search_mode = _validate_enum_field(
            dispatch_cfg,
            "web_search_mode",
            path,
            errors,
            allowed=DISPATCH_WEB_SEARCH_MODES,
            required=True,
        )
    if web_search_mode == "flag":
        _validate_string_field(dispatch_cfg, "web_search_flag", path, errors, required=True)
    elif "web_search_flag" in dispatch_cfg:
        _validate_string_field(dispatch_cfg, "web_search_flag", path, errors)
    for key in {
        "probe_timeout_seconds",
        "startup_timeout_seconds",
        "no_output_timeout_seconds",
        "progress_idle_timeout_seconds",
        "max_runtime_extension_seconds",
    }:
        if key in dispatch_cfg:
            _validate_int_field(dispatch_cfg, key, path, errors, min_value=1)
    for key in {"probe_prompt", "probe_expect_substring"}:
        if key in dispatch_cfg:
            _validate_string_field(dispatch_cfg, key, path, errors)
    if "probe_static_args" in dispatch_cfg:
        _validate_repeated_string_field(dispatch_cfg, "probe_static_args", path, errors)


def _validate_subagent(subagent_name: str, subagent_cfg: dict[str, Any], errors: list[str]) -> None:
    path = f"agent_system.subagents.{subagent_name}"
    _validate_allowed_keys(subagent_cfg, SUBAGENT_KEYS, path, errors)
    enabled = _validate_bool_field(subagent_cfg, "enabled", path, errors)
    subagent_backend_class = _validate_enum_field(subagent_cfg, "subagent_backend_class", path, errors, allowed=SUBAGENT_CLASSES, required=True)
    for key in {
        "detect_command",
        "role",
        "orchestration_tier",
        "cost_priority",
        "default_model",
        "default_profile",
        "write_scope",
        "billing_tier",
        "speed_tier",
        "quality_tier",
    }:
        if key in subagent_cfg:
            _validate_string_field(subagent_cfg, key, path, errors)
    for key in {"max_runtime_seconds", "min_output_bytes", "budget_cost_units"}:
        if key in subagent_cfg:
            _validate_int_field(subagent_cfg, key, path, errors, min_value=0)
    for key in {"models_hint", "profiles", "capability_band", "specialties"}:
        if key in subagent_cfg:
            _validate_repeated_string_field(subagent_cfg, key, path, errors)
    declares_web_search = subagent_declares_web_search_capability(subagent_cfg)
    profiles = _validate_repeated_string_field(subagent_cfg, "profiles", path, errors) if "profiles" in subagent_cfg else None
    default_profile = _validate_string_field(subagent_cfg, "default_profile", path, errors) if "default_profile" in subagent_cfg else None
    if profiles is not None and default_profile and default_profile not in profiles:
        errors.append(f"{path}.default_profile: must be present in profiles")
    dispatch_cfg = _require_mapping(subagent_cfg, "dispatch", path, errors, required=bool(enabled) and subagent_backend_class == "external_cli")
    if dispatch_cfg is not None:
        _validate_dispatch(subagent_name, dispatch_cfg, errors)
        web_search_mode = subagent_dispatch_web_search_mode(subagent_cfg)
        if declares_web_search and web_search_mode not in {"flag", "provider_configured"}:
            errors.append(
                f"{path}.dispatch.web_search_mode: required as 'flag' or 'provider_configured' when capability_band contains 'web_search'"
            )
        if not declares_web_search and web_search_mode in {"flag", "provider_configured"}:
            errors.append(
                f"{path}.capability_band: must contain 'web_search' when dispatch.web_search_mode enables web search"
            )


def _validate_routing(route_name: str, route_cfg: dict[str, Any], errors: list[str]) -> None:
    path = f"agent_system.routing.{route_name}"
    _validate_allowed_keys(route_cfg, ROUTING_KEYS, path, errors)
    subagents = _validate_repeated_string_field(route_cfg, "subagents", path, errors) if "subagents" in route_cfg else None
    if route_name == "default" and subagents is None:
        errors.append(f"{path}.subagents: missing required subagent order")
    _validate_string_map_field(route_cfg, "models", path, errors)
    _validate_string_map_field(route_cfg, "profiles", path, errors)
    for key in {
        "analysis_route_task_class",
        "analysis_merge_policy",
        "coach_route_task_class",
        "write_scope",
        "verification_gate",
        "merge_policy",
        "bridge_fallback_subagent",
        "internal_escalation_trigger",
        "verification_route_task_class",
        "graph_strategy",
        "budget_policy",
    }:
        if key in route_cfg:
            _validate_string_field(route_cfg, key, path, errors)
    for key in {
        "analysis_required",
        "analysis_external_first_required",
        "analysis_receipt_required",
        "analysis_zero_budget_required",
        "analysis_default_in_boot",
        "coach_required",
        "external_first_required",
        "web_search_required",
        "local_execution_allowed",
        "local_execution_preferred",
        "cli_dispatch_required_if_delegating",
        "direct_internal_bypass_forbidden",
        "independent_verification_required",
        "deterministic_first",
    }:
        if key in route_cfg:
            _validate_enum_field(route_cfg, key, path, errors, allowed=YES_NO_VALUES)
    if "dispatch_required" in route_cfg:
        _validate_enum_field(route_cfg, "dispatch_required", path, errors, allowed=ROUTING_DISPATCH_REQUIRED_VALUES)
    for key in {
        "analysis_fanout_min_results",
        "max_runtime_seconds",
        "min_output_bytes",
        "max_budget_units",
        "max_cli_subagent_calls",
        "max_coach_passes",
        "max_verification_passes",
        "max_fallback_hops",
        "max_total_runtime_seconds",
    }:
        if key in route_cfg:
            _validate_int_field(route_cfg, key, path, errors, min_value=0)
    fanout = _validate_repeated_string_field(route_cfg, "fanout_subagents", path, errors) if "fanout_subagents" in route_cfg else None
    analysis_fanout = _validate_repeated_string_field(route_cfg, "analysis_fanout_subagents", path, errors) if "analysis_fanout_subagents" in route_cfg else None
    if "allowed_internal_reasons" in route_cfg:
        _validate_repeated_string_field(route_cfg, "allowed_internal_reasons", path, errors)
    dispatch_required = _normalized_string(route_cfg.get("dispatch_required"))
    analysis_required = _normalized_string(route_cfg.get("analysis_required"))
    analysis_route_task_class = _normalized_string(route_cfg.get("analysis_route_task_class"))
    analysis_external_first_required = _normalized_string(route_cfg.get("analysis_external_first_required"))
    coach_required = _normalized_string(route_cfg.get("coach_required"))
    coach_route_task_class = _normalized_string(route_cfg.get("coach_route_task_class"))
    verification_required = _normalized_string(route_cfg.get("independent_verification_required"))
    verification_route_task_class = _normalized_string(route_cfg.get("verification_route_task_class"))
    local_execution_allowed = _normalized_string(route_cfg.get("local_execution_allowed"))
    local_execution_preferred = _normalized_string(route_cfg.get("local_execution_preferred"))
    external_first_required = _normalized_string(route_cfg.get("external_first_required"))
    if dispatch_required == "fanout_then_synthesize" and not fanout:
        errors.append(f"{path}.fanout_subagents: required when dispatch_required=fanout_then_synthesize")
    if analysis_required == "yes" and not analysis_route_task_class:
        errors.append(f"{path}.analysis_route_task_class: required when analysis_required=yes")
    if analysis_required == "yes" and analysis_external_first_required == "yes" and not analysis_fanout:
        errors.append(
            f"{path}.analysis_fanout_subagents: required when analysis_required=yes and analysis_external_first_required=yes"
        )
    if coach_required == "yes" and not coach_route_task_class:
        errors.append(f"{path}.coach_route_task_class: required when coach_required=yes")
    if verification_required == "yes" and not verification_route_task_class:
        errors.append(f"{path}.verification_route_task_class: required when independent_verification_required=yes")
    if coach_required == "yes" and verification_required == "yes" and coach_route_task_class == verification_route_task_class:
        errors.append(
            f"{path}.coach_route_task_class: must differ from verification_route_task_class when coach_required=yes and independent_verification_required=yes"
        )
    if external_first_required == "yes" and (local_execution_allowed == "yes" or local_execution_preferred == "yes"):
        errors.append(f"{path}.external_first_required: must be 'no' when local_execution_allowed=yes or local_execution_preferred=yes")
    fanout_min = _validate_int_field(route_cfg, "fanout_min_results", path, errors, min_value=0) if "fanout_min_results" in route_cfg else None
    if fanout is not None and fanout_min is not None and fanout_min > len(fanout):
        errors.append(f"{path}.fanout_min_results: must be <= number of fanout_subagents")
    analysis_fanout_min = _validate_int_field(route_cfg, "analysis_fanout_min_results", path, errors, min_value=0) if "analysis_fanout_min_results" in route_cfg else None
    if analysis_fanout is not None and analysis_fanout_min is not None and analysis_fanout_min > len(analysis_fanout):
        errors.append(f"{path}.analysis_fanout_min_results: must be <= number of analysis_fanout_subagents")
    max_coach_passes = _validate_int_field(route_cfg, "max_coach_passes", path, errors, min_value=0) if "max_coach_passes" in route_cfg else None
    if coach_required == "yes" and max_coach_passes is not None and max_coach_passes < 1:
        errors.append(f"{path}.max_coach_passes: must be >= 1 when coach_required=yes")
    max_fallback_hops = _validate_int_field(route_cfg, "max_fallback_hops", path, errors, min_value=0) if "max_fallback_hops" in route_cfg else None
    declared_fallback_hops = int(bool(_normalized_string(route_cfg.get("bridge_fallback_subagent")))) + int(
        bool(_normalized_string(route_cfg.get("internal_escalation_trigger")))
    )
    if max_fallback_hops is not None and max_fallback_hops < declared_fallback_hops:
        errors.append(
            f"{path}.max_fallback_hops: must be >= {declared_fallback_hops} to cover declared bridge_fallback_subagent and internal_escalation_trigger"
        )


def _validate_known_subagent_refs(
    *,
    field_name: str,
    values: list[str],
    path: str,
    known_subagents: set[str],
    route_subagents: set[str] | None,
    errors: list[str],
    require_route_membership: bool,
) -> None:
    for item in values:
        if item not in known_subagents:
            errors.append(f"{path}.{field_name}: unknown subagent '{item}'")
            continue
        if require_route_membership and route_subagents is not None and item not in route_subagents:
            errors.append(f"{path}.{field_name}: '{item}' must also be present in subagents")


def _validate_routing_cross_references(
    route_name: str,
    route_cfg: dict[str, Any],
    *,
    routing_cfg: dict[str, Any],
    subagents_cfg: dict[str, Any],
    known_subagents: set[str],
    known_routes: set[str],
    errors: list[str],
) -> None:
    path = f"agent_system.routing.{route_name}"
    route_subagents_list = _coerce_repeated_strings(route_cfg.get("subagents"))
    route_subagents = set(route_subagents_list) if route_subagents_list else None
    fanout_subagents = _coerce_repeated_strings(route_cfg.get("fanout_subagents"))
    analysis_fanout_subagents = _coerce_repeated_strings(route_cfg.get("analysis_fanout_subagents"))
    _validate_known_subagent_refs(
        field_name="subagents",
        values=route_subagents_list,
        path=path,
        known_subagents=known_subagents,
        route_subagents=None,
        errors=errors,
        require_route_membership=False,
    )
    _validate_known_subagent_refs(
        field_name="fanout_subagents",
        values=fanout_subagents,
        path=path,
        known_subagents=known_subagents,
        route_subagents=route_subagents,
        errors=errors,
        require_route_membership=True,
    )
    _validate_known_subagent_refs(
        field_name="analysis_fanout_subagents",
        values=analysis_fanout_subagents,
        path=path,
        known_subagents=known_subagents,
        route_subagents=None,
        errors=errors,
        require_route_membership=False,
    )

    for field_name in {"models", "profiles"}:
        value = route_cfg.get(field_name)
        if not _is_mapping(value):
            continue
        for item in value:
            if item not in known_subagents:
                errors.append(f"{path}.{field_name}: unknown subagent '{item}'")
                continue

    bridge_fallback_subagent = _normalized_string(route_cfg.get("bridge_fallback_subagent"))
    if bridge_fallback_subagent is not None:
        if bridge_fallback_subagent not in known_subagents:
            errors.append(f"{path}.bridge_fallback_subagent: unknown subagent '{bridge_fallback_subagent}'")
        if bridge_fallback_subagent in set(fanout_subagents):
            errors.append(f"{path}.bridge_fallback_subagent: must not overlap fanout_subagents")
        if bridge_fallback_subagent in set(analysis_fanout_subagents):
            errors.append(f"{path}.bridge_fallback_subagent: must not overlap analysis_fanout_subagents")
    if _normalized_string(route_cfg.get("web_search_required")) == "yes":
        has_web_capable_subagent = False
        wired_web_fanout_count = 0
        for item in route_subagents_list:
            if item not in known_subagents:
                continue
            subagent_cfg = subagents_cfg.get(item)
            if not _is_mapping(subagent_cfg):
                continue
            if subagent_has_web_search_wiring(subagent_cfg):
                has_web_capable_subagent = True
        for item in fanout_subagents:
            if item not in known_subagents:
                continue
            subagent_cfg = subagents_cfg.get(item)
            if _is_mapping(subagent_cfg) and subagent_has_web_search_wiring(subagent_cfg):
                wired_web_fanout_count += 1
        if not has_web_capable_subagent:
            errors.append(
                f"{path}.web_search_required: requires at least one route subagent with declared 'web_search' capability and dispatch wiring"
            )
        fanout_min_results = route_cfg.get("fanout_min_results")
        try:
            configured_fanout_min = int(fanout_min_results) if fanout_min_results is not None else 0
        except (TypeError, ValueError):
            configured_fanout_min = 0
        if fanout_subagents and configured_fanout_min > wired_web_fanout_count:
            errors.append(
                f"{path}.fanout_min_results: must be <= number of web-search-wired fanout_subagents when web_search_required=yes"
            )

    for field_name in {"analysis_route_task_class", "coach_route_task_class", "verification_route_task_class"}:
        route_ref = _normalized_string(route_cfg.get(field_name))
        if route_ref is None:
            continue
        if route_ref not in known_routes:
            errors.append(f"{path}.{field_name}: unknown route '{route_ref}'")
    analysis_route_task_class = _normalized_string(route_cfg.get("analysis_route_task_class"))
    if analysis_route_task_class is not None and analysis_route_task_class in known_routes:
        analysis_route_cfg = routing_cfg.get(analysis_route_task_class)
        if _is_mapping(analysis_route_cfg):
            analysis_route_subagents = set(_coerce_repeated_strings(analysis_route_cfg.get("subagents")))
            for item in analysis_fanout_subagents:
                if item in known_subagents and item not in analysis_route_subagents:
                    errors.append(
                        f"{path}.analysis_fanout_subagents: '{item}' must also be present in agent_system.routing.{analysis_route_task_class}.subagents"
                    )


def _validate_scoring(scoring_cfg: dict[str, Any], errors: list[str]) -> None:
    path = "agent_system.scoring"
    _validate_allowed_keys(scoring_cfg, SCORING_KEYS, path, errors)
    for key in SCORING_KEYS:
        if key in scoring_cfg:
            _validate_int_field(scoring_cfg, key, path, errors, min_value=0)


def _validate_agent_system(agent_cfg: dict[str, Any], errors: list[str]) -> None:
    path = "agent_system"
    _validate_allowed_keys(agent_cfg, AGENT_SYSTEM_KEYS, path, errors)
    if "init_on_boot" in agent_cfg:
        _validate_bool_field(agent_cfg, "init_on_boot", path, errors)
    _validate_enum_field(agent_cfg, "mode", path, errors, allowed=AGENT_SYSTEM_MODES, required=True)
    _validate_string_field(agent_cfg, "state_owner", path, errors, required=True)
    _validate_int_field(agent_cfg, "max_parallel_agents", path, errors, required=True, min_value=1)
    subagents_cfg = _require_mapping(agent_cfg, "subagents", path, errors, required=True)
    if subagents_cfg is not None:
        if not subagents_cfg:
            errors.append(f"{path}.subagents: must not be empty")
        for subagent_name, subagent_cfg in subagents_cfg.items():
            if not isinstance(subagent_name, str) or not subagent_name.strip():
                errors.append(f"{path}.subagents: subagent names must be non-empty strings")
                continue
            if not _is_mapping(subagent_cfg):
                errors.append(f"{path}.subagents.{subagent_name}: expected mapping")
                continue
            _validate_subagent(subagent_name, subagent_cfg, errors)
    routing_cfg = _require_mapping(agent_cfg, "routing", path, errors, required=True)
    if routing_cfg is not None:
        if not routing_cfg:
            errors.append(f"{path}.routing: must not be empty")
        for route_name, route_cfg in routing_cfg.items():
            if not isinstance(route_name, str) or not route_name.strip():
                errors.append(f"{path}.routing: route names must be non-empty strings")
                continue
            if not _is_mapping(route_cfg):
                errors.append(f"{path}.routing.{route_name}: expected mapping")
                continue
            _validate_routing(route_name, route_cfg, errors)
    if subagents_cfg is not None and routing_cfg is not None:
        known_subagents = {
            subagent_name
            for subagent_name, subagent_cfg in subagents_cfg.items()
            if isinstance(subagent_name, str) and subagent_name.strip() and _is_mapping(subagent_cfg)
        }
        known_routes = {
            route_name for route_name, route_cfg in routing_cfg.items() if isinstance(route_name, str) and route_name.strip() and _is_mapping(route_cfg)
        }
        for route_name, route_cfg in routing_cfg.items():
            if not isinstance(route_name, str) or not route_name.strip() or not _is_mapping(route_cfg):
                continue
            _validate_routing_cross_references(
                route_name,
                route_cfg,
                routing_cfg=routing_cfg,
                subagents_cfg=subagents_cfg,
                known_subagents=known_subagents,
                known_routes=known_routes,
                errors=errors,
            )
    scoring_cfg = _require_mapping(agent_cfg, "scoring", path, errors, required=False)
    if scoring_cfg is not None:
        _validate_scoring(scoring_cfg, errors)


def validate_config(cfg: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    if not isinstance(cfg, dict):
        return ["root: expected mapping"]
    _validate_allowed_keys(cfg, TOP_LEVEL_REQUIRED | TOP_LEVEL_OPTIONAL, "root", errors)
    for key in TOP_LEVEL_REQUIRED:
        if key not in cfg:
            errors.append(f"root.{key}: missing required section")

    project_cfg = _require_mapping(cfg, "project", "root", errors, required=True)
    if project_cfg is not None:
        _validate_project(project_cfg, errors)

    protocol_cfg = _require_mapping(cfg, "protocol_activation", "root", errors, required=True)
    agent_system_active = False
    if protocol_cfg is not None:
        agent_system_active = _validate_protocol_activation(protocol_cfg, errors)

    for key, allowed in {
        "language_policy": LANGUAGE_POLICY_KEYS,
        "pack_router_keywords": PACK_ROUTER_KEYS,
    }.items():
        section = _require_mapping(cfg, key, "root", errors, required=False)
        if section is not None:
            _validate_flat_string_section(section, allowed, key, errors)

    bootstrap_cfg = _require_mapping(cfg, "project_bootstrap", "root", errors, required=False)
    if bootstrap_cfg is not None:
        _validate_project_bootstrap(bootstrap_cfg, errors)
    framework_self_diagnosis_cfg = _require_mapping(cfg, "framework_self_diagnosis", "root", errors, required=False)
    if framework_self_diagnosis_cfg is not None:
        _validate_framework_self_diagnosis(framework_self_diagnosis_cfg, errors)

    agent_cfg = _require_mapping(cfg, "agent_system", "root", errors, required=agent_system_active)
    if agent_cfg is not None:
        _validate_agent_system(agent_cfg, errors)
    return errors


def format_validation_errors(errors: list[str]) -> str:
    if not errors:
        return "vida.config.yaml schema OK"
    lines = ["Invalid vida.config.yaml schema:"]
    lines.extend(f"- {item}" for item in errors)
    return "\n".join(lines)


def load_config(*, validate: bool = False) -> dict[str, Any]:
    if not CONFIG_PATH.exists():
        return {}
    payload = parse_yaml_subset(CONFIG_PATH.read_text())
    if validate:
        errors = validate_config(payload)
        if errors:
            raise OverlayValidationError(errors)
    return payload


def load_validated_config() -> dict[str, Any]:
    return load_config(validate=True)


def validate_current_config() -> dict[str, Any]:
    if not CONFIG_PATH.exists():
        return {
            "config_path": str(CONFIG_PATH),
            "exists": False,
            "valid": True,
            "errors": [],
        }
    try:
        payload = load_config(validate=False)
    except ValueError as exc:
        return {
            "config_path": str(CONFIG_PATH),
            "exists": True,
            "valid": False,
            "errors": [str(exc)],
        }
    errors = validate_config(payload)
    return {
        "config_path": str(CONFIG_PATH),
        "exists": True,
        "valid": not errors,
        "errors": errors,
    }


def dotted_get(payload: dict[str, Any], path: str, default: Any = None) -> Any:
    current: Any = payload
    for part in path.split("."):
        if not isinstance(current, dict) or part not in current:
            return default
        current = current[part]
    return current


def usage() -> None:
    print(
        "Usage:\n"
        "  python3 _vida/scripts/vida-config.py path\n"
        "  python3 _vida/scripts/vida-config.py exists\n"
        "  python3 _vida/scripts/vida-config.py json\n"
        "  python3 _vida/scripts/vida-config.py get <dotted.path> [default]\n"
        "  python3 _vida/scripts/vida-config.py validate [--json]\n"
        "  python3 _vida/scripts/vida-config.py protocol-active <name>\n"
        "  python3 _vida/scripts/vida-config.py subagent-mode",
        file=sys.stderr,
    )


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        usage()
        return 1

    cmd = argv[1]
    if cmd == "path":
        print(CONFIG_PATH if CONFIG_PATH.exists() else "")
        return 0
    if cmd == "exists":
        print("yes" if CONFIG_PATH.exists() else "no")
        return 0 if CONFIG_PATH.exists() else 1
    if cmd == "validate":
        as_json = "--json" in argv[2:]
        result = validate_current_config()
        if as_json:
            print(json.dumps(result, indent=2, sort_keys=True))
        elif result["valid"]:
            print("vida.config.yaml schema OK")
        else:
            print(format_validation_errors(result["errors"]), file=sys.stderr)
        return 0 if result["valid"] else 1

    try:
        cfg = load_validated_config()
    except (ValueError, OverlayValidationError) as exc:
        print(str(exc), file=sys.stderr)
        return 2

    if cmd == "json":
        print(json.dumps(cfg, indent=2, sort_keys=True))
        return 0
    if cmd == "get":
        if len(argv) < 3:
            usage()
            return 1
        default = argv[3] if len(argv) > 3 else ""
        value = dotted_get(cfg, argv[2], default)
        if isinstance(value, (dict, list)):
            print(json.dumps(value, sort_keys=True))
        elif value is None:
            print("null")
        else:
            print(value)
        return 0
    if cmd == "protocol-active":
        if len(argv) < 3:
            usage()
            return 1
        active = bool(dotted_get(cfg, f"protocol_activation.{argv[2]}", False))
        print("yes" if active else "no")
        return 0 if active else 1
    if cmd == "subagent-mode":
        print(dotted_get(cfg, "agent_system.mode", "native"))
        return 0

    usage()
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
